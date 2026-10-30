# Roubit

> **Routine + Habit** — A high-performance, minimalist personal platform for daily routines, habit tracking, OKR alignment, and zero-knowledge encrypted journaling.

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue.js-3.5%2B-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Progress](https://img.shields.io/badge/Progress-31%2F48%20Tasks%20(64.6%25)-brightgreen)](docs/02-status-atividades.md)

---

## 🚀 Overview

Roubit is designed around four essential pillars of personal development, built for extreme responsiveness and strict privacy:

- ⏱️ **Routine Management (Time Blocks):** Structured daily routines with $O(N \log N)$ non-overlap validation, real-time timeline evaluation, and adherence tracking.
- ⚡ **Habit Tracker:** Multi-modal habits (binary, numeric targets, timers) with resilient streak calculations and annual contribution heatmaps.
- 🎯 **Goals & OKR Alignment Engine:** Connect long-term quarterly objectives directly to daily habits with automatic progress roll-ups.
- 🔐 **Zero-Knowledge Journaling:** Client-side field-level encryption using **AES-256-GCM** and **Argon2id**. Plaintext reflections never touch the server or disk unencrypted.
- 🏎️ **Extreme Performance & Safety:** Backend engine in Rust (Axum, Tokio, SQLx) targeting sub-10ms P99 latency and zero memory leaks with `zeroize`.

---

## 🚦 Project Status

| Module | Scope | Status |
| :--- | :--- | :---: |
| **01. Workspace & Tooling** | Multi-crate layout, strict lints, CI pipeline, tracing | 🟢 Concluído (4/4) |
| **02. Cryptography & Memory** | Argon2id KDF, AES-256-GCM, Zeroize, NIST KAT vectors | 🟢 Concluído (4/4) |
| **03. Routine Management** | SQLite WAL repo, schedule validator, real-time tracker | 🟢 Concluído (5/5) |
| **04. Habit Tracking** | Check-ins, streak engine, adherence analytics | 🟢 Concluído (5/5) |
| **05. OKRs & Alignment** | Objectives, key results, habit progress linkage | 🟢 Concluído (4/4) |
| **06. Journaling (Zero-Knowledge)**| Dual-storage schema, client crypto service, tamper tests | 🟢 Concluído (4/4) |
| **07. Terminal CLI** | Clap commands, TUI tables, interactive prompts | 🟢 Concluído (5/5) |
| **08. REST API & Idempotência** | Axum HTTP server, JWT auth, transactional idempotency | ⚪ Em breve |
| **09. Frontend SPA** | Vue 3, Pinia, TypeScript, TailwindCSS, Web Crypto | ⚪ Em breve |
| **10. Infrastructure** | Redis TTL notifications, PostgreSQL Docker Compose | ⚪ Em breve |

Detailed tracking is maintained in the **[Activity Status Tracker](docs/02-status-atividades.md)**.

---

## ⚡ Quick Start & Verification

```bash
# Clone repository
git clone https://github.com/filipecsoares/roubit.git
cd roubit

# Run all 158 automated unit & integration tests
cargo test --workspace --manifest-path api/Cargo.toml

# Run quality checks (formatting, pedantic clippy)
./scripts/pre-commit.sh
```

---

## 📁 Repository Structure

```text
roubit/
├── api/              # Rust workspace (Domain Core, CLI, and REST Server)
│   ├── crates/core   # Domain models, crypto, storage & tracking engines
│   ├── crates/cli    # Terminal interface (Clap)
│   └── crates/server # REST HTTP API (Axum)
├── frontend/         # Vue.js 3 SPA (Composition API, Vite, Pinia, TailwindCSS)
└── docs/             # Specifications, architecture, and feature engineering guides
```

---

## 📚 Documentation

- **[Technical Specification & Architecture](docs/00-especificacao-tecnica.md)**
- **[Development Plan & WBS](docs/01-plano-de-desenvolvimento.md)**
- **[Activity Status Tracker (48 Tasks)](docs/02-status-atividades.md)**
- **[Feature Activities Specifications](docs/features/README.md)**

---

## 📄 License

Dual-licensed under either of:
- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
