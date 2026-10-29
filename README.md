# Roubit

> **Routine + Habit** — A high-performance, minimalist personal platform for daily routines, habit tracking, OKR alignment, and zero-knowledge encrypted journaling.

[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue.js-3.5%2B-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

---

## 🚀 Overview

Roubit is designed around four essential pillars of personal development, built for extreme responsiveness and strict privacy:

- ⏱️ **Routine Management (Time Blocks):** Structured daily routines (Morning Kickoff, Deep Work, Evening Review) with real-time progression tracking.
- ⚡ **Habit Tracker:** Multi-modal habits (binary, numeric targets, timers) with resilient streak calculations and contribution heatmaps.
- 🎯 **Goals & OKR Alignment Engine:** Connect long-term quarterly objectives directly to daily habits with automatic progress roll-ups.
- 🔐 **Zero-Knowledge Journaling:** Client-side field-level encryption using **AES-256-GCM** and **Argon2id**. Plaintext reflections never touch the server or disk unencrypted.
- 🏎️ **Extreme Performance & Safety:** Backend engine in Rust (Axum, Tokio, SQLx) targeting sub-10ms P99 latency and <25MB RAM footprint.

---

## 📁 Repository Structure

```
roubit/
├── api/            # Rust workspace (Domain Core, CLI, and REST Server)
├── frontend/       # Vue.js 3 SPA (Composition API, Vite, Pinia, TailwindCSS)
└── docs/           # Specifications, architecture, and feature engineering guides
```

---

## 📚 Documentation & Engineering Activities

- **[Technical Specification & Architecture](file:///home/filipe/Documents/projects/roubit/docs/00-especificacao-tecnica.md)**
- **[Development Plan & WBS](file:///home/filipe/Documents/projects/roubit/docs/01-plano-de-desenvolvimento.md)**
- **[Feature Activities Index (48 Tasks)](file:///home/filipe/Documents/projects/roubit/docs/features/README.md)**

---

## 🛠️ Tech Stack

| Layer | Technologies |
| :--- | :--- |
| **Backend & CLI** | Rust (Tokio, Axum, SQLx, Clap) |
| **Frontend SPA** | Vue.js 3, TypeScript, Vite, Pinia, TailwindCSS |
| **Database** | SQLite (WAL mode for zero-config local) & PostgreSQL (Docker Compose) |
| **Cache & Queues**| Redis (Idempotency locks, TTL keyspace notifications) |
| **Cryptography** | AES-256-GCM, Argon2id, Zeroize, Web Crypto API |

---

## 📄 License

Dual-licensed under either of:
- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
