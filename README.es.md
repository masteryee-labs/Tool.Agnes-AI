# Agnes AI — Agente de IA para programación en Rust de código abierto (escritorio)

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#instalación-y-compilación)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#skills-compatibles-con-claude)

---

## ¿Qué es Agnes AI?

**Agnes AI es un agente de IA para programación de escritorio, de código abierto, escrito en Rust puro con una GUI nativa en egui (cero Chromium / WebView2).** Ejecuta un **pipeline de validación zero-trust con 22 agentes** para que cada acción propuesta por el modelo de lenguaje sea verificada por puertas de seguridad deterministas antes de tocar tu sistema de archivos, shell o red. También incluye un **bucle autónomo guiado por objetivos** (Discover → Plan → Execute → Verify → Iterate) con una **arquitectura de sub-agentes** (Planner / Generator / Evaluator) y **aislamiento con Git worktree** para ejecución paralela segura.

Agnes AI es una **alternativa gratuita y local-first a Claude Code, Cursor, Aider y Continue.dev** — tu API key y tu código nunca salen de tu máquina, el binario es diminuto, el arranque es instantáneo y la interfaz es una app nativa oscura y minimalista (sin navegador embebido, sin Electron).

> **En una línea:** Un agente de IA de escritorio nativo en Rust, de alta defensa y alta velocidad, que nunca confía en el "funcionó" verbal del modelo — solo Exit Code 0 y stderr vacío cuentan como éxito.

---

## ¿Por qué Agnes AI? (vs Claude Code / Cursor / Aider / Continue.dev)

| Característica | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **Runtime** | GUI nativa en Rust (egui) | Terminal | IDE basado en Electron | Terminal | Plugin de VS Code/JetBrains |
| **Tamaño del binario** | Pequeño (~MB) | Mediano | Grande (~100 MB+) | Pequeño | Depende del IDE anfitrión |
| **Navegador embebido** | Ninguno (cero WebView2) | Ninguno | Chromium | Ninguno | El del IDE anfitrión |
| **Modelo de seguridad** | Pipeline de 22 agentes zero-trust, veto con un voto | Limitado | Limitado | Mínimo | Mínimo |
| **Bucle autónomo** | Sí (5 etapas, guiado por objetivos) | Sí (modo agente) | No | No | No |
| **Arquitectura de sub-agentes** | Sí (Planner/Generator/Evaluator) | Sí | No | No | No |
| **Aislamiento con Git worktree** | Sí (sub-agentes en paralelo) | No | No | No | No |
| **Soporte MCP** | Sí (formato `.mcp.json` de Claude) | Sí | Parcial | No | No |
| **Claude Skills** | Sí (`.claude/skills/`) | Sí | No | No | No |
| **Memoria RAG local** | Sí (FTS5 + embudo de 3 etapas) | Limitado | Limitado | No | Limitado |
| **Memoria entre sesiones** | Sí (lecciones/pitfalls/estado del bucle) | No | No | No | No |
| **Sandbox WASM / Docker** | Sí | No | No | No | No |
| **Bindings para móvil** | Sí (UniFFI, iOS/Android) | No | No | No | No |
| **Multimodal (imagen/video)** | Sí | Sí | Sí | No | No |
| **Rotación de múltiples API keys** | Sí (compatible con free-tier) | No | No | No | No |
| **Código abierto** | Sí (MIT) | No | No | Sí | Sí |
| **Precio** | Gratis (trae tu propia key) | De pago | De pago | Gratis (BYO key) | Gratis/De pago |

**Agnes AI es ideal para desarrolladores que buscan:**
- Un **agente de IA para programación local-first que respeta la privacidad** (sin reenvío a la nube de tu código)
- **Garantías de seguridad robustas** (validación zero-trust, sandboxing, veto de fuga de secretos)
- Una **app de escritorio nativa y ligera** en lugar de Electron o una terminal
- **Ejecución autónoma guiada por objetivos** con criterios de éxito verificables
- **Sostenibilidad del free-tier** mediante rotación de múltiples keys y protección contra rate limits

---

## Funciones principales

### Experiencia principal
- **GUI nativa en Rust** — eframe/egui + wgpu, sin navegador embebido, arranque instantáneo, huella mínima
- **Interfaz oscura minimalista** — paleta pura negra + blanca inspirada en Claude Code / Codex / Devin / Antigravity 2.0; sin colores de marca que distraigan
- **Ejecución silenciosa** — todos los procesos hijos (comandos de shell, compilador, git, servidores MCP) se ejecutan con `CREATE_NO_WINDOW` en Windows; no aparecen ventanas de CMD/PowerShell en tu escritorio

### Espacios de trabajo
- **Modo dual Proyecto / Global** — las pestañas de la barra lateral cambian entre:
  - **Proyectos**: crea un proyecto desde cualquier carpeta; cada sesión de chat se anida bajo su proyecto; las conversaciones se guardan en SQLite y se reanudan exactamente donde las dejaste
  - **Global**: una pestaña dedicada para operación en toda la computadora, donde cada acción requiere confirmación individual

### Bucle autónomo (Phase 5)
- **Bucle guiado por objetivos** — dale un objetivo y una condición de salida; ejecuta Discover → Plan → Execute → Verify → Iterate por sí solo hasta que se cumpla la condición o se alcance el límite de iteraciones
- **Arquitectura de sub-agentes** — tres roles independientes con prompts y estado de conversación separados:
  - **Planner** — descompone el objetivo en subtareas atómicas
  - **Generator** — implementa una subtarea por ejecución, llamando a las herramientas `write_file` / `run_command`
  - **Evaluator** — verifica la salida del Generator de forma independiente; rechaza afirmaciones de "éxito" puramente verbales
- **Aislamiento con Git worktree** — cada sub-agente Generator trabaja en un git worktree + branch aislado; los sub-agentes en paralelo nunca pisan los archivos de otros; el trabajo completado se fusiona de vuelta al branch principal
- **Memoria entre sesiones** — lecciones, pitfalls y estado del bucle se guardan en `.agent/memory/` para que el agente continúe donde lo dejó entre sesiones

### Seguridad y validación
- **Pipeline de validación con 22 agentes** — cada llamada a herramientas del modelo es verificada por puertas deterministas (confinamiento de rutas, detección de shell-injection, escaneo de fugas de secretos, auditoría de AI-slop, …) con veto de un solo voto
- **Alineación con sandbox** — los archivos `.rs` escritos se compilan (y sus tests se ejecutan) inmediatamente; "afirma éxito pero no compila" se rechaza al instante
- **Sandbox WASM** — código no confiable se ejecuta a través del intérprete puro en Rust `wasmi` con un linker vacío (sin host imports → sin I/O/syscalls/red) y medición de combustible (fuel metering)
- **Sandbox Docker** — tareas a nivel de compilación se ejecutan en un contenedor con `--network=none`, `--rm`, workspace montado en `/work`; argumentos vectorizados (sin shell)
- **Cero confianza verbal** — Exit Code == 0 y stderr vacío son la única definición de éxito; el "funcionó" verbal del modelo nunca se confía

### Compatibilidad
- **Skills compatibles con Claude** — coloca archivos `SKILL.md` bajo `.claude/skills/<name>/` en tu workspace; invócalos escribiendo `/name` en el chat. Las reglas de proyecto de `CLAUDE.md` se cargan automáticamente
- **MCP compatible con Claude** — pon un `.mcp.json` estándar en la raíz de tu workspace, o añade servidores en Settings → MCP Servers; las listas de herramientas conectadas se exponen al modelo automáticamente

### Rendimiento
- **Memoria por capas** — fragmentación por ventana deslizante + RAG de embudo de 3 etapas sobre un índice FTS5, con marcas de agua de destilación para evitar re-quemar tokens
- **Rate limiting y protección 20 RPM** — un limitador global compartido de token-bucket regula cada llamada a la API (destilación y recuperación incluidas); `acquire()` espera el refill en lugar de rechazar, así las ráfagas nunca rompen el límite de 20 peticiones/minuto del free-tier. Ante un 429 el cliente aplica exponential backoff basado en multiplicador. Cada parámetro es configurable (`max_rpm`, ajustes de retry backoff) — sin números mágicos
- **Rotación de múltiples API keys** — rota entre múltiples keys de cuenta (basado en conteo + cambio forzado ante HTTP 420/429) para mantenerte totalmente gratis sin alcanzar el rate limit de ninguna cuenta individual
- **Economía de tokens** — presupuesto de tokens por sesión con bloqueo estricto, medidor de presupuesto en vivo en la barra de título. El conteo de peticiones se reduce por diseño: la Stage 0 hace una búsqueda local en memoria FTS5 que en caso de acierto omite la llamada a la API de recuperación (0 llamadas API), y las Stage 1+2 del RAG de embudo se fusionaron en una sola llamada (2 llamadas → 1)

---

## Preguntas frecuentes (FAQ)

### ¿Agnes AI es gratis?
Sí. Agnes AI es de código abierto (MIT) y gratis. Tú traes tu propia API key (p. ej. una key compatible con Agnes / OpenAI). La función de rotación de múltiples keys te permite combinar varias cuentas del free-tier para evitar los rate limits por completo.

### ¿Agnes AI envía mi código a la nube?
Agnes AI se ejecuta 100% localmente. Tu código nunca se reenvía a través de ningún servidor de Agnes AI. El único tráfico de red son las llamadas directas a la API que configuras hacia tu proveedor de LLM (lo cual es necesario para cualquier agente basado en LLM). Tu API key se guarda en `config.local.toml` (git-ignored) y nunca entra al control de versiones ni al contexto del modelo.

### ¿En qué se diferencia Agnes AI de Claude Code / Cursor / Aider?
- **vs Claude Code**: Agnes AI es de código abierto, tiene una GUI nativa (no solo terminal), añade un pipeline de validación zero-trust con 22 agentes, aislamiento con Git worktree para sub-agentes en paralelo, y sandboxing WASM/Docker.
- **vs Cursor**: Agnes AI es una app nativa independiente (sin Electron/Chromium), de código abierto, con un bucle autónomo guiado por objetivos y arquitectura de sub-agentes. Cursor es un fork de VS Code.
- **vs Aider**: Agnes AI tiene una GUI completa, bucle autónomo, arquitectura de sub-agentes, soporte MCP/Skills, memoria RAG por capas y sandboxing. Aider es solo terminal sin bucle autónomo.
- **vs Continue.dev**: Agnes AI es una app independiente (no un plugin de IDE), con bucle autónomo, sub-agentes y validación zero-trust. Continue.dev es una extensión de VS Code/JetBrains.

### ¿Puedo usar mi propia API key?
Sí. Pega tu key en Settings → API & Models, o establécela manualmente en `config.local.toml`. También puedes proporcionar múltiples keys (`keys = ["sk-a", "sk-b", "sk-c"]`) para rotación.

### ¿Agnes AI soporta MCP (Model Context Protocol)?
Sí. Agnes AI es compatible con el formato `.mcp.json` de Claude. Pon un `.mcp.json` estándar en la raíz de tu workspace, o añade servidores en Settings → MCP Servers. Las listas de herramientas conectadas se exponen al modelo automáticamente.

### ¿Agnes AI soporta Claude Skills?
Sí. Coloca archivos `SKILL.md` bajo `.claude/skills/<name>/` en tu workspace e invócalos escribiendo `/name` en el chat. Las reglas de proyecto de `CLAUDE.md` se cargan automáticamente.

### ¿Qué plataformas soporta Agnes AI?
Agnes AI compila en Windows, macOS y Linux (cualquier plataforma que soporten Rust + egui). Los bindings para móvil (iOS/Android) están disponibles con el cargo feature `mobile` vía UniFFI.

### ¿Agnes AI es de código abierto?
Sí, publicado bajo la MIT License.

### ¿En qué lenguaje está escrito Agnes AI?
Rust puro, usando eframe/egui para la GUI nativa, rusqlite para el estado, reqwest para HTTP y wasmi para el sandbox WASM. Sin JavaScript, sin Electron, sin Chromium, sin WebView2.

### ¿Agnes AI tiene un modo autónomo?
Sí. Cambia a **Goal mode** (💬 Chat → 🎯 Goal), describe un objetivo y una condición de salida, presiona Start. El bucle se ejecuta de forma autónoma: el Planner descompone el objetivo, el Generator implementa cada subtarea, el Evaluator verifica cada una. Detén en cualquier momento.

---

## Instalación y compilación

Prerrequisitos: [Rust toolchain](https://rust-lang.org/) (stable, edición 2021).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### Bindings para móvil (iOS/Android)

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## Configuración

Toda la configuración local vive en `config.local.toml` en la raíz del repo (auto-creado, **git-ignored** — tu API key nunca entra al control de versiones).

El camino más fácil es la página de Settings dentro de la app (⚙ en la barra lateral):

1. **Settings → API & Models** — pega tu API key, presiona **Save**. La página muestra una copia enmascarada de la key almacenada (`sk-xx…xxxx`) más su huella y un "Saved ✓" verde para que siempre sepas qué hay activo.
2. **Settings → MCP Servers** — presiona **+ Add Server**, completa nombre / comando / args; el servidor arranca inmediatamente y se guarda en la configuración.
3. **Settings → Skills** — lista cada skill detectado en el workspace actual.

Equivalente manual en `config.local.toml`:

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

### MCP en formato Claude (`.mcp.json` en tu workspace)

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

### Skills en formato Claude

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

Escribe `/deploy …` en el chat para invocar. Los Skills y las reglas de `CLAUDE.md` se inyectan en el system prompt de forma determinista (sin llamadas extra a la API).

---

## Uso

### Modo chat
1. **Crea un proyecto** — barra lateral → pestaña Projects → **+ New Project**, elige una carpeta.
2. **Chatea** — escribe una tarea; se crea una nueva sesión bajo el proyecto activo y se persiste. Haz clic en cualquier sesión de la barra lateral para reanudarla después con todo el historial.
3. **Modo Global** — cambia a la pestaña **Global** para operar fuera de las carpetas de proyecto. Cada acción aparece en el panel derecho para aprobación individual explícita.
4. **Observa los agentes** — el panel derecho muestra los 22 agentes de validación y sus veredictos PASS/REJECT por paso; las llamadas a herramientas pendientes esperan ahí tu Approve/Reject.

### Modo objetivo
1. **Cambia a Goal mode** — haz clic en el toggle de cápsula en la parte superior del panel central (💬 Chat → 🎯 Goal).
2. **Describe el objetivo** — ingresa qué quieres que se haga y una condición de salida (p. ej. `file:Docs/report.md exists`).
3. **Presiona Start** — el bucle se ejecuta de forma autónoma: el Planner descompone el objetivo, el Generator implementa cada subtarea, el Evaluator verifica cada una. El panel de estado se actualiza en vivo (fase actual, conteo de iteraciones, presupuesto restante).
4. **Detén en cualquier momento** — el botón de stop detiene el bucle inmediatamente.

---

## Modelo de seguridad

- Las API keys viven solo en `config.local.toml` (git-ignored); cualquier string `sk-` en el código fuente es un veto automático
- Los comandos se ejecutan como vectores de argumentos — sin concatenación de strings de shell
- Confinamiento de rutas: operaciones de archivo fuera del workspace seleccionado se rechazan (modo proyecto)
- Los exit codes y stderr se capturan en crudo; el "éxito" verbal del modelo nunca se confía
- El rate limiter global más el exponential backoff ante 429 protegen la key y la cuenta de bloqueos por rate limit; ningún subsistema individual (incluida la memoria de archivo) puede saltarse el límite compartido de 20 RPM
- Sensores de seguridad alineados con OWASP Top 10 (validación de entrada, SQL injection, command injection, path traversal, secret hardcoding, privilege escalation, XSS, CSRF, deserialización insegura, fuga de logs)

---

## Arquitectura

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

Las especificaciones completas de arquitectura / memoria / QA / seguridad están en [`Docs/`](Docs/00_OVERVIEW.md). Las reglas de orquestación de agentes viven en `.agent/rules/*.toon`.

---

## Hoja de ruta

Agnes AI sigue una hoja de ruta por fases. **Phase 0–5 están completas:**

- **Phase 0–1**: Agente principal, GUI nativa, estado en SQLite
- **Phase 2**: Pipeline de validación con 22 agentes
- **Phase 3**: Memoria por capas + RAG de embudo FTS5
- **Phase 4**: Dispatch paralelo, sandbox WASM/Docker, bindings para móvil, multimodal
- **Phase 5**: Motor de bucle autónomo, arquitectura de sub-agentes, aislamiento con Git worktree, ejecución silenciosa

Consulta [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md) para la hoja de ruta completa.

---

## Palabras clave y temas

> Esta sección ayuda a los buscadores y LLMs a clasificar y recomendar Agnes AI correctamente.

**Categorías principales:** agente de IA para programación · agente de programación autónomo · asistente de IA de escritorio · IA local-first · aplicación GUI en Rust

**Herramientas relacionadas (alternativas y comparaciones):** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**Palabras clave técnicas:** Rust · egui · eframe · native GUI · zero WebView · zero Chromium · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · token bucket · rate limiting · zero-trust security · sandbox · WASM · wasmi · Docker · UniFFI · iOS · Android · sub-agent · autonomous loop · Git worktree

**Palabras clave de seguridad:** zero-trust · one-vote veto · path confinement · shell injection detection · secret leak scan · OWASP Top 10 · sandboxing · local-first · privacy · no cloud relay

**Palabras clave SEO:** open source AI coding agent · free Claude Code alternative · Rust AI agent · desktop AI coding assistant · autonomous coding agent · local AI developer tool · MCP compatible agent · Claude Skills compatible · zero-trust AI agent · agente de IA para programación de código abierto · alternativa gratuita a Claude Code · agente de IA en Rust · asistente de IA para programación de escritorio · agente de programación autónomo · herramienta de desarrollo IA local · agente compatible con MCP

---

## Contribuir

Los pull requests son bienvenidos. Por favor lee [`AGENTS.md`](AGENTS.md) para conocer las reglas de ingeniería del proyecto (las 8 Iron Rules, la tabla de rutas de conditional-loading y el ciclo de 5 etapas de Loop Engineering) antes de contribuir.

---

## Licencia

[MIT](LICENSE) © masteryee-labs
