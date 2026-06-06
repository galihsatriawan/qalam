use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::commands::{
    audit::{self, MetricsExport},
    breakdown, completions, context, doctor, edit,
    export::{self, ExportFormat},
    git_hook,
    graph::{self, GraphFormat},
    hook, init, rfc, serve, skill, spec, status, testplan, ui,
};
use crate::commands::context::Role;

#[derive(Parser)]
#[command(name = "qalam", about = "Spec-driven AI development workflow")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    // ── Project lifecycle ──────────────────────────────────────────────────────
    /// Initialize qalam in the current repository (auto-detects tech stack)
    Init {
        #[arg(long)]
        ai: bool,
    },
    /// Show project status dashboard
    Status,
    /// Validate project structure and completeness
    Doctor {
        #[arg(long)]
        fix: bool,
    },

    // ── SDLC flow ──────────────────────────────────────────────────────────────
    /// Manage RFCs
    Rfc {
        #[command(subcommand)]
        action: rfc::Action,
    },
    /// Manage specs
    Spec {
        #[command(subcommand)]
        action: spec::Action,
    },
    /// Break down a spec into tasks per service
    Breakdown {
        #[arg(long)]
        from: String,
        /// Use AI to add implementation notes per service
        #[arg(long)]
        ai: bool,
    },
    /// Generate a test plan from a spec
    Testplan {
        #[arg(long)]
        from: String,
        /// Use AI to generate edge cases and negative tests
        #[arg(long)]
        ai: bool,
    },

    // ── Context & AI integration ───────────────────────────────────────────────
    /// Print qalam context for the current project (used by hook)
    Context {
        #[arg(long, value_enum, default_value = "engineer")]
        role: Role,
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        watch: bool,
        /// Only include artifacts modified since this date (YYYY-MM-DD) or git ref
        #[arg(long)]
        since: Option<String>,
        /// Read context from a different repo's .qalam/ directory
        #[arg(long)]
        repo: Option<String>,
    },
    /// Manage Claude Code hook integration
    Hook {
        #[command(subcommand)]
        action: hook::Action,
    },
    /// Start the MCP server (stdio transport)
    Serve,

    // ── Skills ─────────────────────────────────────────────────────────────────
    /// Manage skills (install, list, remove, search, publish, expose, update, diff)
    Skill {
        #[command(subcommand)]
        action: skill::Action,
    },

    // ── M2: Dependency & Impact Graph ──────────────────────────────────────────
    /// Render RFC→spec→service dependency graph
    Graph {
        #[arg(long, value_enum, default_value = "ascii")]
        format: GraphFormat,
    },
    /// Show impact of a service or RFC across specs
    Impact {
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        rfc: Option<String>,
    },

    // ── M3: Editor & Git Integration ───────────────────────────────────────────
    /// Open an RFC, spec, or testplan in $EDITOR
    Edit {
        /// Artifact id prefix (e.g. RFC-001, SPEC-001)
        id: String,
    },
    /// Show git diff for an artifact
    Diff {
        id: String,
    },
    /// Show git log for an artifact
    Log {
        id: String,
    },
    /// Manage git pre-commit hook integration
    GitHook {
        #[command(subcommand)]
        action: git_hook::Action,
    },
    /// Stage and commit all .qalam/ changes with a standardized message
    Commit,

    // ── M4: Multi-Repo & Team Sync ─────────────────────────────────────────────
    /// Pull latest .qalam/ from repos referenced in qalam.yaml
    Sync,
    /// Export full project state (RFCs, specs, tasks, testplans)
    Export {
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,
    },

    // ── M5: Integrations ───────────────────────────────────────────────────────
    /// Generate partial OpenAPI 3.0 YAML from a spec's contracts
    Openapi {
        #[arg(long)]
        from: String,
    },
    /// Generate a Postman collection from a spec's contracts
    Postman {
        #[arg(long)]
        from: String,
    },

    // ── M9: Compliance & Governance ────────────────────────────────────────────
    /// Audit specs by service or tag
    Audit {
        #[arg(long)]
        service: Option<String>,
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show SDLC health metrics
    Metrics {
        #[arg(long)]
        service: Option<String>,
        #[arg(long, value_enum)]
        export: Option<MetricsExport>,
    },

    // ── M10: Local Web UI ──────────────────────────────────────────────────────
    /// Start a local web dashboard (default: http://localhost:7734)
    Ui {
        #[arg(long, default_value = "7734")]
        port: u16,
    },

    // ── Shell completions ──────────────────────────────────────────────────────
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

pub fn build_cli() -> clap::Command {
    Cli::command()
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { ai }           => init::run(ai).await,
        Command::Status                => status::run().await,
        Command::Doctor { fix }        => doctor::run(fix).await,
        Command::Rfc { action }        => rfc::run(action).await,
        Command::Spec { action }       => spec::run(action).await,
        Command::Breakdown { from, ai } => breakdown::run(&from, ai).await,
        Command::Testplan { from, ai }  => testplan::run(&from, ai).await,
        Command::Context { role, service, watch, since, repo } => {
            context::run(role, watch, service, since, repo).await
        }
        Command::Hook { action }       => hook::run(action).await,
        Command::Serve                 => serve::run().await,
        Command::Skill { action }      => skill::run(action).await,

        Command::Graph { format }      => graph::run_graph(format).await,
        Command::Impact { service, rfc } => {
            graph::run_impact(service.as_deref(), rfc.as_deref()).await
        }

        Command::Edit { id }           => edit::run_edit(&id).await,
        Command::Diff { id }           => edit::run_diff(&id).await,
        Command::Log { id }            => edit::run_log(&id).await,
        Command::GitHook { action }    => git_hook::run(action).await,
        Command::Commit                => edit::run_commit().await,

        Command::Sync                  => export::run_sync().await,
        Command::Export { format }     => export::run_export(format).await,
        Command::Openapi { from }      => export::run_openapi(&from).await,
        Command::Postman { from }      => export::run_postman(&from).await,

        Command::Audit { service, tag } => {
            audit::run_audit(service.as_deref(), tag.as_deref()).await
        }
        Command::Metrics { service, export } => {
            audit::run_metrics(service.as_deref(), export).await
        }
        Command::Ui { port }           => ui::run_ui(port).await,

        Command::Completions { shell } => {
            completions::run(shell);
            Ok(())
        }
    }
}
