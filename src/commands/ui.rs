use std::env;
use crate::config::QALAM_DIR;
use crate::commands::spec::Spec;
use crate::commands::graph::load_all_specs;

pub async fn run_ui(port: u16) -> anyhow::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;
    println!("✓ Qalam UI → http://localhost:{port}");
    println!("  Press Ctrl+C to stop");

    loop {
        let (mut stream, _) = listener.accept().await?;
        let root = env::current_dir().unwrap_or_default();
        let qalam_dir = root.join(QALAM_DIR);

        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf).await;
            let html = render_dashboard(&qalam_dir);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(), html
            );
            let _ = stream.write_all(response.as_bytes()).await;
        });
    }
}

fn render_dashboard(qalam_dir: &std::path::Path) -> String {
    let specs = load_all_specs(qalam_dir).unwrap_or_default();
    let stats  = render_stats(&specs, qalam_dir);
    let rfcs   = render_rfcs(qalam_dir);
    let specs_html = render_specs(&specs);

    format!(r###"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Qalam Dashboard</title>
<style>
*{{box-sizing:border-box;margin:0;padding:0}}
body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#0f1117;color:#e2e8f0;min-height:100vh}}
header{{background:#1a1d2e;border-bottom:1px solid #2d3748;padding:16px 24px;display:flex;align-items:center;gap:12px}}
header h1{{font-size:20px;font-weight:700;letter-spacing:-0.01em}}
header .sub{{font-size:13px;color:#94a3b8}}
main{{padding:24px;max-width:1100px;margin:0 auto}}
.stats{{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:14px;margin-bottom:28px}}
.sc{{background:#1a1d2e;border:1px solid #2d3748;border-radius:8px;padding:16px}}
.sc .v{{font-size:30px;font-weight:700;color:#60a5fa}}
.sc .l{{font-size:12px;color:#94a3b8;margin-top:4px}}
.sec{{background:#1a1d2e;border:1px solid #2d3748;border-radius:8px;margin-bottom:20px;overflow:hidden}}
.sh{{padding:12px 18px;border-bottom:1px solid #2d3748;font-size:12px;font-weight:600;color:#94a3b8;text-transform:uppercase;letter-spacing:0.06em}}
table{{width:100%;border-collapse:collapse}}
th{{padding:9px 18px;text-align:left;font-size:11px;font-weight:600;color:#64748b;text-transform:uppercase;letter-spacing:0.05em;border-bottom:1px solid #2d3748}}
td{{padding:11px 18px;border-bottom:1px solid #1e2330;font-size:13px;vertical-align:top}}
tr:last-child td{{border-bottom:none}}
tr:hover td{{background:#1e2330}}
.bd{{background:#1e3a5f;color:#60a5fa;padding:2px 8px;border-radius:12px;font-size:11px;white-space:nowrap}}
.bs{{background:#1a3a2a;color:#34d399;padding:2px 8px;border-radius:12px;font-size:11px;white-space:nowrap}}
.br{{background:#3a1a1a;color:#f87171;padding:2px 8px;border-radius:12px;font-size:11px;white-space:nowrap}}
.ba{{background:#1a3a2a;color:#34d399;padding:2px 8px;border-radius:12px;font-size:11px;white-space:nowrap}}
.tags{{display:flex;flex-wrap:wrap;gap:4px}}
.sv{{background:#2d3748;color:#94a3b8;padding:2px 6px;border-radius:4px;font-size:11px;font-family:monospace}}
.tg{{background:#2d1e3a;color:#c084fc;padding:2px 6px;border-radius:4px;font-size:11px}}
.muted{{color:#475569}}
footer{{text-align:center;color:#475569;font-size:12px;padding:20px}}
a{{color:#60a5fa;text-decoration:none}}
</style>
</head>
<body>
<header>
  <h1>قلم Qalam</h1>
  <span class="sub">Spec-driven AI development workflow</span>
</header>
<main>
{stats}
{rfcs}
{specs_html}
</main>
<footer>
  <a href="https://github.com/galihsatriawan/qalam">github.com/galihsatriawan/qalam</a> · Refresh page to reload data
</footer>
</body>
</html>"###)
}

fn render_stats(specs: &[Spec], qalam_dir: &std::path::Path) -> String {
    let total_rfcs  = count_dir(&qalam_dir.join("rfcs"));
    let total_specs = specs.len();
    let active  = specs.iter().filter(|s| !matches!(s.status.as_str(), "shipped"|"closed"|"done")).count();
    let shipped = total_specs - active;
    let skills  = count_dirs(&qalam_dir.join("skills"));

    format!(r#"<div class="stats">
  <div class="sc"><div class="v">{total_rfcs}</div><div class="l">RFCs</div></div>
  <div class="sc"><div class="v">{total_specs}</div><div class="l">Specs</div></div>
  <div class="sc"><div class="v">{active}</div><div class="l">Active</div></div>
  <div class="sc"><div class="v">{shipped}</div><div class="l">Shipped</div></div>
  <div class="sc"><div class="v">{skills}</div><div class="l">Skills</div></div>
</div>"#)
}

fn render_rfcs(qalam_dir: &std::path::Path) -> String {
    let rfcs_dir = qalam_dir.join("rfcs");
    let Ok(entries) = std::fs::read_dir(&rfcs_dir) else {
        return empty_section("RFCs");
    };
    let mut items: Vec<_> = entries.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).collect();
    items.sort_by_key(|e| e.file_name());

    if items.is_empty() { return empty_section("RFCs"); }

    let rows: String = items.iter().map(|e| {
        let name    = e.file_name().to_string_lossy().to_string();
        let content = std::fs::read_to_string(e.path()).unwrap_or_default();
        let status  = extract_rfc_status(&content).unwrap_or_else(|| "Draft".to_string());
        let cls = if status.to_lowercase().contains("accept") || status.to_lowercase().contains("published") { "ba" }
                  else if status.to_lowercase().contains("reject") { "br" }
                  else { "bd" };
        format!(r#"<tr><td style="font-family:monospace;font-size:12px">{name}</td><td><span class="{cls}">{status}</span></td></tr>"#)
    }).collect();

    format!(r#"<div class="sec"><div class="sh">RFCs</div>
<table><thead><tr><th>File</th><th>Status</th></tr></thead><tbody>{rows}</tbody></table></div>"#)
}

fn render_specs(specs: &[Spec]) -> String {
    if specs.is_empty() { return empty_section("Specs"); }

    let rows: String = specs.iter().map(|s| {
        let cls = if matches!(s.status.as_str(), "shipped"|"closed"|"done") { "bs" } else { "bd" };
        let svcs: String = s.services.iter().map(|sv| format!(r#"<span class="sv">{sv}</span>"#)).collect();
        let tags: String = s.tags.iter().map(|t| format!(r#"<span class="tg">{t}</span>"#)).collect();
        let svc_cell = if s.services.is_empty() {
            r#"<span class="muted">—</span>"#.to_string()
        } else {
            format!(r#"<div class="tags">{svcs}</div>"#)
        };
        let tag_cell = if s.tags.is_empty() {
            r#"<span class="muted">—</span>"#.to_string()
        } else {
            format!(r#"<div class="tags">{tags}</div>"#)
        };
        format!(r#"<tr>
<td><strong style="font-size:12px;font-family:monospace">{}</strong><br><span class="muted">{}</span></td>
<td style="font-size:12px;font-family:monospace">{}</td>
<td>{svc_cell}</td>
<td>{tag_cell}</td>
<td><span class="{cls}">{}</span></td>
</tr>"#, s.id, s.feature, s.rfc, s.status)
    }).collect();

    format!(r#"<div class="sec"><div class="sh">Specs</div>
<table><thead><tr><th>ID / Feature</th><th>RFC</th><th>Services</th><th>Tags</th><th>Status</th></tr></thead>
<tbody>{rows}</tbody></table></div>"#)
}

fn extract_rfc_status(content: &str) -> Option<String> {
    let mut in_status = false;
    for line in content.lines() {
        if line == "## Status" { in_status = true; continue; }
        if in_status {
            let t = line.trim();
            if !t.is_empty() { return Some(t.to_string()); }
            if t.starts_with("## ") { break; }
        }
    }
    None
}

fn empty_section(title: &str) -> String {
    format!(r#"<div class="sec"><div class="sh">{title}</div><div style="padding:20px;color:#64748b;font-size:13px">No {title} found</div></div>"#)
}

fn count_dir(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_file()).count())
        .unwrap_or(0)
}

fn count_dirs(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
        .unwrap_or(0)
}
