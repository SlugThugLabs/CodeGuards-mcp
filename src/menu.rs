//! Interactive setup menu — walks a human through installing the binary,
//! connecting to AI agents, seeding the guard-test library, or running the MCP server.

use crate::style::Style;
use crate::util::{get_slugthug_home, get_tests_dir};
use std::io::{BufRead as _, Write as _};
use std::path::PathBuf;

/// Reads one menu choice from stdin. Unparseable input maps to `0` so the
/// caller's `_` arm reports it as invalid rather than panicking.
fn read_choice() -> Result<Option<usize>, Box<dyn std::error::Error>> {
    let mut line = String::new();
    let bytes_read = std::io::stdin().lock().read_line(&mut line)?;
    if bytes_read == 0 {
        // EOF reached
        return Ok(None);
    }
    let choice = line.trim().parse().unwrap_or(0);
    Ok(Some(choice))
}

/// Runs the interactive setup menu. Returns `Ok(true)` if user chose to run `serve`.
pub fn run_menu() -> Result<bool, Box<dyn std::error::Error>> {
    loop {
        let style = Style::stdout();
        print!("{}", render_menu(&style));
        std::io::stdout().flush()?;
        let choice = match read_choice()? {
            Some(c) => c,
            None => return Ok(false), // Exit on EOF
        };
        match choice {
            1 => install_step(),
            2 => connect_step(),
            3 => other_client_step(),
            4 => guard_catalog_step(),
            5 => {
                if confirm_serve()? {
                    return Ok(true);
                }
            }
            6 => return Ok(false),
            _ => {
                println!("\nInvalid choice; pick a number from the list.");
                continue;
            }
        }
        println!();
    }
}

const MENU: &str = "\
┌─────────────────────────────────────────────┐
│           CodeGuards setup                  │
└─────────────────────────────────────────────┘

  ── Setup ───────────────────────────────────

  1) Install the binary  (~/.slugthug/bin)
     A stable path for agents and MCP clients to launch.

  2) Connect to an AI agent
     Register this binary as the `codeguards` MCP server in
     Hermes, Claude Code, Grok, or Codex.

  3) Add CodeGuards to another MCP client
     Print instructions + a config snippet for any other
     tool that supports MCP servers (OpenCode, Cursor, Zed, ...).

  4) Inspect guard-test library
     List available modular tests in ~/.slugthug/codeguards/tests/

  ── Advanced ─────────────────────────────────

  5) Run the MCP server now
     Advanced: starts `serve` in this terminal and blocks
     until Ctrl-C. Normally your AI agent starts it for you.

  ── Exit ─────────────────────────────────────

  6) Exit

Choose an option [1-6]: ";

fn render_menu(style: &Style) -> String {
    if !style.enabled() {
        return MENU.to_owned();
    }
    let mut out = String::with_capacity(MENU.len() + 96);

    for line in MENU.lines() {
        let trimmed = line.trim();
        if let Some(label) = section_label(trimmed) {
            let prefix_len = line.find(label).unwrap_or(0);
            let before = &line[..prefix_len];
            let after = &line[prefix_len + label.len()..];
            out.push_str(before);
            out.push_str(&style.green(&style.bold(label)));
            out.push_str(&style.dim(after));
        } else if trimmed.starts_with('┌') || trimmed.starts_with('└') {
            out.push_str(&style.dim(line));
        } else if trimmed.contains("CodeGuards setup") {
            out.push_str(&line.replace(
                "CodeGuards setup",
                &style.bold(&style.cyan("CodeGuards setup")),
            ));
        } else if trimmed.starts_with("Choose an option") {
            out.push_str(&style.bold(&style.yellow(line)));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    out
}

fn section_label(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("── ")?;
    let (label, _after) = rest.split_once(" ─")?;
    if label.is_empty() || !label.chars().all(|c| c.is_alphanumeric() || c == ' ') {
        return None;
    }
    Some(label)
}

fn running_binary() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("codeguards-mcp"))
}

fn install_step() {
    println!();
    let source = running_binary();
    let bin_dir = get_slugthug_home().join("bin");
    let target = bin_dir.join("codeguards-mcp");

    if let Err(e) = std::fs::create_dir_all(&bin_dir) {
        eprintln!("Failed to create {}: {e}", bin_dir.display());
        return;
    }

    match std::fs::copy(&source, &target) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mut perms) = std::fs::metadata(&target).map(|m| m.permissions()) {
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&target, perms);
                }
            }
            println!("Installed CodeGuards to {}", target.display());
            println!("Ensure {} is in your $PATH.", bin_dir.display());
        }
        Err(e) => eprintln!("Failed to copy binary to {}: {e}", target.display()),
    }
}

fn connect_step() {
    println!();
    println!("Supported AI agent CLIs:");
    println!("  1) Hermes Agent (`hermes mcp add ...`)");
    println!("  2) Claude Code   (`claude mcp add ...`)");
    println!("  3) Grok          (`grok mcp add ...`)");
    println!("  4) Codex         (`codex mcp add ...`)");
    print!("\nChoose an agent [1-4]: ");
    std::io::stdout().flush().ok();

    let choice = read_choice().ok().flatten().unwrap_or(0);
    let (name, cli) = match choice {
        1 => ("Hermes", "hermes"),
        2 => ("Claude Code", "claude"),
        3 => ("Grok", "grok"),
        4 => ("Codex", "codex"),
        _ => {
            println!("Invalid choice.");
            return;
        }
    };

    let bin_path = get_slugthug_home().join("bin").join("codeguards-mcp");
    let exec_path = if bin_path.exists() {
        bin_path
    } else {
        running_binary()
    };

    println!("\nConnecting CodeGuards to {name}...");

    // Remove existing
    let _ = std::process::Command::new(cli)
        .args(["mcp", "remove", "codeguards"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    // Add server
    let status = std::process::Command::new(cli)
        .args(["mcp", "add", "codeguards", "--"])
        .arg(&exec_path)
        .arg("serve")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("Connected successfully! Verify with: {cli} mcp list");
        }
        _ => {
            println!("Could not invoke '{cli}' CLI. Make sure '{cli}' is installed and on your PATH.");
            println!("Manual command: {cli} mcp add codeguards -- {} serve", exec_path.display());
        }
    }
}

fn other_client_step() {
    println!();
    let bin_path = get_slugthug_home().join("bin").join("codeguards-mcp");
    let exec_path = if bin_path.exists() {
        bin_path
    } else {
        running_binary()
    };

    println!("CodeGuards is a standard stdio MCP server:");
    println!("  Server name:  codeguards");
    println!("  Command:      {}", exec_path.display());
    println!("  Arguments:    serve\n");
    println!("For clients reading .mcp.json (OpenCode, Claude, Cursor):");
    println!("{{\n  \"mcpServers\": {{\n    \"codeguards\": {{\n      \"command\": \"{}\",\n      \"args\": [\"serve\"]\n    }}\n  }}\n}}", exec_path.display());
}

fn guard_catalog_step() {
    println!();
    match crate::library::ensure_test_library_seeded() {
        Ok(catalog) => {
            println!("Guard-Test Library Catalog ({} tests available in {}):", catalog.total_tests, get_tests_dir().display());
            for (id, entry) in &catalog.tests {
                println!("  - {:<30} [{}]", id, entry.category);
                println!("    Summary: {}", entry.summary);
                if !entry.aliases.is_empty() {
                    println!("    Aliases: {}", entry.aliases.join(", "));
                }
            }
        }
        Err(e) => eprintln!("Failed to load guard-test library: {e}"),
    }
}

fn confirm_serve() -> Result<bool, Box<dyn std::error::Error>> {
    println!();
    println!("The MCP server (`serve`) is normally started automatically by your AI agent.");
    print!("Start the server in this terminal now? [y/N]: ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}
