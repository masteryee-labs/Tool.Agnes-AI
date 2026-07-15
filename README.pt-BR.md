# Agnes AI — Agente de IA para programação desktop open-source em Rust

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#instalação-e-compilação)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-compatible-skills)

---

## O que é Agnes AI?

**Agnes AI é um agente de IA para programação desktop, open-source, escrito em Rust puro com uma GUI nativa em egui (zero Chromium / WebView2).** Ele executa um **pipeline de validação zero-trust com 22 agentes**, de modo que toda ação proposta pelo modelo de linguagem é verificada por portões de segurança determinísticos antes de tocar seu sistema de arquivos, shell ou rede. Ele também traz um **loop autônomo orientado a objetivos** (Discover → Plan → Execute → Verify → Iterate) com uma **arquitetura de sub-agentes** (Planner / Generator / Evaluator) e **isolamento via Git worktree** para execução paralela segura.

Agnes AI é uma **alternativa gratuita e local-first ao Claude Code, Cursor, Aider e Continue.dev** — sua chave de API e seu código nunca saem da sua máquina, o binário é minúsculo, a inicialização é instantânea e a UI é um app nativo minimalista em modo escuro (sem navegador embutido, sem Electron).

> **Em uma linha:** Um agente de IA desktop nativo em Rust, de alta velocidade e alta defesa, que nunca confia no "funcionou" verbal do modelo — apenas Exit Code 0 e stderr vazio contam como sucesso.

---

## Por que Agnes AI? (vs Claude Code / Cursor / Aider / Continue.dev)

| Recurso | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **Runtime** | GUI nativa em Rust (egui) | Terminal | IDE baseada em Electron | Terminal | Plugin VS Code/JetBrains |
| **Tamanho do binário** | Minúsculo (~MB) | Médio | Grande (~100 MB+) | Minúsculo | Depende do IDE hospedeiro |
| **Navegador embutido** | Nenhum (zero WebView2) | Nenhum | Chromium | Nenhum | O do IDE hospedeiro |
| **Modelo de segurança** | Pipeline zero-trust com 22 agentes, veto de um voto | Limitado | Limitado | Mínimo | Mínimo |
| **Loop autônomo** | Sim (5 estágios, orientado a objetivos) | Sim (modo agente) | Não | Não | Não |
| **Arquitetura de sub-agentes** | Sim (Planner/Generator/Evaluator) | Sim | Não | Não | Não |
| **Isolamento via Git worktree** | Sim (sub-agentes paralelos) | Não | Não | Não | Não |
| **Suporte a MCP** | Sim (formato `.mcp.json` do Claude) | Sim | Parcial | Não | Não |
| **Claude Skills** | Sim (`.claude/skills/`) | Sim | Não | Não | Não |
| **Memória RAG local** | Sim (FTS5 + funil de 3 estágios) | Limitado | Limitado | Não | Limitado |
| **Memória entre sessões** | Sim (lições/armadilhas/estado do loop) | Não | Não | Não | Não |
| **Sandbox WASM / Docker** | Sim | Não | Não | Não | Não |
| **Bindings móveis** | Sim (UniFFI, iOS/Android) | Não | Não | Não | Não |
| **Multimodal (imagem/vídeo)** | Sim | Sim | Sim | Não | Não |
| **Rotação de múltiplas chaves de API** | Sim (amigável a free-tier) | Não | Não | Não | Não |
| **Open source** | Sim (MIT) | Não | Não | Sim | Sim |
| **Preço** | Gratuito (traga sua própria chave) | Pago | Pago | Gratuito (BYO key) | Gratuito/Pago |

**Agnes AI é ideal para desenvolvedores que querem:**
- Um **agente de IA para programação local-first e que respeita a privacidade** (sem retransmissão do seu código pela nuvem)
- **Garantias fortes de segurança** (validação zero-trust, sandboxing, veto contra vazamento de segredos)
- Um **app desktop nativo e leve** em vez de Electron ou um terminal
- **Execução autônoma orientada a objetivos** com critérios de sucesso verificáveis
- **Sustentabilidade no free-tier** via rotação de múltiplas chaves e proteção contra rate limit

---

## Principais recursos

### Experiência principal
- **GUI nativa em Rust** — eframe/egui + wgpu, sem navegador embutido, inicialização instantânea, footprint minúsculo
- **UI escura minimalista** — paleta preto + branco pura inspirada no Claude Code / Codex / Devin / Antigravity 2.0; sem cores de marca que distraem
- **Execução silenciosa** — todos os processos filhos (comandos de shell, compilador, git, servidores MCP) rodam com `CREATE_NO_WINDOW` no Windows; nenhuma janela de CMD/PowerShell aparece na sua área de trabalho

### Workspaces
- **Modo duplo Projeto / Global** — abas na barra lateral alternam entre:
  - **Projetos**: crie um projeto a partir de qualquer pasta; toda sessão de chat é aninhada sob seu projeto; conversas persistem em SQLite e retomam exatamente de onde você parou
  - **Global**: uma aba dedicada para operação em todo o computador, onde toda ação exige confirmação por item

### Loop autônomo (Phase 5)
- **Loop orientado a objetivos** — dê a ele um objetivo e uma condição de saída; ele executa Discover → Plan → Execute → Verify → Iterate por conta própria até que a condição seja atendida ou o limite de iterações seja alcançado
- **Arquitetura de sub-agentes** — três papéis independentes com prompts e estado de conversa separados:
  - **Planner** — decompõe o objetivo em subtarefas atômicas
  - **Generator** — implementa uma subtarefa por execução, chamando as ferramentas `write_file` / `run_command`
  - **Evaluator** — verifica a saída do Generator de forma independente; rejeita afirmações de "sucesso" apenas verbais
- **Isolamento via Git worktree** — cada sub-agente Generator trabalha em um git worktree + branch isolados; sub-agentes paralelos nunca pisam nos arquivos uns dos outros; trabalho concluído é mesclado de volta ao branch principal
- **Memória entre sessões** — lições, armadilhas e estado do loop persistem em `.agent/memory/` para que o agente continue de onde parou entre sessões

### Segurança e validação
- **Pipeline de validação com 22 agentes** — toda chamada de ferramenta do modelo é verificada por portões determinísticos (confinamento de caminho, detecção de shell injection, scan de vazamento de segredos, auditoria de AI-slop, …) com veto de um voto
- **Alinhamento com sandbox** — arquivos `.rs` escritos são compilados (e seus testes executados) imediatamente; "afirma sucesso mas não compila" é rejeitado na hora
- **Sandbox WASM** — código não confiável roda pelo interpretador `wasmi` em Rust puro com um linker vazio (sem host imports → sem I/O/syscalls/rede) e medição por fuel
- **Sandbox Docker** — tarefas de nível de compilação rodam em um container com `--network=none`, `--rm`, workspace montado em `/work`; args vetorizados (sem shell)
- **Sem confiança verbal** — Exit Code == 0 e stderr vazio são a única definição de sucesso; o "funcionou" verbal do modelo nunca é confiável

### Compatibilidade
- **Skills compatíveis com Claude** — coloque arquivos `SKILL.md` sob `.claude/skills/<name>/` no seu workspace; invoque-os digitando `/name` no chat. As regras de projeto do `CLAUDE.md` são carregadas automaticamente
- **MCP compatível com Claude** — coloque um `.mcp.json` padrão na raiz do seu workspace, ou adicione servidores em Settings → MCP Servers; listas de ferramentas conectadas são expostas ao modelo automaticamente

### Desempenho
- **Memória em camadas** — fragmentação por janela deslizante + RAG em funil de 3 estágios sobre um índice FTS5, com marcas d'água de destilação para evitar requeimar tokens
- **Rate limiting e proteção de 20 RPM** — um limitador global compartilhado de token-bucket controla toda chamada de API (destilação e retrieval inclusos); `acquire()` aguarda o reabastecimento em vez de rejeitar, então rajadas nunca violam o limite de 20 requisições/minuto do free-tier. Em um 429 o cliente aplica backoff exponencial baseado em multiplicador. Todo parâmetro é orientado por config (`max_rpm`, configurações de retry backoff) — sem magic numbers
- **Rotação de múltiplas chaves de API** — alterna entre chaves de múltiplas contas (baseado em contagem + troca forçada em HTTP 420/429) para permanecer totalmente gratuito sem atingir o rate limit de qualquer conta individual
- **Economia de tokens** — orçamento de tokens por sessão com bloqueio rígido, medidor de orçamento ao vivo na barra de título. A contagem de requisições é reduzida por design: o Stage 0 faz uma busca local na memória FTS5 que, em caso de acerto, pula a chamada de API de retrieval inteiramente (0 chamadas de API), e os estágios 1+2 do RAG em funil foram mesclados em uma única chamada (2 chamadas → 1)

---

## Perguntas frequentes (FAQ)

### O Agnes AI é gratuito?
Sim. Agnes AI é open-source (MIT) e gratuito. Você traz sua própria chave de API (ex. uma chave Agnes / compatível com OpenAI). O recurso de rotação de múltiplas chaves permite combinar várias contas do free-tier para evitar rate limits inteiramente.

### O Agnes AI envia meu código para a nuvem?
O Agnes AI em si roda 100% localmente. Seu código nunca é retransmitido por qualquer servidor do Agnes AI. O único tráfego de rede são as chamadas diretas de API que você configura para o seu provedor de LLM (o que é necessário para qualquer agente baseado em LLM). Sua chave de API fica em `config.local.toml` (git-ignored) e nunca entra no controle de versão nem no contexto do modelo.

### Como o Agnes AI difere do Claude Code / Cursor / Aider?
- **vs Claude Code**: Agnes AI é open-source, tem uma GUI nativa (não apenas terminal), adiciona um pipeline de validação zero-trust com 22 agentes, isolamento via Git worktree para sub-agentes paralelos e sandboxing WASM/Docker.
- **vs Cursor**: Agnes AI é um app nativo autônomo (sem Electron/Chromium), open-source, com um loop autônomo orientado a objetivos e arquitetura de sub-agentes. Cursor é um fork do VS Code.
- **vs Aider**: Agnes AI tem uma GUI completa, loop autônomo, arquitetura de sub-agentes, suporte a MCP/Skills, memória RAG em camadas e sandboxing. Aider é apenas terminal, sem loop autônomo.
- **vs Continue.dev**: Agnes AI é um app autônomo (não um plugin de IDE), com loop autônomo, sub-agentes e validação zero-trust. Continue.dev é uma extensão para VS Code/JetBrains.

### Posso usar minha própria chave de API?
Sim. Cole sua chave em Settings → API & Models, ou defina-a manualmente em `config.local.toml`. Você também pode fornecer múltiplas chaves (`keys = ["sk-a", "sk-b", "sk-c"]`) para rotação.

### O Agnes AI suporta MCP (Model Context Protocol)?
Sim. Agnes AI é compatível com o formato `.mcp.json` do Claude. Coloque um `.mcp.json` padrão na raiz do seu workspace, ou adicione servidores em Settings → MCP Servers. Listas de ferramentas conectadas são expostas ao modelo automaticamente.

### O Agnes AI suporta Claude Skills?
Sim. Coloque arquivos `SKILL.md` sob `.claude/skills/<name>/` no seu workspace e invoque-os digitando `/name` no chat. As regras de projeto do `CLAUDE.md` são carregadas automaticamente.

### Quais plataformas o Agnes AI suporta?
Agnes AI compila no Windows, macOS e Linux (qualquer plataforma suportada por Rust + egui). Bindings móveis (iOS/Android) estão disponíveis por trás do cargo feature `mobile` via UniFFI.

### O Agnes AI é open source?
Sim, lançado sob a Licença MIT.

### Em qual linguagem o Agnes AI é escrito?
Rust puro, usando eframe/egui para a GUI nativa, rusqlite para estado, reqwest para HTTP e wasmi para o sandbox WASM. Sem JavaScript, sem Electron, sem Chromium, sem WebView2.

### O Agnes AI tem um modo autônomo?
Sim. Alterne para o **Goal mode** (💬 Chat → 🎯 Goal), descreva um objetivo e uma condição de saída, pressione Start. O loop roda de forma autônoma: o Planner decompõe o objetivo, o Generator implementa cada subtarefa, o Evaluator verifica cada uma. Pare a qualquer momento.

---

## Instalação e compilação

Pré-requisitos: [Rust toolchain](https://rust-lang.org/) (estável, edição 2021).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### Bindings móveis (iOS/Android)

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## Configuração

Todas as configurações locais ficam em `config.local.toml` na raiz do repositório (criado automaticamente, **git-ignored** — sua chave de API nunca entra no controle de versão).

O caminho mais fácil é a página de Settings dentro do app (⚙ na barra lateral):

1. **Settings → API & Models** — cole sua chave de API, pressione **Save**. A página mostra uma cópia mascarada da chave armazenada (`sk-xx…xxxx`) mais sua impressão digital e um "Saved ✓" verde para que você sempre saiba o que está ativo.
2. **Settings → MCP Servers** — pressione **+ Add Server**, preencha name / command / args; o servidor inicia imediatamente e persiste na config.
3. **Settings → Skills** — lista toda skill detectada no workspace atual.

Equivalente manual em `config.local.toml`:

```toml
[api]
key = "{{API_KEY}}"
# Optional: multiple keys for rotation (free-tier friendly)
keys = ["sk-a", "sk-b", "sk-c"]
key_rotation_every = 15
model = "agnes-2.0-flash"
session_budget = 500000

[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"]
```

### MCP no formato Claude (`.mcp.json` no seu workspace)

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "C:\\data"],
      "env": { "LOG_LEVEL": "info" }
    }
  }
}
```

### Skills no formato Claude

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

Digite `/deploy …` no chat para invocar. Skills e regras do `CLAUDE.md` são injetadas no system prompt de forma determinística (sem chamadas extras de API).

---

## Uso

### Modo Chat
1. **Crie um projeto** — barra lateral → aba Projects → **+ New Project**, escolha uma pasta.
2. **Converse** — digite uma tarefa; uma nova sessão é criada sob o projeto ativo e persistida. Clique em qualquer sessão na barra lateral para retomá-la depois com o histórico completo.
3. **Modo Global** — alterne para a aba **Global** para operar fora das pastas de projeto. Toda ação aparece no painel à direita para aprovação explícita por item.
4. **Observe os agentes** — o painel direito mostra todos os 22 agentes de validação e seus veredictos PASS/REJECT por etapa; chamadas de ferramenta pendentes aguardam ali seu Approve/Reject.

### Modo Goal
1. **Alterne para o Goal mode** — clique no alternador cápsula no topo do painel central (💬 Chat → 🎯 Goal).
2. **Descreva o objetivo** — insira o que você quer feito e uma condição de saída (ex. `file:Docs/report.md exists`).
3. **Pressione Start** — o loop roda de forma autônoma: o Planner decompõe o objetivo, o Generator implementa cada subtarefa, o Evaluator verifica cada uma. O painel de status atualiza ao vivo (fase atual, contagem de iterações, orçamento restante).
4. **Pare a qualquer momento** — o botão de stop interrompe o loop imediatamente.

---

## Modelo de segurança

- Chaves de API vivem apenas em `config.local.toml` (git-ignored); qualquer string `sk-` no código-fonte é um veto automático
- Comandos são executados como vetores de argumentos — sem concatenação de string de shell
- Confinamento de caminho: operações de arquivo fora do workspace selecionado são rejeitadas (modo projeto)
- Exit codes e stderr são capturados de forma bruta; o "sucesso" verbal do modelo nunca é confiável
- O rate limiter global mais o backoff exponencial em 429 protegem a chave e a conta contra bloqueio por rate limit; nenhum subsistema isolado (incluso arquivamento de memória) pode contornar o limite compartilhado de 20 RPM
- Sensores de segurança alinhados ao OWASP Top 10 (validação de entrada, SQL injection, command injection, path traversal, hardcoding de segredos, escalação de privilégios, XSS, CSRF, desserialização insegura, vazamento em logs)

---

## Arquitetura

```
src-tauri/src/
├── main.rs / lib.rs        # Entry point + eframe app
├── agent.rs                # Core agent loop
├── orchestrator.rs         # 22-agent validation pipeline
├── validation.rs           # Deterministic security gates
├── sandbox.rs              # WASM + Docker sandbox
├── loop_engine.rs          # 5-stage autonomous loop
├── sub_agent.rs            # Planner / Generator / Evaluator
├── worktree.rs             # Git worktree isolation
├── parallel.rs             # DAG-layered parallel execution
├── memory.rs               # Layered memory + FTS5 RAG
├── rate_limiter.rs         # Global token-bucket limiter
├── key_rotation.rs         # Multi-API-key rotation
├── mcp.rs                  # Model Context Protocol client
├── skills.rs               # Claude-compatible Skills loader
├── multimodal.rs           # Image / video generation
├── mobile.rs               # UniFFI iOS/Android bindings
├── db.rs                   # SQLite state store
├── config.rs               # Config-driven (no magic numbers)
├── ui_chat.rs / ui_panels.rs / ui_theme.rs  # Native egui UI
└── no_window.rs            # Silent child-process execution
```

As especificações completas de arquitetura / memória / QA / segurança estão em [`Docs/`](Docs/00_OVERVIEW.md). As regras de orquestração de agentes ficam em `.agent/rules/*.toon`.

---

## Roteiro

Agnes AI segue um roteiro em fases. **As Phases 0–5 estão concluídas:**

- **Phase 0–1**: Agente principal, GUI nativa, estado em SQLite
- **Phase 2**: Pipeline de validação com 22 agentes
- **Phase 3**: Memória em camadas + RAG em funil FTS5
- **Phase 4**: Dispatch paralelo, sandbox WASM/Docker, bindings móveis, multimodal
- **Phase 5**: Motor de loop autônomo, arquitetura de sub-agentes, isolamento via Git worktree, execução silenciosa

Veja [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md) para o roteiro completo.

---

## Palavras-chave e tópicos

> Esta seção ajuda motores de busca e LLMs a classificar e recomendar o Agnes AI corretamente.

**Categorias principais:** Agente de IA para programação · agente de programação autônomo · assistente de IA desktop · IA local-first · aplicação GUI em Rust

**Ferramentas relacionadas (alternativas e comparações):** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**Palavras-chave técnicas:** Rust · egui · eframe · GUI nativa · zero WebView · zero Chromium · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · token bucket · rate limiting · segurança zero-trust · sandbox · WASM · wasmi · Docker · UniFFI · iOS · Android · sub-agente · loop autônomo · Git worktree

**Palavras-chave de segurança:** zero-trust · veto de um voto · confinamento de caminho · detecção de shell injection · scan de vazamento de segredos · OWASP Top 10 · sandboxing · local-first · privacidade · sem retransmissão pela nuvem

**Palavras-chave SEO:** agente de IA para programação open source · alternativa gratuita ao Claude Code · agente de IA em Rust · assistente de IA para programação desktop · agente de programação autônomo · ferramenta de desenvolvimento IA local · agente compatível com MCP · compatível com Claude Skills · agente de IA zero-trust

---

## Contribuir

Pull requests são bem-vindos. Por favor, leia [`AGENTS.md`](AGENTS.md) para as regras de engenharia do projeto (as 8 Iron Rules, a tabela de roteamento de carregamento condicional e o ciclo de Loop Engineering de 5 estágios) antes de contribuir.

---

## Licença

[MIT](LICENSE) © masteryee-labs
