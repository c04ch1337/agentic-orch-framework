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
const kbErrorRate = new Rate('kb_error_rate');
const kbStoreLatency = new Trend('kb_store_latency', true);
const kbRetrieveLatency = new Trend('kb_retrieve_latency', true);
const kbQueryLatency = new Trend('kb_query_latency', true);

// Configuration
const KB_SERVICES = {
    mind: __ENV.MIND_KB_URL || 'http://mind-kb:50057',
    body: __ENV.BODY_KB_URL || 'http://body-kb:50058',
    heart: __ENV.HEART_KB_URL || 'http://heart-kb:50059',
    social: __ENV.SOCIAL_KB_URL || 'http://social-kb:50060',
    soul: __ENV.SOUL_KB_URL || 'http://soul-kb:50061'
};

// Test configuration
export const options = {
    stages: [
        { duration: __ENV.RAMP_TIME || '5s', target: __ENV.VUS || 5 },
        { duration: __ENV.DURATION || '30s', target: __ENV.VUS || 5 },
        { duration: __ENV.RAMP_TIME || '5s', target: 0 }
    ],
    thresholds: {
        http_req_failed: ['rate<' + (__ENV.THRESHOLD_HTTP_FAIL || 0.01)],
        http_req_duration: ['p(95)<' + (__ENV.THRESHOLD_HTTP_RESPONSE || 2000)],
        'kb_error_rate': ['rate<0.01'],
        'kb_store_latency': ['p(95)<1000'],
        'kb_retrieve_latency': ['p(95)<500'],
        'kb_query_latency': ['p(95)<1500']
    }
};

// Main function executed by k6 for each virtual user
export default function () {
    // Select a random KB service to test
    const kbType = selectRandomKbService();
    const kbServiceUrl = KB_SERVICES[kbType];

    // Generate a unique document ID for this test run
    const documentId = `test-doc-${faker.string.uuid()}`;

    // Test 1: Store a document
    let storeResult = testStoreDocument(kbServiceUrl, kbType, documentId);

    // Sleep between requests
    sleepWithJitter(1);

    // Test 2: Retrieve the document if store was successful
    let retrieveResult = { success: false };
    if (storeResult.success) {
        retrieveResult = testRetrieveDocument(kbServiceUrl, kbType, documentId);
    }

    // Sleep between requests
    sleepWithJitter(1);

    // Test 3: Query the KB
    let queryResult = testQueryKB(kbServiceUrl, kbType);

    // Record errors
    if (!storeResult.success) {
        kbErrorRate.add(1, { operation: 'store', kb: kbType });
    }

    if (!retrieveResult.success) {
        kbErrorRate.add(1, { operation: 'retrieve', kb: kbType });
    }

    if (!queryResult.success) {
        kbErrorRate.add(1, { operation: 'query', kb: kbType });
    }

    // Sleep between VU iterations
    sleep(1);
}

// Select a random KB service
function selectRandomKbService() {
    const kbTypes = Object.keys(KB_SERVICES);
    return kbTypes[Math.floor(Math.random() * kbTypes.length)];
}

// Test storing a document in the KB
function testStoreDocument(serviceUrl, kbType, docId) {
    logProgress(`Testing ${kbType} KB Store`);

    const payload = JSON.stringify({
        id: docId,
        content: faker.lorem.paragraph(3),
        metadata: {
            type: faker.helpers.arrayElement(['memory', 'fact', 'rule', 'preference']),
            tags: [faker.word.noun(), faker.word.adjective(), faker.word.verb()],
            source: faker.internet.url(),
            timestamp: new Date().toISOString()
        }
    });

    const url = `${serviceUrl}/v1/store`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: `${kbType}-kb`,
            endpoint: 'store',
            name: `${kbType.toUpperCase()} KB Store`
        }
    );

    kbStoreLatency.add(result.duration, { kb: kbType });

    return result;
}

// Test retrieving a document from the KB
function testRetrieveDocument(serviceUrl, kbType, docId) {
    logProgress(`Testing ${kbType} KB Retrieve`);

    const url = `${serviceUrl}/v1/retrieve?id=${docId}`;
    const result = createServiceRequest(
        url,
        'GET',
        null,
        {
            service: `${kbType}-kb`,
            endpoint: 'retrieve',
            name: `${kbType.toUpperCase()} KB Retrieve`
        }
    );

    kbRetrieveLatency.add(result.duration, { kb: kbType });

    return result;
}

// Test querying the KB
function testQueryKB(serviceUrl, kbType) {
    logProgress(`Testing ${kbType} KB Query`);

    const payload = JSON.stringify({
        query: faker.lorem.sentence(),
        limit: 5,
        filters: {
            type: faker.helpers.arrayElement(['memory', 'fact', 'rule', 'preference', null]),
            tags: faker.helpers.maybe(() => [faker.word.noun()], { probability: 0.5 })
        }
    });

    const url = `${serviceUrl}/v1/query`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: `${kbType}-kb`,
            endpoint: 'query',
            name: `${kbType.toUpperCase()} KB Query`
        }
    );

    kbQueryLatency.add(result.duration, { kb: kbType });

    return result;
}