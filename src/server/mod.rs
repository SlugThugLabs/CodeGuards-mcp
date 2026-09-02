//! MCP Server implementation for Authoring and Governance tools.

use crate::contract::validate_architecture;
use crate::library::catalog::GuardCatalog;
use crate::library::create_custom_guard_test;
use crate::storage::ProjectExceptions;
use crate::types::GuardTestDefinition;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{tool, tool_handler, tool_router, ErrorData, Json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

const INSTRUCTIONS: &str = "CodeGuards architecture governance and modular test authoring tools.";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateArchitectureRequest {
    /// Absolute path to the target project directory (default: current dir)
    pub project_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListGuardTestsRequest {
    /// Optional category filter (e.g. 'structural', 'complexity', 'hygiene', 'languages/rust')
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateGuardTestRequest {
    /// Unique test name (e.g. 'auth_required')
    pub name: String,
    /// Category (structural, complexity, hygiene, quality, languages/rust, custom/<ns>)
    pub category: String,
    /// Short explanation of what the guard checks
    pub summary: String,
    /// Search tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Known aliases
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Execution engine name
    pub engine: String,
    /// Actionable fix instruction when rule fails
    pub remediation: String,
    /// Override duplicate detection warnings
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddExceptionRequest {
    /// Project root directory
    pub project_path: String,
    /// Relative file path receiving exception
    pub file: String,
    /// Guard identifier (e.g. 'complexity/source-limits')
    pub guard_id: String,
    /// Specific architectural justification
    pub reason: String,
}

#[derive(Clone)]
pub struct CodeGuardsMcpServer {
    pub catalog: Arc<Mutex<GuardCatalog>>,
    tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

impl CodeGuardsMcpServer {
    #[must_use]
    pub fn new(catalog: GuardCatalog) -> Self {
        Self {
            catalog: Arc::new(Mutex::new(catalog)),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl CodeGuardsMcpServer {
    #[tool(
        description = "Validates .planning/ARCHITECTURE.md against disk reality and the guard-tests catalog."
    )]
    async fn validate_architecture(
        &self,
        request: Parameters<ValidateArchitectureRequest>,
    ) -> Result<Json<crate::contract::ValidationResult>, ErrorData> {
        let path_str = request
            .0
            .project_path
            .as_deref()
            .unwrap_or(".");
        let project_root = match crate::util::validate_safe_path(Path::new(path_str)) {
            Ok(p) => p,
            Err(e) => return Err(ErrorData::invalid_params(format!("Unsafe path: {e}"), None)),
        };
        let catalog_guard = self.catalog.lock().await;

        match validate_architecture(&project_root, &catalog_guard) {
            Ok(res) => Ok(Json(res)),
            Err(e) => Err(ErrorData::internal_error(format!("Validation error: {e}"), None)),
        }
    }

    #[tool(
        description = "Lists all modular guard tests available in ~/.slugthug/codeguards/tests/."
    )]
    async fn list_guard_tests(
        &self,
        request: Parameters<ListGuardTestsRequest>,
    ) -> Result<Json<BTreeMap<String, crate::library::catalog::GuardCatalogEntry>>, ErrorData> {
        let category_filter = request.0.category.as_deref();
        let catalog = self.catalog.lock().await;

        let filtered: BTreeMap<String, _> = catalog
            .tests
            .iter()
            .filter(|(_, entry)| {
                category_filter.map_or(true, |cat| entry.category.starts_with(cat))
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(Json(filtered))
    }

    #[tool(
        description = "Creates a new modular guard test definition in ~/.slugthug/codeguards/tests/ with duplicate prevention."
    )]
    async fn create_guard_test(
        &self,
        request: Parameters<CreateGuardTestRequest>,
    ) -> Result<Json<String>, ErrorData> {
        let req = request.0;
        let def = GuardTestDefinition {
            id: format!("{}/{}", req.category, req.name),
            name: req.name,
            category: req.category,
            version: "1.0.0".to_string(),
            summary: req.summary,
            tags: req.tags,
            aliases: req.aliases,
            engine: req.engine,
            default_params: BTreeMap::new(),
            remediation: req.remediation,
        };

        let tests_root = crate::util::get_tests_dir();
        match create_custom_guard_test(&tests_root, def, req.force) {
            Ok(path) => {
                if let Ok(new_cat) = crate::library::ensure_test_library_seeded() {
                    let mut cat_lock = self.catalog.lock().await;
                    *cat_lock = new_cat;
                }
                Ok(Json(format!(
                    "Successfully created guard test at {}",
                    path.display()
                )))
            }
            Err(e) => Err(ErrorData::internal_error(format!(
                "Failed to create guard test: {e}"
            ), None)),
        }
    }

    #[tool(
        description = "Authorizes a user-approved exception token for a specific file and guard violation."
    )]
    async fn add_exception(
        &self,
        request: Parameters<AddExceptionRequest>,
    ) -> Result<Json<crate::types::ExceptionEntry>, ErrorData> {
        let req = request.0;
        let project_path = match crate::util::validate_safe_path(Path::new(&req.project_path)) {
            Ok(p) => p,
            Err(e) => return Err(ErrorData::invalid_params(format!("Unsafe project path: {e}"), None)),
        };
        let mut exceptions = ProjectExceptions::load(&project_path).unwrap_or_default();

        match exceptions.add_exception(Path::new(&req.file), &req.guard_id, &req.reason) {
            Ok(entry) => Ok(Json(entry)),
            Err(e) => Err(ErrorData::internal_error(format!("Exception creation failed: {e}"), None)),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for CodeGuardsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::LATEST)
            .with_instructions(INSTRUCTIONS)
            .with_server_info(Implementation::new(
                "codeguards-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
    }
}
