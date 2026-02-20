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
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"))
        .stdout(predicate::str::contains("members"));
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
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("archive"))
        .stdout(predicate::str::contains("unarchive"))
        .stdout(predicate::str::contains("delete"));
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
        .stdout(predicate::str::contains("issues archive"))
        .stdout(predicate::str::contains("issues unarchive"))
        .stdout(predicate::str::contains("issues delete"))
        .stdout(predicate::str::contains("comments create"));
}

#[test]
fn issues_archive_help_shows_identifier() {
    lineark()
        .args(["issues", "archive", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<IDENTIFIER>"));
}

#[test]
fn issues_unarchive_help_shows_identifier() {
    lineark()
        .args(["issues", "unarchive", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<IDENTIFIER>"));
}

#[test]
fn issues_delete_help_shows_flags() {
    lineark()
        .args(["issues", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--permanently"))
        .stdout(predicate::str::contains("<IDENTIFIER>"));
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

// ── Labels ──────────────────────────────────────────────────────────────────

#[test]
fn labels_list_help_shows_team_flag() {
    lineark()
        .args(["labels", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"));
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

// ── --active and --around-active conflict (issue #4) ────────────────────────

#[test]
fn cycles_list_active_and_around_active_conflict() {
    lineark()
        .args(["cycles", "list", "--active", "--around-active", "2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ── No-op update validation (issue #9) ──────────────────────────────────────

#[test]
fn issues_update_no_flags_prints_error() {
    // Use a fake token to skip auth, but the validation should fire before any API call
    lineark()
        .args(["--api-token", "fake-token", "issues", "update", "ENG-123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No update fields provided"));
}

#[test]
fn documents_update_no_flags_prints_error() {
    lineark()
        .args([
            "--api-token",
            "fake-token",
            "documents",
            "update",
            "doc-uuid",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No update fields provided"));
}

// ── Projects ────────────────────────────────────────────────────────────────

#[test]
fn projects_help_shows_subcommands() {
    lineark()
        .args(["projects", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("create"));
}

#[test]
fn projects_create_help_shows_flags() {
    lineark()
        .args(["projects", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--lead"))
        .stdout(predicate::str::contains("--members"))
        .stdout(predicate::str::contains("--start-date"))
        .stdout(predicate::str::contains("--target-date"))
        .stdout(predicate::str::contains("--priority"))
        .stdout(predicate::str::contains("--content"))
        .stdout(predicate::str::contains("--icon"))
        .stdout(predicate::str::contains("--color"));
}

#[test]
fn projects_list_help_shows_led_by_me_flag() {
    lineark()
        .args(["projects", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--led-by-me"));
}

#[test]
fn projects_read_help_shows_description() {
    lineark()
        .args(["projects", "read", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Project name or UUID"));
}

#[test]
fn projects_create_requires_team_flag() {
    lineark()
        .args([
            "--api-token",
            "fake-token",
            "projects",
            "create",
            "My Project",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--team"));
}

#[test]
fn usage_includes_projects_create() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("projects create"));
}

// ── Project milestones ──────────────────────────────────────────────────────

#[test]
fn milestones_help_shows_subcommands() {
    lineark()
        .args(["project-milestones", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("read"))
        .stdout(predicate::str::contains("create"))
        .stdout(predicate::str::contains("update"))
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn milestones_list_help_shows_project_flag() {
    lineark()
        .args(["project-milestones", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--limit"));
}

#[test]
fn milestones_create_help_shows_flags() {
    lineark()
        .args(["project-milestones", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--target-date"))
        .stdout(predicate::str::contains("--description"));
}

#[test]
fn milestones_update_help_shows_flags() {
    lineark()
        .args(["project-milestones", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--target-date"))
        .stdout(predicate::str::contains("--description"));
}

#[test]
fn milestones_update_no_flags_prints_error() {
    lineark()
        .args([
            "--api-token",
            "fake-token",
            "project-milestones",
            "update",
            "some-uuid",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No update fields provided"));
}

#[test]
fn usage_includes_milestones_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("project-milestones list"))
        .stdout(predicate::str::contains("project-milestones read"))
        .stdout(predicate::str::contains("project-milestones create"))
        .stdout(predicate::str::contains("project-milestones update"))
        .stdout(predicate::str::contains("project-milestones delete"));
}

// ── Issues: new flags ──────────────────────────────────────────────────────

#[test]
fn issues_create_help_shows_project_and_cycle() {
    lineark()
        .args(["issues", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--cycle"))
        .stdout(predicate::str::contains("--status"));
}

#[test]
fn issues_update_help_shows_clear_parent_and_project() {
    lineark()
        .args(["issues", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--clear-parent"))
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--cycle"));
}

#[test]
fn issues_search_help_shows_filter_flags() {
    lineark()
        .args(["issues", "search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--team"))
        .stdout(predicate::str::contains("--assignee"))
        .stdout(predicate::str::contains("--status"));
}

#[test]
fn issues_update_clear_parent_conflicts_with_parent() {
    lineark()
        .args([
            "--api-token",
            "fake",
            "issues",
            "update",
            "ENG-123",
            "--parent",
            "ENG-456",
            "--clear-parent",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ── Documents: filter flags ────────────────────────────────────────────────

#[test]
fn documents_list_help_shows_filter_flags() {
    lineark()
        .args(["documents", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--project"))
        .stdout(predicate::str::contains("--issue"));
}

// ── Short flag aliases ─────────────────────────────────────────────────────

#[test]
fn issues_list_accepts_short_limit_flag() {
    // -l should be accepted as --limit alias (will fail on API call, but parsing should succeed)
    lineark()
        .args(["--api-token", "fake", "issues", "list", "-l", "5"])
        .assert()
        .failure() // fails on API, not on arg parsing
        .stderr(predicate::str::contains("limit").not()); // should not complain about limit flag
}

#[test]
fn issues_create_accepts_short_flags() {
    lineark()
        .args(["issues", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-p"))
        .stdout(predicate::str::contains("-d"))
        .stdout(predicate::str::contains("-s"));
}

#[test]
fn issues_update_accepts_short_flags() {
    lineark()
        .args(["issues", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("-p"))
        .stdout(predicate::str::contains("-d"))
        .stdout(predicate::str::contains("-t"))
        .stdout(predicate::str::contains("-s"));
}

// ── Usage includes name resolution info ────────────────────────────────────

#[test]
fn usage_includes_name_resolution() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("NAME RESOLUTION"));
}

#[test]
fn usage_includes_me_alias() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("`me`"));
}

#[test]
fn usage_includes_projects_read() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("projects read"));
}

#[test]
fn usage_includes_led_by_me() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("--led-by-me"));
}

#[test]
fn usage_includes_members_flag() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("--members"));
}

// ── Cycle number parsing rejects NaN/inf (issue #6) ─────────────────────────

#[test]
fn cycles_read_rejects_nan() {
    // NaN should not parse as i64, so it will be treated as a name lookup
    // which requires --team. This verifies it doesn't silently succeed as f64.
    lineark()
        .args(["--api-token", "fake", "cycles", "read", "NaN"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--team"));
}

#[test]
fn cycles_read_rejects_inf() {
    lineark()
        .args(["--api-token", "fake", "cycles", "read", "inf"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--team"));
}

// ── Teams CRUD ──────────────────────────────────────────────────────────

#[test]
fn teams_create_help_shows_flags() {
    lineark()
        .args(["teams", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--key"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--icon"))
        .stdout(predicate::str::contains("--color"))
        .stdout(predicate::str::contains("--timezone"))
        .stdout(predicate::str::contains("--private"))
        .stdout(predicate::str::contains("--cycles-enabled"))
        .stdout(predicate::str::contains("--triage-enabled"));
}

#[test]
fn teams_update_help_shows_flags() {
    lineark()
        .args(["teams", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name"))
        .stdout(predicate::str::contains("--key"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--icon"))
        .stdout(predicate::str::contains("--color"))
        .stdout(predicate::str::contains("--timezone"))
        .stdout(predicate::str::contains("--private"))
        .stdout(predicate::str::contains("--cycles-enabled"))
        .stdout(predicate::str::contains("--triage-enabled"));
}

#[test]
fn teams_update_no_fields_errors() {
    lineark()
        .args(["--api-token", "fake-token", "teams", "update", "ENG"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No update fields provided"));
}

#[test]
fn teams_delete_help_shows_description() {
    lineark()
        .args(["teams", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<ID>"));
}

#[test]
fn teams_read_help_shows_description() {
    lineark()
        .args(["teams", "read", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<ID>"));
}

#[test]
fn teams_members_add_help_shows_flags() {
    lineark()
        .args(["teams", "members", "add", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--user"));
}

#[test]
fn teams_members_remove_help_shows_flags() {
    lineark()
        .args(["teams", "members", "remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--user"));
}

#[test]
fn teams_members_add_requires_user_flag() {
    lineark()
        .args([
            "--api-token",
            "fake-token",
            "teams",
            "members",
            "add",
            "ENG",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--user"));
}

#[test]
fn usage_includes_teams_create() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("teams create"))
        .stdout(predicate::str::contains("teams update"))
        .stdout(predicate::str::contains("teams delete"))
        .stdout(predicate::str::contains("teams read"))
        .stdout(predicate::str::contains("teams members add"))
        .stdout(predicate::str::contains("teams members remove"));
}

// ── Self command ─────────────────────────────────────────────────────────────

#[test]
fn self_help_shows_update_subcommand() {
    lineark()
        .args(["self", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("update"));
}

#[test]
fn self_update_help_shows_check_flag() {
    lineark()
        .args(["self", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--check"));
}

#[test]
fn self_update_dev_build_exits_cleanly() {
    // Dev builds have version 0.0.0 — self update should print a message and succeed.
    lineark()
        .args(["self", "update"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dev build"));
}

#[test]
fn self_update_check_dev_build_exits_cleanly() {
    lineark()
        .args(["self", "update", "--check"])
        .assert()
        .success()
        .stderr(predicate::str::contains("dev build"));
}

#[test]
fn usage_includes_self_update_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("self update"))
        .stdout(predicate::str::contains("--check"));
}

// ── Comments delete ─────────────────────────────────────────────────────────

#[test]
fn comments_help_shows_delete_subcommand() {
    lineark()
        .args(["comments", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("delete"));
}

#[test]
fn comments_delete_help_shows_id_arg() {
    lineark()
        .args(["comments", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<ID>"));
}

#[test]
fn usage_includes_comments_delete() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("comments delete"));
}

// ── Estimate flag ───────────────────────────────────────────────────────────

#[test]
fn issues_create_help_shows_estimate() {
    lineark()
        .args(["issues", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--estimate"))
        .stdout(predicate::str::contains("-e"));
}

#[test]
fn issues_update_help_shows_estimate() {
    lineark()
        .args(["issues", "update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--estimate"))
        .stdout(predicate::str::contains("-e"));
}

#[test]
fn issues_update_no_flags_error_mentions_estimate() {
    lineark()
        .args(["--api-token", "fake-token", "issues", "update", "ENG-123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--estimate"))
}
