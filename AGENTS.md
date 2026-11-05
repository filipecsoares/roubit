# Roubit — Agent & LLM Development Guidelines

> This document is the primary instruction manual and architectural reference for AI coding agents and LLMs working on the **Roubit** repository. Follow these principles, conventions, and invariants strictly when reading, planning, modifying, or testing code.

---

## 1. Project Mission & Overview

**Roubit** (*Routine + Habit*) is a high-performance personal operating system and an end-to-end software engineering study laboratory. The project models real-world production engineering challenges in modern distributed and offline-first systems:

1. **Chronological Routines & Focus Blocks:** Time-blocking schedule with real-time day tracking, $O(N \log N)$ non-overlap validation, and an integrated fullscreen Pomodoro Focus Mode with native Web Audio synthesis.
2. **Smart Habits & Streak Engine:** Atomic $O(1)$ denormalized streak calculations, multi-tiered in-memory and Redis L2 caching, and GitHub-style 52-week momentum heatmaps.
3. **Goal Alignment (OKRs):** Strategic quarterly Objectives and Key Results with direct habit linking and automated progress roll-up.
4. **Zero-Knowledge Encrypted Journal:** Client-side encryption (AES-256-GCM + Argon2id) where reflections are encrypted before transit. The server stores only ciphertext envelopes; reflections cannot be read even under full database exfiltration.
5. **Offline-First PWA:** Monotonic IndexedDB mutation queue managed by Service Worker (Workbox), with idempotent automatic replay upon network recovery.
6. **Hybrid Terminal CLI:** Standalone command-line interface supporting direct local SQLite operations or remote authenticated HTTP REST API access.

---

## 2. Architecture & Repository Structure

Roubit enforces a strict **Clean Architecture** with separation of concerns:

```text
roubit/
├── api/                             # Rust Multi-Crate Workspace (Edition 2021, Rust 1.85+)
│   ├── Cargo.toml                   # Root workspace manifest with centralized dependencies
│   └── crates/
│       ├── core/                    # Pure domain models, business logic, repository traits,
│       │                            # SQLite (WAL) & PostgreSQL implementations, crypto primitives
│       ├── server/                  # Axum 0.8 REST API, JWT auth, SSE streaming, middleware,
│       │                            # Idempotency cache, Problem Details (RFC 7807)
│       └── cli/                     # Terminal CLI (Clap, comfy-table, indicatif, remote HTTP client)
├── frontend/                        # Vue 3.5 PWA (Composition API, TypeScript 5, Vite, Tailwind CSS)
│   ├── src/
│   │   ├── components/              # Modular UI components (dashboard, habits, okrs, journal)
│   │   ├── composables/             # Shared logic (useModal, useToast, usePwa, useCrypto)
│   │   ├── stores/                  # Pinia state management (routine, habit, okr, journal, focus, auth)
│   │   ├── views/                   # Route views (DashboardView, HabitsView, OkrsView, JournalView, LoginView)
│   │   └── services/                # Axios HTTP client, offline IndexedDB queue, SSE listener
│   └── src/views/__tests__/         # Vitest component and view integration test suites
├── docs/                            # Architecture specs, development plans, load benchmarks
├── scripts/                         # QA automation (pre-commit.sh)
├── tests/load/                      # K6 load testing scenarios
└── docker-compose.yml               # Production stack orchestration (PostgreSQL 16 + Redis 7 + API + Nginx)
```

---

## 3. Core Architectural Invariants (Non-Negotiable)

When developing any feature or fix, you **must preserve** these core invariants:

### 🔒 1. Zero-Knowledge Cryptographic Privacy
- **Client-Side Derivation & Encryption:** Journal reflections must be encrypted in the client browser or CLI using **AES-256-GCM** with a 12-byte random IV. Keys are derived locally using **Argon2id** (RFC 9106) with unique per-entry 16-byte random salts.
- **Zero Plaintext on Server:** The backend and database must NEVER receive, log, or persist plaintext reflection content. The payload must strictly consist of `ciphertext_base64`, `nonce_base64`, and `salt_base64` alongside indexed metadata (`entry_date`, `mood_score`, `tags`).
- **Memory Hygiene:** In-memory crypto keys must be scrubbed upon vault lock or drop (Rust `zeroize::Zeroize` / Vue `vaultStore.lock()`).

### 🛡️ 2. Multi-Tenant Row-Level Isolation
- Every database query, repository method, and domain service operation must be explicitly scoped by `user_id`.
- Never trust client IDs blindly: verify ownership and prevent Insecure Direct Object References (IDOR) across Routines, Habits, OKRs, and Journal entries.

### ⚡ 3. Tail Latency & Threadpool Hygiene
- Target: sub-5ms P99 tail latency and $< 15\,\text{MB}$ resident memory (RSS).
- **Never block Tokio async worker threads:** Heavy cryptographic operations (such as Argon2id password hashing/verification) MUST be offloaded to `tokio::task::spawn_blocking`.
- **Constant-Time Timing Defenses:** Failed login attempts against non-existent users must execute an identical Argon2id dummy verification in constant time to prevent user enumeration.

### 🔄 4. Monotonic Idempotency & Offline Replay
- All mutative API endpoints (POST/PUT/DELETE) must support the `Idempotency-Key` header (UUIDv4).
- Mutations generated offline are persisted to **IndexedDB** in FIFO sequence and replayed idempotently upon network restoration without duplicate records.

### 🗄️ 5. Dual-Database Engine Parity
- Roubit supports **SQLite in WAL mode** for local development/single-user CLI and **PostgreSQL 16** for distributed deployments.
- When modifying database schemas or repository methods, ensure compatibility across both engines using SQLx migrations and repository implementations in `api/crates/core/src/domain/*/repository/`.

---

## 4. Development & Execution Runbook

### Prerequisites
- **Rust:** 1.85.0+ (`rustup default stable`)
- **Node.js:** 20+ and npm 10+
- **Docker:** (Optional) Docker 24+ and Docker Compose v2.20+

### Option A: Native Development (SQLite WAL — Fastest & Zero Dependencies)
```bash
# 1. Start Backend API Server (Terminal 1)
# Automatically creates 'roubit.db' with WAL mode and applies migrations
cargo run --bin roubit-server --manifest-path api/Cargo.toml
# Listens on http://127.0.0.1:8080

# 2. Start Frontend Dev Server (Terminal 2)
cd frontend
npm install
npm run dev
# Listens on http://localhost:5173 (proxies API and SSE streams to backend)
```

### Option B: Full Stack via Docker Compose
```bash
docker compose up -d --build
# Web SPA on http://localhost (Port 80 via Nginx)
# Backend API on http://localhost:8080
# Liveness probe: curl http://localhost:8080/health
# Readiness probe: curl http://localhost:8080/ready
```

### Option C: Terminal CLI Execution
```bash
# Local embedded SQLite mode:
cargo run --bin roubit --manifest-path api/Cargo.toml -- habit list

# Remote authenticated HTTP mode:
cargo run --bin roubit --manifest-path api/Cargo.toml -- auth login --url http://127.0.0.1:8080 --email dev@roubit.app
cargo run --bin roubit --manifest-path api/Cargo.toml -- habit list --remote http://127.0.0.1:8080
```

---

## 5. Frontend Guidelines (Vue 3 + TypeScript)

- **Component Architecture:** Use Vue 3 Composition API with `<script setup lang="ts">`.
- **Modal Accessibility (WAI-ARIA):** Modals must use the `useModal` composable to guarantee proper ARIA attributes (`role="dialog"`, `aria-modal="true"`, `aria-labelledby`), automated focus trapping, `Escape` key listeners, and inert background handling.
- **Error Handling & Feedback:** API error responses adhere to **RFC 7807 (Problem Details)**. Display user-friendly error messages via `useToast` (`showError(error.response?.data?.detail || fallback)`).
- **Form State Preservation:** Preserve user inputs in modal forms upon validation errors or unexpected failures. Never clear forms unless the operation succeeds.
- **State Management:** Use Pinia stores (`routine`, `habit`, `okr`, `journal`, `focus`, `notification`, `auth`). Keep business logic in stores and presentation in components.

---

## 6. Backend Guidelines (Rust + Axum)

- **Clean Architecture Layers:**
  - `controllers`: Thin HTTP handlers decoding requests, extracting `AuthUser`, delegating to domain services, and returning RFC 7807 Problem Details on failure.
  - `domain`: Pure business entities, state transition logic, validation rules, and repository interfaces.
  - `repository`: Data access implementations for SQLite and PostgreSQL via SQLx.
- **Error Handling:** Centralize domain errors in `roubit_core::Error`. Implement Axum's `IntoResponse` to return standard Problem Details JSON payloads. Avoid unconstrained `unwrap()` in production paths.
- **Security Headers:** Axum middleware automatically applies OWASP recommended headers (`X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `Content-Security-Policy`).

---

## 7. Quality Gates & Verification (Mandatory)

Before declaring any task, activity, or pull request complete, you **must run and pass the project quality gate**:

```bash
./scripts/pre-commit.sh
```

This automated script validates:
1. `cargo fmt --check` — Strict Rust formatting.
2. `cargo clippy --workspace --all-targets -- -D warnings` — Zero linter warnings.
3. `cargo test --workspace` — All backend unit, integration, and crypto vector tests.
4. `npm run lint --prefix frontend` — ESLint rules on Vue and TypeScript files.
5. `npm test --prefix frontend` — Vitest unit and integration suites (112+ tests across all 5 views and stores).
6. `npm run build --prefix frontend` — TypeScript type checking and production PWA bundling.

---

## 8. Git & Workflow Conventions

- **Language Standard:** English for all code, identifiers, comments, documentation, commit messages, and PR descriptions.
- **Base Branch:** All feature and bugfix branches must branch from and target `develop`.
- **Commit Messages:** Follow [Conventional Commits](https://www.conventionalcommits.org/):
  - `feat: <description>` (new features)
  - `fix: <description>` (bug fixes)
  - `docs: <description>` (documentation changes)
  - `test: <description>` (unit or integration tests)
  - `refactor: <description>` (code refactoring without functional changes)
  - `perf: <description>` (performance optimizations)
- **Granular Commits:** Separate activities and improvements into logical, self-contained commits.

