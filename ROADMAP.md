# Qalam Roadmap

This document captures the full product vision for Qalam. Items are grouped by milestone. Each milestone builds on the previous one and can be shipped independently.

---

## Already Shipped

<details>
<summary>V1–V6 (click to expand)</summary>

**V1 — Core CLI**
- `qalam init` — scaffold `.qalam/` directory
- `qalam rfc generate` — RFC from description
- `qalam spec generate --from RFC-001` — spec from RFC
- `qalam breakdown --from SPEC-001` — per-service task files
- `qalam testplan --from SPEC-001` — test plan

**V2 — MCP + Skills**
- `qalam serve` — MCP server via rmcp (stdio), 7 tools
- `qalam skill install/list/remove/search` — skill system with GitHub registry

**V3 — Hook + Codebase Scan**
- `qalam init` auto-detect tech stack → scaffold matching skills
- `qalam hook install [--global]` — UserPromptSubmit hook
- `qalam context [--role pm|engineer|qa]` — context output for hook
- `qalam rfc publish` — mark RFC as Accepted

**V4 — Status + AI Init**
- `qalam status` — project dashboard with status badges
- `qalam context --watch` — watch mode (2s polling)
- `qalam skill publish` — GitHub issue submission
- `qalam init --ai` — Anthropic API pattern inference

**V5 — Doctor + Service Context**
- `qalam doctor` — validate RFC→spec→tasks→testplan chain
- `qalam context --service <name>` — service-scoped context
- `depends_on` field in Spec YAML
- `qalam skill publish` — full PR workflow (fork + branch + commit + PR)

**V6 — Polish + DX**
- `qalam doctor --fix` — auto-remediation
- `qalam rfc list --status` — filter by RFC status
- `qalam completions bash|zsh|fish` — shell completions
- `README.md` — full documentation

</details>

---

## M1 — Deeper SDLC Integration ✅

*Goal: make the spec→code→verify loop tighter and trackable.*

- [x] **`qalam spec list --service <name>`** — find all specs touching a service; useful when onboarding onto a service or doing impact analysis
- [x] **`qalam rfc reject <id> --reason "..."`** — first-class rejection with reason stored in RFC file; shown in `status` and `doctor`
- [x] **`qalam spec close <id>`** — mark spec as Done/Shipped; removes from active context output, keeps in history
- [ ] **`qalam spec update --from RFC-002`** — re-derive spec from a superseding RFC (generates a diff preview)
- [ ] **`qalam task check <spec-id> <service>`** — interactive checklist to mark acceptance criteria done (writes checkmarks to task file)
- [ ] **`qalam testplan run --from SPEC-001`** — stub that prints test plan and opens it; V2 could integrate with test runner output
- [x] **`qalam context --since <date|git-ref>`** — show only specs/RFCs changed since a date or git ref; useful for daily standup context

---

## M2 — Dependency & Impact Graph ✅

*Goal: understand how specs relate to each other and to services.*

- [ ] **`depends_on` resolution in `doctor`** — warn if a spec is being broken down while its dependency is still Draft
- [x] **`qalam graph`** — render RFC→spec→service dependency graph as ASCII or Mermaid (`--format mermaid`)
- [x] **`qalam impact --service <name>`** — list all specs (open and closed) that touch a given service; helps with blast radius analysis
- [x] **`qalam impact --rfc RFC-001`** — list all specs, tasks, and testplans derived from an RFC
- [ ] **`qalam graph --open`** — open graph in browser as SVG (generated locally, no network)

---

## M3 — Editor & Git Integration ✅

*Goal: make qalam feel like a first-class citizen in the developer's daily workflow.*

- [x] **`qalam edit <rfc-id|spec-id>`** — open artifact in `$EDITOR`; falls back to `$VISUAL`, then `vi`
- [x] **`qalam diff <spec-id>`** — show git diff for a spec file
- [x] **`qalam log <spec-id>`** — show git log for a spec file (`git log .qalam/specs/<file>`)
- [x] **`qalam git-hook install`** — install a git `pre-commit` hook that runs `qalam doctor` and blocks commit if critical checks fail
- [ ] **GitHub Actions workflow** — `qalam doctor` as a CI check; outputs annotations on PRs when spec/task coverage is missing
  - Provide a reusable workflow: `uses: galihsatriawan/qalam/.github/workflows/doctor.yml@main`
- [x] **`qalam commit`** — stage + commit all `.qalam/` changes with a standardized message (`chore(qalam): update RFC-001 status`)

---

## M4 — Multi-Repo & Team Sync ✅

*Goal: work across a microservices monorepo or many repos without losing context.*

- [x] **`qalam.yaml` sources** — reference other repos' `.qalam/` dirs; `qalam context` aggregates context from all sources
  ```yaml
  sources:
    git_history: true
    repos:
      - path: ../auth-service
      - path: ../payment-service
  ```
- [x] **`qalam sync`** — pull latest `.qalam/` from referenced repos (git pull or submodule update)
- [x] **`qalam context --repo <path>`** — show context from a specific referenced repo
- [ ] **Monorepo support** — `qalam init --monorepo`: single `.qalam/` at root, per-package skill overrides in `packages/<name>/.qalam/skills/`
- [x] **`qalam export --format json`** — export full project state as JSON for programmatic consumption

---

## M5 — Integrations ✅

*Goal: connect qalam to the tools engineers already use.*

- [x] **OpenAPI from contracts** — `qalam openapi --from SPEC-001` generates a partial OpenAPI 3.0 YAML from `contracts:` fields in the spec
- [x] **Postman collection** — `qalam postman --from SPEC-001` generates a Postman collection from contracts
- [ ] **Jira/Linear task creation** — `qalam push --to jira --from SPEC-001` creates subtasks in Jira/Linear from `.qalam/tasks/`; needs `JIRA_TOKEN` / `LINEAR_TOKEN`
- [ ] **GitHub Issues sync** — `qalam push --to github --from SPEC-001` creates issues per service task
- [ ] **Notion/Confluence export** — `qalam export --to notion` pushes RFCs and specs to a Notion database; needs `NOTION_TOKEN`
- [ ] **Slack notification** — `qalam notify --channel #eng-platform` when an RFC is published or spec is created; needs `SLACK_TOKEN`

---

## M6 — codebase-memory-mcp Integration ✅

*Goal: combine qalam's SDLC workflow layer ("what to build, why") with codebase-memory-mcp's code intelligence layer ("how the code is structured"). The two are complementary — codebase-memory-mcp already gives ~120x token reduction via graph queries; Qalam's job is to wire the spec context on top of it, not replace it.*

- [x] **`qalam init` auto-detect** — if `codebase-memory-mcp` is in PATH, automatically run `index_repository` for the current project during init; print a note pointing to the SKILL.md setup guide if not installed
- [x] **`.mcp.json` coordination** — `qalam hook install` merges its `qalam serve` entry into the project's `.mcp.json` alongside `codebase-memory-mcp`; never clobbers existing entries
- [ ] **MCP tool: `get_full_context(spec_id, service)`** — proxy that returns qalam spec/task context AND calls `codebase-memory-mcp search_graph` for services mentioned in the spec; single tool call gives AI both layers
- [ ] **MCP tool: `impact_analysis(spec_id)`** — calls codebase-memory-mcp's cross-service linking tools to surface which actual code symbols are affected by a spec's services and contracts
- [ ] **Skill auto-enhancement** — during `qalam init --ai`, if codebase-memory-mcp is indexed, use `search_graph` results (real idioms from codebase) instead of sampling raw source files for the LLM prompt; more accurate, fewer tokens
- [x] **`qalam doctor` check** — warn if codebase-memory-mcp is not installed/indexed; suggest fix via `curl -fsSL ... | bash` one-liner

---

## M7 — LLM-Assisted Spec Generation ✅

*Goal: reduce the time from idea to actionable spec.*

- [x] **`qalam rfc generate --from <prd-file> --ai`** — use LLM to draft RFC sections (Problem, Options, Decision) from a PRD file; engineer reviews and edits
- [x] **`qalam spec generate --from RFC-001 --ai`** — infer services, acceptance criteria, and contracts from RFC content using LLM
- [x] **`qalam testplan --from SPEC-001 --ai`** — generate edge cases and negative test cases using LLM, not just templated Happy Path
- [x] **`qalam breakdown --from SPEC-001 --ai`** — suggest implementation notes per service task using LLM
- [x] **`qalam spec review SPEC-001`** — LLM reviews spec for completeness: missing contracts, vague criteria, circular dependencies

---

## M8 — Skills Ecosystem ✅

*Goal: a rich, community-driven library of AI context packages.*

- [ ] **Official skill registry** at `galihsatriawan/qalam-skills` with categories: `lang/`, `framework/`, `pattern/`, `domain/`
- [ ] **Skill versioning** — `skill.yaml` has `version:` field; `qalam skill install @golang@1.2.0`
- [ ] **Skill inheritance** — `extends: @golang` in `skill.yaml`; child skill inherits parent's `context.md` and adds on top
- [ ] **Skill composition** — `qalam skill bundle create my-stack --from golang,grpc,docker` creates a composite skill
- [x] **`qalam skill update`** — update all installed registry skills to latest versions
- [x] **`qalam skill diff <name>`** — show what changed between installed and latest registry version
- [ ] **Private registry** — `qalam.yaml` supports `registries:` with custom GitHub repos for internal company skills

---

## M9 — Compliance & Governance ✅

*Goal: enforce engineering standards at scale.*

- [ ] **Required fields** — `qalam.yaml` can declare required spec fields (`contracts`, `acceptance_criteria`); `doctor` and CI fail if missing
- [ ] **Spec approval workflow** — `depends_on_approval: [alice, bob]` in spec YAML; `qalam approve SPEC-001` records approval with timestamp
- [x] **PII/security tagging** — `tags: [pii, payment]` on specs; `qalam audit --tag pii` lists all specs touching PII data
- [ ] **RFC decision log** — `qalam rfc decisions` shows a timeline of all accepted/rejected RFCs with reasons; good for engineering knowledge base
- [x] **Stale spec detection** — warn in `doctor` if a spec has been Draft for >30 days (configurable in `qalam.yaml`)
- [x] **`qalam audit --service <name>`** — show all accepted RFCs + closed specs affecting a service; useful for security audits

---

## M10 — Slash Commands from Skills ✅ *(inspired by harness-ai)*

*Goal: skills tidak hanya inject context, tapi juga bisa dipanggil sebagai slash command langsung dari dalam AI tool.*

- [x] **`qalam skill expose <name>`** — generate `.claude/commands/<name>.md` dari skill's `context.md`; membuat skill menjadi callable slash command (e.g., `/golang-review`, `/grpc-patterns`)
- [x] **`qalam skill expose --all`** — expose semua installed skills sebagai slash commands sekaligus
- [ ] **Skill action templates** — `skill.yaml` bisa punya `commands:` section dengan parameterized slash command templates
  ```yaml
  commands:
    - name: review
      description: "Review this file against Go patterns"
      template: "Review {{file}} against the patterns in this skill: ..."
  ```
- [x] **`qalam hook install --cursor`** — install context injection ke `.cursorrules` (Cursor)
- [x] **`qalam hook install --copilot`** — inject ke `.github/copilot-instructions.md`
- [x] **`qalam hook install --all`** — install ke semua detected AI tools sekaligus

---

## M11 — ~~Context Token Optimization~~ *(deferred — solved by codebase-memory-mcp)*

> **Decision:** Qalam injects SDLC context (specs, RFCs, tasks) which is inherently small and structured. Code-level token bloat is solved upstream by codebase-memory-mcp's graph queries (~120x reduction). Building our own compression layer risks hallucination from truncated specs — not worth the complexity. Revisit only if real-world usage shows qalam context itself becoming a bottleneck.

---

## M12 — MCP Write Tools + Spec Metrics *(inspired by harness-ai)*

*Goal: MCP server saat ini read-only. Write tools memungkinkan AI tool membuat RFC/spec langsung tanpa keluar dari chat.*

- [ ] **MCP write tools:**
  - `create_rfc(description)` — draft RFC dan return path
  - `publish_rfc(id)` — set status Accepted
  - `reject_rfc(id, reason)` — set status Rejected
  - `create_spec(from_rfc_id)` — generate spec dari RFC
  - `check_task(spec_id, service, criterion_index)` — centang satu acceptance criterion di task file
- [x] **`qalam metrics`** — SDLC health analytics:
  - RFC acceptance rate (accepted / total)
  - Spec coverage: % specs with tasks + testplan
  - Active specs per service (hotspot detection)
- [x] **`qalam metrics --service <name>`** — per-service breakdown
- [x] **`qalam metrics --export csv`** — export for dashboards (Grafana, Notion)

---

## M13 — Web UI ✅ (Optional, Local)

*Goal: a read-only local dashboard for non-CLI users (PMs, QA leads).*

- [x] **`qalam ui`** — start a local HTTP server (port 7734) serving a single-page app
- [x] **Dashboard view** — RFC and spec list with status badges, dependency graph visualization
- [ ] **RFC/spec viewer** — rendered Markdown/YAML with syntax highlighting
- [ ] **Task tracking** — checklist view of acceptance criteria per service
- [x] **No external deps** — assets bundled into the binary; zero install beyond qalam itself

---
## Non-Goals

These are explicitly out of scope to keep qalam focused:

- Cloud sync / SaaS hosting — Qalam is privacy-first and local
- Code generation beyond scaffolding stubs
- Replacing dedicated project management tools (Jira, Linear)
- IDE plugin — hook + MCP covers the IDE AI tool integration
- Authentication / multi-user accounts

---

## Versioning

Qalam follows semantic versioning. Breaking changes to `.qalam/` file formats will be documented in a migration guide. The YAML schemas for RFC, spec, and skill files are stable from V1.
