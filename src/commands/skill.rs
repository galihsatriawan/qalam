use clap::Subcommand;
use std::env;
use crate::config::QALAM_DIR;

/// Registry base: GitHub repo that hosts community skills.
/// Structure: galihsatriawan/qalam-skills/skills/<name>/{skill.yaml,context.md}
const REGISTRY_REPO: &str = "galihsatriawan/qalam-skills";
const REGISTRY_BRANCH: &str = "main";

#[derive(Subcommand)]
pub enum Action {
    /// Install a skill (local path, @scope/name from registry, or built-in scaffold)
    Install {
        /// Skill name, @scope/name, or local path (./my-skill)
        name: String,
    },
    /// List installed skills
    List,
    /// Remove an installed skill
    Remove {
        /// Skill name to remove
        name: String,
    },
    /// List available skills in the registry
    Search {
        /// Optional filter
        query: Option<String>,
    },
    /// Publish a local skill to the registry (opens a GitHub issue)
    Publish {
        /// Skill name to publish
        name: String,
    },
    /// Expose a skill as a Claude Code slash command (.claude/commands/)
    Expose {
        /// Skill name to expose; use --all to expose all skills
        name: Option<String>,
        /// Expose all installed skills
        #[arg(long)]
        all: bool,
    },
    /// Update an installed registry skill to the latest version
    Update {
        /// Skill name to update (omit to update all)
        name: Option<String>,
    },
    /// Show diff between installed skill and registry version
    Diff {
        /// Skill name to diff
        name: String,
    },
}

pub async fn run(action: Action) -> anyhow::Result<()> {
    match action {
        Action::Install { name } => install(&name).await,
        Action::List => list().await,
        Action::Remove { name } => remove(&name).await,
        Action::Search { query } => search(query.as_deref()).await,
        Action::Publish { name } => publish(&name).await,
        Action::Expose { name, all } => expose(name.as_deref(), all).await,
        Action::Update { name } => update(name.as_deref()).await,
        Action::Diff { name } => diff(&name).await,
    }
}

async fn install(name: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skills_dir = root.join(QALAM_DIR).join("skills");
    std::fs::create_dir_all(&skills_dir)?;

    // Local path
    if name.starts_with('.') || name.starts_with('/') {
        return install_local(name, &skills_dir);
    }

    // Registry: @scope/name or just name with @ prefix
    if name.starts_with('@') {
        return install_from_registry(name, &skills_dir).await;
    }

    // No prefix — scaffold built-in
    install_scaffold(name, &skills_dir)
}

fn install_local(path: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    let src = std::path::Path::new(path);
    anyhow::ensure!(src.exists(), "Path '{}' does not exist", path);
    let skill_name = src.file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    let dest = skills_dir.join(skill_name);
    copy_dir(src, &dest)?;
    println!("✓ Installed skill '{}' from {}", skill_name, path);
    Ok(())
}

async fn install_from_registry(name: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    // @scope/skill-name → use scope as registry owner, skill-name as skill
    // @skill-name → use default registry
    let clean = name.trim_start_matches('@');
    let (registry_path, local_name) = if let Some((scope, skill)) = clean.split_once('/') {
        // @scope/skill → try scope as a github org: scope/qalam-skills
        let repo = format!("{}/qalam-skills", scope);
        (format!("{}/skills/{}", repo, skill), skill.to_string())
    } else {
        (format!("{}/skills/{}", REGISTRY_REPO, clean), clean.to_string())
    };

    let dest = skills_dir.join(&local_name);
    anyhow::ensure!(!dest.exists(), "Skill '{}' is already installed", local_name);

    println!("Fetching {} from registry...", name);

    let files = fetch_registry_skill(&registry_path).await?;
    std::fs::create_dir_all(&dest)?;

    for (filename, content) in &files {
        std::fs::write(dest.join(filename), content)?;
    }

    println!("✓ Installed skill '{}' from registry", local_name);
    Ok(())
}

fn install_scaffold(name: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    let dest = skills_dir.join(name);
    anyhow::ensure!(!dest.exists(), "Skill '{}' is already installed", name);

    std::fs::create_dir_all(&dest)?;
    std::fs::write(dest.join("skill.yaml"), skill_manifest(name))?;
    std::fs::write(dest.join("context.md"), skill_context(name))?;

    println!("✓ Scaffolded skill '{}' in .qalam/skills/{}/", name, name);
    println!("  Edit .qalam/skills/{}/context.md to add your patterns.", name);
    Ok(())
}

async fn list() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skills_dir = root.join(QALAM_DIR).join("skills");

    if !skills_dir.exists() {
        println!("No skills installed. Run: qalam skill install <name>");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&skills_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No skills installed. Run: qalam skill install <name>");
        return Ok(());
    }

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let desc = read_skill_description(&entry.path().join("skill.yaml"))
            .unwrap_or_default();
        if desc.is_empty() {
            println!("  {}", name);
        } else {
            println!("  {} — {}", name, desc);
        }
    }

    Ok(())
}

async fn remove(name: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skill_dir = root.join(QALAM_DIR).join("skills").join(name);
    anyhow::ensure!(skill_dir.exists(), "Skill '{}' is not installed", name);
    std::fs::remove_dir_all(&skill_dir)?;
    println!("✓ Removed skill '{}'", name);
    Ok(())
}

async fn search(query: Option<&str>) -> anyhow::Result<()> {
    println!("Fetching registry index from {}...", REGISTRY_REPO);
    let url = format!(
        "https://api.github.com/repos/{}/contents/skills?ref={}",
        REGISTRY_REPO, REGISTRY_BRANCH
    );

    let client = reqwest::Client::builder()
        .user_agent("qalam-cli")
        .build()?;

    let resp = client.get(&url).send().await;

    match resp {
        Ok(r) if r.status().is_success() => {
            let items: Vec<serde_json::Value> = r.json().await?;
            let skills: Vec<_> = items.iter()
                .filter(|item| item["type"] == "dir")
                .filter_map(|item| item["name"].as_str())
                .filter(|name| {
                    query.map(|q| name.contains(q)).unwrap_or(true)
                })
                .collect();

            if skills.is_empty() {
                println!("No skills found in registry.");
            } else {
                println!("Available skills (install with: qalam skill install @<name>):");
                for skill in skills {
                    println!("  @{}", skill);
                }
            }
        }
        _ => {
            println!("Registry not reachable. Available built-in scaffolds:");
            for name in &["rust", "go", "node", "python", "java", "kotlin", "grpc", "docker"] {
                println!("  {}", name);
            }
            println!("\nInstall with: qalam skill install <name>");
        }
    }

    Ok(())
}

/// Fetch skill files from GitHub raw content.
async fn fetch_registry_skill(registry_path: &str) -> anyhow::Result<Vec<(String, String)>> {
    let parts: Vec<&str> = registry_path.splitn(3, '/').collect();
    anyhow::ensure!(parts.len() == 3, "Invalid registry path: {}", registry_path);
    let owner = parts[0];
    let repo = parts[1];
    let path = parts[2];

    let api_url = format!(
        "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
        owner, repo, path, REGISTRY_BRANCH
    );

    let client = reqwest::Client::builder()
        .user_agent("qalam-cli")
        .build()?;

    let resp = client.get(&api_url).send().await?;
    anyhow::ensure!(
        resp.status().is_success(),
        "Skill not found in registry (HTTP {}). Check: qalam skill search",
        resp.status()
    );

    let items: Vec<serde_json::Value> = resp.json().await?;
    let mut files = Vec::new();

    for item in &items {
        if item["type"] != "file" {
            continue;
        }
        let filename = item["name"].as_str().unwrap_or("").to_string();
        let download_url = item["download_url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing download_url for {}", filename))?;

        let content = client.get(download_url).send().await?.text().await?;
        files.push((filename, content));
    }

    anyhow::ensure!(!files.is_empty(), "No files found for skill at {}", registry_path);
    Ok(files)
}

fn skill_manifest(name: &str) -> String {
    format!("name: {}\ndescription: \"\"\nversion: \"0.1.0\"\nauthor: \"\"\n", name)
}

fn skill_context(name: &str) -> String {
    format!(
        "# Skill: {name}\n\
        \n\
        ## Overview\n\
        <!-- Describe what this skill provides -->\n\
        \n\
        ## Code Patterns\n\
        <!-- Patterns and conventions specific to this skill/tech stack -->\n\
        \n\
        ## Agent Instructions\n\
        <!-- How AI agents should behave when working with this skill -->\n\
        \n\
        ## Examples\n\
        <!-- Representative code examples -->\n"
    )
}

fn read_skill_description(manifest_path: &std::path::Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(manifest_path)?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("description:") {
            let desc = rest.trim().trim_matches('"');
            if !desc.is_empty() {
                return Ok(desc.to_string());
            }
        }
    }
    Ok(String::new())
}

async fn publish(name: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skill_dir = root.join(QALAM_DIR).join("skills").join(name);
    anyhow::ensure!(skill_dir.exists(), "Skill '{}' is not installed. Run: qalam skill list", name);

    let context = std::fs::read_to_string(skill_dir.join("context.md"))
        .unwrap_or_else(|_| "<!-- no context.md found -->".to_string());
    let manifest = std::fs::read_to_string(skill_dir.join("skill.yaml"))
        .unwrap_or_else(|_| format!("name: {name}\n"));

    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| anyhow::anyhow!("GITHUB_TOKEN not set. Set GITHUB_TOKEN to publish."))?;

    let client = reqwest::Client::builder()
        .user_agent("qalam-cli")
        .build()?;

    let (reg_owner, reg_repo) = REGISTRY_REPO.split_once('/').unwrap();

    // 1. Get authenticated user login
    let user: serde_json::Value = client
        .get("https://api.github.com/user")
        .bearer_auth(&token)
        .send().await?.json().await?;
    let login = user["login"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not get GitHub user login"))?
        .to_string();
    println!("Logged in as: {login}");

    // 2. Fork registry (idempotent)
    println!("Forking {REGISTRY_REPO}...");
    client
        .post(format!("https://api.github.com/repos/{reg_owner}/{reg_repo}/forks"))
        .bearer_auth(&token)
        .json(&serde_json::json!({}))
        .send().await?;

    // Give GitHub a moment to create the fork
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // 3. Get default branch SHA
    let branch_info: serde_json::Value = client
        .get(format!("https://api.github.com/repos/{login}/{reg_repo}/git/ref/heads/{REGISTRY_BRANCH}"))
        .bearer_auth(&token)
        .send().await?.json().await?;
    let base_sha = branch_info["object"]["sha"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not get base branch SHA from fork"))?
        .to_string();

    // 4. Create feature branch
    let branch = format!("skill/{name}");
    let create_branch = client
        .post(format!("https://api.github.com/repos/{login}/{reg_repo}/git/refs"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "ref": format!("refs/heads/{branch}"),
            "sha": base_sha
        }))
        .send().await?;
    // 422 = branch already exists, that's fine
    anyhow::ensure!(
        create_branch.status().is_success() || create_branch.status().as_u16() == 422,
        "Failed to create branch: {}",
        create_branch.text().await.unwrap_or_default()
    );
    println!("Branch: {branch}");

    // 5. Commit skill.yaml and context.md
    for (filename, content) in &[("skill.yaml", &manifest), ("context.md", &context)] {
        let path = format!("skills/{name}/{filename}");
        // Check if file exists (need SHA to update)
        let existing: serde_json::Value = client
            .get(format!("https://api.github.com/repos/{login}/{reg_repo}/contents/{path}?ref={branch}"))
            .bearer_auth(&token)
            .send().await?.json().await?;
        let existing_sha = existing["sha"].as_str().map(|s| s.to_string());

        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(content.as_bytes());
        let mut payload = serde_json::json!({
            "message": format!("Add skill: {name}"),
            "content": encoded,
            "branch": branch
        });
        if let Some(sha) = existing_sha {
            payload["sha"] = serde_json::Value::String(sha);
        }

        let resp = client
            .put(format!("https://api.github.com/repos/{login}/{reg_repo}/contents/{path}"))
            .bearer_auth(&token)
            .json(&payload)
            .send().await?;
        anyhow::ensure!(
            resp.status().is_success(),
            "Failed to commit {filename}: {}",
            resp.text().await.unwrap_or_default()
        );
        println!("  ✓ Committed {filename}");
    }

    // 6. Open PR against registry
    let pr_resp = client
        .post(format!("https://api.github.com/repos/{reg_owner}/{reg_repo}/pulls"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": format!("Add skill: {name}"),
            "head": format!("{login}:{branch}"),
            "base": REGISTRY_BRANCH,
            "body": format!(
                "## New Skill: `{name}`\n\nSubmitted via `qalam skill publish`.\n\n### skill.yaml\n\n```yaml\n{manifest}\n```\n\n### context.md\n\n{context}"
            )
        }))
        .send().await?;

    anyhow::ensure!(
        pr_resp.status().is_success(),
        "Failed to open PR: {}",
        pr_resp.text().await.unwrap_or_default()
    );

    let pr: serde_json::Value = pr_resp.json().await?;
    let pr_url = pr["html_url"].as_str().unwrap_or("(unknown)");
    println!("\n✓ PR opened: {pr_url}");

    Ok(())
}

async fn expose(name: Option<&str>, all: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skills_dir = root.join(QALAM_DIR).join("skills");
    let commands_dir = root.join(".claude").join("commands");
    std::fs::create_dir_all(&commands_dir)?;

    let names: Vec<String> = if all || name.is_none() {
        std::fs::read_dir(&skills_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect()
    } else {
        vec![name.unwrap().to_string()]
    };

    if names.is_empty() {
        println!("No skills installed. Run: qalam skill install <name>");
        return Ok(());
    }

    for skill_name in &names {
        let ctx_path = skills_dir.join(skill_name).join("context.md");
        let content = std::fs::read_to_string(&ctx_path)
            .unwrap_or_else(|_| format!("# {skill_name} patterns\n\n(no context.md found)"));

        let command_file = commands_dir.join(format!("{skill_name}.md"));
        let command_content = format!(
            "# qalam skill: {skill_name}\n\n\
            Apply the {skill_name} patterns and conventions from this project's qalam skill.\n\n\
            {content}"
        );
        std::fs::write(&command_file, command_content)?;
        println!("✓ Exposed skill '{skill_name}' as /{skill_name}");
        println!("  Location: .claude/commands/{skill_name}.md");
    }
    Ok(())
}

async fn update(name: Option<&str>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skills_dir = root.join(QALAM_DIR).join("skills");

    let names: Vec<String> = if let Some(n) = name {
        vec![n.to_string()]
    } else {
        std::fs::read_dir(&skills_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect()
    };

    for skill_name in &names {
        let registry_path = format!("{}/skills/{}", REGISTRY_REPO, skill_name);
        print!("Updating {skill_name}... ");
        match fetch_registry_skill(&registry_path).await {
            Ok(files) => {
                let dest = skills_dir.join(skill_name);
                for (filename, content) in &files {
                    std::fs::write(dest.join(filename), content)?;
                }
                println!("✓");
            }
            Err(e) => println!("✗ ({e})"),
        }
    }
    Ok(())
}

async fn diff(name: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skill_dir = root.join(QALAM_DIR).join("skills").join(name);
    anyhow::ensure!(skill_dir.exists(), "Skill '{}' not installed", name);

    let registry_path = format!("{}/skills/{}", REGISTRY_REPO, name);
    println!("Fetching registry version of '{name}'...");
    let files = match fetch_registry_skill(&registry_path).await {
        Ok(f) => f,
        Err(_) => {
            println!("  Skill '{name}' not found in registry (may be local-only).");
            return Ok(());
        }
    };

    let mut any_diff = false;
    for (filename, remote_content) in &files {
        let local_path = skill_dir.join(filename);
        let local_content = std::fs::read_to_string(&local_path).unwrap_or_default();
        if &local_content != remote_content {
            println!("--- installed/{name}/{filename}");
            println!("+++ registry/{name}/{filename}");
            // Simple line diff
            for diff_line in simple_diff(&local_content, remote_content) {
                println!("{diff_line}");
            }
            any_diff = true;
        }
    }
    if !any_diff {
        println!("  No differences between installed and registry version.");
    }
    Ok(())
}

fn simple_diff(a: &str, b: &str) -> Vec<String> {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();
    let mut out = Vec::new();
    let max = a_lines.len().max(b_lines.len());
    for i in 0..max {
        match (a_lines.get(i), b_lines.get(i)) {
            (Some(l), Some(r)) if l == r => {}
            (Some(l), Some(r)) => {
                out.push(format!("-{l}"));
                out.push(format!("+{r}"));
            }
            (Some(l), None) => out.push(format!("-{l}")),
            (None, Some(r)) => out.push(format!("+{r}")),
            _ => {}
        }
    }
    out
}

fn copy_dir(src: &std::path::Path, dest: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dest.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}
