//! CodeGuards library root.

pub mod contract;
pub mod error;
pub mod library;
pub mod storage;
pub mod types;
pub mod util;

pub use contract::{load_architecture, parse_frontmatter, validate_architecture, ArchitectureContract, ValidationResult};
pub use error::{CodeGuardsError, Result};
pub use library::catalog::GuardCatalog;
pub use storage::ProjectExceptions;
pub use types::{ExceptionEntry, GuardReport, GuardTestDefinition, Severity, Violation};
