//! Tracing middleware for Actix-Web requests.
//!
//! Wraps every request in a [`tracing`] span that captures the HTTP method, path,
//! and response status along with the elapsed latency. This is intentionally
//! separate from the authentication middleware so that observability can be
//! enabled or composed independently.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, Result};
use tracing::{Instrument, info_span};

/// Middleware factory that installs a per-request tracing span.
#[derive(Debug, Default, Clone)]
pub struct TracingMiddleware;

impl TracingMiddleware {
    /// Creates a new tracing middleware instance.
    #[must_use]
    pub fn new() -> Self {
        return Self;
    }
}

impl<S, B> Transform<S, ServiceRequest> for TracingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = TracingMiddlewareService<S>;
    type InitError = ();
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        return std::future::ready(Ok(TracingMiddlewareService { service }));
    }
}

/// Wrapped service that creates a span around the inner service call.
pub struct TracingMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for TracingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        return self.service.poll_ready(cx);
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let span = info_span!(
            "http_request",
            http.method = %method,
            http.route = %path,
            http.status_code = tracing::field::Empty,
        );

        let fut = self.service.call(req);
        let span_for_record = span.clone();
        return Box::pin(
            async move {
                let start = std::time::Instant::now();
                let res = fut.await?;
                let status = res.status().as_u16();
                span_for_record.record("http.status_code", status);
                tracing::info!(
                    parent: &span_for_record,
                    latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX),
                    "request completed"
                );
                return Ok(res.map_into_boxed_body());
            }
            .instrument(span),
        );
    }
}
