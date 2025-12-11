/**
 * Phoenix ORCH API Service
 * 
 * Handles communication with the Phoenix ORCH API Gateway
 */

export interface ExecuteResponse {
    result: string;
    metadata: {
        plan: string[];
        routedTo: string;
        payload: Record<string, any>;
        thoughtProcess: string;
    };
}

/**
 * Execute a command through the Phoenix ORCH API
 */
export async function executeCommand(message: string, apiKey?: string): Promise<ExecuteResponse> {
    try {
        const storedApiKey = apiKey || localStorage.getItem('phoenix_orch_api_key') || '';

        const response = await fetch('http://localhost:8282/api/v1/execute', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${storedApiKey}`,
            },
            body: JSON.stringify({ message }),
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => null);
            throw new Error(
                errorData?.error || `Request failed with status ${response.status}`
            );
        }

        return await response.json();
    } catch (error) {
        console.error('API request failed:', error);
        throw error;
    }
}

/**
 * Test if the API key is valid
 */
export async function testApiKey(apiKey: string): Promise<boolean> {
    try {
        const response = await fetch('http://localhost:8282/api/v1/execute', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${apiKey}`,
            },
            body: JSON.stringify({ message: 'test connection' }),
        });

        return response.ok;
    } catch (error) {
        console.error('API key test failed:', error);
        return false;
    }
}