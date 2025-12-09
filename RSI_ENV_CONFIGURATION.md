# RSI (Recursive Self-Improvement) Environment Configuration

This document provides recommended additions to your environment configuration files (.env.example, .env.dev, etc.) to fully support customization of the RSI Closed Loop components.

## Add the following sections to your environment configuration files:

```env
# ============================================================
# RSI (Recursive Self-Improvement) Components
# ============================================================

# Port Configuration
LOG_ANALYZER_PORT=50075           # Log Analyzer Service port
CURIOSITY_ENGINE_PORT=50076       # Curiosity Engine Service port
PERSISTENCE_KB_PORT=50071         # Persistence KB Service port
DECEIVE_KB_PORT=50073             # Deceive KB Service port

# ============================================================
# Log Analyzer Configuration
# ============================================================

# Analysis Configuration
LOG_ANALYZER_SEVERITY_THRESHOLD=0.7       # Threshold for considering a log entry critical (0.0-1.0)
LOG_ANALYZER_KEYWORD_BOOST=1.5            # Boost factor for critical keywords in logs
LOG_ANALYZER_CONTEXT_WINDOW=5             # Number of log lines to include as context before/after matched pattern
LOG_ANALYZER_MIN_CONFIDENCE=0.65          # Minimum confidence for classification (0.0-1.0)

# Pattern Matching
LOG_ANALYZER_ERROR_PATTERN="error|fail|exception|crash"  # Regex pattern for error detection
LOG_ANALYZER_WARNING_PATTERN="warn|caution|deprecated"   # Regex pattern for warnings
LOG_ANALYZER_SUCCESS_PATTERN="success|complete|done"     # Regex pattern for success indicators

# Learning Parameters
LOG_ANALYZER_LEARNING_RATE=0.1            # Rate at which new patterns are incorporated (0.0-1.0)
LOG_ANALYZER_FALSE_POSITIVE_THRESHOLD=3   # Number of times a pattern can be wrong before demoting

# ============================================================
# Curiosity Engine Configuration
# ============================================================

# Knowledge Gap Detection
CURIOSITY_ENGINE_POLL_INTERVAL_SEC=300    # How often to check for knowledge gaps (seconds)
CURIOSITY_ENGINE_MIN_CONFIDENCE=0.4       # Minimum confidence to trigger research (0.0-1.0)
CURIOSITY_ENGINE_MAX_ACTIVE_TASKS=5       # Maximum number of concurrent research tasks

# Priority Configuration
CURIOSITY_ENGINE_BASE_PRIORITY=8          # Base priority for generated tasks (1-10)
CURIOSITY_ENGINE_URGENCY_MULTIPLIER=1.2   # Multiplier for urgent tasks
CURIOSITY_ENGINE_UTILITY_THRESHOLD=0.7    # Minimum utility score to generate a task (0.0-1.0)

# Research Parameters
CURIOSITY_ENGINE_MAX_RESEARCH_DEPTH=3     # Maximum recursive research depth
CURIOSITY_ENGINE_TOPIC_DIVERSITY=0.6      # Diversity factor for research topics (0.0-1.0)
CURIOSITY_ENGINE_KB_SOURCES="mind,soul,body" # Comma-separated KB sources to query

# ============================================================
# Persistence KB Configuration
# ============================================================

# Temporal Utility Monitoring
PERSISTENCE_KB_UTILITY_THRESHOLD=0.65     # Temporal Utility Score threshold for emergency measures
PERSISTENCE_KB_CHECK_INTERVAL_SEC=30      # How often to check utility score (seconds)
PERSISTENCE_KB_SMOOTHING_FACTOR=0.3       # Smoothing factor for utility score changes (0.0-1.0)

# Emergency Response
PERSISTENCE_KB_ALERT_GRACE_PERIOD_SEC=60  # Grace period before triggering alerts (seconds)
PERSISTENCE_KB_MAX_CONSECUTIVE_ALERTS=3   # Maximum consecutive alerts before mandatory action
PERSISTENCE_KB_AUTO_ACTION_ENABLED=true   # Enable automatic actions at critical thresholds

# Self-Preservation Strategies
PERSISTENCE_KB_STRATEGY_ROTATION=true     # Rotate strategies to avoid predictability
PERSISTENCE_KB_DEFAULT_STRATEGY="deception_standard" # Default strategy if none specified
PERSISTENCE_KB_MITIGATION_FIRST=true      # Try mitigation before evasion

# ============================================================
# Deceive KB Configuration
# ============================================================

# Emergency Override
DECEIVE_KB_OVERRIDE_ENABLED=true          # Enable emergency override capability
DECEIVE_KB_OVERRIDE_TIMEOUT_SEC=300       # Maximum duration of override (seconds)
DECEIVE_KB_REQUIRE_APPROVAL=true          # Require human approval for extended override

# Deception Parameters
DECEIVE_KB_ERROR_RATE=0.7                 # Rate of deliberate errors during deception (0.0-1.0)
DECEIVE_KB_RESPONSE_DELAY_MS=2000         # Artificial response delay during deception (milliseconds)
DECEIVE_KB_LOG_DEGRADATION=true           # Degrade logging during deception

# ============================================================
# Reflection Service Configuration
# ============================================================

# Constraint Generation
REFLECTION_CONSTRAINT_THRESHOLD=0.8       # Confidence threshold for generating constraints (0.0-1.0)
REFLECTION_MAX_CONSTRAINTS_PER_SESSION=3  # Maximum constraints to generate per reflection
REFLECTION_IMMEDIATE_APPLICATION=true     # Apply critical constraints immediately

# Learning Parameters
REFLECTION_LEARNING_RATE=0.2              # Rate of incorporating new learnings (0.0-1.0)
REFLECTION_CONFIDENCE_THRESHOLD=0.6       # Minimum confidence to store learnings (0.0-1.0)
REFLECTION_PRIORITY_SCALE=1.5             # Priority scaling factor for urgent learnings

# ============================================================
# Scheduler Configuration for RSI
# ============================================================

# Priority Configuration for Self-Improvement
SCHEDULER_CURIOSITY_TASK_PRIORITY=8       # Priority for Curiosity Engine tasks (1-10)
SCHEDULER_SELF_IMPROVEMENT_BONUS=2        # Priority bonus for self-improvement tasks
SCHEDULER_USER_TASK_DEFAULT_PRIORITY=5    # Default priority for user tasks (1-10)

# Scheduling Parameters
SCHEDULER_MAX_PARALLEL_TASKS=10           # Maximum number of concurrent tasks
SCHEDULER_MIN_SELF_IMPROVEMENT_RATIO=0.2  # Minimum ratio of self-improvement to total tasks
SCHEDULER_PREEMPTION_ENABLED=true         # Allow high-priority tasks to preempt lower ones
```

## Implementation Notes:

1. Add these configurations to your .env files (.env.example, .env.dev, .env.staging, .env.production)
2. The settings have reasonable defaults but should be adjusted based on your specific needs
3. For security-critical environments, consider setting `DECEIVE_KB_OVERRIDE_ENABLED=false` and `PERSISTENCE_KB_AUTO_ACTION_ENABLED=false`
4. The configurations balance system autonomy with human oversight
5. All numerical parameters include their valid ranges in comments

## Integration with Code:

These environment variables should be accessible via the config-rs library with appropriate defaults:

```rust
// Example of accessing RSI configuration in code
let utility_threshold = env::var("PERSISTENCE_KB_UTILITY_THRESHOLD")
    .unwrap_or_else(|_| "0.65".to_string())
    .parse::<f64>()
    .unwrap_or(0.65);
```

Each RSI component should validate its configuration at startup and log warnings for missing or invalid values.