use lineark_sdk::generated::types::{
    Cycle, Initiative, IssueLabel, IssueSearchResult, Project, Team, User,
};
use lineark_sdk::Client;

/// Resolve a team key or name (e.g., "ENG" or "Engineering") to a team UUID.
/// If the input already looks like a UUID, return it as-is.
pub async fn resolve_team_id(client: &Client, team_key: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(team_key).is_ok() {
        return Ok(team_key.to_string());
    }
    let conn = client
        .teams::<Team>()
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
        if team
            .name
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(team_key))
        {
            return Ok(team.id.clone().unwrap_or_default());
        }
    }
    let available: Vec<String> = conn
        .nodes
        .iter()
        .map(|t| {
            let key = t.key.as_deref().unwrap_or("?");
            let name = t.name.as_deref().unwrap_or("?");
            format!("{} ({})", key, name)
        })
        .collect();
    Err(anyhow::anyhow!(
        "Team '{}' not found. Available: {}",
        team_key,
        available.join(", ")
    ))
}

/// Resolve multiple team keys, names, or UUIDs to team UUIDs.
/// Each item follows the same rules as `resolve_team_id`.
pub async fn resolve_team_ids(
    client: &Client,
    team_keys: &[String],
) -> anyhow::Result<Vec<String>> {
    // Fast path: all items are already UUIDs.
    let all_uuids = team_keys.iter().all(|k| uuid::Uuid::parse_str(k).is_ok());
    if all_uuids {
        return Ok(team_keys.to_vec());
    }

    // Fetch teams once.
    let conn = client
        .teams::<Team>()
        .first(250)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut resolved = Vec::with_capacity(team_keys.len());
    for key in team_keys {
        if uuid::Uuid::parse_str(key).is_ok() {
            resolved.push(key.clone());
            continue;
        }
        let found = conn.nodes.iter().find(|t| {
            t.key
                .as_deref()
                .is_some_and(|k| k.eq_ignore_ascii_case(key))
                || t.name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(key))
        });
        match found {
            Some(team) => resolved.push(team.id.clone().unwrap_or_default()),
            None => {
                let available: Vec<String> = conn
                    .nodes
                    .iter()
                    .map(|t| {
                        let k = t.key.as_deref().unwrap_or("?");
                        let n = t.name.as_deref().unwrap_or("?");
                        format!("{} ({})", k, n)
                    })
                    .collect();
                return Err(anyhow::anyhow!(
                    "Team '{}' not found. Available: {}",
                    key,
                    available.join(", ")
                ));
            }
        }
    }
    Ok(resolved)
}

/// Resolve an issue identifier (e.g., ENG-123) to a UUID.
/// If the input already looks like a UUID, return it as-is.
pub async fn resolve_issue_id(client: &Client, identifier: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        return Ok(identifier.to_string());
    }
    let conn = client
        .search_issues::<IssueSearchResult>(identifier)
        .first(5)
        .include_archived(true)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    conn.nodes
        .iter()
        .find(|n| {
            n.identifier
                .as_deref()
                .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
        })
        .and_then(|n| n.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))
}

/// Resolve a user name, display name, UUID, or the special alias `me` to a user UUID.
/// `me` (case-insensitive) resolves to the authenticated user via `whoami`.
/// For all other values, delegates to [`resolve_user_id`].
pub async fn resolve_user_id_or_me(client: &Client, name_or_id: &str) -> anyhow::Result<String> {
    if name_or_id.eq_ignore_ascii_case("me") {
        let viewer = client
            .whoami::<User>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to resolve 'me': {}", e))?;
        return viewer
            .id
            .ok_or_else(|| anyhow::anyhow!("Could not determine authenticated user ID"));
    }
    resolve_user_id(client, name_or_id).await
}

/// Resolve multiple user names, display names, UUIDs, or `me` aliases to user UUIDs.
/// Fetches `whoami` at most once even if `me` appears multiple times.
pub async fn resolve_user_ids_or_me(
    client: &Client,
    names_or_ids: &[String],
) -> anyhow::Result<Vec<String>> {
    // Check if any item is "me" — resolve once.
    let has_me = names_or_ids.iter().any(|s| s.eq_ignore_ascii_case("me"));
    let me_id = if has_me {
        let viewer = client
            .whoami::<User>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to resolve 'me': {}", e))?;
        Some(
            viewer
                .id
                .ok_or_else(|| anyhow::anyhow!("Could not determine authenticated user ID"))?,
        )
    } else {
        None
    };

    let mut resolved = Vec::with_capacity(names_or_ids.len());
    for item in names_or_ids {
        if item.eq_ignore_ascii_case("me") {
            resolved.push(me_id.clone().unwrap());
        } else {
            resolved.push(resolve_user_id(client, item).await?);
        }
    }
    Ok(resolved)
}

/// Resolve a user name, display name, or UUID to a user UUID.
/// If the input already looks like a UUID, return it as-is.
/// Matches case-insensitively on `name` or `display_name`.
pub async fn resolve_user_id(client: &Client, name_or_id: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(name_or_id).is_ok() {
        return Ok(name_or_id.to_string());
    }
    let conn = client
        .users::<User>()
        .last(250)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let matches: Vec<&User> = conn
        .nodes
        .iter()
        .filter(|u| {
            u.name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(name_or_id))
                || u.display_name
                    .as_deref()
                    .is_some_and(|d| d.eq_ignore_ascii_case(name_or_id))
        })
        .collect();

    match matches.len() {
        0 => Err(anyhow::anyhow!("User '{}' not found", name_or_id)),
        1 => Ok(matches[0].id.clone().unwrap_or_default()),
        _ => {
            let names: Vec<String> = matches
                .iter()
                .map(|u| {
                    let name = u.name.as_deref().unwrap_or("?");
                    let display = u.display_name.as_deref().unwrap_or("?");
                    format!("{} ({})", name, display)
                })
                .collect();
            Err(anyhow::anyhow!(
                "Ambiguous user '{}'. Matches: {}",
                name_or_id,
                names.join(", ")
            ))
        }
    }
}

/// Resolve label names or UUIDs to a vec of label UUIDs.
/// For each item: if UUID, keep as-is. Otherwise fetch labels (optionally filtered by team)
/// and match case-insensitively on `name`.
pub async fn resolve_label_ids(
    client: &Client,
    names_or_ids: &[String],
    team_id: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    // Check if all items are already UUIDs — skip the API call.
    let all_uuids = names_or_ids
        .iter()
        .all(|s| uuid::Uuid::parse_str(s).is_ok());
    if all_uuids {
        return Ok(names_or_ids.to_vec());
    }

    // Fetch labels, optionally filtered by team.
    let mut builder = client.issue_labels::<IssueLabel>().first(250);
    if let Some(tid) = team_id {
        let filter: lineark_sdk::generated::inputs::IssueLabelFilter =
            serde_json::from_value(serde_json::json!({ "team": { "id": { "eq": tid } } }))
                .expect("valid IssueLabelFilter");
        builder = builder.filter(filter);
    }
    let conn = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut resolved = Vec::with_capacity(names_or_ids.len());
    for item in names_or_ids {
        if uuid::Uuid::parse_str(item).is_ok() {
            resolved.push(item.clone());
        } else {
            let found = conn.nodes.iter().find(|l| {
                l.name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(item))
            });
            match found {
                Some(label) => resolved.push(label.id.clone().unwrap_or_default()),
                None => {
                    let available: Vec<String> =
                        conn.nodes.iter().filter_map(|l| l.name.clone()).collect();
                    return Err(anyhow::anyhow!(
                        "Label '{}' not found. Available: {}",
                        item,
                        available.join(", ")
                    ));
                }
            }
        }
    }
    Ok(resolved)
}

/// Resolve a project name or UUID to a project UUID.
/// If the input already looks like a UUID, return it as-is.
pub async fn resolve_project_id(client: &Client, name_or_id: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(name_or_id).is_ok() {
        return Ok(name_or_id.to_string());
    }
    let conn = client
        .projects::<Project>()
        .first(250)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let matches: Vec<&Project> = conn
        .nodes
        .iter()
        .filter(|p| {
            p.name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(name_or_id))
        })
        .collect();

    match matches.len() {
        0 => {
            let available: Vec<String> = conn.nodes.iter().filter_map(|p| p.name.clone()).collect();
            Err(anyhow::anyhow!(
                "Project '{}' not found. Available: {}",
                name_or_id,
                available.join(", ")
            ))
        }
        1 => Ok(matches[0].id.clone().unwrap_or_default()),
        _ => {
            let names: Vec<String> = matches.iter().filter_map(|p| p.name.clone()).collect();
            Err(anyhow::anyhow!(
                "Ambiguous project '{}'. Matches: {}",
                name_or_id,
                names.join(", ")
            ))
        }
    }
}

/// Resolve a cycle name, number, or UUID to a cycle UUID.
/// If the input already looks like a UUID, return it as-is.
/// Matches case-insensitively on `name`, or parses as a number to match on `number`.
pub async fn resolve_cycle_id(
    client: &Client,
    name_or_id: &str,
    team_id: &str,
) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(name_or_id).is_ok() {
        return Ok(name_or_id.to_string());
    }

    let filter: lineark_sdk::generated::inputs::CycleFilter =
        serde_json::from_value(serde_json::json!({ "team": { "id": { "eq": team_id } } }))
            .expect("valid CycleFilter");

    let conn = client
        .cycles::<Cycle>()
        .filter(filter)
        .first(250)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Try matching by number first (e.g. "3" → cycle number 3).
    if let Ok(num) = name_or_id.parse::<f64>() {
        if let Some(cycle) = conn.nodes.iter().find(|c| c.number == Some(num)) {
            return Ok(cycle.id.clone().unwrap_or_default());
        }
    }

    // Try matching by name (case-insensitive).
    if let Some(cycle) = conn.nodes.iter().find(|c| {
        c.name
            .as_deref()
            .is_some_and(|n| n.eq_ignore_ascii_case(name_or_id))
    }) {
        return Ok(cycle.id.clone().unwrap_or_default());
    }

    let available: Vec<String> = conn
        .nodes
        .iter()
        .map(|c| {
            let num = c
                .number
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".into());
            let name = c.name.as_deref().unwrap_or("(unnamed)");
            format!("#{} {}", num, name)
        })
        .collect();
    Err(anyhow::anyhow!(
        "Cycle '{}' not found. Available: {}",
        name_or_id,
        available.join(", ")
    ))
}

/// Resolve an initiative name or UUID to an initiative UUID.
/// If the input already looks like a UUID, return it as-is.
/// Matches case-insensitively on `name`.
pub async fn resolve_initiative_id(client: &Client, name_or_id: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(name_or_id).is_ok() {
        return Ok(name_or_id.to_string());
    }
    let conn = client
        .initiatives::<Initiative>()
        .first(250)
        .include_archived(true)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let matches: Vec<&Initiative> = conn
        .nodes
        .iter()
        .filter(|i| {
            i.name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(name_or_id))
        })
        .collect();

    match matches.len() {
        0 => {
            let available: Vec<String> = conn.nodes.iter().filter_map(|i| i.name.clone()).collect();
            Err(anyhow::anyhow!(
                "Initiative '{}' not found. Available: {}",
                name_or_id,
                available.join(", ")
            ))
        }
        1 => Ok(matches[0].id.clone().unwrap_or_default()),
        _ => {
            let names: Vec<String> = matches.iter().filter_map(|i| i.name.clone()).collect();
            Err(anyhow::anyhow!(
                "Ambiguous initiative '{}'. Matches: {}",
                name_or_id,
                names.join(", ")
            ))
        }
    }
}
