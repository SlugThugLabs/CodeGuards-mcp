# Architecture & Workflow Integration

CodeGuards enforces architectural boundaries through a decoupled governance model that separates **test creation** from **normal coding workflows**. This ensures AI agents can code freely while maintaining strict architectural integrity.

## Core Philosophy

> **"Agents code normally; guards enforce automatically."**

- **Coding Phase**: AI agents write standard code in `src/` without any CodeGuards overhead
- **Enforcement Phase**: Git hooks or CI pipelines automatically run `codeguards-mcp check`
- **Planning Phase**: Agents and users collaborate on architecture using MCP authoring tools

## Project Structure

CodeGuards follows the **SlugPlan** convention with strict separation of concerns:

```
your-project/
├── src/                    # Pure product code (agents work here)
├── .planning/             # In-repo Confluence/Jira (architecture truth)
│   └── ARCHITECTURE.md    # TOML frontmatter defines allowed layers, modules, and rules
├── docs/                  # Public documentation (this guide lives here)
└── .git/hooks/            # Pre-commit hook runs codeguards-mcp check
```

### `.planning/ARCHITECTURE.md`

This file is the single source of truth for your project's architectural constraints:

```toml
+++
modules = ["analyzer", "server", "guards"]
layers = ["transport", "orchestration", "engine", "foundation"]

[allowed_dependencies]
server = ["guards", "analyzer", "foundation"]
guards = ["analyzer", "foundation"]
analyzer = ["foundation"]

enforce = [
  "complexity/source-limits",
  "languages/rust/no-unwrap", 
  "hygiene/no-debug-prints",
  "structural/layer-dependencies"
]
+++

# Your architecture documentation goes here...
```

## Guard-Test Library

All guard tests live externally in `~/.slugthug/codeguards/tests/`, keeping your project repositories clean:

```
~/.slugthug/codeguards/tests/
├── catalog.json              # Auto-generated search index
├── structural/               # Architecture & structure rules
├── complexity/               # Code volume & coupling limits  
├── hygiene/                  # General code cleanliness
├── quality/                  # Implementation integrity
└── languages/rust/           # Rust-specific invariants
```

### Built-in Guard Categories

| Category | Purpose | Examples |
|----------|---------|----------|
| **structural** | System architecture | `docs-drift`, `layer-dependencies`, `manifest-dependencies` |
| **complexity** | Code volume limits | `source-limits` (400 lines), `function-limits` |
| **hygiene** | General cleanliness | `no-duplicates`, `no-secrets`, `no-debug-prints` |
| **quality** | Implementation integrity | `no-stubs`, `required-docstrings`, `test-isolation` |
| **languages/rust** | Rust-specific rules | `no-unwrap`, `tracing-instrument`, `unsafe-policy` |

## Exception Token Mechanism

When a genuine exception is needed, CodeGuards uses cryptographic tokens to prevent agent self-authorization:

1. **Agent hits boundary**: Check fails with specific instructions
2. **User grants exception**: Runs `codeguards-mcp exception add ...`
3. **Token generated**: 5-digit code bound to `(file, guard, reason)`
4. **Verification**: Header comment validates against user's private key

```rust
// codeguard-exception: token=40721; guard=languages/rust/no-unwrap; reason="Legacy module"
```

## Integration Points

### Git Pre-commit Hook
Automatically validates staged changes before commit:

```bash
#!/bin/sh
codeguards-mcp check --diff
```

### AI Agent Workflow
Your AI agent automatically integrates with CodeGuards during:

- **Planning**: Validates `ARCHITECTURE.md` and creates custom guard tests
- **Coding**: Requests exceptions when needed (requires user approval)
- **Review**: Runs final validation before PR submission

### CI/CD Pipeline
Full repository validation in continuous integration:

```yaml
- name: Architectural Governance
  run: codeguards-mcp check --all
```

## Performance Characteristics

- **Tokenizer**: 58.2 µs for 205-line file (284 lines/ms)
- **Diff Check**: <5ms for typical commits
- **Full Scan**: <250ms for 200+ file repositories
- **Memory**: <50MB peak usage

CodeGuards is designed to be imperceptible in development workflows while providing comprehensive architectural enforcement.