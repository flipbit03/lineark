use super::*;

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
