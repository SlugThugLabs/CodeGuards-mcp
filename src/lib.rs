//! CodeGuards library root.

pub mod analyzer;
pub mod contract;
pub mod error;
pub mod guards;
pub mod library;
pub mod server;
pub mod storage;
pub mod types;
pub mod util;

pub use analyzer::{collect_git_diff_files, collect_source_files, count_code_lines};
pub use contract::{load_architecture, parse_frontmatter, validate_architecture, ArchitectureContract, ValidationResult};
pub use error::{CodeGuardsError, Result};
pub use guards::run_guard_checks;
pub use library::catalog::GuardCatalog;
pub use server::CodeGuardsMcpServer;
pub use storage::ProjectExceptions;
pub use types::{ExceptionEntry, GuardReport, GuardTestDefinition, Severity, Violation};
