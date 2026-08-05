# Getting Started

## Installation

### Docker Compose (recommended)

```bash
git clone https://github.com/casimirex/casiros.git
cd casiros
docker compose up -d
```

This starts the API server on port 8080, a PostgreSQL database, and a Redis
cache. The worker container starts automatically and polls for jobs.

### From Source

```bash
git clone https://github.com/casimirex/casiros.git
cd casiros
cargo run -p casiros-api
```

Requires Rust 1.85+ and a running PostgreSQL instance.

### Python SDK

```bash
cd python
pip install -e .
```

## Quick Start

### 1. Health Check

```bash
curl http://localhost:8080/healthz
```

Expected response: `{"status":"ok"}`

### 2. Evaluate a DAG

Evaluate a simple future value computation:

```bash
curl -X POST http://localhost:8080/evaluate \
  -H "Content-Type: application/json" \
  -d '{
    "nodes": [
      {"input": {"name": "principal"}},
      {"formula": {
        "name": "fv",
        "kind": {
          "formula": "future_value",
          "present_value": {"node": "principal"},
          "rate": 0.05,
          "periods": 10
        }
      }}
    ],
    "edges": [{"dependency": "principal", "dependent": "fv"}],
    "inputs": {"principal": "100"}
  }'
```

### 3. Run a Monte Carlo Simulation

```bash
curl -X POST http://localhost:8080/simulate \
  -H "Content-Type: application/json" \
  -d '{
    "nodes": [
      {"input": {"name": "x"}},
      {"formula": {
        "name": "doubled",
        "kind": {
          "formula": "future_value",
          "present_value": {"node": "x"},
          "rate": 0,
          "periods": 1
        }
      }}
    ],
    "edges": [{"dependency": "x", "dependent": "doubled"}],
    "bindings": [
      {"node": "x", "distribution": {"kind": "uniform", "low": 0, "high": 100}}
    ],
    "target": "doubled",
    "universe_count": 1000,
    "seed": 42
  }'
```

### 4. Enqueue an Async Job

```bash
curl -X POST http://localhost:8080/simulate/jobs \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-key" \
  -d '{
    "nodes": [
      {"input": {"name": "x"}},
      {"formula": {
        "name": "doubled",
        "kind": {
          "formula": "future_value",
          "present_value": {"node": "x"},
          "rate": 0,
          "periods": 1
        }
      }}
    ],
    "edges": [{"dependency": "x", "dependent": "doubled"}],
    "bindings": [
      {"node": "x", "distribution": {"kind": "uniform", "low": 0, "high": 100}}
    ],
    "target": "doubled",
    "universe_count": 10000,
    "seed": 42
  }'
```

Returns a job ID. Poll for results:

```bash
curl http://localhost:8080/simulate/jobs/{job-id} \
  -H "X-API-Key: your-key"
```

### 5. Using the Python SDK

```python
from casiros import CasirosClient, Node, Edge, EvaluateRequest

client = CasirosClient("http://localhost:8080", api_key="your-key")

# Health check
print(client.healthz())

# Evaluate a DAG
request = EvaluateRequest(
    nodes=[Node("x", "input", None), Node("y", "formula", ...)],
    edges=[Edge("x", "y")],
    inputs={"x": "100"},
)
response = client.evaluate(request)
print(response.outputs)
```

## Next Steps

- Explore the [API Reference](api.md) for all available endpoints.
- Read the [Deployment Guide](deployment.md) for production configuration.
- See the [Architecture Overview](architecture.md) for the system design.
