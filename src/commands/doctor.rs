use std::env;
use crate::config::QALAM_DIR;

struct Check {
    label: String,
    passed: bool,
    detail: Option<String>,
    /// Shell command that auto-fixes this issue when --fix is used
    fix_cmd: Option<String>,
}

impl Check {
    fn ok(label: impl Into<String>) -> Self {
        Self { label: label.into(), passed: true, detail: None, fix_cmd: None }
    }

    fn warn(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { label: label.into(), passed: false, detail: Some(detail.into()), fix_cmd: None }
    }

    fn fixable(label: impl Into<String>, detail: impl Into<String>, fix: impl Into<String>) -> Self {
        Self { label: label.into(), passed: false, detail: Some(detail.into()), fix_cmd: Some(fix.into()) }
    }
}

pub async fn run(fix: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);
    let mut checks: Vec<Check> = Vec::new();

    if !qalam_dir.exists() {
        println!("✗  qalam not initialized. Run: qalam init");
        return Ok(());
    }
    checks.push(Check::ok("qalam initialized (.qalam/ exists)"));

    // Required subdirs
    for subdir in &["rfcs", "specs", "tasks", "testplans", "skills"] {
        let dir = qalam_dir.join(subdir);
        if dir.exists() {
            checks.push(Check::ok(format!(".qalam/{subdir}/ exists")));
        } else {
            checks.push(Check::fixable(
                format!(".qalam/{subdir}/ missing"),
                format!("mkdir .qalam/{subdir}"),
                format!("__mkdir:{}", dir.display()),
            ));
        }
    }

    // Each RFC should have a corresponding spec
    let rfcs = list_ids(&qalam_dir.join("rfcs"), "RFC");
    let specs = list_ids(&qalam_dir.join("specs"), "SPEC");

    if rfcs.is_empty() {
        checks.push(Check::warn(
            "No RFCs found",
            "qalam rfc generate --description \"your feature\"",
        ));
    }

    for (rfc_id, rfc_num) in &rfcs {
        let has_spec = specs.iter().any(|(_, spec_num)| spec_num == rfc_num);
        if has_spec {
            checks.push(Check::ok(format!("{rfc_id} has a spec")));
        } else {
            checks.push(Check::fixable(
                format!("{rfc_id} has no spec"),
                format!("qalam spec generate --from {rfc_id}"),
                format!("qalam spec generate --from {rfc_id}"),
            ));
        }
    }

    // Each spec: tasks + testplan
    let tasks_dir = qalam_dir.join("tasks");
    let testplans_dir = qalam_dir.join("testplans");

    for (spec_id, _) in &specs {
        let spec_content = find_file(&qalam_dir.join("specs"), spec_id);
        let has_services = spec_content.as_ref().map(|c| {
            let parsed: serde_yaml::Value = serde_yaml::from_str(c).unwrap_or(serde_yaml::Value::Null);
            parsed["services"].as_sequence().map(|s| !s.is_empty()).unwrap_or(false)
        }).unwrap_or(false);

        // Check depends_on specs exist
        if let Some(content) = &spec_content {
            let parsed: serde_yaml::Value = serde_yaml::from_str(content).unwrap_or(serde_yaml::Value::Null);
            if let Some(deps) = parsed["depends_on"].as_sequence() {
                for dep in deps {
                    let dep_id = dep.as_str().unwrap_or("");
                    if !dep_id.is_empty() {
                        let dep_exists = specs.iter().any(|(id, _)| id == dep_id);
                        if dep_exists {
                            checks.push(Check::ok(format!("{spec_id} dependency {dep_id} exists")));
                        } else {
                            checks.push(Check::warn(
                                format!("{spec_id} depends on {dep_id} which doesn't exist"),
                                format!("Create {dep_id} or remove it from depends_on"),
                            ));
                        }
                    }
                }
            }
        }

        if has_services {
            let tasks_exist = tasks_dir.join(spec_id).exists();
            if tasks_exist {
                checks.push(Check::ok(format!("{spec_id} has tasks")));
            } else {
                checks.push(Check::fixable(
                    format!("{spec_id} missing tasks"),
                    format!("qalam breakdown --from {spec_id}"),
                    format!("qalam breakdown --from {spec_id}"),
                ));
            }
        } else {
            checks.push(Check::warn(
                format!("{spec_id} has no services defined"),
                format!("Edit .qalam/specs/{spec_id}-*.yaml and fill in services:"),
            ));
        }

        let testplan_exists = std::fs::read_dir(&testplans_dir)
            .ok().into_iter().flatten()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().starts_with(spec_id.as_str()));

        if testplan_exists {
            checks.push(Check::ok(format!("{spec_id} has a testplan")));
        } else {
            checks.push(Check::fixable(
                format!("{spec_id} missing testplan"),
                format!("qalam testplan --from {spec_id}"),
                format!("qalam testplan --from {spec_id}"),
            ));
        }
    }

    // Skills content check
    let skills_dir = qalam_dir.join("skills");
    if skills_dir.exists() {
        let mut skill_dirs: Vec<_> = std::fs::read_dir(&skills_dir)
            .ok().into_iter().flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        skill_dirs.sort_by_key(|e| e.file_name());

        if skill_dirs.is_empty() {
            checks.push(Check::warn("No skills installed", "qalam skill install <name>"));
        } else {
            for skill in &skill_dirs {
                let name = skill.file_name().to_string_lossy().to_string();
                let ctx_path = skill.path().join("context.md");
                if ctx_path.exists() {
                    let content = std::fs::read_to_string(&ctx_path).unwrap_or_default();
                    if content.contains("<!-- Describe what this skill provides -->") {
                        checks.push(Check::warn(
                            format!("Skill '{name}' context.md is unedited"),
                            format!("Edit .qalam/skills/{name}/context.md with real patterns"),
                        ));
                    } else {
                        checks.push(Check::ok(format!("Skill '{name}' has context")));
                    }
                } else {
                    checks.push(Check::warn(
                        format!("Skill '{name}' missing context.md"),
                        format!("Create .qalam/skills/{name}/context.md"),
                    ));
                }
            }
        }
    }

    // Hook check
    let project_hook = has_qalam_hook(&root.join(".claude").join("settings.json"));
    let global_hook = home_dir()
        .map(|h| has_qalam_hook(&h.join(".claude").join("settings.json")))
        .unwrap_or(false);

    if project_hook {
        checks.push(Check::ok("Claude Code hook installed (project)"));
    } else if global_hook {
        checks.push(Check::ok("Claude Code hook installed (global)"));
    } else {
        checks.push(Check::fixable(
            "Claude Code hook not installed",
            "qalam hook install",
            "qalam hook install",
        ));
    }

    // Stale spec detection (draft > 30 days)
    for (spec_id, _) in &specs {
        if let Some(mtime) = find_file_mtime(&qalam_dir.join("specs"), spec_id) {
            let age_days = mtime / 86400;
            let now_days = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86400)
                .unwrap_or(0);
            let days_old = now_days.saturating_sub(age_days);
            if days_old > 30 {
                // Only warn if spec is still draft
                let content = find_file(&qalam_dir.join("specs"), spec_id).unwrap_or_default();
                let is_shipped = content.contains("status: shipped")
                    || content.contains("status: closed")
                    || content.contains("status: done");
                if !is_shipped {
                    checks.push(Check::warn(
                        format!("{spec_id} has been Draft for {days_old} days"),
                        "Consider closing, shipping, or adding a depends_on if blocked",
                    ));
                }
            }
        }
    }

    // codebase-memory-mcp check
    let cmcp_installed = which_binary("codebase-memory-mcp");
    if cmcp_installed {
        checks.push(Check::ok("codebase-memory-mcp installed"));
    } else {
        checks.push(Check::warn(
            "codebase-memory-mcp not installed",
            "Install for ~120x token reduction: curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash -s -- --ui",
        ));
    }

    // Print results
    println!("qalam doctor{}\n", if fix { " --fix" } else { "" });
    let mut warnings = 0;
    let mut fixed = 0;

    for check in &checks {
        if check.passed {
            println!("  ✓  {}", check.label);
        } else {
            if fix {
                if let Some(cmd) = &check.fix_cmd {
                    if let Ok(()) = apply_fix(cmd) {
                        println!("  ✓  {} (fixed)", check.label);
                        fixed += 1;
                        continue;
                    }
                }
            }
            println!("  ✗  {}", check.label);
            if let Some(detail) = &check.detail {
                println!("     → {detail}");
            }
            warnings += 1;
        }
    }

    println!();
    if fix && fixed > 0 {
        println!("{fixed} issue{} fixed.", if fixed == 1 { "" } else { "s" });
    }
    if warnings == 0 {
        println!("All checks passed.");
    } else {
        println!("{warnings} issue{} remaining.", if warnings == 1 { "" } else { "s" });
    }

    Ok(())
}

/// Execute a fix: either a shell command or a special `__mkdir:` directive.
fn apply_fix(cmd: &str) -> anyhow::Result<()> {
    if let Some(path) = cmd.strip_prefix("__mkdir:") {
        std::fs::create_dir_all(path)?;
        return Ok(());
    }

    // Run as a qalam subcommand via the current binary
    let binary = std::env::current_exe()?;
    let args: Vec<&str> = cmd.split_whitespace().collect();
    let status = std::process::Command::new(&binary)
        .args(&args)
        .status()?;

    anyhow::ensure!(status.success(), "Fix command failed: {cmd}");
    Ok(())
}

fn list_ids(dir: &std::path::Path, prefix: &str) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with(prefix) {
                let parts: Vec<&str> = name.splitn(3, '-').collect();
                if parts.len() >= 2 {
                    let id = format!("{}-{}", parts[0], parts[1]);
                    let num = parts[1].to_string();
                    return Some((id, num));
                }
            }
            None
        })
        .collect();
    out.sort();
    out
}

fn find_file(dir: &std::path::Path, id_prefix: &str) -> Option<String> {
    std::fs::read_dir(dir).ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(id_prefix))
        .and_then(|e| std::fs::read_to_string(e.path()).ok())
}

fn has_qalam_hook(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.contains("qalam context"))
        .unwrap_or(false)
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))
        .ok().map(std::path::PathBuf::from)
}

fn find_file_mtime(dir: &std::path::Path, id_prefix: &str) -> Option<u64> {
    std::fs::read_dir(dir).ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(id_prefix))
        .and_then(|e| e.metadata().ok())
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

fn which_binary(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or_else(|_| {
            // Windows fallback
            std::process::Command::new("where")
                .arg(name)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
}
