//! CodeGuards library root.

pub mod error;
pub mod types;
pub mod util;

pub use error::{CodeGuardsError, Result};
pub use types::{ExceptionEntry, GuardReport, GuardTestDefinition, Severity, Violation};
