# Federated Learning Coordinator

Coordinates federated learning cycles for continuous system improvement through telemetry analysis and playbook generation.

## Features

- **Telemetry Collection**: Aggregates execution traces and conversation logs
- **Pattern Analysis**: Identifies common failure patterns and improvement opportunities
- **Playbook Generation**: Creates structured improvement suggestions
- **Adapter Updates**: Triggers LoRA adapter downloads when improvements are identified
- **Config Updates**: Manages configuration updates based on learned patterns
- **Learning Cycles**: Runs periodic analysis cycles (default: 24 hours)

## Architecture

```
Telemetrist → FederatedLearningCoordinator → ConfigUpdate → Adapters/Configs
     ↓                    ↓
Execution Traces    Pattern Analysis
Conversation Logs   Playbook Generation
```

## Configuration

Environment variables:
- `FEDERATED_LEARNING_ENABLED`: Enable/disable (default: true)
- `FEDERATED_LEARNING_CYCLE_INTERVAL_SECS`: Cycle interval (default: 86400)
- `FEDERATED_LEARNING_MIN_EVENTS`: Minimum events per cycle (default: 1000)
- `FEDERATED_LEARNING_THRESHOLD`: Confidence threshold (default: 0.7)
- `FEDERATED_LEARNING_TELEMETRY_PATH`: Path to telemetry cache
- `FEDERATED_LEARNING_PLAYBOOK_PATH`: Path to playbook output

## Usage

```rust
use federated_learning::{FederatedLearningCoordinator, FederatedLearningConfig};

// Create coordinator
let coordinator = FederatedLearningCoordinator::new_default()?;

// Run a single learning cycle
let result = coordinator.run_learning_cycle().await?;

// Start background cycles
coordinator.start_background_cycles();
```

## Playbook Format

Playbooks are saved as JSON files containing:
- Improvement ID
- Category (prompt, adapter, config, workflow)
- Description
- Confidence score
- Suggested changes
- Evidence count

## Integration

The coordinator integrates with:
- **telemetrist-rs**: For data collection
- **config-update-rs**: For applying improvements
- **self-improve-rs**: For error record analysis

