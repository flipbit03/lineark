//! Shared test utilities for lineark online integration tests.
//!
//! Provides token loading, RAII guards for resource cleanup, retry helpers,
//! and team creation helpers used by all three online test suites.

use lineark_sdk::generated::types::*;
use lineark_sdk::Client;
use std::future::Future;

// ── Token loading ────────────────────────────────────────────────────────────

/// Returns `Some(reason)` if the test token file is missing, `None` if present.
/// Used with `test_with::runtime_ignore_if` to skip online tests gracefully.
pub fn no_online_test_token() -> Option<String> {
    let path = home::home_dir()?.join(".linear_api_token_test");
    if path.exists() {
        None
    } else {
        Some("~/.linear_api_token_test not found".to_string())
    }
}

/// Read the test API token from `~/.linear_api_token_test`.
pub fn test_token() -> String {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(".linear_api_token_test");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string()
}

// ── Settle ───────────────────────────────────────────────────────────────────

/// Wait for the Linear API to propagate recently created resources (async).
/// Linear is eventually consistent — created resources may not be queryable immediately.
pub async fn settle_async() {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}

/// Wait for the Linear API to propagate recently created resources (sync).
pub fn settle() {
    std::thread::sleep(std::time::Duration::from_secs(5));
}

// ── Retry helpers ────────────────────────────────────────────────────────────

/// Retry an async create operation up to 3 times with backoff on transient errors.
/// Retries on "conflict on insert" or "already exists" errors from the Linear API.
pub async fn retry_create_async<T, F, Fut>(mut f: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, lineark_sdk::LinearError>>,
{
    for attempt in 0..3u32 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_secs(1u64 << attempt)).await;
        }
        match f().await {
            Ok(val) => return val,
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("conflict on insert") && !msg.contains("already exists") {
                    panic!("create failed with non-transient error: {msg}");
                }
                if attempt == 2 {
                    panic!("create failed after 3 retries: {msg}");
                }
                eprintln!(
                    "retry_create: attempt {attempt} failed with transient error, retrying: {msg}"
                );
            }
        }
    }
    unreachable!()
}

/// Retry a blocking create operation up to 3 times with backoff on transient errors.
/// Retries on "conflict on insert" or "already exists" errors from the Linear API.
pub fn retry_create_sync<T, F>(mut f: F) -> T
where
    F: FnMut() -> Result<T, lineark_sdk::LinearError>,
{
    for attempt in 0..3u32 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_secs(1u64 << attempt));
        }
        match f() {
            Ok(val) => return val,
            Err(e) => {
                let msg = e.to_string();
                if !msg.contains("conflict on insert") && !msg.contains("already exists") {
                    panic!("create failed with non-transient error: {msg}");
                }
                if attempt == 2 {
                    panic!("create failed after 3 retries: {msg}");
                }
                eprintln!(
                    "retry_create: attempt {attempt} failed with transient error, retrying: {msg}"
                );
            }
        }
    }
    unreachable!()
}

/// Retry a search operation with generous backoff for Linear's eventually-consistent search index.
/// Returns `Some(result)` on the first attempt where `predicate` returns true, or `None` after exhausting retries.
pub async fn retry_search<T, F, Fut, P>(mut f: F, mut predicate: P) -> Option<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, lineark_sdk::LinearError>>,
    P: FnMut(&T) -> bool,
{
    for i in 0..12 {
        tokio::time::sleep(std::time::Duration::from_secs(if i < 3 { 2 } else { 5 })).await;
        let result = match f().await {
            Ok(v) => v,
            Err(_) => continue, // rate-limited or transient error — retry
        };
        if predicate(&result) {
            return Some(result);
        }
    }
    None
}

/// Retry a closure up to `max_attempts` times with exponential backoff.
/// Delays: 0s, 1s, 2s, 4s, 8s, 10s, 10s, ... (capped at 10s).
/// Returns `Ok(T)` on the first successful attempt, or `Err(last_error_message)`.
pub fn retry_with_backoff<T, F>(max_attempts: u32, mut f: F) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_err = String::new();
    for attempt in 0..max_attempts {
        let delay = if attempt == 0 {
            0
        } else {
            std::cmp::min(1u64 << (attempt - 1), 10)
        };
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_secs(delay));
        }
        match f() {
            Ok(val) => return Ok(val),
            Err(e) => last_err = e,
        }
    }
    Err(last_err)
}

// ── Cleanup zombies ──────────────────────────────────────────────────────────

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

// ── RAII Guards ──────────────────────────────────────────────────────────────

/// RAII guard — permanently deletes a team on drop.
/// Uses a dedicated thread+runtime since Drop can't be async.
pub struct TeamGuard {
    pub token: String,
    pub id: String,
}

impl Drop for TeamGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.team_delete(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes an issue on drop.
pub struct IssueGuard {
    pub token: String,
    pub id: String,
}

impl Drop for IssueGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.issue_delete::<Issue>(Some(true), id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes a document on drop.
pub struct DocumentGuard {
    pub token: String,
    pub id: String,
}

impl Drop for DocumentGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.document_delete::<Document>(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — permanently deletes a project on drop.
pub struct ProjectGuard {
    pub token: String,
    pub id: String,
}

impl Drop for ProjectGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.project_delete::<Project>(id).await;
                }
            });
        })
        .join();
    }
}

/// RAII guard — deletes an issue label on drop.
pub struct LabelGuard {
    pub token: String,
    pub id: String,
}

impl Drop for LabelGuard {
    fn drop(&mut self) {
        let token = self.token.clone();
        let id = self.id.clone();
        let _ = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                if let Ok(client) = Client::from_token(token) {
                    let _ = client.issue_label_delete(id).await;
                }
            });
        })
        .join();
    }
}

// ── Team creation helpers ────────────────────────────────────────────────────

/// Result of creating a test team, with all info needed by both SDK and CLI tests.
pub struct TestTeam {
    pub id: String,
    pub key: String,
    pub guard: TeamGuard,
}

/// Create a fresh test team (async). Runs cleanup_zombies first, uses retry_create,
/// settles after creation. Returns full team info including key.
pub async fn create_test_team_async(client: &Client) -> TestTeam {
    use lineark_sdk::generated::inputs::TeamCreateInput;
    cleanup_zombies();
    let suffix = &uuid::Uuid::new_v4().to_string()[..8];
    let unique = format!("[test] sdk {suffix}");
    let key = format!("T{}", &suffix[..5]).to_uppercase();
    let input = TeamCreateInput {
        name: Some(unique),
        key: Some(key),
        ..Default::default()
    };
    let team = retry_create_async(|| {
        let input = input.clone();
        async { client.team_create::<Team>(None, input).await }
    })
    .await;
    let team_id = team.id.clone().unwrap();
    let team_key = team.key.clone().unwrap();
    let guard = TeamGuard {
        token: test_token(),
        id: team_id.clone(),
    };
    settle_async().await;
    TestTeam {
        id: team_id,
        key: team_key,
        guard,
    }
}

/// Create a fresh test team (blocking). Runs cleanup_zombies first, uses retry_create,
/// settles after creation. Returns full team info including key.
pub fn create_test_team_sync(client: &lineark_sdk::blocking_client::Client) -> TestTeam {
    use lineark_sdk::generated::inputs::TeamCreateInput;
    cleanup_zombies();
    let suffix = &uuid::Uuid::new_v4().to_string()[..8];
    let unique = format!("[test] blocking {suffix}");
    let key = format!("T{}", &suffix[..5]).to_uppercase();
    let input = TeamCreateInput {
        name: Some(unique),
        key: Some(key),
        ..Default::default()
    };
    let team = retry_create_sync(|| client.team_create::<Team>(None, input.clone()));
    let team_id = team.id.clone().unwrap();
    let team_key = team.key.clone().unwrap();
    let guard = TeamGuard {
        token: test_token(),
        id: team_id.clone(),
    };
    settle();
    TestTeam {
        id: team_id,
        key: team_key,
        guard,
    }
}

// ── Standalone delete helpers ────────────────────────────────────────────────

/// Delete a team by its UUID (sync, using a fresh tokio runtime).
pub fn delete_team(team_id: &str) {
    let client = Client::from_token(test_token()).unwrap();
    let id = team_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.team_delete(id).await.unwrap() });
}

/// Permanently delete an issue by its UUID (sync, using a fresh tokio runtime).
pub fn delete_issue(issue_id: &str) {
    let client = Client::from_token(test_token()).unwrap();
    let id = issue_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.issue_delete::<Issue>(Some(true), id).await.unwrap() });
}
