use lineark_sdk::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NestedUser {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NestedProject {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// Resolve a team key (e.g., "ENG") to a team UUID.
/// If the input already looks like a UUID, return it as-is.
pub async fn resolve_team_id(client: &Client, team_key: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(team_key).is_ok() {
        return Ok(team_key.to_string());
    }
    let conn = client
        .teams()
        .first(250)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    for team in &conn.nodes {
        if team
            .key
            .as_deref()
            .is_some_and(|k| k.eq_ignore_ascii_case(team_key))
        {
            return Ok(team.id.clone().unwrap_or_default());
        }
    }
    Err(anyhow::anyhow!("Team '{}' not found", team_key))
}

/// Resolve an issue identifier (e.g., ENG-123) to a UUID.
/// If the input already looks like a UUID, return it as-is.
pub async fn resolve_issue_id(client: &Client, identifier: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        return Ok(identifier.to_string());
    }
    let variables = serde_json::json!({ "term": identifier, "first": 5 });
    let conn: lineark_sdk::Connection<serde_json::Value> = client
        .execute_connection(
            "query IssueIdResolve($term: String!, $first: Int) { searchIssues(term: $term, first: $first) { nodes { id identifier } pageInfo { hasNextPage endCursor } } }",
            variables,
            "searchIssues",
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    conn.nodes
        .iter()
        .find(|n| {
            n.get("identifier")
                .and_then(|v| v.as_str())
                .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
        })
        .and_then(|n| n.get("id").and_then(|v| v.as_str()).map(String::from))
        .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))
}

/// Check the `success` field in a mutation payload.
pub fn check_success(payload: &serde_json::Value) -> anyhow::Result<()> {
    if payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
        return Err(anyhow::anyhow!(
            "Operation failed: {}",
            serde_json::to_string_pretty(payload).unwrap_or_default()
        ));
    }
    Ok(())
}
