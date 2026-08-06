# CASIROS Python Client SDK

A lightweight, synchronous Python client for the CASIROS REST API.

## Installation

```bash
pip install -e .
```

For development:

```bash
pip install -e ".[dev]"
```

## Quick start

```python
from casiros import CasirosClient, EvaluateRequest, Node, Edge
from casiros.models import FormulaNode, PortBinding

client = CasirosClient("http://localhost:8080", api_key="your-api-key")

print(client.healthz())

request = EvaluateRequest(
    nodes=[
        Node("principal", "input", None),
        Node(
            "fv",
            "formula",
            FormulaNode(
                "fv",
                "future_value",
                {
                    "present_value": PortBinding(node="principal"),
                    "rate": PortBinding(value="0.05"),
                    "periods": PortBinding(value=10),
                },
            ),
        ),
    ],
    edges=[Edge("principal", "fv")],
    inputs={"principal": "100.0"},
)

response = client.evaluate(request)
print(response.outputs)
```

## Running tests

```bash
pytest
```
