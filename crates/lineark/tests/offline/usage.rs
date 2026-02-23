use super::*;

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

#[test]
fn usage_includes_projects_create() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("projects create"));
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

#[test]
fn usage_includes_self_update_commands() {
    lineark()
        .arg("usage")
        .assert()
        .success()
        .stdout(predicate::str::contains("self update"))
        .stdout(predicate::str::contains("--check"));
}
