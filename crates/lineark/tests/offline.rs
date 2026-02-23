//! Offline CLI tests for lineark — no API token needed.

use assert_cmd::Command;
use predicates::prelude::*;

fn lineark() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("lineark").unwrap()
}

#[path = "offline/auth.rs"]
mod auth;
#[path = "offline/comments.rs"]
mod comments;
#[path = "offline/cycles.rs"]
mod cycles;
#[path = "offline/documents.rs"]
mod documents;
#[path = "offline/embeds.rs"]
mod embeds;
#[path = "offline/format.rs"]
mod format;
#[path = "offline/issues.rs"]
mod issues;
#[path = "offline/labels.rs"]
mod labels;
#[path = "offline/milestones.rs"]
mod milestones;
#[path = "offline/projects.rs"]
mod projects;
#[path = "offline/self_update.rs"]
mod self_update;
#[path = "offline/teams.rs"]
mod teams;
#[path = "offline/usage.rs"]
mod usage;
