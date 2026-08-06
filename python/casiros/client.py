"""HTTP client for the CASIROS REST API."""

from typing import Any, Dict, Optional

import requests

from .exceptions import CasirosApiError
from .models import (
    AuditListResponse,
    CreateJobRequest,
    CreateJobResponse,
    CreateKeyRequest,
    CreateKeyResponse,
    EvaluateRequest,
    EvaluateResponse,
    JobResponse,
    ProvisionTenantRequest,
    ProvisionTenantResponse,
    SimulateRequest,
    SimulateResponse,
    TenantListResponse,
    TenantStatsResponse,
)


class CasirosClient:
    """Synchronous client for a CASIROS API server.

    Args:
        base_url: Base URL of the CASIROS API, e.g. ``http://localhost:8080``.
        api_key: Optional API key for authenticated endpoints.
        timeout: Request timeout in seconds.
    """

    def __init__(
        self,
        base_url: str,
        api_key: Optional[str] = None,
        timeout: float = 30.0,
    ) -> None:
        self.base_url = base_url.rstrip("/")
        self.api_key = api_key
        self.timeout = timeout

    def _headers(self) -> Dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["Authorization"] = f"Bearer {self.api_key}"
        return headers

    def _request(self, method: str, path: str, **kwargs: Any) -> Any:
        url = f"{self.base_url}{path}"
        response = requests.request(
            method,
            url,
            headers=self._headers(),
            timeout=self.timeout,
            **kwargs,
        )
        if not response.ok:
            try:
                payload = response.json()
                message = payload.get("error", response.text)
            except ValueError:
                message = response.text
            raise CasirosApiError(response.status_code, message)
        if response.status_code == 204:
            return None
        return response.json()

    def healthz(self) -> Dict[str, Any]:
        """Call ``GET /healthz`` and return the health status."""
        return self._request("GET", "/healthz")

    def evaluate(self, request: EvaluateRequest) -> EvaluateResponse:
        """Call ``POST /evaluate`` and return the computed outputs."""
        payload = self._request("POST", "/evaluate", json=request.to_dict())
        return EvaluateResponse.from_dict(payload)

    def simulate(self, request: SimulateRequest) -> SimulateResponse:
        """Call ``POST /simulate`` and return the aggregated result."""
        payload = self._request("POST", "/simulate", json=request.to_dict())
        return SimulateResponse.from_dict(payload)

    def save_snapshot(self, snapshot_id: str, request: EvaluateRequest) -> Dict[str, str]:
        """Call ``POST /snapshots`` to persist a DAG snapshot."""
        return self._request(
            "POST",
            "/snapshots",
            json={"id": snapshot_id, **request.to_dict()},
        )

    def load_snapshot(self, snapshot_id: str) -> Dict[str, Any]:
        """Call ``GET /snapshots/{id}`` to retrieve a snapshot."""
        return self._request("GET", f"/snapshots/{snapshot_id}")

    def delete_snapshot(self, snapshot_id: str) -> None:
        """Call ``DELETE /snapshots/{id}``."""
        self._request("DELETE", f"/snapshots/{snapshot_id}")

    def list_snapshots(self) -> Dict[str, Any]:
        """Call ``GET /snapshots`` to list stored snapshots."""
        return self._request("GET", "/snapshots")

    def create_job(self, request: CreateJobRequest) -> CreateJobResponse:
        """Call ``POST /simulate/jobs`` to enqueue a simulation job."""
        payload = self._request("POST", "/simulate/jobs", json=request.to_dict())
        return CreateJobResponse.from_dict(payload)

    def get_job(self, job_id: str) -> JobResponse:
        """Call ``GET /simulate/jobs/{id}`` to get job status."""
        payload = self._request("GET", f"/simulate/jobs/{job_id}")
        return JobResponse.from_dict(payload)

    def cancel_job(self, job_id: str) -> Dict[str, Any]:
        """Call ``POST /simulate/jobs/{id}/cancel`` to cancel a job."""
        return self._request("POST", f"/simulate/jobs/{job_id}/cancel")

    def list_audit_events(
        self, limit: Optional[int] = None, offset: Optional[int] = None
    ) -> AuditListResponse:
        """Call ``GET /audit`` to list audit events."""
        params: Dict[str, Any] = {}
        if limit is not None:
            params["limit"] = limit
        if offset is not None:
            params["offset"] = offset
        payload = self._request("GET", "/audit", params=params)
        return AuditListResponse.from_dict(payload)

    def get_metrics(self) -> str:
        """Call ``GET /metrics`` to get Prometheus metrics."""
        url = f"{self.base_url}/metrics"
        response = requests.get(url, headers=self._headers(), timeout=self.timeout)
        if not response.ok:
            raise CasirosApiError(response.status_code, response.text)
        return response.text

    def _admin_request(
        self, method: str, path: str, admin_key: str, **kwargs: Any
    ) -> Any:
        """Make an admin API request with the admin key."""
        headers = {"Content-Type": "application/json", "X-Admin-Key": admin_key}
        url = f"{self.base_url}{path}"
        response = requests.request(
            method, url, headers=headers, timeout=self.timeout, **kwargs
        )
        if not response.ok:
            try:
                payload = response.json()
                message = payload.get("error", response.text)
            except ValueError:
                message = response.text
            raise CasirosApiError(response.status_code, message)
        return response.json()

    def list_tenants(self, admin_key: str) -> TenantListResponse:
        """Call ``GET /admin/tenants`` to list tenants."""
        payload = self._admin_request("GET", "/admin/tenants", admin_key)
        return TenantListResponse.from_dict(payload)

    def provision_tenant(
        self, admin_key: str, request: ProvisionTenantRequest
    ) -> ProvisionTenantResponse:
        """Call ``POST /admin/tenants`` to provision a new tenant."""
        payload = self._admin_request(
            "POST", "/admin/tenants", admin_key, json=request.to_dict()
        )
        return ProvisionTenantResponse.from_dict(payload)

    def get_tenant_stats(
        self, admin_key: str, tenant_id: str
    ) -> TenantStatsResponse:
        """Call ``GET /admin/tenants/{id}/stats`` to get tenant stats."""
        payload = self._admin_request(
            "GET", f"/admin/tenants/{tenant_id}/stats", admin_key
        )
        return TenantStatsResponse.from_dict(payload)

    def create_api_key(
        self, admin_key: str, request: CreateKeyRequest
    ) -> CreateKeyResponse:
        """Call ``POST /admin/keys`` to create a new API key."""
        payload = self._admin_request(
            "POST", "/admin/keys", admin_key, json=request.to_dict()
        )
        return CreateKeyResponse.from_dict(payload)

    def revoke_api_key(self, admin_key: str, key_id: str) -> Dict[str, Any]:
        """Call ``POST /admin/keys/{id}/revoke`` to revoke an API key."""
        return self._admin_request(
            "POST", f"/admin/keys/{key_id}/revoke", admin_key
        )
