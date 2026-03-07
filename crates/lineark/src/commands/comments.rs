use clap::Args;
use lineark_sdk::generated::inputs::{CommentCreateInput, CommentUpdateInput};
use lineark_sdk::generated::types::Comment;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};

use super::helpers::resolve_issue_id;
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
    /// Update a comment's body.
    ///
    /// Examples:
    ///   lineark comments update COMMENT-UUID --body "Updated text"
    Update {
        /// Comment UUID.
        id: String,
        /// New comment body in markdown format.
        #[arg(long)]
        body: Option<String>,
    },
    /// Delete a comment.
    ///
    /// Examples:
    ///   lineark comments delete COMMENT-UUID
    Delete {
        /// Comment UUID.
        id: String,
    },
    /// Resolve a comment thread.
    ///
    /// Examples:
    ///   lineark comments resolve COMMENT-UUID
    ///   lineark comments resolve COMMENT-UUID --resolving-comment REPLY-UUID
    Resolve {
        /// Comment UUID (the thread root to resolve).
        id: String,
        /// Optional UUID of the reply comment that resolves this thread.
        #[arg(long)]
        resolving_comment: Option<String>,
    },
    /// Unresolve a previously resolved comment thread.
    ///
    /// Examples:
    ///   lineark comments unresolve COMMENT-UUID
    Unresolve {
        /// Comment UUID.
        id: String,
    },
}

/// Lean result type for comment mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Comment)]
#[serde(rename_all = "camelCase", default)]
struct CommentRef {
    id: Option<String>,
    body: Option<String>,
    resolved_at: Option<String>,
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

            let comment = client
                .comment_create::<CommentRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&comment, format);
        }
        CommentsAction::Update { id, body } => {
            if body.is_none() {
                anyhow::bail!("No update fields provided. Use --body to set the new comment body.");
            }

            let input = CommentUpdateInput {
                body,
                ..Default::default()
            };

            let comment = client
                .comment_update::<CommentRef>(None, input, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&comment, format);
        }
        CommentsAction::Delete { id } => {
            client
                .comment_delete(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&serde_json::json!({ "success": true }), format);
        }
        CommentsAction::Resolve {
            id,
            resolving_comment,
        } => {
            let comment = client
                .comment_resolve::<CommentRef>(resolving_comment, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&comment, format);
        }
        CommentsAction::Unresolve { id } => {
            let comment = client
                .comment_unresolve::<CommentRef>(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&comment, format);
        }
    }
    Ok(())
}
