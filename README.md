# Qalam (قلم)

**Spec-driven AI development workflow for AI-native engineers.**

Qalam solves context fragmentation across microservices when using AI coding tools. It gives your AI assistant a structured, always-current understanding of *what you're building and why* — not just what the code looks like.

> قلم means "pen" in Arabic — the instrument of writing, knowledge, and deliberate creation.

---

## The Problem

When you work across multiple microservices with an AI coding tool, context gets lost:
- The AI doesn't know which feature you're building
- It doesn't know *why* certain decisions were made (the RFC)
- It doesn't know the acceptance criteria or API contracts
- Each service engineer works in isolation without shared spec context

Qalam fixes this by maintaining a lightweight spec layer (`.qalam/`) that your AI tool reads automatically.

---

## How It Works

```
PRD → RFC → Spec → Tasks → Code (AI-assisted) → Testplan
```

All artifacts live in `.qalam/` as plain Markdown/YAML — Git-friendly, reviewable, diffable. Qalam exposes these to AI tools via:

1. **Claude Code hook** — auto-injects context before every prompt
2. **MCP server** — AI tools pull context on demand via `qalam serve`

---

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/galihsatriawan/qalam
cd qalam
cargo build --release
cp target/release/qalam /usr/local/bin/
```

### Shell completions

```bash
# Bash
qalam completions bash >> ~/.bashrc

# Zsh
qalam completions zsh >> ~/.zshrc

# Fish
qalam completions fish > ~/.config/fish/completions/qalam.fish
```

---

## Quick Start

```bash
# 1. Initialize in your repo (auto-detects tech stack)
cd my-service
qalam init

# Optional: AI-powered pattern inference (needs ANTHROPIC_API_KEY)
qalam init --ai

# 2. Wire up auto-context injection for Claude Code
qalam hook install

# 3. Create an RFC
qalam rfc generate --description "Add GoPay payment integration"

# 4. Edit the RFC, then generate a spec
qalam rfc publish RFC-001
qalam spec generate --from RFC-001

# 5. Edit the spec (add services, acceptance criteria, contracts)
# Then generate tasks per service
qalam breakdown --from SPEC-001

# 6. Generate test plan
qalam testplan --from SPEC-001

# 7. Check project health
qalam doctor
qalam doctor --fix   # auto-fix what's possible
```

---

## Commands

### Project

| Command | Description |
|---------|-------------|
| `qalam init [--ai]` | Initialize `.qalam/`, auto-detect tech stack, scaffold skills |
| `qalam status` | Project dashboard — RFCs, specs, tasks, skills |
| `qalam doctor [--fix]` | Validate RFC→spec→tasks→testplan chain, check hooks |

### SDLC Flow

| Command | Description |
|---------|-------------|
| `qalam rfc generate "<description>"` | Generate RFC from description |
| `qalam rfc list [--status draft\|accepted\|rejected]` | List RFCs with status filter |
| `qalam rfc publish RFC-001` | Mark RFC as Accepted |
| `qalam spec generate --from RFC-001` | Generate spec from RFC |
| `qalam spec list` | List all specs |
| `qalam breakdown --from SPEC-001` | Generate per-service task files |
| `qalam testplan --from SPEC-001` | Generate test plan |

### Context & AI Integration

| Command | Description |
|---------|-------------|
| `qalam context` | Print context (engineer role, for hook) |
| `qalam context --role pm\|engineer\|qa` | Role-specific context |
| `qalam context --service <name>` | Context for a specific service |
| `qalam context --watch` | Watch `.qalam/` and re-print on changes |
| `qalam hook install [--global]` | Add UserPromptSubmit hook to Claude Code |
| `qalam hook uninstall [--global]` | Remove hook |
| `qalam hook status` | Show hook installation status |
| `qalam serve` | Start MCP server (stdio) |

### Skills

| Command | Description |
|---------|-------------|
| `qalam skill install <name>` | Scaffold a skill (e.g. `golang`, `grpc`) |
| `qalam skill install @<name>` | Install from registry |
| `qalam skill install ./path` | Install from local path |
| `qalam skill list` | List installed skills |
| `qalam skill search [query]` | Browse registry |
| `qalam skill remove <name>` | Remove a skill |
| `qalam skill publish <name>` | Submit skill to registry via GitHub PR |

---

## Directory Structure

After `qalam init`, your repo gets:

```
.qalam/
  qalam.yaml        # project config (packages, sources)
  rfcs/
    RFC-001-gopay-payment.md
  specs/
    SPEC-001-gopay-payment.yaml
  tasks/
    SPEC-001/
      payment-service.md
      auth-service.md
  testplans/
    SPEC-001-testplan.md
  skills/
    go/
      skill.yaml
      context.md    # patterns injected into AI context
    grpc/
      skill.yaml
      context.md
```

---

## Spec Format

```yaml
id: SPEC-001
feature: GoPay Payment Integration
rfc: RFC-001
depends_on: []          # other specs this one depends on
services:
  - payment-service
  - auth-service
  - api-gateway
acceptance_criteria:
  - Users can initiate GoPay payment from checkout
  - Payment status is reflected within 5 seconds
contracts:
  - service: payment-service
    endpoint: POST /v1/payments/gopay
  - service: api-gateway
    endpoint: GET /v1/payments/{id}/status
```

---

## Claude Code Integration

### Hook (auto-inject context)

```bash
qalam hook install
```

This adds a `UserPromptSubmit` hook to `.claude/settings.json`. Every prompt you send to Claude Code in this repo will automatically include your active specs and skill patterns.

Use `--global` to inject context in all repos with a `.qalam/` directory.

### MCP Server

Add to your `.mcp.json`:

```json
{
  "mcpServers": {
    "qalam": {
      "command": "qalam",
      "args": ["serve"]
    }
  }
}
```

Available MCP tools:

| Tool | Description |
|------|-------------|
| `list_rfcs` | List all RFCs |
| `get_rfc` | Get RFC content by ID |
| `list_specs` | List all specs |
| `get_spec` | Get spec content by ID |
| `get_task` | Get task file for a service |
| `get_testplan` | Get test plan for a spec |
| `get_context` | Full context bundle (spec + task + skills) |
| `list_skills` | List installed skills |

### Role-based context

```bash
# Engineer: sees active specs + skill patterns
qalam context --role engineer

# PM: sees RFCs + specs overview
qalam context --role pm

# QA: sees specs + testplans
qalam context --role qa

# Service-specific
qalam context --service payment-service
```

---

## Skills System

Skills are Markdown files that describe coding patterns, conventions, and agent instructions for a tech stack. Qalam auto-detects and scaffolds skills on `init`.

**Auto-detected stacks:** Rust, Go, Node/TypeScript, Python, Java, Kotlin, gRPC/Protobuf, Docker

**Install from registry:**

```bash
qalam skill install @golang
qalam skill install @grpc
qalam skill search
```

**Scaffold a custom skill:**

```bash
qalam skill install my-patterns
# Edit .qalam/skills/my-patterns/context.md
```

**Publish to registry** (needs `GITHUB_TOKEN`):

```bash
qalam skill publish my-patterns
# Opens a PR to galihsatriawan/qalam-skills
```

---

## AI-Assisted Init

If `ANTHROPIC_API_KEY` is set, Qalam can analyze your source files and infer project-specific patterns:

```bash
export ANTHROPIC_API_KEY=sk-...
qalam init --ai
```

This samples your source files, calls Claude Haiku, and appends project-specific conventions to each skill's `context.md`.

---

## Doctor

```bash
qalam doctor        # check project health
qalam doctor --fix  # auto-fix what's possible
```

Checks:
- `.qalam/` subdirs exist
- Every RFC has a corresponding spec
- Every spec with services has tasks generated
- Every spec has a testplan
- Skill `context.md` files are filled in (not just templates)
- Claude Code hook is installed
- `depends_on` references are valid

Auto-fixable: missing subdirs, missing specs (runs `qalam spec generate`), missing tasks, missing testplans, missing hooks.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Enable `qalam init --ai` |
| `GITHUB_TOKEN` | Enable `qalam skill publish` |

---

## Privacy

All data stays local. Qalam never sends your code or specs to any server unless you explicitly run `--ai` (Anthropic API) or `skill publish` (GitHub API).

The MCP server runs locally on stdio — no network port opened.

---

## Contributing

```bash
git clone https://github.com/galihsatriawan/qalam
cd qalam
cargo build
cargo test
```

Skill registry: [galihsatriawan/qalam-skills](https://github.com/galihsatriawan/qalam-skills)

---

## License

MIT
