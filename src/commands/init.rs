use std::env;
use crate::config::{Config, QALAM_DIR};
use crate::scanner;
use crate::llm;

pub async fn run(ai: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    if qalam_dir.exists() {
        println!("qalam already initialized in this repository.");
        return Ok(());
    }

    std::fs::create_dir_all(qalam_dir.join("rfcs"))?;
    std::fs::create_dir_all(qalam_dir.join("specs"))?;
    std::fs::create_dir_all(qalam_dir.join("tasks"))?;
    std::fs::create_dir_all(qalam_dir.join("testplans"))?;
    std::fs::create_dir_all(qalam_dir.join("skills"))?;

    let config = Config::default();
    config.save(&root)?;

    println!("✓ Initialized qalam in {}", root.display());
    println!("  .qalam/");
    println!("    rfcs/");
    println!("    specs/");
    println!("    tasks/");
    println!("    testplans/");
    println!("    skills/");
    println!("    qalam.yaml");

    // Auto-detect tech stack and scaffold skills
    let stacks = scanner::detect(&root);
    if stacks.is_empty() {
        println!("\nNo tech stack detected. Add skills manually: qalam skill install <name>");
    } else {
        println!("\nDetected tech stacks:");
        for stack in &stacks {
            let skill_dir = qalam_dir.join("skills").join(stack.name());
            std::fs::create_dir_all(&skill_dir)?;

            let manifest = format!(
                "name: {}\ndescription: \"{}\"\nversion: \"0.1.0\"\nauthor: \"auto-detected\"\n",
                stack.name(),
                stack.description()
            );
            std::fs::write(skill_dir.join("skill.yaml"), manifest)?;
            std::fs::write(skill_dir.join("context.md"), stack.context_md())?;

            println!("  ✓ {} → .qalam/skills/{}/", stack.name(), stack.name());
        }
    }

    // AI-assisted analysis
    if ai {
        match llm::analyze_project(&root, &stacks).await {
            Ok(insights) => {
                println!("\nAI analysis:");
                for (skill_name, extra_context) in &insights {
                    let skill_dir = qalam_dir.join("skills").join(skill_name);
                    if skill_dir.exists() {
                        let ctx_path = skill_dir.join("context.md");
                        let existing = std::fs::read_to_string(&ctx_path).unwrap_or_default();
                        let updated = format!(
                            "{existing}\n## Project-Specific Patterns (AI-inferred)\n\n{extra_context}\n"
                        );
                        std::fs::write(&ctx_path, updated)?;
                        println!("  ✓ Enhanced {skill_name} skill with project patterns");
                    } else {
                        // New skill suggested by AI
                        std::fs::create_dir_all(&skill_dir)?;
                        let manifest = format!(
                            "name: {skill_name}\ndescription: \"AI-inferred\"\nversion: \"0.1.0\"\nauthor: \"ai\"\n"
                        );
                        std::fs::write(skill_dir.join("skill.yaml"), manifest)?;
                        std::fs::write(
                            skill_dir.join("context.md"),
                            format!("# Skill: {skill_name}\n\n{extra_context}\n"),
                        )?;
                        println!("  ✓ Created AI-inferred skill: {skill_name}");
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: AI analysis failed ({e}). Continuing with detected stacks.");
            }
        }
    } else if !stacks.is_empty() {
        println!("\nTip: Run with --ai to get project-specific pattern analysis.");
    }

    println!("\nEdit .qalam/skills/<name>/context.md to customize patterns.");

    // Detect codebase-memory-mcp
    let cmcp_ok = std::process::Command::new("which")
        .arg("codebase-memory-mcp")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if cmcp_ok {
        println!("\ncodebase-memory-mcp detected — indexing project for code intelligence...");
        let idx = std::process::Command::new("codebase-memory-mcp")
            .args(["cli", "index_repository",
                  &format!("{{\"repo_path\":\"{}\"}}", root.display())])
            .output();
        match idx {
            Ok(o) if o.status.success() => println!("✓ codebase-memory-mcp index complete"),
            _ => println!("  (index skipped — run: codebase-memory-mcp cli index_repository '{{\"repo_path\":\"{}\"}}' )", root.display()),
        }
    } else {
        println!("\nTip: Install codebase-memory-mcp for ~120x token reduction on code queries.");
        println!("     curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash -s -- --ui");
    }

    Ok(())
}
