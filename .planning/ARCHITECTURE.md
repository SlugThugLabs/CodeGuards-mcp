+++
modules = ["analyzer", "contract", "error", "guards", "library", "server", "storage", "types", "util"]
layers = ["transport", "orchestration", "engine", "analysis", "storage", "foundation"]
enforce = ["source_limits", "no_unwrap", "no_debug_prints", "layer_dependencies"]

[allowed_dependencies]
server = ["guards", "contract", "analyzer", "storage", "library", "types", "error", "util"]
guards = ["analyzer", "storage", "contract", "library", "types", "error", "util"]
contract = ["storage", "library", "types", "error", "util"]
analyzer = ["types", "error", "util"]
storage = ["types", "error", "util"]
library = ["types", "error", "util"]
types = []
error = []
util = ["types", "error"]
+++

# CodeGuards Architecture & Workflow Integration

> **Product boundary:** CodeGuards is shipped as the single `codeguards-mcp` binary. It is an independent governance and test-runner engine that decouples test creation from normal agent coding workflows.
>
> In target user repositories, CodeGuards reads `.planning/ARCHITECTURE.md`. All guard test definitions live externally in `~/.slugthug/codeguards/tests/`.
>
> Target user repositories remain 100% clean — all test definitions, baselines, and execution state live in `~/.slugthug/codeguards/`.

---

## 1. Modular, DRY Guard-Test Taxonomy (`~/.slugthug/codeguards/tests/`)

Tests are grouped into strict, orthogonal categories. Every test has a unique namespaced identifier: `<category>/<name>` (e.g. `complexity/source-limits`, `rust/no-unwrap`, `hygiene/no-secrets`, `custom/<project>/<name>`).

```text
~/.slugthug/codeguards/tests/
├── catalog.json                              <-- Auto-indexed search cache (tags, summaries)
│
├── structural/                               <-- System architecture, layers, & project truth
│   ├── docs-drift.guard.json                 <-- Bidirectional module-map vs. filesystem reality
│   ├── module-map.guard.json                 <-- Declared module path existence
│   ├── layer-dependencies.guard.json         <-- DAG import hierarchy (allowed_dependencies)
│   └── manifest-dependencies.guard.json      <-- Cargo.toml / package.json dependency whitelist
│
├── complexity/                               <-- Code volume, function size, & coupling
│   ├── source-limits.guard.json              <-- Production code line limits (default: 400 lines)
│   ├── function-limits.guard.json            <-- Max lines per function / method
│   └── fanout-coupling.guard.json            <-- Responsibility clustering & fan-out bounds
│
├── hygiene/                                  <-- General code cleanliness & safety
│   ├── no-duplicates.guard.json              <-- Duplicate test names & commit subjects
│   ├── no-swallowed-errors.guard.json        <-- Discarded / unhandled error blocks
│   ├── no-secrets.guard.json                 <-- API keys, tokens, and credential patterns
│   └── no-debug-prints.guard.json            <-- Leftover println!, console.log, dbg!
│
├── quality/                                  <-- Implementation integrity & documentation
│   ├── no-stubs.guard.json                   <-- Rejects TODO, FIXME, unimplemented!() in prod
│   ├── required-docstrings.guard.json        <-- Documentation coverage on public APIs
│   └── test-isolation.guard.json             <-- Tests isolated from production artifacts
│
├── languages/                                <-- Language-Specific Invariants (Pure Rust Tokenizers)
│   ├── rust/
│   │   ├── no-unwrap.guard.json              <-- Zero .unwrap() / .expect() in prod paths
│   │   ├── tracing-instrument.guard.json     <-- #[tracing::instrument] on public entry points
│   │   ├── unsafe-policy.guard.json          <-- Strict forbid(unsafe_code) or bounded usage
│   │   ├── runtime-leak.guard.json           <-- Async (tokio/wasmtime/rayon) leaks into pure domain
│   │   ├── concurrency-ownership.guard.json  <-- Concurrency primitives (crossbeam/arc-swap) bounds
│   │   ├── error-mapping.guard.json          <-- Typed error propagation across layer boundaries
│   │   ├── edition-lock.guard.json           <-- Toolchain / Rust edition integrity
│   │   └── public-api-bounds.guard.json      <-- Accidental pub fn / pub struct exports
│   └── python/
│       ├── type-annotations.guard.json       <-- Required type hints on function signatures
│       └── no-wildcard-imports.guard.json    <-- Disallows `from module import *`
│
└── custom/                                   <-- Project-Specific / Dynamic Custom Guard Tests
    └── <namespace>/
        └── <custom-test>.guard.json          <-- Interactive AI-authored guard tests
```

---

## 2. Dynamic Custom Guard Test Creation (`create_guard_test`)

When a project has unique semantic or business invariants that standard guards don't cover:
1. **Agent identifies need during architecture discussion:** (e.g. *"All payment endpoints in `src/billing/` must require the `AuditLog` trait"*).
2. **Agent calls `create_guard_test`:**
   * Defines targeted token/pattern rules, path scopes, severity, and remediation advice.
   * Saves to `~/.slugthug/codeguards/tests/custom/<namespace>/<test_name>.guard.json`.
3. **Persisted Globally:** The custom test is registered in `catalog.json` and immediately available across projects without bloating the target repository.

---

## 3. User-Authorized Exception Token Mechanism (`add_exception`)

### The Solution: Cryptographic / Hash-Verified User Tokens
1. **Agent Hits a Boundary:** If an exception is genuinely required, the check fails with exact instructions:
   ```text
   [CODEGUARD-BLOCKED] complexity/source-limits
     File: src/server/transport.rs (450 code lines > max 400)
     Action Required: Ask user to run `codeguards-mcp exception add src/server/transport.rs complexity/source-limits --reason="..."`
   ```
2. **User Grants the Exception:**
   * Via CLI: `codeguards-mcp exception add <file> <guard> --reason="<why>"`
   * Or via User Slash Command / MCP tool: `/codeguards_add-exception` / `add_exception`
3. **Storage & Verification:**
   * Exceptions are stored securely in `~/.slugthug/codeguards/projects/<project_id>/exceptions.json`.
   * Verified by matching token signature against file path and guard rule.
   * Stale tokens (where rule is no longer violated) are flagged for pruning via `codeguards-mcp exception prune`.

---

## 4. Pure-Rust Zero-Dependency Execution & Decoupled Triggers

* **Pure Rust Tokenizer:** Fast state-machine scanner (strips comments, docstrings, char/string literals) without heavy AST or Tree-sitter dependencies. Sub-5ms latency.
* **Manifest Scanner:** Validates `Cargo.toml` dependency whitelists and forbids illegal crate coupling.
* **Decoupled Workflow:**
  * **Planning:** Agent + User use MCP authoring tools (`validate_architecture`, `create_guard_test`).
  * **Coding:** Agent writes normal code in `src/`. Zero CodeGuards MCP overhead.
  * **Enforcement:** Git pre-commit hook runs `codeguards-mcp check --diff`.
