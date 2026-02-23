use super::*;

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
