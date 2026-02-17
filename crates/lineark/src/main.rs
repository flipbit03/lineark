mod commands;
mod output;
mod version_check;

use clap::{Parser, Subcommand};
use lineark_sdk::Client;

/// lineark — Linear CLI for humans and LLMs
#[derive(Debug, Parser)]
#[command(name = "lineark", version, about, after_help = update_hint_blocking())]
struct Cli {
    /// API token (overrides $LINEAR_API_TOKEN and ~/.linear_api_token).
    #[arg(long, global = true)]
    api_token: Option<String>,

    /// Output format. Auto-detected if not specified (human for terminal, json for pipe).
    #[arg(long, global = true)]
    format: Option<output::Format>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show authenticated user info.
    Whoami,
    /// Manage teams.
    Teams(commands::teams::TeamsCmd),
    /// Manage users.
    Users(commands::users::UsersCmd),
    /// Manage projects.
    Projects(commands::projects::ProjectsCmd),
    /// Manage issue labels.
    Labels(commands::labels::LabelsCmd),
    /// Manage cycles.
    Cycles(commands::cycles::CyclesCmd),
    /// Manage issues.
    Issues(commands::issues::IssuesCmd),
    /// Manage comments.
    Comments(commands::comments::CommentsCmd),
    /// Manage documents.
    Documents(commands::documents::DocumentsCmd),
    /// Manage project milestones.
    ProjectMilestones(commands::milestones::MilestonesCmd),
    /// Manage file embeds (download/upload).
    Embeds(commands::embeds::EmbedsCmd),
    /// Print a compact LLM-friendly command reference.
    Usage,
    /// Manage lineark itself (update, etc.).
    #[command(name = "self")]
    SelfCmd(commands::self_cmd::SelfCmd),
}

/// Build an update hint string using a blocking one-shot tokio runtime for use in clap's
/// `after_help` (which requires a plain string at parse time). Uses the cached version check
/// (no network call unless cache is stale/missing).
fn update_hint_blocking() -> String {
    if version_check::is_dev_build() {
        return String::new();
    }
    // Use a lightweight current-thread runtime so we don't conflict with the main runtime.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    let latest = match rt {
        Ok(rt) => rt.block_on(version_check::get_latest_version(false)),
        Err(_) => None,
    };
    format_update_hint(latest.as_deref())
}

/// Format the update hint string. Shared by --help and usage.
pub fn format_update_hint(latest: Option<&str>) -> String {
    let current = version_check::current_version();
    match latest {
        Some(v) if v != current => {
            format!("\nUpdate available: {current} → {v}\nRun `lineark self update` to upgrade.\n")
        }
        _ => String::new(),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let format = output::resolve_format(cli.format);

    // Handle commands that don't need auth.
    match cli.command {
        Command::Usage => {
            commands::usage::run().await;
            return;
        }
        Command::SelfCmd(cmd) => {
            if let Err(e) = commands::self_cmd::run(cmd).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }

    // Resolve client.
    let client = match &cli.api_token {
        Some(token) => Client::from_token(token),
        None => Client::auto(),
    };
    let client = match client {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let result = match cli.command {
        Command::Whoami => commands::viewer::run(&client, format).await,
        Command::Teams(cmd) => commands::teams::run(cmd, &client, format).await,
        Command::Users(cmd) => commands::users::run(cmd, &client, format).await,
        Command::Projects(cmd) => commands::projects::run(cmd, &client, format).await,
        Command::Labels(cmd) => commands::labels::run(cmd, &client, format).await,
        Command::Cycles(cmd) => commands::cycles::run(cmd, &client, format).await,
        Command::Issues(cmd) => commands::issues::run(cmd, &client, format).await,
        Command::Comments(cmd) => commands::comments::run(cmd, &client, format).await,
        Command::Documents(cmd) => commands::documents::run(cmd, &client, format).await,
        Command::Embeds(cmd) => commands::embeds::run(cmd, &client, format).await,
        Command::ProjectMilestones(cmd) => commands::milestones::run(cmd, &client, format).await,
        Command::Usage | Command::SelfCmd(_) => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
