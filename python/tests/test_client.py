"""Unit tests for the CASIROS Python client."""

import pytest
import responses

from casiros import (
    Binding,
    CasirosApiError,
    CasirosClient,
    Distribution,
    Edge,
    EvaluateRequest,
    EvaluateResponse,
    FormulaNode,
    Node,
    PortBinding,
    SimulateRequest,
    SimulateResponse,
)


@responses.activate
def test_healthz_returns_ok():
    responses.add(
        responses.GET,
        "http://localhost:8080/healthz",
        json={"status": "ok"},
        status=200,
    )

    client = CasirosClient("http://localhost:8080")
    result = client.healthz()
    assert result["status"] == "ok"


@responses.activate
def test_evaluate_posts_request_and_parses_response():
    responses.add(
        responses.POST,
        "http://localhost:8080/evaluate",
        json={"outputs": {"fv": "162.8895"}},
        status=200,
    )

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

    client = CasirosClient("http://localhost:8080")
    response = client.evaluate(request)
    assert isinstance(response, EvaluateResponse)
    assert response.outputs["fv"] == "162.8895"


@responses.activate
def test_simulate_posts_request_and_parses_response():
    responses.add(
        responses.POST,
        "http://localhost:8080/simulate",
        json={
            "count": 1000,
            "mean": "10.5",
            "median": "10.2",
            "min": "5.0",
            "max": "20.0",
        },
        status=200,
    )

    request = SimulateRequest(
        nodes=[Node("x", "input", None)],
        edges=[],
        bindings=[
            Binding(
                "x",
                Distribution("uniform", {"low": 0.0, "high": 1.0}),
            )
        ],
        target="x",
        universe_count=1000,
        seed=42,
    )

    client = CasirosClient("http://localhost:8080")
    response = client.simulate(request)
    assert isinstance(response, SimulateResponse)
    assert response.count == 1000
    assert response.mean == "10.5"


@responses.activate
def test_api_error_raises_exception():
    responses.add(
        responses.POST,
        "http://localhost:8080/evaluate",
        json={"error": "bad request"},
        status=400,
    )

    client = CasirosClient("http://localhost:8080")
    with pytest.raises(CasirosApiError) as exc_info:
        client.evaluate(
            EvaluateRequest(nodes=[], edges=[], inputs={})
        )
    assert exc_info.value.status_code == 400
    assert "bad request" in exc_info.value.message


@responses.activate
def test_api_key_is_sent_in_header():
    responses.add(
        responses.GET,
        "http://localhost:8080/healthz",
        json={"status": "ok"},
        status=200,
    )

    client = CasirosClient("http://localhost:8080", api_key="secret")
    client.healthz()
    assert responses.calls[0].request.headers["Authorization"] == "Bearer secret"
