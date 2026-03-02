use clap::Args;
use lineark_sdk::generated::inputs::NotificationUpdateInput;
use lineark_sdk::generated::types::Notification;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use crate::output::{self, Format};

/// Manage notifications.
#[derive(Debug, Args)]
pub struct NotificationsCmd {
    #[command(subcommand)]
    pub action: NotificationsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum NotificationsAction {
    /// List notifications.
    ///
    /// Examples:
    ///   lineark notifications list
    ///   lineark notifications list --unread
    ///   lineark notifications list -l 10
    List {
        /// Maximum number of notifications to return (max 250).
        #[arg(short = 'l', long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Show only unread notifications.
        #[arg(long, default_value = "false")]
        unread: bool,
    },
    /// Show full details for a single notification.
    ///
    /// Examples:
    ///   lineark notifications read NOTIFICATION-UUID
    Read {
        /// Notification UUID.
        id: String,
    },
    /// Mark a notification as read.
    ///
    /// Examples:
    ///   lineark notifications mark-read NOTIFICATION-UUID
    MarkRead {
        /// Notification UUID.
        id: String,
    },
    /// Archive a notification.
    ///
    /// Examples:
    ///   lineark notifications archive NOTIFICATION-UUID
    Archive {
        /// Notification UUID.
        id: String,
    },
    /// Unarchive a notification.
    ///
    /// Examples:
    ///   lineark notifications unarchive NOTIFICATION-UUID
    Unarchive {
        /// Notification UUID.
        id: String,
    },
}

// ── Lean types ───────────────────────────────────────────────────────────────

/// Lean notification type for list views.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Notification)]
#[serde(rename_all = "camelCase", default)]
struct NotificationSummary {
    pub id: Option<String>,
    pub r#type: Option<String>,
    pub title: Option<String>,
    pub read_at: Option<String>,
    pub created_at: Option<String>,
    pub url: Option<String>,
}

// ── List row ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
struct NotificationRow {
    id: String,
    #[tabled(rename = "type")]
    r#type: String,
    title: String,
    read: String,
    created_at: String,
}

impl From<&NotificationSummary> for NotificationRow {
    fn from(n: &NotificationSummary) -> Self {
        Self {
            id: n.id.clone().unwrap_or_default(),
            r#type: n.r#type.clone().unwrap_or_default(),
            title: n.title.clone().unwrap_or_default(),
            read: if n.read_at.is_some() { "yes" } else { "no" }.to_string(),
            created_at: n.created_at.clone().unwrap_or_default(),
        }
    }
}

// ── Command dispatch ─────────────────────────────────────────────────────────

pub async fn run(cmd: NotificationsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        NotificationsAction::List { limit, unread } => {
            let conn = client
                .notifications::<NotificationSummary>()
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let nodes: Vec<&NotificationSummary> = if unread {
                conn.nodes.iter().filter(|n| n.read_at.is_none()).collect()
            } else {
                conn.nodes.iter().collect()
            };

            match format {
                Format::Json => {
                    let json = serde_json::to_string_pretty(&nodes).unwrap_or_default();
                    println!("{json}");
                }
                Format::Human => {
                    let rows: Vec<NotificationRow> =
                        nodes.iter().map(|n| NotificationRow::from(*n)).collect();
                    output::print_table(&rows, format);
                }
            }
        }
        NotificationsAction::Read { id } => {
            let notification = client
                .notification::<Notification>(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&notification, format);
        }
        NotificationsAction::MarkRead { id } => {
            let input = NotificationUpdateInput {
                read_at: Some(chrono::Utc::now()),
                ..Default::default()
            };

            let result = client
                .notification_update(input, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
        NotificationsAction::Archive { id } => {
            let result = client
                .notification_archive(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
        NotificationsAction::Unarchive { id } => {
            let result = client
                .notification_unarchive(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
    }
    Ok(())
}
