use clap::Subcommand;
use serde_json::{json, Value};
use std::{env, path::PathBuf};

#[derive(Subcommand)]
pub enum Action {
    /// Install qalam context hook (Claude Code by default; use flags for other tools)
    Install {
        /// Install globally in ~/.claude/settings.json instead of project
        #[arg(long)]
        global: bool,
        /// Also install context instructions for Cursor (.cursorrules)
        #[arg(long)]
        cursor: bool,
        /// Also install context instructions for GitHub Copilot (.github/copilot-instructions.md)
        #[arg(long)]
        copilot: bool,
        /// Also add qalam serve to .mcp.json
        #[arg(long)]
        mcp: bool,
        /// Install for all detected AI tools at once
        #[arg(long)]
        all: bool,
    },
    /// Remove qalam context hook from .claude/settings.json
    Uninstall {
        /// Remove from global ~/.claude/settings.json
        #[arg(long)]
        global: bool,
    },
    /// Show hook status
    Status,
}

const HOOK_MARKER: &str = "qalam-context";

pub async fn run(action: Action) -> anyhow::Result<()> {
    match action {
        Action::Install { global, cursor, copilot, mcp, all } => {
            install(global).await?;
            if cursor || all  { install_cursor()?; }
            if copilot || all { install_copilot()?; }
            if mcp || all     { install_mcp_json()?; }
            Ok(())
        }
        Action::Uninstall { global } => uninstall(global).await,
        Action::Status => status().await,
    }
}

async fn install(global: bool) -> anyhow::Result<()> {
    let settings_path = settings_file(global)?;
    let mut settings = load_settings(&settings_path)?;

    // Build the hook entry
    let qalam_bin = current_binary()?;
    let hook_command = format!(
        r#"if qalam_out=$("{}" context 2>/dev/null); then echo "$qalam_out"; fi"#,
        qalam_bin.display()
    );

    let hook_entry = json!({
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": hook_command,
            "timeout": 5
        }]
    });

    // Ensure hooks object exists
    if settings.get("hooks").is_none() {
        settings["hooks"] = json!({});
    }
    let hooks = settings["hooks"]
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks field is not an object"))?;

    if hooks.get("UserPromptSubmit").is_none() {
        hooks.insert("UserPromptSubmit".to_string(), json!([]));
    }
    let user_prompt = hooks["UserPromptSubmit"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("UserPromptSubmit is not an array"))?;

    // Idempotent: check if already installed
    let already = user_prompt.iter().any(|entry| {
        entry["hooks"]
            .as_array()
            .and_then(|h| h.first())
            .and_then(|h| h["command"].as_str())
            .map(|cmd| cmd.contains(HOOK_MARKER) || cmd.contains("qalam context"))
            .unwrap_or(false)
    });

    if already {
        println!("Qalam hook is already installed in {}", settings_path.display());
        return Ok(());
    }

    user_prompt.push(hook_entry);
    save_settings(&settings_path, &settings)?;

    let scope = if global { "global" } else { "project" };
    println!("✓ Qalam context hook installed ({scope}) in {}", settings_path.display());
    println!("  Claude Code will auto-inject qalam context before each prompt.");
    println!("  To remove: qalam hook uninstall{}", if global { " --global" } else { "" });

    Ok(())
}

async fn uninstall(global: bool) -> anyhow::Result<()> {
    let settings_path = settings_file(global)?;
    if !settings_path.exists() {
        println!("No settings file found at {}", settings_path.display());
        return Ok(());
    }

    let mut settings = load_settings(&settings_path)?;

    let removed = remove_qalam_hook(&mut settings);
    if removed {
        save_settings(&settings_path, &settings)?;
        println!("✓ Qalam hook removed from {}", settings_path.display());
    } else {
        println!("No qalam hook found in {}", settings_path.display());
    }

    Ok(())
}

async fn status() -> anyhow::Result<()> {
    for (label, path) in [
        ("project", project_settings_file()),
        ("global", global_settings_file()),
    ] {
        let Ok(path) = path else { continue };
        if !path.exists() {
            println!("  {label}: no settings file");
            continue;
        }
        let settings = load_settings(&path)?;
        let installed = has_qalam_hook(&settings);
        let mark = if installed { "✓" } else { "✗" };
        println!("  {mark} {label}: {}", path.display());
    }
    Ok(())
}

fn has_qalam_hook(settings: &Value) -> bool {
    settings["hooks"]["UserPromptSubmit"]
        .as_array()
        .map(|arr| {
            arr.iter().any(|entry| {
                entry["hooks"]
                    .as_array()
                    .and_then(|h| h.first())
                    .and_then(|h| h["command"].as_str())
                    .map(|cmd| cmd.contains("qalam context"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn remove_qalam_hook(settings: &mut Value) -> bool {
    let Some(arr) = settings["hooks"]["UserPromptSubmit"].as_array_mut() else {
        return false;
    };
    let before = arr.len();
    arr.retain(|entry| {
        !entry["hooks"]
            .as_array()
            .and_then(|h| h.first())
            .and_then(|h| h["command"].as_str())
            .map(|cmd| cmd.contains("qalam context"))
            .unwrap_or(false)
    });
    arr.len() < before
}

fn settings_file(global: bool) -> anyhow::Result<PathBuf> {
    if global { global_settings_file() } else { project_settings_file() }
}

fn global_settings_file() -> anyhow::Result<PathBuf> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
    Ok(PathBuf::from(home).join(".claude").join("settings.json"))
}

fn project_settings_file() -> anyhow::Result<PathBuf> {
    let root = env::current_dir()?;
    let dir = root.join(".claude");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("settings.json"))
}

fn load_settings(path: &PathBuf) -> anyhow::Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_settings(path: &PathBuf, value: &Value) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content + "\n")?;
    Ok(())
}

fn current_binary() -> anyhow::Result<PathBuf> {
    std::env::current_exe().map_err(Into::into)
}

fn install_cursor() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let path = root.join(".cursorrules");
    let marker = "# qalam: spec-driven context";
    let block = format!(
        "\n{marker}\n\
        # This project uses Qalam for spec-driven development.\n\
        # Run `qalam context` to see active specs, tasks, and patterns.\n\
        # Artifacts live in .qalam/:\n\
        #   rfcs/     — why we're building this\n\
        #   specs/    — what to build (acceptance criteria, contracts)\n\
        #   tasks/    — per-service implementation checklists\n\
        #   skills/   — coding patterns and conventions\n\
        # Always check .qalam/specs/ for the authoritative spec before implementing.\n"
    );

    if path.exists() {
        let existing = std::fs::read_to_string(&path)?;
        if existing.contains(marker) {
            println!("  Cursor: already installed (.cursorrules)");
            return Ok(());
        }
        std::fs::write(&path, format!("{}\n{}", existing.trim_end(), block))?;
    } else {
        std::fs::write(&path, block.trim_start())?;
    }
    println!("✓ Cursor: appended qalam context to .cursorrules");
    Ok(())
}

fn install_copilot() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let dir = root.join(".github");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("copilot-instructions.md");
    let marker = "<!-- qalam: spec-driven context -->";
    let block = format!(
        "\n{marker}\n\
        ## Qalam Spec Context\n\
        This project uses Qalam for spec-driven development. Before implementing:\n\
        1. Check `.qalam/specs/` for the authoritative spec (acceptance criteria, API contracts)\n\
        2. Check `.qalam/tasks/SPEC-XXX/<service>.md` for your service's task checklist\n\
        3. Review `.qalam/skills/` for project coding patterns\n\
        4. Verify against `.qalam/testplans/` after implementation\n"
    );

    if path.exists() {
        let existing = std::fs::read_to_string(&path)?;
        if existing.contains(marker) {
            println!("  Copilot: already installed (.github/copilot-instructions.md)");
            return Ok(());
        }
        std::fs::write(&path, format!("{}\n{}", existing.trim_end(), block))?;
    } else {
        std::fs::write(&path, block.trim_start())?;
    }
    println!("✓ Copilot: appended qalam context to .github/copilot-instructions.md");
    Ok(())
}

fn install_mcp_json() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let path = root.join(".mcp.json");
    let qalam_bin = current_binary()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "qalam".to_string());

    let mut config: Value = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    };

    if config.get("mcpServers").is_none() {
        config["mcpServers"] = json!({});
    }
    if config["mcpServers"].get("qalam").is_some() {
        println!("  MCP: qalam already in .mcp.json");
        return Ok(());
    }

    config["mcpServers"]["qalam"] = json!({
        "command": qalam_bin,
        "args": ["serve"]
    });

    std::fs::write(&path, serde_json::to_string_pretty(&config)? + "\n")?;
    println!("✓ MCP: added qalam serve to .mcp.json");
    Ok(())
}
