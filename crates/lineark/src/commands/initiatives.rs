use clap::Args;
use lineark_sdk::generated::enums::InitiativeStatus;
use lineark_sdk::generated::inputs::{
    InitiativeCreateInput, InitiativeToProjectCreateInput, InitiativeUpdateInput,
};
use lineark_sdk::generated::types::{
    Initiative, InitiativeToProject, Project, ProjectConnection, User,
};
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{resolve_initiative_id, resolve_project_id, resolve_user_id_or_me};
use crate::output::{self, Format};

/// Parse an initiative status string into the generated enum.
fn parse_initiative_status(s: &str) -> anyhow::Result<InitiativeStatus> {
    match s.to_lowercase().as_str() {
        "planned" => Ok(InitiativeStatus::Planned),
        "active" => Ok(InitiativeStatus::Active),
        "completed" => Ok(InitiativeStatus::Completed),
        _ => Err(anyhow::anyhow!(
            "Invalid initiative status '{}'. Valid values: Planned, Active, Completed",
            s
        )),
    }
}

/// Manage initiatives.
#[derive(Debug, Args)]
pub struct InitiativesCmd {
    #[command(subcommand)]
    pub action: InitiativesAction,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum InitiativesAction {
    /// List all initiatives.
    ///
    /// Examples:
    ///   lineark initiatives list
    ///   lineark initiatives list --limit 10
    List {
        /// Maximum number of initiatives to return.
        #[arg(short = 'l', long, default_value = "50")]
        limit: i64,
    },
    /// Show full details for a single initiative.
    ///
    /// Examples:
    ///   lineark initiatives read "Q1 Goals"
    ///   lineark initiatives read INITIATIVE-UUID
    Read {
        /// Initiative name or UUID.
        id: String,
    },
    /// Create a new initiative.
    ///
    /// Examples:
    ///   lineark initiatives create "Q1 Goals"
    ///   lineark initiatives create "Q1 Goals" --status Active --owner me
    ///   lineark initiatives create "Launch v2" --description "Ship v2 by Q2" --target-date 2026-06-30
    Create {
        /// Initiative name.
        name: String,
        /// Initiative description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// Owner: user name, display name, UUID, or `me`.
        #[arg(long)]
        owner: Option<String>,
        /// Status: Planned, Active, or Completed.
        #[arg(long)]
        status: Option<String>,
        /// Estimated completion date (YYYY-MM-DD).
        #[arg(long)]
        target_date: Option<String>,
        /// Initiative color (hex code).
        #[arg(long)]
        color: Option<String>,
        /// Initiative icon (emoji or icon name).
        #[arg(long)]
        icon: Option<String>,
    },
    /// Update an existing initiative.
    ///
    /// Examples:
    ///   lineark initiatives update "Q1 Goals" --status Active
    ///   lineark initiatives update INITIATIVE-UUID --name "Q2 Goals" --owner me
    Update {
        /// Initiative name or UUID.
        id: String,
        /// New initiative name.
        #[arg(long)]
        name: Option<String>,
        /// New description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// New owner: user name, display name, UUID, or `me`.
        #[arg(long)]
        owner: Option<String>,
        /// New status: Planned, Active, or Completed.
        #[arg(long)]
        status: Option<String>,
        /// New estimated completion date (YYYY-MM-DD).
        #[arg(long)]
        target_date: Option<String>,
    },
    /// Archive an initiative.
    ///
    /// Examples:
    ///   lineark initiatives archive "Q1 Goals"
    ///   lineark initiatives archive INITIATIVE-UUID
    Archive {
        /// Initiative name or UUID.
        id: String,
    },
    /// Unarchive a previously archived initiative.
    ///
    /// Examples:
    ///   lineark initiatives unarchive "Q1 Goals"
    ///   lineark initiatives unarchive INITIATIVE-UUID
    Unarchive {
        /// Initiative name or UUID.
        id: String,
    },
    /// Delete an initiative.
    ///
    /// Examples:
    ///   lineark initiatives delete "Q1 Goals"
    ///   lineark initiatives delete INITIATIVE-UUID
    Delete {
        /// Initiative name or UUID.
        id: String,
    },
    /// Manage project links for an initiative.
    Projects {
        #[command(subcommand)]
        action: ProjectsAction,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum ProjectsAction {
    /// Link a project to an initiative.
    ///
    /// Examples:
    ///   lineark initiatives projects add "Q1 Goals" --project "Mobile App UX"
    ///   lineark initiatives projects add INITIATIVE-UUID --project PROJECT-UUID
    Add {
        /// Initiative name or UUID.
        initiative: String,
        /// Project name or UUID to link.
        #[arg(long)]
        project: String,
    },
    /// Unlink a project from an initiative.
    ///
    /// Examples:
    ///   lineark initiatives projects remove "Q1 Goals" --project "Mobile App UX"
    ///   lineark initiatives projects remove INITIATIVE-UUID --project PROJECT-UUID
    Remove {
        /// Initiative name or UUID.
        initiative: String,
        /// Project name or UUID to unlink.
        #[arg(long)]
        project: String,
    },
}

// ── List row ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
pub struct InitiativeRow {
    pub id: String,
    pub name: String,
    pub status: String,
    pub target_date: String,
}

/// Lean type for `initiatives list`.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Initiative)]
#[serde(rename_all = "camelCase", default)]
struct InitiativeSummary {
    id: Option<String>,
    name: Option<String>,
    status: Option<InitiativeStatus>,
    target_date: Option<chrono::NaiveDate>,
}

// ── Read detail ──────────────────────────────────────────────────────────

/// Full initiative detail for `initiatives read`.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Initiative)]
#[serde(rename_all = "camelCase", default)]
struct InitiativeDetail {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    status: Option<InitiativeStatus>,
    target_date: Option<chrono::NaiveDate>,
    color: Option<String>,
    icon: Option<String>,
    url: Option<String>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    archived_at: Option<chrono::DateTime<chrono::Utc>>,
    #[graphql(nested)]
    owner: Option<OwnerRef>,
    #[graphql(nested)]
    projects: Option<InitiativeProjectsConnection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
struct OwnerRef {
    id: Option<String>,
    name: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = ProjectConnection)]
#[serde(rename_all = "camelCase", default)]
struct InitiativeProjectsConnection {
    #[graphql(nested)]
    nodes: Vec<InitiativeProjectRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Project)]
#[serde(rename_all = "camelCase", default)]
struct InitiativeProjectRef {
    id: Option<String>,
    name: Option<String>,
    slug_id: Option<String>,
}

// ── Mutation result ──────────────────────────────────────────────────────

/// Lean result type for initiative mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Initiative)]
#[serde(rename_all = "camelCase", default)]
struct InitiativeRef {
    id: Option<String>,
    name: Option<String>,
    status: Option<InitiativeStatus>,
}

/// Lean result type for initiative-to-project mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = InitiativeToProject)]
#[serde(rename_all = "camelCase", default)]
struct InitiativeToProjectRef {
    id: Option<String>,
}

// ── Command dispatch ────────────────────────────────────────────────────

pub async fn run(cmd: InitiativesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        InitiativesAction::List { limit } => {
            let conn = client
                .initiatives::<InitiativeSummary>()
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<InitiativeRow> = conn
                .nodes
                .iter()
                .map(|i| InitiativeRow {
                    id: i.id.clone().unwrap_or_default(),
                    name: i.name.clone().unwrap_or_default(),
                    status: i
                        .status
                        .as_ref()
                        .map(|s| format!("{:?}", s))
                        .unwrap_or_default(),
                    target_date: i.target_date.map(|d| d.to_string()).unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
        InitiativesAction::Read { id } => {
            let initiative_id = resolve_initiative_id(client, &id).await?;
            let initiative = client
                .initiative::<InitiativeDetail>(initiative_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&initiative, format);
        }
        InitiativesAction::Create {
            name,
            description,
            owner,
            status,
            target_date,
            color,
            icon,
        } => {
            let owner_id = match owner {
                Some(ref o) => Some(resolve_user_id_or_me(client, o).await?),
                None => None,
            };

            let parsed_status = status.map(|s| parse_initiative_status(&s)).transpose()?;

            let target_date = target_date
                .map(|d| d.parse::<chrono::NaiveDate>())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid target-date (expected YYYY-MM-DD): {}", e))?;

            let input = InitiativeCreateInput {
                name: Some(name),
                description,
                owner_id,
                status: parsed_status,
                target_date,
                color,
                icon,
                ..Default::default()
            };

            let initiative = client
                .initiative_create::<InitiativeRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&initiative, format);
        }
        InitiativesAction::Update {
            id,
            name,
            description,
            owner,
            status,
            target_date,
        } => {
            if name.is_none()
                && description.is_none()
                && owner.is_none()
                && status.is_none()
                && target_date.is_none()
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --description, --owner, --status, or --target-date."
                ));
            }

            let initiative_id = resolve_initiative_id(client, &id).await?;

            let owner_id = match owner {
                Some(ref o) => Some(resolve_user_id_or_me(client, o).await?),
                None => None,
            };

            let parsed_status = status.map(|s| parse_initiative_status(&s)).transpose()?;

            let target_date = target_date
                .map(|d| d.parse::<chrono::NaiveDate>())
                .transpose()
                .map_err(|e| anyhow::anyhow!("Invalid target-date (expected YYYY-MM-DD): {}", e))?;

            let input = InitiativeUpdateInput {
                name,
                description,
                owner_id,
                status: parsed_status,
                target_date,
                ..Default::default()
            };

            let initiative = client
                .initiative_update::<InitiativeRef>(input, initiative_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&initiative, format);
        }
        InitiativesAction::Archive { id } => {
            let initiative_id = resolve_initiative_id(client, &id).await?;

            let initiative = client
                .initiative_archive::<InitiativeRef>(initiative_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&initiative, format);
        }
        InitiativesAction::Unarchive { id } => {
            let initiative_id = resolve_initiative_id(client, &id).await?;

            let initiative = client
                .initiative_unarchive::<InitiativeRef>(initiative_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&initiative, format);
        }
        InitiativesAction::Delete { id } => {
            let initiative_id = resolve_initiative_id(client, &id).await?;

            let result = client
                .initiative_delete(initiative_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
        InitiativesAction::Projects { action } => match action {
            ProjectsAction::Add {
                initiative,
                project,
            } => {
                let initiative_id = resolve_initiative_id(client, &initiative).await?;
                let project_id = resolve_project_id(client, &project).await?;

                let input = InitiativeToProjectCreateInput {
                    initiative_id: Some(initiative_id),
                    project_id: Some(project_id),
                    ..Default::default()
                };

                let join = client
                    .initiative_to_project_create::<InitiativeToProjectRef>(input)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                output::print_one(&join, format);
            }
            ProjectsAction::Remove {
                initiative,
                project,
            } => {
                let initiative_id = resolve_initiative_id(client, &initiative).await?;
                let project_id = resolve_project_id(client, &project).await?;

                // To delete the link we need the InitiativeToProject join entity ID.
                // The Linear API only returns this from the create mutation; there is
                // no standalone query for join entities. We use a create-then-delete
                // pattern: creating a duplicate link returns the existing join entity
                // (idempotent), giving us its ID, which we then delete.
                let input = InitiativeToProjectCreateInput {
                    initiative_id: Some(initiative_id),
                    project_id: Some(project_id),
                    ..Default::default()
                };

                let join = client
                    .initiative_to_project_create::<InitiativeToProjectRef>(input)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let join_id = join.id.ok_or_else(|| {
                    anyhow::anyhow!("Failed to resolve initiative-to-project link")
                })?;

                let result = client
                    .initiative_to_project_delete(join_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                output::print_one(&result, format);
            }
        },
    }
    Ok(())
}
