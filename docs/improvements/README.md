# Roubit — Improvements, Bug Fixes & Stabilization Index (`docs/improvements/`)

This directory contains all modular engineering improvement and bug-fix activities mapped during the full-stack technical audit of **Roubit**. Keeping these activities in `docs/improvements/` preserves [`docs/features/`](file:///home/filipe/Documents/projects/roubit/docs/features/README.md) as the historical record of initial feature development while providing a prioritized roadmap (`P0` to `P3`) for stabilization, dual-database support (SQLite in Local Dev, PostgreSQL in Prod/Test), security hardening, and UX polish.

Each activity can be executed independently in priority order using the `deliver` skill (e.g., `deliver improvement 01.1` or `deliver docs/improvements/01-critical-data-and-vault-fixes/activity-01-frontend-build-tsconfig-and-tailwind-fixes.prompt`).

---

## Priority Phases & Direct Links

### 🔴 [Phase 01: Critical Data, Vault, Notifications & Build Fixes (`P0`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/01-critical-data-and-vault-fixes)
1. [Activity 01: Frontend Build (`tsconfig.app.json`) & Tailwind Utilities Restoration](file:///home/filipe/Documents/projects/roubit/docs/improvements/01-critical-data-and-vault-fixes/activity-01-frontend-build-tsconfig-and-tailwind-fixes.prompt)
2. [Activity 02: SQLite Transaction Safety (`RAII`) & Habit Streak Decay Engine](file:///home/filipe/Documents/projects/roubit/docs/improvements/01-critical-data-and-vault-fixes/activity-02-sqlite-transaction-safety-and-streak-decay.prompt)
3. [Activity 03: Encrypted Journal Vault Cross-Session Salt Derivation, Auto-Lock & AAD](file:///home/filipe/Documents/projects/roubit/docs/improvements/01-critical-data-and-vault-fixes/activity-03-journal-vault-salt-derivation-and-autolock.prompt)
4. [Activity 04: Reminder Worker SQL Fixes, Redis Scheduling, SSE Timeout & Nginx Buffering](file:///home/filipe/Documents/projects/roubit/docs/improvements/01-critical-data-and-vault-fixes/activity-04-reminder-worker-sql-sse-and-nginx-fixes.prompt)

---

### 🔴 [Phase 02: Dual-Database Architecture — SQLite (Local Dev) & PostgreSQL (Prod/Test) (`P0/P1`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/02-dual-database-postgres-prod)
1. [Activity 01: PostgreSQL Migrations & `migrate_sqlite_to_postgres` Data Migrator Fixes](file:///home/filipe/Documents/projects/roubit/docs/improvements/02-dual-database-postgres-prod/activity-01-postgres-migrations-and-data-migrator-fixes.prompt)
2. [Activity 02: PostgreSQL Domain Repositories Implementation (`roubit-core`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/02-dual-database-postgres-prod/activity-02-postgres-domain-repositories-core.prompt)
3. [Activity 03: Server Dynamic DB Wiring, Docker Compose Alignment & `README.md` Startup Guide](file:///home/filipe/Documents/projects/roubit/docs/improvements/02-dual-database-postgres-prod/activity-03-server-dynamic-db-wiring-and-docker-compose.prompt)
4. [Activity 04: PostgreSQL & Redis Automated Integration Test Suite](file:///home/filipe/Documents/projects/roubit/docs/improvements/02-dual-database-postgres-prod/activity-04-postgres-and-redis-integration-test-suite.prompt)

---

### 🟠 [Phase 03: Security, Authentication & Idempotency Hardening (`P1`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/03-security-and-idempotency)
1. [Activity 01: Idempotency Middleware Security Hardening (Body Limit, Path Validation & Lock Hygiene)](file:///home/filipe/Documents/projects/roubit/docs/improvements/03-security-and-idempotency/activity-01-idempotency-middleware-security-hardening.prompt)
2. [Activity 02: Argon2id `spawn_blocking`, Login Timing Attack Defense & Log Header Redaction](file:///home/filipe/Documents/projects/roubit/docs/improvements/03-security-and-idempotency/activity-02-auth-argon2-blocking-and-timing-defense.prompt)
3. [Activity 03: Rate Limiting Middleware, Secure Auth Cookies & Complete Session Logout](file:///home/filipe/Documents/projects/roubit/docs/improvements/03-security-and-idempotency/activity-03-rate-limiting-secure-cookies-and-logout.prompt)

---

### 🟢 [Phase 04: Full-Stack Domain Integration & Timezone Awareness (`P2`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/04-fullstack-domain-integration)
1. [Activity 01: Daily Routine Progress Hydration, `Skipped` Status & Routine Modal Editor](file:///home/filipe/Documents/projects/roubit/docs/improvements/04-fullstack-domain-integration/activity-01-routine-persistence-skip-status-and-editor.prompt)
2. [Activity 02: Habit Analytics REST Endpoint, Real 365-Day Heatmap & Numeric/Timer Check-ins](file:///home/filipe/Documents/projects/roubit/docs/improvements/04-fullstack-domain-integration/activity-02-habit-analytics-endpoint-and-real-heatmap.prompt)
3. [Activity 03: Atomic OKR Creation & Key Result Habit-Link Persistence Endpoint](file:///home/filipe/Documents/projects/roubit/docs/improvements/04-fullstack-domain-integration/activity-03-okr-atomic-creation-and-habit-link-endpoint.prompt)
4. [Activity 04: Local Timezone (`X-Timezone`) Support & PWA Offline Sync Refresh](file:///home/filipe/Documents/projects/roubit/docs/improvements/04-fullstack-domain-integration/activity-04-timezone-awareness-and-pwa-offline-sync.prompt)

---

### 🟡 [Phase 05: CLI Hybrid Remote Mode Completion (`P2`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/05-cli-hybrid-remote-completion)
1. [Activity 01: CLI `HttpClient` REST Routes & DTO Contract Alignment](file:///home/filipe/Documents/projects/roubit/docs/improvements/05-cli-hybrid-remote-completion/activity-01-cli-remote-dto-and-endpoint-alignment.prompt)
2. [Activity 02: Remote Mode Support for `routine`, `okr`, `journal` & Persistent `config set`](file:///home/filipe/Documents/projects/roubit/docs/improvements/05-cli-hybrid-remote-completion/activity-02-cli-remote-routine-okr-journal-commands.prompt)
3. [Activity 03: Full CLI Remote Mode End-to-End Automated Test Suite](file:///home/filipe/Documents/projects/roubit/docs/improvements/05-cli-hybrid-remote-completion/activity-03-cli-remote-full-e2e-test-suite.prompt)

---

### 🟢 [Phase 06: UX Polish, Accessibility (a11y), View Testing & Documentation (`P3`)](file:///home/filipe/Documents/projects/roubit/docs/improvements/06-ux-accessibility-qa-and-docs)
1. [Activity 01: RFC 7807 Error Toasts, Modal Form Preservation & Audio/Listener Leak Fixes](file:///home/filipe/Documents/projects/roubit/docs/improvements/06-ux-accessibility-qa-and-docs/activity-01-frontend-error-toasts-modals-and-memory-leaks.prompt)
2. [Activity 02: Accessibility (ARIA Dialog Semantics, Focus Trap & Keyboard Navigation)](file:///home/filipe/Documents/projects/roubit/docs/improvements/06-ux-accessibility-qa-and-docs/activity-02-accessibility-aria-and-keyboard-navigation.prompt)
3. [Activity 03: Vitest Integration Tests for All 5 Views & ESLint CI Pipeline Integration](file:///home/filipe/Documents/projects/roubit/docs/improvements/06-ux-accessibility-qa-and-docs/activity-03-frontend-views-vitest-and-eslint-ci.prompt)
4. [Activity 04: Load Benchmarks Fix (`k6` / `P99`), Workspace Cleanup & Documentation Consolidation](file:///home/filipe/Documents/projects/roubit/docs/improvements/06-ux-accessibility-qa-and-docs/activity-04-benchmarks-cleanup-and-docs-consolidation.prompt)

