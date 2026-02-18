use clap::Args;
use lineark_sdk::generated::inputs::ProjectCreateInput;
use lineark_sdk::generated::types::Project;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{resolve_team_ids, resolve_user_id};
use crate::output::{self, Format};

/// Manage projects.
#[derive(Debug, Args)]
pub struct ProjectsCmd {
    #[command(subcommand)]
    pub action: ProjectsAction,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ProjectsAction {
    /// List all projects.
    List,
    /// Create a new project.
    ///
    /// Examples:
    ///   lineark projects create "My Project" --team ENG
    ///   lineark projects create "Q4 Initiative" --team ENG,DESIGN --description "Cross-team effort"
    ///   lineark projects create "Alpha" --team ENG --lead "Jane Doe" --target-date 2026-06-01
    Create {
        /// Project name.
        name: String,
        /// Team(s) to associate with (key, name, or UUID). Required. Comma-separated for multiple.
        #[arg(long, required = true, value_delimiter = ',')]
        team: Vec<String>,
        /// Project description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// Project lead: user name, display name, or UUID.
        #[arg(long)]
        lead: Option<String>,
        /// Planned start date (YYYY-MM-DD).
        #[arg(long)]
        start_date: Option<String>,
        /// Planned target/completion date (YYYY-MM-DD).
        #[arg(long)]
        target_date: Option<String>,
        /// Priority: 0=none, 1=urgent, 2=high, 3=medium, 4=low.
        #[arg(short = 'p', long, value_parser = clap::value_parser!(i64).range(0..=4))]
        priority: Option<i64>,
        /// Markdown content for the project.
        #[arg(long)]
        content: Option<String>,
        /// Project icon (emoji or icon name).
        #[arg(long)]
        icon: Option<String>,
        /// Project color (hex color code).
        #[arg(long)]
        color: Option<String>,
    },
}

#[derive(Debug, Serialize, Tabled)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub slug_id: String,
}

/// Lean result type for project mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Project)]
#[serde(rename_all = "camelCase", default)]
struct ProjectRef {
    id: Option<String>,
    name: Option<String>,
    slug_id: Option<String>,
}

pub async fn run(cmd: ProjectsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        ProjectsAction::List => {
            let conn = client
                .projects::<Project>()
                .first(250)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<ProjectRow> = conn
                .nodes
                .iter()
                .map(|p| ProjectRow {
                    id: p.id.clone().unwrap_or_default(),
                    name: p.name.clone().unwrap_or_default(),
                    slug_id: p.slug_id.clone().unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
        ProjectsAction::Create {
            name,
            team,
            description,
            lead,
            start_date,
            target_date,
            priority,
            content,
            icon,
            color,
        } => {
            let team_ids = resolve_team_ids(client, &team).await?;

            let lead_id = match lead {
                Some(ref l) => Some(resolve_user_id(client, l).await?),
                None => None,
            };

            let start_date = start_date
                .map(|d| d.parse::<chrono::NaiveDate>())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid start-date (expected YYYY-MM-DD): {}", e))?;

            let target_date = target_date
                .map(|d| d.parse::<chrono::NaiveDate>())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid target-date (expected YYYY-MM-DD): {}", e))?;

            let input = ProjectCreateInput {
                name: Some(name),
                team_ids: Some(team_ids),
                description,
                lead_id,
                start_date,
                target_date,
                priority,
                content,
                icon,
                color,
                ..Default::default()
            };

            let project = client
                .project_create::<ProjectRef>(None, input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&project, format);
        }
    }
    Ok(())
}
