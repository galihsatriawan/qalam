use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const QALAM_DIR: &str = ".qalam";

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub packages: Vec<String>,
    pub sources: Sources,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Sources {
    #[serde(default = "default_true")]
    pub git_history: bool,
    #[serde(default)]
    pub mcp: Vec<McpSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpSource {
    pub provider: String,
    pub filter: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let path = root.join(QALAM_DIR).join("qalam.yaml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let dir = root.join(QALAM_DIR);
        std::fs::create_dir_all(&dir)?;
        let content = serde_yaml::to_string(self)?;
        std::fs::write(dir.join("qalam.yaml"), content)?;
        Ok(())
    }

    pub fn qalam_dir(root: &Path) -> PathBuf {
        root.join(QALAM_DIR)
    }
}
