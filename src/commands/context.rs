use clap::ValueEnum;
use std::{env, time::{Duration, UNIX_EPOCH}};
use crate::config::QALAM_DIR;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum Role {
    #[default]
    Engineer,
    Pm,
    Qa,
}

pub async fn run(role: Role, watch: bool, service: Option<String>, since: Option<String>, repo: Option<String>) -> anyhow::Result<()> {
    let since_ts = since.as_deref().map(resolve_since_ts).transpose()?;
    if let Some(repo_path) = repo {
        // Override working directory with the specified repo path
        let repo_dir = std::path::PathBuf::from(&repo_path).join(QALAM_DIR);
        if !repo_dir.exists() {
            anyhow::bail!("No .qalam/ directory found in '{}'", repo_path);
        }
        return print_context_from(role, service.as_deref(), since_ts, &repo_dir);
    }
    if watch {
        run_watch(role, service, since_ts).await
    } else {
        print_context(role, service.as_deref(), since_ts)
    }
}

fn print_context_from(role: Role, service: Option<&str>, since_ts: Option<u64>, qalam_dir: &std::path::Path) -> anyhow::Result<()> {
    // Shared impl: print context from an explicit qalam_dir
    let mut sections: Vec<String> = Vec::new();
    if let Some(svc) = service {
        if let Some(s) = service_context_section(qalam_dir, svc, since_ts) { sections.push(s); }
        if let Some(s) = skills_section(qalam_dir) { sections.push(s); }
    } else {
        match role {
            Role::Pm => {
                if let Some(s) = rfcs_section(qalam_dir, 5, since_ts) { sections.push(s); }
                if let Some(s) = specs_overview_section(qalam_dir, since_ts) { sections.push(s); }
            }
            Role::Engineer => {
                if let Some(s) = specs_section(qalam_dir, 3, since_ts) { sections.push(s); }
                if let Some(s) = skills_section(qalam_dir) { sections.push(s); }
            }
            Role::Qa => {
                if let Some(s) = specs_overview_section(qalam_dir, since_ts) { sections.push(s); }
                if let Some(s) = testplans_section(qalam_dir, 3, since_ts) { sections.push(s); }
            }
        }
    }
    if sections.is_empty() { return Ok(()); }
    let role_tag = format!("{role:?}").to_lowercase();
    let svc_tag = service.map(|s| format!(" service={s}")).unwrap_or_default();
    println!("<qalam-context role={role_tag}{svc_tag} repo={}>", qalam_dir.parent().map(|p| p.display().to_string()).unwrap_or_default());
    for section in &sections { println!("{}", section.trim_end()); println!(); }
    println!("</qalam-context>");
    Ok(())
}

// Resolve a --since value to a Unix timestamp.
// Accepts: YYYY-MM-DD date strings, or a git ref (branch/tag/commit).
fn resolve_since_ts(since: &str) -> anyhow::Result<u64> {
    // Try YYYY-MM-DD first
    if let Some(ts) = parse_date(since) {
        return Ok(ts);
    }
    // Try as git ref
    let out = std::process::Command::new("git")
        .args(["log", "--format=%ct", since, "-1"])
        .output();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if let Ok(ts) = s.parse::<u64>() {
            return Ok(ts);
        }
    }
    anyhow::bail!("Cannot parse '{}' as a date (YYYY-MM-DD) or git ref", since)
}

fn parse_date(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 { return None; }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    // Compute days since Unix epoch (1970-01-01)
    let days = days_from_epoch(y, m, d)?;
    Some(days * 86400)
}

fn days_from_epoch(y: i32, m: u32, d: u32) -> Option<u64> {
    // Gregorian calendar days since 1970-01-01
    let mut total: i64 = 0;
    for yr in 1970..y {
        total += if is_leap(yr) { 366 } else { 365 };
    }
    let month_days = [31u32, if is_leap(y) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for mi in 0..(m as usize - 1) {
        total += month_days[mi] as i64;
    }
    total += d as i64 - 1;
    if total < 0 { return None; }
    Some(total as u64)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

async fn run_watch(role: Role, service: Option<String>, since_ts: Option<u64>) -> anyhow::Result<()> {
    eprintln!("Watching .qalam/ for changes (Ctrl+C to stop)...");
    let mut last = snapshot();
    print_context(role, service.as_deref(), since_ts)?;

    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let current = snapshot();
        if current != last {
            println!("\n---\n");
            print_context(role, service.as_deref(), since_ts)?;
            last = current;
        }
    }
}

fn snapshot() -> u64 {
    let root = env::current_dir().unwrap_or_default();
    let qalam_dir = root.join(QALAM_DIR);
    dir_mtime(&qalam_dir)
}

fn dir_mtime(dir: &std::path::Path) -> u64 {
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs())
        .max()
        .unwrap_or(0)
}

fn print_context(role: Role, service: Option<&str>, since_ts: Option<u64>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    if !qalam_dir.exists() {
        return Ok(());
    }

    let mut sections: Vec<String> = Vec::new();

    if let Some(svc) = service {
        if let Some(s) = service_context_section(&qalam_dir, svc, since_ts) { sections.push(s); }
        if let Some(s) = skills_section(&qalam_dir) { sections.push(s); }
    } else {
        match role {
            Role::Pm => {
                if let Some(s) = rfcs_section(&qalam_dir, 5, since_ts) { sections.push(s); }
                if let Some(s) = specs_overview_section(&qalam_dir, since_ts) { sections.push(s); }
            }
            Role::Engineer => {
                if let Some(s) = specs_section(&qalam_dir, 3, since_ts) { sections.push(s); }
                if let Some(s) = skills_section(&qalam_dir) { sections.push(s); }
            }
            Role::Qa => {
                if let Some(s) = specs_overview_section(&qalam_dir, since_ts) { sections.push(s); }
                if let Some(s) = testplans_section(&qalam_dir, 3, since_ts) { sections.push(s); }
            }
        }
    }

    if sections.is_empty() {
        return Ok(());
    }

    let role_tag = format!("{role:?}").to_lowercase();
    let svc_tag = service.map(|s| format!(" service={s}")).unwrap_or_default();
    let since_tag = since_ts.map(|_| " filtered=since").unwrap_or_default();
    println!("<qalam-context role={role_tag}{svc_tag}{since_tag}>");
    for section in &sections {
        println!("{}", section.trim_end());
        println!();
    }
    println!("</qalam-context>");

    Ok(())
}

fn service_context_section(qalam_dir: &std::path::Path, service: &str, since_ts: Option<u64>) -> Option<String> {
    let tasks_dir = qalam_dir.join("tasks");
    let specs_dir = qalam_dir.join("specs");
    let mut block = format!("## Context for service: {service} (qalam)\n");

    let Ok(spec_dirs) = std::fs::read_dir(&tasks_dir) else { return None };
    let mut found_any = false;

    let mut spec_dirs: Vec<_> = spec_dirs.filter_map(|e| e.ok()).collect();
    spec_dirs.sort_by_key(|e| e.file_name());

    for spec_dir in &spec_dirs {
        if !spec_dir.path().is_dir() { continue; }
        let task_file = spec_dir.path().join(format!("{service}.md"));
        if !task_file.exists() { continue; }
        if let Some(ts) = since_ts {
            if !file_modified_after(&task_file, ts) { continue; }
        }
        let spec_id = spec_dir.file_name().to_string_lossy().to_string();
        if let Ok(content) = std::fs::read_to_string(&task_file) {
            block.push_str(&format!("\n### Task: {spec_id} / {service}\n{content}\n"));
            found_any = true;

            if let Some(spec_content) = find_file_in_dir(&specs_dir, &spec_id) {
                block.push_str(&format!("\n### Spec: {spec_id}\n```yaml\n{spec_content}\n```\n"));
            }
        }
    }

    if found_any { Some(block) } else { None }
}

fn find_file_in_dir(dir: &std::path::Path, prefix: &str) -> Option<String> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(prefix))
        .and_then(|e| std::fs::read_to_string(e.path()).ok())
}

fn rfcs_section(qalam_dir: &std::path::Path, limit: usize, since_ts: Option<u64>) -> Option<String> {
    let files = recent_files(&qalam_dir.join("rfcs"), limit, since_ts);
    if files.is_empty() { return None; }
    let mut block = "## RFCs (qalam)\n".to_string();
    for (name, content) in &files {
        block.push_str(&format!("\n### {name}\n{content}\n"));
    }
    Some(block)
}

fn specs_section(qalam_dir: &std::path::Path, limit: usize, since_ts: Option<u64>) -> Option<String> {
    let files = recent_files_filtered_specs(&qalam_dir.join("specs"), limit, since_ts);
    if files.is_empty() { return None; }
    let mut block = "## Active Specs (qalam)\n".to_string();
    for (name, content) in &files {
        block.push_str(&format!("\n### {name}\n```yaml\n{content}\n```\n"));
    }
    Some(block)
}

fn specs_overview_section(qalam_dir: &std::path::Path, since_ts: Option<u64>) -> Option<String> {
    let Ok(entries) = std::fs::read_dir(qalam_dir.join("specs")) else { return None };
    let mut names: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            if let Some(ts) = since_ts { file_modified_after(&e.path(), ts) } else { true }
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    if names.is_empty() { return None; }
    names.sort();
    let list = names.iter().map(|n| format!("- {n}")).collect::<Vec<_>>().join("\n");
    Some(format!("## Specs (qalam)\n\n{list}\n"))
}

fn testplans_section(qalam_dir: &std::path::Path, limit: usize, since_ts: Option<u64>) -> Option<String> {
    let files = recent_files(&qalam_dir.join("testplans"), limit, since_ts);
    if files.is_empty() { return None; }
    let mut block = "## Testplans (qalam)\n".to_string();
    for (name, content) in &files {
        block.push_str(&format!("\n### {name}\n{content}\n"));
    }
    Some(block)
}

fn skills_section(qalam_dir: &std::path::Path) -> Option<String> {
    let skills_dir = qalam_dir.join("skills");
    if !skills_dir.exists() { return None; }

    let mut entries: Vec<_> = std::fs::read_dir(&skills_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut block = String::new();
    for entry in &entries {
        let ctx_path = entry.path().join("context.md");
        if let Ok(content) = std::fs::read_to_string(&ctx_path) {
            block.push_str(&content);
            block.push('\n');
        }
    }

    if block.is_empty() { return None; }
    Some(format!("## Project Skills (qalam)\n\n{block}"))
}

fn file_modified_after(path: &std::path::Path, since_ts: u64) -> bool {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() >= since_ts)
        .unwrap_or(false)
}

fn recent_files(dir: &std::path::Path, limit: usize, since_ts: Option<u64>) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };

    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            if let Some(ts) = since_ts { file_modified_after(&e.path(), ts) } else { true }
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e))
        })
        .collect();

    files.sort_by(|a, b| b.0.cmp(&a.0));

    files.into_iter()
        .take(limit)
        .filter_map(|(_, entry)| {
            let name = entry.file_name().to_string_lossy().to_string();
            let content = std::fs::read_to_string(entry.path()).ok()?;
            Some((name, content))
        })
        .collect()
}

// Like recent_files but also filters out specs with status=shipped
fn recent_files_filtered_specs(dir: &std::path::Path, limit: usize, since_ts: Option<u64>) -> Vec<(String, String)> {
    use crate::commands::spec::Spec;
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };

    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            if let Some(ts) = since_ts { file_modified_after(&e.path(), ts) } else { true }
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e))
        })
        .collect();

    files.sort_by(|a, b| b.0.cmp(&a.0));

    files.into_iter()
        .take(limit * 2) // over-fetch to account for filtered-out shipped specs
        .filter_map(|(_, entry)| {
            let name = entry.file_name().to_string_lossy().to_string();
            let content = std::fs::read_to_string(entry.path()).ok()?;
            // Skip shipped/closed specs
            if let Ok(spec) = serde_yaml::from_str::<Spec>(&content) {
                if matches!(spec.status.as_str(), "shipped" | "closed" | "done") {
                    return None;
                }
            }
            Some((name, content))
        })
        .take(limit)
        .collect()
}
