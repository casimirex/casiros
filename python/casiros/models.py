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
