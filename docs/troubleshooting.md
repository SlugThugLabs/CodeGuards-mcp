# Troubleshooting & FAQ

## Common Issues

### "Command not found: codeguards-mcp"

**Cause**: Binary not in your PATH.

**Solution**:
```bash
# Verify binary exists
ls ~/.slugthug/bin/codeguards-mcp

# Add to PATH permanently
echo 'export PATH="$HOME/.slugthug/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Or run directly
~/.slugthug/bin/codeguards-mcp --help
```

### "Missing architecture contract at .planning/ARCHITECTURE.md"

**Cause**: Project doesn't have an architecture file.

**Solution**:
```bash
# Create the required directory and file
mkdir -p .planning
cat > .planning/ARCHITECTURE.md << 'EOF'
+++
modules = []
layers = []
enforce = []
+++

# Your architecture documentation here
EOF
```

### "Required guard test 'X' is not found"

**Cause**: Referenced guard doesn't exist in your library.

**Solution**:
1. **Check available guards**:
   ```bash
   codeguards-mcp menu  # Option 4: Inspect guard-test library
   ```
2. **Create missing guard** via AI agent or manually
3. **Use correct name/alias** from the catalog

### "Exception token validation failed"

**Cause**: Token doesn't match file/guard/reason combination.

**Solution**:
```bash
# List active exceptions
codeguards-mcp exception list

# Revoke and recreate if needed
codeguards-mcp exception revoke <old-token>
codeguards-mcp exception add <file> <guard> --reason="Updated reason"
```

### "Sandbox violation: path contains .ssh component"

**Cause**: Trying to scan sensitive system directories.

**Solution**: Only run CodeGuards on project directories, never on system paths like `/`, `/home`, or `~/.ssh`.

## Performance Issues

### Slow full repository scans

**Symptoms**: `check --all` takes >1 second on small repos.

**Diagnosis**:
```bash
# Enable debug logging
RUST_LOG=debug codeguards-mcp check --all
```

**Solutions**:
- Ensure you're using release build (`cargo build --release`)
- Exclude large generated directories in `.gitignore`
- Use `check --diff` for pre-commit hooks instead of `--all`

### High memory usage

**Cause**: Scanning very large files (>10MB).

**Solution**: Add large files to `.gitignore` or exclude patterns in your architecture file.

## Integration Problems

### Git pre-commit hook fails silently

**Solution**: Make hook executable and test manually:
```bash
chmod +x .git/hooks/pre-commit
.git/hooks/pre-commit
```

### AI agent can't connect to MCP server

**Solution**: Verify server registration:
```bash
# For Hermes Agent
hermes mcp list

# Re-register if needed
hermes mcp remove codeguards
hermes mcp add codeguards -- ~/.slugthug/bin/codeguards-mcp serve
```

## Advanced Debugging

### Enable verbose logging
```bash
RUST_LOG=codeguards_mcp=debug codeguards-mcp check --all
```

### Profile performance
```bash
# Install flamegraph
cargo install flamegraph

# Run with profiling
flamegraph codeguards-mcp check --all
```

### Inspect internal state
```bash
# View guard catalog
cat ~/.slugthug/codeguards/tests/catalog.json

# View project exceptions  
cat ~/.slugthug/codeguards/projects/*/exceptions.json
```

## FAQ

### Q: Can I disable specific guards for certain files?

**A**: Yes, use exception tokens:
```bash
codeguards-mcp exception add src/legacy.rs complexity/source-limits --reason="Legacy code"
```

### Q: How do I update built-in guard tests?

**A**: Built-in guards are embedded in the binary. Update by upgrading CodeGuards itself.

### Q: Can I use CodeGuards without Rust installed?

**A**: Yes! The binary has zero runtime dependencies. Only building from source requires Rust.

### Q: What happens if I delete ~/.slugthug/codeguards/?

**A**: All custom guards and exceptions are lost. Built-in guards will be re-seeded on next run.

### Q: How do I migrate exceptions between machines?

**A**: Copy the entire `~/.slugthug/codeguards/projects/` directory to the new machine.

### Q: Can I use CodeGuards with non-Rust projects?

**A**: Yes! Built-in guards support Rust, Python, TypeScript, and JavaScript. Custom guards can handle any language.

### Q: Is there a web UI or GUI?

**A**: Not currently. CodeGuards is designed as a CLI/MCP tool for integration into existing workflows.

### Q: How secure are exception tokens?

**A**: Tokens are HMAC-SHA256 signatures using a machine-private 256-bit key. They cannot be forged without access to `~/.slugthug/.secret.key`.

### Q: What's the difference between `check --diff` and `check --all`?

**A**: 
- `--diff`: Only checks files modified in git working tree (fast, for pre-commit)
- `--all`: Checks all source files in the project (comprehensive, for CI)

### Q: Can I customize the violation output format?

**A**: Not currently. Output follows a standard structured format for easy parsing by CI systems.

### Q: How do I contribute new built-in guards?

**A**: Fork the repository, add your guard to `src/library/builtins.rs`, and submit a pull request.