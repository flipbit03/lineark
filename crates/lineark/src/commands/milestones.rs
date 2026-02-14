use clap::Args;
use lineark_sdk::generated::enums::ProjectMilestoneStatus;
use lineark_sdk::generated::inputs::{
    ProjectMilestoneCreateInput, ProjectMilestoneFilter, ProjectMilestoneUpdateInput,
};
use lineark_sdk::generated::types::ProjectMilestone;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::resolve_project_id;
use crate::output::{self, Format};

/// Manage project milestones.
#[derive(Debug, Args)]
pub struct MilestonesCmd {
    #[command(subcommand)]
    pub action: MilestonesAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum MilestonesAction {
    /// List milestones for a project.
    ///
    /// Examples:
    ///   lineark project-milestones list --project "My Project"
    ///   lineark project-milestones list --project PROJECT-UUID --limit 10
    List {
        /// Project name or UUID.
        #[arg(long)]
        project: String,
        /// Maximum number of milestones to return (max 250).
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
    },
    /// Read a specific milestone by UUID or by name (with --project).
    ///
    /// Examples:
    ///   lineark project-milestones read MILESTONE-UUID
    ///   lineark project-milestones read "Beta Release" --project "My Project"
    Read {
        /// Milestone UUID or name.
        id: String,
        /// Project name or UUID (required for name lookup).
        #[arg(long)]
        project: Option<String>,
    },
    /// Create a new milestone.
    ///
    /// Examples:
    ///   lineark project-milestones create "Beta Release" --project "My Project"
    ///   lineark project-milestones create "v1.0" --project "My Project" --target-date 2025-06-01
    Create {
        /// Milestone name.
        name: String,
        /// Project name or UUID.
        #[arg(long)]
        project: String,
        /// Target date (YYYY-MM-DD).
        #[arg(long)]
        target_date: Option<String>,
        /// Description in markdown.
        #[arg(long)]
        description: Option<String>,
    },
    /// Update an existing milestone.
    ///
    /// Examples:
    ///   lineark project-milestones update MILESTONE-UUID --name "GA Release"
    ///   lineark project-milestones update "Beta" --project "My Project" --target-date 2025-07-01
    Update {
        /// Milestone UUID or name (with --project).
        id: String,
        /// Project name or UUID (required for name lookup).
        #[arg(long)]
        project: Option<String>,
        /// New name.
        #[arg(long)]
        name: Option<String>,
        /// New target date (YYYY-MM-DD).
        #[arg(long)]
        target_date: Option<String>,
        /// New description in markdown.
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a milestone.
    ///
    /// Examples:
    ///   lineark project-milestones delete MILESTONE-UUID
    ///   lineark project-milestones delete "Beta Release" --project "My Project"
    Delete {
        /// Milestone UUID or name (with --project).
        id: String,
        /// Project name or UUID (required for name lookup).
        #[arg(long)]
        project: Option<String>,
    },
}

// ── Lean types ───────────────────────────────────────────────────────────────

/// Lean milestone type for list views.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = ProjectMilestone)]
#[serde(rename_all = "camelCase", default)]
struct MilestoneSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    pub target_date: Option<chrono::NaiveDate>,
    pub description: Option<String>,
    pub status: Option<ProjectMilestoneStatus>,
}

/// Lean result type for milestone mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = ProjectMilestone)]
#[serde(rename_all = "camelCase", default)]
struct MilestoneRef {
    id: Option<String>,
    name: Option<String>,
}

// ── Table row ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
struct MilestoneRow {
    id: String,
    name: String,
    target_date: String,
    status: String,
}

impl From<&MilestoneSummary> for MilestoneRow {
    fn from(m: &MilestoneSummary) -> Self {
        Self {
            id: m.id.clone().unwrap_or_default(),
            name: m.name.clone().unwrap_or_default(),
            target_date: m
                .target_date
                .map(|d: chrono::NaiveDate| d.to_string())
                .unwrap_or_default(),
            status: m
                .status
                .as_ref()
                .map(|s| format!("{:?}", s).to_lowercase())
                .unwrap_or_default(),
        }
    }
}

// ── Command dispatch ─────────────────────────────────────────────────────────

pub async fn run(cmd: MilestonesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        MilestonesAction::List { project, limit } => {
            let project_id = resolve_project_id(client, &project).await?;

            let filter: ProjectMilestoneFilter = serde_json::from_value(
                serde_json::json!({ "project": { "id": { "eq": project_id } } }),
            )
            .expect("valid ProjectMilestoneFilter");

            let conn = client
                .project_milestones::<MilestoneSummary>()
                .filter(filter)
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            match format {
                Format::Json => {
                    let json = serde_json::to_string_pretty(&conn.nodes).unwrap_or_default();
                    println!("{json}");
                }
                Format::Human => {
                    let rows: Vec<MilestoneRow> =
                        conn.nodes.iter().map(MilestoneRow::from).collect();
                    output::print_table(&rows, format);
                }
            }
        }
        MilestonesAction::Read { id, project } => {
            let milestone_id = resolve_milestone_id(client, &id, project.as_deref()).await?;

            let milestone = client
                .project_milestone::<ProjectMilestone>(milestone_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&milestone, format);
        }
        MilestonesAction::Create {
            name,
            project,
            target_date,
            description,
        } => {
            let project_id = resolve_project_id(client, &project).await?;

            let target_date = target_date
                .map(|d| d.parse::<chrono::NaiveDate>())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid date format (expected YYYY-MM-DD): {}", e))?;

            let input = ProjectMilestoneCreateInput {
                name: Some(name),
                project_id: Some(project_id),
                target_date,
                description,
                ..Default::default()
            };

            let milestone = client
                .project_milestone_create::<MilestoneRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&milestone, format);
        }
        MilestonesAction::Update {
            id,
            project,
            name,
            target_date,
            description,
        } => {
            if name.is_none() && target_date.is_none() && description.is_none() {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --target-date, or --description."
                ));
            }

            let milestone_id = resolve_milestone_id(client, &id, project.as_deref()).await?;

            let target_date = target_date
                .map(|d| d.parse::<chrono::NaiveDate>())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid date format (expected YYYY-MM-DD): {}", e))?;

            let input = ProjectMilestoneUpdateInput {
                name,
                target_date,
                description,
                ..Default::default()
            };

            let milestone = client
                .project_milestone_update::<MilestoneRef>(input, milestone_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&milestone, format);
        }
        MilestonesAction::Delete { id, project } => {
            let milestone_id = resolve_milestone_id(client, &id, project.as_deref()).await?;

            let result = client
                .project_milestone_delete(milestone_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
    }
    Ok(())
}

/// Resolve a milestone name or UUID to a UUID.
/// If the input is a UUID, return it directly. Otherwise, search within the given
/// project's milestones by name (case-insensitive).
async fn resolve_milestone_id(
    client: &Client,
    name_or_id: &str,
    project: Option<&str>,
) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(name_or_id).is_ok() {
        return Ok(name_or_id.to_string());
    }

    let project_name = project.ok_or_else(|| {
        anyhow::anyhow!(
            "Looking up milestones by name requires --project. \
             Use a UUID to reference without --project."
        )
    })?;

    let project_id = resolve_project_id(client, project_name).await?;

    let filter: ProjectMilestoneFilter =
        serde_json::from_value(serde_json::json!({ "project": { "id": { "eq": project_id } } }))
            .expect("valid ProjectMilestoneFilter");

    let conn = client
        .project_milestones::<MilestoneSummary>()
        .filter(filter)
        .first(250)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    conn.nodes
        .iter()
        .find(|m| {
            m.name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(name_or_id))
        })
        .and_then(|m| m.id.clone())
        .ok_or_else(|| {
            let available: Vec<String> = conn.nodes.iter().filter_map(|m| m.name.clone()).collect();
            if available.is_empty() {
                anyhow::anyhow!(
                    "Milestone '{}' not found in project '{}'",
                    name_or_id,
                    project_name
                )
            } else {
                anyhow::anyhow!(
                    "Milestone '{}' not found in project '{}'. Available: {}",
                    name_or_id,
                    project_name,
                    available.join(", ")
                )
            }
        })
}
