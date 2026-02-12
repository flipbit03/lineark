//! CLI integration tests for lineark.
//!
//! Tests that hit the real Linear API are marked `#[ignore]` and require
//! `~/.linear_api_token_test`. Run them with:
//!   cargo test -p lineark -- --ignored
//!
//! Tests that don't need auth (usage, --help, error handling) always run.

use assert_cmd::Command;
use predicates::prelude::*;

const TEST_TOKEN_FILE: &str = ".linear_api_token_test";

/// Read the test API token from ~/.linear_api_token_test.
fn test_token() -> String {
    let path = home::home_dir()
        .expect("could not determine home directory")
        .join(TEST_TOKEN_FILE);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {}", path.display(), e))
        .trim()
        .to_string()
}

fn lineark() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("lineark").unwrap()
}

// ── Usage command (no auth required) ────────────────────────────────────────

#[test]
fn usage_prints_command_reference() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("lineark"))
        .stdout(predicate::str::contains("COMMANDS"))
        .stdout(predicate::str::contains("whoami"))
        .stdout(predicate::str::contains("teams"))
        .stdout(predicate::str::contains("issues"))
        .stdout(predicate::str::contains("AUTH"));
}

#[test]
fn usage_mentions_global_options() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("--api-token"))
        .stdout(predicate::str::contains("--format"));
}

// ── Help flags (no auth required) ───────────────────────────────────────────

#[test]
fn help_flag_shows_help() {
    lineark()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("lineark"))
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn version_flag_shows_version() {
    lineark()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("lineark"));
}

#[test]
fn teams_help_shows_subcommands() {
    lineark()
        .args(["teams", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

#[test]
fn issues_help_shows_subcommands() {
    lineark()
        .args(["issues", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("search"));
}

// ── Auth error handling (no token file needed — tests bad tokens) ───────────

#[test]
fn invalid_token_prints_error_and_exits_nonzero() {
    lineark()
        .args(["--api-token", "invalid_token_abc", "whoami"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

#[test]
fn empty_token_prints_error() {
    lineark()
        .args(["--api-token", "", "whoami"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error"));
}

// ── JSON output (real API — ignored by default) ─────────────────────────────

#[test]
#[ignore]
fn whoami_json_output_has_expected_fields() {
    let token = test_token();
    let output = lineark()
        .args(["--api-token", &token, "--format", "json", "whoami"])
        .output()
        .unwrap();

    assert!(output.status.success(), "whoami should succeed");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Should be an array with one element (print_table format).
    let arr = parsed.as_array().expect("JSON output should be an array");
    assert_eq!(arr.len(), 1);

    let user = &arr[0];
    assert!(user.get("id").is_some(), "should have id field");
    assert!(user.get("name").is_some(), "should have name field");
    assert!(user.get("email").is_some(), "should have email field");
    assert!(user.get("active").is_some(), "should have active field");
}

#[test]
#[ignore]
fn teams_list_json_output_is_array() {
    let token = test_token();
    let output = lineark()
        .args(["--api-token", &token, "--format", "json", "teams", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let arr = parsed.as_array().expect("teams JSON should be an array");
    assert!(!arr.is_empty(), "should have at least one team");

    let team = &arr[0];
    assert!(team.get("id").is_some());
    assert!(team.get("key").is_some());
    assert!(team.get("name").is_some());
}

#[test]
#[ignore]
fn users_list_json_output_is_array() {
    let token = test_token();
    let output = lineark()
        .args(["--api-token", &token, "--format", "json", "users", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let arr = parsed.as_array().expect("users JSON should be an array");
    assert!(!arr.is_empty(), "should have at least one user");

    let user = &arr[0];
    assert!(user.get("id").is_some());
    assert!(user.get("name").is_some());
    assert!(user.get("email").is_some());
    assert!(user.get("active").is_some());
}

#[test]
#[ignore]
fn labels_list_json_output_is_array() {
    let token = test_token();
    let output = lineark()
        .args(["--api-token", &token, "--format", "json", "labels", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array(), "labels JSON should be an array");
}

#[test]
#[ignore]
fn issues_list_json_output_is_array() {
    let token = test_token();
    let output = lineark()
        .args([
            "--api-token",
            &token,
            "--format",
            "json",
            "issues",
            "list",
            "--limit",
            "5",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array(), "issues JSON should be an array");
}

#[test]
#[ignore]
fn issues_search_json_output_is_array() {
    let token = test_token();
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
            "3",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array(), "search JSON should be an array");
}

// ── Human output (real API — ignored by default) ────────────────────────────

#[test]
#[ignore]
fn whoami_human_output_is_reasonable() {
    let token = test_token();
    let output = lineark()
        .args(["--api-token", &token, "--format", "human", "whoami"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    // Human output should have table headers.
    assert!(stdout.contains("id"), "human table should have id column");
    // Should contain at least one line with data (not just headers).
    assert!(
        stdout.lines().count() >= 3,
        "human table should have header, separator, and data rows"
    );
}

#[test]
#[ignore]
fn teams_human_output_has_table() {
    let token = test_token();
    let output = lineark()
        .args(["--api-token", &token, "--format", "human", "teams", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("id"));
    assert!(stdout.contains("key"));
    assert!(stdout.contains("name"));
}
