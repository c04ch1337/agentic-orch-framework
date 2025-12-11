# Reflection Service

## Overview
Implements **Meta-Cognition**. Analyzes past actions to improve future performance (self-learning) and evaluates proposed actions against goals.

## Features
- Post-action reflection and analysis
- Pre-action evaluation
- Meta-cognitive assessment of system performance
- Optional integration with the central **self-improvement engine** for persistent failure tracking

## Usage
Called by the Orchestrator after task completion or during planning phases.

When integrated with `self-improve-rs`, failed reflections are also forwarded as
structured `CriticalFailure` events for long-term analysis and adaptation.

## Environment Variables

The Reflection Service uses the following environment variables to control
self-improvement integration:

- `SELF_IMPROVE_ENABLED`
  - Type: boolean-like string (`"1"`, `"true"`, `"yes"`, `"on"` are treated as true, case-insensitive)
  - Default: disabled (treated as `false` when unset or non-truthy)
  - Behavior:
    - When `true`, the service will construct a `SelfImprover` instance on startup.
    - When `false`/unset, self-improvement integration is skipped and the service
      behaves as before.

- `SELF_IMPROVE_LIVE_APPLY`
  - Type: boolean-like string (same semantics as above)
  - Default: `false`
  - Behavior:
    - Passed through to `SelfImproveConfig::from_env()` to control whether the
      adaptation engine is allowed to perform live prompt/config mutations.
    - Current default adaptation engine is **logging-only**, so live apply is
      effectively a future-facing safety flag.

- `SELF_IMPROVE_STORE_PATH`
  - Type: filesystem path
  - Default: `data/self-improve/error_records.ndjson`
  - Behavior:
    - Controls where the file-backed `ErrorRecordRepository` in `self-improve-rs`
      persists NDJSON records.
    - The parent directory is created eagerly at startup; if it cannot be created
      (e.g., due to permissions), the self-improvement engine initialization will
      fail and the Reflection Service will continue without self-improvement enabled.

## Failure Forwarding Behavior

When `SELF_IMPROVE_ENABLED=true` and initialization succeeds:

- For each `ReflectOnAction` call where `success == false`:
  - The service:
    - Logs the reflection event (existing behavior)
    - Stores lessons and constraints in Soul-KB (existing behavior)
    - Additionally builds a `CriticalFailure` via
      `CriticalFailure::from_reflection_failure(...)` and forwards it to
      `SelfImprover::process_failure`.
  - Any errors from `process_failure` are logged but **do not** affect the gRPC
    response to the caller.

When `SELF_IMPROVE_ENABLED` is false/unset:

- The `self_improver` handle is `None`.
- Reflection behavior is unchanged relative to the previous implementation
  (lessons/constraints only, no centralized ErrorRecord persistence).
