use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};

pub type RetryPredicate = Arc<dyn Fn(&str) -> bool + Send + Sync>;

#[derive(Debug, Clone)]
pub enum ResilienceError {
    Timeout { duration: Duration },
    CircuitOpen { until: Option<Duration> },
    RetriesExhausted { attempts: u32, last_error: String },
    BulkheadFull { max_concurrent: usize },
    Operation(String),
}

impl std::fmt::Display for ResilienceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout { duration } => {
                write!(f, "Operation timed out after {:?}", duration)
            }
            Self::CircuitOpen { until } => {
                if let Some(d) = until {
                    write!(f, "Circuit breaker open, retry after {:?}", d)
                } else {
                    write!(f, "Circuit breaker open")
                }
            }
            Self::RetriesExhausted {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "All {} retry attempts exhausted. Last error: {}",
                    attempts, last_error
                )
            }
            Self::BulkheadFull { max_concurrent } => {
                write!(
                    f,
                    "Bulkhead full, max {} concurrent requests",
                    max_concurrent
                )
            }
            Self::Operation(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

impl std::error::Error for ResilienceError {}

#[derive(Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter_factor: f64,
    retryable: Option<RetryPredicate>,
}

impl std::fmt::Debug for RetryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetryConfig")
            .field("max_attempts", &self.max_attempts)
            .field("initial_delay", &self.initial_delay)
            .field("max_delay", &self.max_delay)
            .field("backoff_multiplier", &self.backoff_multiplier)
            .field("jitter_factor", &self.jitter_factor)
            .field("retryable", &self.retryable.is_some())
            .finish()
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.2,
            retryable: None,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config with custom max attempts
    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts.max(1);
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay cap
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier.max(1.0);
        self
    }

    /// Set jitter factor (0.0 to 1.0)
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter_factor = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set custom retryable predicate
    pub fn with_retryable<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.retryable = Some(Arc::new(predicate));
        self
    }

    /// Aggressive retry for critical operations
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 1.5,
            jitter_factor: 0.3,
            retryable: None,
        }
    }

    /// Conservative retry for non-critical operations
    pub fn conservative() -> Self {
        Self {
            max_attempts: 2,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
            retryable: None,
        }
    }

    fn calculate_delay(&self, attempt: u32) -> Duration {
        let exponent = i32::try_from(attempt.saturating_sub(1)).unwrap_or(0);
        let base_delay = self.backoff_multiplier.powi(exponent) * self.initial_delay.as_secs_f64();

        let capped_delay = base_delay.min(self.max_delay.as_secs_f64());

        let jitter = if self.jitter_factor > 0.0 {
            let jitter_range = capped_delay * self.jitter_factor;
            // Golden-ratio hashing spreads retries evenly across the jitter
            // window without drawing from a random source, which keeps the
            // backoff reproducible for a given attempt count.
            let pseudo_random = (f64::from(attempt) * std::f64::consts::GOLDEN_RATIO) % 1.0;
            (2.0_f64).mul_add(pseudo_random, -1.0) * jitter_range
        } else {
            0.0
        };

        Duration::from_secs_f64((capped_delay + jitter).max(0.001))
    }

    fn is_retryable(&self, error: &str) -> bool {
        if let Some(ref predicate) = self.retryable {
            predicate(error)
        } else {
            error.contains("timeout")
                || error.contains("connection")
                || error.contains("temporarily")
                || error.contains("503")
                || error.contains("429")
        }
    }
}

pub async fn retry<F, Fut, T>(config: &RetryConfig, mut operation: F) -> Result<T, ResilienceError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let mut last_error = String::new();

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt == config.max_attempts {
                    last_error = e;
                    break;
                }

                if !config.is_retryable(&e) {
                    return Err(ResilienceError::Operation(e));
                }

                last_error = e;
                let delay = config.calculate_delay(attempt);
                sleep(delay).await;
            }
        }
    }

    Err(ResilienceError::RetriesExhausted {
        attempts: config.max_attempts,
        last_error,
    })
}

pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T, ResilienceError>
where
    F: Future<Output = T>,
{
    timeout(duration, future)
        .await
        .map_err(|_| ResilienceError::Timeout { duration })
}
