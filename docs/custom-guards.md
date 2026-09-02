# Custom Guard-Test Authoring

When your project has unique architectural or business rules not covered by built-in guards, you can create custom guard tests. These are stored globally in `~/.slugthug/codeguards/tests/custom/` and immediately available across all projects.

## When to Create Custom Guards

Create a custom guard when you need to enforce:

- **Business-specific patterns**: "All payment handlers must implement `AuditLog` trait"
- **Project-specific conventions**: "API routes must follow `/v1/{service}/{resource}` pattern"  
- **Domain-specific invariants**: "Database models must have `created_at` and `updated_at` fields"
- **Security policies**: "JWT tokens must be validated within 5 minutes of issuance"

## Guard Test Structure

Custom guards use the `.guard.json` schema with these required fields:

```json
{
  "id": "custom/billing/audit-log-required",
  "name": "audit-log-required",
  "category": "custom/billing",
  "version": "1.0.0",
  "summary": "All payment handlers must implement AuditLog trait",
  "tags": ["billing", "security", "audit"],
  "aliases": ["payment-audit", "billing-logs"],
  "engine": "pattern-match",
  "default_params": {
    "pattern": "impl AuditLog for.*PaymentHandler",
    "scope": "src/billing/**/*.rs"
  },
  "remediation": "Add `impl AuditLog for YourPaymentHandler` to satisfy audit requirements."
}
```

### Field Reference

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique namespaced identifier (`custom/<namespace>/<name>`) |
| `name` | Yes | Short descriptive name |
| `category` | Yes | Classification path (`custom/<your-project>`) |
| `version` | Yes | Semantic version (1.0.0) |
| `summary` | Yes | One-sentence explanation of what this guard checks |
| `tags` | No | Search tags for discovery |
| `aliases` | No | Alternative names users might search for |
| `engine` | Yes | Execution engine (`pattern-match`, `import-graph`, `ast-query`, `custom`) |
| `default_params` | Yes | Engine-specific configuration parameters |
| `remediation` | Yes | Actionable fix instruction for violations |

## Creating Custom Guards

### Method 1: AI Agent Creation (Recommended)

During architecture discussions, your AI agent can automatically create custom guards:

1. **You state a requirement**:  
   *"All payment handlers in src/billing/ must implement the AuditLog trait for SOX compliance"*

2. **Agent proposes a guard**:  
   The agent analyzes your request and proposes a complete `.guard.json` definition with appropriate patterns, scope, and remediation.

3. **You approve**:  
   You confirm the guard looks correct: *"Yes, that covers what I need"*

4. **Agent creates via MCP**:  
   The agent calls the `create_guard_test` MCP tool with your approved definition:
   ```json
   {
     "name": "payment-audit-log",
     "category": "custom/billing",
     "summary": "Payment handlers must implement AuditLog trait for SOX compliance",
     "tags": ["billing", "compliance", "audit", "sox"],
     "engine": "pattern-match",
     "default_params": {
       "pattern": "impl AuditLog for.*PaymentHandler",
       "scope": "src/billing/**/*.rs"
     },
     "remediation": "Add `impl AuditLog for YourPaymentHandler` to satisfy SOX compliance requirements."
   }
   ```

5. **Guard is immediately available**:  
   The new guard appears in your catalog and can be referenced in `.planning/ARCHITECTURE.md`:
   ```toml
   enforce = ["custom/billing/payment-audit-log"]
   ```

### MCP Tool Interface

The `create_guard_test` MCP tool accepts these parameters:

| Parameter | Required | Description |
|-----------|----------|-------------|
| `name` | Yes | Short name (e.g., "audit-log-required") |
| `category` | Yes | Classification path (`custom/<your-namespace>`) |
| `summary` | Yes | One-sentence description |
| `tags` | No | Search tags for discovery |
| `aliases` | No | Alternative names |
| `engine` | Yes | Execution engine (`pattern-match`, `import-graph`, `custom`) |
| `remediation` | Yes | Actionable fix instruction |

**Example MCP Call**:
```bash
# This is what your AI agent executes internally
mcp_call create_guard_test '{
  "name": "audit-log-required",
  "category": "custom/billing",
  "summary": "Payment handlers need AuditLog",
  "tags": ["billing", "compliance"],
  "engine": "pattern-match", 
  "remediation": "Implement AuditLog trait"
}'
```

### Method 2: Manual Creation

For advanced users who prefer manual control:

1. **Create directory**:
   ```bash
   mkdir -p ~/.slugthug/codeguards/tests/custom/your-project
   ```

2. **Create guard file**:
   ```bash
   nano ~/.slugthug/codeguards/tests/custom/your-project/your-rule.guard.json
   ```

3. **Add valid JSON** using the schema above

4. **Refresh catalog**:
   ```bash
   codeguards-mcp validate --root /path/to/project
   ```

### Method 3: CLI Creation

Use the command-line interface to create guards interactively:

```bash
# This will prompt for all required fields
codeguards-mcp create-guard-test --interactive
```

> **Note**: The `create-guard-test` CLI command is planned for future releases. Currently, use AI agent creation or manual methods.

## Engine Types

### `pattern-match`
Searches source files for regex patterns outside comments/strings.

**Parameters:**
- `pattern`: Regular expression to match
- `scope`: File glob pattern (e.g., `src/**/*.rs`)
- `case_sensitive`: Boolean (default: true)

### `import-graph`  
Validates module dependency graphs against allowed patterns.

**Parameters:**
- `allowed_from`: Source module patterns
- `allowed_to`: Destination module patterns
- `forbidden`: Explicitly blocked import patterns

### `custom`
Runs user-provided Rust code (advanced).

**Parameters:**
- `module`: Rust module path containing the check function
- `function`: Function name to call
- `args`: Arguments to pass to the function

## Best Practices

### Naming Conventions
- Use descriptive names: `audit-log-required` not `check1`
- Namespace properly: `custom/billing/...` not `custom/my-stuff/...`
- Version appropriately: Increment when behavior changes

### Pattern Design
- **Specific**: Target exact violation patterns, not general code
- **Actionable**: Provide clear remediation instructions
- **Performant**: Avoid expensive regex patterns or deep AST traversal

### Testing Your Custom Guard

1. **Create test file** with intentional violation
2. **Run validation**:
   ```bash
   codeguards-mcp check --all --root /path/to/test-project
   ```
3. **Verify output**: Check that violation message and remediation are clear
4. **Test exception**: Ensure `exception add` works correctly

## Example: Payment Handler Audit Log

**Problem**: All payment handlers must implement the `AuditLog` trait for compliance.

**Solution**:
```json
{
  "id": "custom/billing/payment-audit-log",
  "name": "payment-audit-log",
  "category": "custom/billing", 
  "version": "1.0.0",
  "summary": "Payment handlers must implement AuditLog trait for SOX compliance",
  "tags": ["billing", "compliance", "audit", "sox"],
  "aliases": ["billing-audit", "payment-compliance"],
  "engine": "pattern-match",
  "default_params": {
    "pattern": "struct ([A-Za-z0-9_]+Handler).*?impl.*PaymentHandler.*?impl AuditLog for \\1",
    "scope": "src/billing/handlers/**/*.rs",
    "case_sensitive": true
  },
  "remediation": "Add `impl AuditLog for YourHandlerName` to satisfy SOX compliance requirements. See docs/compliance/audit-logging.md for implementation details."
}
```

This guard ensures every payment handler struct that implements `PaymentHandler` also implements `AuditLog`, with a clear path to resolution.

## Managing Custom Guards

### Listing Custom Guards
```bash
codeguards-mcp validate --root /path/to/project
# Shows all available guards including custom ones
```

### Updating Custom Guards
1. Edit the `.guard.json` file
2. Update the `version` field
3. Run validation to refresh the catalog

### Deleting Custom Guards
```bash
rm ~/.slugthug/codeguards/tests/custom/namespace/guard-name.guard.json
# Catalog automatically updates on next validation
```

Custom guards become part of your global governance library and can be reused across all projects without duplication.