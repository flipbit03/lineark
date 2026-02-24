use clap::Args;
use lineark_sdk::generated::inputs::{TeamCreateInput, TeamMembershipCreateInput, TeamUpdateInput};
use lineark_sdk::generated::types::{Team, TeamMembership, User, UserConnection};
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{resolve_team_id, resolve_user_id_or_me};
use crate::output::{self, Format};

/// Manage teams.
#[derive(Debug, Args)]
pub struct TeamsCmd {
    #[command(subcommand)]
    pub action: TeamsAction,
}

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum TeamsAction {
    /// List all teams.
    List,
    /// Show full details for a single team, including members and settings.
    ///
    /// Examples:
    ///   lineark teams read ENG
    ///   lineark teams read "Engineering"
    ///   lineark teams read TEAM-UUID
    Read {
        /// Team key, name, or UUID.
        id: String,
    },
    /// Create a new team.
    ///
    /// Examples:
    ///   lineark teams create "Backend"
    ///   lineark teams create "Frontend" --key FE --description "Frontend team"
    ///   lineark teams create "Platform" --cycles-enabled --triage-enabled
    Create {
        /// Team name.
        name: String,
        /// Team key (auto-generated from name if omitted).
        #[arg(long)]
        key: Option<String>,
        /// Team description.
        #[arg(long)]
        description: Option<String>,
        /// Team icon.
        #[arg(long)]
        icon: Option<String>,
        /// Team color (hex code).
        #[arg(long)]
        color: Option<String>,
        /// Team timezone (e.g. "America/New_York").
        #[arg(long)]
        timezone: Option<String>,
        /// Make the team private.
        #[arg(long)]
        private: Option<bool>,
        /// Enable cycles for the team.
        #[arg(long)]
        cycles_enabled: Option<bool>,
        /// Enable triage mode for the team.
        #[arg(long)]
        triage_enabled: Option<bool>,
    },
    /// Update an existing team.
    ///
    /// Examples:
    ///   lineark teams update ENG --description "Updated description"
    ///   lineark teams update "Backend" --name "Backend Services"
    Update {
        /// Team key, name, or UUID.
        id: String,
        /// New team name.
        #[arg(long)]
        name: Option<String>,
        /// New team key.
        #[arg(long)]
        key: Option<String>,
        /// New team description.
        #[arg(long)]
        description: Option<String>,
        /// New team icon.
        #[arg(long)]
        icon: Option<String>,
        /// New team color (hex code).
        #[arg(long)]
        color: Option<String>,
        /// New team timezone.
        #[arg(long)]
        timezone: Option<String>,
        /// Whether the team is private.
        #[arg(long)]
        private: Option<bool>,
        /// Whether cycles are enabled.
        #[arg(long)]
        cycles_enabled: Option<bool>,
        /// Whether triage mode is enabled.
        #[arg(long)]
        triage_enabled: Option<bool>,
    },
    /// Delete a team.
    ///
    /// Examples:
    ///   lineark teams delete ENG
    ///   lineark teams delete TEAM-UUID
    Delete {
        /// Team key, name, or UUID.
        id: String,
    },
    /// Manage team members.
    Members {
        #[command(subcommand)]
        action: MembersAction,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum MembersAction {
    /// Add a user to a team.
    ///
    /// Examples:
    ///   lineark teams members add ENG --user me
    ///   lineark teams members add "Backend" --user "alice"
    Add {
        /// Team key, name, or UUID.
        team: String,
        /// User name, display name, UUID, or `me`.
        #[arg(long)]
        user: String,
    },
    /// Remove a user from a team.
    ///
    /// Examples:
    ///   lineark teams members remove ENG --user me
    ///   lineark teams members remove "Backend" --user "alice"
    Remove {
        /// Team key, name, or UUID.
        team: String,
        /// User name, display name, UUID, or `me`.
        #[arg(long)]
        user: String,
    },
}

// ── List row ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
pub struct TeamRow {
    pub id: String,
    pub key: String,
    pub name: String,
}

// ── Lean types ───────────────────────────────────────────────────────────

/// Lean result type for team mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Team)]
#[serde(rename_all = "camelCase", default)]
struct TeamRef {
    id: Option<String>,
    key: Option<String>,
    name: Option<String>,
}

/// Full team detail for `teams read`.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Team)]
#[serde(rename_all = "camelCase", default)]
struct TeamDetail {
    id: Option<String>,
    key: Option<String>,
    name: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    private: Option<bool>,
    cycles_enabled: Option<bool>,
    triage_enabled: Option<bool>,
    timezone: Option<String>,
    #[graphql(nested)]
    members: Option<TeamMembersConnection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = UserConnection)]
#[serde(rename_all = "camelCase", default)]
struct TeamMembersConnection {
    #[graphql(nested)]
    nodes: Vec<TeamMemberRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
struct TeamMemberRef {
    id: Option<String>,
    name: Option<String>,
    display_name: Option<String>,
}

/// Lean result type for team membership mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = TeamMembership)]
#[serde(rename_all = "camelCase", default)]
struct TeamMembershipRef {
    id: Option<String>,
}

// ── Command dispatch ────────────────────────────────────────────────────

pub async fn run(cmd: TeamsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        TeamsAction::List => {
            let conn = client
                .teams::<Team>()
                .first(250)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<TeamRow> = conn
                .nodes
                .iter()
                .map(|t| TeamRow {
                    id: t.id.clone().unwrap_or_default(),
                    key: t.key.clone().unwrap_or_default(),
                    name: t.name.clone().unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
        TeamsAction::Read { id } => {
            let team_id = resolve_team_id(client, &id).await?;
            let team = client
                .team::<TeamDetail>(team_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&team, format);
        }
        TeamsAction::Create {
            name,
            key,
            description,
            icon,
            color,
            timezone,
            private,
            cycles_enabled,
            triage_enabled,
        } => {
            let input = TeamCreateInput {
                name: Some(name),
                key,
                description,
                icon,
                color,
                timezone,
                private,
                cycles_enabled,
                triage_enabled,
                ..Default::default()
            };

            let team = client
                .team_create::<TeamRef>(None, input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&team, format);
        }
        TeamsAction::Update {
            id,
            name,
            key,
            description,
            icon,
            color,
            timezone,
            private,
            cycles_enabled,
            triage_enabled,
        } => {
            if name.is_none()
                && key.is_none()
                && description.is_none()
                && icon.is_none()
                && color.is_none()
                && timezone.is_none()
                && private.is_none()
                && cycles_enabled.is_none()
                && triage_enabled.is_none()
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --key, --description, --icon, --color, --timezone, --private, --cycles-enabled, or --triage-enabled."
                ));
            }

            let team_id = resolve_team_id(client, &id).await?;

            let input = TeamUpdateInput {
                name,
                key,
                description,
                icon,
                color,
                timezone,
                private,
                cycles_enabled,
                triage_enabled,
                ..Default::default()
            };

            let team = client
                .team_update::<TeamRef>(None, input, team_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&team, format);
        }
        TeamsAction::Delete { id } => {
            let team_id = resolve_team_id(client, &id).await?;

            let result = client
                .team_delete(team_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
        TeamsAction::Members { action } => match action {
            MembersAction::Add { team, user } => {
                let team_id = resolve_team_id(client, &team).await?;
                let user_id = resolve_user_id_or_me(client, &user).await?;

                let input = TeamMembershipCreateInput {
                    team_id: Some(team_id),
                    user_id: Some(user_id),
                    ..Default::default()
                };

                let membership = client
                    .team_membership_create::<TeamMembershipRef>(input)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                output::print_one(&membership, format);
            }
            MembersAction::Remove { team, user } => {
                let team_id = resolve_team_id(client, &team).await?;
                let user_id = resolve_user_id_or_me(client, &user).await?;

                // Fetch team members to find the membership ID for this user.
                let team_detail = client
                    .team::<TeamDetail>(team_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let empty = Vec::new();
                let member_nodes = team_detail
                    .members
                    .as_ref()
                    .map(|m| &m.nodes)
                    .unwrap_or(&empty);
                let found_member = member_nodes
                    .iter()
                    .any(|m| m.id.as_deref() == Some(&user_id));

                if !found_member {
                    return Err(anyhow::anyhow!("User is not a member of this team"));
                }

                // Look up the TeamMembership entity via the team's memberships query.
                // The team detail gives us User nodes, but we need the TeamMembership ID.
                // We need to query team memberships directly to find the one for this user.
                let team_with_memberships = client
                    .team::<TeamMembershipLookup>(team_detail.id.clone().unwrap_or_default())
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                let membership_id = team_with_memberships
                    .memberships
                    .as_ref()
                    .and_then(|m| {
                        m.nodes.iter().find(|tm| {
                            tm.user
                                .as_ref()
                                .and_then(|u| u.id.as_deref())
                                .is_some_and(|uid| uid == user_id)
                        })
                    })
                    .and_then(|tm| tm.id.clone())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Could not find team membership for this user")
                    })?;

                let result = client
                    .team_membership_delete(None, membership_id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                output::print_one(&result, format);
            }
        },
    }
    Ok(())
}

// ── Internal types for membership lookup ────────────────────────────────

/// Used internally to look up TeamMembership IDs for a team.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Team)]
#[serde(rename_all = "camelCase", default)]
struct TeamMembershipLookup {
    id: Option<String>,
    #[graphql(nested)]
    memberships: Option<TeamMembershipsConnection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::TeamMembershipConnection)]
#[serde(rename_all = "camelCase", default)]
struct TeamMembershipsConnection {
    #[graphql(nested)]
    nodes: Vec<TeamMembershipNode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = TeamMembership)]
#[serde(rename_all = "camelCase", default)]
struct TeamMembershipNode {
    id: Option<String>,
    #[graphql(nested)]
    user: Option<TeamMembershipUserRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
struct TeamMembershipUserRef {
    id: Option<String>,
}
