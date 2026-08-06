//! Prometheus metrics for the CASIROS API server.
//!
//! All metrics use the `casiros_` prefix. [`init_metrics`] builds the registry
//! once at startup; the counters and histograms live in module-level statics
//! that the middleware and handlers record through.

use std::sync::OnceLock;

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, Opts, Registry, TextEncoder};

/// The global Prometheus registry, initialised once at startup.
static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// HTTP request count by method, path, and status code.
static HTTP_REQUESTS: OnceLock<IntCounterVec> = OnceLock::new();

/// HTTP request duration in seconds by method and path.
static HTTP_DURATION: OnceLock<HistogramVec> = OnceLock::new();

/// Rate-limit denials by tenant.
static RATE_LIMIT_DENIALS: OnceLock<IntCounterVec> = OnceLock::new();

/// Job state transitions by status.
static JOBS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

/// Audit write failures.
static AUDIT_WRITE_FAILURES: OnceLock<IntCounterVec> = OnceLock::new();

/// Initialises all metrics and registers them with the global registry.
///
/// Must be called exactly once at server startup.
///
/// # Panics
///
/// Panics if called more than once, because the `OnceLock` rejects a second
/// initialisation.
pub fn init_metrics() {
    let registry = Registry::new();

    let http_requests = IntCounterVec::new(
        Opts::new("casiros_http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"],
    )
    .expect("metric definition is valid");
    registry
        .register(Box::new(http_requests.clone()))
        .expect("registration succeeds once");

    let http_duration = HistogramVec::new(
        HistogramOpts::new(
            "casiros_http_request_duration_seconds",
            "HTTP request duration in seconds",
        )
        .buckets(prometheus::DEFAULT_BUCKETS.to_vec()),
        &["method", "path"],
    )
    .expect("metric definition is valid");
    registry
        .register(Box::new(http_duration.clone()))
        .expect("registration succeeds once");

    let rate_limit_denials = IntCounterVec::new(
        Opts::new(
            "casiros_rate_limit_denials_total",
            "Rate-limit denials by tenant",
        ),
        &["tenant"],
    )
    .expect("metric definition is valid");
    registry
        .register(Box::new(rate_limit_denials.clone()))
        .expect("registration succeeds once");

    let jobs_total = IntCounterVec::new(
        Opts::new("casiros_jobs_total", "Job state transitions by status"),
        &["status"],
    )
    .expect("metric definition is valid");
    registry
        .register(Box::new(jobs_total.clone()))
        .expect("registration succeeds once");

    let audit_write_failures = IntCounterVec::new(
        Opts::new(
            "casiros_audit_write_failures_total",
            "Audit log write failures",
        ),
        &[],
    )
    .expect("metric definition is valid");
    registry
        .register(Box::new(audit_write_failures.clone()))
        .expect("registration succeeds once");

    REGISTRY.set(registry).expect("init_metrics called once");
    HTTP_REQUESTS
        .set(http_requests)
        .expect("init_metrics called once");
    HTTP_DURATION
        .set(http_duration)
        .expect("init_metrics called once");
    RATE_LIMIT_DENIALS
        .set(rate_limit_denials)
        .expect("init_metrics called once");
    JOBS_TOTAL
        .set(jobs_total)
        .expect("init_metrics called once");
    AUDIT_WRITE_FAILURES
        .set(audit_write_failures)
        .expect("init_metrics called once");
}

/// Records an HTTP request with its method, path, status, and duration.
pub fn observe_http(method: &str, path: &str, status: u16, duration_secs: f64) {
    if let Some(counter) = HTTP_REQUESTS.get() {
        counter
            .get_metric_with_label_values(&[method, path, &status.to_string()])
            .map(|c| c.inc())
            .ok();
    }
    if let Some(hist) = HTTP_DURATION.get() {
        hist.get_metric_with_label_values(&[method, path])
            .map(|h| h.observe(duration_secs))
            .ok();
    }
}

/// Records a rate-limit denial for a tenant.
pub fn inc_rate_limit_denial(tenant: &str) {
    if let Some(counter) = RATE_LIMIT_DENIALS.get() {
        counter
            .get_metric_with_label_values(&[tenant])
            .map(|c| c.inc())
            .ok();
    }
}

/// Records a job state transition.
pub fn inc_job(status: &str) {
    if let Some(counter) = JOBS_TOTAL.get() {
        counter
            .get_metric_with_label_values(&[status])
            .map(|c| c.inc())
            .ok();
    }
}

/// Records an audit write failure.
pub fn inc_audit_write_failure() {
    if let Some(counter) = AUDIT_WRITE_FAILURES.get() {
        // prometheus 0.14 made the label-values generic over AsRef<str>, so an
        // empty slice needs an explicit element type.
        counter
            .get_metric_with_label_values(&[] as &[&str])
            .map(|c| c.inc())
            .ok();
    }
}

/// Renders all metrics in Prometheus text format.
///
/// # Errors
///
/// Returns a string error if encoding fails, which should never happen.
pub fn render() -> Result<String, String> {
    let registry = REGISTRY.get().ok_or("metrics not initialised")?;
    let metric_families = registry.gather();
    TextEncoder::new()
        .encode_to_string(&metric_families)
        .map_err(|err| format!("metrics encoding failed: {err}"))
}
