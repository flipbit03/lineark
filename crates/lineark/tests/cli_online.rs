//! Online CLI tests for lineark — these hit the real Linear API.
//!
//! Requires a valid Linear API token at `~/.linear_api_token_test`.
//! When the token file is missing, tests are automatically skipped with a message.

use assert_cmd::Command;
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

test_with::runner!(cli_online);

#[test_with::module]
mod cli_online {
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
        let arr = json.as_array().expect("whoami JSON should be an array");
        assert!(!arr.is_empty(), "whoami array should not be empty");
        assert!(arr[0].get("id").is_some(), "whoami entry should contain id");
        assert!(
            arr[0].get("email").is_some(),
            "whoami entry should contain email"
        );
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
        assert!(json.is_array(), "issues list JSON should be an array");
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
        assert!(json.is_array(), "issues search JSON should be an array");
    }
}
