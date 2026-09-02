# Integration Guides

## Git Pre-commit Hooks

Automatically validate code before every commit.

### Installation

Create the hook file:
```bash
mkdir -p .git/hooks
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/sh
# CodeGuards pre-commit hook
# Validates staged changes against architectural rules

# Skip if no CodeGuards binary found
if ! command -v codeguards-mcp >/dev/null 2>&1; then
    echo "CodeGuards not found. Skipping architectural validation."
    exit 0
fi

# Run diff-based check (fast)
if ! codeguards-mcp check --diff; then
    echo ""
    echo "Architectural violations detected!"
    echo "Fix issues or add approved exceptions before committing."
    echo ""
    exit 1
fi
EOF

chmod +x .git/hooks/pre-commit
```

### Usage

- **Normal workflow**: `git add . && git commit` — automatically validates
- **Skip validation**: `git commit --no-verify` (use sparingly)
- **Manual run**: `.git/hooks/pre-commit`

### Best Practices

- Use `--diff` for speed (<5ms typical)
- Keep hooks idempotent and fast
- Provide clear error messages for developers

## AI Agent Integration

Connect CodeGuards to your AI coding assistant.

### Hermes Agent

**Register the server**:
```bash
hermes mcp add codeguards -- ~/.slugthug/bin/codeguards-mcp serve
```

**Verify registration**:
```bash
hermes mcp list
```

**Remove if needed**:
```bash
hermes mcp remove codeguards
```

### Claude Code

**Register the server**:
```bash
claude mcp add codeguards -- ~/.slugthug/bin/codeguards-mcp serve
```

### Grok / Codex

Use the same pattern:
```bash
grok mcp add codeguards -- ~/.slugthug/bin/codeguards-mcp serve
codex mcp add codeguards -- ~/.slugthug/bin/codeguards-mcp serve
```

### Other MCP Clients (OpenCode, Cursor, Zed)

Add this configuration to your client's `.mcp.json`:

```json
{
  "mcpServers": {
    "codeguards": {
      "command": "~/.slugthug/bin/codeguards-mcp",
      "args": ["serve"]
    }
  }
}
```

## CI/CD Pipeline Integration

### GitHub Actions

```yaml
name: Architectural Governance
on: [push, pull_request]

jobs:
  codeguards:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install CodeGuards
        run: |
          git clone https://github.com/SlugThugLabs/CodeGuards-mcp.git
          cd CodeGuards-mcp
          cargo build --release
          mkdir -p ~/.slugthug/bin
          cp target/release/codeguards-mcp ~/.slugthug/bin/
          echo "$HOME/.slugthug/bin" >> $GITHUB_PATH
      
      - name: Run Full Validation
        run: codeguards-mcp check --all
```

### GitLab CI

```yaml
codeguards:
  image: rust:latest
  script:
    - git clone https://github.com/SlugThugLabs/CodeGuards-mcp.git
    - cd CodeGuards-mcp && cargo build --release
    - cp target/release/codeguards-mcp ~/.slugthug/bin/
    - export PATH="$HOME/.slugthug/bin:$PATH"
    - codeguards-mcp check --all
```

### Jenkins

```groovy
pipeline {
    agent any
    stages {
        stage('CodeGuards') {
            steps {
                sh '''
                    if [ ! -f ~/.slugthug/bin/codeguards-mcp ]; then
                        git clone https://github.com/SlugThugLabs/CodeGuards-mcp.git
                        cd CodeGuards-mcp
                        cargo build --release
                        mkdir -p ~/.slugthug/bin
                        cp target/release/codeguards-mcp ~/.slugthug/bin/
                    fi
                    ~/.slugthug/bin/codeguards-mcp check --all
                '''
            }
        }
    }
}
```

## IDE Integration

### VS Code

1. Install the **MCP Client** extension
2. Add to `settings.json`:
```json
{
  "mcp.servers": {
    "codeguards": {
      "command": "~/.slugthug/bin/codeguards-mcp",
      "args": ["serve"]
    }
  }
}
```

### Vim/Neovim

Use with **mcp.nvim**:
```lua
require('mcp').setup({
  servers = {
    codeguards = {
      command = '~/.slugthug/bin/codeguards-mcp',
      args = {'serve'}
    }
  }
})
```

## Advanced Workflows

### Monorepo Support

For projects with multiple packages:

```bash
# Validate specific package
codeguards-mcp validate --root ./packages/frontend

# Check all packages
for pkg in packages/*; do
  echo "Validating $pkg..."
  codeguards-mcp check --all --root "$pkg" || exit 1
done
```

### Docker Integration

**Dockerfile**:
```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/codeguards-mcp /usr/local/bin/
ENTRYPOINT ["codeguards-mcp"]
```

**Usage**:
```bash
docker build -t codeguards .
docker run -v $(pwd):/project codeguards check --all --root /project
```

### Remote Development

For SSH/remote development environments:

1. Install CodeGuards on the remote machine
2. Configure your local AI agent to use remote MCP:
   ```bash
   # Local machine
   hermes mcp add codeguards -- ssh user@remote-host "~/.slugthug/bin/codeguards-mcp serve"
   ```

## Migration from Other Tools

### From traditional linters

- **ESLint/TSLint**: Replace language-specific rules with CodeGuards' multi-language guards
- **RuboCop**: Use custom guards for Ruby-specific patterns  
- **Checkstyle**: Migrate Java rules to custom guard patterns

### From architecture enforcement tools

- **ArchUnit**: Map layer dependency rules to `structural/layer-dependencies`
- **Depcruiser**: Convert dependency graphs to `allowed_dependencies` in ARCHITECTURE.md
- **SonarQube**: Replace quality gates with CodeGuards' built-in hygiene/quality guards

## Performance Optimization

### Large Repository Tuning

For repositories with 1000+ files:

1. **Use incremental checks**: Always prefer `--diff` over `--all` in pre-commit
2. **Exclude generated code**: Add patterns to `.gitignore`
3. **Parallel validation**: Split monorepos into separate validation jobs
4. **Cache results**: Store exception tokens and catalog between runs

### Memory-Constrained Environments

For containers with limited memory:

```bash
# Limit parallelism
RAYON_NUM_THREADS=2 codeguards-mcp check --all

# Monitor memory usage
/usr/bin/time -v codeguards-mcp check --all
```

These integrations ensure CodeGuards fits seamlessly into any development workflow while maintaining architectural integrity.