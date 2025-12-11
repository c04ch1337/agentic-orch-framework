export interface ExecuteMetadata {
    session_id?: string;
    [key: string]: unknown;
}

export interface ExecuteRequestBody {
    id?: string;
    method: "orchestrated_chat";
    payload: string;
    metadata?: ExecuteMetadata;
}

export interface ExecuteResponseBody {
    final_answer?: string;
    execution_plan?: unknown;
    [key: string]: unknown;
}

const EXECUTE_ENDPOINT = "http://localhost:8282/api/v1/execute";

export async function sendExecuteRequest(
    message: string,
    apiKey: string,
    authToken: string,
    metadata?: ExecuteMetadata
): Promise<ExecuteResponseBody> {
    if (!apiKey) {
        throw new Error("Phoenix API key is required");
    }

    if (!authToken) {
        throw new Error("Authorization token is required");
    }

    const body: ExecuteRequestBody = {
        method: "orchestrated_chat",
        payload: message,
        metadata: metadata ? { ...metadata } : undefined,
    };

    if (metadata && metadata.session_id && !body.id) {
        body.id = String(metadata.session_id);
    }

    const headers: HeadersInit = {
        "Content-Type": "application/json",
        "X-PHOENIX-API-KEY": apiKey,
        Authorization: `Bearer ${authToken}`,
    };

    console.debug("[sendExecuteRequest] Sending request", {
        url: EXECUTE_ENDPOINT,
        hasApiKey: !!apiKey,
        hasAuthToken: !!authToken,
        body,
    });

    let response: Response;

    try {
        response = await fetch(EXECUTE_ENDPOINT, {
            method: "POST",
            headers,
            body: JSON.stringify(body),
        });
    } catch (err) {
        console.error("[sendExecuteRequest] Network error", err);
        throw new Error(
            err instanceof Error
                ? `Network error while calling execute endpoint: ${err.message}`
                : "Unknown network error while calling execute endpoint"
        );
    }

    if (!response.ok) {
        let errorText: string | undefined;

        try {
            errorText = await response.text();
        } catch (err) {
            console.error("[sendExecuteRequest] Failed to read error response", err);
        }

        console.error("[sendExecuteRequest] Non-OK response", {
            status: response.status,
            statusText: response.statusText,
            body: errorText,
        });

        throw new Error(
            `Execute request failed with status ${response.status} ${response.statusText}${errorText ? `: ${errorText}` : ""
            }`
        );
    }

    let data: unknown;

    try {
        data = await response.json();
    } catch (err) {
        console.error("[sendExecuteRequest] Failed to parse JSON response", err);
        throw new Error("Failed to parse JSON response from execute endpoint");
    }

    if (typeof data !== "object" || data === null || Array.isArray(data)) {
        console.error("[sendExecuteRequest] Unexpected response shape", data);
        throw new Error("Execute endpoint returned an unexpected response shape");
    }

    console.debug("[sendExecuteRequest] Received response", data);

    return data as ExecuteResponseBody;
}