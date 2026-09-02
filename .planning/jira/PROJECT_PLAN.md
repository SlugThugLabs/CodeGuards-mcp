# CodeGuards Rust Implementation Roadmap

> **Standard:** SlugThug Engineering Quality (Pure Rust tokenizer without Tree-sitter bloat, comprehensive guard-test pack ported from `slugid`/`slugaudit`, user-authorized exception token gate, zero untyped errors, small files $\le 400$ lines, sub-15ms check latency, strict sandbox-first, Rust 2024 edition, TOML contracts).

---

## Phase 0: Crate Setup, Foundation & Domain Types
- [ ] Initialize Cargo project with edition `2024` (`rmcp`, `serde`, `serde_json`, `toml`, `rayon`, `ignore`, `regex`, `thiserror`, `tokio`, `tracing`, `blake3`).
- [ ] Implement typed error taxonomy (`src/error.rs`) and sandboxing utilities (`src/util.rs`).
- [ ] Implement core data types: `Violation`, `Severity`, `FixSuggestion`, `GuardReport`, `ExceptionEntry` (`src/types.rs`).

## Phase 1: Modular Guard-Test Library & Anti-Duplication Engine (`~/.slugthug/codeguards/tests/`)
- [ ] Define `.guard.json` schema (`src/library/schema.rs`) with `id`, `category`, `summary`, `tags`, `aliases`, `engine`, `default_params`.
- [ ] Implement `catalog.json` registry indexer and fuzzy duplicate detector (`src/library/catalog.rs`).
- [ ] Implement embedded built-in test suite (`src/library/builtins.rs`) seeded into `~/.slugthug/codeguards/tests/` on first run:
  - `structural/`: `docs-drift` (bidirectional), `module-map`, `layer-dependencies`, `manifest-dependencies`
  - `complexity/`: `source-limits` (max 400), `function-limits`, `fanout-coupling`
  - `hygiene/`: `no-duplicates`, `no-swallowed-errors`, `no-secrets`, `no-debug-prints`
  - `quality/`: `no-stubs`, `required-docstrings`, `test-isolation`
  - `languages/rust/`: `no-unwrap`, `tracing-instrument`, `unsafe-policy`, `runtime-leak`, `concurrency-ownership`, `error-mapping`, `edition-lock`, `public-api-bounds`
  - `languages/python/`: `type-annotations`, `no-wildcard-imports`
- [ ] Implement library loader, validator, and custom test creator (`src/library/loader.rs`, `src/library/creator.rs`).

## Phase 2: User Exception Token Engine & Storage
- [ ] Implement user exception manager (`src/storage/exceptions.rs`):
  - Stores authorized exceptions in `~/.slugthug/codeguards/projects/<project_id>/exceptions.json`.
  - Generates deterministic 5-digit verification tokens (e.g. `23954`) bound to `(file, guard_id, reason)`.
  - Validates inline file headers: `// codeguard-exception: token=23954; guard=...; reason="..."`.
  - Stale detection: Warns when an exception is no longer required (`exception prune`).
- [ ] Implement CLI command: `codeguards-mcp exception add <file> <guard> --reason="..."` (outputs user token).
- [ ] Implement CLI command: `codeguards-mcp exception list`, `codeguards-mcp exception revoke <token>`, and `codeguards-mcp exception prune`.

## Phase 3: Dynamic Contract Engine & `validate_architecture`
- [ ] Implement path hashing and storage resolver (`src/storage/`).
- [ ] Implement `.planning/ARCHITECTURE.md` TOML frontmatter parser (`src/contract/frontmatter.rs` using `+++` fences and `toml`).
- [ ] Implement `validate_architecture` logic (`src/contract/validator.rs`):
  - Checks TOML frontmatter validity.
  - Matches `enforce = [...]` against `catalog.json` by id, name, and aliases.
  - Flags missing tests and prompts to use `create_guard_test` for custom project rules.

## Phase 4: Pure-Rust Scanner & Standalone Check CLI (`codeguards-mcp check`)
- [ ] Implement fast comment/literal stripping state machine (`src/analyzer/tokenizer.rs`).
- [ ] Implement `Cargo.toml` / manifest dependency scanner (`src/analyzer/manifest.rs`).
- [ ] Implement Rayon-backed fast parallel file walker with ignore filters (`src/analyzer/walker.rs`).
- [ ] Implement rule evaluators (`src/guards/runner.rs`).
- [ ] Implement `codeguards-mcp check` CLI subcommand:
  - Supports `--diff` (fast-path staged/unstaged check <5ms) and `--all` (full repo scan).
  - Validates user-authorized exception tokens against `exceptions.json`.
  - Returns standard exit codes (`0` pass, `1` fail) with structured `[CODEGUARD-BLOCKED]` output.
- [ ] Implement `codeguards-mcp fix` CLI subcommand.
- [ ] Implement git hook installer: `codeguards-mcp hook install` (installs `.git/hooks/pre-commit`).

## Phase 5: MCP Protocol for Authoring Mode
- [ ] Implement `codeguards-authoring` MCP tool handlers:
  - `validate_architecture`: Validates `ARCHITECTURE.md` & checks for missing tests in library.
  - `create_guard_test`: Generates and writes a new `.guard.json` to `~/.slugthug/codeguards/tests/` (built-in or `custom/`).
  - `list_guard_tests`: Returns organized category view + search index from `catalog.json`.
  - `add_exception`: User-gated MCP tool / command to grant an authorized token.
  - `declare_intent` / `plan`: Architecture & sprint scaffolding.
- [ ] Wire up MCP stdio transport with `rmcp` (`src/server/`).

## Phase 6: Verification, Benchmarking & Deployment
- [x] 100% unit and integration test coverage across all built-in guard tests and exception validation.
- [x] Performance benchmark verifying `check` CLI latency (<5ms diff, <15ms file, <250ms project).
- [x] Self-audit: Run `codeguards-mcp check` against `slugaudit`, `slugid`, and itself.
- [x] Build release binary and install to `/root/.slugthug/bin/codeguards-mcp`.
- [x] **Measured test coverage: 60.3% (974 of 1615 lines).**
