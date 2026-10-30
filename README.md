# Roubit (Routine + Habit)

> **O seu sistema integrado de rotina diária, hábitos, metas e reflexão privada.**  
> Planeje o seu dia com blocos de tempo, construa consistência com hábitos inteligentes, alinhe suas ações diárias aos seus objetivos de longo prazo e registre suas reflexões em um diário com privacidade absoluta.

[![Progresso](https://img.shields.io/badge/Progresso-48%2F48%20Atividades%20(100%25)-brightgreen)](docs/02-status-atividades.md)
[![Privacidade](https://img.shields.io/badge/Privacidade-Zero--Knowledge%20(Criptografado)-blueviolet)](docs/00-especificacao-tecnica.md#5-segurança--criptografia)
[![Docker Ready](https://img.shields.io/badge/Docker-Pronto%20para%20Uso-2496ED?logo=docker)](docker-compose.yml)
[![Licença: MIT/Apache-2.0](https://img.shields.io/badge/Licen%C3%A7a-MIT%20ou%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

## 🎯 Por que o Roubit?

Muitas ferramentas de produtividade funcionam isoladas: você tem um aplicativo para tarefas diárias, outro para rastrear hábitos, uma planilha corporativa para metas e um bloco de notas para pensamentos pessoais. O resultado é dispersão, perda de tempo e falta de conexão entre o que você faz hoje e onde quer chegar no futuro.

O **Roubit** resolve isso unindo os quatro pilares fundamentais da vida pessoal em uma experiência única, rápida e sem distrações:

1. **Rotinas Claras:** Menos ansiedade sabendo exatamente o que fazer em cada momento do dia.
2. **Hábitos Conectados:** Seus hábitos diários alimentam diretamente seus grandes objetivos de vida.
3. **Privacidade Absoluta:** Suas anotações íntimas são criptografadas no seu dispositivo antes de serem salvas. Ninguém além de você consegue ler.
4. **Agilidade Instantânea:** Interface web moderna ou comandos diretos no terminal para quem prefere o teclado.

---

## ✨ Funcionalidades Principais

### ⏱️ 1. Planejamento Diário por Blocos de Tempo (Time Blocking)
- **Organização Cronológica:** Monte seu dia definindo blocos de foco (ex.: *Trabalho Profundo*, *Treino*, *Almoço*, *Estudos*, *Descanso*).
- **Sem Conflitos de Horário:** O sistema valida automaticamente sua grade e impede agendamentos sobrepostos.
- **Linha do Tempo em Tempo Real:** Acompanhe qual atividade está acontecendo agora e atualize o status com um clique (`Pendente` $\to$ `Em Andamento` $\to$ `Concluído` ou `Pulado`).
- **Aderência Diária:** Veja no final do dia a sua porcentagem real de cumprimento da rotina planejada.

### ⚡ 2. Rastreador de Hábitos com Streaks Inteligentes
- **3 Tipos de Hábitos Flexíveis:**
  - **Sim/Não (Binário):** Tarefas simples de marcar (ex.: *Tomar suplementos*, *Arrumar a cama*).
  - **Quantitativo (Numérico):** Atingimento de metas diárias acumuladas (ex.: *Beber 2.500 ml de água*, *Ler 20 páginas*).
  - **Cronômetro (Timer):** Medição de tempo dedicado com cronômetro integrado (ex.: *30 min de meditação*, *45 min de estudo*).
- **Streaks Resilientes:** Configure dias específicos da semana para cada hábito. Se um hábito é apenas de segunda a sexta, seus dias de descanso no fim de semana não quebram sua sequência de dias seguidos (*streak*)!
- **Mapa de Calor Anual:** Visualize seu histórico de consistência em um gráfico de calor anual de 52 semanas no estilo GitHub.

### 🎯 3. Alinhamento de Objetivos & OKRs
- **Metas Trimestrais Claras:** Crie seus grandes objetivos para o trimestre (ex.: *Conquistar Fluência em Espanhol*, *Melhorar Condicionamento Físico*).
- **Resultados-Chave Mensuráveis (KRs):** Divida o objetivo em métricas tangíveis (ex.: *Fazer 60 lições*, *Correr 100 km*).
- **Evolução Automática:** Conecte seus hábitos diários aos seus objetivos. Conforme você registra seus hábitos no dia a dia, a barra de progresso do seu objetivo trimestral sobe automaticamente, sem retrabalho!

### 🔒 4. Diário Pessoal Criptografado (Zero-Knowledge)
- **Privacidade Inviolável:** Suas reflexões e pensamentos são cifrados diretamente no seu navegador ou terminal usando uma senha mestra pessoal (algoritmo AES-256-GCM).
- **Nem o Servidor Consegue Ler:** O texto descriptografado nunca transita pela rede e nunca é salvo em texto puro no banco de dados. Mesmo se o servidor for invadido, suas anotações continuam ilegíveis.
- **Acompanhamento de Humor e Tags:** Registre sua nota de humor do dia (1 a 5) e tags temáticas para identificar padrões de bem-estar ao longo dos meses.

### 🖥️ 5. Duas Formas de Usar: Web Moderna ou Terminal (CLI)
- **Interface Web Completa:** Desenvolvida em Vue 3, com visual moderno, modo escuro e gráficos intuitivos.
- **CLI Rápida de Terminal:** Para desenvolvedores e usuários avançados que preferem gerenciar o dia sem tirar as mãos do teclado.

---

## 🚀 Como Executar o Roubit

Você pode executar o Roubit de forma imediata via **Docker** (recomendado para uso) ou em modo de **desenvolvimento local**.

### Pré-requisitos Básicos
- **Para rodar com Docker:** Docker 24+ e Docker Compose v2+.
- **Para rodar o código-fonte:** Rust 1.85+ e Node.js 20+.

---

### Opção 1: Inicialização em 1 Comando (Docker Compose — Recomendado)

Esta opção inicializa todo o ecossistema pronto para uso (Interface Web, Servidor API, Banco de Dados e Cache):

```bash
# 1. Clone o repositório
git clone https://github.com/filipecsoares/roubit.git
cd roubit

# 2. Inicie todos os serviços em segundo plano
docker compose up -d
```

Pronto! Acesse no seu navegador:
- 🌐 **Interface Web (Aplicação):** [http://localhost](http://localhost)
- 🔌 **API REST de Apoio:** [http://localhost:8080](http://localhost:8080)

Para parar os serviços quando terminar:
```bash
docker compose down
```

---

### Opção 2: Desenvolvimento Local (Nativo)

Se você deseja alterar o código ou rodar diretamente na sua máquina:

#### 1. Iniciar o Servidor (Backend em Rust)
Por padrão, o servidor utiliza um banco local SQLite automático que não requer configuração adicional:
```bash
cargo run --bin roubit-server
```
*O servidor estará ativo em `http://127.0.0.1:8080`.*

#### 2. Iniciar a Interface Web (Frontend em Vue 3)
Em outro terminal:
```bash
cd frontend
npm install
npm run dev
```
*Abra [http://localhost:5173](http://localhost:5173) no navegador.*

---

### Opção 3: Usando Diretamente no Terminal (CLI)

O Roubit conta com uma ferramenta de linha de comando (`roubit`) que gerencia suas rotinas e hábitos localmente:

```bash
# Compilar o executável
cargo build --release --bin roubit

# Adicionar aos comandos do sistema (opcional)
sudo cp target/release/roubit /usr/local/bin/
```

#### Comandos Rápidos do Dia a Dia:
```bash
# 📅 Ver os blocos da rotina de hoje
roubit routine list

# Iniciar ou concluir um bloco
roubit routine start <ID_DO_BLOCO>
roubit routine complete <ID_DO_BLOCO>

# ⚡ Registrar cumprimento de um hábito
roubit habit checkin <ID_DO_HABITO>

# Ver suas sequências de hábitos (streaks)
roubit habit streaks

# 🎯 Ver seus objetivos e metas trimestrais
roubit okr list

# 🔒 Escrever no seu diário com criptografia (solicita sua senha mestra)
roubit journal write --mood 5 --tags "foco,produtividade"

# Ler o diário de hoje
roubit journal read
```

---

## 🛡️ Como Funciona a Criptografia do Diário?

1. Ao abrir o diário, você define uma **senha mestra pessoal**.
2. Essa senha é processada no seu próprio dispositivo para gerar uma chave criptográfica temporária.
3. Quando você clica em salvar, seu texto é transformado em um código ilegível (*ciphertext*) antes de ser enviado.
4. Ao fechar a aba ou bloquear o cofre, a chave é imediatamente apagada da memória volátil. Nem você nem ninguém conseguirá ler o texto sem informar a senha novamente.

---

## 📚 Documentação Técnica para Desenvolvedores

Se você é desenvolvedor e quer explorar a arquitetura interna, os benchmarks de performance ou as especificações das rotas, consulte os documentos detalhados na pasta [`docs/`](docs/):

- 📐 **[Especificação Técnica Completa](docs/00-especificacao-tecnica.md):** Decisões de arquitetura, padrões e diagramas.
- 📋 **[Plano de Desenvolvimento & WBS](docs/01-plano-de-desenvolvimento.md):** Estrutura analítica das 10 features do projeto.
- 🚦 **[Painel de Estado das Atividades](docs/02-status-atividades.md):** Detalhamento das 48 tarefas concluídas.
- ⚡ **[Relatório Oficial de Carga e Benchmark](docs/benchmarks/01-load-test-p99-memory-report.md):** Validação de P99 < 5ms e consumo de memória < 14MB.

---

## 📄 Licença

O Roubit é disponibilizado sob licença dupla:
- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

Escolha a que melhor atende às suas necessidades.
