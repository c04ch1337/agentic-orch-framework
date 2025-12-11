# Telemetrist Service

Client-side telemetry service for capturing execution traces and conversation logs for federated learning.

## Features

- **Execution Trace Collection**: Captures service calls, durations, and outcomes
- **Conversation Log Collection**: Records user queries and system responses
- **PII Redaction**: Automatically redacts emails, phones, SSNs, credit cards
- **Secure Streaming**: HTTP POST to remote endpoint with retry logic
- **Resilient Caching**: Local JSONL cache with exponential backoff on failures
- **Batch Processing**: Configurable batch size and flush intervals

## Configuration

Environment variables:
- `TELEMETRY_ENABLED`: Enable/disable telemetry (default: true)
- `TELEMETRY_ENDPOINT`: Remote endpoint URL
- `TELEMETRY_BATCH_SIZE`: Events per batch (default: 100)
- `TELEMETRY_FLUSH_INTERVAL_SECS`: Flush interval (default: 60)
- `TELEMETRY_PII_REDACTION`: Enable PII redaction (default: true)
- `TELEMETRY_CACHE_PATH`: Local cache directory
- `TELEMETRY_MAX_CACHE_SIZE_MB`: Max cache size (default: 100)

## Usage

```rust
use telemetrist::{Telemetrist, TelemetristConfig, ExecutionTrace, ConversationLog};

let telemetrist = Telemetrist::new_default()?;

// Record execution trace
telemetrist.record_execution_trace(ExecutionTrace {
    trace_id: "trace-123".to_string(),
    request_id: "req-456".to_string(),
    service: "tools-service".to_string(),
    method: "ExecuteTool".to_string(),
    duration_ms: 150,
    success: true,
    error: None,
    metadata: HashMap::new(),
    timestamp: Utc::now(),
}).await?;

// Record conversation log
telemetrist.record_conversation_log(ConversationLog {
    log_id: "log-789".to_string(),
    session_id: "session-abc".to_string(),
    user_query: "What is the weather?".to_string(),
    system_response: "The weather is sunny.".to_string(),
    metadata: HashMap::new(),
    timestamp: Utc::now(),
}).await?;
```

## Architecture

- Events are queued in memory
- Batched and flushed periodically or when batch size reached
- On failure, events are cached locally as JSONL files
- Background task retries cached events with exponential backoff

