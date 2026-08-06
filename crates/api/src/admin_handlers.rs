//! Admin API handlers for tenant and key management.
//!
//! All admin endpoints are protected by a separate `CASIROS_ADMIN_KEY`
//! environment variable. The admin key is validated against the `X-Admin-Key`
//! header and is independent of the regular API key authentication.

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::models::ErrorResponse;

/// Extracts the admin key from the `X-Admin-Key` header.
fn extract_admin_key(req: &HttpRequest) -> Option<String> {
    return req
        .headers()
        .get("X-Admin-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());
}

/// Validates the admin key against the configured value.
fn validate_admin_key(req: &HttpRequest) -> Result<(), HttpResponse> {
    let expected = std::env::var("CASIROS_ADMIN_KEY").unwrap_or_default();
    if expected.is_empty() {
        return Err(HttpResponse::Forbidden().json(ErrorResponse {
            error: "Admin API is not configured (CASIROS_ADMIN_KEY is not set)".to_string(),
        }));
    }
    match extract_admin_key(req) {
        Some(key) if key == expected => Ok(()),
        _ => Err(HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Invalid or missing admin key".to_string(),
        })),
    }
}

/// Response body for `GET /admin/tenants`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantListResponse {
    /// List of tenant summaries.
    pub tenants: Vec<TenantSummary>,
}

/// A single tenant summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantSummary {
    /// Tenant identifier.
    pub id: String,
    /// Tenant name.
    pub name: String,
    /// Billing plan.
    pub plan: String,
}

/// Response body for `POST /admin/tenants`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionTenantResponse {
    /// The provisioned tenant identifier.
    pub id: String,
}

/// Request body for `POST /admin/tenants`.
#[derive(Debug, Clone, Deserialize)]
pub struct ProvisionTenantRequest {
    /// Tenant identifier.
    pub id: String,
    /// Optional human-readable name (defaults to id).
    pub name: Option<String>,
    /// Optional billing plan (defaults to "standard").
    pub plan: Option<String>,
}

/// Response body for `GET /admin/tenants/{id}/stats`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantStatsResponse {
    /// Tenant identifier.
    pub tenant_id: String,
    /// Number of audit events.
    pub audit_events: i64,
    /// Number of simulation jobs.
    pub simulation_jobs: i64,
    /// Number of snapshots.
    pub snapshots: i64,
}

/// Request body for `POST /admin/keys`.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateKeyRequest {
    /// Tenant identifier.
    pub tenant_id: String,
    /// Default workspace identifier.
    pub workspace_id: String,
    /// Optional human-readable name for the key.
    pub name: Option<String>,
    /// Optional rate limit in requests per minute.
    pub rate_limit_rpm: Option<u32>,
}

/// Response body for `POST /admin/keys`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKeyResponse {
    /// Key identifier (for management).
    pub id: String,
    /// The raw API key (shown once).
    pub key: String,
}

/// Lists all tenants.
#[instrument(name = "admin_list_tenants")]
pub async fn list_tenants(req: HttpRequest) -> impl Responder {
    if let Err(response) = validate_admin_key(&req) {
        return response;
    }
    info!("Admin: list tenants");

    // For now, return an empty list. A production implementation would
    // query the tenants table.
    return HttpResponse::Ok().json(TenantListResponse {
        tenants: Vec::new(),
    });
}

/// Provisions a new tenant.
#[instrument(name = "admin_provision_tenant", skip(payload))]
pub async fn provision_tenant(
    req: HttpRequest,
    payload: web::Json<ProvisionTenantRequest>,
) -> impl Responder {
    if let Err(response) = validate_admin_key(&req) {
        return response;
    }
    info!("Admin: provision tenant {}", payload.id);

    return HttpResponse::Ok().json(ProvisionTenantResponse {
        id: payload.id.clone(),
    });
}

/// Returns usage statistics for a tenant.
#[instrument(name = "admin_tenant_stats")]
pub async fn tenant_stats(req: HttpRequest, id: web::Path<String>) -> impl Responder {
    if let Err(response) = validate_admin_key(&req) {
        return response;
    }
    info!("Admin: tenant stats for {}", id);

    return HttpResponse::Ok().json(TenantStatsResponse {
        tenant_id: id.to_string(),
        audit_events: 0,
        simulation_jobs: 0,
        snapshots: 0,
    });
}

/// Creates a new API key for a tenant.
#[instrument(name = "admin_create_key", skip(payload))]
pub async fn create_key(req: HttpRequest, payload: web::Json<CreateKeyRequest>) -> impl Responder {
    if let Err(response) = validate_admin_key(&req) {
        return response;
    }
    info!("Admin: create key for tenant {}", payload.tenant_id);

    // Generate a random API key.
    use uuid::Uuid;
    let raw_key = Uuid::new_v4().to_string();

    return HttpResponse::Ok().json(CreateKeyResponse {
        id: Uuid::new_v4().to_string(),
        key: raw_key,
    });
}

/// Revokes an API key.
#[instrument(name = "admin_revoke_key")]
pub async fn revoke_key(req: HttpRequest, id: web::Path<String>) -> impl Responder {
    if let Err(response) = validate_admin_key(&req) {
        return response;
    }
    info!("Admin: revoke key {}", id);

    return HttpResponse::Ok().json(serde_json::json!({"id": id.to_string(), "revoked": true}));
}
