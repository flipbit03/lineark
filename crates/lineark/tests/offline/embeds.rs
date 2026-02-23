use super::*;

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
