//! Online CLI tests for lineark — these hit the real Linear API.
//!
//! Requires a valid Linear API token at `~/.linear_api_token_test`.
//! When the token file is missing, tests are automatically skipped with a message.

use assert_cmd::Command;
use lineark_sdk::generated::inputs::ProjectCreateInput;
use lineark_sdk::generated::types::{Issue, IssueRelation, Project};
use lineark_sdk::Client;
use predicates::prelude::*;

fn no_online_test_token() -> Option<String> {
    let path = home::home_dir()?.join(".linear_api_token_test");
    if path.exists() {
        None
    } else {
        Some("~/.linear_api_token_test not found".to_string())
    }
}

fn api_token() -> String {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(".linear_api_token_test");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string()
}

fn lineark() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("lineark").unwrap()
}

/// Delete a team by its UUID to keep the workspace clean.
fn delete_team(team_id: &str) {
    let client = Client::from_token(api_token()).unwrap();
    let id = team_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.team_delete(id).await.unwrap() });
}

/// Permanently delete an issue by its UUID to keep the workspace clean.
fn delete_issue(issue_id: &str) {
    let client = Client::from_token(api_token()).unwrap();
    let id = issue_id.to_string();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { client.issue_delete::<Issue>(Some(true), id).await.unwrap() });
}

/// Retry a closure up to `max_attempts` times with backoff.
/// Returns `Ok(T)` on the first successful attempt, or `Err(last_error_message)`.
fn retry_with_backoff<T, F>(max_attempts: u32, mut f: F) -> Result<T, String>
where
    F: FnMut() -> Result<T, String>,
{
    let mut last_err = String::new();
    for attempt in 0..max_attempts {
        let delay = if attempt == 0 {
            0
        } else if attempt < 3 {
            1
        } else {
            3
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

/// RAII guard — permanently deletes a team on drop.
/// Ensures cleanup even when the test panics mid-way.
struct TeamGuard {
    token: String,
    id: String,
}

impl Drop for TeamGuard {
    fn drop(&mut self) {
        let Ok(client) = Client::from_token(self.token.clone()) else {
            return;
        };
        let id = self.id.clone();
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { client.team_delete(id).await });
    }
}

/// RAII guard — permanently deletes an issue on drop.
struct IssueGuard {
    token: String,
    id: String,
}

impl Drop for IssueGuard {
    fn drop(&mut self) {
        let Ok(client) = Client::from_token(self.token.clone()) else {
            return;
        };
        let id = self.id.clone();
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { client.issue_delete::<Issue>(Some(true), id).await });
    }
}

/// RAII guard — permanently deletes a project on drop.
struct ProjectGuard {
    token: String,
    id: String,
}

impl Drop for ProjectGuard {
    fn drop(&mut self) {
        let Ok(client) = Client::from_token(self.token.clone()) else {
            return;
        };
        let id = self.id.clone();
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { client.project_delete::<Project>(id).await });
    }
}

/// RAII guard — deletes an issue label on drop.
struct LabelGuard {
    token: String,
    id: String,
}

impl Drop for LabelGuard {
    fn drop(&mut self) {
        let Ok(client) = Client::from_token(self.token.clone()) else {
            return;
        };
        let id = self.id.clone();
        let _ = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { client.issue_label_delete(id).await });
    }
}

test_with::runner!(online);

#[test_with::module]
mod online {
    // ── Whoami ────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn whoami_json_returns_valid_json() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "whoami"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "whoami should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_object(), "whoami JSON should be an object");
        assert!(json.get("id").is_some(), "whoami should contain id");
        assert!(json.get("email").is_some(), "whoami should contain email");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn whoami_human_shows_user_info() {
        let token = api_token();
        lineark()
            .args(["--api-token", &token, "--format", "human", "whoami"])
            .assert()
            .success()
            .stdout(predicate::str::contains("name"));
    }

    // ── Teams ─────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "teams list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "teams list JSON should be an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_list_human_shows_table() {
        let token = api_token();
        lineark()
            .args(["--api-token", &token, "--format", "human", "teams", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("name"));
    }

    // ── Users ─────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn users_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "users", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "users list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "users list JSON should be an array");
    }

    // ── Labels ────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn labels_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "labels", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "labels list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "labels list JSON should be an array");
    }

    /// Regression: labels list must include team field (#65)
    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn labels_list_json_includes_team_field() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "labels", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "labels list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");

        if let Some(label) = arr.first() {
            assert!(
                label.get("team").is_some(),
                "each label should include a 'team' field"
            );
            // team should be a string (team key or empty for workspace-wide)
            assert!(
                label["team"].is_string(),
                "team should be a string, got: {}",
                label["team"]
            );
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn labels_create_update_and_delete() {
        let token = api_token();

        // Create a workspace-level label.
        let unique_name = format!("[test] lbl-crud {}", &uuid::Uuid::new_v4().to_string()[..8]);
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "labels",
                "create",
                &unique_name,
                "--color",
                "#eb5757",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "labels create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let label_id = created["id"]
            .as_str()
            .expect("created label should have id")
            .to_string();
        let _label_guard = LabelGuard {
            token: token.clone(),
            id: label_id.clone(),
        };
        assert_eq!(created["name"].as_str(), Some(unique_name.as_str()));
        assert_eq!(created["color"].as_str(), Some("#eb5757"));

        // Read the label back.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "labels",
                "read",
                &label_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "labels read should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(detail["id"].as_str(), Some(label_id.as_str()));
        assert_eq!(detail["name"].as_str(), Some(unique_name.as_str()));

        // Update the label color.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "labels",
                "update",
                &label_id,
                "--color",
                "#4ea7fc",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "labels update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(updated["color"].as_str(), Some("#4ea7fc"));

        // Delete the label via CLI.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "labels",
                "delete",
                &label_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "labels delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
    }

    // ── Issues ────────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "issues", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json
            .as_array()
            .expect("issues list JSON should be an array");

        // Regression: relation fields must be populated, not null (#63)
        if let Some(issue) = arr.first() {
            assert!(
                issue.get("state").is_some_and(|v| !v.is_null()),
                "state should be populated"
            );
            assert!(
                issue.get("team").is_some_and(|v| !v.is_null()),
                "team should be populated"
            );
            // assignee can legitimately be null (unassigned issues), so just check the field exists
            assert!(
                issue.get("assignee").is_some(),
                "assignee field should be present"
            );
        }
    }

    /// Regression: issues list JSON must be flat — no nested objects (#65)
    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_list_json_is_flat() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "list",
                "--limit",
                "1",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");

        if let Some(issue) = arr.first() {
            // state, assignee, team must be flat strings, not nested objects
            assert!(
                issue["state"].is_string(),
                "state should be a flat string, got: {}",
                issue["state"]
            );
            assert!(
                issue["team"].is_string(),
                "team should be a flat string, got: {}",
                issue["team"]
            );
            // assignee can be "" for unassigned
            assert!(
                issue["assignee"].is_string(),
                "assignee should be a flat string, got: {}",
                issue["assignee"]
            );
            // id and priority (numeric) must be absent
            assert!(
                issue.get("id").is_none(),
                "id (UUID) should not be in list output"
            );
            assert!(
                issue.get("priority").is_none(),
                "priority (numeric) should not be in list output"
            );
            // url and priorityLabel must be present
            assert!(issue.get("url").is_some(), "url should be in list output");
            assert!(
                issue.get("priorityLabel").is_some(),
                "priorityLabel should be in list output"
            );
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_search_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "search",
                "test",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues search should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json
            .as_array()
            .expect("issues search JSON should be an array");

        // Regression: relation fields must be populated, not null (#63)
        if let Some(issue) = arr.first() {
            assert!(
                issue.get("state").is_some_and(|v| !v.is_null()),
                "state should be populated"
            );
            assert!(
                issue.get("team").is_some_and(|v| !v.is_null()),
                "team should be populated"
            );
            assert!(
                issue.get("assignee").is_some(),
                "assignee field should be present"
            );
        }
    }

    /// Regression: issues search JSON must be flat — same as list (#65)
    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_search_json_is_flat() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "search",
                "test",
                "--limit",
                "1",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "issues search should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");

        if let Some(issue) = arr.first() {
            assert!(
                issue["state"].is_string(),
                "state should be a flat string in search output"
            );
            assert!(
                issue["team"].is_string(),
                "team should be a flat string in search output"
            );
            assert!(
                issue.get("id").is_none(),
                "id (UUID) should not be in search output"
            );
            assert!(
                issue.get("priority").is_none(),
                "priority (numeric) should not be in search output"
            );
        }
    }

    // ── Issues create + update + archive ─────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_create_update_and_archive() {
        let token = api_token();

        // Get the first team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .expect("failed to execute lineark");
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI create+update",
                "--team",
                &team_key,
                "--priority",
                "4",
                "--description",
                "Automated CLI test — will be archived.",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)");
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.to_string(),
        };

        // Update the issue (use UUID to avoid search index lag).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "update",
                issue_id,
                "--priority",
                "2",
                "--title",
                "[test] CLI create+update — updated",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Mutation now returns only the entity with `id` (serde_json::Value
        // selects only `id`), so we just verify the command succeeded and
        // the output is valid JSON with an id.
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            updated.get("id").is_some(),
            "update response should contain id"
        );

        // Clean up: permanently delete the issue.
        delete_issue(issue_id);
    }

    // ── Issues archive / unarchive ─────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_archive_and_unarchive_cycle() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI archive/unarchive",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)")
            .to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };

        // Archive the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "archive",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues archive should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Mutation now returns the entity directly (no payload wrapper).
        // Success is checked internally by the SDK; the CLI just outputs the entity.
        let archived: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            archived.get("id").is_some(),
            "archive response should contain id"
        );

        // Read the issue and verify archivedAt is set (retry for eventual consistency).
        let token_r = token.clone();
        let issue_id_r = issue_id.clone();
        retry_with_backoff(5, move || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token_r,
                    "--format",
                    "json",
                    "issues",
                    "read",
                    &issue_id_r,
                ])
                .output()
                .unwrap();
            assert!(output.status.success());
            let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            if detail.get("archivedAt").and_then(|v| v.as_str()).is_some() {
                Ok(())
            } else {
                Err("archivedAt should be set after archiving".to_string())
            }
        })
        .expect("archivedAt should be set after archiving");

        // Unarchive the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "unarchive",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues unarchive should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let unarchived: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            unarchived.get("id").is_some(),
            "unarchive response should contain id"
        );

        // Read again and verify archivedAt is cleared (retry for eventual consistency).
        let token_r = token.clone();
        let issue_id_r = issue_id.clone();
        retry_with_backoff(5, move || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token_r,
                    "--format",
                    "json",
                    "issues",
                    "read",
                    &issue_id_r,
                ])
                .output()
                .unwrap();
            assert!(output.status.success());
            let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            if detail.get("archivedAt").unwrap().is_null() {
                Ok(())
            } else {
                Err("archivedAt should be null after unarchiving".to_string())
            }
        })
        .expect("archivedAt should be null after unarchiving");

        // Clean up: permanently delete.
        delete_issue(&issue_id);
    }

    // ── Issues unarchive by human identifier ───────────────────────────────────
    // Regression test: resolve_issue_id must include archived issues in search,
    // otherwise `lineark issues unarchive CAD-XXXX` fails with "not found".

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_unarchive_by_human_identifier() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] unarchive by identifier",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"].as_str().unwrap().to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };
        let identifier = created["identifier"]
            .as_str()
            .expect("created issue should have identifier (e.g. CAD-1234)")
            .to_string();

        // Archive the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "archive",
                &issue_id,
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "archive should succeed");

        // Unarchive using the HUMAN identifier (e.g. CAD-1234), not the UUID.
        // This is the regression case: search_issues must include_archived(true)
        // for resolve_issue_id to find archived issues.
        //
        // Linear's search index is async — the newly created+archived issue may
        // not be searchable immediately. Retry with backoff to avoid flakiness.
        let stdout = retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "issues",
                    "unarchive",
                    &identifier,
                ])
                .output()
                .expect("failed to execute lineark");
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if output.status.success() {
                Ok(stdout)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                Err(format!("stdout: {stdout}\nstderr: {stderr}"))
            }
        })
        .expect("unarchive by human identifier should succeed (after retries)");
        let unarchived: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            unarchived.get("id").is_some(),
            "unarchive response should contain id"
        );

        // Clean up.
        delete_issue(&issue_id);
    }

    // ── Issues delete ──────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_delete_permanently() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue to delete.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI issues delete",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)");
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.to_string(),
        };

        // Delete the issue permanently via CLI.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "delete",
                issue_id,
                "--permanently",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues delete --permanently should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Delete mutations may not return an entity (just success checked
        // internally). Verify the command succeeded above is sufficient.
        // If output is present, just verify it's valid JSON.
        if !stdout.trim().is_empty() {
            let _deleted: serde_json::Value = serde_json::from_str(&stdout)
                .expect("delete output should be valid JSON if present");
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_delete_trash_and_verify() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI issues trash",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)")
            .to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };

        // Delete without --permanently (trash).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "delete",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues delete (trash) should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Trash mutation returns the entity directly (no payload wrapper).
        if !stdout.trim().is_empty() {
            let _trashed: serde_json::Value = serde_json::from_str(&stdout)
                .expect("trash output should be valid JSON if present");
        }

        // Clean up: permanently delete the trashed issue.
        delete_issue(&issue_id);
    }

    // ── Documents ─────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn documents_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "list",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "documents list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "documents list JSON should be an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn documents_create_read_update_and_delete() {
        let token = api_token();

        // Get a team key first (documents require a parent like project/issue/team).
        // Create an issue to associate the document with.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] doc parent issue",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let issue: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = issue["id"].as_str().unwrap().to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };

        // Create a document associated with the issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "create",
                "--title",
                "[test] CLI documents CRUD",
                "--content",
                "Automated CLI test document.",
                "--issue",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let doc_id = created["id"]
            .as_str()
            .expect("created document should have id");

        // Read the document.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "read",
                doc_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents read should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let read_doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            read_doc["title"].as_str(),
            Some("[test] CLI documents CRUD")
        );

        // Update the document.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "update",
                doc_id,
                "--title",
                "[test] CLI documents CRUD — updated",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Mutation returns the entity with only `id` selected (serde_json::Value),
        // so we just verify the command succeeded and output has an id.
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            updated.get("id").is_some(),
            "document update response should contain id"
        );

        // Delete the document.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "documents",
                "delete",
                doc_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "documents delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Clean up the parent issue.
        delete_issue(&issue_id);
    }

    // ── Cycles ───────────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_json_returns_array() {
        let token = api_token();
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "cycles", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "cycles list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(json.is_array(), "cycles list JSON should be an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_active_json() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "list",
                "--active",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(
            output.status.success(),
            "cycles list --active should succeed"
        );
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        assert!(
            json.is_array(),
            "cycles list --active JSON should be an array"
        );
        // Active filter should return 0 or 1 cycle.
        let arr = json.as_array().unwrap();
        assert!(
            arr.len() <= 1,
            "cycles list --active should return at most 1 cycle, got {}",
            arr.len()
        );
    }

    // ── Cycles --team and --around-active ────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_with_team_filter() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "list",
                "--team",
                &team_key,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cycles list --team should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(json.is_array(), "cycles list --team should return an array");
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_list_around_active() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "list",
                "--team",
                &team_key,
                "--around-active",
                "1",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cycles list --around-active should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let arr = json.as_array().expect("should be an array");
        // --around-active 1 returns at most 3 cycles (active ± 1).
        assert!(
            arr.len() <= 3,
            "cycles list --around-active 1 should return at most 3 cycles, got {}",
            arr.len()
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn cycles_read_by_uuid() {
        let token = api_token();

        // Get a cycle UUID from the list.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "cycles", "list"])
            .output()
            .unwrap();
        let cycles: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let arr = cycles.as_array().unwrap();
        if arr.is_empty() {
            // No cycles in workspace — skip.
            return;
        }
        let cycle_id = arr[0]["id"].as_str().unwrap();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "cycles",
                "read",
                cycle_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "cycles read by UUID should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let read_cycle: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(read_cycle.get("id").is_some(), "cycle should have an id");
    }

    // ── Embeds upload + download ─────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn embeds_upload_and_download_round_trip() {
        let token = api_token();
        let dir = tempfile::tempdir().unwrap();

        // Create a temp file to upload.
        let upload_path = dir.path().join("test-upload.txt");
        let content = "lineark CLI embeds round-trip test content";
        std::fs::write(&upload_path, content).unwrap();

        // Upload.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "upload",
                upload_path.to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "embeds upload should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let upload_result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let asset_url = upload_result["assetUrl"]
            .as_str()
            .expect("upload result should have assetUrl");
        assert!(
            asset_url.starts_with("https://"),
            "asset URL should be HTTPS, got: {asset_url}"
        );

        // Download the uploaded file.
        let download_path = dir.path().join("downloaded.txt");
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "embeds download should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Verify the downloaded file exists and has content.
        assert!(download_path.exists(), "downloaded file should exist");
        let downloaded = std::fs::read_to_string(&download_path).unwrap();
        assert_eq!(
            downloaded, content,
            "downloaded content should match uploaded content"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn embeds_download_overwrite_flag() {
        let token = api_token();
        let dir = tempfile::tempdir().unwrap();

        // Create and upload a file.
        let upload_path = dir.path().join("overwrite-test.txt");
        std::fs::write(&upload_path, "overwrite test").unwrap();

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "upload",
                upload_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let upload_result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let asset_url = upload_result["assetUrl"].as_str().unwrap();

        // Download to a path.
        let download_path = dir.path().join("target.txt");
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "first download should succeed");

        // Try downloading again without --overwrite — should fail.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "download without --overwrite should fail when file exists"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("already exists"),
            "error should mention file already exists"
        );

        // Try with --overwrite — should succeed.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "embeds",
                "download",
                asset_url,
                "--output",
                download_path.to_str().unwrap(),
                "--overwrite",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "download with --overwrite should succeed"
        );
    }

    // ── Issues read with relations ──────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_read_shows_relations() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create two issues.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] relation parent",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let issue_a: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_a_id = issue_a["id"].as_str().unwrap().to_string();
        let _issue_a_guard = IssueGuard {
            token: token.clone(),
            id: issue_a_id.clone(),
        };

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] relation child",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let issue_b: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_b_id = issue_b["id"].as_str().unwrap().to_string();
        let _issue_b_guard = IssueGuard {
            token: token.clone(),
            id: issue_b_id.clone(),
        };

        // Create a relation between them via the SDK.
        {
            use lineark_sdk::generated::enums::IssueRelationType;
            use lineark_sdk::generated::inputs::IssueRelationCreateInput;
            let client = Client::from_token(api_token()).unwrap();
            let input = IssueRelationCreateInput {
                issue_id: Some(issue_a_id.clone()),
                related_issue_id: Some(issue_b_id.clone()),
                r#type: Some(IssueRelationType::Blocks),
                ..Default::default()
            };
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                client
                    .issue_relation_create::<IssueRelation>(None, input)
                    .await
                    .unwrap()
            });
        }

        // Read issue A via CLI — should show the relation.
        // Linear's API may take a moment to propagate relations, so retry.
        let issue_b_id_clone = issue_b_id.clone();
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "issues",
                    "read",
                    &issue_a_id,
                ])
                .output()
                .expect("failed to execute lineark");
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("issues read failed: {stdout}"));
            }
            let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let relations = detail
                .get("relations")
                .and_then(|r| r.get("nodes"))
                .and_then(|n| n.as_array());
            let Some(relations) = relations else {
                return Err("relations field missing".to_string());
            };
            if relations.is_empty() {
                return Err("relations is empty".to_string());
            }
            let has_our_relation = relations.iter().any(|r| {
                r.get("relatedIssue")
                    .and_then(|ri| ri.get("id"))
                    .and_then(|id| id.as_str())
                    == Some(&issue_b_id_clone)
            });
            if !has_our_relation {
                return Err("relation to issue B not found".to_string());
            }
            Ok(())
        })
        .expect("relations should contain the relation to issue B (after retries)");

        // Clean up.
        delete_issue(&issue_a_id);
        delete_issue(&issue_b_id);
    }

    // ── Issues read: sub-issues and comments ─────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_read_shows_children_and_comments() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create a parent issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] parent with children",
                "--team",
                &team_key,
                "-p",
                "4",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "parent issue creation should succeed"
        );
        let parent: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let parent_id = parent["id"].as_str().unwrap().to_string();
        let _parent_guard = IssueGuard {
            token: token.clone(),
            id: parent_id.clone(),
        };

        // Create a child issue with --parent.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] child issue",
                "--team",
                &team_key,
                "-p",
                "4",
                "--parent",
                &parent_id,
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child issue creation should succeed"
        );
        let child: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let child_id = child["id"].as_str().unwrap().to_string();
        let _child_guard = IssueGuard {
            token: token.clone(),
            id: child_id.clone(),
        };

        // Add a comment on the parent.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "comments",
                "create",
                &parent_id,
                "--body",
                "Test comment for children+comments test",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "comment creation should succeed");

        // Read the parent — should include children and comments.
        // Linear may take a moment to propagate children/comments, so retry.
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "issues",
                    "read",
                    &parent_id,
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("issues read failed: {stdout}"));
            }
            let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();

            let children = detail
                .get("children")
                .and_then(|c| c.get("nodes"))
                .and_then(|n| n.as_array());
            let Some(children) = children else {
                return Err("children field missing".to_string());
            };
            if children.is_empty() {
                return Err("children is empty".to_string());
            }

            let comments = detail
                .get("comments")
                .and_then(|c| c.get("nodes"))
                .and_then(|n| n.as_array());
            let Some(comments) = comments else {
                return Err("comments field missing".to_string());
            };
            if comments.is_empty() {
                return Err("comments is empty".to_string());
            }

            Ok(())
        })
        .expect("parent should have children and comments (after retries)");

        // Clean up.
        delete_issue(&child_id);
        delete_issue(&parent_id);
    }

    // ── Issues search with filters ──────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_search_with_team_filter() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Search with --team filter.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "search",
                "test",
                "--team",
                &team_key,
                "--limit",
                "5",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues search with --team should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let arr = json.as_array().expect("should be an array");
        // All results should belong to the specified team.
        for issue in arr {
            assert_eq!(
                issue["team"].as_str(),
                Some(team_key.as_str()),
                "all search results should belong to team {team_key}"
            );
        }
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_search_with_status_filter() {
        let token = api_token();

        // Search with --status filter.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "search",
                "test",
                "--status",
                "Backlog",
                "--limit",
                "5",
                "--show-done",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues search with --status should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let arr = json.as_array().expect("should be an array");
        for issue in arr {
            assert_eq!(
                issue["state"].as_str(),
                Some("Backlog"),
                "all search results should have status Backlog"
            );
        }
    }

    // ── Issues create/update with --clear-parent ────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_create_with_parent_and_clear_parent() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create parent.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] clear-parent parent",
                "--team",
                &team_key,
                "-p",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let parent: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let parent_id = parent["id"].as_str().unwrap().to_string();
        let _parent_guard = IssueGuard {
            token: token.clone(),
            id: parent_id.clone(),
        };

        // Create child with --parent.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] clear-parent child",
                "--team",
                &team_key,
                "-p",
                "4",
                "--parent",
                &parent_id,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let child: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let child_id = child["id"].as_str().unwrap().to_string();
        let _child_guard = IssueGuard {
            token: token.clone(),
            id: child_id.clone(),
        };

        // Update child with --clear-parent.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "update",
                &child_id,
                "--clear-parent",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues update --clear-parent should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Read parent and verify child is no longer in children.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "read",
                &parent_id,
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let children = detail["children"]["nodes"].as_array().unwrap();
        assert!(
            children.is_empty(),
            "after --clear-parent, parent should have no children"
        );

        // Clean up.
        delete_issue(&child_id);
        delete_issue(&parent_id);
    }

    // ── Project milestones CRUD ──────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn project_milestones_full_crud() {
        let token = api_token();

        // Get a team ID (projectCreate requires teamIds).
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_id = teams[0]["id"].as_str().unwrap().to_string();

        // Create a test project via the SDK.
        let client = Client::from_token(api_token()).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let project: Project = rt.block_on(async {
            let input = ProjectCreateInput {
                name: Some("[test] milestones CRUD project".to_string()),
                team_ids: Some(vec![team_id]),
                ..Default::default()
            };
            client.project_create::<Project>(None, input).await.unwrap()
        });
        let project_id = project.id.as_ref().unwrap().to_string();
        let _project_guard = ProjectGuard {
            token: token.clone(),
            id: project_id.clone(),
        };

        // Create a milestone via CLI (use project_id to avoid ambiguity with stale data).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "project-milestones",
                "create",
                "[test] Beta Release",
                "--project",
                &project_id,
                "--target-date",
                "2026-12-31",
                "--description",
                "Test milestone for CLI CRUD.",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "milestone create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let milestone_id = created["id"]
            .as_str()
            .expect("milestone should have id")
            .to_string();

        // List milestones.
        // Linear may take a moment to propagate the new milestone, so retry.
        let milestone_id_clone = milestone_id.clone();
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "project-milestones",
                    "list",
                    "--project",
                    &project_id,
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("milestone list failed: {stdout}"));
            }
            let milestones: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let arr = milestones.as_array().ok_or("not an array")?;
            if arr
                .iter()
                .any(|m| m["id"].as_str() == Some(&milestone_id_clone))
            {
                Ok(())
            } else {
                Err("created milestone not in list".to_string())
            }
        })
        .expect("list should include the created milestone (after retries)");

        // Read the milestone by name (uses name resolution).
        // Name resolution queries the same list, so also retry.
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "project-milestones",
                    "read",
                    "[test] Beta Release",
                    "--project",
                    &project_id,
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("milestone read by name failed: {stdout}"));
            }
            let read_ms: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            if read_ms["id"].as_str() == Some(milestone_id.as_str()) {
                Ok(())
            } else {
                Err("read returned wrong milestone".to_string())
            }
        })
        .expect("milestone read by name should return the created milestone (after retries)");

        // Update the milestone.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "project-milestones",
                "update",
                &milestone_id,
                "--name",
                "[test] GA Release",
                "--target-date",
                "2027-03-15",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "milestone update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let updated: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            updated["name"].as_str(),
            Some("[test] GA Release"),
            "updated name should match"
        );

        // Delete the milestone.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "project-milestones",
                "delete",
                &milestone_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "milestone delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Clean up: delete the test project.
        rt.block_on(async {
            client.project_delete::<Project>(project_id).await.unwrap();
        });
    }

    // ── Projects create ─────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn projects_create_and_delete() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create a project via CLI.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "create",
                "[test] CLI projects create",
                "--team",
                &team_key,
                "--description",
                "Automated CLI test project — will be deleted.",
                "--priority",
                "3",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "projects create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let project_id = created["id"]
            .as_str()
            .expect("created project should have id")
            .to_string();
        let _project_guard = ProjectGuard {
            token: token.clone(),
            id: project_id.clone(),
        };
        assert!(
            created.get("name").is_some(),
            "created project should have name"
        );
        assert!(
            created.get("slugId").is_some(),
            "created project should have slugId"
        );

        // List projects and verify the created one is present.
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "projects",
                    "list",
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("projects list failed: {stdout}"));
            }
            let projects: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let arr = projects.as_array().ok_or("not an array")?;
            if arr.iter().any(|p| p["id"].as_str() == Some(&project_id)) {
                Ok(())
            } else {
                Err("created project not in list".to_string())
            }
        })
        .expect("projects list should include the created project (after retries)");

        // Clean up: delete the test project via SDK.
        let client = Client::from_token(api_token()).unwrap();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            client.project_delete::<Project>(project_id).await.unwrap();
        });
    }

    // ── Projects list includes lead field ─────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn projects_list_json_includes_lead_field() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "list",
            ])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "projects list should succeed");
        let json: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("output should be valid JSON");
        let arr = json.as_array().expect("should be an array");
        // Every project row should have a "lead" field (string, possibly empty).
        for project in arr {
            assert!(
                project.get("lead").is_some(),
                "each project should have a 'lead' field"
            );
            assert!(
                project["lead"].is_string(),
                "lead should be a flat string, got: {}",
                project["lead"]
            );
        }
    }

    // ── Projects list --led-by-me ──────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn projects_list_led_by_me_returns_array() {
        let token = api_token();
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "list",
                "--led-by-me",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "projects list --led-by-me should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            json.is_array(),
            "projects list --led-by-me should return an array"
        );
    }

    // ── Projects read ──────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn projects_read_by_id_and_name() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create a project with --lead me.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "create",
                "[test] CLI projects read",
                "--team",
                &team_key,
                "--lead",
                "me",
                "--description",
                "Test project for read command.",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "projects create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let project_id = created["id"].as_str().unwrap().to_string();
        let _project_guard = ProjectGuard {
            token: token.clone(),
            id: project_id.clone(),
        };

        // Read by UUID.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "read",
                &project_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "projects read by UUID should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(detail["id"].as_str(), Some(project_id.as_str()));
        assert_eq!(detail["name"].as_str(), Some("[test] CLI projects read"));
        assert!(
            detail.get("lead").is_some() && !detail["lead"].is_null(),
            "read output should have a lead (set to me)"
        );
        assert!(
            detail.get("members").is_some(),
            "read output should have members field"
        );
        assert!(
            detail.get("teams").is_some(),
            "read output should have teams field"
        );
        assert!(
            detail.get("description").is_some(),
            "read output should have description field"
        );

        // Read by name (with retry for propagation).
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "projects",
                    "read",
                    "[test] CLI projects read",
                ])
                .output()
                .expect("failed to execute lineark");
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                return Err(format!(
                    "read by name failed.\nstdout: {stdout}\nstderr: {stderr}"
                ));
            }
            let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            if detail["id"].as_str() == Some(project_id.as_str()) {
                Ok(())
            } else {
                Err("read by name returned wrong project".to_string())
            }
        })
        .expect("projects read by name should resolve correctly (after retries)");

        // Clean up.
        let client = Client::from_token(api_token()).unwrap();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            client.project_delete::<Project>(project_id).await.unwrap();
        });
    }

    // ── Projects create with --members ──────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn projects_create_with_members_and_read_back() {
        let token = api_token();

        // Get a team key and find two users.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create a project with --lead me --members me.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "create",
                "[test] CLI members test",
                "--team",
                &team_key,
                "--lead",
                "me",
                "--members",
                "me",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "projects create with --members should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let project_id = created["id"].as_str().unwrap().to_string();
        let _project_guard = ProjectGuard {
            token: token.clone(),
            id: project_id.clone(),
        };

        // Get the authenticated user's info for comparison.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "whoami"])
            .output()
            .unwrap();
        let whoami: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let my_id = whoami["id"].as_str().unwrap();

        // Read the project back and verify members.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "projects",
                "read",
                &project_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "projects read should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();

        // Lead should be me.
        let lead = &detail["lead"];
        assert!(!lead.is_null(), "lead should not be null");
        assert_eq!(
            lead["id"].as_str(),
            Some(my_id),
            "lead should be the authenticated user"
        );

        // Members should include me.
        let members = detail["members"]["nodes"]
            .as_array()
            .expect("members.nodes should be an array");
        assert!(
            members.iter().any(|m| m["id"].as_str() == Some(my_id)),
            "members should include the authenticated user"
        );

        // Clean up.
        let client = Client::from_token(api_token()).unwrap();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            client.project_delete::<Project>(project_id).await.unwrap();
        });
    }

    // ── Issues create with --assignee me ───────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_create_with_assignee_me() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Get my user ID.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "whoami"])
            .output()
            .unwrap();
        let whoami: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let my_name = whoami["name"].as_str().unwrap_or("").to_string();

        // Create an issue with --assignee me.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI assignee me",
                "--team",
                &team_key,
                "--assignee",
                "me",
                "--priority",
                "4",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues create --assignee me should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let issue_id = created["id"].as_str().unwrap().to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };

        // Read the issue back and verify assignee.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "read",
                &issue_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let assignee_name = detail["assignee"]["name"].as_str().unwrap_or("");
        assert_eq!(
            assignee_name, my_name,
            "assignee should be the authenticated user"
        );

        // Clean up.
        delete_issue(&issue_id);
    }

    // ── Issues update with --assignee me ───────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn issues_update_with_assignee_me() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Get my user ID.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "whoami"])
            .output()
            .unwrap();
        let whoami: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let my_name = whoami["name"].as_str().unwrap_or("").to_string();

        // Create an unassigned issue.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI update assignee me",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success());
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"].as_str().unwrap().to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };

        // Update with --assignee me.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "update",
                &issue_id,
                "--assignee",
                "me",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "issues update --assignee me should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Read the issue back and verify assignee (retry for eventual consistency).
        let token_r = token.clone();
        let issue_id_r = issue_id.clone();
        let my_name_r = my_name.clone();
        retry_with_backoff(5, move || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token_r,
                    "--format",
                    "json",
                    "issues",
                    "read",
                    &issue_id_r,
                ])
                .output()
                .unwrap();
            let detail: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            let assignee_name = detail["assignee"]["name"].as_str().unwrap_or("");
            if assignee_name == my_name_r {
                Ok(())
            } else {
                Err(format!(
                    "after update, assignee should be '{}', got '{}'",
                    my_name_r, assignee_name
                ))
            }
        })
        .expect("after update, assignee should be the authenticated user");

        // Clean up.
        delete_issue(&issue_id);
    }

    // ── Comments create ──────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn comments_create_on_issue() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue to comment on.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI comments_create",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)");
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.to_string(),
        };

        // Create a comment (use UUID to avoid search index lag).
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "comments",
                "create",
                issue_id,
                "--body",
                "Automated CLI test comment.",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "comment create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let comment: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            comment.get("id").is_some(),
            "comment should have an id field"
        );

        // Clean up: permanently delete the issue.
        delete_issue(issue_id);
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn comments_create_and_delete() {
        let token = api_token();

        // Get a team key.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let team_key = teams[0]["key"].as_str().unwrap().to_string();

        // Create an issue to comment on.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] CLI comments_delete",
                "--team",
                &team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "issue creation should succeed");
        let created: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let issue_id = created["id"]
            .as_str()
            .expect("created issue should have id (UUID)")
            .to_string();
        let _issue_guard = IssueGuard {
            token: token.clone(),
            id: issue_id.clone(),
        };

        // Create a comment.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "comments",
                "create",
                &issue_id,
                "--body",
                "Comment that will be deleted.",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "comment create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let comment: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let comment_id = comment["id"]
            .as_str()
            .expect("comment should have an id")
            .to_string();

        // Delete the comment.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "comments",
                "delete",
                &comment_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "comments delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        // Verify the delete response indicates success.
        let delete_result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(
            delete_result["success"].as_bool(),
            Some(true),
            "delete response should have success: true"
        );

        // Verify the comment is gone from the issue.
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "issues",
                    "read",
                    &issue_id,
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("issues read failed: {stdout}"));
            }
            let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let comments = detail
                .get("comments")
                .and_then(|c| c.get("nodes"))
                .and_then(|n| n.as_array());
            let Some(comments) = comments else {
                return Err("comments field missing".to_string());
            };
            let has_deleted = comments
                .iter()
                .any(|c| c["id"].as_str() == Some(&comment_id));
            if has_deleted {
                Err("deleted comment still present".to_string())
            } else {
                Ok(())
            }
        })
        .expect("comment should be gone after deletion (after retries)");

        // Clean up: permanently delete the issue.
        delete_issue(&issue_id);
    }

    // ── Teams CRUD ──────────────────────────────────────────────────────────

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_create_and_delete() {
        let token = api_token();

        // Create a team via CLI.
        let unique_name = format!(
            "[test] tm-create {}",
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "create",
                &unique_name,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let team_id = created["id"]
            .as_str()
            .expect("created team should have id")
            .to_string();
        let _team_guard = TeamGuard {
            token: token.clone(),
            id: team_id.clone(),
        };
        assert!(
            created.get("name").is_some(),
            "created team should have name"
        );
        assert!(created.get("key").is_some(), "created team should have key");

        // Verify the team appears in the list.
        retry_with_backoff(8, || {
            let output = lineark()
                .args(["--api-token", &token, "--format", "json", "teams", "list"])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("teams list failed: {stdout}"));
            }
            let teams: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let arr = teams.as_array().ok_or("not an array")?;
            if arr.iter().any(|t| t["id"].as_str() == Some(&team_id)) {
                Ok(())
            } else {
                Err("created team not in list".to_string())
            }
        })
        .expect("teams list should include the created team (after retries)");

        // Clean up: delete the test team via SDK.
        delete_team(&team_id);
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_create_update_read_and_delete() {
        let token = api_token();

        // Create a team.
        let unique_name = format!("[test] tm-crud {}", &uuid::Uuid::new_v4().to_string()[..8]);
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "create",
                &unique_name,
                "--description",
                "Original description.",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let team_id = created["id"].as_str().unwrap().to_string();
        let _team_guard = TeamGuard {
            token: token.clone(),
            id: team_id.clone(),
        };

        // Update the team's description.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "update",
                &team_id,
                "--description",
                "Updated description.",
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams update should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Read the team back and verify.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "read",
                &team_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams read should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(detail["id"].as_str(), Some(team_id.as_str()));
        assert_eq!(detail["description"].as_str(), Some("Updated description."));
        assert!(
            detail.get("members").is_some(),
            "read output should have members field"
        );

        // Delete the team via CLI.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "delete",
                &team_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn teams_members_add_and_remove() {
        let token = api_token();

        // Create a team (the authenticated user becomes creator + auto-member).
        let unique_name = format!(
            "[test] tm-members {}",
            &uuid::Uuid::new_v4().to_string()[..8]
        );
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "create",
                &unique_name,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let created: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let team_id = created["id"].as_str().unwrap().to_string();
        let _team_guard = TeamGuard {
            token: token.clone(),
            id: team_id.clone(),
        };

        // Discover a different user to add as a member.
        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "whoami"])
            .output()
            .unwrap();
        let whoami: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let my_id = whoami["id"].as_str().unwrap().to_string();

        let output = lineark()
            .args(["--api-token", &token, "--format", "json", "users", "list"])
            .output()
            .expect("failed to execute lineark");
        assert!(output.status.success(), "users list should succeed");
        let users: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap();
        let other_user = users
            .iter()
            .find(|u| u["id"].as_str() != Some(&my_id))
            .expect("workspace must have at least two users to run this test");
        let other_user_id = other_user["id"].as_str().unwrap().to_string();

        // Add the other user as a member — must succeed cleanly.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "members",
                "add",
                &team_id,
                "--user",
                &other_user_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams members add should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let membership: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(
            membership.get("id").is_some(),
            "membership should have an id"
        );

        // Verify the other user appears in team members.
        retry_with_backoff(8, || {
            let output = lineark()
                .args([
                    "--api-token",
                    &token,
                    "--format",
                    "json",
                    "teams",
                    "read",
                    &team_id,
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                return Err(format!("teams read failed: {stdout}"));
            }
            let detail: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            let members = detail["members"]["nodes"]
                .as_array()
                .ok_or("members.nodes missing")?;
            if members
                .iter()
                .any(|m| m["id"].as_str() == Some(&other_user_id))
            {
                Ok(())
            } else {
                Err("other user not found in team members".to_string())
            }
        })
        .expect("team should contain the added member (after retries)");

        // Remove the other user.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "teams",
                "members",
                "remove",
                &team_id,
                "--user",
                &other_user_id,
            ])
            .output()
            .expect("failed to execute lineark");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "teams members remove should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );

        // Clean up: delete the team.
        delete_team(&team_id);
    }

    // ── Relations ────────────────────────────────────────────────────────────

    /// Helper: create two test issues, returning their UUIDs and guards.
    fn create_two_issues(
        token: &str,
        team_key: &str,
    ) -> ((String, IssueGuard), (String, IssueGuard)) {
        let out1 = lineark()
            .args([
                "--api-token",
                token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] relation issue A",
                "--team",
                team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(out1.status.success(), "issue A creation should succeed");
        let a: serde_json::Value = serde_json::from_slice(&out1.stdout).unwrap();
        let a_id = a["id"].as_str().unwrap().to_string();
        let a_guard = IssueGuard {
            token: token.to_string(),
            id: a_id.clone(),
        };

        let out2 = lineark()
            .args([
                "--api-token",
                token,
                "--format",
                "json",
                "issues",
                "create",
                "[test] relation issue B",
                "--team",
                team_key,
                "--priority",
                "4",
            ])
            .output()
            .unwrap();
        assert!(out2.status.success(), "issue B creation should succeed");
        let b: serde_json::Value = serde_json::from_slice(&out2.stdout).unwrap();
        let b_id = b["id"].as_str().unwrap().to_string();
        let b_guard = IssueGuard {
            token: token.to_string(),
            id: b_id.clone(),
        };

        ((a_id, a_guard), (b_id, b_guard))
    }

    /// Helper: get the first team key.
    fn first_team_key(token: &str) -> String {
        let output = lineark()
            .args(["--api-token", token, "--format", "json", "teams", "list"])
            .output()
            .unwrap();
        let teams: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        teams[0]["key"].as_str().unwrap().to_string()
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn relations_create_blocks() {
        let token = api_token();
        let team_key = first_team_key(&token);
        let ((a_id, _ga), (b_id, _gb)) = create_two_issues(&token, &team_key);

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "create",
                &a_id,
                "--blocks",
                &b_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations create --blocks should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let rel: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(rel.get("id").is_some(), "relation should have an id");
        assert_eq!(rel["type"].as_str(), Some("blocks"));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn relations_create_blocked_by() {
        let token = api_token();
        let team_key = first_team_key(&token);
        let ((a_id, _ga), (b_id, _gb)) = create_two_issues(&token, &team_key);

        // "A --blocked-by B" means B blocks A.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "create",
                &a_id,
                "--blocked-by",
                &b_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations create --blocked-by should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let rel: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(rel.get("id").is_some(), "relation should have an id");
        assert_eq!(rel["type"].as_str(), Some("blocks"));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn relations_create_related() {
        let token = api_token();
        let team_key = first_team_key(&token);
        let ((a_id, _ga), (b_id, _gb)) = create_two_issues(&token, &team_key);

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "create",
                &a_id,
                "--related",
                &b_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations create --related should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let rel: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(rel.get("id").is_some(), "relation should have an id");
        assert_eq!(rel["type"].as_str(), Some("related"));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn relations_create_duplicate() {
        let token = api_token();
        let team_key = first_team_key(&token);
        let ((a_id, _ga), (b_id, _gb)) = create_two_issues(&token, &team_key);

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "create",
                &a_id,
                "--duplicate",
                &b_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations create --duplicate should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let rel: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(rel.get("id").is_some(), "relation should have an id");
        assert_eq!(rel["type"].as_str(), Some("duplicate"));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn relations_create_similar() {
        let token = api_token();
        let team_key = first_team_key(&token);
        let ((a_id, _ga), (b_id, _gb)) = create_two_issues(&token, &team_key);

        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "create",
                &a_id,
                "--similar",
                &b_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations create --similar should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let rel: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert!(rel.get("id").is_some(), "relation should have an id");
        assert_eq!(rel["type"].as_str(), Some("similar"));
    }

    #[test_with::runtime_ignore_if(no_online_test_token)]
    fn relations_create_and_delete() {
        let token = api_token();
        let team_key = first_team_key(&token);
        let ((a_id, _ga), (b_id, _gb)) = create_two_issues(&token, &team_key);

        // Create a relation.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "create",
                &a_id,
                "--blocks",
                &b_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations create should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let rel: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        let rel_id = rel["id"]
            .as_str()
            .expect("relation should have id")
            .to_string();

        // Delete the relation.
        let output = lineark()
            .args([
                "--api-token",
                &token,
                "--format",
                "json",
                "relations",
                "delete",
                &rel_id,
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "relations delete should succeed.\nstdout: {stdout}\nstderr: {stderr}"
        );
        let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(result["success"].as_bool(), Some(true));
    }
}
