//! Integration tests for the CASIROS HTTP API.

use actix_web::{App, test, web};
use casiros_api::handlers;
use casiros_api::streaming_handlers;
use rust_decimal_macros::dec;
use serde_json::json;

#[actix_web::test]
async fn healthz_returns_ok() {
    let app =
        test::init_service(App::new().route("/healthz", web::get().to(handlers::healthz))).await;
    let req = test::TestRequest::get().uri("/healthz").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn evaluate_future_value() {
    let app =
        test::init_service(App::new().route("/evaluate", web::post().to(handlers::evaluate))).await;

    let payload = json!({
        "nodes": [
            { "input": { "name": "principal" } },
            { "input": { "name": "rate" } },
            { "formula": {
                "name": "fv",
                "kind": {
                    "formula": "future_value",
                    "present_value": { "node": "principal" },
                    "rate": { "node": "rate" },
                    "periods": 10
                }
            }}
        ],
        "edges": [
            { "dependency": "principal", "dependent": "fv" },
            { "dependency": "rate", "dependent": "fv" }
        ],
        "inputs": {
            "principal": "100.0",
            "rate": "0.05"
        }
    });

    let req = test::TestRequest::post()
        .uri("/evaluate")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let fv = body["outputs"]["fv"]
        .as_str()
        .unwrap()
        .parse::<rust_decimal::Decimal>()
        .unwrap();
    assert_eq!(fv.round_dp(4), dec!(162.8895));
}

#[actix_web::test]
async fn simulate_future_value_distribution() {
    let app =
        test::init_service(App::new().route("/simulate", web::post().to(handlers::simulate))).await;

    let payload = json!({
        "nodes": [
            { "input": { "name": "principal" } },
            { "input": { "name": "rate" } },
            { "formula": {
                "name": "fv",
                "kind": {
                    "formula": "future_value",
                    "present_value": { "node": "principal" },
                    "rate": { "node": "rate" },
                    "periods": 1
                }
            }}
        ],
        "edges": [
            { "dependency": "principal", "dependent": "fv" },
            { "dependency": "rate", "dependent": "fv" }
        ],
        "bindings": [
            {
                "node": "principal",
                "distribution": { "kind": "uniform", "low": 90.0, "high": 110.0 }
            },
            {
                "node": "rate",
                "distribution": { "kind": "fixed", "value": 0.05 }
            }
        ],
        "target": "fv",
        "universe_count": 200,
        "seed": 42
    });

    let req = test::TestRequest::post()
        .uri("/simulate")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "simulate returned non-success: {:?}",
        resp.status()
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["count"], 200);

    let mean = body["mean"]
        .as_str()
        .unwrap()
        .parse::<rust_decimal::Decimal>()
        .unwrap();
    // Uniform[90, 110] * 1.05 has mean 105.
    assert!(mean > dec!(95.0), "mean too low: {mean}");
    assert!(mean < dec!(115.0), "mean too high: {mean}");
}

#[actix_web::test]
async fn simulate_stream_returns_sse_events_and_final_result() {
    let app = test::init_service(App::new().route(
        "/simulate/stream",
        web::post().to(streaming_handlers::simulate_stream),
    ))
    .await;

    let payload = json!({
        "nodes": [
            { "input": { "name": "x" } },
            {
                "formula": {
                    "name": "doubled",
                    "kind": {
                        "formula": "future_value",
                        "present_value": { "node": "x" },
                        "rate": 0,
                        "periods": 1
                    }
                }
            }
        ],
        "edges": [{ "dependency": "x", "dependent": "doubled" }],
        "bindings": [
            { "node": "x", "distribution": { "kind": "uniform", "low": 0.0, "high": 100.0 } }
        ],
        "target": "doubled",
        "universe_count": 100,
        "seed": 42
    });

    let req = test::TestRequest::post()
        .uri("/simulate/stream")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "streaming simulate returned non-success: {:?}",
        resp.status()
    );

    let content_type = resp.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/event-stream"));

    let body = test::read_body(resp).await;
    let text = String::from_utf8(body.to_vec()).unwrap();
    let events: Vec<&str> = text.split("\n\n").filter(|s| !s.is_empty()).collect();
    assert!(!events.is_empty());

    let last = events.last().unwrap();
    let last_json: serde_json::Value =
        serde_json::from_str(last.strip_prefix("data: ").unwrap_or(last).trim()).unwrap();
    assert_eq!(last_json["type"], "result");
    assert_eq!(last_json["result"]["count"], 100);
}

// ---------------------------------------------------------------------------
// Amortization schedule
//
// The schedule endpoint exists because its result is a table rather than a
// single Decimal, so it cannot be reached through /evaluate like every other
// formula. These tests pin the arithmetic that makes a schedule correct, not
// merely that the route answers.
// ---------------------------------------------------------------------------

/// An app exposing only the schedule route.
///
/// A macro rather than a function: naming the return type of
/// `test::init_service` requires `actix_http::Request`, which is not a direct
/// dependency of this crate.
macro_rules! schedule_app {
    () => {
        test::init_service(App::new().route(
            "/schedule/amortization",
            web::post().to(handlers::amortization_schedule),
        ))
        .await
    };
}

/// A $1,000 loan at 1% a month over 12 months.
///
/// Three things must hold for the schedule to be arithmetically sound: the
/// balance reaches zero, the principal repaid sums to the amount borrowed, and
/// interest falls every period as the balance shrinks.
#[actix_web::test]
async fn amortization_schedule_is_internally_consistent() {
    let app = schedule_app!();

    let req = test::TestRequest::post()
        .uri("/schedule/amortization")
        .set_json(json!({ "principal": "1000.0", "rate": "0.01", "periods": 12 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body["schedule"].as_array().unwrap();
    assert_eq!(rows.len(), 12, "one row per period");

    let dec = |v: &serde_json::Value| {
        v.as_str()
            .unwrap()
            .parse::<rust_decimal::Decimal>()
            .unwrap()
    };

    // Periods are 1-indexed and in order.
    assert_eq!(rows[0]["period"], 1);
    assert_eq!(rows[11]["period"], 12);

    // The loan is fully repaid.
    let final_balance = dec(&rows[11]["remaining_balance"]);
    assert!(
        final_balance.abs() < dec!(0.01),
        "balance should reach zero, got {final_balance}"
    );

    // Principal repaid sums to the amount borrowed.
    let principal_sum: rust_decimal::Decimal = rows.iter().map(|r| dec(&r["principal_paid"])).sum();
    assert!(
        (principal_sum - dec!(1000.0)).abs() < dec!(0.01),
        "principal repaid should sum to 1000, got {principal_sum}"
    );

    // Interest declines monotonically as the outstanding balance falls.
    let interest: Vec<_> = rows.iter().map(|r| dec(&r["interest_paid"])).collect();
    for pair in interest.windows(2) {
        assert!(
            pair[1] < pair[0],
            "interest must fall as the balance amortizes: {pair:?}"
        );
    }

    // First period's interest is exactly one month's rate on the full balance.
    assert_eq!(interest[0].round_dp(2), dec!(10.00));

    // Reported totals must agree with the rows they summarise.
    let total_interest: rust_decimal::Decimal = interest.iter().copied().sum();
    assert_eq!(dec(&body["total_interest"]), total_interest);
    assert_eq!(
        dec(&body["payment"]).round_dp(2),
        (dec(&rows[0]["principal_paid"]) + interest[0]).round_dp(2)
    );
}

/// The period cap is the only lever a caller has to make the response
/// arbitrarily large, so it must be rejected with a message naming the limit.
#[actix_web::test]
async fn amortization_schedule_rejects_too_many_periods() {
    let app = schedule_app!();

    let req = test::TestRequest::post()
        .uri("/schedule/amortization")
        .set_json(json!({ "principal": "1000.0", "rate": "0.01", "periods": 5000 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let err = body["error"].as_str().unwrap();
    assert!(
        err.contains("5000") && err.contains("1000"),
        "error should name both the request and the limit, got: {err}"
    );
}

/// A negative principal is not a loan. The core crate rejects it; this pins
/// that the rejection surfaces as a 400 rather than a 500.
#[actix_web::test]
async fn amortization_schedule_rejects_negative_principal() {
    let app = schedule_app!();

    let req = test::TestRequest::post()
        .uri("/schedule/amortization")
        .set_json(json!({ "principal": "-1000.0", "rate": "0.01", "periods": 12 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

/// Zero periods is legal and yields an empty schedule. The handler reads the
/// level payment from the first row, so this is the case that would panic on
/// an unguarded index.
#[actix_web::test]
async fn amortization_schedule_handles_zero_periods() {
    let app = schedule_app!();

    let req = test::TestRequest::post()
        .uri("/schedule/amortization")
        .set_json(json!({ "principal": "1000.0", "rate": "0.01", "periods": 0 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["schedule"].as_array().unwrap().is_empty());
    assert_eq!(body["payment"].as_str().unwrap(), "0");
}

/// A zero-interest loan repays in equal slices with no interest at all.
#[actix_web::test]
async fn amortization_schedule_handles_zero_rate() {
    let app = schedule_app!();

    let req = test::TestRequest::post()
        .uri("/schedule/amortization")
        .set_json(json!({ "principal": "1200.0", "rate": "0", "periods": 12 }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let dec = |v: &serde_json::Value| {
        v.as_str()
            .unwrap()
            .parse::<rust_decimal::Decimal>()
            .unwrap()
    };
    assert_eq!(dec(&body["total_interest"]), dec!(0));
    assert_eq!(dec(&body["payment"]).round_dp(2), dec!(100.00));
}
