use std::collections::{HashMap, HashSet};
use std::env;
use crate::config::QALAM_DIR;
use crate::commands::spec::Spec;
use crate::commands::graph::load_all_specs;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum MetricsExport {
    Csv,
    Json,
}

pub async fn run_audit(service: Option<&str>, tag: Option<&str>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);
    let all_specs = load_all_specs(&qalam_dir)?;

    let specs: Vec<&Spec> = all_specs.iter().filter(|s| {
        service.map(|svc| s.services.iter().any(|sv| sv == svc)).unwrap_or(true)
            && tag.map(|t| s.tags.iter().any(|tg| tg == t)).unwrap_or(true)
    }).collect();

    if specs.is_empty() {
        match (service, tag) {
            (Some(svc), Some(t)) => println!("No specs for service '{svc}' with tag '{t}'."),
            (Some(svc), None)    => println!("No specs for service '{svc}'."),
            (None, Some(t))      => println!("No specs with tag '{t}'."),
            _                    => println!("No specs found."),
        }
        return Ok(());
    }

    let header = match (service, tag) {
        (Some(svc), Some(t)) => format!("Audit — service: {svc}, tag: {t}"),
        (Some(svc), None)    => format!("Audit — service: {svc}"),
        (None, Some(t))      => format!("Audit — tag: {t}"),
        _                    => "Audit — all specs".to_string(),
    };
    println!("{header}");
    println!("{}\n", "─".repeat(header.len()));

    let accepted_rfcs = accepted_rfc_ids(&qalam_dir);

    for s in &specs {
        let sm = if matches!(s.status.as_str(), "shipped"|"closed"|"done") { "✓" } else { "○" };
        let rm = if accepted_rfcs.contains(&s.rfc) { "✓" } else { "○" };
        println!("  {sm} {} — {} [{}]", s.id, s.feature, s.status);
        println!("       {rm} RFC: {}  services: {}",
            s.rfc,
            if s.services.is_empty() { "(none)".to_string() } else { s.services.join(", ") },
        );
        if !s.tags.is_empty() {
            println!("       tags: {}", s.tags.join(", "));
        }
        if !s.depends_on.is_empty() {
            println!("       depends_on: {}", s.depends_on.join(", "));
        }
        println!();
    }
    println!("Total: {} spec(s)", specs.len());
    Ok(())
}

pub async fn run_metrics(service: Option<&str>, export: Option<MetricsExport>) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);
    let all_specs = load_all_specs(&qalam_dir)?;

    let specs: Vec<&Spec> = if let Some(svc) = service {
        all_specs.iter().filter(|s| s.services.iter().any(|sv| sv == svc)).collect()
    } else {
        all_specs.iter().collect()
    };

    let total_rfcs  = count_dir(&qalam_dir.join("rfcs"));
    let accepted    = accepted_rfc_ids(&qalam_dir).len();
    let total_specs = specs.len();
    let shipped     = specs.iter().filter(|s| matches!(s.status.as_str(), "shipped"|"closed"|"done")).count();
    let active      = total_specs - shipped;

    let with_tasks = specs.iter().filter(|s| {
        let td = qalam_dir.join("tasks").join(&s.id);
        td.exists() && std::fs::read_dir(&td).map(|d| d.count() > 0).unwrap_or(false)
    }).count();
    let with_plan = specs.iter().filter(|s| {
        qalam_dir.join("testplans").join(format!("{}-testplan.md", s.id)).exists()
    }).count();

    let mut svc_counts: HashMap<&str, usize> = HashMap::new();
    for s in &specs {
        for sv in &s.services { *svc_counts.entry(sv.as_str()).or_insert(0) += 1; }
    }
    let mut hotspots: Vec<(&&str, &usize)> = svc_counts.iter().collect();
    hotspots.sort_by(|a, b| b.1.cmp(a.1));

    match export {
        Some(MetricsExport::Json) => {
            let data = serde_json::json!({
                "rfcs":   { "total": total_rfcs, "accepted": accepted },
                "specs":  {
                    "total": total_specs, "active": active, "shipped": shipped,
                    "with_tasks": with_tasks, "with_testplan": with_plan,
                    "coverage_pct": if total_specs > 0 { with_tasks * 100 / total_specs } else { 0 }
                },
                "service_hotspots": hotspots.iter().map(|(s, c)| serde_json::json!({"service": s, "count": c})).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Some(MetricsExport::Csv) => {
            println!("metric,value");
            println!("rfcs_total,{total_rfcs}");
            println!("rfcs_accepted,{accepted}");
            println!("specs_total,{total_specs}");
            println!("specs_active,{active}");
            println!("specs_shipped,{shipped}");
            println!("specs_with_tasks,{with_tasks}");
            println!("specs_with_testplan,{with_plan}");
        }
        None => {
            let title = service.map(|s| format!("Metrics — {s}")).unwrap_or_else(|| "Metrics".to_string());
            println!("{title}");
            println!("{}\n", "─".repeat(title.len()));
            println!("RFCs");
            println!("  Total:    {total_rfcs}");
            if total_rfcs > 0 {
                println!("  Accepted: {accepted} ({:.0}%)", accepted as f64 / total_rfcs as f64 * 100.0);
            }
            println!("\nSpecs");
            println!("  Total:   {total_specs}  (active: {active}, shipped: {shipped})");
            if total_specs > 0 {
                println!("  Tasks:   {with_tasks}/{total_specs} ({:.0}%)", with_tasks as f64 / total_specs as f64 * 100.0);
                println!("  Plans:   {with_plan}/{total_specs} ({:.0}%)",  with_plan  as f64 / total_specs as f64 * 100.0);
            }
            if !hotspots.is_empty() {
                println!("\nService Hotspots");
                for (svc, count) in hotspots.iter().take(5) {
                    println!("  {:35} {}", svc, count);
                }
            }
        }
    }
    Ok(())
}

pub fn accepted_rfc_ids(qalam_dir: &std::path::Path) -> HashSet<String> {
    let rfcs_dir = qalam_dir.join("rfcs");
    let Ok(entries) = std::fs::read_dir(&rfcs_dir) else { return HashSet::new() };
    entries.filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let content = std::fs::read_to_string(e.path()).ok()?;
            let is_accepted = content.lines()
                .skip_while(|l| *l != "## Status")
                .nth(1)
                .map(|l| {
                    let lower = l.to_lowercase();
                    lower.contains("accepted") || lower.contains("published")
                })
                .unwrap_or(false);
            if !is_accepted { return None; }
            // "RFC-001-gopay-payment.md" → "RFC-001"
            let stem = name.trim_end_matches(".md");
            let rfc_id = stem.splitn(3, '-').take(2).collect::<Vec<_>>().join("-");
            Some(rfc_id)
        })
        .collect()
}

fn count_dir(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).count())
        .unwrap_or(0)
}
