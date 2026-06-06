use std::env;
use serde_json::{json, Value};
use crate::config::QALAM_DIR;
use crate::commands::spec::Spec;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ExportFormat {
    Json,
    Yaml,
}

pub async fn run_export(format: ExportFormat) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let qalam_dir = root.join(QALAM_DIR);

    let output = json!({
        "rfcs":      files_as_json(&qalam_dir.join("rfcs")),
        "specs":     specs_as_json(&qalam_dir.join("specs")),
        "tasks":     tasks_as_json(&qalam_dir.join("tasks")),
        "testplans": files_as_json(&qalam_dir.join("testplans")),
    });

    match format {
        ExportFormat::Json => println!("{}", serde_json::to_string_pretty(&output)?),
        ExportFormat::Yaml => println!("{}", serde_yaml::to_string(&output)?),
    }
    Ok(())
}

pub async fn run_openapi(spec_id: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let spec = load_spec(&root.join(QALAM_DIR).join("specs"), spec_id)?;

    let mut paths = serde_json::Map::new();
    for contract in &spec.contracts {
        let (method, path) = split_endpoint(&contract.endpoint);
        let entry = paths.entry(path.clone()).or_insert_with(|| json!({}));
        if let Value::Object(methods) = entry {
            methods.insert(method, json!({
                "summary": format!("{} ({})", spec.feature, contract.service),
                "tags": [contract.service],
                "operationId": sanitize_operation_id(&contract.endpoint),
                "responses": { "200": { "description": "Success" } }
            }));
        }
    }

    let doc = json!({
        "openapi": "3.0.0",
        "info": {
            "title": spec.feature,
            "version": "1.0.0",
            "description": format!("Generated from qalam spec {}. Review before use.", spec.id)
        },
        "paths": paths
    });
    println!("{}", serde_yaml::to_string(&doc)?);
    Ok(())
}

pub async fn run_postman(spec_id: &str) -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let spec = load_spec(&root.join(QALAM_DIR).join("specs"), spec_id)?;

    let items: Vec<Value> = spec.contracts.iter().map(|c| {
        let (method, path) = split_endpoint(&c.endpoint);
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        json!({
            "name": format!("{} {}", method.to_uppercase(), path),
            "request": {
                "method": method.to_uppercase(),
                "url": {
                    "raw": format!("{{{{base_url}}}}{path}"),
                    "host": ["{{base_url}}"],
                    "path": segments
                },
                "description": format!("Service: {}", c.service)
            }
        })
    }).collect();

    let collection = json!({
        "info": {
            "name": spec.feature,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": items,
        "variable": [{ "key": "base_url", "value": "http://localhost:8080", "type": "string" }]
    });
    println!("{}", serde_json::to_string_pretty(&collection)?);
    Ok(())
}

pub async fn run_sync() -> anyhow::Result<()> {
    let root = env::current_dir()?;
    let config_path = root.join(QALAM_DIR).join("qalam.yaml");
    if !config_path.exists() {
        println!("No qalam.yaml found. Nothing to sync.");
        return Ok(());
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config: Value = serde_yaml::from_str(&content)?;

    let repos = config.get("sources")
        .and_then(|s| s.get("repos"))
        .and_then(|r| r.as_array())
        .cloned()
        .unwrap_or_default();

    if repos.is_empty() {
        println!("No repos configured under sources.repos in qalam.yaml");
        return Ok(());
    }

    for repo in &repos {
        let Some(path) = repo.get("path").and_then(|p| p.as_str()) else { continue };
        let abs = if std::path::Path::new(path).is_absolute() {
            std::path::PathBuf::from(path)
        } else {
            root.join(path)
        };
        print!("Syncing {}... ", abs.display());
        let ok = std::process::Command::new("git")
            .args(["-C", abs.to_str().unwrap_or("."), "pull", "--ff-only"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        println!("{}", if ok { "✓" } else { "✗ (pull failed or not a git repo)" });
    }
    Ok(())
}

fn files_as_json(dir: &std::path::Path) -> Value {
    let Ok(entries) = std::fs::read_dir(dir) else { return json!({}) };
    let mut map = serde_json::Map::new();
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.path().is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            map.insert(name, json!(content));
        }
    }
    Value::Object(map)
}

fn specs_as_json(dir: &std::path::Path) -> Value {
    let Ok(entries) = std::fs::read_dir(dir) else { return json!({}) };
    let mut map = serde_json::Map::new();
    for entry in entries.filter_map(|e| e.ok()) {
        if entry.path().is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let v = serde_yaml::from_str::<Value>(&content).unwrap_or(json!(content));
            map.insert(name, v);
        }
    }
    Value::Object(map)
}

fn tasks_as_json(dir: &std::path::Path) -> Value {
    let Ok(spec_dirs) = std::fs::read_dir(dir) else { return json!({}) };
    let mut outer = serde_json::Map::new();
    for spec_dir in spec_dirs.filter_map(|e| e.ok()) {
        if !spec_dir.path().is_dir() { continue; }
        let spec_id = spec_dir.file_name().to_string_lossy().to_string();
        let mut inner = serde_json::Map::new();
        if let Ok(tasks) = std::fs::read_dir(spec_dir.path()) {
            for task in tasks.filter_map(|e| e.ok()) {
                if task.path().is_file() {
                    let svc = task.file_name().to_string_lossy().to_string();
                    let content = std::fs::read_to_string(task.path()).unwrap_or_default();
                    inner.insert(svc, json!(content));
                }
            }
        }
        outer.insert(spec_id, Value::Object(inner));
    }
    Value::Object(outer)
}

pub fn load_spec(specs_dir: &std::path::Path, spec_id: &str) -> anyhow::Result<Spec> {
    let entry = std::fs::read_dir(specs_dir)?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with(spec_id))
        .ok_or_else(|| anyhow::anyhow!("Spec '{}' not found", spec_id))?;
    let content = std::fs::read_to_string(entry.path())?;
    Ok(serde_yaml::from_str(&content)?)
}

pub fn split_endpoint(endpoint: &str) -> (String, String) {
    let parts: Vec<&str> = endpoint.splitn(2, ' ').collect();
    if parts.len() == 2 {
        (parts[0].to_lowercase(), parts[1].to_string())
    } else {
        ("get".to_string(), endpoint.to_string())
    }
}

fn sanitize_operation_id(endpoint: &str) -> String {
    endpoint.to_lowercase()
        .replace([' ', '/', '{', '}', '-'], "_")
        .trim_matches('_')
        .to_string()
}
