use super::*;

// ---------------------------------------------------------------------------
// Tower Layer / Service middleware for Axum (server-side tracing)
// ---------------------------------------------------------------------------

/// A Tower Layer that wraps every request with W3C trace context propagation.
///
/// Extracts or creates a traceparent, logs the request, and passes the
/// SpanContext to downstream services via request extensions.
#[derive(Clone)]
pub struct TracingMiddleware {
    pub service_name: String,
}

impl TracingMiddleware {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }
}

impl<S> tower::Layer<S> for TracingMiddleware {
    type Service = TracingService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracingService {
            inner,
            service_name: self.service_name.clone(),
        }
    }
}

/// The Tower Service produced by TracingMiddleware.
#[derive(Clone)]
pub struct TracingService<S> {
    inner: S,
    service_name: String,
}

impl<S, ReqBody> tower::Service<http::Request<ReqBody>> for TracingService<S>
where
    S: tower::Service<http::Request<ReqBody>>,
    S::Future: Send + 'static,
    S::Error: 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<ReqBody>) -> Self::Future {
        let span = extract_context(req.headers())
            .unwrap_or_else(|| SpanContext::new(format!("{} {}", req.method(), req.uri().path())));
        let service_name = self.service_name.clone();

        log::info!(
            "[{}] tracing: {} {} trace_id={} span_id={}",
            service_name,
            req.method(),
            req.uri().path(),
            span.trace_id,
            span.span_id
        );

        let fut = self.inner.call(req);
        Box::pin(async move {
            let resp = fut.await?;
            log::debug!(
                "[tracing] completed span={} trace_id={}",
                span.span_id,
                span.trace_id
            );
            Ok(resp)
        })
    }
}
