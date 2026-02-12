use clap::Args;
use lineark_sdk::generated::inputs::CommentCreateInput;
use lineark_sdk::Client;

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

            if payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
                return Err(anyhow::anyhow!(
                    "Comment creation failed: {}",
                    serde_json::to_string_pretty(&payload).unwrap_or_default()
                ));
            }

            let comment = payload.get("comment").cloned().unwrap_or_default();
            output::print_one(&comment, format);
        }
    }
    Ok(())
}

/// Resolve an issue identifier (e.g., ENG-123) to a UUID.
async fn resolve_issue_id(client: &Client, identifier: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        return Ok(identifier.to_string());
    }
    let variables = serde_json::json!({ "term": identifier, "first": 5 });
    let conn: lineark_sdk::Connection<serde_json::Value> = client
        .execute_connection(
            "query IssueIdSearch($term: String!, $first: Int) { searchIssues(term: $term, first: $first) { nodes { id identifier } pageInfo { hasNextPage endCursor } } }",
            variables,
            "searchIssues",
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    conn.nodes
        .iter()
        .find(|n| {
            n.get("identifier")
                .and_then(|v| v.as_str())
                .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
        })
        .and_then(|n| n.get("id").and_then(|v| v.as_str()).map(String::from))
        .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))
}
