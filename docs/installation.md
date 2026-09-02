# Installation Guide

CodeGuards is distributed as a single static binary with zero runtime dependencies. Choose your preferred installation method below.

## Prerequisites

- **Rust 1.93+** (for building from source)
- **Git** (for development workflows)

> **Note**: The binary itself requires no Rust installation — only the build process does.

## Method 1: Pre-built Binary (Recommended)

1. **Download the latest release** from [GitHub Releases](https://github.com/SlugThugLabs/CodeGuards-mcp/releases) (coming soon) or build locally:

```bash
# Clone the repository
git clone https://github.com/SlugThugLabs/CodeGuards-mcp.git
cd CodeGuards-mcp

# Build optimized release binary
cargo build --release

# Install to standard location
mkdir -p ~/.slugthug/bin
cp target/release/codeguards-mcp ~/.slugthug/bin/
```

2. **Add to your PATH** by adding this line to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
export PATH="$HOME/.slugthug/bin:$PATH"
```

3. **Reload your shell**:

```bash
source ~/.bashrc  # or source ~/.zshrc
```

4. **Verify installation**:

```bash
codeguards-mcp --help
```

You should see the help menu with all available commands.

## Method 2: Build from Source

If you prefer to build directly from the latest source:

```bash
# Clone and enter the repository
git clone https://github.com/SlugThugLabs/CodeGuards-mcp.git
cd CodeGuards-mcp

# Build and install in one step
cargo install --path .
```

This installs the binary to `~/.cargo/bin/` which should already be in your PATH if you have Rust installed.

## Verifying Your Installation

After installation, run these commands to verify everything works:

```bash
# Check version and basic help
codeguards-mcp --help

# Run interactive setup menu
codeguards-mcp menu

# Validate a project's architecture file
codeguards-mcp validate --root /path/to/your/project
```

## Next Steps

- [Configure your first project](./architecture.md)
- [Learn CLI commands](./cli-reference.md)
- [Set up git pre-commit hooks](./integrations.md)