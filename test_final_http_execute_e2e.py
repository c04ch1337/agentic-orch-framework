import os
import json
import time

import pytest
import requests

from test_api_schema import validate_agi_response_schema

BASE_URL = os.environ.get("PHOENIX_API_BASE_URL", "http://localhost:8000")
DEFAULT_API_KEY = os.environ.get("PHOENIX_API_KEY", "phoenix-default-key-2024")
TEST_BEARER_TOKEN_ENV = "PHOENIX_E2E_EXECUTE_TOKEN"


def require_execute_token() -> str:
    token = os.environ.get(TEST_BEARER_TOKEN_ENV)
    if not token:
        pytest.skip(
            f"Environment variable {TEST_BEARER_TOKEN_ENV} must be set to a token "
            "with execute:invoke permission to run the final HTTP E2E test"
        )
    return token


@pytest.mark.e2e
def test_final_http_execute_unified_schema():
    """
    HTTP-level E2E test for /api/v1/execute using the legacy ProcessRequest stub.

    This test verifies that the API Gateway exposes the unified AgiResponse schema
    over HTTP when the request does NOT opt into PlanAndExecute orchestration.
    In this mode, the gateway invokes OrchestratorService.ProcessRequest with
    service="orchestrator", and the orchestrator returns a simple stubbed response.

    Important architectural note:
    - Only requests that explicitly opt into PlanAndExecute (via method or
      metadata) go through the full orchestration pipeline.
    - This test validates schema integrity and basic plumbing for the
      non-orchestrated code path (no Tools, no multi-stage orchestration).
    """
    token = require_execute_token()

    url = f"{BASE_URL}/api/v1/execute"

    execute_id = "final-e2e-http-001"

    payload_text = (
        "What is the primary function of the Data Router service, and then write a "
        "simple Python function to log a message."
    )

    request_body = {
        "id": execute_id,
        # Use a non-plan method name so the gateway routes to ProcessRequest stub
        "method": "simple_chat",
        "payload": payload_text,
        "metadata": {
            "response_format": "agi_response",
            "test_scenario": "http_orchestrator_stub",
            "timestamp": str(int(time.time())),
        },
    }

    headers = {
        "Content-Type": "application/json",
        "X-PHOENIX-API-KEY": DEFAULT_API_KEY,
        "Authorization": f"Bearer {token}",
    }

    response = requests.post(url, headers=headers, json=request_body, timeout=15)

    assert response.status_code == 200, (
        f"Expected 200 from /api/v1/execute, got {response.status_code}: "
        f"{response.text}"
    )

    try:
        data = response.json()
    except json.JSONDecodeError as exc:
        pytest.fail(f"Response from /api/v1/execute is not valid JSON: {exc}\n{response.text}")

    validation_errors = validate_agi_response_schema(data)
    assert not validation_errors, (
        "Unified AgiResponse schema validation failed:\n"
        + "\n".join(f"- {err}" for err in validation_errors)
        + f"\n\nActual response:\n{json.dumps(data, indent=2)}"
    )

    assert data.get("routed_service") == "orchestrator", (
        "When the request does not opt into PlanAndExecute, /api/v1/execute should "
        'route to OrchestratorService.ProcessRequest with service="orchestrator".'
    )
    assert data.get("phoenix_session_id") == execute_id, (
        "phoenix_session_id should echo the request id for unified tracing."
    )

    assert isinstance(data.get("final_answer"), str) and data["final_answer"], (
        "final_answer should be a non-empty string"
    )
    assert isinstance(data.get("execution_plan"), str) and data["execution_plan"], (
        "execution_plan should be a non-empty string"
    )
    assert isinstance(data.get("output_artifact_urls"), list), (
        "output_artifact_urls should be a list (may be empty)"
    )


@pytest.mark.e2e
def test_final_http_execute_plan_and_execute_mode():
    """
    HTTP-level E2E test for /api/v1/execute using PlanAndExecute orchestration.

    This test verifies that callers can opt into the orchestrated PlanAndExecute
    path while preserving the unified AgiResponse schema, and that the response
    is attributed to the final LLM service.

    Request shape mirrors the canonical gRPC PlanAndExecute query, but routed
    through the API Gateway.
    """
    token = require_execute_token()

    url = f"{BASE_URL}/api/v1/execute"

    execute_id = "final-e2e-http-plan-001"

    payload_text = (
        "What is the primary function of the Data Router service, and then write a "
        "simple Python function to log a message."
    )

    request_body = {
        "id": execute_id,
        "method": "plan_and_execute",
        "payload": payload_text,
        "metadata": {
            "orchestration_mode": "plan_and_execute",
            "tool_preference": "auto",
            "response_format": "agi_response",
            "test_scenario": "http_plan_and_execute",
            "timestamp": str(int(time.time())),
        },
    }

    headers = {
        "Content-Type": "application/json",
        "X-PHOENIX-API-KEY": DEFAULT_API_KEY,
        "Authorization": f"Bearer {token}",
    }

    response = requests.post(url, headers=headers, json=request_body, timeout=30)

    assert response.status_code == 200, (
        f"Expected 200 from /api/v1/execute in PlanAndExecute mode, got {response.status_code}: "
        f"{response.text}"
    )

    try:
        data = response.json()
    except json.JSONDecodeError as exc:
        pytest.fail(
            "Response from /api/v1/execute (PlanAndExecute mode) is not valid JSON: "
            f"{exc}\n{response.text}"
        )

    validation_errors = validate_agi_response_schema(data)
    assert not validation_errors, (
        "Unified AgiResponse schema validation failed for PlanAndExecute mode:\n"
        + "\n".join(f"- {err}" for err in validation_errors)
        + f"\n\nActual response:\n{json.dumps(data, indent=2)}"
    )

    # In the orchestrated path, the final routed_service should be the LLM.
    assert data.get("routed_service") == "llm-service", (
        "PlanAndExecute over HTTP should ultimately route to llm-service for this query; "
        f"got routed_service={data.get('routed_service')!r}"
    )

    # phoenix_session_id should echo the HTTP id for unified tracing.
    assert data.get("phoenix_session_id") == execute_id, (
        "phoenix_session_id should echo the HTTP request id for PlanAndExecute mode."
    )

    # Basic sanity checks on content
    assert isinstance(data.get("final_answer"), str) and data["final_answer"], (
        "final_answer should be a non-empty string in PlanAndExecute mode"
    )
    assert isinstance(data.get("execution_plan"), str) and data["execution_plan"], (
        "execution_plan should be a non-empty string in PlanAndExecute mode"
    )
    assert isinstance(data.get("output_artifact_urls"), list), (
        "output_artifact_urls should be a list (may be empty) in PlanAndExecute mode"
    )