use clap::{Subcommand, ValueEnum};
use std::env;
use crate::config::QALAM_DIR;
use crate::llm;

#[derive(Debug, Clone, ValueEnum)]
pub enum RfcStatus {
    Draft,
    Accepted,
    Rejected,
    Superseded,
}

impl RfcStatus {
    fn matches(&self, status_line: &str) -> bool {
        let s = status_line.to_lowercase();
        match self {
            RfcStatus::Draft => s.contains("draft"),
            RfcStatus::Accepted => s.contains("accepted") || s.contains("published"),
            RfcStatus::Rejected => s.contains("reject"),
            RfcStatus::Superseded => s.contains("superseded"),
        }
    }
}

#[derive(Subcommand)]
pub enum Action {
    /// Generate a new RFC from a description or PRD file
    Generate {
        /// Feature description
        description: String,
        /// Optional path to PRD file
        #[arg(long)]
        from: Option<String>,
        /// Use AI (ANTHROPIC_API_KEY) to draft RFC sections
        #[arg(long)]
        ai: bool,
    },
    /// List all RFCs, optionally filtered by status
    List {
        /// Filter by status
        #[arg(long, value_enum)]
        status: Option<RfcStatus>,
    },
    /// Mark an RFC as Accepted
    Publish {
        /// RFC id (e.g. RFC-001)
        id: String,
    },
    /// Mark an RFC as Rejected with a reason
    Reject {
        /// RFC id (e.g. RFC-001)
        id: String,
        /// Reason for rejection
        #[arg(long)]
        reason: String,
    },
}

pub async fn run(action: Action) -> anyhow::Result<()> {
    match action {
        Action::Generate { description, from, ai } => generate(&description, from.as_deref(), ai).await,
        Action::List { status } => list(status.as_ref()).await,
        Action::Publish { id } => publish(&id).await,
        Action::Reject { id, reason } => reject(&id, &reason).await,
    }
}

async fn generate(description: &str, from: Option<&str>, ai: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let rfcs_dir = root.join(QALAM_DIR).join("rfcs");

    let id = next_id(&rfcs_dir, "RFC")?;
    let filename = format!("{}-{}.md", id, slug(description));
    let path = rfcs_dir.join(&filename);

    let prd_content = match from {
        Some(f) => std::fs::read_to_string(f)?,
        None => String::new(),
    };

    let content = if ai {
        println!("Generating RFC sections with AI...");
        match llm::generate_rfc_sections(description, &prd_content).await {
            Ok(sections) => rfc_template_ai(&id, description, &sections),
            Err(e) => {
                eprintln!("Warning: AI generation failed ({e}). Using template.");
                rfc_template(&id, description, &prd_content)
            }
        }
    } else {
        rfc_template(&id, description, &prd_content)
    };

    std::fs::write(&path, content)?;
    println!("✓ Created {}", path.display());
    println!("  Review the RFC, then run: qalam spec generate --from {}", id);
    Ok(())
}

async fn list(filter: Option<&RfcStatus>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let rfcs_dir = root.join(QALAM_DIR).join("rfcs");

    let mut entries: Vec<_> = std::fs::read_dir(&rfcs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No RFCs found. Run: qalam rfc generate \"your feature\"");
        return Ok(());
    }

    let mut shown = 0;
    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let status = std::fs::read_to_string(entry.path())
            .ok()
            .and_then(|c| extract_status_line(&c))
            .unwrap_or_else(|| "Draft".to_string());

        if let Some(f) = filter {
            if !f.matches(&status) { continue; }
        }

        let marker = match status.to_lowercase().as_str() {
            s if s.contains("accepted") || s.contains("published") => "✓",
            s if s.contains("reject") || s.contains("superseded") => "✗",
            _ => "○",
        };
        println!("  {marker} {name}  [{status}]");
        shown += 1;
    }

    if shown == 0 {
        println!("  No RFCs match the filter.");
    }

    Ok(())
}

fn extract_status_line(content: &str) -> Option<String> {
    let mut in_status = false;
    for line in content.lines() {
        if line == "## Status" { in_status = true; continue; }
        if in_status {
            let t = line.trim();
            if !t.is_empty() { return Some(t.to_string()); }
            if t.starts_with("## ") { break; }
        }
    }
    None
}

async fn publish(id: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let rfcs_dir = root.join(QALAM_DIR).join("rfcs");

    let entry = std::fs::read_dir(&rfcs_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(id))
        .ok_or_else(|| anyhow::anyhow!("RFC '{}' not found", id))?;

    let path = entry.path();
    let content = std::fs::read_to_string(&path)?;

    let updated = update_status(&content, "Accepted");
    std::fs::write(&path, updated)?;

    println!("✓ {} marked as Accepted", id);
    println!("  Next: qalam spec generate --from {}", id);
    Ok(())
}

async fn reject(id: &str, reason: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let rfcs_dir = root.join(QALAM_DIR).join("rfcs");

    let entry = std::fs::read_dir(&rfcs_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(id))
        .ok_or_else(|| anyhow::anyhow!("RFC '{}' not found", id))?;

    let path = entry.path();
    let content = std::fs::read_to_string(&path)?;
    let updated = update_status(&content, &format!("Rejected: {reason}"));
    std::fs::write(&path, updated)?;

    println!("✗ {} marked as Rejected", id);
    println!("  Reason: {}", reason);
    Ok(())
}

fn update_status(content: &str, new_status: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    let mut in_status = false;
    for line in &mut lines {
        if *line == "## Status" {
            in_status = true;
            continue;
        }
        if in_status && !line.trim().is_empty() {
            *line = Box::leak(new_status.to_string().into_boxed_str());
            break;
        }
        if in_status && line.starts_with("## ") {
            break;
        }
    }
    lines.join("\n") + "\n"
}

fn rfc_template(id: &str, title: &str, prd: &str) -> String {
    let prd_section = if prd.is_empty() {
        String::new()
    } else {
        format!("\n## PRD Reference\n\n{}\n", prd)
    };

    format!(
        "# {id}: {title}\n\
        \n\
        ## Status\n\
        Draft\n\
        \n\
        ## Problem\n\
        <!-- What problem are we solving? -->\n\
        \n\
        ## Options Considered\n\
        <!-- What approaches did we evaluate? -->\n\
        \n\
        ## Decision\n\
        <!-- What did we decide and why? -->\n\
        \n\
        ## Tradeoffs\n\
        <!-- What are we giving up? -->\n\
        \n\
        ## Affected Services\n\
        <!-- List services that will be impacted -->\n\
        {prd_section}"
    )
}

fn rfc_template_ai(id: &str, title: &str, ai_sections: &str) -> String {
    format!(
        "# {id}: {title}\n\
        \n\
        ## Status\n\
        Draft\n\
        \n\
        {ai_sections}\n"
    )
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
