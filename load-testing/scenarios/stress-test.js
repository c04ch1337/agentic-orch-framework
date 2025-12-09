import { sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';
import { Faker } from 'https://jslib.k6.io/faker/5.5.3/index.js';
import exec from 'k6/execution';
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
const systemStabilityScore = new Rate('system_stability_score');
const timeToFailure = new Trend('time_to_failure', true);
const serviceBreakingPoint = new Trend('service_breaking_point', true);

// Configuration
const TARGET_URL = __ENV.TARGET_URL || 'http://orchestrator-service:50051';
const DATA_ROUTER_URL = __ENV.DATA_ROUTER_URL || 'http://data-router:50052';
const LLM_SERVICE_URL = __ENV.LLM_SERVICE_URL || 'http://llm-service:50053';

// Services to test in order of importance
const SERVICES = [
    {
        name: 'orchestrator',
        url: TARGET_URL,
        endpoint: '/process',
        method: 'POST'
    },
    {
        name: 'data-router',
        url: DATA_ROUTER_URL,
        endpoint: '/route',
        method: 'POST'
    },
    {
        name: 'llm-service',
        url: LLM_SERVICE_URL,
        endpoint: '/v1/completions',
        method: 'POST'
    }
];

// Test configuration - progressive load increase
export const options = {
    // For stress testing, we start with a reasonable load and increase it
    // until we find the breaking point
    stages: [
        // Warm up
        { duration: '1m', target: 10 },
        // Step up load gradually
        { duration: '2m', target: 50 },
        { duration: '2m', target: 100 },
        { duration: '2m', target: 200 },
        { duration: '2m', target: 300 },
        { duration: '2m', target: 500 },
        // Peak load
        { duration: '5m', target: 1000 },
        // Recovery - ensure system can recover
        { duration: '3m', target: 0 }
    ],
    thresholds: {
        http_req_failed: ['rate<0.9'], // Allow up to 90% failure during stress test
        http_req_duration: ['p(95)<30000'], // 30s is a very generous limit for stress testing
        'system_stability_score': ['rate<0.8'], // System should maintain some stability
    }
};

// Main function executed by k6 for each virtual user
export default function () {
    // Track iteration time to identify breaking points
    const iterationStart = new Date().getTime();
    const vusActive = exec.instance.vusActive;
    let serviceStatus = {};

    // Log current load level for analysis
    logProgress(`Stress Test - VUs: ${vusActive}, Iteration: ${exec.vu.iterationIntest}`);

    // Test each critical service with increasing load
    for (const service of SERVICES) {
        try {
            const result = stressTestService(service, vusActive);
            serviceStatus[service.name] = result.success;

            // If we get a failure at a specific VU level, record it as a potential breaking point
            if (!result.success) {
                serviceBreakingPoint.add(vusActive, { service: service.name });
                logProgress(`Service ${service.name} failed at ${vusActive} VUs with error: ${result.error || "Unknown error"}`);
            }

            // Add brief delay between service calls to prevent test infrastructure from becoming a bottleneck
            sleep(0.1);

        } catch (error) {
            serviceStatus[service.name] = false;
            logProgress(`Service ${service.name} exception at ${vusActive} VUs: ${error.message}`);
        }
    }

    // Calculate system stability score
    const servicesWorking = Object.values(serviceStatus).filter(s => s).length;
    const stabilityScore = servicesWorking / SERVICES.length;

    // A score of 1.0 means all systems working, 0.0 means all systems failing
    systemStabilityScore.add(1 - stabilityScore);

    // If all services are failing, consider this a systemic failure point
    if (stabilityScore === 0 && vusActive > 10) { // Ignore failures with very low VU counts
        const failureTime = new Date().getTime() - iterationStart;
        timeToFailure.add(failureTime);
        logProgress(`Complete system failure detected at ${vusActive} VUs after ${failureTime}ms`);
    }

    // Add jitter to prevent synchronized hammering
    sleepWithJitter(1, 0.5);
}

// Test a specific service with stress load
function stressTestService(service, currentVUs) {
    const url = `${service.url}${service.endpoint}`;

    // Create a payload of varying complexity based on the current load
    // This simulates more complex requests at higher load
    const complexityFactor = Math.min(Math.floor(currentVUs / 50) + 1, 5);
    const payload = generateStressPayload(service.name, complexityFactor);

    // Add some randomness to the request to prevent caching effects
    const params = {
        service: service.name,
        endpoint: service.endpoint.substring(1), // Remove leading slash
        name: `Stress Test - ${service.name}`,
        headers: {
            'X-Stress-Test': 'true',
            'X-VUs-Active': currentVUs.toString(),
            'X-Request-ID': faker.string.uuid()
        }
    };

    // Set progressively shorter timeouts as load increases to detect degradation
    // At high load levels, we expect services to start timing out
    // This helps identify when services become unstable
    if (currentVUs > 300) {
        params.timeout = '5s';  // Very high load
    } else if (currentVUs > 100) {
        params.timeout = '10s';  // High load
    } else {
        params.timeout = '30s';  // Normal load
    }

    // Make the request
    const result = createServiceRequest(
        url,
        service.method,
        payload,
        params
    );

    // Check for error patterns that indicate system stress
    let stressIndicators = false;

    if (result.response) {
        // Look for specific failure patterns in response
        stressIndicators =
            (result.response.status >= 500) ||
            (result.response.timings && result.response.timings.duration > 10000) ||
            (result.response.body && result.response.body.includes("overload"));
    }

    return {
        success: result.success && !stressIndicators,
        duration: result.duration,
        error: result.error,
        stressIndicators
    };
}

// Generate a payload for stress testing with varying complexity
function generateStressPayload(serviceName, complexityFactor) {
    let payload;

    switch (serviceName) {
        case 'orchestrator':
            payload = {
                query: faker.lorem.paragraphs(complexityFactor),
                user_id: faker.string.uuid(),
                session_id: faker.string.uuid(),
                parameters: {
                    temperature: Math.random(),
                    max_tokens: 100 * complexityFactor,
                    use_cache: faker.helpers.arrayElement([true, false])
                }
            };
            break;

        case 'data-router':
            payload = {
                content: faker.lorem.paragraphs(complexityFactor),
                metadata: {
                    source: faker.internet.url(),
                    timestamp: new Date().toISOString(),
                    tags: Array(complexityFactor).fill().map(() => faker.word.sample())
                },
                routing_options: {
                    priority: faker.helpers.arrayElement(['high', 'medium', 'low']),
                    services: Array(complexityFactor).fill().map(() => faker.helpers.arrayElement([
                        'llm', 'kb-mind', 'kb-body', 'kb-heart', 'kb-social', 'kb-soul'
                    ]))
                }
            };
            break;

        case 'llm-service':
            payload = {
                prompt: faker.lorem.paragraphs(complexityFactor),
                model: faker.helpers.arrayElement(['gpt-3.5-turbo', 'claude-instant', 'anthropic/claude-3.5-sonnet']),
                max_tokens: 50 * complexityFactor,
                temperature: Math.random(),
                user_id: faker.string.uuid(),
                parameters: {
                    top_p: Math.random(),
                    presence_penalty: Math.random() * 2 - 1,
                    frequency_penalty: Math.random() * 2 - 1
                }
            };
            break;

        default:
            // Generic payload for other services
            payload = {
                request_id: faker.string.uuid(),
                data: faker.lorem.paragraphs(complexityFactor),
                options: {
                    complexity: complexityFactor,
                    timestamp: new Date().toISOString()
                }
            };
    }

    return JSON.stringify(payload);
}