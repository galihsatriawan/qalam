# Qalam — AI Development Workflow Tool

## What is Qalam?

Qalam is a spec-driven AI development workflow CLI for AI-native engineers. It solves the problem of **context fragmentation** across multiple microservices when using AI coding tools.

The name "Qalam" (قلم) means "pen" in Arabic — the instrument of writing, knowledge, and deliberate creation.

## Core Philosophy

- **Spec as source of truth** — RFC → Spec → Tasks → Code → Testplan
- **Privacy-first** — all artifacts are local plaintext files in `.qalam/`
- **Git-friendly** — everything is Markdown/YAML, reviewable, diffable
- **AI-agnostic** — works with any AI tool via MCP (V2)

## SDLC Flow

```
PRD → RFC (.qalam/rfcs/)
    → Spec (.qalam/specs/)
    → Tasks (.qalam/tasks/{spec-id}/{service}.md)
    → Code (AI-assisted via MCP in V2)
    → Testplan (.qalam/testplans/)
```

## Tech Stack

- **Language**: Rust (cross-platform single binary)
- **CLI**: clap (derive macros)
- **Async**: tokio
- **Serialization**: serde + serde_yaml + serde_json
- **MCP**: rmcp v1.7.0 (planned V2)

## Project Structure

```
src/
  main.rs          — entry point
  cli.rs           — CLI definition (clap)
  config.rs        — QALAM_DIR constant, Config struct
  scanner.rs       — tech stack detection (detect() → Vec<Stack>)
  llm.rs           — Anthropic API client (analyze_project for --ai init)
  commands/
    init.rs        — qalam init [--ai]
    status.rs      — qalam status (project dashboard with RFC badges)
    doctor.rs      — qalam doctor [--fix] (validate + auto-fix)
    completions.rs — qalam completions bash|zsh|fish
    rfc.rs         — qalam rfc generate/list/publish
    spec.rs        — qalam spec generate/list  (Spec struct has depends_on field)
    breakdown.rs   — qalam breakdown --from SPEC-001
    testplan.rs    — qalam testplan --from SPEC-001
    serve.rs       — qalam serve (MCP server via rmcp)
    skill.rs       — qalam skill install/list/remove/search/publish (full PR workflow)
    hook.rs        — qalam hook install/uninstall/status
    context.rs     — qalam context [--role pm|engineer|qa] [--service <name>] [--watch]
```

## V1 Scope

- [x] `qalam init` — scaffold `.qalam/` directory
- [x] `qalam rfc generate` — generate RFC from description
- [x] `qalam spec generate --from RFC-001` — generate spec from RFC
- [x] `qalam breakdown --from SPEC-001` — generate per-service task files
- [x] `qalam testplan --from SPEC-001` — generate test plan

## V2 Scope

- [x] `qalam serve` — MCP server via rmcp (stdio transport, for Claude Code / any MCP client)
  - Tools: `list_rfcs`, `get_rfc`, `list_specs`, `get_spec`, `get_task`, `get_testplan`, `get_context`, `list_skills`
- [x] `qalam skill install/list/remove/search` — skill system with GitHub registry support
  - Local: `qalam skill install ./my-skill`
  - Registry: `qalam skill install @golang` (fetches from `galihsatriawan/qalam-skills`)
  - Scaffold: `qalam skill install golang` (generates template)

## V3 Scope (current)

- [x] `qalam init` auto-detect tech stack → scaffold matching skills (Rust/Go/Node/Python/Java/Kotlin/gRPC/Docker)
- [x] `qalam hook install` — add UserPromptSubmit hook to `.claude/settings.json`
- [x] `qalam hook install --global` — global hook for all projects
- [x] `qalam hook uninstall` / `qalam hook status`
- [x] `qalam context` — prints active specs + skill context (called by hook)
- [x] `qalam rfc publish <id>` — marks RFC status as Accepted in the file

## V4 Scope (current)

- [x] `qalam status` — project dashboard (RFCs with status badges, specs, tasks, skills)
- [x] `qalam context --role pm|engineer|qa` — role-specific context output
- [x] `qalam context --watch` — watch `.qalam/` for changes and re-print (2s polling)
- [x] `qalam skill publish <name>` — open GitHub issue in registry (needs `GITHUB_TOKEN`)
- [x] `qalam init --ai` — LLM-assisted init via Anthropic API (needs `ANTHROPIC_API_KEY`)
  - Samples source files, calls Claude Haiku, infers project-specific patterns
  - Appends AI-inferred section to each skill's `context.md`

## V5 Scope

- [x] `qalam doctor` — validate project structure (RFC→spec→tasks→testplan chain, skills content, hook install)
- [x] `qalam context --service <name>` — filter context to tasks + specs for a specific service
- [x] `qalam context --role ... --service ...` — composable flags
- [x] RFC dependency tracking — `depends_on:` field in `Spec` struct (YAML), shown in status
- [x] `qalam skill publish <name>` — full PR workflow: fork registry → branch → commit files → open PR (needs `GITHUB_TOKEN`)

## V6 Scope (current)

- [x] `qalam doctor --fix` — auto-fix: create missing dirs, run `spec generate`, `breakdown`, `testplan`, `hook install`
- [x] `qalam rfc list --status draft|accepted|rejected|superseded` — filter RFCs by status
- [x] `qalam completions bash|zsh|fish` — shell tab completions via `clap_complete`
- [x] Doctor validates `depends_on` references (warns if SPEC-XXX in depends_on doesn't exist)
- [x] `README.md` — full project documentation

## V7 Scope (M1 — Deeper SDLC Integration)

- [x] `qalam spec list --service <name>` — filter specs by service, shows status badges
- [x] `qalam spec close <id>` — mark spec as shipped; auto-excluded from active context
- [x] `qalam rfc reject <id> --reason "..."` — mark RFC as rejected with reason stored in file
- [x] `qalam context --since <date|git-ref>` — filter context to artifacts modified since date/ref
- [x] `Spec.status` field added to YAML schema (`draft` default, `shipped` when closed)
- [x] `specs_section` in context auto-excludes shipped specs

## V8–V13 Scope (M1–M10, M12–M13 — all shipped)

- [x] `qalam graph [--format ascii|mermaid]` — RFC→spec→service dependency graph
- [x] `qalam impact [--service <name>] [--rfc RFC-001]` — blast radius analysis
- [x] `qalam edit <id>` / `qalam diff <id>` / `qalam log <id>` — editor + git integration
- [x] `qalam git-hook install|uninstall|status` — pre-commit hook for `qalam doctor`
- [x] `qalam commit` — smart commit of `.qalam/` changes
- [x] `qalam sync` — pull referenced repos from `qalam.yaml`
- [x] `qalam export [--format json|yaml]` — full project state export
- [x] `qalam openapi --from SPEC-001` — partial OpenAPI 3.0 YAML from contracts
- [x] `qalam postman --from SPEC-001` — Postman collection from contracts
- [x] `qalam context --repo <path>` — context from a different repo's `.qalam/`
- [x] `qalam rfc generate --ai` / `qalam spec generate --from RFC-001 --ai` — LLM-assisted generation
- [x] `qalam spec review <id>` — AI spec review for completeness
- [x] `qalam breakdown --from SPEC-001 --ai` / `qalam testplan --from SPEC-001 --ai` — AI-enhanced
- [x] `qalam skill expose [<name>|--all]` — write skills to `.claude/commands/` as slash commands
- [x] `qalam skill update [<name>]` — re-fetch skill from registry
- [x] `qalam skill diff <name>` — diff installed vs registry version
- [x] `qalam hook install --cursor|--copilot|--mcp|--all` — multi-AI-tool hook install
- [x] `qalam audit [--service <name>] [--tag <tag>]` — compliance audit view
- [x] `qalam metrics [--service <name>] [--export csv|json]` — SDLC health metrics
- [x] `qalam ui [--port 7734]` — local web dashboard (no extra deps)

## codebase-memory-mcp Reference

Repo: `https://github.com/DeusData/codebase-memory-mcp`

Key facts for M6 integration work:
- **~120x fewer tokens** — 5 graph queries replace ~412,000 tokens of file-by-file search
- **14 MCP tools**: `search_graph`, `trace_call_path`, `list_projects`, `index_repository`, architecture/impact analysis, dead code detection, cross-service linking, Cypher queries
- **Zero friction** — single static binary, no Docker, no API keys
- Auto-configures Claude Code, Windsurf, Zed, Aider, VS Code via install script

Install (macOS/Linux):
```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash -s -- --ui
```

Index a repo:
```bash
codebase-memory-mcp cli index_repository '{"repo_path": "/absolute/path/to/repo"}'
```

MCP config entry (`.mcp.json`):
```json
{
  "mcpServers": {
    "codebase-memory-mcp": {
      "command": "/path/to/codebase-memory-mcp",
      "args": []
    }
  }
}
```

> Qalam's role: inject SDLC context (what to build, why). codebase-memory-mcp's role: code intelligence (how the code is structured). The two are additive — Qalam should detect it and coordinate `.mcp.json` entries, never clobber.

---

## Key Design Decisions

1. **Why Rust?** Cross-platform single binary, fast startup, official MCP SDK (rmcp)
2. **Why local files?** Privacy-first, Git-native, no vendor lock-in
3. **Why not just codebase-memory-mcp?** That handles "understand the code"; Qalam handles "what to build and how" — SDLC workflow layer on top
4. **Skill system** — package manager style (`qalam install`), enables per-repo pattern customization without submodules

## Repository

`github.com/galihsatriawan/qalam`

## Development

```bash
cargo build
cargo run -- init                                          # init + auto-detect tech stack
cargo run -- init --ai                                     # init + AI pattern inference
cargo run -- status                                        # project dashboard
cargo run -- rfc generate --description "My feature"
cargo run -- rfc publish RFC-001
cargo run -- spec generate --from RFC-001
cargo run -- breakdown --from SPEC-001
cargo run -- testplan --from SPEC-001
cargo run -- skill install golang                          # scaffold skill
cargo run -- skill install @golang                        # from registry
cargo run -- skill search
cargo run -- skill publish golang                          # submit to registry via GitHub issue
cargo run -- hook install                                  # add Claude Code hook
cargo run -- hook install --global
cargo run -- context                                       # engineer view (default)
cargo run -- context --role pm                            # PM view
cargo run -- context --role qa                            # QA view
cargo run -- context --watch                              # watch mode
cargo run -- context --service payment-service            # service-specific context
cargo run -- doctor                                        # validate project structure
cargo run -- doctor --fix                                  # auto-fix issues
cargo run -- rfc list --status draft                      # filter RFCs
cargo run -- rfc reject RFC-001 --reason "not feasible"  # reject RFC with reason
cargo run -- spec list                                     # list all specs with status badges
cargo run -- spec list --service payment-service          # filter specs by service
cargo run -- spec close SPEC-001                          # mark spec as shipped
cargo run -- context --since 2026-06-01                  # context filtered to recent changes
cargo run -- context --since main                         # context filtered since git ref
cargo run -- context --repo ../other-service              # context from another repo
cargo run -- completions bash                             # shell completions
cargo run -- serve                                         # start MCP server on stdio

# M2 — Dependency graph
cargo run -- graph                                         # ASCII RFC→spec→service graph
cargo run -- graph --format mermaid                       # Mermaid diagram
cargo run -- impact --service payment-service             # blast radius for a service
cargo run -- impact --rfc RFC-001                         # all artifacts from an RFC

# M3 — Editor & git integration
cargo run -- edit SPEC-001                                # open in $EDITOR
cargo run -- diff SPEC-001                                # git diff for spec
cargo run -- log SPEC-001                                 # git log for spec
cargo run -- git-hook install                             # install pre-commit hook
cargo run -- commit                                       # smart commit of .qalam/ changes

# M4 — Multi-repo sync & export
cargo run -- sync                                         # pull referenced repos
cargo run -- export                                       # full state as JSON
cargo run -- export --format yaml                         # full state as YAML

# M5 — Integrations
cargo run -- openapi --from SPEC-001                     # OpenAPI 3.0 YAML from contracts
cargo run -- postman --from SPEC-001                     # Postman collection

# M7 — AI-assisted generation
cargo run -- rfc generate --description "feature" --ai   # AI-drafted RFC sections
cargo run -- spec generate --from RFC-001 --ai           # AI-inferred spec fields
cargo run -- spec review SPEC-001                        # AI spec review
cargo run -- breakdown --from SPEC-001 --ai              # AI implementation notes
cargo run -- testplan --from SPEC-001 --ai               # AI edge cases

# M8 — Skill management
cargo run -- skill expose golang                          # expose skill as slash command
cargo run -- skill expose --all                           # expose all skills
cargo run -- skill update golang                          # re-fetch skill from registry
cargo run -- skill diff golang                            # diff installed vs registry

# M9 — Audit & compliance
cargo run -- audit                                        # all specs with RFC status
cargo run -- audit --service payment-service             # filter by service
cargo run -- audit --tag pii                             # filter by tag
cargo run -- metrics                                     # SDLC health metrics
cargo run -- metrics --service payment-service           # per-service metrics
cargo run -- metrics --export csv                        # export metrics as CSV

# M10 — Hook multi-tool support
cargo run -- hook install --cursor                       # inject into .cursorrules
cargo run -- hook install --copilot                      # inject into copilot-instructions.md
cargo run -- hook install --mcp                          # merge into .mcp.json
cargo run -- hook install --all                          # all AI tools at once

# M13 — Web UI
cargo run -- ui                                           # dashboard at http://localhost:7734
cargo run -- ui --port 8080                              # custom port
```
