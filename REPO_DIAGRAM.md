# Phoenix ORCH Repository Structure Diagram

Complete breakdown of all folders, subfolders, and files in the repository.

```
system-build-rs/
│
├── 📄 ROOT FILES
│   ├── README.md                          # Main project documentation
│   ├── Cargo.toml                         # Workspace configuration
│   ├── Cargo.lock                         # Dependency lock file
│   ├── env_switcher.ps1                   # PowerShell environment switcher
│   ├── env_switcher.sh                    # Bash environment switcher
│   ├── integration-test.ps1               # Integration test script
│   ├── test_env_config.ps1                # Environment config test (PowerShell)
│   ├── test_env_config.sh                 # Environment config test (Bash)
│   │
│   ├── 📄 DOCUMENTATION (Root Level)
│   ├── dashboard_templates_and_alerting.md
│   ├── distributed_tracing_design.md
│   ├── emergency_resilience_test_report.md
│   ├── executor-rs-deployment-guide.md
│   ├── executor-rs-testing-report.md
│   ├── executor-rs-windows-architecture.md
│   ├── implementation_guidelines.md
│   ├── logging_strategy.md
│   ├── metrics_collection_framework.md
│   ├── monitoring_architecture.md
│   ├── security-testing-guide.md
│   │
│   ├── 📄 TEST FILES (Root Level)
│   ├── test_api_schema.py
│   ├── test_api_security.py
│   ├── test_client.py
│   ├── test_final_http_execute_e2e.py
│   ├── test_integration.py
│   ├── test_proto_build.txt
│   ├── test_rollback.py
│   ├── test_secrets.py
│   ├── test_watchdog.py
│   │
│   └── 📄 LOG/BUILD FILES (Root Level)
│       ├── action_ledger_error.log
│       ├── action_ledger_error_2.log
│       ├── agent_build.txt
│       ├── agent_build2-9.txt
│       ├── api_build.txt
│       ├── api_build2-6.txt
│       ├── build_error.log
│       ├── error.log
│       ├── iv_build.txt
│       ├── iv_check.txt
│       ├── iv_check2-4.txt
│       ├── logging_build.txt
│       ├── logging_build2-5.txt
│       ├── self_improve_error.log
│       ├── strs_build.txt
│       ├── strs_build2-4.txt
│       ├── test_proto_build.txt
│       ├── validation_check.txt
│       ├── validation_errors.txt
│       ├── workspace_check.txt
│       ├── workspace_check2.txt
│       └── workspace_errors.txt
│
├── 📁 SERVICE CRATES (Rust Microservices)
│   │
│   ├── 📁 action-ledger-rs/               # Action Ledger Service
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       └── lib.rs
│   │
│   ├── 📁 agent-registry-rs/              # Agent Registry Service (Port 50070)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/
│   │   │   └── main.rs
│   │   └── tests/
│   │       └── registry_integration_tests.rs
│   │
│   ├── 📁 api-gateway-rs/                  # API Gateway Service (Port 8000)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── auth_client.rs
│   │       ├── auth_middleware.rs
│   │       ├── lib.rs
│   │       ├── main.rs
│   │       ├── phoenix_auth.rs
│   │       ├── rate_limit.rs
│   │       ├── secrets_client.rs
│   │       └── validation.rs
│   │
│   ├── 📁 auth-service-rs/                 # Auth Service (Port 50090)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── admin.rs
│   │       ├── audit.rs
│   │       ├── auth_service.rs
│   │       ├── certificates.rs
│   │       ├── delegation.rs
│   │       ├── jwt.rs
│   │       ├── lib.rs
│   │       ├── main.rs
│   │       ├── middleware.rs
│   │       ├── rbac.rs
│   │       ├── secrets_client.rs
│   │       ├── service_mesh.rs
│   │       └── token_manager.rs
│   │
│   ├── 📁 body-kb-rs/                      # Body KB Service (Port 50058)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       ├── rules_engine.rs
│   │       ├── state_store.rs
│   │       └── validation.rs
│   │
│   ├── 📁 context-manager-rs/              # Context Manager Service (Port 50064)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── lib.rs
│   │       └── main.rs
│   │
│   ├── 📁 curiosity-engine-rs/             # Curiosity Engine Service (Port 50076)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       └── knowledge_gap_analyzer.rs
│   │
│   ├── 📁 data-router-rs/                  # Data Router Service (Port 50052)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── circuit_breaker.rs
│   │       ├── kb_clients.rs
│   │       ├── language_detector.rs
│   │       ├── lib.rs
│   │       ├── main.rs
│   │       ├── router.rs
│   │       └── tests/
│   │           └── scope_isolation_tests.rs
│   │
│   ├── 📁 error-handling-rs/                # Error Handling Library
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── circuit_breaker.rs
│   │       ├── context.rs
│   │       ├── fallback.rs
│   │       ├── lib.rs
│   │       ├── logging.rs
│   │       ├── monitoring.rs
│   │       ├── reporting.rs
│   │       ├── retry.rs
│   │       ├── sanitization.rs
│   │       ├── supervisor.rs
│   │       └── types.rs
│   │
│   ├── 📁 executor-rs/                     # Executor Service (Port 50062)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── security_test.py
│   │   ├── security_validation_report.json
│   │   ├── security_validation_simple.py
│   │   ├── security_validation.py
│   │   ├── test_client.py
│   │   ├── src/
│   │   │   ├── execution_logic.rs
│   │   │   ├── lib.rs
│   │   │   ├── main.rs
│   │   │   ├── recovery_logger.rs
│   │   │   └── windows_executor.rs
│   │   └── tests/
│   │       └── service_recovery_tests.rs
│   │
│   ├── 📁 heart-kb-rs/                     # Heart KB Service (Port 50059)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       ├── rules_engine.rs
│   │       ├── state_store.rs
│   │       └── validation.rs
│   │
│   ├── 📁 input-validation-rs/             # Input Validation Library
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── builder.rs
│   │       ├── errors.rs
│   │       ├── lib.rs
│   │       ├── schema.rs
│   │       ├── sanitizers/
│   │       │   ├── command.rs
│   │       │   ├── html.rs
│   │       │   ├── mod.rs
│   │       │   ├── path.rs
│   │       │   └── string.rs
│   │       └── validators/
│   │           ├── generic.rs
│   │           ├── mod.rs
│   │           ├── numeric.rs
│   │           ├── path.rs
│   │           ├── redos.rs
│   │           ├── security.rs
│   │           ├── string.rs
│   │           └── url.rs
│   │
│   ├── 📁 llm-service-rs/                  # LLM Service (Port 50053)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── llm_client.rs
│   │       ├── main.rs
│   │       ├── prompt_manager.rs
│   │       ├── secrets_client.rs
│   │       └── tests.rs
│   │
│   ├── 📁 log-analyzer-rs/                 # Log Analyzer Service (Port 50075)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── 📁 logging-service-rs/              # Logging Service (Port 50056)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── cost_tracker.rs
│   │       ├── log_handler.rs
│   │       └── main.rs
│   │
│   ├── 📁 mind-kb-rs/                      # Mind KB Service (Port 50057)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── graph_db.rs
│   │       ├── main.rs
│   │       ├── queries.rs
│   │       ├── text_preprocessor.rs
│   │       ├── validation.rs
│   │       └── vector_store.rs
│   │
│   ├── 📁 orchestrator-service-rs/         # Orchestrator Service (Port 50051)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── api_client.rs
│   │       ├── main.rs
│   │       ├── pipeline.rs
│   │       └── tests/
│   │           ├── plan_and_execute_e2e.rs
│   │           └── registry_integration_tests.rs
│   │
│   ├── 📁 persistence-kb-rs/               # Persistence KB Service (Port 50071)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       └── snapshot.rs
│   │
│   ├── 📁 reflection-rs/                   # Reflection Library (Port 50065)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── 📁 reflection-service-rs/           # Reflection Service (Port 50065)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── logging_client.rs
│   │       ├── main.rs
│   │       ├── reflection_logic.rs
│   │       ├── service.rs
│   │       ├── soul_kb_client.rs
│   │       └── tests.rs
│   │
│   ├── 📁 safety-service-rs/                # Safety Service (Port 50055)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── filters.rs
│   │       ├── main.rs
│   │       ├── policy_engine.rs
│   │       ├── soul_config.rs
│   │       ├── threat_filter.rs
│   │       └── validation.rs
│   │
│   ├── 📁 scheduler-rs/                     # Scheduler Service (Port 50066)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       └── main.rs
│   │
│   ├── 📁 secrets-service-rs/              # Secrets Service (Port 50080)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── auth.rs
│   │       ├── main.rs
│   │       ├── service.rs
│   │       └── vault_client.rs
│   │
│   ├── 📁 self-improve-rs/                  # Self-Improvement Library
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── adaptation.rs
│   │       ├── classifier.rs
│   │       ├── error_record.rs
│   │       ├── lib.rs
│   │       ├── model.rs
│   │       ├── repository.rs
│   │       └── tests.rs
│   │
│   ├── 📁 sensor-rs/                        # Sensor Library (Client)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       └── system_monitor.rs
│   │
│   ├── 📁 shared-types-rs/                  # Shared Types Library
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── config.rs
│   │       ├── lib.rs
│   │       └── secrets.rs
│   │
│   ├── 📁 social-kb-rs/                     # Social KB Service (Port 50060)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       ├── social_graph.rs
│   │       ├── theory_of_mind.rs
│   │       └── validation.rs
│   │
│   ├── 📁 soul-kb-rs/                       # Soul KB Service (Port 50061)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       └── validation.rs
│   │
│   ├── 📁 tools-service-rs/                 # Tools Service (Port 50054)
│   │   ├── build.rs
│   │   ├── Cargo.toml
│   │   ├── Dockerfile.dev
│   │   ├── README.md
│   │   └── src/
│   │       ├── main.rs
│   │       ├── tool_manager.rs
│   │       ├── tools.rs
│   │       └── validation.rs
│   │
│   └── 📁 tool-sdk/                         # Tool SDK Library
│       ├── Cargo.toml
│       ├── README.md
│       ├── docs/
│       │   ├── ARCHITECTURE.md
│       │   ├── ERROR_HANDLING.md
│       │   ├── EXTENDING.md
│       │   ├── INTEGRATION_GUIDE.md
│       │   ├── README.md
│       │   └── TROUBLESHOOTING.md
│       ├── examples/
│       │   ├── openai_completion.rs
│       │   ├── resilience_demo.rs
│       │   └── serpapi_search.rs
│       └── src/
│           ├── config/
│           │   └── mod.rs
│           ├── core/
│           │   ├── builder.rs
│           │   └── mod.rs
│           ├── error/
│           │   ├── mapping.rs
│           │   └── mod.rs
│           ├── lib.rs
│           ├── resilience/
│           │   ├── circuit_breaker.rs
│           │   ├── mod.rs
│           │   └── retry.rs
│           ├── services/
│           │   ├── common.rs
│           │   ├── mod.rs
│           │   ├── openai/
│           │   │   ├── mod.rs
│           │   │   └── models.rs
│           │   └── serpapi/
│           │       ├── mod.rs
│           │       └── models.rs
│           └── tests/
│               ├── config_extension_tests.rs
│               ├── config_tests.rs
│               ├── core_tests.rs
│               ├── error_extension_tests.rs
│               ├── error_tests.rs
│               ├── integration_tests.rs
│               ├── mod.rs
│               ├── openai_mock_tests.rs
│               ├── README.md
│               ├── resilience_extension_tests.rs
│               ├── resilience_tests.rs
│               └── serpapi_mock_tests.rs
│
├── 📁 CONFIGURATION
│   └── 📁 config/
│       ├── agent_registry.toml
│       ├── phoenix_api_keys.txt
│       ├── phoenix.toml
│       ├── README.md
│       └── test_api_keys.txt
│
├── 📁 INFRASTRUCTURE
│   │
│   ├── 📁 certs/                            # Certificate Management
│   │   ├── generate_certs.sh
│   │   └── README.md
│   │
│   ├── 📁 docker/                           # Docker Configuration
│   │   ├── docker-compose.dev.yml
│   │   ├── docker-compose.monitoring.yml
│   │   ├── docker-compose.security-metrics.yml
│   │   ├── docker-compose.yml
│   │   ├── Dockerfile.template
│   │   └── README.md
│   │
│   ├── 📁 k8s/                              # Kubernetes Manifests
│   │   ├── 00-namespace.yml
│   │   ├── 01-resource-quotas.yml
│   │   ├── 02-network-policies.yml
│   │   ├── 03-pod-security.yml
│   │   ├── 04-autoscaling.yml
│   │   ├── 05-volume-security.yml
│   │   └── README.md
│   │
│   ├── 📁 monitoring/                       # Monitoring Configuration
│   │   ├── alertmanager/
│   │   │   └── alertmanager.yml
│   │   ├── dashboards/
│   │   │   └── circuit-breaker-dashboard.json
│   │   ├── prometheus/
│   │   │   ├── prometheus.yml
│   │   │   └── rules/
│   │   │       └── resource-alerts.yml
│   │   └── README.md
│   │
│   └── 📁 scripts/                          # Utility Scripts
│       ├── deploy.sh
│       ├── install_protoc.ps1
│       ├── install_protoc.sh
│       └── README.md
│
├── 📁 PROTOCOL BUFFERS
│   │
│   ├── 📁 phoenix_orch_proto/               # Generated Proto Files
│   │   └── (generated files)
│   │
│   └── 📁 protoc/                           # Protocol Buffer Compiler
│       ├── bin/
│       │   └── protoc.exe
│       ├── include/
│       │   └── google/
│       │       └── protobuf/
│       │           ├── any.proto
│       │           ├── api.proto
│       │           ├── compiler/
│       │           │   └── plugin.proto
│       │           ├── descriptor.proto
│       │           ├── duration.proto
│       │           ├── empty.proto
│       │           ├── field_mask.proto
│       │           ├── source_context.proto
│       │           ├── struct.proto
│       │           ├── timestamp.proto
│       │           ├── type.proto
│       │           └── wrappers.proto
│       ├── README.md
│       └── readme.txt
│
├── 📁 FRONTEND
│   └── 📁 frontend/                         # Next.js Frontend Application
│       ├── app/
│       │   ├── dashboard/
│       │   │   └── page.tsx
│       │   ├── favicon.ico
│       │   ├── globals.css
│       │   ├── layout.tsx
│       │   ├── page.tsx
│       │   └── settings/
│       │       └── page.tsx
│       ├── components/
│       │   ├── dashboard/
│       │   │   └── ChatContainer.tsx
│       │   └── ui/
│       │       ├── ChatInput.tsx
│       │       ├── ChatMessage.tsx
│       │       ├── ErrorBoundary.tsx
│       │       └── Header.tsx
│       ├── lib/
│       │   ├── api/
│       │   │   └── orchService.ts
│       │   ├── hooks/
│       │   │   └── useErrorHandler.ts
│       │   └── utils/
│       │       └── apiKeyStorage.ts
│       ├── public/
│       │   ├── file.svg
│       │   ├── globe.svg
│       │   ├── next.svg
│       │   ├── vercel.svg
│       │   └── window.svg
│       ├── types/
│       │   └── index.ts
│       ├── utils/
│       │   └── api.ts
│       ├── eslint.config.mjs
│       ├── next-env.d.ts
│       ├── next.config.ts
│       ├── package.json
│       ├── package-lock.json
│       ├── postcss.config.mjs
│       ├── README.md
│       └── tsconfig.json
│
├── 📁 DOCUMENTATION
│   └── 📁 docs/
│       ├── architecture/
│       │   ├── ARCHITECTURE_WORKFLOW.md
│       │   ├── emergency_resilience_implementation_plan.md
│       │   ├── emergency_resilience_spec.md
│       │   ├── FEATURE_INTEGRATION.md
│       │   ├── FINAL_PORT_MAP.md
│       │   ├── port_map.md
│       │   ├── RSI_CLOSED_LOOP_IMPLEMENTATION.md
│       │   └── UI_INTEGRATION.md
│       ├── deployment/
│       │   ├── DEPLOYMENT.md
│       │   ├── phoenix_orch_production_readiness_assessment.md
│       │   └── RSI_ENV_CONFIGURATION.md
│       ├── prompts/
│       │   ├── Master_System_Prompt_AGI_System_Build.md
│       │   └── MASTER_SYSTEM_PROMPT.md
│       ├── security/
│       │   ├── CONTAINER_SECURITY_CHECKLIST.md
│       │   ├── CONTAINER_SECURITY_GUIDE.md
│       │   ├── input-validation-framework.md
│       │   ├── secret-management.md
│       │   ├── security-audit-template.md
│       │   ├── security-framework.md
│       │   └── security-metrics-dashboard.md
│       ├── status/
│       │   ├── AGI_System_Build_Status.md
│       │   ├── customization_summary.md
│       │   ├── NEXT_STEPS.md
│       │   ├── PROJECT_COMPLETION_SUMMARY.md
│       │   └── verification_plan.md
│       ├── cicd-documentation.md
│       ├── environment-configuration-guide.md
│       └── service_ports.md
│
├── 📁 LOAD TESTING
│   └── 📁 load-testing/
│       ├── chaos/
│       │   ├── chaos-runner.sh
│       │   ├── chaos-scenarios.json
│       │   └── orchestrate-chaos-tests.sh
│       ├── configs/
│       │   ├── grafana/
│       │   │   ├── dashboards/
│       │   │   │   ├── k6-performance-dashboard.json
│       │   │   │   ├── load-testing-comprehensive-dashboard.json
│       │   │   │   ├── phoenix-services-dashboard.json
│       │   │   │   └── security-metrics-dashboard.json
│       │   │   └── provisioning/
│       │   │       ├── dashboards/
│       │   │       │   └── dashboards.yml
│       │   │       └── datasources/
│       │   │           └── datasources.yml
│       │   ├── prometheus/
│       │   │   └── security-metrics.yml
│       │   ├── prometheus.yml
│       │   └── statsd-mapping.conf
│       ├── scenarios/
│       │   ├── baseline.js
│       │   ├── kb-services-test.js
│       │   ├── llm-service-test.js
│       │   ├── stress-test.js
│       │   └── user-journey-test.js
│       ├── scripts/
│       │   ├── aggregate-metrics.js
│       │   ├── analyze-results.sh
│       │   ├── common.js
│       │   └── compare-benchmarks.sh
│       ├── security-metrics-exporter/
│       │   ├── Dockerfile
│       │   └── security_metrics_exporter.py
│       ├── results/
│       ├── docker-compose.yml
│       ├── Dockerfile
│       ├── entrypoint.sh
│       ├── QUICK-REFERENCE.md
│       ├── README.md
│       └── run-test.sh
│
└── 📁 BUILD ARTIFACTS
    ├── 📁 target/                           # Rust build artifacts (gitignored)
    └── 📁 frontend/node_modules/            # Node.js dependencies (gitignored)
```

## Repository Statistics

### Service Crates: 31
- **Control Plane**: 6 services
- **RSI Layer**: 2 services
- **Cognitive Layer**: 6 KB services
- **Functional Layer**: 6 services
- **Security & Infrastructure**: 2 services
- **Gateway**: 1 service
- **Shared Libraries**: 8 crates

### Total Files (Approximate)
- **Rust Source Files**: ~200+
- **TypeScript/TSX Files**: ~15
- **Configuration Files**: ~20
- **Documentation Files**: ~50
- **Test Files**: ~30
- **Scripts**: ~10
- **Docker/K8s Configs**: ~15

### Key Directories
- **Service Crates**: 31 Rust microservices
- **Documentation**: Comprehensive docs in `docs/`
- **Infrastructure**: Docker, K8s, monitoring configs
- **Frontend**: Next.js React application
- **Load Testing**: K6 test scenarios and configs
- **Protocol Buffers**: Proto definitions and compiler

## Module Categories

### 🔴 Core Services (Ports 50051-50070)
- Orchestrator, Data Router, LLM, Tools, Safety, Logging
- Mind-KB, Body-KB, Heart-KB, Social-KB, Soul-KB, Persistence-KB
- Context Manager, Reflection, Scheduler, Agent Registry

### 🟡 RSI Services (Ports 50075-50076)
- Log Analyzer, Curiosity Engine

### 🟢 Security Services (Ports 50080-50090)
- Secrets Service, Auth Service

### 🔵 Gateway (Port 8000)
- API Gateway

### 🟣 Libraries (No Ports)
- action-ledger-rs, error-handling-rs, input-validation-rs
- self-improve-rs, sensor-rs, shared-types-rs, config-rs, tool-sdk

