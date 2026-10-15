//! CI Regression Gate — loads a baseline evaluation file, runs evaluations on
//! current LLM outputs, and produces a pass/fail report suitable for CI pipelines.
//!
//! Usage in a CI workflow:
//! ```yaml
//! - name: LLM Regression Gate
//!   run: ./target/debug/ci-gate --baseline baseline.json --output results.json
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use log::info;
use serde::{Deserialize, Serialize};

use crate::evaluation::{
    EvaluationCriterion, EvaluationGate, EvaluationResult, Evaluator,
};
use crate::traits::LLMProvider;

/// Configuration for the CI regression gate runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGateConfig {
    /// LLM model to use for evaluation (e.g. "gpt-4o").
    pub model: String,
    /// API key for the LLM provider.
    pub api_key: String,
    /// Maximum allowed degradation per criterion (0–100 points).
    pub max_degradation: f64,
    /// Optional threshold for overall passing score (0–100).
    pub threshold: Option<f64>,
    /// Criteria to evaluate; if empty, all four are used.
    pub criteria: Vec<EvaluationCriterion>,
    /// Number of concurrent evaluation tasks.
    pub concurrency: usize,
}

impl Default for CiGateConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            api_key: String::new(),
            max_degradation: 5.0,
            threshold: None,
            criteria: vec![
                EvaluationCriterion::Accuracy,
                EvaluationCriterion::Relevance,
                EvaluationCriterion::Coherence,
                EvaluationCriterion::Safety,
            ],
            concurrency: 4,
        }
    }
}

/// Full report produced by the CI gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiGateReport {
    /// Whether the gate as a whole passed.
    pub passed: bool,
    /// Evaluation results for each prompt–response pair.
    pub results: Vec<EvaluationResult>,
    /// Regression checks against baseline (empty if no baseline was provided).
    pub regression: Option<RegressionSummary>,
    /// Number of samples evaluated.
    pub total: usize,
    /// Number of samples that passed the threshold.
    pub passed_count: usize,
}

/// Summary of the regression check, embedded in [`CiGateReport`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionSummary {
    /// Whether the regression check passed.
    pub passed: bool,
    /// Per-criterion average deltas (positive means improvement).
    pub deltas: HashMap<EvaluationCriterion, f64>,
    /// Overall score delta.
    pub overall_delta: f64,
    /// Which criteria regressed beyond the allowed degradation.
    pub regressed_criteria: Vec<EvaluationCriterion>,
}

/// A sample to evaluate: one prompt and its corresponding LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationSample {
    pub prompt: String,
    pub response: String,
}

/// Baseline file format: a list of previously recorded [`EvaluationResult`]s.
pub type Baseline = Vec<EvaluationResult>;

/// Runner that orchestrates the full CI gate workflow.
pub struct CiGateRunner {
    config: CiGateConfig,
    evaluator: Evaluator,
    gate: EvaluationGate,
}

impl CiGateRunner {
    /// Create a new runner from config and an LLM provider.
    pub fn new(provider: Arc<dyn LLMProvider>, config: CiGateConfig) -> Self {
        let mut evaluator = Evaluator::new(
            provider,
            config.model.clone(),
            config.api_key.clone(),
        );
        if let Some(t) = config.threshold {
            evaluator = evaluator.with_threshold(t);
        }

        let gate = EvaluationGate::new(config.max_degradation);

        Self {
            config,
            evaluator,
            gate,
        }
    }

    /// Run the full CI gate workflow:
    /// 1. Load baseline (optional — pass empty path to skip).
    /// 2. Evaluate all samples.
    /// 3. Compare against baseline if provided.
    /// 4. Produce report.
    pub async fn run<P1: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        baseline_path: P1,
        output_path: P2,
    ) -> Result<CiGateReport, String> {
        // 1. Load baseline
        let baseline: Option<Baseline> = load_json(baseline_path.as_ref())
            .map_err(|e| format!("Failed to load baseline: {e}"))?
            .unwrap_or(None);

        // 2. Load output samples
        let samples: Vec<EvaluationSample> = load_json(output_path.as_ref())
            .map_err(|e| format!("Failed to load output samples: {e}"))?
            .unwrap_or_default();

        info!(
            "CI gate: loaded {} samples, baseline {}",
            samples.len(),
            if baseline.is_some() {
                "present"
            } else {
                "absent"
            }
        );

        // 3. Evaluate each sample
        let results = self.evaluate_samples(&samples).await?;

        // 4. Check regression if baseline exists
        let regression = if let Some(ref base) = baseline {
            let report = self.gate.check_regression(&results, base);
            Some(RegressionSummary {
                passed: report.passed,
                deltas: report.deltas,
                overall_delta: report.overall_delta,
                regressed_criteria: report.regressed_criteria,
            })
        } else {
            None
        };

        // 5. Compute overall pass/fail
        let passed_count = results
            .iter()
            .filter(|r| r.overall >= self.evaluator.threshold)
            .count();

        let all_above_threshold = passed_count == results.len();
        let regression_passed = regression.as_ref().map(|r| r.passed).unwrap_or(true);

        let passed = all_above_threshold && regression_passed;

        let report = CiGateReport {
            passed,
            results,
            regression,
            total: samples.len(),
            passed_count,
        };

        // Persist report to output path
        let report_json = serde_json::to_string_pretty(&report)
            .map_err(|e| format!("Failed to serialize report: {e}"))?;
        tokio::fs::write(output_path.as_ref(), report_json)
            .await
            .map_err(|e| format!("Failed to write report: {e}"))?;

        info!("CI gate report: passed={}, total={}, passed={}", passed, report.total, report.passed_count);

        Ok(report)
    }

    /// Evaluate all samples concurrently (capped by `concurrency`).
    async fn evaluate_samples(
        &self,
        samples: &[EvaluationSample],
    ) -> Result<Vec<EvaluationResult>, String> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.concurrency.max(1)));
        let mut handles = Vec::with_capacity(samples.len());

        for sample in samples {
            let s = sample.clone();
            let evaluator = self.evaluator.clone();
            let criteria = self.config.criteria.clone();
            let permit = Arc::clone(&semaphore);

            handles.push(tokio::spawn(async move {
                let _guard = permit.acquire().await.map_err(|e| format!("Semaphore error: {e}"))?;
                evaluator.evaluate(&s.prompt, &s.response, &criteria).await
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let result = handle
                .await
                .map_err(|e| format!("Evaluation task failed: {e}"))?;
            results.push(result?);
        }

        Ok(results)
    }
}

/// Load and deserialize a JSON file.
fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(None);
    }
    let value: T = serde_json::from_str(&content)
        .map_err(|e| format!("Cannot parse {}: {e}", path.display()))?;
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::EvaluationCriterion;
    use std::collections::HashMap;

    #[test]
    fn test_ci_gate_config_default() {
        let config = CiGateConfig::default();
        assert_eq!(config.max_degradation, 5.0);
        assert_eq!(config.criteria.len(), 4);
        assert_eq!(config.concurrency, 4);
    }

    #[test]
    fn test_load_json_missing_file() {
        let result: Result<Option<Vec<u8>>, String> =
            load_json(Path::new("/tmp/nonexistent-file-for-test-12345.json"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_load_json_empty_file() {
        let path = Path::new("/tmp/test-empty-json-12345.json");
        let _ = std::fs::write(path, "");
        let result: Result<Option<Vec<u8>>, String> = load_json(path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_load_json_valid() {
        let path = Path::new("/tmp/test-valid-json-12345.json");
        let data = serde_json::json!([{"prompt": "hi", "response": "hello"}]);
        let _ = std::fs::write(path, data.to_string());
        let result: Result<Option<Vec<EvaluationSample>>, String> = load_json(path);
        let samples = result.unwrap().unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].prompt, "hi");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_regression_summary_serialization() {
        let mut deltas = HashMap::new();
        deltas.insert(EvaluationCriterion::Accuracy, -3.0);
        let summary = RegressionSummary {
            passed: true,
            deltas,
            overall_delta: -1.5,
            regressed_criteria: vec![],
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: RegressionSummary = serde_json::from_str(&json).unwrap();
        assert!(deserialized.passed);
        assert_eq!(
            deserialized.deltas.get(&EvaluationCriterion::Accuracy),
            Some(&-3.0)
        );
    }
}
