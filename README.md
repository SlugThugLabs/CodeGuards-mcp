# CodeGuards (`codeguards-mcp`)

> Continuous code governance and architectural enforcement for AI agents and developers.

CodeGuards enforces architectural boundaries declared in `.planning/ARCHITECTURE.md` using modular, reusable tests stored outside target repositories in `~/.slugthug/codeguards/tests/`.

---

## AI Agent Integration (Hermes, Claude Code, OpenCode, Codex)

CodeGuards is built to be driven directly by AI agents through MCP tools and CLI execution.

### Registering the MCP Server

```bash
# Hermes Agent
hermes mcp add codeguards --command "codeguards-mcp serve"
```

For clients configuring via `mcp.json` (OpenCode, Claude Code, Cursor):
```json
{
  "mcpServers": {
    "codeguards": {
      "command": "codeguards-mcp",
      "args": ["serve"]
    }
  }
}
```

---

## Agent Workflow

### 1. Authoring & Architecture Setup
During conversational planning, the agent creates or updates `.planning/ARCHITECTURE.md` with TOML frontmatter:

```toml
+++
modules = ["server", "domain", "storage"]
layers = ["transport", "domain", "storage"]
enforce = [
  "source_limits",
  "no_unwrap",
  "no_debug_prints",
  "layer_dependencies",
]

[allowed_dependencies]
server = ["domain"]
domain = ["storage"]
storage = []

[guard_settings.source_limits]
max_lines = 400
+++

# Architecture & System Design
```

The agent validates the contract via MCP:
* **`validate_architecture`**: Checks that all declared `enforce` rules exist in `~/.slugthug/codeguards/tests/`.
* **`list_guard_tests`**: Inspects available guard tests across categories (`structural`, `complexity`, `hygiene`, `languages/rust`).
* **`create_guard_test`**: Authors a new reusable test definition when a project requires custom invariants.

---

## 2. Enforcement & Coding Loop

During normal coding, the agent runs standard implementation tasks. Verification happens via CLI triggers:

```bash
# Check modified / staged files in active git diff (<5ms)
codeguards-mcp check

# Run full project scan across all files
codeguards-mcp check --all

# Validate architecture contract
codeguards-mcp validate
```

### Git Hook Integration
Install into `.git/hooks/pre-commit`:
```bash
#!/usr/bin/env bash
codeguards-mcp check
```

---

## 3. User-Authorized Exceptions

When a file genuinely requires an exemption (e.g. file size limit for generated code), agents cannot self-authorize. The user grants an exception token:

```bash
# User grants exception
codeguards-mcp exception add src/server/transport.rs complexity/source-limits --reason="Combined transport scaffolding"

# List active exceptions
codeguards-mcp exception list

# Revoke an exception
codeguards-mcp exception revoke <token>
```

The file adds the verified header:
```rust
// codeguard-exception: token=23954; guard=complexity/source-limits; reason="Combined transport scaffolding"
```

---

## Build & Install

```bash
cargo build --release
cp target/release/codeguards-mcp /root/.slugthug/bin/codeguards-mcp
```
