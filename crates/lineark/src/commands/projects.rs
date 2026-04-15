use clap::Args;
use lineark_sdk::generated::inputs::{ProjectCreateInput, ProjectFilter, ProjectUpdateInput};
use lineark_sdk::generated::types::{
    Project, ProjectStatus, Team, TeamConnection, User, UserConnection,
};
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{
    parse_priority, resolve_project_id, resolve_project_label_ids, resolve_project_status_id,
    resolve_team_ids, resolve_user_id_or_me, resolve_user_ids_or_me,
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
        /// Priority: 0-4 or none, urgent, high, medium, low.
        #[arg(short = 'p', long, value_parser = parse_priority)]
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
    /// Update an existing project. Returns the updated project.
    ///
    /// Examples:
    ///   lineark projects update "Mobile App UX" --description "Updated scope"
    ///   lineark projects update PROJECT-UUID --lead me --priority high
    ///   lineark projects update "Alpha" --status "In Progress" --target-date 2026-06-01
    ///   lineark projects update "Alpha" --clear-lead --clear-target-date
    Update {
        /// Project name or UUID.
        id: String,
        /// New project name.
        #[arg(long)]
        name: Option<String>,
        /// Project description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// Markdown content for the project.
        #[arg(long)]
        content: Option<String>,
        /// Project lead: user name, display name, UUID, or `me`.
        #[arg(long)]
        lead: Option<String>,
        /// Remove the project lead.
        #[arg(long, default_value = "false", conflicts_with = "lead")]
        clear_lead: bool,
        /// Project members: comma-separated user names, display names, UUIDs, or `me`. Replaces the existing set.
        #[arg(long, value_delimiter = ',')]
        members: Option<Vec<String>>,
        /// Team(s) to associate with (key, name, or UUID). Comma-separated. Replaces the existing set.
        #[arg(long, value_delimiter = ',')]
        team: Option<Vec<String>>,
        /// Planned start date (YYYY-MM-DD).
        #[arg(long)]
        start_date: Option<String>,
        /// Remove the planned start date.
        #[arg(long, default_value = "false", conflicts_with = "start_date")]
        clear_start_date: bool,
        /// Planned target/completion date (YYYY-MM-DD).
        #[arg(long)]
        target_date: Option<String>,
        /// Remove the planned target date.
        #[arg(long, default_value = "false", conflicts_with = "target_date")]
        clear_target_date: bool,
        /// Priority: 0-4 or none, urgent, high, medium, low.
        #[arg(short = 'p', long, value_parser = parse_priority)]
        priority: Option<i64>,
        /// Project status name or UUID.
        #[arg(long)]
        status: Option<String>,
        /// Project icon (emoji or icon name).
        #[arg(long)]
        icon: Option<String>,
        /// Project color (hex color code).
        #[arg(long)]
        color: Option<String>,
        /// Comma-separated project label names or UUIDs. Replaces the existing set.
        #[arg(long, value_delimiter = ',')]
        labels: Option<Vec<String>>,
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
        ProjectsAction::Update {
            id,
            name,
            description,
            content,
            lead,
            clear_lead,
            members,
            team,
            start_date,
            clear_start_date,
            target_date,
            clear_target_date,
            priority,
            status,
            icon,
            color,
            labels,
        } => {
            if name.is_none()
                && description.is_none()
                && content.is_none()
                && lead.is_none()
                && !clear_lead
                && members.is_none()
                && team.is_none()
                && start_date.is_none()
                && !clear_start_date
                && target_date.is_none()
                && !clear_target_date
                && priority.is_none()
                && status.is_none()
                && icon.is_none()
                && color.is_none()
                && labels.is_none()
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --description, --content, --lead, --members, --team, --start-date, --target-date, --priority, --status, --icon, --color, or --labels to specify changes."
                ));
            }

            let project_id = resolve_project_id(client, &id).await?;

            let lead_id = match lead {
                Some(ref l) => Some(resolve_user_id_or_me(client, l).await?),
                None => None,
            };

            let member_ids = match members {
                Some(ref m) => Some(resolve_user_ids_or_me(client, m).await?),
                None => None,
            };

            let team_ids = match team {
                Some(ref t) => Some(resolve_team_ids(client, t).await?),
                None => None,
            };

            let status_id = match status {
                Some(ref s) => Some(resolve_project_status_id(client, s).await?),
                None => None,
            };

            let label_ids = match labels {
                Some(ref l) => Some(resolve_project_label_ids(client, l).await?),
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

            let input = ProjectUpdateInput {
                name,
                description,
                content,
                lead_id,
                member_ids,
                team_ids,
                start_date,
                target_date,
                priority,
                status_id,
                icon,
                color,
                label_ids,
                ..Default::default()
            };

            // When --clear-* flags are used, we need to send `null` for the
            // relevant field. ProjectUpdateInput uses skip_serializing_if so
            // None omits the field (no-op). We serialize to Value and inject null.
            let project = if clear_lead || clear_start_date || clear_target_date {
                let mut input_val = serde_json::to_value(&input)?;
                let obj = input_val.as_object_mut().unwrap();
                if clear_lead {
                    obj.insert("leadId".to_string(), serde_json::Value::Null);
                }
                if clear_start_date {
                    obj.insert("startDate".to_string(), serde_json::Value::Null);
                }
                if clear_target_date {
                    obj.insert("targetDate".to_string(), serde_json::Value::Null);
                }
                let variables = serde_json::json!({ "input": input_val, "id": project_id });
                let sel = <ProjectRef as GraphQLFields>::selection();
                let query = format!(
                    "mutation($input: ProjectUpdateInput!, $id: String!) {{ projectUpdate(input: $input, id: $id) {{ success project {{ {sel} }} }} }}"
                );
                let payload: serde_json::Value = client
                    .execute(&query, variables, "projectUpdate")
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                serde_json::from_value::<ProjectRef>(
                    payload.get("project").cloned().unwrap_or_default(),
                )?
            } else {
                client
                    .project_update::<ProjectRef>(input, project_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?
            };

            output::print_one(&project, format);
        }
    }
    Ok(())
}
