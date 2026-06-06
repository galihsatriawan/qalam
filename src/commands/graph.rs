use std::collections::HashMap;
use std::env;
use clap::ValueEnum;
use crate::config::QALAM_DIR;
use crate::commands::spec::Spec;

#[derive(Debug, Clone, ValueEnum)]
pub enum GraphFormat {
    Ascii,
    Mermaid,
}

pub async fn run_graph(format: GraphFormat) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let specs = load_all_specs(&root.join(QALAM_DIR))?;
    if specs.is_empty() {
        println!("No specs found. Run: qalam spec generate --from RFC-001");
        return Ok(());
    }
    match format {
        GraphFormat::Ascii => print_ascii(&specs),
        GraphFormat::Mermaid => print_mermaid(&specs),
    }
    Ok(())
}

pub async fn run_impact(service: Option<&str>, rfc: Option<&str>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);
    let specs = load_all_specs(&qalam_dir)?;

    match (service, rfc) {
        (Some(svc), _) => {
            let matching: Vec<&Spec> = specs.iter()
                .filter(|s| s.services.iter().any(|sv| sv == svc))
                .collect();
            if matching.is_empty() {
                println!("No specs found touching service '{svc}'.");
                return Ok(());
            }
            println!("Impact analysis — service: {svc}\n");
            for s in matching {
                let m = if matches!(s.status.as_str(), "shipped"|"closed"|"done") { "✓" } else { "○" };
                println!("  {m} {} — {} [{}]", s.id, s.feature, s.status);
                let task_file = qalam_dir.join("tasks").join(&s.id).join(format!("{svc}.md"));
                let testplan = qalam_dir.join("testplans").join(format!("{}-testplan.md", s.id));
                println!("       RFC: {}  tasks: {}  testplan: {}",
                    s.rfc,
                    if task_file.exists() { "✓" } else { "✗" },
                    if testplan.exists() { "✓" } else { "✗" },
                );
                if !s.depends_on.is_empty() {
                    println!("       depends_on: {}", s.depends_on.join(", "));
                }
            }
        }
        (_, Some(rfc_id)) => {
            let rfc_upper = rfc_id.to_uppercase();
            let matching: Vec<&Spec> = specs.iter()
                .filter(|s| s.rfc.to_uppercase().starts_with(&rfc_upper))
                .collect();
            if matching.is_empty() {
                println!("No specs derived from '{rfc_id}'.");
                return Ok(());
            }
            println!("Impact analysis — RFC: {rfc_id}\n");
            for s in matching {
                let m = if matches!(s.status.as_str(), "shipped"|"closed"|"done") { "✓" } else { "○" };
                println!("  {m} {} — {}", s.id, s.feature);
                if !s.services.is_empty() {
                    println!("       services: {}", s.services.join(", "));
                }
                for svc in &s.services {
                    let tf = qalam_dir.join("tasks").join(&s.id).join(format!("{svc}.md"));
                    println!("       {} tasks/{}/{}.md", if tf.exists() { "✓" } else { "✗" }, s.id, svc);
                }
                let testplan = qalam_dir.join("testplans").join(format!("{}-testplan.md", s.id));
                println!("       {} testplans/{}-testplan.md", if testplan.exists() { "✓" } else { "✗" }, s.id);
            }
        }
        (None, None) => anyhow::bail!("Specify --service <name> or --rfc <id>"),
    }
    Ok(())
}

pub fn load_all_specs(qalam_dir: &std::path::Path) -> anyhow::Result<Vec<Spec>> {
    let specs_dir = qalam_dir.join("specs");
    if !specs_dir.exists() { return Ok(vec![]); }
    let mut entries: Vec<_> = std::fs::read_dir(&specs_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("yaml"))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    Ok(entries.into_iter()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|c| serde_yaml::from_str::<Spec>(&c).ok())
        .collect())
}

fn print_ascii(specs: &[Spec]) {
    let mut rfc_map: HashMap<String, Vec<&Spec>> = HashMap::new();
    for s in specs { rfc_map.entry(s.rfc.clone()).or_default().push(s); }
    let mut rfcs: Vec<_> = rfc_map.keys().cloned().collect();
    rfcs.sort();

    println!("Qalam Project Graph");
    println!("═══════════════════");
    for rfc in &rfcs {
        let spec_list = &rfc_map[rfc];
        println!("\n[RFC] {rfc}");
        for (i, s) in spec_list.iter().enumerate() {
            let is_last_spec = i == spec_list.len() - 1;
            let branch = if is_last_spec { "└─" } else { "├─" };
            let m = if matches!(s.status.as_str(), "shipped"|"closed"|"done") { "✓" } else { "○" };
            let dep = if s.depends_on.is_empty() {
                String::new()
            } else {
                format!("  ← {}", s.depends_on.join(", "))
            };
            println!("  {branch} {m} {} — {}{}", s.id, s.feature, dep);
            let indent = if is_last_spec { "     " } else { "  │  " };
            for (j, svc) in s.services.iter().enumerate() {
                let sb = if j == s.services.len() - 1 { "└─" } else { "├─" };
                println!("  {indent}{sb} ⬡ {svc}");
            }
        }
    }
}

fn print_mermaid(specs: &[Spec]) {
    println!("```mermaid");
    println!("graph TD");
    for s in specs {
        let rid = s.rfc.replace('-', "_");
        let sid = s.id.replace('-', "_");
        println!("  {rid}[\"{}\"] --> {sid}[\"{}\"]", s.rfc, s.id);
        for svc in &s.services {
            let sv = svc.replace(['-', '/'], "_");
            println!("  {sid}[\"{}\"] --> svc_{sv}[\"⬡ {svc}\"]", s.id);
        }
        for dep in &s.depends_on {
            let di = dep.replace('-', "_");
            println!("  {sid}[\"{}\"] -.depends_on.-> {di}[\"{dep}\"]", s.id);
        }
    }
    println!("```");
    println!("\n# Paste into https://mermaid.live to visualize");
}
