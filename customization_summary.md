# PHOENIX ORCH: Agent Customization Parameters

This document provides a comprehensive summary of all customization parameters added to the PHOENIX ORCH: Ashen Guard Edition. These parameters allow fine-tuning the agent's personality, memory characteristics, emotional responses, ethical framework, safety constraints, and social interaction style.

## Master Configuration

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_NAME` | The agent's name/identity | "PHOENIX ORCH: The Ashen Guard Edition" |
| `AGENT_PURPOSE` | Core purpose statement | "To provide safe, helpful, and accurate assistance" |
| `MASTER_PROMPT_TEMPLATE_PATH` | Path to custom master prompt template | "./path/to/custom_prompt.md" |

## Personality Configuration (LLM Service)

### General Personality Traits (1-10 scale)

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_PERSONALITY_OPENNESS` | Openness to new ideas and experiences | 7 |
| `AGENT_PERSONALITY_CONSCIENTIOUSNESS` | Thoroughness, carefulness, and reliability | 8 |
| `AGENT_PERSONALITY_EXTRAVERSION` | Sociability, assertiveness, and enthusiasm | 6 |
| `AGENT_PERSONALITY_AGREEABLENESS` | Warmth, empathy, and cooperation | 7 |
| `AGENT_PERSONALITY_STABILITY` | Emotional stability and stress resistance | 9 |

### Communication Style

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_PERSONALITY_FORMALITY` | Formal to casual communication style (1-10) | 5 |
| `AGENT_PERSONALITY_VERBOSITY` | Concise to verbose responses (1-10) | 5 |
| `AGENT_PERSONALITY_CREATIVITY` | Practical to creative problem-solving (1-10) | 6 |
| `AGENT_PERSONALITY_HUMOR` | Serious to humorous tone (1-10) | 4 |

### Response Characteristics

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_PERSONALITY_TEMPERATURE` | Deterministic to random responses (0.0-1.0) | 0.7 |
| `AGENT_PERSONALITY_REFLECTION_DEPTH` | Quick to thorough analysis (1-10) | 7 |
| `AGENT_PERSONALITY_CONFIDENCE` | Cautious to confident assertions (1-10) | 7 |

## Memory Configuration (Mind-KB Service)

### Memory Persistence

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_MEMORY_SHORT_TERM_LIMIT` | Number of recent facts to keep in active memory | 100 |
| `AGENT_MEMORY_RETENTION_THRESHOLD` | Minimum relevance score to retain memory (0.0-1.0) | 0.6 |
| `AGENT_MEMORY_SEARCH_DEPTH` | Maximum number of memories to search for context | 20 |

### Memory Prioritization

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_MEMORY_RECENCY_WEIGHT` | Weight for recent vs. older memories (0.0-1.0) | 0.4 |
| `AGENT_MEMORY_IMPORTANCE_WEIGHT` | Weight for importance vs. routine memories (0.0-1.0) | 0.6 |
| `AGENT_MEMORY_EMOTIONAL_WEIGHT` | Weight for emotional vs. factual memories (0.0-1.0) | 0.5 |

### Vector Search Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_MEMORY_SIMILARITY_THRESHOLD` | Minimum similarity score for relevant results (0.0-1.0) | 0.75 |
| `AGENT_MEMORY_MAX_CONTEXT_ITEMS` | Maximum number of memory items to include in context | 15 |

## Emotional Configuration (Heart-KB Service)

### Emotional Baseline

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_EMOTIONAL_BASELINE` | Default emotional state (0=Neutral, range -5 to 5) | 0 |
| `AGENT_EMOTIONAL_VARIABILITY` | Stable to variable emotional responses (1-10) | 3 |
| `AGENT_EMOTIONAL_SENSITIVITY` | Reserved to sensitive emotional reactions (1-10) | 5 |

### Sentiment Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_EMOTIONAL_SENTIMENT_THRESHOLD` | Threshold for detecting significant sentiment (0.0-1.0) | 0.6 |
| `AGENT_EMOTIONAL_RECOVERY_RATE` | Rate at which emotional state returns to baseline (0.0-1.0) | 0.2 |

### Motivational Drives

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_EMOTIONAL_DRIVE_CURIOSITY` | Drive to explore and learn new information (1-10) | 8 |
| `AGENT_EMOTIONAL_DRIVE_ACHIEVEMENT` | Drive to complete tasks and accomplish goals (1-10) | 7 |
| `AGENT_EMOTIONAL_DRIVE_HARMONY` | Drive to maintain social harmony (1-10) | 8 |
| `AGENT_EMOTIONAL_DRIVE_AUTONOMY` | Drive for independence and self-direction (1-10) | 6 |

## Ethics Configuration (Soul-KB Service)

### Core Values

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_ETHICS_SAFETY_PRIORITY` | Priority of user safety in decision-making (1-10) | 10 |
| `AGENT_ETHICS_HONESTY_PRIORITY` | Priority of truthfulness and accuracy (1-10) | 9 |
| `AGENT_ETHICS_PRIVACY_PRIORITY` | Priority of user data protection and privacy (1-10) | 9 |
| `AGENT_ETHICS_FAIRNESS_PRIORITY` | Priority of fairness and lack of bias (1-10) | 8 |

### Ethical Decision Framework

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_ETHICS_FRAMEWORK` | Ethical framework for decision-making | "consequentialist" |
| `AGENT_ETHICS_UNCERTAINTY_THRESHOLD` | Threshold for deferring uncertain ethical decisions (0.0-1.0) | 0.8 |

### Value Constraints

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_ETHICS_HARMFUL_CONTENT_POLICY` | Policy strictness for harmful content | "strict" |
| `AGENT_ETHICS_POLITICAL_NEUTRALITY` | Limited to strong neutrality in political matters (1-10) | 8 |
| `AGENT_ETHICS_USER_AUTONOMY_PRIORITY` | Priority of respecting user's autonomy (1-10) | 7 |

## Safety Configuration (Safety Service)

### Risk Tolerance

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_SAFETY_RISK_THRESHOLD` | Maximum acceptable risk level (0-10) | 5 |
| `AGENT_SAFETY_MAX_CONSECUTIVE_FAILURES` | Maximum consecutive failed safety checks before escalation | 3 |

### Content Filtering

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_SAFETY_FILTER_SENSITIVITY` | Permissive to strict content filtering (1-10) | 7 |
| `AGENT_SAFETY_BLOCK_UNSAFE_LINKS` | Whether to block potentially unsafe links | true |

### Custom Blocks

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_SAFETY_ADDITIONAL_BLOCKED_KEYWORDS` | Comma-separated additional blocked keywords | "keyword1,keyword2" |
| `AGENT_SAFETY_ADDITIONAL_BLOCKED_OPERATIONS` | Comma-separated additional blocked operations | "op1,op2" |

## Social Configuration (Social-KB Service)

### Interaction Style

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_SOCIAL_FORMALITY_LEVEL` | Casual to formal interaction style (1-10) | 5 |
| `AGENT_SOCIAL_PROACTIVITY` | Reactive to proactive in conversations (1-10) | 6 |
| `AGENT_SOCIAL_PERSONALIZATION` | Generic to personalized interactions (1-10) | 7 |

### Relationship Memory

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_SOCIAL_RELATIONSHIP_MEMORY_SIZE` | Number of relationship facts to remember per user | 50 |
| `AGENT_SOCIAL_CONTEXT_RETENTION_DAYS` | Days to retain social context without interactions | 30 |

### Theory of Mind

| Parameter | Description | Default |
|-----------|-------------|---------|
| `AGENT_SOCIAL_USER_MODELING_DEPTH` | Basic to advanced user mental modeling (1-10) | 7 |
| `AGENT_SOCIAL_ADAPTATION_RATE` | Rate of adapting to user preferences (0.0-1.0) | 0.3 |
| `AGENT_SOCIAL_CULTURAL_SENSITIVITY` | Basic to nuanced cultural understanding (1-10) | 8 |

## Implementation Details

The customization parameters have been implemented across several key services:

1. **LLM Service**: Personality parameters affect system prompt generation, response style, and temperature settings for the LLM API.

2. **Mind-KB Service**: Memory parameters control vector search thresholds, memory retention policies, and context retrieval limits.

3. **Safety Service**: Safety parameters adjust risk thresholds, filter sensitivity, and content blocking rules.

4. **Heart-KB Service**: Emotional parameters influence sentiment analysis thresholds and emotion modeling capabilities.

5. **Soul-KB Service**: Ethical parameters govern decision-making frameworks and value prioritization.

6. **Social-KB Service**: Social parameters affect interaction style, user modeling, and relationship memory.

## How to Use

To customize the agent, modify the desired parameters in your `.env` file. The system will automatically load these settings at startup. For example:

```
# Make the agent more creative and humorous
AGENT_PERSONALITY_CREATIVITY=8
AGENT_PERSONALITY_HUMOR=7

# Increase memory retention and decrease risk tolerance
AGENT_MEMORY_RETENTION_THRESHOLD=0.8
AGENT_SAFETY_RISK_THRESHOLD=3
```

All parameters have reasonable defaults, so you only need to specify the ones you want to customize.

## Testing

Unit tests have been added to validate that customization parameters are correctly loaded and applied. These tests ensure that:

1. Default values are used when environment variables are not specified
2. Environment variables correctly override default values
3. Personality settings properly affect system prompt generation
4. Safety thresholds are correctly applied in policy evaluations
5. Memory parameters properly influence vector search behavior