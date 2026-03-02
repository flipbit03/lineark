use chrono::{NaiveDate, TimeZone, Utc};
use clap::Args;
use lineark_sdk::generated::inputs::{CycleCreateInput, CycleFilter, CycleUpdateInput};
use lineark_sdk::generated::types::Cycle;
use lineark_sdk::{Client, GraphQLFields};
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
#[allow(clippy::large_enum_variant)]
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
    /// Create a new cycle.
    ///
    /// Examples:
    ///   lineark cycles create --team ENG --starts-at 2026-03-10 --ends-at 2026-03-24
    ///   lineark cycles create --team ENG --starts-at 2026-04-01 --ends-at 2026-04-14 --name "Sprint 5"
    Create {
        /// Team key, name, or UUID.
        #[arg(long)]
        team: String,
        /// Start date (YYYY-MM-DD).
        #[arg(long)]
        starts_at: String,
        /// End date (YYYY-MM-DD).
        #[arg(long)]
        ends_at: String,
        /// Custom cycle name.
        #[arg(long)]
        name: Option<String>,
        /// Cycle description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Update an existing cycle.
    ///
    /// Examples:
    ///   lineark cycles update CYCLE-UUID --name "Sprint 5 (revised)"
    ///   lineark cycles update CYCLE-UUID --starts-at 2026-03-11 --ends-at 2026-03-25
    Update {
        /// Cycle UUID.
        id: String,
        /// New cycle name.
        #[arg(long)]
        name: Option<String>,
        /// New cycle description.
        #[arg(long)]
        description: Option<String>,
        /// New start date (YYYY-MM-DD).
        #[arg(long)]
        starts_at: Option<String>,
        /// New end date (YYYY-MM-DD).
        #[arg(long)]
        ends_at: Option<String>,
    },
    /// Archive a cycle.
    ///
    /// Examples:
    ///   lineark cycles archive CYCLE-UUID
    Archive {
        /// Cycle UUID.
        id: String,
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

/// Lean result type for cycle mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Cycle)]
#[serde(rename_all = "camelCase", default)]
struct CycleRef {
    id: Option<String>,
    name: Option<String>,
    number: Option<f64>,
    starts_at: Option<String>,
    ends_at: Option<String>,
}

fn cycle_status_label(cycle: &Cycle) -> String {
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

/// Parse a YYYY-MM-DD string into a `DateTime<Utc>` at midnight UTC.
fn parse_date_to_utc(s: &str, field_name: &str) -> anyhow::Result<chrono::DateTime<Utc>> {
    let date = s
        .parse::<NaiveDate>()
        .map_err(|e| anyhow::anyhow!("Invalid {} (expected YYYY-MM-DD): {}", field_name, e))?;
    let dt = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid {} datetime", field_name))?;
    Ok(Utc.from_utc_datetime(&dt))
}

pub async fn run(cmd: CyclesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        CyclesAction::List {
            limit,
            team,
            active,
            around_active,
        } => {
            let mut filter_map = serde_json::Map::new();

            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                filter_map.insert(
                    "team".into(),
                    serde_json::json!({ "id": { "eq": team_id } }),
                );
            }

            if active {
                filter_map.insert("isActive".into(), serde_json::json!({ "eq": true }));
            }

            let filter: CycleFilter = serde_json::from_value(serde_json::Value::Object(filter_map))
                .expect("valid CycleFilter");

            let conn = client
                .cycles::<Cycle>()
                .filter(filter)
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items: Vec<&Cycle> = if let Some(n) = around_active {
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
                            starts_at: c.starts_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                            ends_at: c.ends_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
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
                    .cycle::<Cycle>(id)
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

            let mut filter_map = serde_json::Map::new();
            filter_map.insert(
                "team".into(),
                serde_json::json!({ "id": { "eq": team_id } }),
            );

            // Try parsing as a number first.
            if let Ok(num) = id.parse::<i64>() {
                filter_map.insert("number".into(), serde_json::json!({ "eq": num }));
            } else {
                filter_map.insert("name".into(), serde_json::json!({ "eq": id }));
            }

            let filter: CycleFilter = serde_json::from_value(serde_json::Value::Object(filter_map))
                .expect("valid CycleFilter");

            let conn = client
                .cycles::<Cycle>()
                .filter(filter)
                .first(1)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let cycle = conn.nodes.into_iter().next().ok_or_else(|| {
                anyhow::anyhow!("Cycle '{}' not found in team '{}'", id, team_key)
            })?;

            // Re-fetch with full detail using the cycle's ID.
            let cycle_id = cycle.id.ok_or_else(|| anyhow::anyhow!("Cycle has no ID"))?;

            let full_cycle = client
                .cycle::<Cycle>(cycle_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&full_cycle, format);
        }
        CyclesAction::Create {
            team,
            starts_at,
            ends_at,
            name,
            description,
        } => {
            let team_id = resolve_team_id(client, &team).await?;
            let starts_at_dt = parse_date_to_utc(&starts_at, "starts-at")?;
            let ends_at_dt = parse_date_to_utc(&ends_at, "ends-at")?;

            let input = CycleCreateInput {
                team_id: Some(team_id),
                starts_at: Some(starts_at_dt),
                ends_at: Some(ends_at_dt),
                name,
                description,
                ..Default::default()
            };

            let cycle = client
                .cycle_create::<CycleRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&cycle, format);
        }
        CyclesAction::Update {
            id,
            name,
            description,
            starts_at,
            ends_at,
        } => {
            if name.is_none() && description.is_none() && starts_at.is_none() && ends_at.is_none() {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --description, --starts-at, or --ends-at."
                ));
            }

            let starts_at_dt = starts_at
                .map(|s| parse_date_to_utc(&s, "starts-at"))
                .transpose()?;
            let ends_at_dt = ends_at
                .map(|s| parse_date_to_utc(&s, "ends-at"))
                .transpose()?;

            let input = CycleUpdateInput {
                name,
                description,
                starts_at: starts_at_dt,
                ends_at: ends_at_dt,
                ..Default::default()
            };

            let cycle = client
                .cycle_update::<CycleRef>(input, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&cycle, format);
        }
        CyclesAction::Archive { id } => {
            let cycle = client
                .cycle_archive::<CycleRef>(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&cycle, format);
        }
    }
    Ok(())
}

/// Filter cycles to the active one ± N neighbors by cycle number.
fn around_active_filter(cycles: &[Cycle], n: i64) -> Vec<&Cycle> {
    let active_number = cycles
        .iter()
        .find(|c| c.is_active.unwrap_or(false))
        .and_then(|c| c.number);

    let Some(active_num) = active_number else {
        eprintln!("Warning: no active cycle found");
        return Vec::new();
    };

    let n = n as f64;
    let lo = active_num - n;
    let hi = active_num + n;

    cycles
        .iter()
        .filter(|c| c.number.is_some_and(|num| num >= lo && num <= hi))
        .collect()
}
