use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::env;
use crate::config::QALAM_DIR;
use crate::llm;

#[derive(Subcommand)]
pub enum Action {
    /// Generate a spec from an RFC
    Generate {
        /// RFC id (e.g. RFC-001)
        #[arg(long)]
        from: String,
        /// Use AI to infer services, acceptance criteria, and contracts
        #[arg(long)]
        ai: bool,
    },
    /// Review a spec for completeness using AI
    Review {
        /// Spec id (e.g. SPEC-001)
        id: String,
    },
    /// List all specs, optionally filtered by service
    List {
        /// Only show specs that involve this service
        #[arg(long)]
        service: Option<String>,
    },
    /// Mark a spec as shipped/closed
    Close {
        /// Spec id (e.g. SPEC-001)
        id: String,
    },
}

fn default_status() -> String {
    "draft".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub feature: String,
    pub rfc: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub services: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub contracts: Vec<Contract>,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Contract {
    pub service: String,
    pub endpoint: String,
}

pub async fn run(action: Action) -> anyhow::Result<()> {
    match action {
        Action::Generate { from, ai } => generate(&from, ai).await,
        Action::List { service } => list(service.as_deref()).await,
        Action::Close { id } => close(&id).await,
        Action::Review { id } => review(&id).await,
    }
}

async fn generate(rfc_id: &str, ai: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let rfcs_dir = root.join(QALAM_DIR).join("rfcs");
    let specs_dir = root.join(QALAM_DIR).join("specs");

    let rfc_path = find_rfc(&rfcs_dir, rfc_id)?;
    let rfc_content = std::fs::read_to_string(&rfc_path)?;

    let id = next_id(&specs_dir, "SPEC")?;
    let rfc_title = extract_title(&rfc_content);
    let feature = rfc_title.as_deref().unwrap_or(rfc_id);
    let feature_slug = slug(rfc_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(rfc_id)
        .trim_start_matches(&format!("{}-", rfc_id)));
    let filename = format!("{}-{}.yaml", id, feature_slug);

    let mut spec = Spec {
        id: id.clone(),
        feature: feature.to_string(),
        rfc: rfc_id.to_string(),
        depends_on: vec![],
        services: vec![],
        acceptance_criteria: vec!["<!-- add acceptance criteria -->".to_string()],
        contracts: vec![],
        status: default_status(),
        tags: vec![],
    };

    if ai {
        print!("Inferring spec fields from RFC with AI... ");
        match llm::suggest_spec_fields(&rfc_content).await {
            Ok(json_str) => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    if let Some(svcs) = v["services"].as_array() {
                        spec.services = svcs.iter()
                            .filter_map(|s| s.as_str().map(|x| x.to_string()))
                            .collect();
                    }
                    if let Some(ac) = v["acceptance_criteria"].as_array() {
                        let criteria: Vec<String> = ac.iter()
                            .filter_map(|s| s.as_str().map(|x| x.to_string()))
                            .collect();
                        if !criteria.is_empty() { spec.acceptance_criteria = criteria; }
                    }
                    if let Some(contracts) = v["contracts"].as_array() {
                        spec.contracts = contracts.iter().filter_map(|c| {
                            Some(Contract {
                                service: c["service"].as_str()?.to_string(),
                                endpoint: c["endpoint"].as_str()?.to_string(),
                            })
                        }).collect();
                    }
                    println!("✓");
                } else {
                    println!("✗ (could not parse AI response, using empty template)");
                }
            }
            Err(e) => println!("✗ ({e})"),
        }
    }

    let content = serde_yaml::to_string(&spec)?;
    std::fs::write(specs_dir.join(&filename), content)?;

    println!("✓ Created .qalam/specs/{}", filename);
    if !ai {
        println!("  Fill in services and acceptance_criteria, then run:");
    }
    println!("  qalam breakdown --from {}", id);
    println!("  qalam testplan --from {}", id);

    Ok(())
}

async fn review(spec_id: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let specs_dir = root.join(QALAM_DIR).join("specs");

    let entry = std::fs::read_dir(&specs_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(spec_id))
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' not found", spec_id))?;

    let spec_yaml = std::fs::read_to_string(entry.path())?;
    println!("Reviewing {} with AI...\n", spec_id);

    match llm::review_spec(&spec_yaml).await {
        Ok(feedback) => println!("{feedback}"),
        Err(e) => eprintln!("AI review failed: {e}\nSet ANTHROPIC_API_KEY to use this feature."),
    }
    Ok(())
}

async fn list(service: Option<&str>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let specs_dir = root.join(QALAM_DIR).join("specs");

    let mut entries: Vec<_> = std::fs::read_dir(&specs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No specs found. Run: qalam spec generate --from RFC-001");
        return Ok(());
    }

    let mut shown = 0;
    for entry in &entries {
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let spec: Spec = match serde_yaml::from_str(&content) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if let Some(svc) = service {
            if !spec.services.iter().any(|s| s == svc) { continue; }
        }

        let marker = match spec.status.as_str() {
            "shipped" | "closed" | "done" => "✓",
            "draft" => "○",
            _ => "○",
        };
        let svc_list = if spec.services.is_empty() {
            String::new()
        } else {
            format!("  [{}]", spec.services.join(", "))
        };
        println!("  {marker} {}  {}{}",
            entry.file_name().to_string_lossy(),
            spec.feature,
            svc_list,
        );
        shown += 1;
    }

    if shown == 0 {
        if let Some(svc) = service {
            println!("  No specs found for service '{svc}'.");
        } else {
            println!("  No specs found.");
        }
    }

    Ok(())
}

async fn close(id: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let specs_dir = root.join(QALAM_DIR).join("specs");

    let entry = std::fs::read_dir(&specs_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(id))
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' not found", id))?;

    let path = entry.path();
    let content = std::fs::read_to_string(&path)?;
    let mut spec: Spec = serde_yaml::from_str(&content)?;

    if spec.status == "shipped" {
        println!("  {} is already closed.", id);
        return Ok(());
    }

    spec.status = "shipped".to_string();
    std::fs::write(&path, serde_yaml::to_string(&spec)?)?;

    println!("✓ {} marked as shipped", id);
    println!("  Spec will no longer appear in active context output.");
    Ok(())
}

fn find_rfc(dir: &std::path::Path, id: &str) -> anyhow::Result<std::path::PathBuf> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(id) {
            return Ok(entry.path());
        }
    }
    anyhow::bail!("RFC '{}' not found in .qalam/rfcs/", id)
}

fn extract_title(content: &str) -> Option<String> {
    content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").to_string())
}

fn next_id(dir: &std::path::Path, prefix: &str) -> anyhow::Result<String> {
    let count = std::fs::read_dir(dir)?.count();
    Ok(format!("{}-{:03}", prefix, count + 1))
}

fn slug(s: &str) -> String {
    s.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}
