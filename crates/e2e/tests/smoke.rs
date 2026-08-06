//! End-to-end smoke tests against the real binaries.
//!
//! Each test here corresponds to a defect that shipped despite a green test
//! suite, because the in-process tests rebuild the application by hand and
//! never execute `main.rs`.
//!
//! These tests need a reachable `PostgreSQL`. Run `docker compose up -d postgres`
//! first, or set `CASIROS__POSTGRES__URL`.

// Digit separators inside JSON payloads would obscure the values rather than
// clarify them: these are wire literals mirroring what a client would send,
// not Rust constants.
#![allow(clippy::unreadable_literal)]
#![allow(clippy::needless_return)]

mod harness;

use harness::{ADMIN_KEY, API_KEY, ApiServer, Worker, client, formula_request};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Configuration wiring
// ---------------------------------------------------------------------------

/// The documented Postgres environment variables must actually select Postgres.
///
/// Regression test: the docs, README, `docker-compose.yml`, and `Dockerfile`
/// all used a single underscore after the `CASIROS` prefix. The config crate
/// silently ignores that form, so the shipped compose file ran in-memory while
/// claiming Postgres. Nothing failed — the server simply used defaults.
#[test]
fn postgres_backend_is_selected_by_documented_env_vars() {
    let api = ApiServer::start_postgres();
    assert!(
        api.logged("Using Postgres"),
        "expected Postgres backend, startup log said: {:?}",
        api.startup_log()
    );
    assert!(!api.logged_now("Using in-memory"));
}

/// The default configuration still starts, on the in-memory backend.
#[test]
fn memory_backend_is_the_default() {
    let api = ApiServer::start_memory();
    assert!(api.logged("Using in-memory"));
    assert!(!api.logged_now("Using Postgres"));
}

/// `CASIROS__BIND_ADDR` must be honoured.
///
/// Regression test: the `Dockerfile` set `CASIROS_BIND_ADDR` (single
/// underscore) to `0.0.0.0:8080` so containers would accept external traffic.
/// The variable was ignored and the server fell back to localhost, leaving the
/// container unreachable. The harness sets the double-underscore form for every
/// test, so a regression here breaks every test in this file at once — but the
/// assertion makes the intent explicit.
#[test]
fn bind_address_env_var_is_honoured() {
    let api = ApiServer::start_memory();
    assert!(api.base.starts_with("http://127.0.0.1:"));
    let resp = client()
        .get(format!("{}/healthz", api.base))
        .send()
        .expect("health request succeeds");
    assert_eq!(resp.status(), 200);
}

// ---------------------------------------------------------------------------
// Public paths
// ---------------------------------------------------------------------------

/// Probe and asset endpoints must be reachable without credentials, at both
/// the root and the versioned prefix.
///
/// Regression test: adding the `/v1` scope left `is_public_path` matching only
/// unversioned paths, so `/v1/healthz` and `/v1/metrics` returned 401. Health
/// probes and Prometheus scrapes cannot present an API key. Separately,
/// `/dashboard` was never public at all, so the browser UI could not load
/// whenever authentication was enabled.
#[test]
fn public_paths_need_no_api_key() {
    let api = ApiServer::start_memory();
    let http = client();

    for path in [
        "/healthz",
        "/metrics",
        "/v1/healthz",
        "/v1/metrics",
        "/openapi.json",
        "/dashboard",
        "/dashboard/style.css",
        "/dashboard/app.js",
    ] {
        let resp = http
            .get(format!("{}{path}", api.base))
            .send()
            .unwrap_or_else(|e| panic!("request to {path} failed: {e}"));
        assert!(
            resp.status().is_success(),
            "{path} should be public, got {}",
            resp.status()
        );
    }
}

/// Protected endpoints must still reject unauthenticated callers, including
/// under the version prefix — the public-path fix must not have opened a hole.
#[test]
fn protected_paths_still_require_a_key() {
    let api = ApiServer::start_memory();
    let http = client();

    for path in ["/evaluate", "/v1/evaluate"] {
        let resp = http
            .post(format!("{}{path}", api.base))
            .json(&serde_json::json!({}))
            .send()
            .expect("request completes");
        assert_eq!(resp.status(), 401, "{path} must require a key");
    }

    for path in ["/snapshots", "/audit", "/v1/snapshots", "/admin/tenants"] {
        let resp = http
            .get(format!("{}{path}", api.base))
            .send()
            .expect("request completes");
        assert_eq!(resp.status(), 401, "{path} must require a key");
    }
}

/// The dashboard must be served regardless of the process working directory.
///
/// Regression test: the static-files route used a bare relative path, so it
/// resolved against the working directory. Started from anywhere but the repo
/// root — including the Docker image, whose WORKDIR is /app — /dashboard
/// returned 404. The image also never copied web/ at all.
#[test]
fn dashboard_is_served_from_any_working_directory() {
    let api = ApiServer::start_from_dir(std::path::Path::new("/"));
    let resp = client()
        .get(format!("{}/dashboard", api.base))
        .send()
        .expect("request completes");
    assert!(
        resp.status().is_success(),
        "/dashboard should not depend on the working directory, got {}",
        resp.status()
    );
}

/// A path that merely looks versioned must not be treated as public.
#[test]
fn version_lookalike_paths_are_not_public() {
    let api = ApiServer::start_memory();
    let resp = client()
        .get(format!("{}/vault/healthz", api.base))
        .send()
        .expect("request completes");
    assert_ne!(resp.status(), 200, "/vault/healthz must not be public");
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Evaluation returns an exact decimal result over real HTTP.
#[test]
fn evaluate_returns_an_exact_result() {
    let api = ApiServer::start_memory();
    let body = serde_json::json!({
        "nodes": [
            {"input": {"name": "principal"}},
            {"formula": {"name": "fv", "kind": {
                "formula": "future_value",
                "present_value": {"node": "principal"},
                "rate": 0.05,
                "periods": 10
            }}}
        ],
        "edges": [{"dependency": "principal", "dependent": "fv"}],
        "inputs": {"principal": "100"}
    });

    let resp: serde_json::Value = client()
        .post(format!("{}/evaluate", api.base))
        .header("X-API-Key", API_KEY)
        .json(&body)
        .send()
        .expect("request completes")
        .json()
        .expect("response is JSON");

    let fv = resp["outputs"]["fv"].as_str().expect("fv is a string");
    assert!(
        fv.starts_with("162.889462677744"),
        "unexpected future value: {fv}"
    );
}

/// Series-valued ports must accept a JSON array.
///
/// Regression test: `PortRequest` accepted only a scalar or a node reference,
/// so five formulas could not be called at all. The DAG's documented
/// workaround — a comma-separated string in a scalar port — could never work,
/// because resolving a scalar port yields exactly one `Decimal`.
#[test]
fn series_ports_accept_an_array() {
    let api = ApiServer::start_memory();
    let http = client();

    // Mean of the last three of [10, 12, 14, 16, 18] is 16. A one-element
    // series would fail outright, which is how the original bug surfaced.
    let resp: serde_json::Value = http
        .post(format!("{}/evaluate", api.base))
        .header("X-API-Key", API_KEY)
        .json(&formula_request(&serde_json::json!({
            "formula": "simple_moving_average",
            "prices": [10, 12, 14, 16, 18],
            "window": 3
        })))
        .send()
        .expect("request completes")
        .json()
        .expect("response is JSON");
    assert_eq!(resp["outputs"]["result"].as_str(), Some("16"));

    // Beta needs two series of equal length.
    let resp: serde_json::Value = http
        .post(format!("{}/evaluate", api.base))
        .header("X-API-Key", API_KEY)
        .json(&formula_request(&serde_json::json!({
            "formula": "beta",
            "asset_returns": [0.05, 0.02, -0.01, 0.03],
            "market_returns": [0.03, 0.01, -0.02, 0.02]
        })))
        .send()
        .expect("request completes")
        .json()
        .expect("response is JSON");
    let beta = resp["outputs"]["result"]
        .as_str()
        .expect("beta is a string");
    assert!(beta.starts_with("1.14285"), "unexpected beta: {beta}");
}

/// Every formula in the catalog must be callable and return a value.
///
/// This is the broad net: it would have caught the series-port defect the
/// moment it was introduced, for all five affected formulas at once.
#[test]
fn every_formula_in_the_catalog_is_callable() {
    let api = ApiServer::start_memory();
    let http = client();

    // One representative request per formula. Values are chosen to be valid
    // for the formula's domain, not to be financially meaningful.
    let cases: Vec<serde_json::Value> = vec![
        serde_json::json!({"formula":"future_value","present_value":100,"rate":0.05,"periods":10}),
        serde_json::json!({"formula":"present_value","future_value":162.89,"rate":0.05,"periods":10}),
        serde_json::json!({"formula":"growing_perpetuity_present_value","payment":100,"rate":0.08,"growth_rate":0.03}),
        serde_json::json!({"formula":"continuous_compounding_future_value","present_value":100,"rate":0.05,"time":10}),
        serde_json::json!({"formula":"amortization_payment","principal":300000,"rate":0.004,"periods":360}),
        serde_json::json!({"formula":"return_on_equity","net_income":150000,"equity":1000000}),
        serde_json::json!({"formula":"return_on_investment","gain":150000,"cost":100000}),
        serde_json::json!({"formula":"profit_margin","net_income":150000,"revenue":1000000}),
        serde_json::json!({"formula":"asset_turnover","revenue":1000000,"total_assets":500000}),
        serde_json::json!({"formula":"equity_multiplier","total_assets":2000000,"shareholders_equity":1000000}),
        serde_json::json!({"formula":"quick_ratio","current_assets":500000,"inventory":150000,"current_liabilities":250000}),
        serde_json::json!({"formula":"interest_coverage","ebit":500000,"interest_expense":100000}),
        serde_json::json!({"formula":"inventory_turnover","cogs":750000,"inventory":150000}),
        serde_json::json!({"formula":"cash_conversion_cycle","days_inventory_outstanding":60,"days_sales_outstanding":45,"days_payables_outstanding":30}),
        serde_json::json!({"formula":"altman_z_score","working_capital_to_assets":0.30,"retained_earnings_to_assets":0.20,"ebit_to_assets":0.25,"equity_to_liabilities":2.0,"sales_to_assets":1.5}),
        serde_json::json!({"formula":"wacc","equity_value":700000,"debt_value":300000,"cost_of_equity":0.12,"cost_of_debt":0.06,"tax_rate":0.21}),
        serde_json::json!({"formula":"sustainable_growth_rate","roe":0.15,"dividend_payout_ratio":0.40}),
        serde_json::json!({"formula":"internal_growth_rate","roe":0.15,"dividend_payout_ratio":0.40}),
        serde_json::json!({"formula":"free_cash_flow_to_equity","fcff":500000,"interest_expense_after_tax":80000,"net_borrowing":50000}),
        serde_json::json!({"formula":"economic_value_added","nopat":200000,"invested_capital":1000000,"wacc":0.10}),
        serde_json::json!({"formula":"tax_shield","tax_rate":0.21,"debt":1000000}),
        serde_json::json!({"formula":"adjusted_present_value","unlevered_npv":500000,"pv_tax_shield":210000}),
        serde_json::json!({"formula":"treynor_ratio","portfolio_return":0.12,"risk_free_rate":0.02,"beta":1.2}),
        serde_json::json!({"formula":"sortino_ratio","portfolio_return":0.12,"risk_free_rate":0.02,"downside_deviation":0.08}),
        serde_json::json!({"formula":"calmar_ratio","cagr":0.15,"max_drawdown":0.20}),
        serde_json::json!({"formula":"beta","asset_returns":[0.05,0.02,-0.01,0.03],"market_returns":[0.03,0.01,-0.02,0.02]}),
        serde_json::json!({"formula":"value_at_risk","portfolio_value":1000000,"mean_return":0.08,"std_dev":0.15,"z_score":1.645}),
        serde_json::json!({"formula":"expected_shortfall","portfolio_value":1000000,"mean_return":0.08,"std_dev":0.15,"z_score":1.645}),
        serde_json::json!({"formula":"simple_moving_average","prices":[10,12,14,16,18],"window":3}),
        serde_json::json!({"formula":"yield_to_maturity_approximation","face_value":1000,"coupon_payment":50,"price":950,"periods":10}),
        serde_json::json!({"formula":"discounted_cash_flow","cash_flows":[100,200,300,400],"discount_rate":0.10}),
        serde_json::json!({"formula":"macaulay_duration","cash_flows":[50,50,50,1050],"yield_per_period":0.05}),
        serde_json::json!({"formula":"modified_duration","macaulay_duration":3.72,"yield_per_period":0.05}),
        serde_json::json!({"formula":"convexity","cash_flows":[50,50,50,1050],"yield_per_period":0.05}),
        serde_json::json!({"formula":"capital_adequacy_ratio","total_capital":120000000,"risk_weighted_assets":1000000000}),
        serde_json::json!({"formula":"provision_coverage_ratio","provisions":75000000,"non_performing_assets":100000000}),
        serde_json::json!({"formula":"black_scholes_call","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1}),
        serde_json::json!({"formula":"black_scholes_put","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1}),
        serde_json::json!({"formula":"binomial_option_call","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1,"steps":50}),
        serde_json::json!({"formula":"binomial_option_put","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1,"steps":50}),
        serde_json::json!({"formula":"black_scholes_delta","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1,"style":"call"}),
        serde_json::json!({"formula":"black_scholes_gamma","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1}),
        serde_json::json!({"formula":"black_scholes_vega","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1}),
        serde_json::json!({"formula":"black_scholes_theta","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1,"style":"call"}),
        serde_json::json!({"formula":"black_scholes_rho","spot":100,"strike":100,"risk_free_rate":0.05,"volatility":0.20,"time_to_maturity":1,"style":"call"}),
        serde_json::json!({"formula":"net_present_value","rate":0.10,"cash_flows":[-1000,300,400,500,600]}),
        serde_json::json!({"formula":"internal_rate_of_return","cash_flows":[-1000,300,400,500,600]}),
        serde_json::json!({"formula":"annuity_present_value","payment":100,"rate":0.05,"periods":10}),
        serde_json::json!({"formula":"annuity_future_value","payment":100,"rate":0.05,"periods":10}),
        serde_json::json!({"formula":"perpetuity_present_value","payment":100,"rate":0.05}),
        serde_json::json!({"formula":"effective_annual_rate","nominal_rate":0.12,"compounding_periods":12}),
        serde_json::json!({"formula":"return_on_assets","net_income":150000,"avg_total_assets":2000000}),
        serde_json::json!({"formula":"dupont_roe","profit_margin":0.15,"asset_turnover":2.0,"equity_multiplier":2.0}),
        serde_json::json!({"formula":"current_ratio","current_assets":500000,"current_liabilities":250000}),
        serde_json::json!({"formula":"debt_to_equity","total_liabilities":400000,"shareholders_equity":1000000}),
        serde_json::json!({"formula":"net_interest_margin","interest_income":500000,"interest_expense":200000,"avg_earning_assets":10000000}),
        serde_json::json!({"formula":"loan_to_deposit_ratio","total_loans":800000,"total_deposits":1000000}),
        serde_json::json!({"formula":"sharpe_ratio","portfolio_return":0.12,"risk_free_rate":0.02,"portfolio_std_dev":0.15}),
        serde_json::json!({"formula":"jensens_alpha","portfolio_return":0.12,"risk_free_rate":0.02,"market_return":0.10,"beta":1.2}),
        serde_json::json!({"formula":"dividend_discount_model","next_dividend":2.0,"required_return":0.10,"growth_rate":0.04}),
        serde_json::json!({"formula":"bond_price","face_value":1000,"coupon_payment":50,"yield_per_period":0.04,"periods":10}),
        serde_json::json!({"formula":"free_cash_flow_to_firm","ebit":500000,"tax_rate":0.21,"depreciation":100000,"delta_working_capital":50000,"capex":150000}),
    ];

    let mut failures = Vec::new();
    for case in &cases {
        let name = case["formula"].as_str().unwrap_or("?").to_string();
        let resp = http
            .post(format!("{}/evaluate", api.base))
            .header("X-API-Key", API_KEY)
            .json(&formula_request(case))
            .send()
            .expect("request completes");
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if status != 200 || !text.contains("\"result\"") {
            failures.push(format!("{name} -> {status}: {}", text.trim()));
        }
    }

    assert!(
        failures.is_empty(),
        "{} formula(s) not callable:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );

    // Guard against the catalog growing without this list growing with it.
    assert_eq!(cases.len(), 62, "formula catalog size changed");
}

// ---------------------------------------------------------------------------
// Async job pipeline
// ---------------------------------------------------------------------------

/// A job enqueued through the API must be executed by a separate worker.
///
/// Regression test: `main.rs` hardcoded `InMemoryJobStore` regardless of
/// configuration, so jobs never reached Postgres. The worker reads from
/// Postgres, so it never saw them and every job sat `queued` forever. Nothing
/// errored — the pipeline was simply inert.
#[test]
fn job_enqueued_by_api_is_completed_by_worker() {
    let api = ApiServer::start_postgres();
    assert!(
        api.logged("Using Postgres"),
        "this test is meaningless without the Postgres backend"
    );
    let _worker = Worker::start();
    let http = client();

    let created: serde_json::Value = http
        .post(format!("{}/simulate/jobs", api.base))
        .header("X-API-Key", API_KEY)
        .json(&serde_json::json!({
            "nodes": [
                {"input": {"name": "x"}},
                {"formula": {"name": "fv", "kind": {
                    "formula": "future_value",
                    "present_value": {"node": "x"},
                    "rate": 0.05,
                    "periods": 10
                }}}
            ],
            "edges": [{"dependency": "x", "dependent": "fv"}],
            "bindings": [{"node": "x", "distribution": {"kind": "uniform", "low": 90, "high": 110}}],
            "target": "fv",
            "universe_count": 500,
            "seed": 42
        }))
        .send()
        .expect("enqueue completes")
        .json()
        .expect("response is JSON");

    let id = created["id"].as_str().expect("job id returned").to_string();
    assert_eq!(created["status"].as_str(), Some("queued"));

    // The worker polls every 5 seconds, so allow generous headroom.
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut last = String::new();
    while Instant::now() < deadline {
        let job: serde_json::Value = http
            .get(format!("{}/simulate/jobs/{id}", api.base))
            .header("X-API-Key", API_KEY)
            .send()
            .expect("status request completes")
            .json()
            .expect("response is JSON");
        last = job["status"].as_str().unwrap_or("?").to_string();
        if last == "completed" {
            assert_eq!(job["progress"]["universes_completed"].as_u64(), Some(500));
            assert!(
                (job["progress"]["fraction"].as_f64().unwrap_or(0.0) - 1.0).abs() < f64::EPSILON
            );
            return;
        }
        assert_ne!(last, "failed", "job failed: {job}");
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("job {id} never completed; last status was '{last}'");
}

// ---------------------------------------------------------------------------
// Tenant isolation and audit
// ---------------------------------------------------------------------------

/// Snapshots round-trip and the audit trail records the operations.
#[test]
fn snapshots_round_trip_and_are_audited() {
    let api = ApiServer::start_postgres();
    let http = client();
    let id = format!("smoke-{}", std::process::id());

    let save = http
        .post(format!("{}/snapshots", api.base))
        .header("X-API-Key", API_KEY)
        .json(&serde_json::json!({
            "id": id,
            "nodes": [
                {"input": {"name": "p"}},
                {"formula": {"name": "fv", "kind": {
                    "formula": "future_value",
                    "present_value": {"node": "p"},
                    "rate": 0.07,
                    "periods": 30
                }}}
            ],
            "edges": [{"dependency": "p", "dependent": "fv"}]
        }))
        .send()
        .expect("save completes");
    assert_eq!(save.status(), 200);

    let listed: serde_json::Value = http
        .get(format!("{}/snapshots", api.base))
        .header("X-API-Key", API_KEY)
        .send()
        .expect("list completes")
        .json()
        .expect("response is JSON");
    let found = listed["snapshots"]
        .as_array()
        .expect("snapshots is an array")
        .iter()
        .any(|s| s["id"].as_str() == Some(id.as_str()));
    assert!(found, "saved snapshot missing from listing");

    // The audit trail must have recorded both operations.
    let audit: serde_json::Value = http
        .get(format!("{}/audit?limit=50", api.base))
        .header("X-API-Key", API_KEY)
        .send()
        .expect("audit completes")
        .json()
        .expect("response is JSON");
    let events = audit["events"].as_array().expect("events is an array");
    assert!(
        events.iter().any(|e| e["action"] == "snapshot_create"),
        "no snapshot_create event recorded"
    );
    assert!(
        events.iter().all(|e| e["tenant_id"] == harness::TENANT),
        "audit returned another tenant's events"
    );

    let _ = http
        .delete(format!("{}/snapshots/{id}", api.base))
        .header("X-API-Key", API_KEY)
        .send();
}

/// Admin endpoints require both the API key and the admin key.
#[test]
fn admin_requires_both_keys() {
    let api = ApiServer::start_memory();
    let http = client();
    let url = format!("{}/admin/tenants", api.base);

    // Admin key alone is rejected by the outer auth layer.
    let resp = http
        .get(&url)
        .header("X-Admin-Key", ADMIN_KEY)
        .send()
        .expect("request completes");
    assert_eq!(resp.status(), 401);

    // API key with a wrong admin key is rejected by the admin guard.
    let resp = http
        .get(&url)
        .header("X-API-Key", API_KEY)
        .header("X-Admin-Key", "wrong")
        .send()
        .expect("request completes");
    assert_eq!(resp.status(), 401);

    // Both correct.
    let resp = http
        .get(&url)
        .header("X-API-Key", API_KEY)
        .header("X-Admin-Key", ADMIN_KEY)
        .send()
        .expect("request completes");
    assert_eq!(resp.status(), 200);
}

// ---------------------------------------------------------------------------
// Observability
// ---------------------------------------------------------------------------

/// `/metrics` must expose Prometheus output that reflects real traffic.
#[test]
fn metrics_reflect_served_requests() {
    let api = ApiServer::start_memory();
    let http = client();

    for _ in 0..3 {
        let _ = http
            .post(format!("{}/evaluate", api.base))
            .header("X-API-Key", API_KEY)
            .json(&formula_request(&serde_json::json!({
                "formula": "return_on_equity",
                "net_income": 150000,
                "equity": 1000000
            })))
            .send();
    }

    let body = http
        .get(format!("{}/metrics", api.base))
        .send()
        .expect("metrics request completes")
        .text()
        .expect("metrics body is text");

    assert!(body.contains("casiros_http_requests_total"));
    assert!(body.contains("casiros_http_request_duration_seconds"));
    assert!(
        body.contains("path=\"/evaluate\""),
        "evaluate traffic not reflected in metrics"
    );
}

/// Rate limiting must return 429 once a key's per-minute allowance is spent.
#[test]
fn rate_limit_returns_429() {
    let api = ApiServer::start(&[(
        "CASIROS_API_KEY_TENANTS",
        format!("{API_KEY}:{}:{}:3", harness::TENANT, harness::WORKSPACE),
    )]);
    let http = client();

    let mut statuses = Vec::new();
    for _ in 0..5 {
        let resp = http
            .get(format!("{}/snapshots", api.base))
            .header("X-API-Key", API_KEY)
            .send()
            .expect("request completes");
        statuses.push(resp.status().as_u16());
    }

    assert_eq!(
        &statuses[..3],
        &[200, 200, 200],
        "first three requests should pass: {statuses:?}"
    );
    assert!(
        statuses[3..].iter().all(|s| *s == 429),
        "requests past the limit should be 429: {statuses:?}"
    );
}

/// The `OpenAPI` document must be served and describe the documented routes.
#[test]
fn openapi_spec_is_served() {
    let api = ApiServer::start_memory();
    let spec: serde_json::Value = client()
        .get(format!("{}/openapi.json", api.base))
        .send()
        .expect("spec request completes")
        .json()
        .expect("spec is JSON");

    let paths = spec["paths"].as_object().expect("spec has paths");
    for p in ["/evaluate", "/simulate", "/snapshots", "/audit"] {
        assert!(paths.contains_key(p), "{p} missing from OpenAPI spec");
    }
}
