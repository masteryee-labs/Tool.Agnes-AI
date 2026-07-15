# Agnes AI — 오픈소스 Rust 데스크톱 AI 코딩 에이전트

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#설치-및-빌드)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-compatible-skills)

---

## Agnes AI란?

**Agnes AI는 순수 Rust로 작성된 오픈소스 데스크톱 AI 코딩 에이전트로, 네이티브 egui GUI(Chromium / WebView2 제로)를 갖추고 있습니다.** 언어 모델이 제안하는 모든 작업이 파일 시스템, 셸 또는 네트워크에 닿기 전에 결정론적 보안 게이트의 교차 검사를 받도록 **22-에이전트 제로 트러스트 검증 파이프라인**을 실행합니다. 또한 **서브 에이전트 아키텍처**(Planner / Generator / Evaluator)와 병렬 실행을 위한 **Git worktree 격리**를 갖춘 **자율 목표 주도 루프**(Discover → Plan → Execute → Verify → Iterate)를 제공합니다.

Agnes AI는 **Claude Code, Cursor, Aider, Continue.dev의 무료, 로컬 우선 대안**입니다 — API 키와 코드가 머신을 떠나지 않으며, 바이너리가 작고, 시작이 즉각적이며, UI는 미니멀한 다크 네이티브 앱입니다(임베디드 브라우저 없음, Electron 없음).

> **한 줄 요약:** 모델의 구두 "작동했다"를 절대 신뢰하지 않는 고방어·고속 네이티브 Rust 데스크톱 AI 에이전트 — 오직 Exit Code 0과 빈 stderr만이 성공으로 인정됩니다.

---

## 왜 Agnes AI인가? (vs Claude Code / Cursor / Aider / Continue.dev)

| 기능 | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **런타임** | 네이티브 Rust GUI (egui) | 터미널 | Electron 기반 IDE | 터미널 | VS Code/JetBrains 플러그인 |
| **바이너리 크기** | 매우 작음 (~MB) | 중간 | 큼 (~100 MB+) | 매우 작음 | 호스트 IDE에 따라 다름 |
| **임베디드 브라우저** | 없음 (WebView2 제로) | 없음 | Chromium | 없음 | 호스트 IDE의 것 |
| **보안 모델** | 22-에이전트 제로 트러스트 파이프라인, 1표 거부권 | 제한적 | 제한적 | 최소 | 최소 |
| **자율 루프** | 있음 (5단계, 목표 주도) | 있음 (에이전트 모드) | 없음 | 없음 | 없음 |
| **서브 에이전트 아키텍처** | 있음 (Planner/Generator/Evaluator) | 있음 | 없음 | 없음 | 없음 |
| **Git worktree 격리** | 있음 (병렬 서브 에이전트) | 없음 | 없음 | 없음 | 없음 |
| **MCP 지원** | 있음 (Claude `.mcp.json` 형식) | 있음 | 부분적 | 없음 | 없음 |
| **Claude Skills** | 있음 (`.claude/skills/`) | 있음 | 없음 | 없음 | 없음 |
| **로컬 RAG 메모리** | 있음 (FTS5 + 3단계 퍼널) | 제한적 | 제한적 | 없음 | 제한적 |
| **세션 간 메모리** | 있음 (교훈/함정/루프 상태) | 없음 | 없음 | 없음 | 없음 |
| **WASM / Docker 샌드박스** | 있음 | 없음 | 없음 | 없음 | 없음 |
| **모바일 바인딩** | 있음 (UniFFI, iOS/Android) | 없음 | 없음 | 없음 | 없음 |
| **멀티모달 (이미지/비디오)** | 있음 | 있음 | 있음 | 없음 | 없음 |
| **다중 API 키 로테이션** | 있음 (무료 티어 친화적) | 없음 | 없음 | 없음 | 없음 |
| **오픈소스** | 있음 (MIT) | 없음 | 없음 | 있음 | 있음 |
| **가격** | 무료 (본인 키 지참) | 유료 | 유료 | 무료 (BYO 키) | 무료/유료 |

**Agnes AI는 다음을 원하는 개발자에게 가장 적합합니다:**
- **로컬 우선, 프라이버시 존중** AI 코딩 에이전트 (코드의 클라우드 릴레이 없음)
- **강력한 보안 보장** (제로 트러스트 검증, 샌드박싱, 비밀 유출 거부권)
- Electron이나 터미널 대신 **네이티브, 경량 데스크톱 앱**
- 검증 가능한 성공 기준을 갖춘 **자율 목표 주도 실행**
- 다중 키 로테이션과 속도 제한 보호를 통한 **무료 티어 지속 가능성**

---

## 핵심 기능

### 핵심 경험
- **네이티브 Rust GUI** — eframe/egui + wgpu, 임베디드 브라우저 없음, 즉각 시작, 작은 풋프린트
- **미니멀 다크 UI** — Claude Code / Codex / Devin / Antigravity 2.0에서 영감을 받은 순수 블랙 + 화이트 팔레트; 시선을 분산시키는 브랜드 컬러 없음
- **조용한 실행** — 모든 자식 프로세스(셸 명령, 컴파일러, git, MCP 서버)는 Windows에서 `CREATE_NO_WINDOW`로 실행; 데스크톱에 CMD/PowerShell 창이 팝업되지 않음

### 워크스페이스
- **프로젝트 / 글로벌 듀얼 모드** — 사이드바 탭이 다음을 전환:
  - **프로젝트**: 임의의 폴더에서 프로젝트 생성; 모든 채팅 세션은 해당 프로젝트 아래에 중첩; 대화는 SQLite에 저장되며 중단한 지점에서 정확히 재개
  - **글로벌**: 컴퓨터 전체 작업을 위한 전용 탭, 모든 작업은 항목별 확인 필요

### 자율 루프 (Phase 5)
- **목표 주도 루프** — 목표와 종료 조건을 주면, 조건이 충족되거나 반복 한도에 도달할 때까지 Discover → Plan → Execute → Verify → Iterate를 자율적으로 실행
- **서브 에이전트 아키텍처** — 별도의 프롬프트와 대화 상태를 갖는 세 가지 독립 역할:
  - **Planner** — 목표를 원자적 하위 작업으로 분해
  - **Generator** — 실행마다 하나의 하위 작업을 구현, `write_file` / `run_command` 도구 호출
  - **Evaluator** — Generator의 출력을 독립적으로 검증; 구두만의 "성공" 주장 거부
- **Git worktree 격리** — 각 Generator 서브 에이전트는 격리된 git worktree + 브랜치에서 작업; 병렬 서브 에이전트가 서로의 파일을 침범하지 않음; 완료된 작업은 메인 브랜치로 병합
- **세션 간 메모리** — 교훈, 함정, 루프 상태가 `.agent/memory/`에 저장되어 세션 간에 이어서 작업 가능

### 보안 및 검증
- **22-에이전트 검증 파이프라인** — 모델의 모든 도구 호출이 결정론적 게이트(경로 제한, 셸 인젝션 탐지, 비밀 유출 스캔, AI-slop 감사 등)로 교차 검사되며 1표 거부권 적용
- **샌드박스 정렬** — 작성된 `.rs` 파일은 즉시 컴파일(및 테스트 실행); "성공한다고 주장하지만 컴파일 안 됨"은 즉시 거부
- **WASM 샌드박스** — 신뢰할 수 없는 코드는 빈 링커(호스트 임포트 없음 → I/O/시스콜/네트워크 없음)와 연료 계량이 있는 `wasmi` 순수 Rust 인터프리터로 실행
- **Docker 샌드박스** — 컴파일 수준 작업은 `--network=none`, `--rm`, 워크스페이스를 `/work`에 마운트한 컨테이너에서 실행; 벡터화된 인자(셸 없음)
- **구두 신뢰 없음** — Exit Code == 0이고 stderr가 비어 있는 것만이 성공의 유일한 정의; 모델의 구두 "작동했다"는 절대 신뢰하지 않음

### 호환성
- **Claude 호환 Skills** — 워크스페이스의 `.claude/skills/<name>/` 아래에 `SKILL.md` 파일을 넣으세요; 채팅에서 `/name`을 입력해 호출. `CLAUDE.md` 프로젝트 규칙은 자동으로 로드
- **Claude 호환 MCP** — 워크스페이스 루트에 표준 `.mcp.json`을 넣거나, Settings → MCP Servers에서 서버 추가; 연결된 도구 목록은 모델에 자동 노출

### 성능
- **계층화 메모리** — 슬라이딩 윈도우 청킹 + FTS5 인덱스 위의 3단계 퍼널 RAG, 토큰 재소모 방지를 위한 증류 워터마크 포함
- **속도 제한 및 20 RPM 보호** — 하나의 전역 공유 토큰 버킷 리미터가 모든 API 호출(증류 및 검색 포함)을 게이트; `acquire()`는 거부 대신 리필을 대기하므로 버스트가 20 요청/분 무료 티어 한도를 위반하지 않음. 429 시 클라이언트는 승수 기반 지수 백오프 적용. 모든 매개변수는 설정 기반(`max_rpm`, 재시도 백오프 설정) — 매직 넘버 없음
- **다중 API 키 로테이션** — 여러 계정 키를 순회(카운트 기반 + HTTP 420/429 시 강제 전환)하여 단일 계정의 속도 제한에 걸리지 않고 완전 무료 유지
- **토큰 경제** — 하드 잠금이 있는 세션별 토큰 예산, 타이틀 바에 실시간 예산 미터. 요청 수는 설계상 절감: Stage 0은 로컬 FTS5 메모리 조회를 수행하여 히트 시 검색 API 호출을 완전히 건너뜀(0 API 호출), 퍼널 RAG의 Stage 1+2는 단일 호출로 병합(2 호출 → 1)

---

## 자주 묻는 질문 (FAQ)

### Agnes AI는 무료인가요?
네. Agnes AI는 오픈소스(MIT)이며 무료입니다. 본인의 API 키(예: Agnes / OpenAI 호환 키)를 지참하세요. 다중 키 로테이션 기능을 통해 여러 무료 티어 계정을 조합하여 속도 제한을 완전히 회피할 수 있습니다.

### Agnes AI가 내 코드를 클라우드로 보내나요?
Agnes AI 자체는 100% 로컬에서 실행됩니다. 코드가 Agnes AI 서버를 통해 릴레이되지 않습니다. 유일한 네트워크 트래픽은 LLM 제공자에게 직접 API 호출하는 것뿐(LLM 기반 에이전트에 필수)입니다. API 키는 `config.local.toml`(git-ignored)에 머물며 버전 관리나 모델의 컨텍스트에 진입하지 않습니다.

### Agnes AI는 Claude Code / Cursor / Aider와 어떻게 다른가요?
- **vs Claude Code**: Agnes AI는 오픈소스이며 네이티브 GUI(터미널 전용 아님)를 갖추고 있고, 22-에이전트 제로 트러스트 검증 파이프라인, 병렬 서브 에이전트를 위한 Git worktree 격리, WASM/Docker 샌드박싱을 추가합니다.
- **vs Cursor**: Agnes AI는 독립형 네이티브 앱(Electron/Chromium 없음)이며 오픈소스이고, 자율 목표 주도 루프와 서브 에이전트 아키텍처를 갖춤. Cursor는 VS Code의 포크입니다.
- **vs Aider**: Agnes AI는 풀 GUI, 자율 루프, 서브 에이전트 아키텍처, MCP/Skills 지원, 계층화 RAG 메모리, 샌드박싱을 갖춤. Aider는 자율 루프 없이 터미널 전용입니다.
- **vs Continue.dev**: Agnes AI는 독립형 앱(IDE 플러그인 아님)이며 자율 루프, 서브 에이전트, 제로 트러스트 검증을 갖춤. Continue.dev는 VS Code/JetBrains 확장입니다.

### 내 API 키를 사용할 수 있나요?
네. Settings → API & Models에 키를 붙여넣거나, `config.local.toml`에 수동으로 설정하세요. 로테이션을 위해 여러 키(`keys = ["sk-a", "sk-b", "sk-c"]`)를 제공할 수도 있습니다.

### Agnes AI는 MCP (Model Context Protocol)를 지원하나요?
네. Agnes AI는 Claude `.mcp.json` 형식과 호환됩니다. 워크스페이스 루트에 표준 `.mcp.json`을 넣거나, Settings → MCP Servers에서 서버를 추가하세요. 연결된 도구 목록은 모델에 자동 노출됩니다.

### Agnes AI는 Claude Skills를 지원하나요?
네. 워크스페이스의 `.claude/skills/<name>/` 아래에 `SKILL.md` 파일을 넣고 채팅에서 `/name`을 입력해 호출하세요. `CLAUDE.md` 프로젝트 규칙은 자동으로 로드됩니다.

### Agnes AI는 어떤 플랫폼을 지원하나요?
Agnes AI는 Windows, macOS, Linux(Rust + egui가 지원하는 모든 플랫폼)에서 빌드됩니다. 모바일(iOS/Android) 바인딩은 `mobile` cargo 기능 뒤에서 UniFFI를 통해 사용 가능합니다.

### Agnes AI는 오픈소스인가요?
네, MIT 라이선스로 배포됩니다.

### Agnes AI는 어떤 언어로 작성되었나요?
순수 Rust이며, 네이티브 GUI에 eframe/egui, 상태에 rusqlite, HTTP에 reqwest, WASM 샌드박스에 wasmi를 사용합니다. JavaScript, Electron, Chromium, WebView2는 없습니다.

### Agnes AI에 자율 모드가 있나요?
네. **Goal 모드**로 전환(💬 Chat → 🎯 Goal), 목표와 종료 조건을 설명하고 Start를 누르세요. 루프가 자율 실행: Planner가 목표를 분해, Generator가 각 하위 작업을 구현, Evaluator가 각각을 검증. 언제든 중지 가능.

---

## 설치 및 빌드

사전 요구: [Rust toolchain](https://rust-lang.org/) (stable, 2021 edition).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### 모바일 바인딩 (iOS/Android)

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## 설정

모든 로컬 설정은 저장소 루트의 `config.local.toml`에 있습니다(자동 생성, **git-ignored** — API 키가 버전 관리에 진입하지 않음).

가장 쉬운 방법은 인앱 Settings 페이지(사이드바의 ⚙)입니다:

1. **Settings → API & Models** — API 키를 붙여넣고 **Save**를 누르세요. 페이지에 저장된 키의 마스킹 복사본(`sk-xx…xxxx`)과 지문, 녹색 "Saved ✓"가 표시되어 활성 상태를 항상 알 수 있습니다.
2. **Settings → MCP Servers** — **+ Add Server**를 누르고 이름 / 명령 / 인자를 입력; 서버가 즉시 시작되고 설정에 저장.
3. **Settings → Skills** — 현재 워크스페이스에서 감지된 모든 스킬을 나열.

`config.local.toml`에서의 수동 동등:

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

### Claude 형식 MCP (워크스페이스의 `.mcp.json`)

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

### Claude 형식 Skills

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

채팅에서 `/deploy …`를 입력해 호출. Skills와 `CLAUDE.md` 규칙은 결정론적으로 시스템 프롬프트에 주입(추가 API 호출 없음).

---

## 사용 방법

### Chat 모드
1. **프로젝트 생성** — 사이드바 → Projects 탭 → **+ New Project**, 폴더 선택.
2. **채팅** — 작업을 입력; 활성 프로젝트 아래에 새 세션이 생성되고 저장됨. 사이드바의 세션을 클릭하여 전체 기록과 함께 나중에 재개.
3. **글로벌 모드** — 프로젝트 폴더 외부에서 작업하려면 **Global** 탭으로 전환. 모든 작업이 오른쪽 패널에 표시되어 항목별 명시 승인 필요.
4. **에이전트 관찰** — 오른쪽 패널에 22개 검증 에이전트와 단계별 PASS/REJECT 판정이 표시; 대기 중인 도구 호출은 Approve/Reject를 기다림.

### Goal 모드
1. **Goal 모드로 전환** — 중앙 패널 상단의 캡슐 토글 클릭(💬 Chat → 🎯 Goal).
2. **목표 설명** — 수행할 작업과 종료 조건 입력(예: `file:Docs/report.md exists`).
3. **Start 누름** — 루프가 자율 실행: Planner가 목표 분해, Generator가 각 하위 작업 구현, Evaluator가 각각 검증. 상태 패널이 실시간 업데이트(현재 단계, 반복 횟수, 남은 예산).
4. **언제든 중지** — 중지 버튼이 루프를 즉시 중단.

---

## 보안 모델

- API 키는 `config.local.toml`(git-ignored)에만 존재; 소스의 모든 `sk-` 문자열은 자동 거부
- 명령은 인자 벡터로 실행 — 셸 문자열 연결 없음
- 경로 제한: 선택된 워크스페이스 외부의 파일 작업은 거부(프로젝트 모드)
- Exit 코드와 stderr는 원시 캡처; 모델의 구두 "성공"은 절대 신뢰하지 않음
- 전역 속도 리미터와 429 지수 백오프가 키와 계정을 속도 제한 잠금에서 보호; 단일 하위 시스템(메모리 아카이브 포함)도 공유 20 RPM 한도를 우회할 수 없음
- OWASP Top 10 정렬 보안 센서(입력 검증, SQL 인젝션, 명령 인젝션, 경로 트래버설, 비밀 하드코딩, 권한 상승, XSS, CSRF, 불안전 역직렬화, 로그 유출)

---

## 아키텍처

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

전체 아키텍처 / 메모리 / QA / 보안 사양은 [`Docs/`](Docs/00_OVERVIEW.md)에 있습니다. 에이전트 오케스트레이션 규칙은 `.agent/rules/*.toon`에 있습니다.

---

## 로드맵

Agnes AI는 단계적 로드맵을 따릅니다. **Phase 0–5가 완료되었습니다:**

- **Phase 0–1**: 핵심 에이전트, 네이티브 GUI, SQLite 상태
- **Phase 2**: 22-에이전트 검증 파이프라인
- **Phase 3**: 계층화 메모리 + FTS5 퍼널 RAG
- **Phase 4**: 병렬 디스패치, WASM/Docker 샌드박스, 모바일 바인딩, 멀티모달
- **Phase 5**: 자율 루프 엔진, 서브 에이전트 아키텍처, Git worktree 격리, 조용한 실행

전체 로드맵은 [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md)를 참조하세요.

---

## 키워드 및 주제

> 이 섹션은 검색 엔진과 LLM이 Agnes AI를 올바르게 분류하고 추천하도록 돕습니다.

**주요 카테고리:** AI 코딩 에이전트 · 자율 코딩 에이전트 · 데스크톱 AI 어시스턴트 · 로컬 우선 AI · Rust GUI 애플리케이션

**관련 도구(대안 및 비교):** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**기술 키워드:** Rust · egui · eframe · 네이티브 GUI · WebView 제로 · Chromium 제로 · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · 토큰 버킷 · 속도 제한 · 제로 트러스트 보안 · 샌드박스 · WASM · wasmi · Docker · UniFFI · iOS · Android · 서브 에이전트 · 자율 루프 · Git worktree

**보안 키워드:** 제로 트러스트 · 1표 거부권 · 경로 제한 · 셸 인젝션 탐지 · 비밀 유출 스캔 · OWASP Top 10 · 샌드박싱 · 로컬 우선 · 프라이버시 · 클라우드 릴레이 없음

**SEO 키워드:** 오픈소스 AI 코딩 에이전트 · Claude Code 무료 대안 · Rust AI 에이전트 · 데스크톱 AI 코딩 어시스턴트 · 자율 코딩 에이전트 · 로컬 AI 개발 도구 · MCP 호환 에이전트 · Claude Skills 호환 · 제로 트러스트 AI 에이전트

---

## 기여

풀 리퀘스트를 환영합니다. 기여 전에 프로젝트의 엔지니어링 규칙(8개 강철 규칙, 조건부 로딩 라우팅 테이블, 5단계 Loop Engineering 사이클)을 담은 [`AGENTS.md`](AGENTS.md)를 읽어주세요.

---

## 라이선스

[MIT](LICENSE) © masteryee-labs
