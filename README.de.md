# Agnes AI — Open-Source Rust Desktop AI Coding Agent

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#installation--build)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-kompatible-skills)

---

## Was ist Agnes AI?

**Agnes AI ist ein Open-Source-Desktop-AI-Coding-Agent, geschrieben in purem Rust mit einer nativen egui-GUI (null Chromium / WebView2).** Er betreibt eine **22-Agenten-Null-Trust-Validierungspipeline**, sodass jede vom Sprachmodell vorgeschlagene Aktion durch deterministische Sicherheitsgates quergeprüft wird, bevor sie Ihr Dateisystem, Ihre Shell oder Ihr Netzwerk berührt. Zudem liefert er eine **autonome zielgesteuerte Schleife** (Discover → Plan → Execute → Verify → Iterate) mit einer **Sub-Agenten-Architektur** (Planner / Generator / Evaluator) und **Git-Worktree-Isolation** für sichere parallele Ausführung.

Agnes AI ist eine **kostenlose, lokale Alternative zu Claude Code, Cursor, Aider und Continue.dev** — Ihr API-Schlüssel und Ihr Code verlassen niemals Ihren Rechner, die Binärdatei ist winzig, der Start erfolgt augenblicklich, und die UI ist eine minimalistische native Dark-App (kein eingebetteter Browser, kein Electron).

> **In einem Satz:** Ein hochabwehrfähiger, hochgeschwindigkeits-Desktop-AI-Agent in nativem Rust, der dem verbalen „es hat funktioniert" des Modells niemals vertraut — nur Exit Code 0 und eine leere stderr gelten als Erfolg.

---

## Warum Agnes AI? (vs Claude Code / Cursor / Aider / Continue.dev)

| Funktion | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **Laufzeitumgebung** | Native Rust-GUI (egui) | Terminal | Electron-basierte IDE | Terminal | VS Code/JetBrains-Plugin |
| **Binärgröße** | Winzig (~MB) | Mittel | Groß (~100 MB+) | Winzig | Abhängig von Host-IDE |
| **Eingebetteter Browser** | Keiner (null WebView2) | Keiner | Chromium | Keiner | Host-IDE |
| **Sicherheitsmodell** | 22-Agenten-Null-Trust-Pipeline, Ein-Stimmen-Veto | Begrenzt | Begrenzt | Minimal | Minimal |
| **Autonome Schleife** | Ja (5-stufig, zielgesteuert) | Ja (Agent-Modus) | Nein | Nein | Nein |
| **Sub-Agenten-Architektur** | Ja (Planner/Generator/Evaluator) | Ja | Nein | Nein | Nein |
| **Git-Worktree-Isolation** | Ja (parallele Sub-Agenten) | Nein | Nein | Nein | Nein |
| **MCP-Unterstützung** | Ja (Claude `.mcp.json`-Format) | Ja | Teilweise | Nein | Nein |
| **Claude Skills** | Ja (`.claude/skills/`) | Ja | Nein | Nein | Nein |
| **Lokales RAG-Gedächtnis** | Ja (FTS5 + 3-stufiger Trichter) | Begrenzt | Begrenzt | Nein | Begrenzt |
| **Sitzungsübergreifendes Gedächtnis** | Ja (Lessons/Pitfalls/Schleifenzustand) | Nein | Nein | Nein | Nein |
| **WASM-/Docker-Sandbox** | Ja | Nein | Nein | Nein | Nein |
| **Mobile Bindings** | Ja (UniFFI, iOS/Android) | Nein | Nein | Nein | Nein |
| **Multimodal (Bild/Video)** | Ja | Ja | Ja | Nein | Nein |
| **Multi-API-Key-Rotation** | Ja (Free-Tier-freundlich) | Nein | Nein | Nein | Nein |
| **Open Source** | Ja (MIT) | Nein | Nein | Ja | Ja |
| **Preis** | Kostenlos (eigener Key) | Kostenpflichtig | Kostenpflichtig | Kostenlos (BYO Key) | Kostenlos/Kostenpflichtig |

**Agnes AI eignet sich am besten für Entwickler, die Folgendes wünschen:**
- Einen **lokalen, datenschutzrespektierenden** AI-Coding-Agenten (keine Cloud-Weiterleitung Ihres Codes)
- **Starke Sicherheitsgarantien** (Null-Trust-Validierung, Sandboxing, Secret-Leak-Veto)
- Eine **native, ressourcenschonende Desktop-App** statt Electron oder einem Terminal
- **Autonome zielgesteuerte Ausführung** mit überprüfbaren Erfolgskriterien
- **Free-Tier-Nachhaltigkeit** durch Multi-Key-Rotation und Ratenbegrenzungsschutz

---

## Hauptfunktionen

### Kern-Erlebnis
- **Native Rust-GUI** — eframe/egui + wgpu, kein eingebetteter Browser, sofortiger Start, minimaler Fußabdruck
- **Minimalistische Dark-UI** — rein schwarze + weiße Palette, inspiriert von Claude Code / Codex / Devin / Antigravity 2.0; keine ablenkenden Markenfarben
- **Stille Ausführung** — alle Kind-Prozesse (Shell-Befehle, Compiler, git, MCP-Server) laufen unter Windows mit `CREATE_NO_WINDOW`; auf Ihrem Desktop erscheinen keine CMD-/PowerShell-Fenster

### Workspaces
- **Projekt-/Globaler Dual-Modus** — Seitenleisten-Tabs wechseln zwischen:
  - **Projekte**: Erstellen Sie ein Projekt aus einem beliebigen Ordner; jede Chat-Sitzung ist ihrem Projekt untergeordnet; Konversationen werden in SQLite gespeichert und genau dort fortgesetzt, wo Sie aufgehört haben
  - **Global**: Ein eigener Tab für den computerweiten Betrieb, bei dem jede Aktion eine Bestätigung pro Element erfordert

### Autonome Schleife (Phase 5)
- **Zielgesteuerte Schleife** — geben Sie ein Ziel und eine Abbruchbedingung an; es führt Discover → Plan → Execute → Verify → Iterate selbstständig aus, bis die Bedingung erfüllt oder die Iterationsgrenze erreicht ist
- **Sub-Agenten-Architektur** — drei unabhängige Rollen mit separaten Prompts und Konversationszustand:
  - **Planner** — zerlegt das Ziel in atomare Teilaufgaben
  - **Generator** — implementiert eine Teilaufgabe pro Durchlauf und ruft die Werkzeuge `write_file` / `run_command` auf
  - **Evaluator** — verifiziert die Ausgabe des Generators unabhängig; lehnt verbale „Erfolgs"-Behauptungen ab
- **Git-Worktree-Isolation** — jeder Generator-Sub-Agent arbeitet in einem isolierten git-Worktree + Branch; parallele Sub-Agenten treten sich niemals auf die Dateien des anderen; abgeschlossene Arbeit wird zurück in den Haupt-Branch gemergt
- **Sitzungsübergreifendes Gedächtnis** — Lessons, Pitfalls und Schleifenzustand werden in `.agent/memory/` gespeichert, sodass der Agent über Sitzungen hinweg dort ansetzt, wo er aufgehört hat

### Sicherheit & Validierung
- **22-Agenten-Validierungspipeline** — jeder Werkzeugaufruf des Modells wird durch deterministische Gates quergeprüft (Pfadbegrenzung, Shell-Injection-Erkennung, Secret-Leak-Scan, AI-Slop-Audit, …) mit Ein-Stimmen-Veto
- **Sandbox-Abgleich** — geschriebene `.rs`-Dateien werden sofort kompiliert (und ihre Tests ausgeführt); „behauptet Erfolg, kompiliert aber nicht" wird sofort abgelehnt
- **WASM-Sandbox** — nicht vertrauenswürdiger Code läuft über den `wasmi` pure-Rust-Interpreter mit leerem Linker (keine Host-Imports → kein I/O/Syscalls/Netzwerk) und Fuel-Metering
- **Docker-Sandbox** — Aufgaben auf Kompilierungsebene laufen in einem Container mit `--network=none`, `--rm`, Workspace gemountet unter `/work`; vektorisierte Argumente (keine Shell)
- **Kein verbales Vertrauen** — Exit Code == 0 und leere stderr sind die einzige Definition von Erfolg; das verbale „es hat funktioniert" des Modells wird niemals vertraut

### Kompatibilität
- **Claude-kompatible Skills** — legen Sie `SKILL.md`-Dateien unter `.claude/skills/<name>/` in Ihrem Workspace ab; rufen Sie sie auf, indem Sie `/name` im Chat eingeben. `CLAUDE.md`-Projektregeln werden automatisch geladen
- **Claude-kompatibles MCP** — legen Sie eine Standard-`.mcp.json` in Ihren Workspace-Root oder fügen Sie Server unter Einstellungen → MCP-Server hinzu; verbundene Werkzeuglisten werden dem Modell automatisch bereitgestellt

### Leistung
- **Schichtgedächtnis** — Sliding-Window-Chunking + 3-stufiger Trichter-RAG über einem FTS5-Index, mit Destillations-Wasserzeichen zur Vermeidung erneuter Token-Belastung
- **Ratenbegrenzung & 20-RPM-Schutz** — ein globaler, gemeinsam genutzter Token-Bucket-Limiter steuert jeden API-Aufruf (Destillation und Retrieval inbegriffen); `acquire()` wartet auf Auffüllung, anstatt abzulehnen, sodass Bursts niemals das 20-Anfragen/Minute-Free-Tier-Limit überschreiten. Bei einer 429 wendet der Client multiplikatorbasiertes exponentielles Backoff an. Jeder Parameter ist konfigurationsgesteuert (`max_rpm`, Retry-Backoff-Einstellungen) — keine Magic Numbers
- **Multi-API-Key-Rotation** — rotieren Sie über mehrere Kontoschlüssel (zählerbasiert + erzwungener Wechsel bei HTTP 420/429), um vollständig kostenlos zu bleiben, ohne das Ratenlimit eines einzelnen Kontos zu erreichen
- **Token-Ökonomie** — pro-Sitzung-Token-Budget mit harter Sperre, Live-Budget-Anzeige in der Titelleiste. Die Anzahl der Anfragen ist bewusst reduziert: Stufe 0 führt ein lokales FTS5-Gedächtnis-Lookup durch, das bei einem Treffer den Retrieval-API-Aufruf vollständig überspringt (0 API-Aufrufe), und Stufe 1+2 des Trichter-RAG wurden zu einem einzigen Aufruf zusammengefasst (2 Aufrufe → 1)

---

## Häufig gestellte Fragen (FAQ)

### Ist Agnes AI kostenlos?
Ja. Agnes AI ist Open-Source (MIT) und kostenlos. Sie bringen Ihren eigenen API-Schlüssel mit (z. B. einen Agnes-/OpenAI-kompatiblen Key). Die Multi-Key-Rotation-Funktion ermöglicht es, mehrere Free-Tier-Konten zu kombinieren, um Ratenlimits vollständig zu vermeiden.

### Sendet Agnes AI meinen Code in die Cloud?
Agnes AI selbst läuft zu 100 % lokal. Ihr Code wird niemals über einen Agnes-AI-Server weitergeleitet. Der einzige Netzwerkverkehr sind die direkten API-Aufrufe, die Sie zu Ihrem LLM-Anbieter konfigurieren (was für jeden LLM-basierten Agenten notwendig ist). Ihr API-Schlüssel bleibt in `config.local.toml` (git-ignored) und gelangt niemals in die Versionskontrolle oder den Kontext des Modells.

### Wie unterscheidet sich Agnes AI von Claude Code / Cursor / Aider?
- **vs Claude Code**: Agnes AI ist Open-Source, besitzt eine native GUI (nicht nur Terminal), ergänzt eine 22-Agenten-Null-Trust-Validierungspipeline, Git-Worktree-Isolation für parallele Sub-Agenten sowie WASM-/Docker-Sandboxing.
- **vs Cursor**: Agnes AI ist eine eigenständige native App (kein Electron/Chromium), Open-Source, mit einer autonomen zielgesteuerten Schleife und Sub-Agenten-Architektur. Cursor ist ein Fork von VS Code.
- **vs Aider**: Agnes AI besitzt eine vollständige GUI, autonome Schleife, Sub-Agenten-Architektur, MCP-/Skills-Unterstützung, geschichtetes RAG-Gedächtnis und Sandboxing. Aider ist nur Terminal-basiert ohne autonome Schleife.
- **vs Continue.dev**: Agnes AI ist eine eigenständige App (kein IDE-Plugin) mit autonomer Schleife, Sub-Agenten und Null-Trust-Validierung. Continue.dev ist eine VS-Code-/JetBrains-Erweiterung.

### Kann ich meinen eigenen API-Schlüssel verwenden?
Ja. Fügen Sie Ihren Schlüssel unter Einstellungen → API & Modelle ein oder setzen Sie ihn manuell in `config.local.toml`. Sie können auch mehrere Schlüssel (`keys = ["sk-a", "sk-b", "sk-c"]`) für die Rotation angeben.

### Unterstützt Agnes AI MCP (Model Context Protocol)?
Ja. Agnes AI ist mit dem Claude-`.mcp.json`-Format kompatibel. Legen Sie eine Standard-`.mcp.json` in Ihren Workspace-Root oder fügen Sie Server unter Einstellungen → MCP-Server hinzu. Verbundene Werkzeuglisten werden dem Modell automatisch bereitgestellt.

### Unterstützt Agnes AI Claude Skills?
Ja. Legen Sie `SKILL.md`-Dateien unter `.claude/skills/<name>/` in Ihrem Workspace ab und rufen Sie sie auf, indem Sie `/name` im Chat eingeben. `CLAUDE.md`-Projektregeln werden automatisch geladen.

### Welche Plattformen unterstützt Agnes AI?
Agnes AI lässt sich auf Windows, macOS und Linux builden (jede Plattform, die Rust + egui unterstützen). Mobile (iOS/Android) Bindings sind hinter dem `mobile`-Cargo-Feature über UniFFI verfügbar.

### Ist Agnes AI Open Source?
Ja, veröffentlicht unter der MIT-Lizenz.

### In welcher Sprache ist Agnes AI geschrieben?
Purem Rust, unter Verwendung von eframe/egui für die native GUI, rusqlite für den Zustand, reqwest für HTTP und wasmi für die WASM-Sandbox. Kein JavaScript, kein Electron, kein Chromium, kein WebView2.

### Hat Agnes AI einen autonomen Modus?
Ja. Wechseln Sie in den **Goal-Modus** (💬 Chat → 🎯 Goal), beschreiben Sie ein Ziel und eine Abbruchbedingung, drücken Sie Start. Die Schleife läuft autonom: Planner zerlegt das Ziel, Generator implementiert jede Teilaufgabe, Evaluator verifiziert jede einzelne. Stoppen Sie jederzeit.

---

## Installation & Build

Voraussetzungen: [Rust-Toolchain](https://rust-lang.org/) (stable, 2021-Edition).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### Mobile Bindings (iOS/Android)

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## Konfiguration

Alle lokalen Einstellungen liegen in `config.local.toml` im Repo-Root (auto-erstellt, **git-ignored** — Ihr API-Schlüssel gelangt niemals in die Versionskontrolle).

Der einfachste Weg ist die In-App-Einstellungsseite (⚙ in der Seitenleiste):

1. **Einstellungen → API & Modelle** — fügen Sie Ihren API-Schlüssel ein, drücken Sie **Save**. Die Seite zeigt eine maskierte Kopie des gespeicherten Schlüssels (`sk-xx…xxxx`) samt Fingerprint und einem grünen „Saved ✓", sodass Sie stets wissen, was aktiv ist.
2. **Einstellungen → MCP-Server** — drücken Sie **+ Add Server**, füllen Sie Name / Befehl / Argumente aus; der Server startet sofort und wird in der Konfiguration gespeichert.
3. **Einstellungen → Skills** — listet alle im aktuellen Workspace erkannten Skills auf.

Manuelles Äquivalent in `config.local.toml`:

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

### Claude-format MCP (`.mcp.json` in Ihrem Workspace)

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

### Claude-format Skills

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

Geben Sie `/deploy …` im Chat ein, um aufzurufen. Skills und `CLAUDE.md`-Regeln werden deterministisch in den System-Prompt injiziert (keine zusätzlichen API-Aufrufe).

---

## Verwendung

### Chat-Modus
1. **Projekt erstellen** — Seitenleiste → Projekte-Tab → **+ New Project**, wählen Sie einen Ordner.
2. **Chatten** — geben Sie eine Aufgabe ein; eine neue Sitzung wird unter dem aktiven Projekt erstellt und gespeichert. Klicken Sie auf eine Sitzung in der Seitenleiste, um sie später mit vollem Verlauf fortzusetzen.
3. **Globaler Modus** — wechseln Sie zum **Global**-Tab, um außerhalb von Projektordnern zu operieren. Jede Aktion erscheint im rechten Panel zur expliziten Freigabe pro Element.
4. **Agenten beobachten** — das rechte Panel zeigt alle 22 Validierungs-Agenten und deren PASS/REJECT-Urteile pro Schritt; ausstehende Werkzeugaufrufe warten dort auf Ihre Freigabe/Ablehnung.

### Goal-Modus
1. **In den Goal-Modus wechseln** — klicken Sie auf den Kapsel-Umschalter oben im zentralen Panel (💬 Chat → 🎯 Goal).
2. **Ziel beschreiben** — geben Sie ein, was Sie erledigt haben möchten, und eine Abbruchbedingung (z. B. `file:Docs/report.md exists`).
3. **Start drücken** — die Schleife läuft autonom: Planner zerlegt das Ziel, Generator implementiert jede Teilaufgabe, Evaluator verifiziert jede einzelne. Das Status-Panel aktualisiert sich live (aktuelle Phase, Iterationsanzahl, verbleibendes Budget).
4. **Jederzeit stoppen** — die Stopptaste hält die Schleife sofort an.

---

## Sicherheitsmodell

- API-Schlüssel liegen ausschließlich in `config.local.toml` (git-ignored); jede `sk-`-Zeichenkette im Quellcode führt zu einem automatischen Veto
- Befehle werden als Argumentvektoren ausgeführt — keine Shell-String-Konkatenation
- Pfadbegrenzung: Dateioperationen außerhalb des ausgewählten Workspaces werden abgelehnt (Projekt-Modus)
- Exit-Codes und stderr werden roh erfasst; das verbale „Erfolg" des Modells wird niemals vertraut
- Der globale Ratenbegrenzer plus 429-exponentielles Backoff schützen den Schlüssel und das Konto vor Ratenlimit-Sperren; kein einzelnes Subsystem (Gedächtnis-Archivierung inbegriffen) kann die gemeinsam genutzte 20-RPM-Obergrenze umgehen
- OWASP-Top-10-ausgerichtete Sicherheitssensoren (Eingabevalidierung, SQL-Injection, Befehls-Injection, Path-Traversal, Secret-Hardcoding, Privilege-Escalation, XSS, CSRF, unsichere Deserialisierung, Log-Leakage)

---

## Architektur

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

Die vollständigen Architektur-/Gedächtnis-/QA-/Security-Spezifikationen befinden sich unter [`Docs/`](Docs/00_OVERVIEW.md). Die Agenten-Orchestrierungsregeln liegen in `.agent/rules/*.toon`.

---

## Roadmap

Agnes AI folgt einer phasenbasierten Roadmap. **Phase 0–5 sind abgeschlossen:**

- **Phase 0–1**: Kern-Agent, native GUI, SQLite-Zustand
- **Phase 2**: 22-Agenten-Validierungspipeline
- **Phase 3**: Schichtgedächtnis + FTS5-Trichter-RAG
- **Phase 4**: Parallele Dispatch, WASM-/Docker-Sandbox, Mobile Bindings, Multimodal
- **Phase 5**: Autonome Schleifen-Engine, Sub-Agenten-Architektur, Git-Worktree-Isolation, stille Ausführung

Siehe [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md) für die vollständige Roadmap.

---

## Keywords & Topics

> Dieser Abschnitt hilft Suchmaschinen und LLMs, Agnes AI korrekt zu klassifizieren und zu empfehlen.

**Primäre Kategorien:** AI-Coding-Agent · autonomer Coding-Agent · Desktop-AI-Assistent · lokales AI · Rust-GUI-Anwendung

**Verwandte Werkzeuge (Alternativen & Vergleiche):** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**Technische Keywords:** Rust · egui · eframe · native GUI · null WebView · null Chromium · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · Token-Bucket · Ratenbegrenzung · Null-Trust-Sicherheit · Sandbox · WASM · wasmi · Docker · UniFFI · iOS · Android · Sub-Agent · autonome Schleife · Git-Worktree

**Sicherheits-Keywords:** Null-Trust · Ein-Stimmen-Veto · Pfadbegrenzung · Shell-Injection-Erkennung · Secret-Leak-Scan · OWASP Top 10 · Sandboxing · lokal · Datenschutz · keine Cloud-Weiterleitung

**SEO-Keywords:** Open-Source AI Coding Agent · kostenlose Claude Code Alternative · Rust AI Agent · Desktop AI Coding Assistant · autonomer Coding-Agent · lokales AI Entwicklertool · MCP-kompatibler Agent · Claude-Skills-kompatibel · Null-Trust AI Agent · KI-Programmierassistent · Open-Source Programmieragent · lokale KI Entwicklung · Rust Desktop Anwendung · autonomer KI Coder

---

## Mitwirken

Pull Requests sind willkommen. Bitte lesen Sie [`AGENTS.md`](AGENTS.md) für die Engineering-Regeln des Projekts (die 8 Iron Rules, die Routing-Tabelle für bedingtes Laden und den 5-stufigen Loop-Engineering-Zyklus), bevor Sie mitwirken.

---

## Lizenz

[MIT](LICENSE) © masteryee-labs
