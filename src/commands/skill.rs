use clap::Subcommand;
use std::env;
use crate::config::QALAM_DIR;

/// Primary registry: skills.sh ecosystem (any GitHub repo with SKILL.md files).
/// Extended registry: galihsatriawan/qalam-skills (qalam-specific skills).
/// Install formats:
///   @name          → galihsatriawan/qalam-skills/skills/<name>/SKILL.md
///   owner/repo     → skills.sh-compatible GitHub repo (delegates to npx skills, or raw fetch)
///   https://...    → direct SKILL.md URL
///   ./path         → local directory copy
const REGISTRY_REPO: &str = "galihsatriawan/qalam-skills";
const REGISTRY_BRANCH: &str = "main";

#[derive(Subcommand)]
pub enum Action {
    /// Install a skill.
    /// @name → from qalam-skills registry.
    /// owner/repo → any skills.sh-compatible GitHub repo.
    /// ./path → local directory.
    Install {
        name: String,
    },
    /// List installed skills
    List,
    /// Remove an installed skill
    Remove {
        name: String,
    },
    /// Search for skills. Delegates to skills.sh for community search; shows qalam-skills registry without a query.
    Search {
        query: Option<String>,
    },
    /// Publish a local skill to qalam-skills registry (opens a PR)
    Publish {
        name: String,
    },
    /// Expose a skill as a Claude Code slash command (.claude/commands/<name>.md)
    Expose {
        name: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// Update an installed skill to the latest registry version
    Update {
        name: Option<String>,
    },
    /// Show diff between installed skill and registry version
    Diff {
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

    // Direct HTTPS URL
    if name.starts_with("https://") {
        return install_from_url(name, &skills_dir).await;
    }

    // @name → qalam-skills registry
    if name.starts_with('@') {
        return install_from_qalam_registry(name, &skills_dir).await;
    }

    // owner/repo → skills.sh-compatible GitHub repo
    if name.contains('/') {
        return install_from_skills_sh(name, &skills_dir).await;
    }

    // No prefix — scaffold new skill with SKILL.md template
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

/// Install from a skills.sh-compatible GitHub repo (owner/repo format).
/// Delegates to `npx skills add` if available, otherwise falls back to raw GitHub fetch.
async fn install_from_skills_sh(repo: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    // Try npx skills first (preserves skills.sh agent directory routing)
    let npx = std::process::Command::new("npx")
        .args(["--yes", "skills", "add", repo, "--all", "--yes"])
        .status();

    match npx {
        Ok(s) if s.success() => {
            println!("✓ Installed via skills.sh CLI — run `qalam skill list` to see installed skills.");
            println!("  Tip: also copy SKILL.md into .qalam/skills/<name>/ to make it available to qalam context.");
            return Ok(());
        }
        _ => {}
    }

    // Fallback: fetch root SKILL.md from the GitHub repo
    println!("npx skills not available — fetching SKILL.md directly from {repo}...");
    let parts: Vec<&str> = repo.splitn(2, '/').collect();
    anyhow::ensure!(parts.len() == 2, "Expected owner/repo format, got: {}", repo);
    let (owner, repo_name) = (parts[0], parts[1]);
    let local_name = repo_name.trim_end_matches("-skills");

    let url = format!(
        "https://raw.githubusercontent.com/{owner}/{repo_name}/main/SKILL.md"
    );
    let content = fetch_raw_text(&url).await
        .map_err(|_| anyhow::anyhow!("Could not fetch SKILL.md from {url}\nCheck that the repo exists and has a SKILL.md at the root."))?;

    let dest = skills_dir.join(local_name);
    std::fs::create_dir_all(&dest)?;
    std::fs::write(dest.join("SKILL.md"), content)?;
    println!("✓ Installed skill '{}' from {}", local_name, repo);
    Ok(())
}

/// Install from a direct HTTPS URL pointing to a SKILL.md file.
async fn install_from_url(url: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    let content = fetch_raw_text(url).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch {url}: {e}"))?;

    // Derive name from URL path
    let name = url.split('/').rev()
        .find(|s| !s.is_empty() && *s != "SKILL.md")
        .unwrap_or("custom-skill");

    let dest = skills_dir.join(name);
    std::fs::create_dir_all(&dest)?;
    std::fs::write(dest.join("SKILL.md"), content)?;
    println!("✓ Installed skill '{}' from URL", name);
    Ok(())
}

async fn install_from_qalam_registry(name: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    let clean = name.trim_start_matches('@');
    let (registry_path, local_name) = if let Some((scope, skill)) = clean.split_once('/') {
        (format!("{scope}/qalam-skills/skills/{skill}"), skill.to_string())
    } else {
        (format!("{REGISTRY_REPO}/skills/{clean}"), clean.to_string())
    };

    let dest = skills_dir.join(&local_name);
    anyhow::ensure!(!dest.exists(), "Skill '{}' is already installed. Use: qalam skill update {}", local_name, local_name);

    println!("Fetching {} from qalam-skills registry...", name);
    let files = fetch_skill_files(&registry_path).await?;
    std::fs::create_dir_all(&dest)?;

    for (filename, content) in &files {
        std::fs::write(dest.join(filename), content)?;
    }

    println!("✓ Installed skill '{}' from qalam-skills registry", local_name);
    Ok(())
}

fn install_scaffold(name: &str, skills_dir: &std::path::Path) -> anyhow::Result<()> {
    let dest = skills_dir.join(name);
    anyhow::ensure!(!dest.exists(), "Skill '{}' already exists", name);

    std::fs::create_dir_all(&dest)?;
    std::fs::write(dest.join("SKILL.md"), skill_template(name))?;

    println!("✓ Scaffolded skill '{}' in .qalam/skills/{}/", name, name);
    println!("  Edit .qalam/skills/{}/SKILL.md to add your patterns.", name);
    println!("  Tip: this format is compatible with skills.sh — you can publish via npx skills or qalam skill publish.");
    Ok(())
}

async fn list() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skills_dir = root.join(QALAM_DIR).join("skills");

    if !skills_dir.exists() {
        println!("No skills installed.");
        println!("  From qalam-skills registry: qalam skill install @golang");
        println!("  From skills.sh ecosystem:   qalam skill install owner/repo");
        println!("  Scaffold new:               qalam skill install my-skill");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&skills_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No skills installed.");
        return Ok(());
    }

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let desc = read_skill_description(&entry.path()).unwrap_or_default();
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

/// Search for skills.
/// - With a query: delegates to `npx skills find` (skills.sh ecosystem).
/// - Without a query: lists available skills in the qalam-skills registry.
async fn search(query: Option<&str>) -> anyhow::Result<()> {
    if let Some(q) = query {
        // Delegate to skills.sh CLI for community search
        println!("Searching skills.sh for '{q}'...\n");
        let status = std::process::Command::new("npx")
            .args(["--yes", "skills", "find", q])
            .status();

        match status {
            Ok(s) if s.success() => return Ok(()),
            _ => {
                println!("  npx skills not available. Browse skills.sh manually: https://www.skills.sh/");
                println!("  Install CLI: npm install -g skills\n");
            }
        }
    }

    // Always also show qalam-skills registry
    println!("Available skills in qalam-skills registry (install with: qalam skill install @<name>):");
    let url = format!(
        "https://api.github.com/repos/{REGISTRY_REPO}/contents/skills?ref={REGISTRY_BRANCH}"
    );

    let client = http_client()?;
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => {
            let items: Vec<serde_json::Value> = r.json().await?;
            let skills: Vec<_> = items.iter()
                .filter(|item| item["type"] == "dir")
                .filter_map(|item| item["name"].as_str())
                .filter(|name| query.map(|q| name.contains(q)).unwrap_or(true))
                .collect();

            if skills.is_empty() {
                println!("  No matching skills found.");
            } else {
                for skill in skills {
                    println!("  @{skill}");
                }
            }
        }
        _ => {
            println!("  Registry not reachable. Skills available: golang, rust, python, nodejs, kotlin, java, grpc, rest-api, gin, fiber, fastapi, nestjs, spring-boot, clean-arch, hexagonal, cqrs, event-sourcing, ddd, payment, auth, notification, ecommerce");
        }
    }

    Ok(())
}

/// Fetch all files from a GitHub directory (via GitHub API).
async fn fetch_skill_files(registry_path: &str) -> anyhow::Result<Vec<(String, String)>> {
    let parts: Vec<&str> = registry_path.splitn(3, '/').collect();
    anyhow::ensure!(parts.len() == 3, "Invalid registry path: {}", registry_path);
    let (owner, repo, path) = (parts[0], parts[1], parts[2]);

    let api_url = format!(
        "https://api.github.com/repos/{owner}/{repo}/contents/{path}?ref={REGISTRY_BRANCH}"
    );

    let client = http_client()?;
    let resp = client.get(&api_url).send().await?;
    anyhow::ensure!(
        resp.status().is_success(),
        "Skill not found in registry (HTTP {}). Run: qalam skill search",
        resp.status()
    );

    let items: Vec<serde_json::Value> = resp.json().await?;
    let mut files = Vec::new();

    for item in &items {
        if item["type"] != "file" { continue; }
        let filename = item["name"].as_str().unwrap_or("").to_string();
        // Only fetch SKILL.md; skip unrelated files
        if filename != "SKILL.md" { continue; }
        let download_url = item["download_url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing download_url for {}", filename))?;
        let content = fetch_raw_text(download_url).await?;
        files.push((filename, content));
    }

    anyhow::ensure!(!files.is_empty(), "No SKILL.md found for skill at {}", registry_path);
    Ok(files)
}

async fn fetch_raw_text(url: &str) -> anyhow::Result<String> {
    let client = http_client()?;
    Ok(client.get(url).send().await?.text().await?)
}

fn http_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder().user_agent("qalam-cli").build()?)
}

fn skill_template(name: &str) -> String {
    format!(
        "---\n\
        name: {name}\n\
        description: \"\"\n\
        category: lang\n\
        tags: []\n\
        version: 0.1.0\n\
        ---\n\
        \n\
        # {name}\n\
        \n\
        ## Overview\n\
        <!-- Describe what this skill provides -->\n\
        \n\
        ## Patterns & Conventions\n\
        <!-- Patterns and conventions specific to this skill/tech stack -->\n\
        \n\
        ## Code Examples\n\
        <!-- Representative code examples -->\n\
        \n\
        ## Agent Instructions\n\
        <!-- How AI agents should behave when working in this context -->\n"
    )
}

/// Read skill description from SKILL.md frontmatter, falling back to legacy skill.yaml.
fn read_skill_description(skill_dir: &std::path::Path) -> anyhow::Result<String> {
    // Primary: SKILL.md frontmatter
    let skill_md = skill_dir.join("SKILL.md");
    if skill_md.exists() {
        let content = std::fs::read_to_string(&skill_md)?;
        if let Some(desc) = parse_frontmatter_field(&content, "description") {
            return Ok(desc);
        }
    }
    // Fallback: legacy skill.yaml
    let skill_yaml = skill_dir.join("skill.yaml");
    if skill_yaml.exists() {
        let content = std::fs::read_to_string(&skill_yaml)?;
        if let Some(desc) = parse_frontmatter_field(&content, "description") {
            return Ok(desc);
        }
    }
    Ok(String::new())
}

fn parse_frontmatter_field(content: &str, field: &str) -> Option<String> {
    // Works for both YAML frontmatter (--- ... ---) and plain YAML files
    let body = if content.starts_with("---") {
        content.splitn(3, "---").nth(1).unwrap_or(content)
    } else {
        content
    };
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix(&format!("{field}:")) {
            let val = rest.trim().trim_matches('"').trim_matches('\'');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Read SKILL.md content, stripping frontmatter for context injection.
pub fn read_skill_content(skill_dir: &std::path::Path) -> Option<String> {
    // Primary: SKILL.md
    let skill_md = skill_dir.join("SKILL.md");
    if skill_md.exists() {
        let raw = std::fs::read_to_string(&skill_md).ok()?;
        return Some(strip_frontmatter(&raw));
    }
    // Fallback: legacy context.md
    let ctx = skill_dir.join("context.md");
    if ctx.exists() {
        return std::fs::read_to_string(&ctx).ok();
    }
    None
}

fn strip_frontmatter(content: &str) -> String {
    if !content.starts_with("---") {
        return content.to_string();
    }
    // Split on second "---\n"
    if let Some(rest) = content[3..].find("\n---") {
        return content[3 + rest + 4..].to_string();
    }
    content.to_string()
}

async fn publish(name: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let skill_dir = root.join(QALAM_DIR).join("skills").join(name);
    anyhow::ensure!(skill_dir.exists(), "Skill '{}' not found. Run: qalam skill list", name);

    let skill_md_path = skill_dir.join("SKILL.md");
    anyhow::ensure!(skill_md_path.exists(), "No SKILL.md found in skill '{}'. Run: qalam skill install {}", name, name);
    let skill_content = std::fs::read_to_string(&skill_md_path)?;

    let token = std::env::var("GITHUB_TOKEN")
        .map_err(|_| anyhow::anyhow!("GITHUB_TOKEN not set. Set GITHUB_TOKEN to publish."))?;

    let client = http_client()?;
    let (reg_owner, reg_repo) = REGISTRY_REPO.split_once('/').unwrap();

    let user: serde_json::Value = client
        .get("https://api.github.com/user")
        .bearer_auth(&token)
        .send().await?.json().await?;
    let login = user["login"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not get GitHub user login"))?
        .to_string();
    println!("Logged in as: {login}");

    println!("Forking {REGISTRY_REPO}...");
    client
        .post(format!("https://api.github.com/repos/{reg_owner}/{reg_repo}/forks"))
        .bearer_auth(&token)
        .json(&serde_json::json!({}))
        .send().await?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let branch_info: serde_json::Value = client
        .get(format!("https://api.github.com/repos/{login}/{reg_repo}/git/ref/heads/{REGISTRY_BRANCH}"))
        .bearer_auth(&token)
        .send().await?.json().await?;
    let base_sha = branch_info["object"]["sha"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not get base branch SHA"))?
        .to_string();

    let branch = format!("skill/{name}");
    let r = client
        .post(format!("https://api.github.com/repos/{login}/{reg_repo}/git/refs"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "ref": format!("refs/heads/{branch}"), "sha": base_sha }))
        .send().await?;
    anyhow::ensure!(
        r.status().is_success() || r.status().as_u16() == 422,
        "Failed to create branch: {}", r.text().await.unwrap_or_default()
    );
    println!("Branch: {branch}");

    // Commit SKILL.md
    let file_path = format!("skills/{name}/SKILL.md");
    let existing: serde_json::Value = client
        .get(format!("https://api.github.com/repos/{login}/{reg_repo}/contents/{file_path}?ref={branch}"))
        .bearer_auth(&token)
        .send().await?.json().await?;
    let existing_sha = existing["sha"].as_str().map(|s| s.to_string());

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(skill_content.as_bytes());
    let mut payload = serde_json::json!({
        "message": format!("Add skill: {name}"),
        "content": encoded,
        "branch": branch
    });
    if let Some(sha) = existing_sha {
        payload["sha"] = serde_json::Value::String(sha);
    }

    let resp = client
        .put(format!("https://api.github.com/repos/{login}/{reg_repo}/contents/{file_path}"))
        .bearer_auth(&token)
        .json(&payload)
        .send().await?;
    anyhow::ensure!(
        resp.status().is_success(),
        "Failed to commit SKILL.md: {}", resp.text().await.unwrap_or_default()
    );
    println!("  ✓ Committed SKILL.md");

    let desc = parse_frontmatter_field(&std::fs::read_to_string(&skill_md_path)?, "description")
        .unwrap_or_default();

    let pr_resp = client
        .post(format!("https://api.github.com/repos/{reg_owner}/{reg_repo}/pulls"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "title": format!("Add skill: {name}"),
            "head": format!("{login}:{branch}"),
            "base": REGISTRY_BRANCH,
            "body": format!(
                "## New Skill: `{name}`\n\n**Description:** {desc}\n\nSubmitted via `qalam skill publish`.\n\nThis skill is compatible with the [skills.sh](https://www.skills.sh) ecosystem.\n\n<details><summary>SKILL.md preview</summary>\n\n{skill_content}\n\n</details>"
            )
        }))
        .send().await?;

    anyhow::ensure!(
        pr_resp.status().is_success(),
        "Failed to open PR: {}", pr_resp.text().await.unwrap_or_default()
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
        println!("No skills installed. Run: qalam skill install @golang");
        return Ok(());
    }

    for skill_name in &names {
        let skill_dir = skills_dir.join(skill_name);
        let content = read_skill_content(&skill_dir)
            .unwrap_or_else(|| format!("# {skill_name}\n\n(no SKILL.md found)"));

        let command_file = commands_dir.join(format!("{skill_name}.md"));
        // Keep the Claude command header minimal so the skill content reads naturally
        let command_content = format!(
            "Apply the {skill_name} patterns and conventions from this project's qalam skill:\n\n{content}"
        );
        std::fs::write(&command_file, command_content)?;
        println!("✓ Exposed '/{skill_name}' → .claude/commands/{skill_name}.md");
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
        let registry_path = format!("{REGISTRY_REPO}/skills/{skill_name}");
        print!("Updating {skill_name}... ");
        match fetch_skill_files(&registry_path).await {
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

    let registry_path = format!("{REGISTRY_REPO}/skills/{name}");
    println!("Fetching registry version of '{name}'...");
    let files = match fetch_skill_files(&registry_path).await {
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
            (Some(l), Some(r)) => { out.push(format!("-{l}")); out.push(format!("+{r}")); }
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
