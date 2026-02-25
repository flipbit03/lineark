use clap::Args;
use lineark_sdk::generated::enums::IssueRelationType;
use lineark_sdk::generated::inputs::IssueRelationCreateInput;
use lineark_sdk::generated::types::IssueRelation;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};

use super::helpers::resolve_issue_id;
use crate::output::{self, Format};

/// Manage issue relations (blocking, related, duplicate, similar).
#[derive(Debug, Args)]
pub struct RelationsCmd {
    #[command(subcommand)]
    pub action: RelationsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum RelationsAction {
    /// Create a relation between two issues.
    ///
    /// Exactly one relation flag is required: --blocks, --blocked-by, --related,
    /// --duplicate, or --similar.
    ///
    /// Examples:
    ///   lineark relations create ENG-100 --blocks ENG-101
    ///   lineark relations create ENG-100 --blocked-by ENG-99
    ///   lineark relations create ENG-100 --related ENG-200
    ///   lineark relations create ENG-100 --duplicate ENG-200
    ///   lineark relations create ENG-100 --similar ENG-200
    Create {
        /// Source issue identifier (e.g., ENG-123) or UUID.
        issue: String,
        /// The source issue blocks the specified issue.
        #[arg(long, group = "relation_type")]
        blocks: Option<String>,
        /// The source issue is blocked by the specified issue.
        #[arg(long, group = "relation_type")]
        blocked_by: Option<String>,
        /// The source issue is related to the specified issue.
        #[arg(long, group = "relation_type")]
        related: Option<String>,
        /// The source issue is a duplicate of the specified issue.
        #[arg(long, group = "relation_type")]
        duplicate: Option<String>,
        /// The source issue is similar to the specified issue.
        #[arg(long, group = "relation_type")]
        similar: Option<String>,
    },
    /// Delete an issue relation.
    ///
    /// Examples:
    ///   lineark relations delete RELATION-UUID
    Delete {
        /// Relation UUID (visible in `lineark issues read` output).
        id: String,
    },
}

/// Lean result type for relation mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueRelation)]
#[serde(rename_all = "camelCase", default)]
struct RelationRef {
    id: Option<String>,
    r#type: Option<String>,
}

pub async fn run(cmd: RelationsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        RelationsAction::Create {
            issue,
            blocks,
            blocked_by,
            related,
            duplicate,
            similar,
        } => {
            // Determine relation type and target issue.
            let (relation_type, source, target) = if let Some(ref target) = blocks {
                (IssueRelationType::Blocks, &issue, target)
            } else if let Some(ref blocker) = blocked_by {
                // "A is blocked by B" means "B blocks A" — swap source and target.
                (IssueRelationType::Blocks, blocker, &issue)
            } else if let Some(ref target) = related {
                (IssueRelationType::Related, &issue, target)
            } else if let Some(ref target) = duplicate {
                (IssueRelationType::Duplicate, &issue, target)
            } else if let Some(ref target) = similar {
                (IssueRelationType::Similar, &issue, target)
            } else {
                return Err(anyhow::anyhow!(
                    "Exactly one relation flag is required: --blocks, --blocked-by, --related, --duplicate, or --similar."
                ));
            };

            let source_id = resolve_issue_id(client, source).await?;
            let target_id = resolve_issue_id(client, target).await?;

            let input = IssueRelationCreateInput {
                issue_id: Some(source_id),
                related_issue_id: Some(target_id),
                r#type: Some(relation_type),
                ..Default::default()
            };

            let relation = client
                .issue_relation_create::<RelationRef>(None, input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&relation, format);
        }
        RelationsAction::Delete { id } => {
            client
                .issue_relation_delete(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&serde_json::json!({ "success": true }), format);
        }
    }
    Ok(())
}
