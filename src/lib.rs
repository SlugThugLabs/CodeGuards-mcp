//! CodeGuards library root.

pub mod error;
pub mod library;
pub mod storage;
pub mod types;
pub mod util;

pub use error::{CodeGuardsError, Result};
pub use library::catalog::GuardCatalog;
pub use storage::ProjectExceptions;
pub use types::{ExceptionEntry, GuardReport, GuardTestDefinition, Severity, Violation};
