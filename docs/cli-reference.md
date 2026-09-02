# CLI Reference

CodeGuards provides a complete command-line interface for all governance operations. All commands follow the pattern:

```bash
codeguards-mcp <command> [options]
```

## Global Options

- `--help`, `-h`: Show help message
- `--version`: Show version information

## Commands

### `menu` — Interactive Setup

Launches an interactive terminal menu for initial setup and management.

```bash
codeguards-mcp menu
```

**Menu Options:**
1. **Install the binary** — Copies the running binary to `~/.slugthug/bin/codeguards-mcp`
2. **Connect to an AI agent** — Registers CodeGuards with Hermes, Claude Code, Grok, or Codex
3. **Add to other MCP clients** — Shows configuration snippet for OpenCode, Cursor, Zed, etc.
4. **Inspect guard-test library** — Lists all available modular tests
5. **Run MCP server now** — Starts the server in the current terminal
6. **Exit**

### `check` — Run Architectural Guard Checks

Validates source code against rules defined in `.planning/ARCHITECTURE.md`.

```bash
# Check only modified/unstaged files (default, fast)
codeguards-mcp check

# Check all source files in the project
codeguards-mcp check --all

# Check a different project directory
codeguards-mcp check --root /path/to/project
```

**Exit Codes:**
- `0`: All checks passed
- `1`: Architecture violations detected
- `2`: Configuration or setup error

**Output Format:**
- `[CODEGUARD-PASS]` — Success message with timing
- `[CODEGUARD-BLOCKED]` — Detailed violation report with file, line, rule, and fix suggestions

### `validate` — Validate Architecture Contract

Checks if `.planning/ARCHITECTURE.md` is valid and all referenced guard tests exist.

```bash
# Validate current directory
codeguards-mcp validate

# Validate specific project
codeguards-mcp validate --root /path/to/project
```

**Exit Codes:**
- `0`: Architecture contract is valid
- `1`: Missing guard tests or invalid TOML
- `2`: File system error

### `exception` — Manage User Exceptions

Handles user-authorized exception tokens for approved rule violations.

#### `exception add`
Authorizes an exception for a specific file and guard rule.

```bash
codeguards-mcp exception add <file> <guard> --reason="Justification"
```

**Example:**
```bash
codeguards-mcp exception add src/server.rs complexity/source-limits --reason="Legacy module requires refactoring"
```

**Output:** Generates a 5-digit verification token and shows the exact header comment to add to the file.

#### `exception list`
Lists all active exceptions for the current project.

```bash
codeguards-mcp exception list
```

#### `exception revoke`
Revokes an exception by its token.

```bash
codeguards-mcp exception revoke <token>
```

### `serve` — Start MCP Server

Launches the Model Context Protocol server for integration with AI agents.

```bash
# Start stdio server (default, for AI agent integration)
codeguards-mcp serve

# Start TCP server on specific port
codeguards-mcp serve --port 8080
```

**Note:** This command is typically called automatically by your AI agent. Use `codeguards-mcp menu` option 5 to run it manually for testing.

## Environment Variables

- `SLUGTHUG_HOME`: Override the default `~/.slugthug` directory
- `RUST_LOG`: Enable debug logging (e.g., `RUST_LOG=debug`)

## Common Workflows

### Git Pre-commit Hook
```bash
# Add to .git/hooks/pre-commit
#!/bin/sh
codeguards-mcp check --diff
```

### CI Pipeline
```yaml
# GitHub Actions example
- name: Run CodeGuards
  run: codeguards-mcp check --all
```

### AI Agent Integration

Your AI agent automatically uses these MCP tools during development:

#### `create_guard_test`
When you discuss architecture requirements like *"All payment handlers must implement AuditLog"*, your agent will:

1. **Propose a guard definition**:
   ```json
   {
     "name": "audit-log-required",
     "category": "custom/billing", 
     "summary": "Payment handlers must implement AuditLog trait",
     "tags": ["billing", "compliance"],
     "engine": "pattern-match",
     "remediation": "Add `impl AuditLog for YourHandler`"
   }
   ```

2. **Call the MCP tool**:
   ```bash
   # Agent executes this internally
   mcp_call create_guard_test '{"name":"audit-log-required","category":"custom/billing",...}'
   ```

3. **Reference in ARCHITECTURE.md**:
   ```toml
   enforce = ["custom/billing/audit-log-required"]
   ```

#### `add_exception`  
When code violates a rule but needs an exception, your agent will:

1. **Request user approval**: *"This legacy module needs an exception for source-limits"*
2. **Call MCP tool with user reason**:  
   ```bash
   mcp_call add_exception '{"file":"src/legacy.rs","guard_id":"complexity/source-limits","reason":"Legacy module"}'
   ```
3. **Add header comment** with the generated token

#### `validate_architecture`
During planning, your agent validates that all referenced guards exist:

```bash
# Agent runs this to check ARCHITECTURE.md
mcp_call validate_architecture '{"project_path":"/current/project"}'
```

If guards are missing, it offers to create them via `create_guard_test`.