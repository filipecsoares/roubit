# Roubit

> **Routine + Habit** — A high-performance personal operating system and engineering study project exploring daily time-blocking, habit tracking, OKR goal alignment, client-side zero-knowledge encrypted journaling, and offline-first Progressive Web App (PWA) synchronization.

[![Progress](https://img.shields.io/badge/Progress-70%2F70%20Tasks%20(100%25)-brightgreen)](docs/02-status-atividades.md)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Vue 3](https://img.shields.io/badge/Vue.js-3.5%2B-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.0%2B-3178C6?logo=typescript)](https://www.typescriptlang.org/)
[![PWA Ready](https://img.shields.io/badge/PWA-Offline%20First-5A0FC8?logo=pwa)](frontend/README.md)
[![Security](https://img.shields.io/badge/Security-Zero--Knowledge%20(AES--256--GCM)-blueviolet)](docs/00-especificacao-tecnica.md#5-segurança--criptografia)
[![Docker Ready](https://img.shields.io/badge/Docker-Compose%20Ready-2496ED?logo=docker)](docker-compose.yml)
[![P99 Latency](https://img.shields.io/badge/P99%20Latency-4.875ms-success)](docs/benchmarks/01-load-test-p99-memory-report.md)
[![Memory RSS](https://img.shields.io/badge/Memory%20RSS-13.11MB-success)](docs/benchmarks/01-load-test-p99-memory-report.md)
[![Tests](https://img.shields.io/badge/Tests-200%2B%20Rust%20%7C%20112%20Vitest-success)](docs/02-status-atividades.md)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

## 📸 Screenshots & Main Flows

<div align="center">

| ⏱️ **Daily Focus & Routine Timeline** | ⚡ **Smart Habits, Streaks & 52-Week Heatmap** |
| :---: | :---: |
| ![Daily Focus Dashboard](docs/images/01_dashboard.png) | ![Habits & Consistency Heatmap](docs/images/02_habitos.png) |
| *Time-blocking cronológico com cálculo de progresso em tempo real* | *Streaks atômicos em O(1), contadores diários e mapa de calor anual* |

| 🎯 **OKRs & Goal Alignment Engine** | ⏱️ **Pomodoro Focus Mode** |
| :---: | :---: |
| ![OKRs & Strategic Alignment](docs/images/04_okrs.png) | ![Pomodoro Focus Mode](docs/images/05_modo_foco.png) |
| *Metas trimestrais conectadas diretamente a hábitos com progresso automático* | *Imersão sem distrações, feedback harmônico Web Audio e atalhos de teclado* |

| 🔒 **Zero-Knowledge Encrypted Journal** | 📱 **PWA Authentication & Offline Engine** |
| :---: | :---: |
| ![Encrypted Journal Vault](docs/images/03_diario_vault.png) | ![PWA Login & Install](docs/images/00_login.png) |
| *Cifragem no cliente com AES-256-GCM + Argon2id e limpeza da RAM* | *PWA instalável, sincronização offline idempotente e auth segura* |

</div>

---

## 🎓 Sobre Este Projeto de Estudo

O **Roubit** foi concebido e implementado como um **laboratório de engenharia de software de ponta a ponta**. O objetivo é demonstrar, em código de produção, soluções para desafios reais de arquitetura de sistemas contemporâneos:

1. **Backend de Baixíssima Latência em Rust:** Uso de Axum 0.8, Tokio e SQLx com tempo de resposta de cauda (P99) inferior a 5ms, throughput superior a 10.000 req/s e consumo de memória inferior a 15MB RSS.
2. **Criptografia Zero-Knowledge no Cliente:** Garantia matemática de privacidade onde anotações reflexivas são cifradas diretamente no navegador/CLI (AES-256-GCM + Argon2id). O servidor armazena apenas envelopes cifrados; nem mesmo administradores ou vazamentos de banco podem ler as reflexões.
3. **Estratégia de Banco de Dados Dual:** Desenvolvimento local com **SQLite WAL** (zero configuração externa) e homologação/produção distribuída com **PostgreSQL 16** e **Redis 7** (cache L2, locks distribuídos e mensageria SSE via TTL keyspace).
4. **PWA Offline-First & Sincronização Idempotente:** Fila monotônica de mutações em IndexedDB gerenciada pelo Service Worker (Workbox). Permite registrar hábitos, completar blocos e planejar tarefas sem conexão à internet, com drenagem automática e idempotente (UUIDv4) ao restabelecer o sinal.
5. **Arquitetura Limpa & Paridade de Interfaces:** Separação estrita entre Domínio (`roubit-core`), Servidor REST (`roubit-server`) e Terminal CLI (`roubit`). A CLI opera em modo local (chamadas diretas ao banco) ou em modo remoto autenticado contra a API HTTP.
6. **Cultura de Qualidade & CI/CD Estrito:** Suíte com mais de 200 testes no backend (unitários, integração, vetores criptográficos NIST/RFC e estresse de concorrência), 112 testes no frontend (Vitest e ESLint), e pipeline de pre-commit automatizado.

---

## 🎯 Por que o Roubit?

Muitos setups de produtividade são fragmentados: listas de tarefas em um app, hábitos em outro, metas trimestrais em planilhas e pensamentos pessoais em notas soltas. Isso gera atrito cognitivo e perda de contexto.

O **Roubit** une esses pilares em um ecossistema integrado:

- **Foco Diário Estruturado:** Seu dia dividido em blocos de tempo cronológicos e executado com timer Pomodoro integrado.
- **Hábitos com Propósito Estratégico:** Hábitos diários vinculados a Resultados-Chave (KRs) dos seus OKRs trimestrais — cada check-in avança suas metas macro.
- **Privacidade Inegociável:** Suas reflexões mais íntimas nunca chegam em texto aberto ao servidor.
- **Resiliência Offline:** Funcione normalmente no avião ou sem sinal; tudo sincroniza sem duplicação ao reconectar.
- **Velocidade Extrema:** Sem telas de espera, construído para responder instantaneamente.

---

## ✨ Core Features

### ⏱️ 1. Time-Blocked Daily Routines & Focus Mode (RF01 & Feature 11)
- **Chronological Day Planning:** Build daily schedules with distinct focus blocks (*Deep Work*, *Workout*, *Study*, *Rest*).
- **Conflict Prevention Engine:** Mathematical non-overlap validation running in $O(N \log N)$ prevents scheduling collisions.
- **Real-Time Day Tracker:** Follow the active block in real time and log state transitions (`Pending` $\to$ `In Progress` $\to$ `Completed` / `Skipped`).
- **Integrated Pomodoro Focus Mode:** Fullscreen distraction-free focus sessions directly from routine blocks, featuring circular countdown animations, cycle counters, keyboard shortcuts (`Space`, `R`, `Esc`), and native Web Audio harmonic sound synthesis.
- **Adherence Score:** Automatically calculates your scheduled vs. completed duration percentage at the end of each day.

### ⚡ 2. Smart Habit Tracker & High-Performance Streaks (RF02 & Feature 13)
- **Three Flexible Habit Types:**
  - **Binary Habits:** Simple completion toggles (e.g., *Take vitamins*, *Make bed*).
  - **Numeric Habits:** Incremental daily targets (e.g., *Drink 2500 ml water*, *Read 25 pages*).
  - **Timer Habits:** Integrated stopwatch for duration-based habits (e.g., *Meditate 20 min*, *Practice guitar*).
- **Atomic $O(1)$ Streak Engine:** Denormalized streak counters updated atomically upon check-in with scheduled off-day preservation.
- **Multi-Tiered Caching:** Tiered L1 (in-memory RAM) and L2 (Redis) summary caching with active invalidation for sub-5ms response times.
- **52-Week Contribution Heatmap:** GitHub-style visual consistency grid tracking your momentum across the entire year.

### 🎯 3. Goal Alignment & OKRs Engine (RF03)
- **Quarterly Objectives & Key Results:** Structure large strategic ambitions into quantifiable, measurable targets.
- **Habit-to-KR Linking:** Connect daily habits directly to quarterly key results.
- **Automated Progress Roll-Up:** Whenever you record a habit check-in, your linked OKR progress bars advance automatically.

### 🔒 4. Zero-Knowledge Encrypted Journal (RF04)
- **Client-Side Encryption:** Personal reflections are encrypted in your browser or terminal using **AES-256-GCM** with keys derived via **Argon2id**.
- **Zero Plaintext on Disk/Server:** The server only stores ciphertext envelopes. Even if the database is exposed, your private thoughts remain completely unreadable.
- **Indexed Search Metadata:** Mood ratings (1–5) and categorical tags remain open for trend analysis without compromising reflection text.
- **Volatile Memory Scrubbing:** Ephemeral key buffers implement Rust's `zeroize` trait, wiping sensitive bytes from RAM immediately upon locking or drop.

### 📱 5. Progressive Web App (PWA) & Offline-First Engine (Feature 16)
- **Installable Desktop & Mobile App:** Fully compliant Web App Manifest (`/manifest.webmanifest`) and Service Worker precaching powered by Workbox.
- **Offline Mutation Queue:** Queues actions (habit check-ins, routine block completions, OKRs) in **IndexedDB** with monotonic FIFO ordering.
- **Idempotent Background Synchronization:** Automatically replays queued mutations with unique UUID idempotency keys upon network reconnection, preventing duplicate records.
- **Native App Badges:** Dynamically updates the application icon badge counter with pending uncompleted habits for the current day.
- **Reactive Network Status Indicator:** Live visual alerts and synchronization counters during offline operations.

### 🔔 6. Real-Time Push Notifications (Feature 14)
- **Server-Sent Events (SSE):** Streaming notification pipeline connected to Redis keyspace expiration triggers for scheduled routine and habit reminders.
- **Web Notifications API:** Native desktop notifications with audio chime synthesis.
- **In-App Toast Feed:** Interactive notification tray displaying real-time events and reminder history.

### 🖥️ 7. Dual Interface: Web SPA & Hybrid Terminal CLI (RF05 & Feature 15)
- **Modern Web App:** Responsive Vue 3 Single Page Application with clean aesthetics, glassmorphism design, and dark mode.
- **Fast Terminal CLI (`roubit`):** Full-featured command-line tool with interactive prompts, Unicode tables (`comfy-table`), and progress bars (`indicatif`).
- **Hybrid Transport:** Seamlessly switch between local offline SQLite mode and remote authenticated HTTP API access with secure token storage in `~/.roubit/credentials.json`.

### 🛡️ 8. Enterprise Multi-Tenancy & Data Isolation (Feature 12)
- **Row-Level Tenant Isolation:** Complete user-scoping across all domain repositories, preventing Insecure Direct Object References (IDOR).
- **Dual Database Engine:** Zero-setup local SQLite in WAL mode or production-ready PostgreSQL 16 with integrated migration tooling.

---

## 🏗️ Architecture & Technology Stack

```mermaid
flowchart TD
    subgraph Clients["Clients (End-to-End Encryption & PWA)"]
        Web["Vue 3 SPA & PWA<br/>(Service Worker / IndexedDB / Web Crypto / Pinia)"]
        CLI["Terminal CLI<br/>(Local SQLite WAL / Remote JWT Transport / comfy-table)"]
    end

    subgraph Server["Backend Engine (Rust / Axum 0.8)"]
        API["REST API & SSE Stream<br/>(Axum / Tokio / Tracing / Problem Details RFC 7807)"]
        Idempotency["Idempotency Guard<br/>(24h Key Interceptor)"]
        Domain["Domain Services & Clean Architecture<br/>(Routines, Habits, OKRs, Journal, Users)"]
    end

    subgraph Storage["Data & Infrastructure Layer"]
        Redis[("Redis 7<br/>L2 Summary Cache, Distributed Locks, TTL Keyspace")]
        DB[("Dual Database Engine<br/>SQLite WAL & PostgreSQL 16")]
    end

    Web -->|HTTP / REST + SSE| API
    Web -.->|Offline Mutations| Web
    CLI -->|Local File or Remote HTTP| Domain
    API --> Idempotency --> Domain
    Domain --> DB
    Domain --> Redis
    Redis -.->|Expired TTL Events| API
```

| Layer | Technologies | Key Highlights |
| :--- | :--- | :--- |
| **Backend** | Rust 1.85+, Axum 0.8, Tokio, SQLx | Clean Architecture (Thin Controllers, Repository Pattern), sub-5ms P99 latency, compile-time SQL verification, OWASP security headers. |
| **Frontend & PWA**| Vue 3.5, TypeScript 5, Vite, Pinia, TailwindCSS | Composition API, Workbox App Shell, IndexedDB offline mutation queue, Web Crypto API (Argon2id WASM + AES-256-GCM). |
| **Storage** | SQLite (WAL mode) / PostgreSQL 16 | Zero-setup local SQLite for single-user/CLI; native PostgreSQL for distributed stack with automated migration tool. |
| **Cache & Queue** | Redis 7 | Tiered L2 caching, distributed mutual exclusion locks, keyspace expiration notifications for SSE reminders. |
| **Security** | Argon2id & AES-256-GCM, Zeroize | Zero-knowledge client-side encryption, authenticated envelopes, memory sanitization on drop. |

---

## 🚀 Quick Start & Como Rodar o Projeto Sem Erros

O **Roubit** foi desenhado para iniciar de forma simples e previsível. Você pode rodar o ecossistema completo em **1 comando via Docker Compose** ou rodar **nativamente** em modo de desenvolvimento leve.

### 📋 Pré-requisitos
- **Para execução com Docker (Recomendado):** Docker 24.0+ e Docker Compose v2.20+.
- **Para execução nativa:** 
  - [Rust 1.85.0+](https://rustup.rs/) (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
  - [Node.js 20+](https://nodejs.org/) e npm 10+ (`node -v` && `npm -v`)

---

### Opção 1: Execução Completa via Docker Compose (PostgreSQL 16 + Redis 7 + Nginx)

Esta é a opção mais rápida para testar a aplicação completa sem precisar configurar banco ou ambiente de compilação:

```bash
# 1. Clone o repositório
git clone https://github.com/filipecsoares/roubit.git
cd roubit

# 2. Suba todos os serviços em segundo plano (Postgres 16, Redis 7, Backend Axum e Frontend PWA)
docker compose up -d --build

# 3. Verifique se os 4 containers estão saudáveis
docker compose ps
```

#### 🌐 Pontos de Acesso:
- **Aplicação Web & PWA:** [http://localhost](http://localhost) (Porta 80 — servida via Nginx com compressão gzip e cache PWA)
- **API REST Backend:** [http://localhost:8080](http://localhost:8080)
- **Liveness Probe:** `curl http://localhost:8080/health` (retorna `{"status":"ok"}`)
- **Readiness Probe:** `curl http://localhost:8080/ready` (valida conexão ativa com Postgres e Redis)

Para parar a aplicação e manter os dados dos volumes:
```bash
docker compose down
```

---

### Opção 2: Desenvolvimento Nativo (SQLite WAL — Zero Dependências Externas)

No modo de desenvolvimento nativo, o backend utiliza **SQLite em WAL mode** com migrações automáticas. Você não precisa instalar PostgreSQL nem Redis:

#### Passo 1: Iniciar o Servidor Backend (Terminal 1)
```bash
# O servidor inicializa automaticamente o arquivo SQLite 'roubit.db' com todas as tabelas
cargo run --bin roubit-server --manifest-path api/Cargo.toml
```
*O servidor estará escutando em `http://127.0.0.1:8080` com migrações aplicadas automaticamente.*

#### Passo 2: Iniciar a Aplicação Frontend (Terminal 2)
```bash
cd frontend
npm install
npm run dev
```
*Abra [http://localhost:5173](http://localhost:5173) no seu navegador. As chamadas à API e streams SSE são roteados automaticamente para o backend.*

---

### 🔑 Primeiro Acesso & Credenciais de Demonstração

1. Acesse `http://localhost:5173` (ou `http://localhost` no Docker).
2. Na tela de login, clique em **"Não tem conta? Cadastre-se"**.
3. Crie qualquer conta informando um e-mail e senha (ex.: `dev@roubit.app` / `SenhaForte123!`). O hash Argon2id é derivado de forma isolada na threadpool.
4. **Para o Diário Criptografado:** Clique na aba **"Diário"** e em **"Desbloquear Diário"**. Digite uma senha mestre (ex.: `senha-cofre-2026`). A chave AES-256-GCM é derivada em memória via Argon2id WASM no seu browser e nunca é enviada ao servidor.

---

### 💡 Roteiro Rápido para Testar as Principais Funcionalidades

- **Cronograma de Rotina & Time-blocking:** No Dashboard, clique em **"Configurar Rotina"** para criar blocos diários de foco. Use o botão **"Modo Foco"** para iniciar o timer Pomodoro com atalhos de teclado (`Espaço` para pausar, `Esc` para minimizar).
- **Rastreamento de Hábitos & Streaks:** Na aba **"Hábitos"**, crie hábitos binários, numéricos ou por timer. Ao clicar em **"Check-in"**, o streak é recalculado atomicamente em $O(1)$ e o mapa de calor anual é atualizado.
- **Conexão Estratégica Hábito $\to$ OKR:** Na aba **"OKRs"**, crie um Objetivo trimestral e vincule um Resultado-Chave a um dos seus hábitos. Ao realizar check-ins diários do hábito, veja a barra de progresso do objetivo avançar automaticamente!
- **Segurança Zero-Knowledge:** Escreva uma reflexão no Diário, salve e inspecione as requisições HTTP na aba Network do DevTools: você verá apenas campos de metadados (`entry_date`, `mood_score`, `tags`) e o envelope cifrado (`ciphertext_base64`, `nonce_base64`, `salt_base64`). Zero texto puro no tráfego ou no banco.
- **PWA & Modo Offline:** No Chrome/Edge, abra o DevTools $\to$ aba **Network** $\to$ marque **"Offline"**. Realize check-ins de hábitos ou complete blocos de rotina. Você verá o badge de rede alertar sobre mutações offline enfileiradas no IndexedDB. Desmarque o modo offline e veja a sincronização automática acontecer em segundo plano com idempotência!

---

### 🛠️ Solução de Problemas Comuns (Troubleshooting)

- **Porta 8080 ou 5173 já em uso:**
  - Caso tenha outro processo utilizando a porta 8080, inicie o backend definindo outra porta: `PORT=8090 cargo run --bin roubit-server --manifest-path api/Cargo.toml`.
  - No frontend, o Vite selecionará automaticamente a próxima porta disponível (ex.: 5174).
- **Erro de conexão com Docker:**
  - Certifique-se de que o daemon do Docker está rodando (`sudo systemctl status docker` no Linux ou Docker Desktop no Mac/Windows).
- **Conflito de pacotes no Frontend:**
  - Execute `npm ci` dentro da pasta `frontend/` para garantir as versões exatas do `package-lock.json`.
- **Como executar toda a bateria de validação do projeto:**
  - Basta rodar `./scripts/pre-commit.sh`. Ele executa `cargo fmt`, `cargo clippy`, todos os testes Rust do workspace, ESLint no frontend, testes Vitest das Views e build do PWA.

---

### Option C: Terminal CLI Standalone & Remote Modes

The command-line interface (`roubit`) can run directly against your local SQLite database or connect remotely to an authenticated server:

```bash
# Compile optimized release binary
cargo build --release --bin roubit --manifest-path api/Cargo.toml

# (Optional) Install to system PATH
sudo cp target/release/roubit /usr/local/bin/
```

#### Daily CLI Commands (Local & Remote Modes):

All domain commands (`routine`, `habit`, `okr`, `journal`) seamlessly support both embedded local SQLite execution and remote authenticated HTTP REST API access via the `--remote <URL>` global flag, `ROUBIT_API_URL` / `ROUBIT_REMOTE_URL` environment variables, or persisted configuration in `~/.roubit/config.json`.

```bash
# 📅 Routines: Create, activate, list, and transition blocks (local or --remote)
roubit routine create --name "Workday" --blocks "08:00-12:00:Deep Work,12:00-13:00:Lunch,13:00-17:00:Projects" --active
roubit routine list
roubit routine start <BLOCK_ID>
roubit routine complete <BLOCK_ID>
roubit routine skip <BLOCK_ID>
roubit routine activate <ROUTINE_ID>
roubit routine delete <ROUTINE_ID>

# ⚡ Habits: Create, check in, view streaks, archive, or delete (local or --remote)
roubit habit create --name "Meditate" --kind timer --target 20 --unit min
roubit habit create --name "Hydration" --kind numeric --target 2500 --unit ml
roubit habit checkin <HABIT_ID> --value 1.0
roubit habit streaks
roubit habit archive <HABIT_ID>
roubit habit delete <HABIT_ID>

# 🎯 OKRs: Create objectives, add Key Results, inspect progress (local or --remote)
roubit okr create --title "Improve Physical Health" --quarter "2026-Q4"
roubit okr add-kr --objective-id <OBJ_ID> --title "Run 100km" --target 100 --unit km
roubit okr list
roubit okr summary
roubit okr delete <OBJ_ID>

# 🔒 Journal: Client-side Zero-Knowledge encryption (Argon2id + AES-256-GCM)
roubit journal write --mood 5 --tags "focus,shipping" --content "Personal reflection content"
roubit journal read --date 2026-11-05
roubit journal list --from 2026-11-01 --to 2026-11-30
roubit journal delete --date 2026-11-05

# 🌐 Remote Mode & Authentication (Features 14 & 15)
roubit auth login --url http://127.0.0.1:8080 --email user@example.com
roubit auth status
roubit habit list --remote http://127.0.0.1:8080
roubit routine list --remote http://127.0.0.1:8080
roubit okr list --remote http://127.0.0.1:8080
roubit journal read --remote http://127.0.0.1:8080
roubit auth logout

# ⚙️ Persistent Configuration (~/.roubit/config.json with 0600 permissions)
roubit config set --db ~/.roubit/roubit.db --api-url http://127.0.0.1:8080 --output-format json
roubit config show
roubit config show --json

# ⚙️ Administrative: Migrate SQLite data to PostgreSQL
roubit admin migrate-to-postgres \
  --from sqlite://roubit.db \
  --to postgres://user:password@localhost:5432/roubit
```

#### Environment Variables:
- `ROUBIT_REMOTE_URL` or `ROUBIT_API_URL`: Target remote REST API base URL (activates remote mode automatically).
- `ROUBIT_AUTH_TOKEN`: Override active Bearer JWT session token.
- `ROUBIT_PASSPHRASE`: Master passphrase for non-interactive Zero-Knowledge journal encryption/decryption.
- `ROUBIT_CREDENTIALS_PATH`: Custom path for saved session credentials (defaults to `~/.roubit/credentials.json`).
- `ROUBIT_CONFIG_PATH`: Custom path for persistent CLI configuration (defaults to `~/.roubit/config.json`).

---

## 🔒 Zero-Knowledge Security Model

Roubit ensures confidentiality using a **zero-knowledge cryptographic pipeline**:

1. **Client-Side Key Derivation:** Passphrases are never transmitted over the network. Keys are derived on-device using **Argon2id** (RFC 9106) with $m=64\,\text{MB}$, $t=3$ iterations, and a unique 16-byte random salt.
2. **Authenticated Encryption:** Plaintext notes are encrypted using **AES-256-GCM** (NIST SP 800-38D) with a fresh 12-byte random IV and a 16-byte authentication tag per entry.
3. **Dual Storage Isolation:** Searchable metadata (mood score 1–5, tags, dates) is indexed for fast timeline navigation, while reflection contents remain strictly ciphertext.
4. **Memory Hygiene:** Ephemeral key buffers implement Rust's `Zeroize` trait, wiping sensitive bytes from RAM immediately after encryption or decryption.

### Backend Authentication Hardening & Anti-DoS Protections
- **Argon2id Thread Isolation:** Password hashing and verification are offloaded to `tokio::task::spawn_blocking`, preventing heavy 64 MiB cryptographic derivations from starving Tokio async I/O worker threads.
- **Timing Attack Defense:** Login execution against nonexistent emails executes an identical Argon2id dummy verification in constant time, preventing account enumeration.
- **Payload & Secret Constraints:** Passwords are capped at 128 characters to thwart algorithmic complexity attacks, and `JWT_SECRET` must be at least 32 characters long (insecure default secret is strictly rejected in production or remote PostgreSQL).
- **Log Header Redaction:** Sensitive `Authorization` and `Cookie` headers are stripped from request trace logging.

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

Roubit enforces zero warnings (`-D warnings`), Clippy linter hygiene, and comprehensive test coverage:

```bash
# Run all Rust workspace unit and integration tests (SQLite in-memory by default)
cargo test --workspace --manifest-path api/Cargo.toml

# Run PostgreSQL & Redis integration and E2E test suites against active containers
TEST_POSTGRES_URL=postgres://roubit_user:roubit_secure_password@localhost:5432/roubit \
TEST_REDIS_URL=redis://127.0.0.1:6379 \
cargo test --workspace --manifest-path api/Cargo.toml

# Run NIST SP 800-38D and RFC 9106 crypto test vectors
cargo test --test crypto_vectors --manifest-path api/Cargo.toml

# Run in-process P99 load benchmark
cargo test --test load_p99_benchmark --manifest-path api/Cargo.toml --release -- --nocapture

# Run frontend ESLint check
npm run lint --prefix frontend

# Run frontend Vitest suite (112 unit and integration tests)
npm test --prefix frontend

# Type-check TypeScript and build production PWA bundle
npm run build --prefix frontend

# Run complete pre-commit quality gate (fmt, clippy, tests, build)
./scripts/pre-commit.sh
```

---

## 📁 Repository Structure

```text
roubit/
├── api/                     # Rust Multi-Crate Workspace
│   ├── crates/core/         # Domain engines, SQLite/PostgreSQL repositories, Users, Habits, Crypto
│   ├── crates/cli/          # Terminal interface (Clap, comfy-table, Prompts, Remote Client)
│   └── crates/server/       # REST API (Axum 0.8), JWT auth, Idempotency, SSE workers & Controllers
├── frontend/                # Vue 3 PWA (Composition API, Pinia, TailwindCSS, Web Crypto, Workbox)
├── docs/                    # Technical specifications, development plans & benchmarks
│   ├── 00-especificacao-tecnica.md
│   ├── 01-plano-de-desenvolvimento.md
│   ├── 02-status-atividades.md
│   ├── 04-plano-de-melhorias-e-correcoes.md
│   ├── features/            # Feature prompts & engineering activities (16 features)
│   ├── improvements/        # Cycle improvements & QA prompts (6 phases)
│   └── benchmarks/          # P99 latency and RSS load reports
├── scripts/                 # Quality assurance scripts (pre-commit.sh)
├── tests/load/              # K6 distributed load testing scenarios
├── docker-compose.yml       # Production stack orchestration (PostgreSQL + Redis + API + SPA)
├── LICENSE-MIT              # MIT License
└── LICENSE-APACHE           # Apache 2.0 License
```

---

## 📚 Technical Documentation

For in-depth architectural blueprints and activity tracking:
- 📐 **[Technical Specification & Architecture](docs/00-especificacao-tecnica.md)**
- 📋 **[Development Plan & WBS (16 Features)](docs/01-plano-de-desenvolvimento.md)**
- 🚦 **[Activity Status Tracker (70/70 Tasks Completed)](docs/02-status-atividades.md)**
- 📑 **[Features & Activities Matrix (All 16 Features)](docs/features/README.md)**
- 📈 **[Official Benchmark & Load Test Report](docs/benchmarks/01-load-test-p99-memory-report.md)**
- 📱 **[Frontend SPA & PWA Architecture](frontend/README.md)**

---

## 📄 License

Dual-licensed under either of:
- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
