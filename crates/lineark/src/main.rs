mod commands;
mod output;

use clap::{Parser, Subcommand};
use lineark_sdk::Client;

/// lineark — Linear CLI for humans and LLMs
#[derive(Debug, Parser)]
#[command(name = "lineark", version, about)]
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
    /// Print a compact LLM-friendly command reference.
    Usage,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let format = output::resolve_format(cli.format);

    // Handle usage command before auth (it doesn't need a client).
    if matches!(cli.command, Command::Usage) {
        commands::usage::run();
        return;
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
        Command::Usage => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
