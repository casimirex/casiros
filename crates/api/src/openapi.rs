//! `OpenAPI` documentation and Swagger UI serving.
//!
//! This module builds the `OpenAPI` contract from the `utoipa` annotations on the
//! request/response models and HTTP handlers. The resulting JSON document is
//! served at `/openapi.json` and rendered interactively at `/swagger-ui`.

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers;
use crate::models;

/// The complete CASIROS REST API `OpenAPI` contract.
///
/// This derive collects all `#[utoipa::path(...)]` handler annotations and all
/// `#[derive(ToSchema)]` types referenced by those paths.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "CASIROS API",
        version = "0.1.0",
        description = "NASA/JPL-grade Financial Physics Engine & Multiverse Simulator"
    ),
    paths(handlers::healthz, handlers::evaluate, handlers::simulate,),
    components(schemas(
        models::HealthzResponse,
        models::EvaluateRequest,
        models::EvaluateResponse,
        models::SimulateRequest,
        models::SimulateResponse,
        models::ErrorResponse,
        models::NodeRequest,
        models::FormulaRequest,
        models::PortRequest,
        models::EdgeRequest,
        models::DistributionRequest,
        models::BindingRequest,
    ))
)]
pub struct ApiDoc;

/// Returns an Actix-Web service config that mounts the Swagger UI at `/swagger-ui`.
///
/// The `OpenAPI` JSON contract is served automatically at `/openapi.json`.
#[must_use]
pub fn swagger_ui() -> SwaggerUi {
    return SwaggerUi::new("/swagger-ui/{_:.*}").url("/openapi.json", ApiDoc::openapi());
}

/// Returns the `OpenAPI` document as a pretty-printed JSON string.
///
/// This is useful for snapshot tests and for generating static client specs.
#[must_use]
pub fn spec_pretty() -> String {
    return ApiDoc::openapi().to_pretty_json().unwrap_or_default();
}
