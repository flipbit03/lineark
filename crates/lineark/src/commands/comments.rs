use clap::Args;
use lineark_sdk::generated::inputs::CommentCreateInput;
use lineark_sdk::Client;

use super::helpers::{check_success, resolve_issue_id};
use crate::output::{self, Format};

/// Manage comments.
#[derive(Debug, Args)]
pub struct CommentsCmd {
    #[command(subcommand)]
    pub action: CommentsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum CommentsAction {
    /// Create a comment on an issue.
    ///
    /// Examples:
    ///   lineark comments create ENG-123 --body "Working on it"
    ///   lineark comments create ENG-123 --body "Fixed in PR #42"
    Create {
        /// Issue identifier (e.g., ENG-123) or UUID.
        issue: String,
        /// Comment body in markdown format.
        #[arg(long)]
        body: String,
    },
}

pub async fn run(cmd: CommentsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        CommentsAction::Create { issue, body } => {
            // Resolve the issue identifier to a UUID if needed.
            let issue_id = resolve_issue_id(client, &issue).await?;

            let input = CommentCreateInput {
                body: Some(body),
                issue_id: Some(issue_id),
                ..Default::default()
            };

            let payload = client
                .comment_create(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            check_success(&payload)?;

            let comment = payload.get("comment").cloned().unwrap_or_default();
            output::print_one(&comment, format);
        }
    }
    Ok(())
}
