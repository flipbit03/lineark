//! Offline CLI tests for lineark.
//!
//! These tests don't need any API token — they test usage, help, and error handling.
//! Online tests that hit the real Linear API live in `cli_online.rs`.

use assert_cmd::Command;
use predicates::prelude::*;

fn lineark() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("lineark").unwrap()
}

// ── Usage command ───────────────────────────────────────────────────────────

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

// ── Help flags ──────────────────────────────────────────────────────────────

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
        .stdout(predicate::str::contains("search"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"));
}

#[test]
fn issues_create_help_shows_flags() {
    lineark()
        .args(["issues", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"))
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--assignee"))
        .stdout(predicate::str::contains("--labels"));
}

#[test]
fn issues_update_help_shows_flags() {
    lineark()
        .args(["issues", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--status"))
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("--labels"))
        .stdout(predicate::str::contains("--label-by"))
        .stdout(predicate::str::contains("--clear-labels"))
        .stdout(predicate::str::contains("--assignee"))
        .stdout(predicate::str::contains("--parent"));
}

#[test]
fn comments_help_shows_subcommands() {
    lineark()
        .args(["comments", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create"));
}

#[test]
fn comments_create_help_shows_flags() {
    lineark()
        .args(["comments", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--body"))
        .stdout(predicate::str::contains("<ISSUE>"));
}

#[test]
fn usage_includes_write_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("issues create"))
        .stdout(predicate::str::contains("issues update"))
        .stdout(predicate::str::contains("comments create"));
}

// ── Documents ───────────────────────────────────────────────────────────────

#[test]
fn documents_help_shows_subcommands() {
    lineark()
        .args(["documents", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn documents_create_help_shows_flags() {
    lineark()
        .args(["documents", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--content"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--issue"));
}

#[test]
fn documents_update_help_shows_flags() {
    lineark()
        .args(["documents", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--content"));
}

// ── Embeds ──────────────────────────────────────────────────────────────────

#[test]
fn embeds_help_shows_subcommands() {
    lineark()
        .args(["embeds", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("download"))
        .stdout(predicate::str::contains("upload"));
}

#[test]
fn embeds_download_help_shows_flags() {
    lineark()
        .args(["embeds", "download", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output"))
        .stdout(predicate::str::contains("--overwrite"));
}

#[test]
fn embeds_upload_help_shows_flags() {
    lineark()
        .args(["embeds", "upload", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--public"));
}

// ── Cycles ──────────────────────────────────────────────────────────────────

#[test]
fn cycles_help_shows_subcommands() {
    lineark()
        .args(["cycles", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"));
}

#[test]
fn cycles_list_help_shows_flags() {
    lineark()
        .args(["cycles", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--active"))
        .stdout(predicate::str::contains("--team"))
        .stdout(predicate::str::contains("--around-active"))
        .stdout(predicate::str::contains("--limit"));
}

#[test]
fn cycles_read_help_shows_team_flag() {
    lineark()
        .args(["cycles", "read", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"));
}

// ── Usage includes Phase 3 commands ─────────────────────────────────────────

#[test]
fn usage_includes_documents_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("documents list"))
        .stdout(predicate::str::contains("documents read"))
        .stdout(predicate::str::contains("documents create"))
        .stdout(predicate::str::contains("documents update"))
        .stdout(predicate::str::contains("documents delete"));
}

#[test]
fn usage_includes_embeds_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("embeds download"))
        .stdout(predicate::str::contains("embeds upload"));
}

#[test]
fn usage_includes_cycles_flags() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("--active"))
        .stdout(predicate::str::contains("--around-active"));
}

// ── Auth error handling ─────────────────────────────────────────────────────

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
