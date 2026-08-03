"""HTTP client for the CASIROS REST API."""

from typing import Any, Dict, Optional

import requests

from .exceptions import CasirosApiError
from .models import EvaluateRequest, EvaluateResponse, SimulateRequest, SimulateResponse


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
