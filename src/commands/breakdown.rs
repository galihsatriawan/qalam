use std::env;
use crate::config::QALAM_DIR;
use crate::llm;
use super::spec::Spec;

pub async fn run(spec_id: &str, ai: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let specs_dir = root.join(QALAM_DIR).join("specs");
    let tasks_dir = root.join(QALAM_DIR).join("tasks").join(spec_id);

    let spec = load_spec(&specs_dir, spec_id)?;
    let spec_yaml = std::fs::read_to_string(
        std::fs::read_dir(&specs_dir)?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with(spec_id))
            .map(|e| e.path())
            .unwrap_or_default()
    ).unwrap_or_default();

    std::fs::create_dir_all(&tasks_dir)?;

    for service in &spec.services {
        let filename = format!("{}.md", service);
        let mut content = task_template(&spec, service);

        if ai {
            print!("  AI notes for {service}... ");
            match llm::suggest_task_notes(&spec_yaml, service).await {
                Ok(notes) => {
                    content = content.replace("<!-- implementation notes -->", &notes);
                    println!("✓");
                }
                Err(e) => println!("✗ ({e})"),
            }
        }

        std::fs::write(tasks_dir.join(&filename), content)?;
        println!("✓ Created .qalam/tasks/{}/{}", spec_id, filename);
    }

    if spec.services.is_empty() {
        println!("No services defined in spec. Add services to .qalam/specs/{spec_id}.yaml");
    }

    Ok(())
}

fn task_template(spec: &Spec, service: &str) -> String {
    let criteria = spec.acceptance_criteria
        .iter()
        .map(|c| format!("- [ ] {}", c))
        .collect::<Vec<_>>()
        .join("\n");

    let contracts: Vec<_> = spec.contracts
        .iter()
        .filter(|c| c.service == service)
        .collect();

    let contracts_section = if contracts.is_empty() {
        "None".to_string()
    } else {
        contracts.iter()
            .map(|c| format!("- `{}`", c.endpoint))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "# Task: {} — {}\n\
        \n\
        RFC: {}\n\
        Spec: {}\n\
        \n\
        ## Acceptance Criteria\n\
        {}\n\
        \n\
        ## Contracts\n\
        {}\n\
        \n\
        ## Notes\n\
        <!-- implementation notes -->\n",
        spec.feature, service, spec.rfc, spec.id, criteria, contracts_section
    )
}

fn load_spec(dir: &std::path::Path, id: &str) -> anyhow::Result<Spec> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(id) {
            let content = std::fs::read_to_string(entry.path())?;
            return Ok(serde_yaml::from_str(&content)?);
        }
    }
    anyhow::bail!("Spec '{}' not found in .qalam/specs/", id)
}
