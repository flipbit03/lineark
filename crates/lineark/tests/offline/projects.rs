use super::*;

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
