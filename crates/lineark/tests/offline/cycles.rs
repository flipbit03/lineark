use super::*;

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

#[test]
fn cycles_list_active_and_around_active_conflict() {
    lineark()
        .args(["cycles", "list", "--active", "--around-active", "2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn cycles_read_rejects_nan() {
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
