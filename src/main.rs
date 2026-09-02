//! CodeGuards CLI entrypoint and subcommand dispatch.

use codeguards_mcp::analyzer::{collect_git_diff_files, collect_source_files};
use codeguards_mcp::contract::{load_architecture, validate_architecture};
use codeguards_mcp::guards::run_guard_checks;
use codeguards_mcp::library::ensure_test_library_seeded;
use codeguards_mcp::storage::ProjectExceptions;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "check".to_string());

    match command.as_str() {
        "check" => run_check_cli(args).await,
        "exception" => run_exception_cli(args).await,
        "validate" => run_validate_cli(args).await,
        "serve" => run_serve_cli(args).await,
        "--help" | "-h" | "help" => {
            print_help();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("Unknown command: {other}. Use 'codeguards-mcp --help' for usage.");
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!(
        "CodeGuards MCP (Rust 2024)\n\
         Continuous Code Governance & Architectural Enforcement\n\n\
         USAGE:\n\
           codeguards-mcp check [--all] [--root <path>]  Run guard checks on modified files (or all)\n\
           codeguards-mcp validate [--root <path>]      Validate .planning/ARCHITECTURE.md against tests\n\
           codeguards-mcp exception add <file> <guard> --reason=\"...\"\n\
           codeguards-mcp exception list\n\
           codeguards-mcp exception revoke <token>\n\
           codeguards-mcp serve [--port <port>]         Start MCP server (stdio default)"
    );
}

async fn run_check_cli(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut check_all = false;
    let mut root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--all" => check_all = true,
            "--diff" => check_all = false,
            "--root" => {
                if let Some(r) = args.next() {
                    root = PathBuf::from(r);
                }
            }
            _ => {}
        }
    }

    let catalog = match ensure_test_library_seeded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Failed to seed test library: {e}");
            return ExitCode::from(2);
        }
    };

    let contract = match load_architecture(&root) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Could not load ARCHITECTURE.md: {e}");
            return ExitCode::from(2);
        }
    };

    let exceptions = ProjectExceptions::load(&root).unwrap_or_default();

    let files = if check_all {
        collect_source_files(&root).unwrap_or_default()
    } else {
        collect_git_diff_files(&root).unwrap_or_default()
    };

    if files.is_empty() {
        println!("[CODEGUARD-PASS] No source files to check.");
        return ExitCode::SUCCESS;
    }

    let report = match run_guard_checks(&root, &files, &contract, &catalog, &exceptions) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Check evaluation failed: {e}");
            return ExitCode::from(2);
        }
    };

    if report.is_pass() {
        println!(
            "[CODEGUARD-PASS] {} files verified across {} active guard rules in {}ms.",
            report.total_files_checked,
            report.passed_tests.len(),
            report.duration_ms
        );
        ExitCode::SUCCESS
    } else {
        println!("\n════════════════════════════════════════════════════════════");
        println!(" [CODEGUARD-BLOCKED] ARCHITECTURE GUARD VIOLATIONS DETECTED");
        println!("════════════════════════════════════════════════════════════");
        for v in &report.violations {
            let line_str = v.line.map_or(String::new(), |l| format!(":{l}"));
            println!("\n❌ [{}] {}{}", v.guard_id, v.file.display(), line_str);
            println!("   Message: {}", v.message);
            if let Some(rule) = &v.rule_reference {
                println!("   Rule:    {rule}");
            }
            if let Some(fix) = &v.fix_suggestion {
                println!("   Fix:     {fix}");
            }
        }
        println!("\n════════════════════════════════════════════════════════════");
        println!(" Total Errors: {}", report.error_count());
        println!(" Run 'codeguards exception add <file> <guard> --reason=\"...\"' if an approved exception is required.");
        println!("════════════════════════════════════════════════════════════\n");
        ExitCode::from(1)
    }
}

async fn run_validate_cli(mut args: impl Iterator<Item = String>) -> ExitCode {
    let mut root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    while let Some(arg) = args.next() {
        if arg == "--root" {
            if let Some(r) = args.next() {
                root = PathBuf::from(r);
            }
        }
    }

    let catalog = match ensure_test_library_seeded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Failed to seed test library: {e}");
            return ExitCode::from(2);
        }
    };

    let result = match validate_architecture(&root, &catalog) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Validation error: {e}");
            return ExitCode::from(2);
        }
    };

    if result.is_valid {
        println!("[CODEGUARD-PASS] .planning/ARCHITECTURE.md is valid and all required guards exist!");
        println!("  Active Guards: {:?}", result.ready_guards);
        ExitCode::SUCCESS
    } else {
        eprintln!("[CODEGUARD-BLOCKED] Architecture validation failed:");
        for err in result.errors {
            eprintln!("  ❌ {err}");
        }
        ExitCode::from(1)
    }
}

async fn run_exception_cli(mut args: impl Iterator<Item = String>) -> ExitCode {
    let sub = args.next().unwrap_or_else(|| "list".to_string());
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut exceptions = ProjectExceptions::load(&root).unwrap_or_default();

    match sub.as_str() {
        "add" => {
            let file_str = match args.next() {
                Some(f) => f,
                None => {
                    eprintln!("Usage: codeguards exception add <file> <guard> --reason=\"...\"");
                    return ExitCode::from(2);
                }
            };
            let guard = match args.next() {
                Some(g) => g,
                None => {
                    eprintln!("Usage: codeguards exception add <file> <guard> --reason=\"...\"");
                    return ExitCode::from(2);
                }
            };
            let mut reason = "User authorized exception".to_string();
            while let Some(arg) = args.next() {
                if arg.starts_with("--reason=") {
                    reason = arg["--reason=".len()..].trim_matches('"').to_string();
                } else if arg == "--reason" {
                    if let Some(r) = args.next() {
                        reason = r;
                    }
                }
            }

            match exceptions.add_exception(Path::new(&file_str), &guard, &reason) {
                Ok(entry) => {
                    println!("[CODEGUARD-SUCCESS] Exception authorized.");
                    println!("  Token:   {}", entry.token);
                    println!("  File:    {}", entry.file.display());
                    println!("  Guard:   {}", entry.guard_id);
                    println!("  Header:  // codeguard-exception: token={}; guard={}; reason=\"{}\"", entry.token, entry.guard_id, entry.reason);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("[CODEGUARD-ERROR] Could not save exception: {e}");
                    ExitCode::from(1)
                }
            }
        }
        "list" => {
            println!("Active exceptions for {}:", root.display());
            if exceptions.exceptions.is_empty() {
                println!("  (No active exceptions)");
            } else {
                for e in &exceptions.exceptions {
                    println!("  - [{}] {} (Guard: {}) - Reason: {}", e.token, e.file.display(), e.guard_id, e.reason);
                }
            }
            ExitCode::SUCCESS
        }
        "revoke" => {
            let token = match args.next() {
                Some(t) => t,
                None => {
                    eprintln!("Usage: codeguards exception revoke <token>");
                    return ExitCode::from(2);
                }
            };
            match exceptions.revoke(&token) {
                Ok(true) => {
                    println!("[CODEGUARD-SUCCESS] Exception token {token} revoked.");
                    ExitCode::SUCCESS
                }
                Ok(false) => {
                    eprintln!("[CODEGUARD-ERROR] Token {token} not found.");
                    ExitCode::from(1)
                }
                Err(e) => {
                    eprintln!("[CODEGUARD-ERROR] Revocation failed: {e}");
                    ExitCode::from(1)
                }
            }
        }
        other => {
            eprintln!("Unknown exception subcommand: {other}");
            ExitCode::from(2)
        }
    }
}

async fn run_serve_cli(_args: impl Iterator<Item = String>) -> ExitCode {
    use codeguards_mcp::server::CodeGuardsMcpServer;
    use rmcp::service::ServiceExt;
    use rmcp::transport::io::stdio;

    let catalog = match ensure_test_library_seeded() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Failed to seed test library: {e}");
            return ExitCode::from(2);
        }
    };

    let handler = CodeGuardsMcpServer::new(catalog);

    match handler.serve(stdio()).await {
        Ok(service) => match service.waiting().await {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("[CODEGUARD-ERROR] Service error: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("[CODEGUARD-ERROR] Server start error: {e}");
            ExitCode::from(1)
        }
    }
}
