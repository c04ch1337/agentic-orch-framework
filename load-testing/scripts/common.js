import { check, sleep } from 'k6';
import { Counter, Rate, Trend } from 'k6/metrics';
import http from 'k6/http';
import exec from 'k6/execution';

// Custom metrics
export const errorRate = new Rate('error_rate');
export const serviceCallCounter = new Counter('service_calls');
export const serviceLatency = new Trend('service_latency', true);

// Constants and default settings
export const DEFAULT_TIMEOUT = '30s';
export const DEFAULT_HEADERS = {
    'Content-Type': 'application/json',
    'User-Agent': 'phoenix-load-testing'
};

// Helper function to generate a correlation ID
export function generateCorrelationId() {
    return `load-test-${new Date().getTime()}-${Math.floor(Math.random() * 10000)}`;
}

// Helper function to create a service request
export function createServiceRequest(url, method, payload = null, params = {}) {
    const correlationId = generateCorrelationId();
    const headers = {
        ...DEFAULT_HEADERS,
        'X-Correlation-ID': correlationId,
        ...params.headers
    };

    const requestOptions = {
        headers,
        timeout: params.timeout || DEFAULT_TIMEOUT,
        tags: { name: params.name || url }
    };

    let response;
    const startTime = new Date().getTime();

    try {
        if (method.toUpperCase() === 'GET') {
            response = http.get(url, requestOptions);
        } else if (method.toUpperCase() === 'POST') {
            response = http.post(url, payload, requestOptions);
        } else if (method.toUpperCase() === 'PUT') {
            response = http.put(url, payload, requestOptions);
        } else if (method.toUpperCase() === 'DELETE') {
            response = http.del(url, null, requestOptions);
        } else {
            throw new Error(`Unsupported method: ${method}`);
        }

        const endTime = new Date().getTime();
        const duration = endTime - startTime;

        // Record metrics
        serviceCallCounter.add(1, { service: params.service || 'unknown', endpoint: params.endpoint || 'unknown' });
        serviceLatency.add(duration, { service: params.service || 'unknown', endpoint: params.endpoint || 'unknown' });

        // Check response status
        const checkResult = check(response, {
            'status is 2xx': (r) => r.status >= 200 && r.status < 300,
        });

        errorRate.add(!checkResult);

        return {
            response,
            duration,
            success: checkResult,
            correlationId
        };
    } catch (error) {
        console.error(`Request to ${url} failed: ${error.message}`);
        errorRate.add(1);
        return {
            response: null,
            duration: new Date().getTime() - startTime,
            success: false,
            correlationId,
            error
        };
    }
}

// Helper function to log test progress
export function logProgress(message) {
    console.log(`[${new Date().toISOString()}][VU:${exec.vu.idInTest}] ${message}`);
}

// Helper function to sleep with jitter to avoid synchronized requests
export function sleepWithJitter(base, jitter = 0.3) {
    const jitterValue = base * jitter;
    const sleepTime = base + (Math.random() * jitterValue * 2) - jitterValue;
    sleep(sleepTime);
}