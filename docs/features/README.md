# Roubit — Features & Engineering Activities Index

This directory organizes all engineering features for the Roubit project. Each feature contains modular, detailed activity specifications (3 to 6 activities per feature) written in English following industry best practices and modern Rust / Vue 3 engineering standards.

---

## Feature Matrix & Direct Links

### [01. Workspace Setup & Tooling](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup)
1. [Activity 01: Cargo Workspace Initialization & Monorepo Structure](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-01-cargo-workspace-init.prompt)
2. [Activity 02: Linting, Code Formatting & Quality Standards](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-02-linting-formatting-git-hooks.prompt)
3. [Activity 03: GitHub Actions Continuous Integration (CI) Pipeline](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-03-github-actions-ci-pipeline.prompt)
4. [Activity 04: Environment Configuration & Structured Tracing Foundation](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-04-environment-configuration-logging.prompt)

---

### [02. Selective Cryptography & Memory Safety](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography)
1. [Activity 01: Argon2id Master Key Derivation](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-01-argon2id-key-derivation.prompt)
2. [Activity 02: Authenticated AES-256-GCM Encryption & Decryption](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-02-aes-256-gcm-cipher.prompt)
3. [Activity 03: Memory Sanitization with Zeroize](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-03-zeroize-memory-safety.prompt)
4. [Activity 04: Cryptographic Test Vectors & Benchmarks](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-04-cryptographic-test-vectors-benchmarks.prompt)

---

### [03. Routine Management & Time Blocks](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines)
1. [Activity 01: Routine Domain Models & SQLite Schema](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-01-routine-domain-models-schema.prompt)
2. [Activity 02: Routine SQLite Repository with WAL Mode](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-02-sqlite-repository-wal.prompt)
3. [Activity 03: Time Block Validation & Overlap Engine](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-03-time-block-validation-overlap-engine.prompt)
4. [Activity 04: Daily Routine Execution & Progress Tracker](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-04-daily-routine-progress-tracker.prompt)
5. [Activity 05: Routine Integration Tests & Use Case Validation](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-05-routine-unit-integration-tests.prompt)

---

### [04. Habit Tracking & Consistency Analytics](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits)
1. [Activity 01: Habit Domain Models & SQLite Schema](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-01-habit-domain-models-schema.prompt)
2. [Activity 02: Habit SQLite Repository with Atomic Check-Ins](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-02-habit-repository-sqlx.prompt)
3. [Activity 03: Resilient Streak Calculation Engine](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-03-streak-calculation-engine.prompt)
4. [Activity 04: Habit Adherence Analytics & Contribution Heatmap](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-04-adherence-metrics-analytics.prompt)
5. [Activity 05: Habit Unit & Integration Test Suite](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-05-habit-unit-integration-tests.prompt)

---

### [05. Goals & OKR Alignment Engine](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs)
1. [Activity 01: OKR Domain Models & SQLite Schema](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-01-okr-domain-models-schema.prompt)
2. [Activity 02: OKR SQLite Repository with SQLx](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-02-okr-repository-sqlx.prompt)
3. [Activity 03: Habit-to-KR Linkage & Progress Computation Engine](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-03-habit-to-kr-linkage-progress-engine.prompt)
4. [Activity 04: OKR Integration Tests & Cascade Validation](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-04-okr-unit-integration-tests.prompt)

---

### [06. Encrypted Journaling & Dual-Storage Architecture](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling)
1. [Activity 01: Journal Domain Models & Dual-Storage Schema](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-01-journal-domain-models-schema.prompt)
2. [Activity 02: Journal SQLite Repository with Metadata Indexing](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-02-journal-repository-sqlx.prompt)
3. [Activity 03: Journal Cryptographic Application Service](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-03-journal-service-crypto-integration.prompt)
4. [Activity 04: Journal Integration Tests & Zero-Knowledge Validation](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-04-journal-unit-integration-tests.prompt)

---

### [07. CLI Application & Terminal UI](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli)
1. [Activity 01: Modular CLI Command Tree with Clap](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-01-clap-command-structure.prompt)
2. [Activity 02: Terminal UI Formatting, Tables & Visual Metrics](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-02-terminal-ui-tables-colors.prompt)
3. [Activity 03: Interactive Prompts & Terminal Journal Editor](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-03-interactive-prompts-journal-editor.prompt)
4. [Activity 04: Hybrid Mode Adapter (Local SQLite & Remote HTTP API)](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-04-hybrid-mode-local-http-adapter.prompt)
5. [Activity 05: CLI End-to-End Automated Tests](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-05-cli-e2e-automated-tests.prompt)

---

### [08. REST API Service & Idempotency](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest)
1. [Activity 01: Axum HTTP Server Architecture, Tracing & Error Handling](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-01-axum-server-architecture-tracing.prompt)
2. [Activity 02: Authentication, JWT & OWASP Security Headers](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-02-authentication-jwt-security-headers.prompt)
3. [Activity 03: Idempotency-Key Interceptor & Guard Middleware](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-03-idempotency-middleware-rf05.prompt)
4. [Activity 04: REST Controllers for Routines, Habits & OKRs](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-04-domain-endpoints-routines-habits-okrs.prompt)
5. [Activity 05: Zero-Knowledge Journal REST Controllers](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-05-journal-encrypted-payload-endpoints.prompt)
6. [Activity 06: REST API Integration & SAST Security Scanning](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-06-api-integration-sast-tests.prompt)

---

### [09. Frontend SPA & Web Crypto](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa)
1. [Activity 01: Vue 3 SPA Scaffolding, TypeScript & TailwindCSS](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-01-vite-vue3-tailwind-scaffolding.prompt)
2. [Activity 02: HTTP Client, Idempotency-Key Interceptor & Pinia Stores](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-02-api-client-idempotency-state-management.prompt)
3. [Activity 03: Web Crypto API Client-Side Selective Encryption](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-03-web-crypto-selective-encryption.prompt)
4. [Activity 04: Daily Dashboard Component (Routines & Habits Integration)](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-04-daily-dashboard-routines-habits.prompt)
5. [Activity 05: Habit Analytics, Heatmap & OKR Alignment Views](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-05-habits-heatmap-okrs-views.prompt)
6. [Activity 06: Encrypted Journaling UI & Vitest Component Tests](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-06-encrypted-journal-component-tests.prompt)

---

### [10. Infrastructure, Redis & PostgreSQL Transition](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres)
1. [Activity 01: Redis Connection Pooling, Distributed Locks & Idempotency Cache](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-01-redis-connection-distributed-locks.prompt)
2. [Activity 02: Scheduled Reminders via Redis Keyspace Notifications & TTL](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-02-scheduled-reminders-ttl-keyspace.prompt)
3. [Activity 03: Dual-Database Support (SQLite & PostgreSQL) with SQLx](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-03-dual-database-postgresql-support.prompt)
4. [Activity 04: Multi-Stage Dockerfiles & Docker Compose Orchestration](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-04-docker-compose-production-orchestration.prompt)
5. [Activity 05: Load Testing, Memory Profiling & P99 Benchmarks](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-05-load-testing-p99-benchmarks.prompt)
