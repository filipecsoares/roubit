# Roubit

> **Routine + Habit** — A high-performance personal operating system that integrates daily time-blocking, habit tracking, OKR goal alignment, and zero-knowledge encrypted journaling.

[![Progress](https://img.shields.io/badge/Progress-48%2F48%20Tasks%20(100%25)-brightgreen)](docs/02-status-atividades.md)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue.js-3.5%2B-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0%2B-3178C6?logo=typescript)](https://www.typescriptlang.org/)
[![Security](https://img.shields.io/badge/Security-Zero--Knowledge%20(AES--256--GCM)-blueviolet)](docs/00-especificacao-tecnica.md#5-segurança--criptografia)
[![Docker Ready](https://img.shields.io/badge/Docker-Compose%20Ready-2496ED?logo=docker)](docker-compose.yml)
[![P99 Latency](https://img.shields.io/badge/P99%20Latency-4.875ms-success)](docs/benchmarks/01-load-test-p99-memory-report.md)
[![Memory RSS](https://img.shields.io/badge/Memory%20RSS-13.11MB-success)](docs/benchmarks/01-load-test-p99-memory-report.md)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

## 🎯 Why Roubit?

Most productivity setups are fragmented: a to-do list in one app, habit tracking in another, quarterly goals buried in spreadsheets, and private thoughts scattered across text files. This fragmentation creates friction, context-switching, and a disconnect between what you do each day and what you want to achieve long-term.

**Roubit** brings these four essential pillars together into a cohesive, distraction-free environment:

1. **Daily Focus:** Structure your day into clear time blocks, avoiding overbooking and decision fatigue.
2. **Habits with Purpose:** Your daily habits directly feed into and advance your long-term quarterly goals.
3. **Uncompromising Privacy:** Your journal entries are encrypted on your device using client-side zero-knowledge encryption before reaching storage.
4. **Blazing Speed & Safety:** Powered by a Rust backend with sub-5ms tail latency, minimal memory usage, and zero cloud lock-in.

---

## ✨ Core Features

### ⏱️ 1. Time-Blocked Daily Routines (RF01)
- **Chronological Day Planning:** Build daily schedules with distinct focus blocks (*Deep Work*, *Workout*, *Study*, *Rest*).
- **Conflict Prevention Engine:** Mathematical non-overlap validation running in $O(N \log N)$ prevents scheduling collisions.
- **Real-Time Day Tracker:** Follow the active block in real time and log state transitions (`Pending` $\to$ `In Progress` $\to$ `Completed` / `Skipped`).
- **Adherence Score:** Automatically calculates your scheduled vs. completed duration percentage at the end of each day.

### ⚡ 2. Smart Habit Tracker & Resilient Streaks (RF02)
- **Three Flexible Habit Types:**
  - **Binary Habits:** Simple completion toggles (e.g., *Take vitamins*, *Make bed*).
  - **Numeric Habits:** Incremental daily targets (e.g., *Drink 2500 ml water*, *Read 25 pages*).
  - **Timer Habits:** Integrated stopwatch for duration-based habits (e.g., *Meditate 20 min*, *Practice guitar*).
- **Smart Streaks:** Rest days and custom schedules (e.g., weekday-only habits) do not break your streaks on scheduled off-days.
- **52-Week Contribution Heatmap:** GitHub-style visual consistency grid tracking your momentum across the entire year.

### 🎯 3. Goal Alignment & OKRs Engine (RF03)
- **Quarterly Objectives & Key Results:** Structure large strategic ambitions into quantifiable, measurable targets.
- **Habit-to-KR Linking:** Connect daily habits directly to quarterly key results.
- **Automated Progress Roll-Up:** Whenever you record a habit check-in, your linked OKR progress bars advance automatically.

### 🔒 4. Zero-Knowledge Encrypted Journal (RF04)
- **Client-Side Encryption:** Personal reflections are encrypted in your browser or terminal using **AES-256-GCM** with keys derived via **Argon2id**.
- **Zero Plaintext on Disk/Server:** The server only stores ciphertext envelopes. Even if the database is exposed, your private thoughts remain completely unreadable.
- **Indexed Search Metadata:** Mood ratings (1–5) and categorical tags remain open for trend analysis without compromising reflection text.
- **Volatile Memory Scrubbing:** Derived keys and sensitive buffers are zeroed out in memory upon use using Rust's `zeroize` trait.

### 🖥️ 5. Dual Interface: Web SPA & Terminal CLI
- **Modern Web App:** Responsive Vue 3 Single Page Application with clean aesthetics, dark mode, and interactive modals.
- **Fast Terminal CLI:** Full-featured command-line interface (`roubit`) for keyboard-centric and offline workflows.

---

## 🏗️ Architecture & Technology Stack

```mermaid
flowchart TD
    subgraph Clients["Clients (End-to-End Encryption)"]
        Web["Vue 3 SPA<br/>(Web Crypto API / Pinia / Tailwind)"]
        CLI["Terminal CLI<br/>(Clap / Ratatui UI / Zeroize)"]
    end

    subgraph Server["Backend Engine (Rust / Axum)"]
        API["REST API & SSE Stream<br/>(Axum 0.8 / Tokio / Tracing)"]
        Idempotency["Idempotency Interceptor<br/>(24h Key Guard)"]
        Domain["Domain Services<br/>(Routines, Habits, OKRs, Journal)"]
    end

    subgraph Storage["Data & Infrastructure Layer"]
        Redis[("Redis 7<br/>Locks, TTL Keyspace, Cache")]
        DB[("Dual Database Engine<br/>SQLite WAL & PostgreSQL 16")]
    end

    Web -->|HTTP / REST + SSE| API
    CLI -->|In-Process or HTTP| Domain
    API --> Idempotency --> Domain
    Domain --> DB
    Domain --> Redis
    Redis -.->|Expired TTL Events| API
```

| Layer | Technologies | Key Highlights |
| :--- | :--- | :--- |
| **Backend** | Rust 1.85+, Axum 0.8, Tokio, SQLx | Non-blocking asynchronous I/O, compile-time SQL checks, OWASP headers. |
| **Frontend** | Vue 3.5, TypeScript 5, Vite, Pinia, TailwindCSS | Composition API, responsive layout, Web Crypto API for client encryption. |
| **Storage** | SQLite (WAL mode) / PostgreSQL 16 | Zero-setup local SQLite for single-user; native PostgreSQL for distributed stack. |
| **Cache & Queue** | Redis 7 | Distributed mutual exclusion locks, keyspace expiration notifications for SSE reminders. |
| **Security** | Argon2id & AES-256-GCM, Zeroize | Memory-hard KDF, authenticated ciphertext envelopes, memory wiping on drop. |

---

## 🚀 Quick Start & Execution

Roubit can be launched in one command via **Docker Compose** or run natively for **local development**.

### Prerequisites
- **For Docker Deployment:** Docker Engine 24.0+ and Docker Compose v2.20+.
- **For Native Development:** Rust 1.85.0+ and Node.js 20.x+ (with npm 10.x+).

---

### Option A: 1-Command Stack via Docker Compose (Recommended)

Spins up PostgreSQL 16, Redis 7, Axum REST API, and the Vue 3 SPA behind an Nginx reverse proxy:

```bash
# 1. Clone the repository
git clone https://github.com/filipecsoares/roubit.git
cd roubit

# 2. Start all services in the background
docker compose up -d

# 3. Check running status
docker compose ps
```

#### Access Points:
- 🌐 **Web Application:** [http://localhost](http://localhost) (Port 80)
- 🔌 **REST API Service:** [http://localhost:8080](http://localhost:8080)
- 📊 **Health Check:** `curl http://localhost:8080/health`

To stop all services:
```bash
docker compose down
```

---

### Option B: Local Native Development

#### 1. Start the Backend API (Rust)
By default, the server runs against an embedded local SQLite database (`roubit.db`) with zero setup needed:
```bash
cargo run --bin roubit-server
```
*The server starts listening on `http://127.0.0.1:8080`.*

#### 2. Start the Frontend App (Vue 3)
In a separate terminal:
```bash
cd frontend
npm install
npm run dev
```
*Open [http://localhost:5173](http://localhost:5173) in your browser. API requests are automatically proxied to the backend.*

---

### Option C: Terminal CLI Standalone Mode

The command-line interface (`roubit`) can run directly against your local SQLite database without needing the web server:

```bash
# Compile optimized release binary
cargo build --release --bin roubit

# (Optional) Install to system PATH
sudo cp target/release/roubit /usr/local/bin/
```

#### Daily CLI Commands:
```bash
# 📅 Routines: Create, activate, list, and transition blocks
roubit routine create --name "Workday" --blocks "08:00-12:00:Deep Work,12:00-13:00:Lunch,13:00-17:00:Projects" --active
roubit routine list
roubit routine start <BLOCK_ID>
roubit routine complete <BLOCK_ID>

# ⚡ Habits: Create, check in, view streaks, archive, or delete
roubit habit create --name "Meditate" --kind timer --target 20 --unit min
roubit habit create --name "Hydration" --kind numeric --target 2500 --unit ml
roubit habit checkin <HABIT_ID> --value 1.0
roubit habit streaks
roubit habit archive <HABIT_ID>

# 🎯 OKRs: Create objectives, add Key Results, inspect progress
roubit okr create --title "Improve Physical Health" --quarter "2026-Q4"
roubit okr add-kr --objective-id <OBJ_ID> --title "Run 100km" --target 100 --unit km
roubit okr list
roubit okr summary

# 🔒 Journal: Write encrypted reflections, read, or delete
roubit journal write --mood 5 --tags "focus,shipping"
roubit journal read
roubit journal delete --date 2026-10-30

# ⚙️ Migrate SQLite data to PostgreSQL
roubit admin migrate-to-postgres \
  --from sqlite://roubit.db \
  --to postgres://user:password@localhost:5432/roubit
```

---

## 🔒 Zero-Knowledge Security Model

Roubit ensures confidentiality using a **zero-knowledge cryptographic pipeline**:

1. **Client-Side Key Derivation:** Passphrases are never transmitted over the network. Keys are derived on-device using **Argon2id** (RFC 9106) with $m=64\,\text{MB}$, $t=3$ iterations, and a unique 16-byte random salt.
2. **Authenticated Encryption:** Plaintext notes are encrypted using **AES-256-GCM** (NIST SP 800-38D) with a fresh 12-byte random IV and a 16-byte authentication tag per entry.
3. **Dual Storage Isolation:** Searchable metadata (mood score 1–5, tags, dates) is indexed for fast timeline navigation, while reflection contents remain strictly ciphertext.
4. **Memory Hygiene:** Ephemeral key buffers implement Rust's `Zeroize` trait, wiping sensitive bytes from RAM immediately after encryption or decryption.

---

## 📊 Performance Benchmarks (RNF04)

Under concurrent load testing on Linux x86_64 (`release` profile, LTO enabled):

| Metric | Target (RNF04) | Official Result | Verdict |
| :--- | :---: | :---: | :---: |
| **Throughput** | $> 1,000\text{ req/s}$ | **10,491.17 req/s** | 🟢 **10x above requirement** |
| **P50 Latency (Median)** | — | **1.770 ms** | 🟢 Sub-2ms response |
| **P95 Latency** | $< 5.0\text{ ms}$ | **2.625 ms** | 🟢 Exceeded |
| **P99 Tail Latency** | **$< 10.0\text{ ms}$** | **4.875 ms** | 🟢 **Approved (48.7% of budget)** |
| **Resident Memory (RSS)** | **$< 25.0\text{ MB}$** | **13.11 MB** | 🟢 **Approved (52.4% of budget)** |
| **Success Rate (HTTP 200)** | $> 99.9\%$ | **100.0% (1,000 / 1,000)** | 🟢 0 errors |

*Detailed benchmark methodology and K6 scripts are available in [docs/benchmarks/01-load-test-p99-memory-report.md](docs/benchmarks/01-load-test-p99-memory-report.md).*

---

## 🧪 Testing & Quality Suite

Roubit enforces zero warnings (`-D warnings`), Clippy linter hygiene, and full test coverage:

```bash
# Run all Rust workspace unit and integration tests (158 tests)
cargo test --workspace --manifest-path api/Cargo.toml

# Run NIST SP 800-38D and RFC 9106 crypto vectors
cargo test --test crypto_vectors --manifest-path api/Cargo.toml

# Run in-process P99 load benchmark
cargo test --test load_p99_benchmark --manifest-path api/Cargo.toml --release -- --nocapture

# Run frontend Vitest suite (45 tests) & TypeScript build
npm test --prefix frontend
npm run build --prefix frontend

# Run complete pre-commit quality gate (fmt, clippy, tests, build)
./scripts/pre-commit.sh
```

---

## 📁 Repository Structure

```text
roubit/
├── api/                     # Rust Multi-Crate Workspace
│   ├── crates/core/         # Domain engines, SQLite/PostgreSQL repositories & crypto
│   ├── crates/cli/          # Terminal interface (Clap, Ratatui UI, Prompts)
│   └── crates/server/       # REST API (Axum 0.8), JWT auth, Idempotency & SSE workers
├── frontend/                # Vue 3 SPA (Composition API, Pinia, TailwindCSS, Web Crypto)
├── docs/                    # Technical specifications, development plans & benchmarks
│   ├── 00-especificacao-tecnica.md
│   ├── 01-plano-de-desenvolvimento.md
│   ├── 02-status-atividades.md
│   └── benchmarks/01-load-test-p99-memory-report.md
├── scripts/                 # Quality assurance scripts (pre-commit.sh)
├── tests/load/              # K6 distributed load testing scenarios
└── docker-compose.yml       # Production stack orchestration (PostgreSQL + Redis + API + SPA)
```

---

## 📚 Technical Documentation

For in-depth architectural blueprints and activity tracking:
- 📐 **[Technical Specification & Architecture](docs/00-especificacao-tecnica.md)**
- 📋 **[Development Plan & WBS (10 Features)](docs/01-plano-de-desenvolvimento.md)**
- 🚦 **[Activity Status Tracker (48/48 Tasks Completed)](docs/02-status-atividades.md)**
- 📈 **[Official Benchmark & Load Test Report](docs/benchmarks/01-load-test-p99-memory-report.md)**

---

## 📄 License

Dual-licensed under either of:
- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
