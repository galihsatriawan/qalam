# Qalam — Agent & MCP Context

This file is for AI agents (Claude Code, Cursor, Copilot, etc.) consuming Qalam as an MCP server or working inside this repo.

## Project Purpose

Qalam is a **spec-driven AI workflow tool**. When you're working on a feature in any codebase using Qalam:

1. Check `.qalam/specs/` for the authoritative spec
2. Check `.qalam/tasks/{spec-id}/` for your service's task file
3. Follow acceptance criteria and contract definitions in the spec
4. After implementation, verify against `.qalam/testplans/`

## How to Read Qalam Artifacts

### RFC (`.qalam/rfcs/RFC-XXX-*.md`)
- Why we're building this, the problem, options considered, decision made
- Read this for context on *why*, not *what*

### Spec (`.qalam/specs/SPEC-XXX-*.yaml`)
- What exactly to build: services involved, acceptance criteria, API contracts
- This is the source of truth for implementation

### Tasks (`.qalam/tasks/SPEC-XXX/{service}.md`)
- Per-service implementation checklist
- Linked to spec and RFC

### Testplan (`.qalam/testplans/SPEC-XXX-testplan.md`)
- Happy path, edge cases, negative cases, contract tests
- Use this to verify your implementation is complete

## For AI Agents Working in This Repo (qalam itself)

### Code conventions
- No unnecessary comments — code should be self-documenting
- Rust idioms: `?` for error propagation, `anyhow::Result` at boundaries
- Async where needed (tokio), sync where sufficient
- Keep commands thin — business logic in helpers within the command module
- New external HTTP deps: use `reqwest` (already in Cargo.toml)

### Adding a new command
1. Add variant to `src/cli.rs` `Command` enum
2. Create `src/commands/{name}.rs`
3. Export in `src/commands/mod.rs`
4. Handle in `cli.rs` match arm

### Module map
- `src/scanner.rs` — tech stack detection, `detect(root) -> Vec<Stack>`
- `src/llm.rs` — Anthropic API call for AI-assisted init (`analyze_project`)
- `src/commands/init.rs` — qalam init [--ai]
- `src/commands/status.rs` — project dashboard with RFC status badges
- `src/commands/doctor.rs` — project health checks + `--fix` auto-remediation
- `src/commands/completions.rs` — shell completions via `clap_complete`
- `src/commands/serve.rs` — MCP server (rmcp `#[tool_router]` pattern)
- `src/commands/skill.rs` — install/list/remove/search/publish (publish = fork→branch→PR)
- `src/commands/hook.rs` — read/write `.claude/settings.json` for Claude Code hooks
- `src/commands/context.rs` — `<qalam-context>` block with `--role`, `--service`, `--watch`

### Key data model
`Spec` struct (`src/commands/spec.rs`):
```rust
pub struct Spec {
    pub id: String,        // SPEC-001
    pub feature: String,   // human-readable name
    pub rfc: String,       // RFC-001
    pub depends_on: Vec<String>,  // e.g. ["SPEC-000"]
    pub services: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub contracts: Vec<Contract>,
}
```
`depends_on` is serialized to YAML automatically. Doctor validates dependency chain.

### MCP server tools (qalam serve)
Implemented in `src/commands/serve.rs` using rmcp `#[tool_router]`:
- `list_rfcs`, `get_rfc` — RFC discovery and content
- `list_specs`, `get_spec` — spec discovery and content
- `get_task(spec_id, service)` — per-service task file
- `get_testplan(spec_id)` — test plan file
- `get_context(spec_id, service)` — full context bundle (spec + task + skills)
- `list_skills` — installed skills

### Hook injection flow
`qalam hook install` → writes UserPromptSubmit hook to `.claude/settings.json`
→ hook calls `qalam context`
→ `qalam context` outputs `<qalam-context>...</qalam-context>` with active specs + skill context
→ Claude Code prepends this to every prompt

### File naming conventions
- RFCs: `RFC-{id}-{slug}.md` (e.g., `RFC-001-gopay-payment.md`)
- Specs: `SPEC-{id}-{slug}.yaml`
- Tasks: `.qalam/tasks/{spec-id}/{service}.md`
- Testplans: `.qalam/testplans/{spec-id}-testplan.md`
- Skills: `.qalam/skills/{name}/skill.yaml` + `context.md`
