use std::path::PathBuf;

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool_router,
    tool,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::config::QALAM_DIR;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdParam {
    #[schemars(description = "ID prefix, e.g. RFC-001 or SPEC-001")]
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ServiceParam {
    #[schemars(description = "Spec ID, e.g. SPEC-001")]
    pub spec_id: String,
    #[schemars(description = "Service name")]
    pub service: String,
}

#[derive(Clone)]
pub struct QalamServer {
    root: PathBuf,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl QalamServer {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            tool_router: Self::tool_router(),
        }
    }

    fn qalam_dir(&self) -> PathBuf {
        self.root.join(QALAM_DIR)
    }

    fn read_dir_names(&self, subdir: &str) -> Vec<String> {
        let dir = self.qalam_dir().join(subdir);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return vec![];
        };
        let mut names: Vec<_> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        names
    }

    fn find_file(&self, subdir: &str, id_prefix: &str) -> Option<String> {
        let dir = self.qalam_dir().join(subdir);
        std::fs::read_dir(&dir)
            .ok()?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with(id_prefix))
            .and_then(|e| std::fs::read_to_string(e.path()).ok())
    }
}

#[tool_router]
impl QalamServer {
    #[tool(description = "List all RFCs in the project")]
    fn list_rfcs(&self) -> String {
        let names = self.read_dir_names("rfcs");
        if names.is_empty() {
            "No RFCs found. Run: qalam rfc generate --description \"My feature\"".to_string()
        } else {
            names.join("\n")
        }
    }

    #[tool(description = "Get RFC content by ID prefix (e.g. RFC-001)")]
    fn get_rfc(&self, Parameters(IdParam { id }): Parameters<IdParam>) -> String {
        self.find_file("rfcs", &id)
            .unwrap_or_else(|| format!("RFC '{}' not found", id))
    }

    #[tool(description = "List all specs in the project")]
    fn list_specs(&self) -> String {
        let names = self.read_dir_names("specs");
        if names.is_empty() {
            "No specs found. Run: qalam spec generate --from RFC-001".to_string()
        } else {
            names.join("\n")
        }
    }

    #[tool(description = "Get spec content by ID prefix (e.g. SPEC-001)")]
    fn get_spec(&self, Parameters(IdParam { id }): Parameters<IdParam>) -> String {
        self.find_file("specs", &id)
            .unwrap_or_else(|| format!("Spec '{}' not found", id))
    }

    #[tool(description = "Get task file for a specific service in a spec")]
    fn get_task(&self, Parameters(ServiceParam { spec_id, service }): Parameters<ServiceParam>) -> String {
        let path = self.qalam_dir().join("tasks").join(&spec_id).join(format!("{}.md", service));
        std::fs::read_to_string(&path)
            .unwrap_or_else(|_| format!("Task for '{}' in '{}' not found", service, spec_id))
    }

    #[tool(description = "Get test plan for a spec by ID prefix (e.g. SPEC-001)")]
    fn get_testplan(&self, Parameters(IdParam { id }): Parameters<IdParam>) -> String {
        self.find_file("testplans", &id)
            .unwrap_or_else(|| format!("Testplan for '{}' not found", id))
    }

    #[tool(description = "Get all context for a service: relevant specs, tasks, and skills")]
    fn get_context(&self, Parameters(ServiceParam { spec_id, service }): Parameters<ServiceParam>) -> String {
        let spec = self.find_file("specs", &spec_id)
            .unwrap_or_else(|| format!("Spec '{}' not found", spec_id));
        let task_path = self.qalam_dir().join("tasks").join(&spec_id).join(format!("{}.md", service));
        let task = std::fs::read_to_string(&task_path)
            .unwrap_or_else(|_| "No task file found for this service.".to_string());
        let skills = self.list_skills_text();

        format!(
            "## Spec: {spec_id}\n\n{spec}\n\n---\n\n## Task: {service}\n\n{task}\n\n---\n\n## Installed Skills\n\n{skills}"
        )
    }

    #[tool(description = "List all installed skills")]
    fn list_skills(&self) -> String {
        self.list_skills_text()
    }
}

impl QalamServer {
    fn list_skills_text(&self) -> String {
        let names = self.read_dir_names("skills");
        if names.is_empty() {
            "No skills installed. Run: qalam skill install <name>".to_string()
        } else {
            names.join("\n")
        }
    }
}

impl ServerHandler for QalamServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "Qalam MCP server: provides spec-driven workflow context for AI-native development. \
                Use list_rfcs/get_rfc to explore decisions, list_specs/get_spec for implementation specs, \
                get_context for full service context, get_task for per-service tasks."
            )
    }
}

pub async fn run() -> anyhow::Result<()> {
    let root = std::env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    if !qalam_dir.exists() {
        anyhow::bail!("Not a qalam project. Run: qalam init");
    }

    eprintln!("Qalam MCP server starting on stdio...");
    let server = QalamServer::new(root);
    let (stdin, stdout) = stdio();
    server.serve((stdin, stdout)).await?
        .waiting().await?;

    Ok(())
}
