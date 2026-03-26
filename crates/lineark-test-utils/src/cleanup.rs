use lineark_sdk::generated::types::*;
use lineark_sdk::Client;

use crate::test_token;

/// Delete all test resources from the workspace.
///
/// The test workspace is a dedicated free-plan Linear workspace used exclusively
/// for CI. Free plans have hard resource limits (e.g. max 1 team), so we must
/// ensure a clean slate before tests run.
///
/// Teams are only deleted if `[test]`-prefixed (the default workspace team must
/// stay — Linear won't actually delete it and the free plan only allows one).
/// All other resource types are deleted unconditionally.
///
/// Order matters: issues before teams (issues belong to teams), documents before
/// projects, etc. Errors are logged but tolerated (best-effort).
pub async fn cleanup_workspace(client: &Client) {
    // Issues first — they belong to teams, and deleting a team may fail if
    // issues still reference it.
    if let Ok(conn) = client.issues::<Issue>().first(250).send().await {
        for issue in &conn.nodes {
            if let Some(id) = &issue.id {
                let title = issue.title.as_deref().unwrap_or("<untitled>");
                eprintln!("cleanup: deleting issue {title:?} ({id})");
                let _ = client.issue_delete::<Issue>(Some(true), id.clone()).await;
            }
        }
    }

    // Documents before projects.
    if let Ok(conn) = client.documents::<Document>().first(250).send().await {
        for doc in &conn.nodes {
            if let Some(id) = &doc.id {
                let title = doc.title.as_deref().unwrap_or("<untitled>");
                eprintln!("cleanup: deleting document {title:?} ({id})");
                let _ = client.document_delete::<Document>(id.clone()).await;
            }
        }
    }

    // Projects.
    if let Ok(conn) = client.projects::<Project>().first(250).send().await {
        for project in &conn.nodes {
            if let Some(id) = &project.id {
                let name = project.name.as_deref().unwrap_or("<unnamed>");
                eprintln!("cleanup: deleting project {name:?} ({id})");
                let _ = client.project_delete::<Project>(id.clone()).await;
            }
        }
    }

    // Issue labels (only custom ones — built-in labels will fail silently).
    if let Ok(conn) = client.issue_labels::<IssueLabel>().first(250).send().await {
        for label in &conn.nodes {
            if let Some(id) = &label.id {
                let name = label.name.as_deref().unwrap_or("<unnamed>");
                eprintln!("cleanup: deleting label {name:?} ({id})");
                let _ = client.issue_label_delete(id.clone()).await;
            }
        }
    }

    // Teams — only [test]-prefixed. The default workspace team must stay
    // (free plan allows max 1 team; Linear won't actually delete it).
    if let Ok(conn) = client.teams::<Team>().first(250).send().await {
        for team in &conn.nodes {
            if let (Some(id), Some(name)) = (&team.id, &team.name) {
                if name.starts_with("[test]") {
                    eprintln!("cleanup: deleting team {name:?} ({id})");
                    let _ = client.team_delete(id.clone()).await;
                }
            }
        }
    }
}

/// Delete all test resources from the workspace (sync wrapper).
/// Runs once per process via `std::sync::Once`. Best-effort, tolerates failures.
pub fn cleanup_zombies() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = std::thread::spawn(|| {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let Ok(client) = Client::from_token(test_token()) else {
                    return;
                };
                cleanup_workspace(&client).await;
            });
        })
        .join();
    });
}
