use std::env;

#[derive(clap::Subcommand)]
pub enum Action {
    /// Install qalam doctor as git pre-commit hook
    Install,
    /// Remove qalam doctor from git pre-commit hook
    Uninstall,
    /// Show git hook status
    Status,
}

pub async fn run(action: Action) -> anyhow::Result<()> {
    match action {
        Action::Install => install(),
        Action::Uninstall => uninstall(),
        Action::Status => status(),
    }
}

fn hook_path() -> anyhow::Result<std::path::PathBuf> {
    let root = env::current_dir()?;
    let git_dir = root.join(".git");
    anyhow::ensure!(git_dir.exists(), "Not a git repository (no .git directory)");
    Ok(git_dir.join("hooks").join("pre-commit"))
}

fn install() -> anyhow::Result<()> {
    let path = hook_path()?;
    let qalam_lines = "# qalam spec health check\nqalam doctor\n";

    if path.exists() {
        let existing = std::fs::read_to_string(&path)?;
        if existing.contains("qalam doctor") {
            println!("  qalam doctor already in pre-commit hook.");
            return Ok(());
        }
        std::fs::write(&path, format!("{}\n{}", existing.trim_end(), qalam_lines))?;
        println!("✓ Appended 'qalam doctor' to existing pre-commit hook");
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, format!("#!/bin/sh\n{}", qalam_lines))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
        println!("✓ Installed pre-commit hook: runs 'qalam doctor' before each commit");
        println!("  Location: {}", path.display());
    }
    Ok(())
}

fn uninstall() -> anyhow::Result<()> {
    let path = hook_path()?;
    if !path.exists() {
        println!("  No pre-commit hook found.");
        return Ok(());
    }
    let content = std::fs::read_to_string(&path)?;
    if !content.contains("qalam doctor") {
        println!("  qalam doctor not in pre-commit hook.");
        return Ok(());
    }
    let updated: String = content.lines()
        .filter(|l| !l.contains("qalam doctor") && !l.contains("qalam spec health check"))
        .collect::<Vec<_>>()
        .join("\n") + "\n";

    let has_real_commands = updated.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .count() > 0;

    if !has_real_commands {
        std::fs::remove_file(&path)?;
        println!("✓ Removed pre-commit hook");
    } else {
        std::fs::write(&path, updated)?;
        println!("✓ Removed qalam doctor from pre-commit hook");
    }
    Ok(())
}

fn status() -> anyhow::Result<()> {
    let path = match hook_path() {
        Ok(p) => p,
        Err(_) => {
            println!("✗ Not a git repository");
            return Ok(());
        }
    };
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        if content.contains("qalam doctor") {
            println!("✓ pre-commit hook installed (runs qalam doctor)");
            println!("  Location: {}", path.display());
        } else {
            println!("○ pre-commit hook exists but does not run qalam doctor");
            println!("  Run: qalam git-hook install");
        }
    } else {
        println!("✗ pre-commit hook not installed");
        println!("  Run: qalam git-hook install");
    }
    Ok(())
}
