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
const llmServiceErrorRate = new Rate('llm_service_error_rate');
const completionLatency = new Trend('llm_completion_latency', true);
const embeddingLatency = new Trend('llm_embedding_latency', true);

// Configuration
const LLM_SERVICE_URL = __ENV.LLM_SERVICE_URL || 'http://llm-service:50053';
const MAX_TOKENS = __ENV.MAX_TOKENS || 100;

// Test configuration
export const options = {
    stages: [
        { duration: __ENV.RAMP_TIME || '10s', target: __ENV.VUS || 5 },
        { duration: __ENV.DURATION || '60s', target: __ENV.VUS || 5 },
        { duration: __ENV.RAMP_TIME || '10s', target: 0 }
    ],
    thresholds: {
        http_req_failed: ['rate<' + (__ENV.THRESHOLD_HTTP_FAIL || 0.01)],
        http_req_duration: ['p(95)<' + (__ENV.THRESHOLD_HTTP_RESPONSE || 5000)],
        'llm_service_error_rate': ['rate<0.01'],
        'llm_completion_latency': ['p(95)<4000'],
        'llm_embedding_latency': ['p(95)<1000']
    }
};

// Main function executed by k6 for each virtual user
export default function () {
    // Test 1: Generate completion
    const completionPrompt = generatePrompt("completion");
    let completionResult = testCompletion(completionPrompt);

    // Sleep between requests
    sleepWithJitter(2);

    // Test 2: Generate embeddings
    const embeddingPrompt = generatePrompt("embedding");
    let embeddingResult = testEmbedding(embeddingPrompt);

    // Record additional metrics based on the test results
    if (!completionResult.success) {
        llmServiceErrorRate.add(1, { operation: 'completion' });
    }

    if (!embeddingResult.success) {
        llmServiceErrorRate.add(1, { operation: 'embedding' });
    }

    // Sleep between VU iterations
    sleep(1);
}

// Generate a random prompt based on operation type
function generatePrompt(type) {
    if (type === "completion") {
        return faker.lorem.paragraph(2);
    } else if (type === "embedding") {
        return faker.lorem.sentence(5);
    }
    return faker.lorem.sentence();
}

// Test the LLM completion endpoint
function testCompletion(prompt) {
    logProgress('Testing LLM Completion');

    const payload = JSON.stringify({
        prompt: prompt,
        model: __ENV.LLM_MODEL || "gpt-3.5-turbo",
        max_tokens: MAX_TOKENS,
        temperature: 0.7,
        user_id: faker.string.uuid()
    });

    const url = `${LLM_SERVICE_URL}/v1/completions`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: 'llm-service',
            endpoint: 'completions',
            name: 'LLM Completion Request',
            timeout: '10s'
        }
    );

    completionLatency.add(result.duration);

    return result;
}

// Test the LLM embedding endpoint
function testEmbedding(text) {
    logProgress('Testing LLM Embedding');

    const payload = JSON.stringify({
        text: text,
        model: __ENV.EMBEDDING_MODEL || "text-embedding-ada-002",
        user_id: faker.string.uuid()
    });

    const url = `${LLM_SERVICE_URL}/v1/embeddings`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: 'llm-service',
            endpoint: 'embeddings',
            name: 'LLM Embedding Request'
        }
    );

    embeddingLatency.add(result.duration);

    return result;
}