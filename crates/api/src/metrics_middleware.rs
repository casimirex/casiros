//! Actix-Web middleware that records HTTP metrics to Prometheus.
//!
//! Wraps every request in a timer and records method, path, and status code
//! to the `casiros_http_requests_total` counter and
//! `casiros_http_request_duration_seconds` histogram.

use std::time::Instant;

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{Error, middleware::Next};

use crate::metrics;

/// Records request metrics and passes through to the next service.
///
/// # Errors
///
/// Returns the underlying error if the inner service fails.
pub async fn metrics_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
) -> Result<ServiceResponse<BoxBody>, Error>
where
    B: MessageBody + 'static,
{
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.path().to_string();

    let response = next.call(req).await?.map_into_boxed_body();

    let status = response.status().as_u16();
    let duration = start.elapsed().as_secs_f64();

    metrics::observe_http(&method, &path, status, duration);

    return Ok(response);
}
