import { sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
import { Faker } from 'https://jslib.k6.io/faker/5.5.3/index.js';
import {
    createServiceRequest,
    logProgress,
    sleepWithJitter,
    errorRate,
    serviceCallCounter,
    serviceLatency
} from '../scripts/common.js';

// Initialize faker
const faker = new Faker({ locale: 'en' });

// Custom metrics
const orchestratorErrorRate = new Rate('orchestrator_error_rate');
const orchestratorLatency = new Trend('orchestrator_latency', true);
const dataRouterLatency = new Trend('data_router_latency', true);
const llmServiceLatency = new Trend('llm_service_latency', true);

// Configuration
const TARGET_URL = __ENV.TARGET_URL || 'http://orchestrator-service:50051';
const DATA_ROUTER_URL = __ENV.DATA_ROUTER_URL || 'http://data-router:50052';
const LLM_SERVICE_URL = __ENV.LLM_SERVICE_URL || 'http://llm-service:50053';

// Test configuration
export const options = {
    stages: [
        { duration: __ENV.RAMP_TIME || '5s', target: __ENV.VUS || 10 },
        { duration: __ENV.DURATION || '30s', target: __ENV.VUS || 10 },
        { duration: __ENV.RAMP_TIME || '5s', target: 0 }
    ],
    thresholds: {
        http_req_failed: ['rate<' + (__ENV.THRESHOLD_HTTP_FAIL || 0.01)],
        http_req_duration: ['p(95)<' + (__ENV.THRESHOLD_HTTP_RESPONSE || 2000)],
        'orchestrator_error_rate': ['rate<0.01'],
        'orchestrator_latency{endpoint:health}': ['p(95)<500'],
        'orchestrator_latency{endpoint:process}': ['p(95)<2000']
    }
};

// Main function executed by k6 for each virtual user
export default function () {
    // Test 1: Health check
    let healthCheck = testOrchestratorHealth();

    // Sleep between requests
    sleepWithJitter(1);

    // Test 2: Basic processing request
    let processingRequest = testOrchestratorProcessing();

    // Record additional metrics based on the test results
    if (!healthCheck.success) {
        orchestratorErrorRate.add(1, { endpoint: 'health' });
    }

    if (!processingRequest.success) {
        orchestratorErrorRate.add(1, { endpoint: 'process' });
    }

    // Sleep between VU iterations
    sleep(1);
}

// Test the health endpoint of the orchestrator service
function testOrchestratorHealth() {
    logProgress('Testing Orchestrator Health');

    const url = `${TARGET_URL}/health`;
    const result = createServiceRequest(
        url,
        'GET',
        null,
        {
            service: 'orchestrator',
            endpoint: 'health',
            name: 'Orchestrator Health Check'
        }
    );

    orchestratorLatency.add(result.duration, { endpoint: 'health' });

    return result;
}

// Test the processing endpoint of the orchestrator service
function testOrchestratorProcessing() {
    logProgress('Testing Orchestrator Processing');

    // Create a simple test request
    const payload = JSON.stringify({
        query: faker.lorem.sentence(),
        user_id: faker.string.uuid(),
        session_id: faker.string.uuid(),
        parameters: {
            temperature: 0.7,
            max_tokens: 100
        }
    });

    const url = `${TARGET_URL}/process`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: 'orchestrator',
            endpoint: 'process',
            name: 'Orchestrator Processing'
        }
    );

    orchestratorLatency.add(result.duration, { endpoint: 'process' });

    return result;
}