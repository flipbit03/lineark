use super::*;

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
