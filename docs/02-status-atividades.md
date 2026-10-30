# Roubit — Painel de Estado das Atividades (Activity Tracker)

> **Status Geral do Projeto:** 🟡 Em Desenvolvimento (Fase 2: API REST & Idempotência)  
> **Última Atualização:** 2026-10-30  
> **Legenda:**  
> 🟢 **CONCLUÍDO** | 🟡 **EM PROGRESSO** | ⚪ **NÃO INICIADO**

---

## Resumo de Progresso

| Feature | Total de Atividades | Concluídas | Em Progresso | Não Iniciadas | Status |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **01. Workspace Setup & Tooling** | 4 | 4 | 0 | 0 | 🟢 Concluído |
| **02. Criptografia & Memory Safety** | 4 | 4 | 0 | 0 | 🟢 Concluído |
| **03. Gestão de Rotinas (Time Blocks)** | 5 | 5 | 0 | 0 | 🟢 Concluído |
| **04. Rastreamento de Hábitos (Streaks)** | 5 | 5 | 0 | 0 | 🟢 Concluído |
| **05. Objetivos & OKRs** | 4 | 4 | 0 | 0 | 🟢 Concluído |
| **06. Journaling Criptografado** | 4 | 4 | 0 | 0 | 🟢 Concluído |
| **07. Aplicação CLI (Terminal)** | 5 | 5 | 0 | 0 | 🟢 Concluído |
| **08. API REST & Idempotência** | 6 | 3 | 0 | 3 | 🟡 Em Progresso |
| **09. Frontend SPA (Vue 3)** | 6 | 0 | 0 | 6 | ⚪ Não Iniciado |
| **10. Infraestrutura Redis & PostgreSQL** | 5 | 0 | 0 | 5 | ⚪ Não Iniciado |
| **TOTAL** | **48** | **34** | **0** | **14** | **70.8%** |

---

## Detalhamento por Feature

### Feature 01: Workspace Setup & Tooling
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **01.1** | Cargo Workspace & Project Layout | [activity-01-cargo-workspace-init.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-01-cargo-workspace-init.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **01.2** | Linting, Formatting & Quality Standards | [activity-02-linting-formatting-git-hooks.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-02-linting-formatting-git-hooks.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **01.3** | GitHub Actions CI Pipeline | [activity-03-github-actions-ci-pipeline.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-03-github-actions-ci-pipeline.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **01.4** | Configuração de Ambiente & Tracing | [activity-04-environment-configuration-logging.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/01-workspace-setup/activity-04-environment-configuration-logging.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |

---

### Feature 02: Criptografia Seletiva & Memory Safety
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **02.1** | Derivação de Chaves com Argon2id | [activity-01-argon2id-key-derivation.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-01-argon2id-key-derivation.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **02.2** | Cifra Autenticada AES-256-GCM | [activity-02-aes-256-gcm-cipher.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-02-aes-256-gcm-cipher.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **02.3** | Sanitização de Memória com Zeroize | [activity-03-zeroize-memory-safety.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-03-zeroize-memory-safety.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **02.4** | Vetores de Teste NIST e Benchmarks | [activity-04-cryptographic-test-vectors-benchmarks.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/02-cryptography/activity-04-cryptographic-test-vectors-benchmarks.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |

---

### Feature 03: Gestão de Rotinas (Time Blocks)
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **03.1** | Modelos de Domínio & Schema SQLite | [activity-01-routine-domain-models-schema.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-01-routine-domain-models-schema.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **03.2** | Repositório SQLite em Modo WAL | [activity-02-sqlite-repository-wal.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-02-sqlite-repository-wal.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **03.3** | Motor de Validação & Detecção de Overlap | [activity-03-time-block-validation-overlap-engine.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-03-time-block-validation-overlap-engine.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **03.4** | Tracker de Execução e Aderência Diária | [activity-04-daily-routine-progress-tracker.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-04-daily-routine-progress-tracker.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **03.5** | Testes de Integração de Rotinas | [activity-05-routine-unit-integration-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/03-routines/activity-05-routine-unit-integration-tests.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |

---

### Feature 04: Rastreamento de Hábitos & Métricas
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **04.1** | Modelos de Hábitos (Binário/Numérico/Timer) | [activity-01-habit-domain-models-schema.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-01-habit-domain-models-schema.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **04.2** | Repositório de Hábitos & Check-in Atômico | [activity-02-habit-repository-sqlx.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-02-habit-repository-sqlx.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **04.3** | Motor Resiliente de Cálculo de Streaks | [activity-03-streak-calculation-engine.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-03-streak-calculation-engine.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **04.4** | Análise de Aderência & Heatmap Anual | [activity-04-adherence-metrics-analytics.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-04-adherence-metrics-analytics.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **04.5** | Suíte de Testes de Integração de Hábitos | [activity-05-habit-unit-integration-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/04-habits/activity-05-habit-unit-integration-tests.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |

---

### Feature 05: Gestão de Objetivos & OKRs
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **05.1** | Modelos de Domínio & Schema de OKRs | [activity-01-okr-domain-models-schema.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-01-okr-domain-models-schema.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **05.2** | Repositório SQLx de OKRs | [activity-02-okr-repository-sqlx.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-02-okr-repository-sqlx.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **05.3** | Motor de Vínculo Hábito-KR & Progresso | [activity-03-habit-to-kr-linkage-progress-engine.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-03-habit-to-kr-linkage-progress-engine.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **05.4** | Testes de Integração & Cascata de OKRs | [activity-04-okr-unit-integration-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/05-okrs/activity-04-okr-unit-integration-tests.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |

---

### Feature 06: Journaling Criptografado (Dual-Storage)
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **06.1** | Arquitetura Dual-Storage & Schema | [activity-01-journal-domain-models-schema.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-01-journal-domain-models-schema.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **06.2** | Repositório SQLite com Busca por Metadados | [activity-02-journal-repository-sqlx.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-02-journal-repository-sqlx.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **06.3** | Serviço Criptográfico do Diário | [activity-03-journal-service-crypto-integration.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-03-journal-service-crypto-integration.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |
| **06.4** | Testes de Zero-Knowledge & Tampering | [activity-04-journal-unit-integration-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/06-journaling/activity-04-journal-unit-integration-tests.prompt) | 🟢 CONCLUÍDO | 2026-10-29 |

---

### Feature 07: Aplicação CLI (Rust / Clap)
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **07.1** | Árvore de Comandos Modular com Clap | [activity-01-clap-command-structure.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-01-clap-command-structure.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **07.2** | UI de Terminal (Tabelas, Cores e Barras) | [activity-02-terminal-ui-tables-colors.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-02-terminal-ui-tables-colors.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **07.3** | Prompts Seguros & Editor de Terminal | [activity-03-interactive-prompts-journal-editor.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-03-interactive-prompts-journal-editor.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **07.4** | Adaptador de Modo Híbrido (Local vs HTTP) | [activity-04-hybrid-mode-local-http-adapter.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-04-hybrid-mode-local-http-adapter.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **07.5** | Testes E2E Automatizados da CLI | [activity-05-cli-e2e-automated-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/07-cli/activity-05-cli-e2e-automated-tests.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |

---

### Feature 08: API REST & Idempotência (Axum)
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **08.1** | Arquitetura do Servidor Axum & Tracing | [activity-01-axum-server-architecture-tracing.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-01-axum-server-architecture-tracing.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **08.2** | Autenticação, JWT & Headers OWASP | [activity-02-authentication-jwt-security-headers.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-02-authentication-jwt-security-headers.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **08.3** | Middleware de Idempotência (RF05) | [activity-03-idempotency-middleware-rf05.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-03-idempotency-middleware-rf05.prompt) | 🟢 CONCLUÍDO | 2026-10-30 |
| **08.4** | Controladores REST de Rotinas, Hábitos e OKRs | [activity-04-domain-endpoints-routines-habits-okrs.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-04-domain-endpoints-routines-habits-okrs.prompt) | ⚪ NÃO INICIADO | - |
| **08.5** | Controladores do Diário Zero-Knowledge | [activity-05-journal-encrypted-payload-endpoints.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-05-journal-encrypted-payload-endpoints.prompt) | ⚪ NÃO INICIADO | - |
| **08.6** | Testes de Integração de API & SAST | [activity-06-api-integration-sast-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/08-api-rest/activity-06-api-integration-sast-tests.prompt) | ⚪ NÃO INICIADO | - |

---

### Feature 09: Frontend SPA (Vue 3 / Vite)
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **09.1** | Scaffolding Vue 3, TS & TailwindCSS | [activity-01-vite-vue3-tailwind-scaffolding.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-01-vite-vue3-tailwind-scaffolding.prompt) | ⚪ NÃO INICIADO | - |
| **09.2** | Cliente HTTP, Idempotência & Pinia | [activity-02-api-client-idempotency-state-management.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-02-api-client-idempotency-state-management.prompt) | ⚪ NÃO INICIADO | - |
| **09.3** | Criptografia no Cliente via Web Crypto API | [activity-03-web-crypto-selective-encryption.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-03-web-crypto-selective-encryption.prompt) | ⚪ NÃO INICIADO | - |
| **09.4** | Dashboard Unificado (Rotinas & Hábitos) | [activity-04-daily-dashboard-routines-habits.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-04-daily-dashboard-routines-habits.prompt) | ⚪ NÃO INICIADO | - |
| **09.5** | Visualização de Heatmap & Árvore de OKRs | [activity-05-habits-heatmap-okrs-views.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-05-habits-heatmap-okrs-views.prompt) | ⚪ NÃO INICIADO | - |
| **09.6** | UI do Diário Criptografado & Testes Vitest | [activity-06-encrypted-journal-component-tests.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/09-frontend-spa/activity-06-encrypted-journal-component-tests.prompt) | ⚪ NÃO INICIADO | - |

---

### Feature 10: Infraestrutura Redis & PostgreSQL
| ID | Atividade | Arquivo | Estado | Commit / Data |
| :---: | :--- | :--- | :---: | :---: |
| **10.1** | Redis Connection & Locks Distribuídos | [activity-01-redis-connection-distributed-locks.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-01-redis-connection-distributed-locks.prompt) | ⚪ NÃO INICIADO | - |
| **10.2** | Lembretes Agendados via Redis TTL | [activity-02-scheduled-reminders-ttl-keyspace.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-02-scheduled-reminders-ttl-keyspace.prompt) | ⚪ NÃO INICIADO | - |
| **10.3** | Suporte Dual Database (SQLite / Postgres) | [activity-03-dual-database-postgresql-support.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-03-dual-database-postgresql-support.prompt) | ⚪ NÃO INICIADO | - |
| **10.4** | Dockerfiles & Orquestração Docker Compose | [activity-04-docker-compose-production-orchestration.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-04-docker-compose-production-orchestration.prompt) | ⚪ NÃO INICIADO | - |
| **10.5** | Testes de Carga, P99 < 10ms & RAM < 25MB | [activity-05-load-testing-p99-benchmarks.prompt](file:///home/filipe/Documents/projects/roubit/docs/features/10-infrastructure-redis-postgres/activity-05-load-testing-p99-benchmarks.prompt) | ⚪ NÃO INICIADO | - |
