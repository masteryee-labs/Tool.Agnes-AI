# Agnes AI — Agent IA de programmation desktop open-source en Rust

> **Languages / 語言 / 言語 / Sprachen / Idiomas / Языки / 언어 :**
> [English](README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Русский](README.ru.md) · [한국어](README.ko.md) · [Français](README.fr.md) · [Português (BR)](README.pt-BR.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![GUI: egui](https://img.shields.io/badge/GUI-egui%2Feframe-blue.svg)](https://github.com/emilk/egui)
[![Platform: Desktop](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#installation-et-compilation)
[![MCP compatible](https://img.shields.io/badge/MCP-compatible-purple.svg)](https://modelcontextprotocol.io)
[![Claude Skills compatible](https://img.shields.io/badge/Claude%20Skills-compatible-green.svg)](#claude-compatible-skills)

---

## Qu'est-ce que Agnes AI ?

**Agnes AI est un agent IA de programmation desktop open-source, écrit en Rust pur avec une interface graphique native egui (zéro Chromium / WebView2).** Il exécute un **pipeline de validation zero-trust à 22 agents** de sorte que chaque action proposée par le modèle de langage est vérifiée croisée par des portes de sécurité déterministes avant de toucher votre système de fichiers, votre shell ou votre réseau. Il embarque également une **boucle autonome pilotée par objectif** (Discover → Plan → Execute → Verify → Iterate) avec une **architecture à sous-agents** (Planner / Generator / Evaluator) et une **isolation par Git worktree** pour une exécution parallèle sûre.

Agnes AI est une **alternative gratuite et local-first à Claude Code, Cursor, Aider et Continue.dev** — votre clé API et votre code ne quittent jamais votre machine, le binaire est minuscule, le démarrage est instantané et l'interface est une application native sombre et minimaliste (pas de navigateur embarqué, pas d'Electron).

> **En une ligne :** Un agent IA desktop en Rust natif, à haute défense et haute vitesse, qui ne fait jamais confiance au « ça a marché » verbal du modèle — seul un Exit Code 0 et un stderr vide comptent comme succès.

---

## Pourquoi Agnes AI ? (vs Claude Code / Cursor / Aider / Continue.dev)

| Fonctionnalité | Agnes AI | Claude Code | Cursor | Aider | Continue.dev |
|---|---|---|---|---|---|
| **Runtime** | GUI native Rust (egui) | Terminal | IDE basé sur Electron | Terminal | Plugin VS Code/JetBrains |
| **Taille du binaire** | Minuscule (~Mo) | Moyenne | Grande (~100 Mo+) | Minuscule | Dépend de l'IDE hôte |
| **Navigateur embarqué** | Aucun (zéro WebView2) | Aucun | Chromium | Aucun | Celui de l'IDE hôte |
| **Modèle de sécurité** | Pipeline zero-trust à 22 agents, veto à une voix | Limité | Limité | Minimal | Minimal |
| **Boucle autonome** | Oui (5 étapes, pilotée par objectif) | Oui (mode agent) | Non | Non | Non |
| **Architecture à sous-agents** | Oui (Planner/Generator/Evaluator) | Oui | Non | Non | Non |
| **Isolation par Git worktree** | Oui (sous-agents parallèles) | Non | Non | Non | Non |
| **Support MCP** | Oui (format `.mcp.json` de Claude) | Oui | Partiel | Non | Non |
| **Claude Skills** | Oui (`.claude/skills/`) | Oui | Non | Non | Non |
| **Mémoire RAG locale** | Oui (FTS5 + entonnoir à 3 étapes) | Limité | Limité | Non | Limité |
| **Mémoire inter-sessions** | Oui (leçons/pièges/état de boucle) | Non | Non | Non | Non |
| **Sandbox WASM / Docker** | Oui | Non | Non | Non | Non |
| **Bindings mobiles** | Oui (UniFFI, iOS/Android) | Non | Non | Non | Non |
| **Multimodal (image/vidéo)** | Oui | Oui | Oui | Non | Non |
| **Rotation multi-clés API** | Oui (compatible free-tier) | Non | Non | Non | Non |
| **Open source** | Oui (MIT) | Non | Non | Oui | Oui |
| **Prix** | Gratuit (apportez votre propre clé) | Payant | Payant | Gratuit (BYO key) | Gratuit/Payant |

**Agnes AI est idéal pour les développeurs qui veulent :**
- Un **agent IA de programmation local-first et respectueux de la vie privée** (pas de relais cloud de votre code)
- Des **garanties de sécurité fortes** (validation zero-trust, sandboxing, veto sur fuite de secret)
- Une **application desktop native et légère** au lieu d'Electron ou d'un terminal
- Une **exécution autonome pilotée par objectif** avec des critères de succès vérifiables
- Une **soutenabilité du free-tier** via la rotation multi-clés et la protection contre les limites de débit

---

## Fonctionnalités clés

### Expérience principale
- **GUI native Rust** — eframe/egui + wgpu, pas de navigateur embarqué, démarrage instantané, empreinte minuscule
- **Interface sombre minimaliste** — palette noir pur + blanc inspirée de Claude Code / Codex / Devin / Antigravity 2.0 ; aucune couleur de marque distrayante
- **Exécution silencieuse** — tous les processus enfants (commandes shell, compilateur, git, serveurs MCP) s'exécutent avec `CREATE_NO_WINDOW` sur Windows ; aucune fenêtre CMD/PowerShell ne s'ouvre sur votre bureau

### Espaces de travail
- **Mode double Projet / Global** — les onglets de la barre latérale basculent entre :
  - **Projets** : créez un projet à partir de n'importe quel dossier ; chaque session de discussion s'imbrique sous son projet ; les conversations sont persistées dans SQLite et reprennent exactement là où vous les avez laissées
  - **Global** : un onglet dédié pour l'opération sur l'ordinateur entier, où chaque action nécessite une confirmation par élément

### Boucle autonome (Phase 5)
- **Boucle pilotée par objectif** — donnez-lui un objectif et une condition de sortie ; elle exécute Discover → Plan → Execute → Verify → Iterate de façon autonome jusqu'à ce que la condition soit remplie ou que la limite d'itérations soit atteinte
- **Architecture à sous-agents** — trois rôles indépendants avec des prompts et un état de conversation séparés :
  - **Planner** — décompose l'objectif en sous-tâches atomiques
  - **Generator** — implémente une sous-tâche par exécution, en appelant les outils `write_file` / `run_command`
  - **Evaluator** — vérifie indépendamment la sortie du Generator ; rejette les affirmations de « succès » purement verbales
- **Isolation par Git worktree** — chaque sous-agent Generator travaille dans un git worktree + branche isolé ; les sous-agents parallèles ne se marchent jamais sur les fichiers ; le travail terminé est fusionné vers la branche principale
- **Mémoire inter-sessions** — les leçons, les pièges et l'état de boucle sont persistés dans `.agent/memory/` afin que l'agent reprenne là où il s'était arrêté entre les sessions

### Sécurité et validation
- **Pipeline de validation à 22 agents** — chaque appel d'outil du modèle est vérifié croisé par des portes déterministes (confinement de chemin, détection d'injection shell, scan de fuite de secret, audit d'AI-slop, …) avec veto à une voix
- **Alignement sandbox** — les fichiers `.rs` écrits sont compilés (et leurs tests exécutés) immédiatement ; « prétend le succès mais ne compile pas » est rejeté sur-le-champ
- **Sandbox WASM** — le code non fiable s'exécute via l'interpréteur Rust pur `wasmi` avec un linker vide (aucun import hôte → pas d'I/O/syscalls/réseau) et un compteur de carburant
- **Sandbox Docker** — les tâches de niveau compilation s'exécutent dans un conteneur avec `--network=none`, `--rm`, le workspace monté sur `/work` ; arguments vectorisés (pas de shell)
- **Pas de confiance verbale** — Exit Code == 0 et stderr vide constituent la seule définition du succès ; le « ça a marché » verbal du modèle n'est jamais cru

### Compatibilité
- **Skills compatibles Claude** — déposez des fichiers `SKILL.md` sous `.claude/skills/<name>/` dans votre workspace ; invoquez-les en tapant `/name` dans la discussion. Les règles de projet `CLAUDE.md` sont chargées automatiquement
- **MCP compatible Claude** — placez un fichier `.mcp.json` standard à la racine de votre workspace, ou ajoutez des serveurs dans Paramètres → Serveurs MCP ; les listes d'outils connectés sont exposées au modèle automatiquement

### Performance
- **Mémoire en couches** — découpage par fenêtre glissante + RAG en entonnoir à 3 étapes sur un index FTS5, avec des watermarks de distillation pour éviter de rebrûler des tokens
- **Limitation de débit et protection 20 RPM** — un limiteur global partagé à seau de tokens contrôle chaque appel API (distillation et récupération incluses) ; `acquire()` attend le remplissage plutôt que de rejeter, donc les rafales ne franchissent jamais le plafond de 20 requêtes/minute du free-tier. Sur un 429, le client applique un backoff exponentiel basé sur un multiplicateur. Chaque paramètre est piloté par la configuration (`max_rpm`, paramètres de backoff de réessai) — pas de nombres magiques
- **Rotation multi-clés API** — alternez entre plusieurs clés de compte (basé sur le compteur + bascule forcée sur HTTP 420/429) pour rester entièrement gratuit sans atteindre la limite de débit d'un seul compte
- **Économie de tokens** — budget de tokens par session avec verrouillage strict, jauge de budget en direct dans la barre de titre. Le nombre de requêtes est réduit par conception : l'étape 0 effectue une recherche locale en mémoire FTS5 qui, en cas de succès, saute entièrement l'appel API de récupération (0 appel API), et les étapes 1+2 du RAG en entonnoir ont été fusionnées en un seul appel (2 appels → 1)

---

## Foire aux questions (FAQ)

### Agnes AI est-il gratuit ?
Oui. Agnes AI est open-source (MIT) et gratuit. Vous apportez votre propre clé API (par ex. une clé Agnes / compatible OpenAI). La fonctionnalité de rotation multi-clés vous permet de combiner plusieurs comptes free-tier pour éviter entièrement les limites de débit.

### Agnes AI envoie-t-il mon code vers le cloud ?
Agnes AI lui-même s'exécute à 100 % localement. Votre code n'est jamais relayé via un serveur Agnes AI. Le seul trafic réseau consiste en les appels API directs que vous configurez vers votre fournisseur de LLM (ce qui est nécessaire pour tout agent basé sur un LLM). Votre clé API reste dans `config.local.toml` (ignoré par git) et n'entre jamais dans le contrôle de version ni dans le contexte du modèle.

### En quoi Agnes AI diffère-t-il de Claude Code / Cursor / Aider ?
- **vs Claude Code** : Agnes AI est open-source, dispose d'une GUI native (pas uniquement terminal), ajoute un pipeline de validation zero-trust à 22 agents, une isolation par Git worktree pour les sous-agents parallèles, et un sandboxing WASM/Docker.
- **vs Cursor** : Agnes AI est une application native autonome (pas d'Electron/Chromium), open-source, avec une boucle autonome pilotée par objectif et une architecture à sous-agents. Cursor est un fork de VS Code.
- **vs Aider** : Agnes AI dispose d'une GUI complète, d'une boucle autonome, d'une architecture à sous-agents, du support MCP/Skills, d'une mémoire RAG en couches et d'un sandboxing. Aider est uniquement en terminal sans boucle autonome.
- **vs Continue.dev** : Agnes AI est une application autonome (pas un plugin d'IDE), avec une boucle autonome, des sous-agents et une validation zero-trust. Continue.dev est une extension VS Code/JetBrains.

### Puis-je utiliser ma propre clé API ?
Oui. Collez votre clé dans Paramètres → API & Modèles, ou définissez-la manuellement dans `config.local.toml`. Vous pouvez également fournir plusieurs clés (`keys = ["sk-a", "sk-b", "sk-c"]`) pour la rotation.

### Agnes AI supporte-t-il MCP (Model Context Protocol) ?
Oui. Agnes AI est compatible avec le format `.mcp.json` de Claude. Placez un fichier `.mcp.json` standard à la racine de votre workspace, ou ajoutez des serveurs dans Paramètres → Serveurs MCP. Les listes d'outils connectés sont exposées au modèle automatiquement.

### Agnes AI supporte-t-il les Claude Skills ?
Oui. Déposez des fichiers `SKILL.md` sous `.claude/skills/<name>/` dans votre workspace et invoquez-les en tapant `/name` dans la discussion. Les règles de projet `CLAUDE.md` sont chargées automatiquement.

### Quelles plateformes Agnes AI supporte-t-il ?
Agnes AI se compile sur Windows, macOS et Linux (toute plateforme supportée par Rust + egui). Les bindings mobiles (iOS/Android) sont disponibles via la cargo feature `mobile` grâce à UniFFI.

### Agnes AI est-il open source ?
Oui, publié sous licence MIT.

### En quelle langue Agnes AI est-il écrit ?
En Rust pur, utilisant eframe/egui pour la GUI native, rusqlite pour l'état, reqwest pour HTTP et wasmi pour le sandbox WASM. Pas de JavaScript, pas d'Electron, pas de Chromium, pas de WebView2.

### Agnes AI dispose-t-il d'un mode autonome ?
Oui. Basculez en **Mode objectif** (💬 Discussion → 🎯 Objectif), décrivez un objectif et une condition de sortie, appuyez sur Démarrer. La boucle s'exécute de façon autonome : le Planner décompose l'objectif, le Generator implémente chaque sous-tâche, l'Evaluator vérifie chacune. Arrêtez à tout moment.

---

## Installation et compilation

Prérequis : [chaîne d'outils Rust](https://rust-lang.org/) (stable, édition 2021).

```powershell
git clone https://github.com/masteryee-labs/Tool.Agnes-AI.git
cd Tool.Agnes-AI
cargo build --release --manifest-path src-tauri/Cargo.toml
# Run the GUI
cargo run --release --manifest-path src-tauri/Cargo.toml --bin agnes-ai
```

### Bindings mobiles (iOS/Android)

```powershell
cargo build --release --manifest-path src-tauri/Cargo.toml --features mobile
```

---

## Configuration

Tous les paramètres locaux se trouvent dans `config.local.toml` à la racine du dépôt (auto-créé, **ignoré par git** — votre clé API n'entre jamais dans le contrôle de version).

Le chemin le plus simple est la page Paramètres intégrée à l'application (⚙ dans la barre latérale) :

1. **Paramètres → API & Modèles** — collez votre clé API, appuyez sur **Enregistrer**. La page affiche une copie masquée de la clé stockée (`sk-xx…xxxx`) ainsi que son empreinte et un « Enregistré ✓ » vert pour que vous sachiez toujours ce qui est actif.
2. **Paramètres → Serveurs MCP** — appuyez sur **+ Ajouter un serveur**, remplissez le nom / la commande / les args ; le serveur démarre immédiatement et est persisté dans la configuration.
3. **Paramètres → Skills** — liste chaque skill détecté dans le workspace courant.

Équivalent manuel dans `config.local.toml` :

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

### MCP au format Claude (`.mcp.json` dans votre workspace)

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

### Skills au format Claude

```
your-project/
└── .claude/
    └── skills/
        └── deploy/
            └── SKILL.md   # YAML frontmatter: name + description, then instructions
```

Tapez `/deploy …` dans la discussion pour invoquer. Les Skills et les règles `CLAUDE.md` sont injectés dans le prompt système de manière déterministe (aucun appel API supplémentaire).

---

## Utilisation

### Mode discussion
1. **Créez un projet** — barre latérale → onglet Projets → **+ Nouveau projet**, choisissez un dossier.
2. **Discutez** — tapez une tâche ; une nouvelle session est créée sous le projet actif et persistée. Cliquez sur n'importe quelle session dans la barre latérale pour la reprendre plus tard avec tout l'historique.
3. **Mode Global** — basculez vers l'onglet **Global** pour opérer en dehors des dossiers de projet. Chaque action apparaît dans le panneau de droite pour une approbation explicite par élément.
4. **Observez les agents** — le panneau de droite affiche les 22 agents de validation et leurs verdicts PASS/REJECT à chaque étape ; les appels d'outils en attente y attendent votre Approbation/Rejet.

### Mode objectif
1. **Basculez en Mode objectif** — cliquez sur la bascule à capsule en haut du panneau central (💬 Discussion → 🎯 Objectif).
2. **Décrivez l'objectif** — saisissez ce que vous voulez accomplir et une condition de sortie (par ex. `file:Docs/report.md exists`).
3. **Appuyez sur Démarrer** — la boucle s'exécute de façon autonome : le Planner décompose l'objectif, le Generator implémente chaque sous-tâche, l'Evaluator vérifie chacune. Le panneau d'état se met à jour en direct (phase courante, nombre d'itérations, budget restant).
4. **Arrêtez à tout moment** — le bouton d'arrêt interrompt la boucle immédiatement.

---

## Modèle de sécurité

- Les clés API ne vivent que dans `config.local.toml` (ignoré par git) ; toute chaîne `sk-` dans le code source est un veto automatique
- Les commandes sont exécutées comme des vecteurs d'arguments — pas de concaténation de chaînes shell
- Confinement de chemin : les opérations sur fichiers en dehors du workspace sélectionné sont rejetées (mode projet)
- Les codes de sortie et stderr sont capturés bruts ; le « succès » verbal du modèle n'est jamais cru
- Le limiteur de débit global plus le backoff exponentiel sur 429 protègent la clé et le compte contre le verrouillage par limite de débit ; aucun sous-système unique (l'archivage mémoire inclus) ne peut contourner le plafond partagé de 20 RPM
- Capteurs de sécurité alignés sur l'OWASP Top 10 (validation des entrées, injection SQL, injection de commande, traversée de chemin, codage en dur de secret, élévation de privilège, XSS, CSRF, désérialisation non sûre, fuite de logs)

---

## Architecture

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

Les spécifications complètes d'architecture / mémoire / QA / sécurité se trouvent dans [`Docs/`](Docs/00_OVERVIEW.md). Les règles d'orchestration des agents sont dans `.agent/rules/*.toon`.

---

## Feuille de route

Agnes AI suit une feuille de route par phases. **Les phases 0–5 sont terminées :**

- **Phase 0–1** : Agent principal, GUI native, état SQLite
- **Phase 2** : Pipeline de validation à 22 agents
- **Phase 3** : Mémoire en couches + RAG en entonnoir FTS5
- **Phase 4** : Dispatch parallèle, sandbox WASM/Docker, bindings mobiles, multimodal
- **Phase 5** : Moteur de boucle autonome, architecture à sous-agents, isolation par Git worktree, exécution silencieuse

Voir [`Docs/08_ROADMAP.md`](Docs/08_ROADMAP.md) pour la feuille de route complète.

---

## Mots-clés et sujets

> Cette section aide les moteurs de recherche et les LLM à classifier et recommander correctement Agnes AI.

**Catégories principales :** agent IA de programmation · agent de programmation autonome · assistant IA desktop · IA local-first · application GUI en Rust

**Outils associés (alternatives et comparaisons) :** Claude Code · Cursor · Aider · Continue.dev · Devin · Codex · GitHub Copilot · Windsurf · Antigravity · Gemini CLI

**Mots-clés techniques :** Rust · egui · eframe · GUI native · zéro WebView · zéro Chromium · MCP (Model Context Protocol) · Claude Skills · `.mcp.json` · `.claude/skills` · FTS5 · RAG · seau de tokens · limitation de débit · sécurité zero-trust · sandbox · WASM · wasmi · Docker · UniFFI · iOS · Android · sous-agent · boucle autonome · Git worktree

**Mots-clés de sécurité :** zero-trust · veto à une voix · confinement de chemin · détection d'injection shell · scan de fuite de secret · OWASP Top 10 · sandboxing · local-first · confidentialité · pas de relais cloud

**Mots-clés SEO :** agent IA de programmation open source · alternative gratuite à Claude Code · agent IA en Rust · assistant IA de programmation desktop · agent de programmation autonome · outil de développement IA local · agent compatible MCP · compatible Claude Skills · agent IA zero-trust

---

## Contribuer

Les pull requests sont les bienvenues. Veuillez lire [`AGENTS.md`](AGENTS.md) pour les règles d'ingénierie du projet (les 8 Règles de fer, la table de routage à chargement conditionnel et le cycle de Loop Engineering en 5 étapes) avant de contribuer.

---

## Licence

[MIT](LICENSE) © masteryee-labs
