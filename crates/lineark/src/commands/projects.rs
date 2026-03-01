use clap::Args;
use lineark_sdk::generated::inputs::{ProjectCreateInput, ProjectFilter};
use lineark_sdk::generated::types::{
    Project, ProjectSearchResult, ProjectStatus, Team, TeamConnection, User, UserConnection,
};
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{
    resolve_project_id, resolve_team_id, resolve_team_ids, resolve_user_id_or_me,
    resolve_user_ids_or_me,
};
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
    ///
    /// Examples:
    ///   lineark projects list
    ///   lineark projects list --led-by-me
    List {
        /// Show only projects where the authenticated user is the lead.
        #[arg(long, default_value = "false")]
        led_by_me: bool,
    },
    /// Show full details for a single project, including lead, members, status, dates, teams, and description.
    ///
    /// Examples:
    ///   lineark projects read "Mobile App UX"
    ///   lineark projects read PROJECT-UUID
    Read {
        /// Project name or UUID.
        id: String,
    },
    /// Full-text search across project names and descriptions.
    ///
    /// Examples:
    ///   lineark projects search "mobile app"
    ///   lineark projects search "Q4" --limit 10
    ///   lineark projects search "infrastructure" --team ENG
    Search {
        /// Search query text.
        query: String,
        /// Maximum number of results (max 250).
        #[arg(short = 'l', long, default_value = "25", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Filter by team key, name, or UUID.
        #[arg(long)]
        team: Option<String>,
    },
    /// Create a new project.
    ///
    /// Examples:
    ///   lineark projects create "My Project" --team ENG
    ///   lineark projects create "Q4 Initiative" --team ENG,DESIGN --description "Cross-team effort"
    ///   lineark projects create "Alpha" --team ENG --lead me --members anna,rick
    Create {
        /// Project name.
        name: String,
        /// Team(s) to associate with (key, name, or UUID). Required. Comma-separated for multiple.
        #[arg(long, required = true, value_delimiter = ',')]
        team: Vec<String>,
        /// Project description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// Project lead: user name, display name, UUID, or `me`.
        #[arg(long)]
        lead: Option<String>,
        /// Project members: comma-separated user names, display names, UUIDs, or `me`.
        #[arg(long, value_delimiter = ',')]
        members: Option<Vec<String>>,
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

// ── List row ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub slug_id: String,
    pub lead: String,
}

/// Lean type for `projects list` — includes nested lead for the lead column.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Project)]
#[serde(rename_all = "camelCase", default)]
struct ProjectListRef {
    id: Option<String>,
    name: Option<String>,
    slug_id: Option<String>,
    #[graphql(nested)]
    lead: Option<LeadRef>,
}

// ── Read detail ──────────────────────────────────────────────────────────

/// Full project detail for `projects read`.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Project)]
#[serde(rename_all = "camelCase", default)]
struct ProjectDetail {
    id: Option<String>,
    name: Option<String>,
    slug_id: Option<String>,
    description: Option<String>,
    priority: Option<i64>,
    start_date: Option<chrono::NaiveDate>,
    target_date: Option<chrono::NaiveDate>,
    url: Option<String>,
    #[graphql(nested)]
    status: Option<StatusRef>,
    #[graphql(nested)]
    lead: Option<LeadRef>,
    #[graphql(nested)]
    members: Option<MembersConnection>,
    #[graphql(nested)]
    teams: Option<TeamsConnection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = ProjectStatus)]
#[serde(rename_all = "camelCase", default)]
struct StatusRef {
    name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
struct LeadRef {
    id: Option<String>,
    name: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = UserConnection)]
#[serde(rename_all = "camelCase", default)]
struct MembersConnection {
    #[graphql(nested)]
    nodes: Vec<MemberRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
struct MemberRef {
    id: Option<String>,
    name: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = TeamConnection)]
#[serde(rename_all = "camelCase", default)]
struct TeamsConnection {
    #[graphql(nested)]
    nodes: Vec<TeamRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Team)]
#[serde(rename_all = "camelCase", default)]
struct TeamRef {
    key: Option<String>,
    name: Option<String>,
}

// ── Mutation result ──────────────────────────────────────────────────────

/// Lean result type for project mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Project)]
#[serde(rename_all = "camelCase", default)]
struct ProjectRef {
    id: Option<String>,
    name: Option<String>,
    slug_id: Option<String>,
}

// ── Search types ────────────────────────────────────────────────────────

/// Lean search result type for `projects search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = ProjectSearchResult)]
#[serde(rename_all = "camelCase", default)]
struct ProjSearchSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    pub slug_id: Option<String>,
    pub url: Option<String>,
    #[graphql(nested)]
    pub lead: Option<LeadRef>,
}

#[derive(Debug, Serialize, Tabled)]
struct ProjSearchRow {
    id: String,
    name: String,
    slug_id: String,
    lead: String,
    url: String,
}

impl From<&ProjSearchSummary> for ProjSearchRow {
    fn from(p: &ProjSearchSummary) -> Self {
        Self {
            id: p.id.clone().unwrap_or_default(),
            name: p.name.clone().unwrap_or_default(),
            slug_id: p.slug_id.clone().unwrap_or_default(),
            lead: p
                .lead
                .as_ref()
                .and_then(|l| l.display_name.clone().or_else(|| l.name.clone()))
                .unwrap_or_default(),
            url: p.url.clone().unwrap_or_default(),
        }
    }
}

// ── Command dispatch ────────────────────────────────────────────────────

pub async fn run(cmd: ProjectsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        ProjectsAction::List { led_by_me } => {
            let mut builder = client.projects::<ProjectListRef>().first(250);

            if led_by_me {
                let viewer = client
                    .whoami::<User>()
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                let viewer_id = viewer
                    .id
                    .ok_or_else(|| anyhow::anyhow!("Could not determine authenticated user ID"))?;
                let filter: ProjectFilter = serde_json::from_value(
                    serde_json::json!({ "lead": { "id": { "eq": viewer_id } } }),
                )
                .expect("valid ProjectFilter");
                builder = builder.filter(filter);
            }

            let conn = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<ProjectRow> = conn
                .nodes
                .iter()
                .map(|p| ProjectRow {
                    id: p.id.clone().unwrap_or_default(),
                    name: p.name.clone().unwrap_or_default(),
                    slug_id: p.slug_id.clone().unwrap_or_default(),
                    lead: p
                        .lead
                        .as_ref()
                        .and_then(|l| l.display_name.clone().or_else(|| l.name.clone()))
                        .unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
        ProjectsAction::Read { id } => {
            let project_id = resolve_project_id(client, &id).await?;
            let project = client
                .project::<ProjectDetail>(project_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&project, format);
        }
        ProjectsAction::Search { query, limit, team } => {
            let mut builder = client
                .search_projects::<ProjSearchSummary>(query)
                .first(limit);

            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                builder = builder.team_id(team_id);
            }

            let conn = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<ProjSearchRow> = conn.nodes.iter().map(ProjSearchRow::from).collect();
            output::print_table(&rows, format);
        }
        ProjectsAction::Create {
            name,
            team,
            description,
            lead,
            members,
            start_date,
            target_date,
            priority,
            content,
            icon,
            color,
        } => {
            let team_ids = resolve_team_ids(client, &team).await?;

            let lead_id = match lead {
                Some(ref l) => Some(resolve_user_id_or_me(client, l).await?),
                None => None,
            };

            let member_ids = match members {
                Some(ref m) => Some(resolve_user_ids_or_me(client, m).await?),
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
                member_ids,
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
