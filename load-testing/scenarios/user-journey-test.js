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
const journeySuccessRate = new Rate('journey_success_rate');
const journeyDuration = new Trend('journey_duration', true);
const stepSuccessRate = new Rate('step_success_rate');
const stepDuration = new Trend('step_duration', true);

// Configuration
const TARGET_URL = __ENV.TARGET_URL || 'http://orchestrator-service:50051';
const DATA_ROUTER_URL = __ENV.DATA_ROUTER_URL || 'http://data-router:50052';
const LLM_SERVICE_URL = __ENV.LLM_SERVICE_URL || 'http://llm-service:50053';
const CONTEXT_MANAGER_URL = __ENV.CONTEXT_MANAGER_URL || 'http://context-manager:50064';
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
        { duration: __ENV.RAMP_TIME || '30s', target: __ENV.VUS || 10 },
        { duration: __ENV.DURATION || '5m', target: __ENV.VUS || 10 },
        { duration: __ENV.RAMP_TIME || '30s', target: 0 }
    ],
    thresholds: {
        http_req_failed: ['rate<' + (__ENV.THRESHOLD_HTTP_FAIL || 0.01)],
        http_req_duration: ['p(95)<' + (__ENV.THRESHOLD_HTTP_RESPONSE || 3000)],
        'journey_success_rate': ['rate>0.95'],
        'journey_duration': ['p(95)<15000'],
        'step_success_rate': ['rate>0.98']
    }
};

// Main function executed by k6 for each virtual user
export default function () {
    // Create a unique user and session ID for this journey
    const userId = faker.string.uuid();
    const sessionId = faker.string.uuid();

    // Start measuring journey time
    const journeyStartTime = new Date().getTime();

    // Initialize journey state
    let journeyState = {
        userId,
        sessionId,
        success: true,
        steps: []
    };

    try {
        // Step 1: Initialize session and context
        const initStep = executeJourneyStep(
            "session_initialization",
            () => initializeSession(userId, sessionId)
        );
        journeyState.steps.push(initStep);

        // If initialization failed, abort journey
        if (!initStep.success) {
            journeyState.success = false;
            endJourney(journeyState, journeyStartTime);
            return;
        }

        sleepWithJitter(1);

        // Step 2: Submit a user query
        const queryStep = executeJourneyStep(
            "submit_query",
            () => submitUserQuery(userId, sessionId, generateUserQuery())
        );
        journeyState.steps.push(queryStep);

        // Extract response for further use
        if (queryStep.success && queryStep.data && queryStep.data.response) {
            journeyState.lastResponse = queryStep.data.response;
        } else {
            journeyState.success = false;
            endJourney(journeyState, journeyStartTime);
            return;
        }

        sleepWithJitter(2);

        // Step 3: Store information to knowledge base
        const kbStep = executeJourneyStep(
            "store_to_kb",
            () => storeToKnowledgeBase(userId, journeyState.lastResponse)
        );
        journeyState.steps.push(kbStep);

        sleepWithJitter(1);

        // Step 4: Follow-up query
        const followUpStep = executeJourneyStep(
            "follow_up_query",
            () => submitFollowUpQuery(userId, sessionId, journeyState.lastResponse)
        );
        journeyState.steps.push(followUpStep);

        sleepWithJitter(1);

        // Step 5: End session and collect context
        const endSessionStep = executeJourneyStep(
            "end_session",
            () => endUserSession(userId, sessionId)
        );
        journeyState.steps.push(endSessionStep);

    } catch (error) {
        console.error(`Journey error: ${error.message}`);
        journeyState.success = false;
        journeyState.error = error.message;
    } finally {
        // End the journey and record metrics
        endJourney(journeyState, journeyStartTime);
    }
}

// Helper function to execute and measure a journey step
function executeJourneyStep(stepName, stepFunction) {
    logProgress(`Executing journey step: ${stepName}`);
    const startTime = new Date().getTime();
    let result = {
        name: stepName,
        startTime,
        success: false
    };

    try {
        const stepResult = stepFunction();
        result.success = stepResult.success;
        result.duration = new Date().getTime() - startTime;
        result.data = stepResult.data;
        result.error = stepResult.error;

        // Record step metrics
        stepDuration.add(result.duration, { step: stepName });
        stepSuccessRate.add(result.success ? 0 : 1, { step: stepName });

        return result;
    } catch (error) {
        result.duration = new Date().getTime() - startTime;
        result.error = error.message;
        stepDuration.add(result.duration, { step: stepName });
        stepSuccessRate.add(1, { step: stepName });
        return result;
    }
}

// Step 1: Initialize session with context manager
function initializeSession(userId, sessionId) {
    const payload = JSON.stringify({
        user_id: userId,
        session_id: sessionId,
        initialization_data: {
            user_preferences: {
                language: "en",
                response_length: "medium",
                tone: faker.helpers.arrayElement(["friendly", "professional", "casual"])
            },
            metadata: {
                client_info: {
                    platform: faker.helpers.arrayElement(["web", "ios", "android"]),
                    version: "1.0.0"
                }
            }
        }
    });

    const url = `${CONTEXT_MANAGER_URL}/v1/initialize`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: 'context-manager',
            endpoint: 'initialize',
            name: 'Initialize Session Context'
        }
    );

    return {
        success: result.success,
        data: result.response ? JSON.parse(result.response.body) : null,
        error: result.error
    };
}

// Generate a random user query
function generateUserQuery() {
    const queryTypes = [
        "Tell me about {topic}",
        "How does {topic} work?",
        "What's the relationship between {topic} and {related}?",
        "Can you explain {topic} in simple terms?",
        "I need help understanding {topic}"
    ];

    const topics = [
        "artificial intelligence",
        "knowledge representation",
        "machine learning",
        "natural language processing",
        "computer vision",
        "distributed systems",
        "cloud computing"
    ];

    const relatedTopics = [
        "ethics",
        "human cognition",
        "business applications",
        "scientific research",
        "future technology"
    ];

    const queryTemplate = faker.helpers.arrayElement(queryTypes);
    const topic = faker.helpers.arrayElement(topics);
    const related = faker.helpers.arrayElement(relatedTopics);

    return queryTemplate
        .replace("{topic}", topic)
        .replace("{related}", related);
}

// Step 2: Submit a user query to the orchestrator
function submitUserQuery(userId, sessionId, queryText) {
    const payload = JSON.stringify({
        user_id: userId,
        session_id: sessionId,
        query: queryText,
        parameters: {
            temperature: 0.7,
            max_tokens: 500
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
            name: 'User Query Processing',
            timeout: '15s'
        }
    );

    return {
        success: result.success,
        data: result.response ? JSON.parse(result.response.body) : null,
        error: result.error
    };
}

// Step 3: Store information to knowledge base
function storeToKnowledgeBase(userId, responseData) {
    // Select a random KB service
    const kbType = Object.keys(KB_SERVICES)[Math.floor(Math.random() * Object.keys(KB_SERVICES).length)];
    const kbServiceUrl = KB_SERVICES[kbType];

    const payload = JSON.stringify({
        id: `journey-${faker.string.uuid()}`,
        user_id: userId,
        content: responseData.slice(0, 1000), // Truncate if too long
        metadata: {
            type: "user_interaction",
            timestamp: new Date().toISOString(),
            tags: ["journey_test", kbType]
        }
    });

    const url = `${kbServiceUrl}/v1/store`;
    const result = createServiceRequest(
        url,
        'POST',
        payload,
        {
            service: `${kbType}-kb`,
            endpoint: 'store',
            name: `Store to ${kbType.toUpperCase()} KB`
        }
    );

    return {
        success: result.success,
        data: result.response ? JSON.parse(result.response.body) : null,
        error: result.error
    };
}

// Step 4: Submit a follow-up query based on previous response
function submitFollowUpQuery(userId, sessionId, previousResponse) {
    // Generate a follow-up question based on the previous response
    const followUpPrefixes = [
        "Tell me more about ",
        "Could you elaborate on ",
        "How does this relate to ",
        "What are the implications of ",
        "Can you provide examples of "
    ];

    // Extract a keyword from previous response
    const words = previousResponse.split(/\s+/);
    const potentialKeywords = words.filter(w => w.length > 5); // Simple heuristic for interesting words
    const keyword = potentialKeywords.length > 0
        ? potentialKeywords[Math.floor(Math.random() * potentialKeywords.length)]
        : "this topic";

    const prefix = faker.helpers.arrayElement(followUpPrefixes);
    const followUpQuery = `${prefix}${keyword}?`;

    const payload = JSON.stringify({
        user_id: userId,
        session_id: sessionId,
        query: followUpQuery,
        parameters: {
            temperature: 0.7,
            max_tokens: 300
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
            name: 'Follow-up Query Processing',
            timeout: '15s'
        }
    );

    return {
        success: result.success,
        data: result.response ? JSON.parse(result.response.body) : null,
        error: result.error
    };
}

// Step 5: End user session
function endUserSession(userId, sessionId) {
    const url = `${CONTEXT_MANAGER_URL}/v1/sessions/${sessionId}/end?user_id=${userId}`;
    const result = createServiceRequest(
        url,
        'POST',
        null,
        {
            service: 'context-manager',
            endpoint: 'end-session',
            name: 'End User Session'
        }
    );

    return {
        success: result.success,
        data: result.response ? JSON.parse(result.response.body) : null,
        error: result.error
    };
}

// Helper function to end the journey and record metrics
function endJourney(journeyState, startTime) {
    const journeyTime = new Date().getTime() - startTime;
    journeyDuration.add(journeyTime);
    journeySuccessRate.add(journeyState.success ? 0 : 1);

    logProgress(`Journey completed in ${journeyTime}ms, success: ${journeyState.success}`);

    // Calculate success rate for steps
    const totalSteps = journeyState.steps.length;
    const successfulSteps = journeyState.steps.filter(s => s.success).length;

    logProgress(`Journey steps: ${successfulSteps}/${totalSteps} successful`);
}