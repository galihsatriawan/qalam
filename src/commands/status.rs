use std::env;
use crate::config::QALAM_DIR;

pub async fn run() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    if !qalam_dir.exists() {
        anyhow::bail!("Not a qalam project. Run: qalam init");
    }

    println!("qalam status — {}\n", root.display());

    print_section("RFCs", &qalam_dir.join("rfcs"), |content| {
        extract_status(content).unwrap_or_else(|| "Draft".to_string())
    });

    print_section_plain("Specs", &qalam_dir.join("specs"));

    // Tasks: count per spec
    let tasks_dir = qalam_dir.join("tasks");
    if tasks_dir.exists() {
        let mut specs: Vec<_> = std::fs::read_dir(&tasks_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        specs.sort_by_key(|e| e.file_name());

        if !specs.is_empty() {
            println!("Tasks");
            for spec in &specs {
                let spec_id = spec.file_name().to_string_lossy().to_string();
                let count = std::fs::read_dir(spec.path())
                    .ok()
                    .map(|d| d.count())
                    .unwrap_or(0);
                println!("  {spec_id}/ ({count} service{})", if count == 1 { "" } else { "s" });
            }
            println!();
        }
    }

    print_section_plain("Testplans", &qalam_dir.join("testplans"));
    print_skills(&qalam_dir.join("skills"));

    Ok(())
}

fn print_section(label: &str, dir: &std::path::Path, badge: impl Fn(&str) -> String) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    files.sort_by_key(|e| e.file_name());

    if files.is_empty() {
        return;
    }

    println!("{} ({})", label, files.len());
    for f in &files {
        let name = f.file_name().to_string_lossy().to_string();
        let content = std::fs::read_to_string(f.path()).unwrap_or_default();
        let b = badge(&content);
        let marker = status_marker(&b);
        println!("  {marker} {name}  [{b}]");
    }
    println!();
}

fn print_section_plain(label: &str, dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();
    files.sort_by_key(|e| e.file_name());

    if files.is_empty() {
        return;
    }

    println!("{} ({})", label, files.len());
    for f in &files {
        println!("  {}", f.file_name().to_string_lossy());
    }
    println!();
}

fn print_skills(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut skills: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    skills.sort_by_key(|e| e.file_name());

    if skills.is_empty() {
        return;
    }

    println!("Skills ({})", skills.len());
    for s in &skills {
        let name = s.file_name().to_string_lossy().to_string();
        let desc = read_skill_desc(&s.path().join("skill.yaml")).unwrap_or_default();
        if desc.is_empty() {
            println!("  {name}");
        } else {
            println!("  {name} — {desc}");
        }
    }
    println!();
}

fn extract_status(content: &str) -> Option<String> {
    let mut in_status = false;
    for line in content.lines() {
        if line == "## Status" {
            in_status = true;
            continue;
        }
        if in_status {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
            if trimmed.starts_with("## ") {
                break;
            }
        }
    }
    None
}

fn status_marker(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        s if s.contains("accepted") || s.contains("published") => "✓",
        s if s.contains("draft") => "○",
        s if s.contains("reject") || s.contains("superseded") => "✗",
        _ => "·",
    }
}

fn read_skill_desc(path: &std::path::Path) -> anyhow::Result<String> {
    let content = std::fs::read_to_string(path)?;
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
