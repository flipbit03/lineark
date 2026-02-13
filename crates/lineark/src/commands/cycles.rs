use clap::Args;
use lineark_sdk::Client;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::resolve_team_id;
use crate::output::{self, Format};

/// Manage cycles.
#[derive(Debug, Args)]
pub struct CyclesCmd {
    #[command(subcommand)]
    pub action: CyclesAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum CyclesAction {
    /// List cycles. By default shows all cycles; use --active to filter.
    ///
    /// Examples:
    ///   lineark cycles list
    ///   lineark cycles list --active
    ///   lineark cycles list --team ENG --around-active 2
    List {
        /// Maximum number of cycles to return (max 250).
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Filter by team key (e.g., ENG) or UUID.
        #[arg(long)]
        team: Option<String>,
        /// Show only the currently active cycle.
        #[arg(long, default_value = "false")]
        active: bool,
        /// Show the active cycle plus N neighbors on each side (by number).
        /// Implies fetching from the relevant team.
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..=50), conflicts_with = "active")]
        around_active: Option<i64>,
    },
    /// Read a specific cycle by UUID or by name/number.
    ///
    /// Examples:
    ///   lineark cycles read CYCLE-UUID
    ///   lineark cycles read "Sprint 42" --team ENG
    Read {
        /// Cycle UUID or name/number.
        id: String,
        /// Team key (required when looking up by name/number).
        #[arg(long)]
        team: Option<String>,
    },
}

#[derive(Debug, Serialize, Tabled)]
pub struct CycleRow {
    pub id: String,
    pub number: String,
    pub name: String,
    pub starts_at: String,
    pub ends_at: String,
    pub active: String,
}

fn cycle_status_label(cycle: &CycleListItem) -> String {
    if cycle.is_active.unwrap_or(false) {
        "active".to_string()
    } else if cycle.is_next.unwrap_or(false) {
        "next".to_string()
    } else if cycle.is_previous.unwrap_or(false) {
        "previous".to_string()
    } else if cycle.is_future.unwrap_or(false) {
        "future".to_string()
    } else if cycle.is_past.unwrap_or(false) {
        "past".to_string()
    } else {
        String::new()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CycleListItem {
    id: Option<String>,
    number: Option<i64>,
    name: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    is_active: Option<bool>,
    is_next: Option<bool>,
    is_previous: Option<bool>,
    is_future: Option<bool>,
    is_past: Option<bool>,
}

const CYCLE_LIST_QUERY: &str =
    "query Cycles($first: Int, $filter: CycleFilter) { cycles(first: $first, filter: $filter) { \
nodes { id number name startsAt endsAt isActive isNext isPrevious isFuture isPast } \
pageInfo { hasNextPage endCursor } } }";

pub async fn run(cmd: CyclesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        CyclesAction::List {
            limit,
            team,
            active,
            around_active,
        } => {
            let mut filter = serde_json::Map::new();

            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                filter.insert(
                    "team".into(),
                    serde_json::json!({ "id": { "eq": team_id } }),
                );
            }

            if active {
                filter.insert("isActive".into(), serde_json::json!({ "eq": true }));
            }

            let variables = serde_json::json!({
                "first": limit,
                "filter": filter,
            });

            let conn = client
                .execute_connection::<CycleListItem>(CYCLE_LIST_QUERY, variables, "cycles")
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items: Vec<&CycleListItem> = if let Some(n) = around_active {
                // Find the active cycle and return it ± N neighbors by number.
                around_active_filter(&conn.nodes, n)
            } else {
                conn.nodes.iter().collect()
            };

            match format {
                Format::Json => {
                    let json = serde_json::to_string_pretty(&items).unwrap_or_default();
                    println!("{json}");
                }
                Format::Human => {
                    let rows: Vec<CycleRow> = items
                        .iter()
                        .map(|c| CycleRow {
                            id: c.id.clone().unwrap_or_default(),
                            number: c.number.map(|n| n.to_string()).unwrap_or_default(),
                            name: c.name.clone().unwrap_or_default(),
                            starts_at: c.starts_at.clone().unwrap_or_default(),
                            ends_at: c.ends_at.clone().unwrap_or_default(),
                            active: cycle_status_label(c),
                        })
                        .collect();
                    output::print_table(&rows, format);
                }
            }
        }
        CyclesAction::Read { id, team } => {
            // Try UUID first.
            if uuid::Uuid::parse_str(&id).is_ok() {
                let cycle = client
                    .cycle(id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                output::print_one(&cycle, format);
                return Ok(());
            }

            // Try resolving by name or number within a team.
            let team_key = team.as_deref().ok_or_else(|| {
                anyhow::anyhow!(
                    "Looking up cycles by name/number requires --team. \
                     Use a UUID to read without --team."
                )
            })?;

            let team_id = resolve_team_id(client, team_key).await?;

            let mut filter = serde_json::Map::new();
            filter.insert(
                "team".into(),
                serde_json::json!({ "id": { "eq": team_id } }),
            );

            // Try parsing as a number first.
            if let Ok(num) = id.parse::<i64>() {
                filter.insert("number".into(), serde_json::json!({ "eq": num }));
            } else {
                filter.insert("name".into(), serde_json::json!({ "eq": id }));
            }

            let variables = serde_json::json!({
                "first": 1,
                "filter": filter,
            });

            let conn = client
                .execute_connection::<serde_json::Value>(CYCLE_LIST_QUERY, variables, "cycles")
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let cycle = conn.nodes.into_iter().next().ok_or_else(|| {
                anyhow::anyhow!("Cycle '{}' not found in team '{}'", id, team_key)
            })?;

            // Re-fetch with full detail using the cycle's ID.
            let cycle_id = cycle
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Cycle has no ID"))?
                .to_string();

            let full_cycle = client
                .cycle(cycle_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&full_cycle, format);
        }
    }
    Ok(())
}

/// Filter cycles to the active one ± N neighbors by cycle number.
fn around_active_filter(cycles: &[CycleListItem], n: i64) -> Vec<&CycleListItem> {
    let active_number = cycles
        .iter()
        .find(|c| c.is_active.unwrap_or(false))
        .and_then(|c| c.number);

    let Some(active_num) = active_number else {
        eprintln!("Warning: no active cycle found");
        return Vec::new();
    };

    let lo = active_num - n;
    let hi = active_num + n;

    cycles
        .iter()
        .filter(|c| c.number.is_some_and(|num| num >= lo && num <= hi))
        .collect()
}
