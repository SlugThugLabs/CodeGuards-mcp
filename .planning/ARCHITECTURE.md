---
modules: [server, guards, plugins, detectors, intent, planning, import_analyzer, fixes, config]
layers: [transport, orchestration, checks, analysis, contracts, extension, config]
allowed_dependencies:
  server: [guards, detectors, config, intent, planning]
  guards: [detectors, fixes, intent, plugins, import_analyzer, config]
  plugins: []
  detectors: []
  intent: []
  planning: []
  import_analyzer: []
  fixes: []
  config: []
enforce: []
---

# Architecture — single source of truth

> This document is the canonical record of this project's goal and architecture.
> It covers goal, layers, modules, constraints, and quality goals.
> Editing here means you mean it — a deliberate act, not a drive-by.

---

## Goal

CodeGuards is an MCP server that lets any AI coding agent declare its architectural
intent before writing code, then enforces that intent with language-agnostic quality
guards, structural drift detection, and actionable fix suggestions — so generated
code matches the intended architecture and does not silently degrade.

---

## Principles

- **Generic-first** — full baseline coverage with zero plugins, zero AST, zero
  language knowledge. Plugins are additive precision, never prerequisites
  (v0.2 core guarantee #3; `guards/generic.py` contains no language branches).
- **Coach, not cop** — every violation ships with a fix suggestion; intent
  cross-referencing re-frames violations against the rules the user declared.
  The v0.2 "coaching report" (positives, streaks, suggestions) is the target;
  today's `format_report` is a simpler ancestor.
- **Contract-first** — no declared intent, no full-project enforcement.
  `check_project` refuses to run without `.codeguards/intent.json`.
- **Architecture is authoritative** — this file is the contract *and* the scope
  gate. Its YAML frontmatter is machine-read on every `check_project`
  (`planning.load_architecture`).
- **No implicit global opinion** — every rule either derives from intent /
  architecture or is an explicitly declared baseline in `config.py` DEFAULTS.
- **Sandbox-first** — every MCP path argument passes `_is_safe_project_path`
  before any file I/O: credential stores (`~/.aws`, `~/.ssh`, ...), kernel
  filesystems (`/proc`, `/sys`, `/dev`), and non-existent paths are refused.
- **Visible thinking** — `notify_thinking()` signals before any operation
  expected to take >3s, so silence is never mistaken for a freeze
  (see `THINKING_PAUSE.md`).

---

## Layers

| Layer | Where | Responsibility |
|---|---|---|
| transport | `server.py` | MCP protocol, stdio + SSE transports, tool dispatch, report formatting |
| orchestration | `guards/__init__.py` | Pipeline assembly: generic → structural → plugin → project-level checks; post-processing (fix suggestions, intent cross-reference) |
| checks | `guards/generic.py`, `guards/structural.py` | Guard implementations (19 generic, 5 structural + baseline) |
| analysis | `import_analyzer.py`, `detectors.py` | Cross-language import analysis without AST; language sniffing; third-party classification |
| contracts | `intent.py`, `planning.py` | `intent.json`, `ARCHITECTURE.md`, `PROJECT_PLAN.md` creation, parsing, task state |
| extension | `plugins/` | `GuardRegistry`; per-language guards and capability extractors |
| config | `config.py`, `constants.py` | `.codeguards.yaml` loading + deep merge; threshold defaults |

---

## Modules

- **server** (`server.py`, 619 lines) — entry point; 10 MCP tools
  (`check_project`, `check_file`, `detect_languages`, `list_guards`,
  `declare_intent`, `save_baseline`, `probe`, `plan`, `update_task`,
  `list_tasks`); sandbox; `main()` stdio vs `--port` SSE (starlette + uvicorn).
  Largest file by design — orchestrator, tuned in `.codeguards.yaml`.
- **guards** (`guards/`) — `__init__.py` (321): `run_checks`,
  `check_missing_tests`, `enrich_with_fixes`; `generic.py` (882): 19
  language-agnostic checks in `ALL_GENERIC_CHECKS`; `structural.py` (293):
  responsibility clusters, fan-out, layer enforcement, structural health,
  growth drift, structural baseline.
- **plugins** (`plugins/`) — `__init__.py` (131): `GuardRegistry` (guards +
  extractors), global registry, `load_plugins()` discovery; `python.py` (106):
  function-block + missing-docs extractors; `rust.py` (133): `no_unwrap` and
  `tracing_instrument` guards + missing-docs extractor.
- **detectors** (`detectors.py`, 198) — `detect_languages`, `relevant_file_globs`,
  `is_third_party`, `walk_source_files`.
- **intent** (`intent.py`, 97) — `.codeguards/intent.json` save/load,
  `has_intent` gate, violation cross-reference, human summary.
- **planning** (`planning.py`, 304) — `create_architecture`, `load_architecture`,
  `create_plan`, `update_task`, `get_pending_tasks`; documents are YAML
  frontmatter (machine) + markdown body (human).
- **import_analyzer** (`import_analyzer.py`, 330) — regex-based, cross-language
  import analysis (no AST): domain extraction, layer-violation detection,
  structural health score.
- **fixes** (`fixes.py`, 178) — per-guard fix-suggestion generation.
- **config** (`config.py` 82 + `constants.py` 28) — `DEFAULTS` for 24 guards,
  `.codeguards.yaml` deep merge; `constants.py` is the threshold source.

---

## Constraints

Import rules below are verified against the code as of 2026-08-31 and mirrored
in the frontmatter `allowed_dependencies` (module-level):

- `server` may import: `guards`, `detectors`, `config`, `intent`, `planning`
  (intent/planning are lazy-imported inside handlers).
- `guards` package may import: `detectors`, `fixes`, `intent`, `plugins`,
  `import_analyzer`, `config` (the latter two via `structural.py`).
- `guards.generic` may import: `constants`, `fixes`, `plugins`
  (plugins used **only** for capability extractors — never language branching).
- `guards.structural` may import: `import_analyzer`, `detectors`, `config`.
- `plugins/*` import nothing in-project — self-contained, registered via
  `register(registry)`.
- `intent`, `planning`, `import_analyzer`, `fixes`, `detectors`, `config` are
  leaf modules — no in-project imports.
- Direction rule: transport → orchestration → checks → analysis / contracts /
  extension / config. No upward imports. New top-level modules should be
  leaves or depend only on lower layers.

---

## Quality Goals

Self-check thresholds effective for this repo (`.codeguards.yaml` overrides;
defaults in `constants.py`):

- `max_file_lines`: 800 prod / 600 test (orchestrator files are legitimately large)
- `max_function_lines`: 60 · `max_params`: 7 · `max_depth`: 15
- `fan_out`: 30 deps · `god_file`: 25 public items / 40 imports
- Disabled for self (rationale in `.codeguards.yaml` header):
  `forbidden_phrases`, `missing_docs`, `magic_numbers`, `structural_health`,
  `responsibility_clusters`, `duplicated_code`
- Test policy: every module has a mirrored test file (`tests/test_<module>.py`);
  15 test files at time of writing.
- Doc split (project convention, 2026-08-30): `docs/` is end-user only;
  internal planning/design docs live in `.planning/`.

---

## Out of scope — parked (do not implement without explicit go-ahead)

- **v0.2** (`design_v0.2.md`, status PARKED): ARCHITECTURE.md absorbing
  `intent.json` (`## Goal` write-once lifecycle with confirm-before-change);
  closed-set `quality_goals` schema; `.codeguards/checks.json` session-scoped
  checks; v0.1.0 migration tool.
- **v0.2 spec** (`V0.2_ARCHITECTURE_SPEC.md`): enforcement modes
  (fast / balanced / strict); full coaching-report shape (positives, streaks,
  skip-with-reason); guard test library outside the project
  (`~/.codeguard/tests/`, v0.3); central `ArchitectureContractGuard`.
- **Anti-drift authorization** (`ANTI_DRIFT.md`, design from 2026-06-30):
  MCP-held token + pre-commit hook + hash baseline protecting guard and
  architecture files from model-induced drift. Not implemented in this repo.
- Multi-user concurrent editing of ARCHITECTURE.md; a rule DSL / regex
  compiler; arbitrary new quality-goal vocabulary.

---

## Status

- **v0.1.0 shipped**: probe→plan gate, intent-based violation
  cross-referencing, plugin-less generic coverage, layer enforcement.
- This repo is **self-governed**: its own `.codeguards.yaml` tunes the guards
  against itself.
- **Known gap (honest note)**: `check_layer_enforcement` reads layer rules from
  `.codeguards.yaml` (`layer_enforcement.layers`), *not* from this file's
  frontmatter. Today `check_project` uses this frontmatter only for the module
  scope summary. "Frontmatter is the live contract" is the v0.2 design, not
  today's code.
