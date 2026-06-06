use std::env;
use crate::config::QALAM_DIR;

pub async fn run_edit(id: &str) -> anyhow::Result<()> {
    let path = find_artifact(id)?;
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());
    std::process::Command::new(&editor).arg(&path).status()?;
    Ok(())
}

pub async fn run_diff(id: &str) -> anyhow::Result<()> {
    let path = find_artifact(id)?;
    let ok = std::process::Command::new("git")
        .args(["diff", "--", path.to_str().unwrap_or("")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        println!("  (no diff or not in a git repo)");
    }
    Ok(())
}

pub async fn run_log(id: &str) -> anyhow::Result<()> {
    let path = find_artifact(id)?;
    let ok = std::process::Command::new("git")
        .args(["log", "--oneline", "--follow", "--", path.to_str().unwrap_or("")])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        println!("  (no history or not in a git repo)");
    }
    Ok(())
}

pub async fn run_commit() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    std::process::Command::new("git")
        .args(["add", qalam_dir.to_str().unwrap_or(".qalam")])
        .status()?;

    let status_out = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--", ".qalam/"])
        .output()?;
    let changed = String::from_utf8_lossy(&status_out.stdout);
    let files: Vec<&str> = changed.lines().filter(|l| !l.is_empty()).collect();

    if files.is_empty() {
        println!("  Nothing staged in .qalam/");
        return Ok(());
    }

    let msg = if files.len() == 1 {
        let name = files[0]
            .trim_start_matches(".qalam/")
            .trim_start_matches('/');
        format!("chore(qalam): update {name}")
    } else {
        format!("chore(qalam): update {} artifacts", files.len())
    };

    let result = std::process::Command::new("git")
        .args(["commit", "-m", &msg])
        .status()?;

    if result.success() {
        println!("✓ {msg}");
    } else {
        anyhow::bail!("git commit failed");
    }
    Ok(())
}

pub fn find_artifact(id: &str) -> anyhow::Result<std::path::PathBuf> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    for subdir in &["rfcs", "specs", "testplans"] {
        let dir = qalam_dir.join(subdir);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_name().to_string_lossy().starts_with(id) {
                    return Ok(entry.path());
                }
            }
        }
    }
    anyhow::bail!("No artifact found for id '{}'", id)
}
