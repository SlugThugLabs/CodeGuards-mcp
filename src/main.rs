//! CodeGuards binary entrypoint.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    println!("codeguards-mcp v0.2.0 (Rust 2024)");
    ExitCode::SUCCESS
}
