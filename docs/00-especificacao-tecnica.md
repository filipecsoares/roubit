# Roubit — Especificação Técnica, Arquitetura e Roteiro de Engenharia

# 1. Visão Geral e Filosofia do Projeto

* **Nome Oficial:** Roubit (Routine + Habit).  
* **Objetivo Prático:** Plataforma pessoal de alta performance, integrada e minimalista para gestão diária de Rotinas, Hábitos, Objetivos e Journaling (Diário de Bordo Pessoal).  
* **Pilares:**  
  * **Foco Funcional & Simplicidade:** Eliminar complexidade desnecessária com foco nos 4 módulos essenciais da rotina.  
  * **Performance Extrema & Memory Safety:** Backend em Rust (Axum, Tokio, SQLx). Consumo ínfimo de memória e zero runtime overhead.  
  * **Criptografia Seletiva Pragmática (Field-Level Encryption):** Apenas os textos sensíveis do Journaling e anotações pessoais são cifrados no cliente com AES-256-GCM. Metadados operacionais e contadores ficam abertos no banco para consultas instantâneas.  
  * **Custo Zero e Padrões Abertos:** Começa com SQLite local e evolui para PostgreSQL na fase final. Versionamento e pipelines via GitHub.

# 2. Decisões Arquiteturais e Stack Tecnológica

* **Backend & CLI:** Rust (Tokio, Axum, SQLx, Clap).  
* **Frontend SPA:** Vue.js 3 (Composition API, Vite, Pinia, TailwindCSS).  
* **Banco de Dados:** SQLite (modo WAL) nas Fases 1 a 3; transição transparente para PostgreSQL via Docker Compose na Fase 4.  
* **Cache, Idempotência & Filas:** Redis para locks distribuídos, cache de idempotência e lembretes por TTL.  
* **Criptografia Seletiva:** AES-256-GCM com derivação de chave mestra via Argon2id.

# 3. Desenho da Arquitetura do Sistema

```
+-----------------------+               +-----------------------+
|      CLI (Rust)       |               |     SPA (Vue.js 3)    |
|    (Fase 1 Local &    |               |  (Web Crypto API:     |
|    Fase 2 via HTTP)   |               |   AES-256-GCM local)  |
+-----------+-----------+               +-----------+-----------+
            |                                       |
            +-------------------+-------------------+
                                | HTTPS / REST (JSON)
                                | Headers: Idempotency-Key, Bearer JWT
                                v
       +-------------------------------------------------+
       |           Backend API Service (Rust / Axum)     |
       |                                                 |
       |  [Security & Rate Limit Middleware]             |
       |  [Idempotency-Key Interceptor & Guard]          |
       |                                                 |
       |  [Domain & Use Cases]:                          |
       |    - Routine Manager (Time Blocks)              |
       |    - Habit Tracker (Streaks & Metrics)          |
       |    - Goal & OKR Alignment Engine                |
       |    - Journaling (Encrypted Payload Storage)     |
       +------------------------+------------------------+
                                |
                +---------------+---------------+
                |                               |
                v (SQLx Compile-time queries)   v (Redis CLI/Driver)
     +----------------------+        +----------------------+
     |   Database Storage   |        |   Redis In-Memory    |
     |                      |        |                      |
     | - Fase 1-3: SQLite   |        | - Idempotency Keys   |
     |   (WAL mode, zero-cfg|        |   (locks & responses)|
     | - Fase 4: PostgreSQL |        | - Scheduled Reminders|
     |   (Container Docker) |        |   via TTL & Keyspace |
     +----------------------+        +----------------------+
```

# 4. Requisitos Funcionais (Core do Roubit)

* **RF01 — Gestão de Rotinas (Time Blocks):** Configuração e acompanhamento de blocos estruturados do dia (Rotina Matinal, Foco Profundo, Descompressão Noturna) com progresso em tempo real.  
* **RF02 — Rastreamento de Hábitos (Habit Tracker):** Cadastro e check-in com medição binária, numérica (páginas, km, ml) ou timer (duração). Cálculo automático de streaks (dias consecutivos) e taxas de aderência.  
* **RF03 — Gestão de Objetivos e Metas (OKRs):** Metas de médio e longo prazo desdobradas em resultados-chave mensuráveis, conectados aos hábitos do dia a dia.  
* **RF04 — Journaling Criptografado:** Diário pessoal de intenções e reflexões noturnas com criptografia AES-256-GCM no cliente. O servidor armazena apenas dados cifrados.  
* **RF05 — Idempotência Transacional:** Cabeçalho Idempotency-Key para evitar duplicidade de registros em check-ins decorrentes de falhas de conexão.

# 5. Requisitos Não Funcionais e Segurança

* **RNF01 — Criptografia Seletiva no Cliente:** Dados sensíveis protegidos sem comprometer a velocidade de agregação dos dados de hábitos.  
* **RNF02 — Resiliência e Memory Safety:** Benefício intrínseco do compilador Rust.  
* **RNF03 — Hardening OWASP:** Cookies HttpOnly/Secure/SameSite=Strict, proteção CSRF e queries SQL parametrizadas via SQLx.  
* **RNF04 — Desempenho:** P99 < 10ms local e footprint < 25MB RAM.

# 6. Roteiro Evolutivo (Fases de Desenvolvimento)

* **Fase 1: Fundação do Domínio & CLI Local com SQLite (Rust).** CLI autônomo, modelo de dados SQLx, cifra AES-256-GCM e pipeline básico de testes no GitHub Actions.  
* **Fase 2: API HTTP REST Idempotente (Rust / Axum).** Exposição dos endpoints REST, middleware de idempotência, autenticação JWT e pipeline SAST no GitHub Actions.  
* **Fase 3: Frontend SPA Reativo em Vue.js 3.** Dashboard visual integrado (Rotinas + Hábitos), painel de OKRs, tela de Journaling com Web Crypto API e pipeline de build no GitHub Actions.  
* **Fase 4: Redis TTL, Notificações e Transição para PostgreSQL.** Filas de lembretes via Redis TTL, avisos nativos, suporte a PostgreSQL via Docker Compose e orquestração completa em 1 comando.

# 7. Governança e Pipelines no GitHub

1. **Branches:** main (estável e protegida), develop e feature/*.  
2. **GitHub Actions:**  
   1. **rust-ci.yml:** cargo fmt, cargo clippy, cargo test, cargo audit.  
   2. **vue-ci.yml:** ESLint, type-checking e build de produção.
