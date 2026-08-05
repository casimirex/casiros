"""Typed request and response models for the CASIROS Python client."""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional


@dataclass
class PortBinding:
    """A port binding: either a literal constant or a reference to another node."""

    value: Any = None
    node: Optional[str] = None

    def to_dict(self) -> Any:
        """Serialize the binding for a JSON request."""
        if self.node is not None:
            return {"node": self.node}
        return self.value

    @classmethod
    def from_dict(cls, data: Any) -> "PortBinding":
        """Deserialize a binding from JSON."""
        if isinstance(data, dict):
            return cls(node=data.get("node"))
        return cls(value=data)


@dataclass
class FormulaNode:
    """A formula node in a DAG request."""

    name: str
    formula: str
    inputs: Dict[str, PortBinding] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        """Serialize the formula node for a JSON request."""
        return {
            "formula": self.formula,
            **{key: binding.to_dict() for key, binding in self.inputs.items()},
        }


@dataclass
class Node:
    """A single node in a DAG request."""

    name: str
    kind: str
    data: Any

    def to_dict(self) -> Dict[str, Any]:
        """Serialize the node for a JSON request."""
        if self.kind == "input":
            return {"input": {"name": self.name}}
        if self.kind == "formula":
            formula = self.data
            return {
                "formula": {
                    "name": self.name,
                    "kind": formula.to_dict(),
                }
            }
        raise ValueError(f"unknown node kind: {self.kind}")


@dataclass
class Edge:
    """A directed edge between two nodes."""

    dependency: str
    dependent: str

    def to_dict(self) -> Dict[str, str]:
        return {"dependency": self.dependency, "dependent": self.dependent}


@dataclass
class EvaluateRequest:
    """Request body for the ``POST /evaluate`` endpoint."""

    nodes: List[Node]
    edges: List[Edge]
    inputs: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "nodes": [node.to_dict() for node in self.nodes],
            "edges": [edge.to_dict() for edge in self.edges],
            "inputs": self.inputs,
        }


@dataclass
class EvaluateResponse:
    """Response body from the ``POST /evaluate`` endpoint."""

    outputs: Dict[str, str]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "EvaluateResponse":
        return cls(outputs=data.get("outputs", {}))


@dataclass
class Distribution:
    """A distribution for a Monte Carlo simulation input binding."""

    kind: str
    parameters: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {"kind": self.kind, **self.parameters}


@dataclass
class Binding:
    """An input-to-distribution binding for simulation."""

    node: str
    distribution: Distribution

    def to_dict(self) -> Dict[str, Any]:
        return {"node": self.node, "distribution": self.distribution.to_dict()}


@dataclass
class SimulateRequest:
    """Request body for the ``POST /simulate`` endpoint."""

    nodes: List[Node]
    edges: List[Edge]
    bindings: List[Binding]
    target: str
    universe_count: int
    seed: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "nodes": [node.to_dict() for node in self.nodes],
            "edges": [edge.to_dict() for edge in self.edges],
            "bindings": [binding.to_dict() for binding in self.bindings],
            "target": self.target,
            "universe_count": self.universe_count,
        }
        if self.seed is not None:
            payload["seed"] = self.seed
        return payload


@dataclass
class SimulateResponse:
    """Response body from the ``POST /simulate`` endpoint."""

    count: int
    mean: str
    median: str
    min_value: str
    max_value: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "SimulateResponse":
        return cls(
            count=data["count"],
            mean=data["mean"],
            median=data["median"],
            min_value=data["min"],
            max_value=data["max"],
        )

@dataclass
class CreateJobRequest:
    """Request body for the ``POST /simulate/jobs`` endpoint."""

    nodes: List[Node]
    edges: List[Edge]
    bindings: List[Binding]
    target: str
    universe_count: int
    seed: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "nodes": [node.to_dict() for node in self.nodes],
            "edges": [edge.to_dict() for edge in self.edges],
            "bindings": [binding.to_dict() for binding in self.bindings],
            "target": self.target,
            "universe_count": self.universe_count,
        }
        if self.seed is not None:
            payload["seed"] = self.seed
        return payload


@dataclass
class CreateJobResponse:
    """Response body from the ``POST /simulate/jobs`` endpoint."""

    id: str
    status: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CreateJobResponse":
        return cls(id=data["id"], status=data["status"])


@dataclass
class JobProgressResponse:
    """Progress summary returned in job responses."""

    universes_total: int
    universes_completed: int
    fraction: float

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "JobProgressResponse":
        return cls(
            universes_total=data["universes_total"],
            universes_completed=data["universes_completed"],
            fraction=data["fraction"],
        )


@dataclass
class JobResponse:
    """Response body from the ``GET /simulate/jobs/{id}`` endpoint."""

    id: str
    status: str
    progress: JobProgressResponse
    result: Optional[Any] = None
    error: Optional[str] = None
    created_at: str = ""
    updated_at: str = ""

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "JobResponse":
        return cls(
            id=data["id"],
            status=data["status"],
            progress=JobProgressResponse.from_dict(data.get("progress", {})),
            result=data.get("result"),
            error=data.get("error"),
            created_at=data.get("created_at", ""),
            updated_at=data.get("updated_at", ""),
        )


@dataclass
class AuditEventResponse:
    """A single audit event returned by ``GET /audit``."""

    id: str
    timestamp: str
    tenant_id: str
    workspace_id: str
    api_key_id: str
    action: str
    resource: str
    result: str
    error: Optional[str] = None
    metadata: Dict[str, str] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "AuditEventResponse":
        return cls(
            id=data["id"],
            timestamp=data["timestamp"],
            tenant_id=data["tenant_id"],
            workspace_id=data["workspace_id"],
            api_key_id=data["api_key_id"],
            action=data["action"],
            resource=data["resource"],
            result=data["result"],
            error=data.get("error"),
            metadata=data.get("metadata", {}),
        )


@dataclass
class AuditListResponse:
    """Response body from the ``GET /audit`` endpoint."""

    total: int
    events: List[AuditEventResponse]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "AuditListResponse":
        return cls(
            total=data["total"],
            events=[AuditEventResponse.from_dict(e) for e in data.get("events", [])],
        )


@dataclass
class TenantSummary:
    """A single tenant summary."""

    id: str
    name: str
    plan: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TenantSummary":
        return cls(id=data["id"], name=data["name"], plan=data["plan"])


@dataclass
class TenantListResponse:
    """Response body from the ``GET /admin/tenants`` endpoint."""

    tenants: List[TenantSummary]

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TenantListResponse":
        return cls(
            tenants=[TenantSummary.from_dict(t) for t in data.get("tenants", [])]
        )


@dataclass
class ProvisionTenantRequest:
    """Request body for the ``POST /admin/tenants`` endpoint."""

    id: str
    name: Optional[str] = None
    plan: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        payload: Dict[str, Any] = {"id": self.id}
        if self.name is not None:
            payload["name"] = self.name
        if self.plan is not None:
            payload["plan"] = self.plan
        return payload


@dataclass
class ProvisionTenantResponse:
    """Response body from the ``POST /admin/tenants`` endpoint."""

    id: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "ProvisionTenantResponse":
        return cls(id=data["id"])


@dataclass
class TenantStatsResponse:
    """Response body from the ``GET /admin/tenants/{id}/stats`` endpoint."""

    tenant_id: str
    audit_events: int
    simulation_jobs: int
    snapshots: int

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TenantStatsResponse":
        return cls(
            tenant_id=data["tenant_id"],
            audit_events=data["audit_events"],
            simulation_jobs=data["simulation_jobs"],
            snapshots=data["snapshots"],
        )


@dataclass
class CreateKeyRequest:
    """Request body for the ``POST /admin/keys`` endpoint."""

    tenant_id: str
    workspace_id: str
    name: Optional[str] = None
    rate_limit_rpm: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        payload: Dict[str, Any] = {
            "tenant_id": self.tenant_id,
            "workspace_id": self.workspace_id,
        }
        if self.name is not None:
            payload["name"] = self.name
        if self.rate_limit_rpm is not None:
            payload["rate_limit_rpm"] = self.rate_limit_rpm
        return payload


@dataclass
class CreateKeyResponse:
    """Response body from the ``POST /admin/keys`` endpoint."""

    id: str
    key: str

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "CreateKeyResponse":
        return cls(id=data["id"], key=data["key"])
