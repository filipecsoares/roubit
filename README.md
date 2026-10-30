# Roubit

> **Routine + Habit** — A high-performance, minimalist personal platform for daily routines, habit tracking, OKR alignment, and zero-knowledge encrypted journaling.

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue.js-3.5%2B-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0%2B-3178C6?logo=typescript)](https://www.typescriptlang.org/)
[![Docker](https://img.shields.io/badge/Docker-Compose%20Ready-2496ED?logo=docker)](docker-compose.yml)
[![Zero-Knowledge](https://img.shields.io/badge/Security-AES--256--GCM%20%2B%20Argon2id-blueviolet)](docs/00-especificacao-tecnica.md#5-segurança--criptografia)
[![P99 Latency](https://img.shields.io/badge/P99%20Latency-4.875ms-success)](docs/benchmarks/01-load-test-p99-memory-report.md)
[![Memory Footprint](https://img.shields.io/badge/Memory%20RSS-13.11MB-success)](docs/benchmarks/01-load-test-p99-memory-report.md)
[![Progress](https://img.shields.io/badge/Progress-48%2F48%20Tasks%20(100%25)-brightgreen)](docs/02-status-atividades.md)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Key Features](#-key-features)
- [System Architecture](#-system-architecture)
- [Prerequisites](#-prerequisites)
- [Execution & Quick Start](#-execution--quick-start)
  - [Method 1: Docker Compose (1-Command Full Stack)](#method-1-docker-compose-production-stack)
  - [Method 2: Local Fullstack Development](#method-2-local-fullstack-development)
  - [Method 3: Terminal CLI Mode (Standalone)](#method-3-terminal-cli-mode-standalone)
- [Terminal CLI Command Reference](#-terminal-cli-command-reference)
- [REST API & SSE Reference](#-rest-api--sse-reference)
- [Zero-Knowledge Security Model](#-zero-knowledge-security-model)
- [Performance Benchmarks (RNF04)](#-performance-benchmarks-rnf04)
- [Automated Testing & Quality Suite](#-automated-testing--quality-suite)
- [Configuration & Environment Variables](#-configuration--environment-variables)
- [Repository Structure](#-repository-structure)
- [License](#-license)

---

## 🚀 Overview

**Roubit** combines time-blocked daily execution, long-term strategic alignment, and private personal reflection into a single unified engine engineered in **Rust** and **Vue 3**.

Designed with **extreme performance**, **zero-knowledge privacy**, and **offline resilience** at its core:
- **Zero-Knowledge Privacy:** Personal daily reflections are encrypted directly on your device with **AES-256-GCM** using keys derived via **Argon2id**. Plaintext reflections **never** touch the server or disk unencrypted.
- **Extreme Speed:** Tail latency under **5ms P99** and resident memory usage under **14MB RSS** under heavy concurrent load (10,490+ req/s).
- **Dual Engine Flexibility:** Run locally with zero setup via an embedded **SQLite (WAL mode)** database, or scale up to cloud-native **PostgreSQL 16** with streaming migration tools.
- **Hybrid Access:** Manage your day seamlessly through either a **Vue 3 SPA** or a lightning-fast terminal **CLI** using the exact same domain engine.

---

## ✨ Key Features

### ⏱️ Time-Blocked Routine Management (RF01)
- Organize days into structured, chronological time blocks (Deep Work, Exercise, Meetings, Rest).
- Automated non-overlap validation running in $O(N \log N)$ time, preventing double-booking.
- Real-time timeline progress tracking with state transitions (`PENDING` $\to$ `IN_PROGRESS` $\to$ `COMPLETED` / `SKIPPED`).
- Daily adherence score calculations based on completed vs. scheduled duration.

### ⚡ Multi-Modal Habit Tracker (RF02)
- Supports three habit modalities:
  - **Binary Habits:** Simple completion toggles (e.g., *Take vitamins*).
  - **Numeric Habits:** Cumulative daily targets with units (e.g., *Drink 2500 ml water*).
  - **Timer Habits:** Stopwatch and duration tracking (e.g., *Meditate 20 min*).
- Resilient streak engine supporting custom target days (e.g., weekday-only streaks that don't break on weekends).
- GitHub-style 52-week contribution heatmaps and consistency metrics.

### 🎯 Goal Alignment & OKRs Engine (RF03)
- Hierarchical structure: **Objectives** broken down into quantifiable **Key Results**.
- Direct linkage between daily habits and key results with automatic live progress roll-ups.
- Quarterly tracking (e.g., `2026-Q4`) with overall completion percentages.

### 🔐 Zero-Knowledge Encrypted Journal (RF04)
- Dual-storage architecture: open metadata (mood score 1–5, tags, timestamps) for fast filtering, alongside strictly encrypted reflection ciphertext.
- End-to-end client-side encryption using **Web Crypto API** (Browser) and **AES-256-GCM** / **Argon2id** (CLI).
- Memory hygiene via the Rust `zeroize` trait to wipe cryptographic keys from RAM upon drop.

### 🛡️ Enterprise-Grade Resilience (RF05, RNF01–RNF04)
- **Redis Distributed Locks:** Atomic mutual exclusion with Lua release scripts preventing race conditions across concurrent nodes.
- **Scheduled Reminders via Redis TTL:** Keyspace notifications (`Ex`) triggering asynchronous Tokio workers that stream real-time push events to clients over **Server-Sent Events (SSE)**.
- **Dual-Database Support:** Transparent runtime abstraction between **SQLite** and **PostgreSQL 16**.
- **Transactional Idempotency:** Guaranteed once-only mutation semantics for all state-changing endpoints using the `Idempotency-Key` header with automatic 24-hour response caching.
- **OWASP Security Headers:** Automated injection of HSTS, CSP, X-Frame-Options, X-Content-Type-Options, and Referrer-Policy.

---

## 🏗️ System Architecture

```mermaid
flowchart TD
    subgraph Clients["Clients Layer"]
        Web["Vue 3 SPA<br/>(Web Crypto API / Pinia / Tailwind)"]
        CLI["Terminal CLI<br/>(Clap / Ratatui UI / Zeroize)"]
    end

    subgraph Edge["Transport & Security Layer"]
        Nginx["Nginx Reverse Proxy<br/>(Port 80)"]
        Axum["Axum REST API Engine<br/>(Port 8080)"]
        SSE["SSE Push Worker<br/>(/api/v1/notifications/stream)"]
    end

    subgraph Core["Domain Core (roubit_core)"]
        RoutineEngine["Routine Engine<br/>(Overlap & Timeline)"]
        HabitEngine["Habit Engine<br/>(Streaks & Heatmaps)"]
        OkrEngine["OKR Engine<br/>(Progress Roll-ups)"]
        CryptoService["Crypto Service<br/>(Argon2id & AES-256-GCM)"]
    end

    subgraph State["Data & State Layer"]
        Redis[("Redis 7<br/>Locks, TTL Keyspace, Idempotency Cache")]
        DB[("Dual Database<br/>SQLite WAL & PostgreSQL 16")]
    end

    Web -->|HTTP / REST| Nginx
    Nginx -->|Reverse Proxy| Axum
    Web -->|SSE Stream| Axum
    CLI -->|In-Process or HTTP| Core
    Axum --> RoutineEngine
    Axum --> HabitEngine
    Axum --> OkrEngine
    Axum --> CryptoService
    Axum --> SSE
    SSE -.->|Listen Expiration| Redis
    Axum -->|Distributed Locks & Cache| Redis
    RoutineEngine --> DB
    HabitEngine --> DB
    OkrEngine --> DB
    CryptoService --> DB
```

---

## 📋 Prerequisites

Depending on how you wish to run Roubit, ensure you have the following installed:

### For Docker Stack (Quickest & Recommended)
- **Docker Engine:** 24.0+
- **Docker Compose:** v2.20+

### For Local Native Development
- **Rust Toolchain:** 1.85.0 or newer (`cargo`, `rustc`, `clippy`, `rustfmt`)
- **Node.js:** 20.x or newer (LTS recommended) & **npm** 10.x+
- **SQLite 3:** (optional; SQLx builds with bundled SQLite drivers by default)
- **Redis & PostgreSQL:** (optional; needed only if running with Redis TTL reminders or PostgreSQL mode locally)

---

## ⚡ Execution & Quick Start

### Method 1: Docker Compose (Production Stack)

The simplest way to spin up the complete Roubit ecosystem (PostgreSQL 16, Redis 7, Axum REST API, and Vue 3 SPA behind Nginx):

```bash
# 1. Clone the repository
git clone https://github.com/filipecsoares/roubit.git
cd roubit

# 2. Launch the entire containerized stack in background
docker compose up -d

# 3. Verify container health
docker compose ps
```

#### Services and Access Points:
| Service | URL / Port | Description |
| :--- | :--- | :--- |
| **Frontend Web SPA** | [http://localhost](http://localhost) (Port 80) | Vue 3 Dashboard, Routines, Habits, OKRs, Encrypted Journal |
| **Backend REST API** | [http://localhost:8080](http://localhost:8080) | Axum API, JWT auth, SSE reminders |
| **PostgreSQL Database** | `localhost:5432` | Primary persistent relational store |
| **Redis Cache & Pub/Sub** | `localhost:6379` | Distributed locks and TTL keyspace notifications |

To stop the stack:
```bash
docker compose down
```

---

### Method 2: Local Fullstack Development

To run the backend API and frontend SPA natively on your development machine:

#### Step 1: Start the Backend API
By default, the backend runs against a zero-configuration local SQLite database (`roubit.db`):

```bash
# From the project root:
cargo run --bin roubit-server
```
The API server will automatically run migrations and start listening on `http://127.0.0.1:8080`.

*(Optional)* To run with Redis and PostgreSQL locally, pass environment variables:
```bash
DATABASE_URL=postgres://roubit_user:roubit_secure_password@localhost:5432/roubit \
REDIS_URL=redis://127.0.0.1:6379 \
cargo run --bin roubit-server
```

#### Step 2: Start the Frontend SPA
In a separate terminal window:

```bash
cd frontend

# Install dependencies
npm install

# Start the Vite development server with HMR
npm run dev
```
Open [http://localhost:5173](http://localhost:5173) in your browser. Requests made to `/api/*` are automatically proxied to `http://127.0.0.1:8080`.

---

### Method 3: Terminal CLI Mode (Standalone)

Roubit includes a high-performance command-line binary (`roubit`) that can operate directly against a local SQLite database without running any server:

```bash
# Build the optimized release binary
cargo build --release --bin roubit

# (Optional) Symlink or copy to your PATH
cp target/release/roubit /usr/local/bin/
```

Test the installation:
```bash
roubit --help
```

---

## 💻 Terminal CLI Command Reference

Roubit CLI provides intuitive, colorful, and scriptable subcommands with JSON output support (`--json`).

### Global Flags
| Flag | Short | Description |
| :--- | :---: | :--- |
| `--db <PATH>` | `-d` | Path to custom SQLite database file or database URL |
| `--remote <URL>` | `-r` | Remote API base URL (switches to HTTP client mode) |
| `--json` | - | Formats output in raw machine-readable JSON |
| `--verbose` | `-v` | Increases logging verbosity (`-v` for debug, `-vv` for trace) |

---

### 1. Routine Management (`roubit routine`)
Manage time blocks, daily schedules, and mark task progress.

```bash
# List scheduled time blocks and adherence for today
roubit routine list

# List time blocks for a specific date
roubit routine list --date 2026-10-30

# Transition a time block status
roubit routine start <BLOCK_UUID>
roubit routine complete <BLOCK_UUID>
roubit routine skip <BLOCK_UUID>
```

---

### 2. Habit Tracking (`roubit habit`)
Record check-ins, view streaks, and monitor consistency.

```bash
# List all active habits
roubit habit list

# List all habits including archived ones
roubit habit list --all

# Record a binary habit check-in
roubit habit checkin <HABIT_UUID>

# Record a numeric or timer habit check-in (e.g., 2500 ml or 30 minutes)
roubit habit checkin <HABIT_UUID> --value 2500 --date 2026-10-30

# View streaks, current consistency, and completion records
roubit habit streaks
```

---

### 3. Goal Alignment & OKRs (`roubit okr`)
Track strategic quarterly goals and progress roll-ups.

```bash
# View active objectives and their key results for current quarter
roubit okr list

# Filter objectives by specific quarter
roubit okr list --quarter 2026-Q4

# Display an executive summary of all OKRs with progress percentages
roubit okr summary
```

---

### 4. Zero-Knowledge Journal (`roubit journal`)
Write and read encrypted reflections securely from your terminal.

```bash
# Write an encrypted reflection (prompts interactively for content & passphrase)
roubit journal write --mood 5 --tags "focus,shipping"

# Write directly with content flag (still prompts securely for passphrase)
roubit journal write --mood 4 --tags "learning,rust" --content "Implemented distributed locks."

# Decrypt and read a reflection (prompts securely for master passphrase)
roubit journal read --date 2026-10-30

# List public metadata (mood, tags, timestamps) across a date range without decrypting
roubit journal list --from 2026-10-01 --to 2026-10-31
```

---

### 5. Settings & Database Administration (`roubit config` & `roubit admin`)
Inspect configurations and execute streaming database migrations.

```bash
# Inspect active configuration
roubit config show

# Set default local database path
roubit config set --db ~/.local/share/roubit/roubit.db

# Migrate data from SQLite to PostgreSQL (streaming migration)
roubit admin migrate-to-postgres \
  --from sqlite://roubit.db \
  --to postgres://roubit_user:roubit_secure_password@localhost:5432/roubit
```

---

## 🌐 REST API & SSE Reference

All REST endpoints are namespaced under `/api/v1` and implement RFC 7807 Problem Details for standard error responses.

### Authentication & User
| Method | Endpoint | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `POST` | `/api/v1/auth/register` | No | Register a new user account |
| `POST` | `/api/v1/auth/login` | No | Login and obtain JWT Bearer token |
| `GET` | `/api/v1/auth/me` | Bearer JWT | Retrieve current authenticated user profile |

### Routines & Time Blocks
| Method | Endpoint | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `GET` | `/api/v1/routines` | Bearer JWT | List routines for the authenticated user |
| `POST` | `/api/v1/routines` | Bearer JWT | Create a new routine with time blocks |
| `GET` | `/api/v1/routines/active` | Bearer JWT | Fetch the active routine definition |
| `POST` | `/api/v1/routines/:id/activate` | Bearer JWT | Set a routine as the active daily template |
| `GET` | `/api/v1/routines/timeline/:date` | Bearer JWT | Retrieve timeline and adherence score for date |
| `POST` | `/api/v1/routines/blocks/:id/log` | Bearer JWT | Record status transition (`PENDING`/`IN_PROGRESS`/`COMPLETED`/`SKIPPED`) |

### Habits & Streaks
| Method | Endpoint | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `GET` | `/api/v1/habits` | Bearer JWT | List habits (`?archived=true` to include archived) |
| `POST` | `/api/v1/habits` | Bearer JWT | Create a new habit (binary, numeric, or timer) |
| `GET` | `/api/v1/habits/:id` | Bearer JWT | Get detailed habit record |
| `POST` | `/api/v1/habits/:id/checkin` | Bearer JWT | Record a daily check-in (supports `Idempotency-Key`) |
| `GET` | `/api/v1/habits/streaks` | Bearer JWT | Retrieve active streak calculations and consistency |
| `GET` | `/api/v1/habits/:id/heatmap` | Bearer JWT | Get annual check-in frequencies for heatmap rendering |

### Objectives & OKRs
| Method | Endpoint | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `GET` | `/api/v1/okrs` | Bearer JWT | List objectives and key results (`?quarter=YYYY-QN`) |
| `POST` | `/api/v1/okrs` | Bearer JWT | Create an objective with key results & habit links |
| `GET` | `/api/v1/okrs/summary` | Bearer JWT | High-level summary with calculated progress roll-ups |
| `POST` | `/api/v1/okrs/kr/:id/progress` | Bearer JWT | Manually update progress on a key result |

### Zero-Knowledge Journal
| Method | Endpoint | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `GET` | `/api/v1/journal` | Bearer JWT | List entry metadata (`?from=YYYY-MM-DD&to=YYYY-MM-DD`) |
| `POST` | `/api/v1/journal` | Bearer JWT | Store encrypted envelope (ciphertext, IV, salt, tag) |
| `GET` | `/api/v1/journal/:date` | Bearer JWT | Retrieve encrypted envelope for a specific date |
| `DELETE` | `/api/v1/journal/:date` | Bearer JWT | Delete journal entry |

### Real-Time Push & Health
| Method | Endpoint | Auth Required | Description |
| :--- | :--- | :---: | :--- |
| `GET` | `/api/v1/notifications/stream` | Bearer JWT | Server-Sent Events (SSE) stream for real-time reminders |
| `GET` | `/health` | No | System health check (`{"status":"ok","timestamp":...}`) |

---

## 🔒 Zero-Knowledge Security Model

The Roubit encrypted journal implements a **true zero-knowledge architecture**:

1. **Client-Side Key Derivation (Argon2id):**
   - The master passphrase never leaves the browser or terminal.
   - Keys are derived locally using **Argon2id** (RFC 9106) with $m=64\,\text{MB}$, $t=3$ iterations, $p=4$ parallelism, and a unique 16-byte cryptographically secure salt.
2. **Authenticated Cipher (AES-256-GCM):**
   - Reflection plaintext is encrypted using **AES-256-GCM** (NIST SP 800-38D) with a fresh, random 12-byte initialization vector (IV) per entry.
   - Produces a 16-byte authentication tag ensuring confidentiality and tamper resistance.
3. **Dual-Storage Isolation:**
   - Non-sensitive search metadata (`mood_score`, `tags`, `entry_date`) is stored in plaintext columns to enable indexing and analytics.
   - The reflection text exists **exclusively** as an encrypted envelope (`ciphertext`, `iv`, `tag`, `salt`).
   - Even in the event of a full database leak or compromised server, user reflections cannot be deciphered.
4. **Memory Hygiene:**
   - Cryptographic buffers, salt structures, and derived keys implement the Rust `Zeroize` and `ZeroizeOnDrop` traits, securely scrubbing volatile memory upon deallocation.

---

## 📊 Performance Benchmarks (RNF04)

Roubit's backend was subjected to intensive concurrent load testing to validate **RNF04** requirements.

> **Test Environment:** Linux x86_64, Rust 1.85+ (`--release`, LTO enabled, `opt-level = 3`).  
> **Workload:** 1,000 authenticated concurrent requests across 20 virtual workers with HTTP keep-alive.

| Metric | Target (RNF04) | Official Result | Verdict |
| :--- | :---: | :---: | :---: |
| **Throughput** | $> 1,000\text{ req/s}$ | **10,491.17 req/s** | 🟢 **10x above requirement** |
| **P50 Latency (Median)** | — | **1.770 ms** | 🟢 Ultra-fast |
| **P90 Latency** | — | **2.345 ms** | 🟢 Ultra-fast |
| **P95 Latency** | $< 5.0\text{ ms}$ | **2.625 ms** | 🟢 Exceeded |
| **P99 Tail Latency** | **$< 10.0\text{ ms}$** | **4.875 ms** | 🟢 **Approved (48.7% of budget)** |
| **Max Tail Latency** | — | **5.392 ms** | 🟢 No outliers |
| **Resident Memory (RSS)** | **$< 25.0\text{ MB}$** | **13.11 MB** | 🟢 **Approved (52.4% of budget)** |
| **Success Rate (HTTP 200)** | $> 99.9\%$ | **100.0% (1,000 / 1,000)** | 🟢 Zero errors |

Full report details and K6 scenarios are available in **[docs/benchmarks/01-load-test-p99-memory-report.md](docs/benchmarks/01-load-test-p99-memory-report.md)**.

---

## 🧪 Automated Testing & Quality Suite

Roubit enforces strict quality standards: zero compiler warnings (`-D warnings`), pedantic Clippy rules, comprehensive cryptographic test vectors, and end-to-end integration tests.

### Running Backend Tests (Rust)
```bash
# Run all workspace unit and integration tests (158+ tests)
cargo test --workspace --manifest-path api/Cargo.toml

# Run NIST SP 800-38D & RFC 9106 cryptographic vectors
cargo test --test crypto_vectors --manifest-path api/Cargo.toml

# Run the in-process P99 latency & memory benchmark
cargo test --test load_p99_benchmark --manifest-path api/Cargo.toml --release -- --nocapture
```

### Running Frontend Tests (Vue 3 / Vitest)
```bash
cd frontend

# Run Vitest test suite (stores, components, Web Crypto)
npm test

# Run TypeScript typechecks and production build
npm run build
```

### Running the Pre-Commit Verification Script
```bash
./scripts/pre-commit.sh
```
This script validates:
1. `cargo fmt --check` (Rust code formatting)
2. `cargo clippy --workspace --all-targets -- -D warnings` (Pedantic linter)
3. Workspace test execution
4. Frontend Vitest suite and production bundling

---

## ⚙️ Configuration & Environment Variables

Roubit components are configured via standard environment variables or `.env` files:

| Variable | Default Value | Description |
| :--- | :--- | :--- |
| `DATABASE_URL` | `sqlite://roubit.db?mode=rwc` | SQLite connection string or PostgreSQL URL |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection URL for distributed locks and TTL reminders |
| `SERVER_HOST` | `127.0.0.1` | Host address for Axum HTTP API |
| `SERVER_PORT` | `8080` | Port for Axum HTTP API |
| `JWT_SECRET` | *(auto-generated / default)* | Secret key used to sign and verify JWT authentication tokens |
| `CORS_ALLOWED_ORIGINS` | `*` | Comma-separated list of allowed CORS origins |
| `ROUBIT_LOG_LEVEL` | `info` | Tracing logging level (`trace`, `debug`, `info`, `warn`, `error`) |
| `ROUBIT_REMOTE_URL` | *(empty)* | Optional remote API URL for Roubit CLI hybrid mode |

---

## 📁 Repository Structure

```text
roubit/
├── api/                                # Rust Multi-Crate Workspace
│   ├── Cargo.toml                      # Workspace manifest and dependencies
│   ├── Dockerfile                      # Multi-stage production container build
│   └── crates/
│       ├── core/                       # Domain models, storage engines & crypto
│       │   ├── src/crypto/             # Argon2id, AES-256-GCM, Zeroize wrappers
│       │   ├── src/domain/             # Routines, Habits, OKRs, Journal services
│       │   ├── src/redis/              # Redis connection pool & distributed locks
│       │   ├── migrations/             # SQLite migrations (WAL mode)
│       │   └── migrations_pg/          # PostgreSQL native migrations
│       ├── cli/                        # Terminal interface (Clap, Ratatui, Prompts)
│       │   └── src/commands/           # Routine, habit, okr, journal & admin commands
│       └── server/                     # REST API (Axum 0.8, Tower, SSE)
│           ├── src/auth/               # JWT authentication & password hashing
│           ├── src/controllers/        # Domain controllers & error mapping
│           ├── src/middleware/         # Idempotency, OWASP headers, CORS
│           └── src/workers/            # Reminder worker & SSE stream broadcaster
├── frontend/                           # Vue 3 Single Page Application
│   ├── src/
│   │   ├── components/                 # Dashboard, Habits, OKRs, Journal, Modals
│   │   ├── services/                   # Axios client & Web Crypto API service
│   │   ├── stores/                     # Pinia stores (auth, routines, habits, okrs, journal)
│   │   └── views/                      # Main view pages & navigation
│   ├── Dockerfile                      # Production Nginx container build
│   └── nginx.conf                      # Nginx reverse proxy configuration
├── docs/                               # Project documentation & engineering specs
│   ├── 00-especificacao-tecnica.md     # Architecture & technical blueprint
│   ├── 01-plano-de-desenvolvimento.md  # 10 features development plan (WBS)
│   ├── 02-status-atividades.md         # 48/48 task completion status tracker
│   ├── benchmarks/                     # Official load test reports & metrics
│   └── features/                       # Activity prompts (01 to 10)
├── scripts/                            # Operational scripts & pre-commit hooks
│   └── pre-commit.sh                   # Automated quality gate script
├── tests/load/                         # K6 distributed benchmark scenarios
└── docker-compose.yml                  # Production stack orchestration
```

---

## 📄 License

Roubit is dual-licensed under either of:

- **[Apache License, Version 2.0](LICENSE-APACHE)**
- **[MIT License](LICENSE-MIT)**

at your option.
