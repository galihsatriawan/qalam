use std::path::Path;
use crate::scanner::Stack;

const ANTHROPIC_API: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-haiku-4-5-20251001";

/// Calls the Anthropic API to infer project-specific patterns.
/// Returns a list of (skill_name, extra_context_markdown) pairs.
/// Requires ANTHROPIC_API_KEY environment variable.
pub async fn analyze_project(
    root: &Path,
    stacks: &[Stack],
) -> anyhow::Result<Vec<(String, String)>> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let codebase_snapshot = build_snapshot(root, stacks);
    let stack_names: Vec<_> = stacks.iter().map(|s| s.name()).collect();

    let prompt = format!(
        "You are analyzing a software project to extract coding patterns and conventions for an AI assistant.\n\
        \n\
        Detected tech stacks: {stacks}\n\
        \n\
        Code snapshot:\n\
        {snapshot}\n\
        \n\
        Based on the code above, identify project-specific patterns, conventions, and constraints \
        that an AI coding assistant should follow. Focus on:\n\
        - Naming conventions\n\
        - Error handling patterns\n\
        - Project structure patterns\n\
        - Domain-specific terms or entities\n\
        - Any framework-specific idioms\n\
        \n\
        Respond with JSON only, in this format:\n\
        {{\"skills\": [{{\"name\": \"<stack-name>\", \"patterns\": \"<markdown text with bullet points>\"}}]}}\n\
        Only include skills for the detected stacks: {stacks}. Keep each patterns field under 300 words.",
        stacks = stack_names.join(", "),
        snapshot = codebase_snapshot,
    );

    let client = reqwest::Client::new();
    let resp = client
        .post(ANTHROPIC_API)
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": MODEL,
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    anyhow::ensure!(
        resp.status().is_success(),
        "Anthropic API error ({})",
        resp.status()
    );

    let body: serde_json::Value = resp.json().await?;
    let text = body["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Unexpected API response shape"))?;

    parse_llm_response(text)
}

fn parse_llm_response(text: &str) -> anyhow::Result<Vec<(String, String)>> {
    // Strip markdown code fences if present
    let json_text = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let value: serde_json::Value = serde_json::from_str(json_text)?;
    let skills = value["skills"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'skills' array in response"))?;

    let mut result = Vec::new();
    for skill in skills {
        let name = skill["name"].as_str().unwrap_or("").to_string();
        let patterns = skill["patterns"].as_str().unwrap_or("").to_string();
        if !name.is_empty() && !patterns.is_empty() {
            result.push((name, patterns));
        }
    }

    Ok(result)
}

/// Generate RFC sections (Problem, Options, Decision) from a description + optional PRD.
pub async fn generate_rfc_sections(description: &str, prd: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let prd_block = if prd.is_empty() { String::new() } else { format!("\n\nPRD content:\n{prd}") };
    let prompt = format!(
        "You are helping draft an RFC for a software feature.\n\
        Feature description: {description}{prd_block}\n\n\
        Write concise, engineering-quality content for these sections:\n\
        1. Problem — what problem are we solving?\n\
        2. Options Considered — 2-3 options with tradeoffs\n\
        3. Decision — what did we decide and why?\n\
        4. Tradeoffs — what are we giving up?\n\
        5. Affected Services — infer from the description\n\n\
        Format as Markdown. Be specific, avoid filler. Max 400 words total."
    );
    call_api(&api_key, &prompt, 1024).await
}

/// Suggest services, acceptance criteria, and contracts for a spec from RFC content.
/// Returns a JSON string: {{\"services\":[...],\"acceptance_criteria\":[...],\"contracts\":[...]}}
pub async fn suggest_spec_fields(rfc_content: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let prompt = format!(
        "You are extracting structured information from an RFC to populate a spec YAML.\n\
        RFC content:\n{rfc_content}\n\n\
        Extract and respond with JSON only:\n\
        {{\n\
        \"services\": [\"service-name\", ...],\n\
        \"acceptance_criteria\": [\"criterion text\", ...],\n\
        \"contracts\": [{{\"service\": \"name\", \"endpoint\": \"METHOD /path\"}}, ...]\n\
        }}\n\
        Infer service names from the RFC (look for \"Affected Services\" and system names mentioned).\n\
        Write acceptance criteria as testable, user-facing statements.\n\
        Keep endpoint paths RESTful. Max 6 acceptance criteria, max 4 contracts."
    );

    let raw = call_api(&api_key, &prompt, 512).await?;
    // strip markdown fences
    let json = raw.trim()
        .trim_start_matches("```json").trim_start_matches("```")
        .trim_end_matches("```").trim().to_string();
    Ok(json)
}

/// Generate edge cases and negative test cases for a spec.
pub async fn generate_test_cases(spec_yaml: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let prompt = format!(
        "You are a senior QA engineer writing a test plan for a software feature.\n\
        Spec YAML:\n```yaml\n{spec_yaml}\n```\n\n\
        Write Markdown test cases for:\n\
        ## Edge Cases\n\
        (boundary values, concurrent requests, empty inputs, large payloads)\n\n\
        ## Negative Cases\n\
        (invalid inputs, auth failures, rate limiting, missing dependencies)\n\n\
        Format each case as a checkbox: `- [ ] description`\n\
        Be specific and testable. Max 5 edge cases, 5 negative cases."
    );
    call_api(&api_key, &prompt, 512).await
}

/// Generate implementation notes for a specific service task.
pub async fn suggest_task_notes(spec_yaml: &str, service: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let prompt = format!(
        "You are a senior engineer writing implementation notes for a service.\n\
        Spec YAML:\n```yaml\n{spec_yaml}\n```\n\n\
        Service to implement: {service}\n\n\
        Write concise implementation notes covering:\n\
        - Key design decisions and approach\n\
        - Potential gotchas or tricky parts\n\
        - Suggested implementation order\n\n\
        Format as a Markdown bullet list. Be specific, max 150 words."
    );
    call_api(&api_key, &prompt, 384).await
}

/// Review a spec for completeness and flag issues.
pub async fn review_spec(spec_yaml: &str) -> anyhow::Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let prompt = format!(
        "You are reviewing a software spec for quality and completeness.\n\
        Spec YAML:\n```yaml\n{spec_yaml}\n```\n\n\
        Review and flag issues in these categories:\n\
        - Missing or vague acceptance criteria\n\
        - Missing contracts (if services are defined)\n\
        - Circular or missing depends_on references\n\
        - Ambiguous service responsibilities\n\
        - Missing tags (e.g., pii, payment, auth)\n\n\
        Format as:\n\
        **[ISSUE]** description — suggestion\n\n\
        If no issues, write: **✓ Spec looks complete.**\n\
        Be brief, max 200 words."
    );
    call_api(&api_key, &prompt, 512).await
}

async fn call_api(api_key: &str, prompt: &str, max_tokens: u32) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(ANTHROPIC_API)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&serde_json::json!({
            "model": MODEL,
            "max_tokens": max_tokens,
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    anyhow::ensure!(resp.status().is_success(), "Anthropic API error ({})", resp.status());

    let body: serde_json::Value = resp.json().await?;
    let text = body["content"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Unexpected API response shape"))?;
    Ok(text.to_string())
}

/// Builds a compact snapshot of the codebase for LLM analysis.
/// Reads a sample of source files to stay within token limits.
fn build_snapshot(root: &Path, stacks: &[Stack]) -> String {
    let extensions: Vec<&str> = stacks.iter().flat_map(|s| match s {
        Stack::Rust => vec!["rs"],
        Stack::Go => vec!["go"],
        Stack::Node => vec!["ts", "js"],
        Stack::Python => vec!["py"],
        Stack::Java => vec!["java"],
        Stack::Kotlin => vec!["kt"],
        Stack::Grpc => vec!["proto"],
        Stack::Docker => vec!["Dockerfile"],
    }).collect();

    let mut files = Vec::new();
    collect_source_files(root, &extensions, 5, 200, &mut files);

    if files.is_empty() {
        return "(no source files found)".to_string();
    }

    files.into_iter()
        .map(|(path, content)| format!("// {path}\n{content}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Collect up to `max_files` source files, truncating each to `max_lines`.
fn collect_source_files(
    dir: &Path,
    extensions: &[&str],
    max_files: usize,
    max_lines: usize,
    out: &mut Vec<(String, String)>,
) {
    if out.len() >= max_files {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };

    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        if out.len() >= max_files {
            break;
        }
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.starts_with('.') || matches!(name, "target" | "node_modules" | "vendor" | ".git") {
            continue;
        }

        if path.is_dir() {
            collect_source_files(&path, extensions, max_files, max_lines, out);
        } else if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or(name);
            if extensions.contains(&ext) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let truncated: String = content
                        .lines()
                        .take(max_lines)
                        .collect::<Vec<_>>()
                        .join("\n");
                    let rel = path.strip_prefix(dir.parent().unwrap_or(dir))
                        .unwrap_or(&path)
                        .display()
                        .to_string();
                    out.push((rel, truncated));
                }
            }
        }
    }
}
