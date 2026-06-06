use std::env;
use crate::config::QALAM_DIR;
use crate::llm;
use super::spec::Spec;

pub async fn run(spec_id: &str, ai: bool) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let specs_dir = root.join(QALAM_DIR).join("specs");
    let testplans_dir = root.join(QALAM_DIR).join("testplans");

    let spec = load_spec(&specs_dir, spec_id)?;
    let spec_yaml = std::fs::read_to_string(
        std::fs::read_dir(&specs_dir)?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with(spec_id))
            .map(|e| e.path())
            .unwrap_or_default()
    ).unwrap_or_default();

    let filename = format!("{}-testplan.md", spec_id);
    let mut content = testplan_template(&spec);

    if ai {
        print!("Generating edge/negative cases with AI... ");
        match llm::generate_test_cases(&spec_yaml).await {
            Ok(cases) => {
                content = content
                    .replace("<!-- add edge cases -->", &cases
                        .lines()
                        .take_while(|l| !l.starts_with("## Negative"))
                        .collect::<Vec<_>>().join("\n"))
                    .replace("<!-- add negative cases -->", &cases
                        .lines()
                        .skip_while(|l| !l.starts_with("## Negative"))
                        .skip(1)
                        .collect::<Vec<_>>().join("\n"));
                println!("✓");
            }
            Err(e) => println!("✗ ({e})"),
        }
    }

    std::fs::write(testplans_dir.join(&filename), content)?;
    println!("✓ Created .qalam/testplans/{}", filename);
    Ok(())
}

fn testplan_template(spec: &Spec) -> String {
    let happy_path = spec.acceptance_criteria
        .iter()
        .map(|c| format!("- [ ] {}", c))
        .collect::<Vec<_>>()
        .join("\n");

    let contracts = if spec.contracts.is_empty() {
        "<!-- no contracts defined -->".to_string()
    } else {
        spec.contracts
            .iter()
            .map(|c| format!("- [ ] `{}` on {} matches spec schema", c.endpoint, c.service))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "# Test Plan: {}\n\
        \n\
        Spec: {} | RFC: {}\n\
        \n\
        ## Happy Path\n\
        {}\n\
        \n\
        ## Edge Cases\n\
        <!-- add edge cases -->\n\
        \n\
        ## Negative Cases\n\
        <!-- add negative cases -->\n\
        \n\
        ## Contract Tests\n\
        {}\n\
        \n\
        ## Regression\n\
        - [ ] Existing functionality unaffected\n",
        spec.feature, spec.id, spec.rfc, happy_path, contracts
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
