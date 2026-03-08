use lineark_sdk::generated::types::*;
use lineark_sdk::Client;

use crate::test_token;

/// Delete leftover `[test]`-prefixed resources from previous test runs (async impl).
async fn cleanup_zombies_impl() {
    let Ok(client) = Client::from_token(test_token()) else {
        return;
    };

    // Clean up zombie teams.
    if let Ok(conn) = client.teams::<Team>().first(250).send().await {
        for team in &conn.nodes {
            if let (Some(id), Some(name)) = (&team.id, &team.name) {
                if name.starts_with("[test]") {
                    eprintln!("cleanup_zombies: deleting team {name:?} ({id})");
                    let _ = client.team_delete(id.clone()).await;
                }
            }
        }
    }

    // Clean up zombie projects.
    if let Ok(conn) = client.projects::<Project>().first(250).send().await {
        for project in &conn.nodes {
            if let (Some(id), Some(name)) = (&project.id, &project.name) {
                if name.starts_with("[test]") {
                    eprintln!("cleanup_zombies: deleting project {name:?} ({id})");
                    let _ = client.project_delete::<Project>(id.clone()).await;
                }
            }
        }
    }

    // Clean up zombie issues.
    if let Ok(conn) = client.issues::<Issue>().first(250).send().await {
        for issue in &conn.nodes {
            if let (Some(id), Some(title)) = (&issue.id, &issue.title) {
                if title.starts_with("[test]") {
                    eprintln!("cleanup_zombies: deleting issue {title:?} ({id})");
                    let _ = client.issue_delete::<Issue>(Some(true), id.clone()).await;
                }
            }
        }
    }
}

/// Delete leftover `[test]`-prefixed resources from previous test runs.
/// Runs once per process via `std::sync::Once`. Best-effort, tolerates failures.
pub fn cleanup_zombies() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = std::thread::spawn(|| {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(cleanup_zombies_impl());
        })
        .join();
    });
}
