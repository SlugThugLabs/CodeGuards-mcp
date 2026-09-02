# CodeGuards (`codeguards-mcp`)

> Continuous code governance and architectural enforcement for AI coding agents and human engineering teams.

CodeGuards turns your `.planning/ARCHITECTURE.md` into machine-enforceable boundary contracts, runs sub-15ms pre-commit checks, and prevents AI coding agents from drifting or taking shortcuts.

---

## 1. Installation

### Quick Install (Single Binary)
```bash
# Clone and build
git clone https://github.com/SlugThugLabs/CodeGuards-mcp.git
cd CodeGuards-mcp
cargo build --release

# Copy to your PATH
cp target/release/codeguards-mcp /usr/local/bin/   # or ~/.slugthug/bin/
```

---

## 2. Using CodeGuards in Any Project

### Step 1: Initialize Your Project Architecture
Inside any project repository, create `.planning/ARCHITECTURE.md` (or run `slugplan init`).

Add your architecture rules in standard TOML frontmatter between `+++` fences:

```toml
+++
modules = ["server", "domain", "storage"]
layers = ["transport", "domain", "storage"]
enforce = [
  "source_limits",        # Max 400 code lines per file
  "no_unwrap",            # No bare .unwrap() / .expect() in production
  "no_debug_prints",      # No leftover dbg! / console.log
  "layer_dependencies",   # Enforces DAG import boundaries below
]

[allowed_dependencies]
server = ["domain"]
domain = ["storage"]
storage = []

[guard_settings.source_limits]
max_lines = 300
+++

# My Project Architecture
System design notes, component docs, and data flows go here.
```

---

### Step 2: Validate the Contract
Make sure your architecture rules are valid and that all declared guard tests exist:

```bash
codeguards-mcp validate
```

If a required guard test doesn't exist yet, CodeGuards will let you know so your planning agent can create it.

---

### Step 3: Run the Guard Checks (CLI & Git Pre-Commit)

#### Manual Check:
```bash
# Check modified / staged files in active git diff (Instant: ~1ms)
codeguards-mcp check

# Run full project scan across all files
codeguards-mcp check --all
```

#### Automatic Git Pre-Commit Hook:
Drop this into `.git/hooks/pre-commit` and `chmod +x .git/hooks/pre-commit`:
```bash
#!/usr/bin/env bash
codeguards-mcp check
```
Now, every time you or an AI agent runs `git commit`, CodeGuards validates the changes. If a rule is breached, the commit is blocked with an exact file citation and fix suggestion.

---

## 3. User-Authorized Exceptions (`codeguards-mcp exception`)

If a file genuinely needs to exceed a line limit or needs an architectural exemption, an AI agent **cannot** bypass the gate on its own. It must ask you for an authorization token.

### 1. You Authorize the Exception:
```bash
codeguards-mcp exception add src/server/transport.rs complexity/source-limits --reason="Combined transport scaffolding"
```
Output:
```text
[CODEGUARD-SUCCESS] Exception authorized.
  Token:   23954
  File:    src/server/transport.rs
  Guard:   complexity/source-limits
  Header:  // codeguard-exception: token=23954; guard=complexity/source-limits; reason="Combined transport scaffolding"
```

### 2. Add the Header to the File:
```rust
// codeguard-exception: token=23954; guard=complexity/source-limits; reason="Combined transport scaffolding"
```
CodeGuards verifies the token using a private HMAC salt. If valid, the file passes.

---

## 4. Connecting to AI Agents via MCP

CodeGuards can run as an MCP server for planning agents (Claude Code, OpenCode, Codex, Hermes):

### Registering with Clients

#### Hermes Agent:
```bash
hermes mcp add codeguards --command "codeguards-mcp serve"
```

#### Claude Code / OpenCode (`mcp.json`):
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

### Tools Exposed to Planning Agents:
* **`validate_architecture`**: Validates `.planning/ARCHITECTURE.md` against disk reality and the guard test library.
* **`list_guard_tests`**: Catalogs all available modular tests in `~/.slugthug/codeguards/tests/`.
* **`create_guard_test`**: Interactively authors and saves new reusable guard test definitions.
* **`add_exception`**: User-gated tool to authorize exception tokens.
