use clap::Args;
use lineark_sdk::generated::inputs::{DocumentCreateInput, DocumentUpdateInput};
use lineark_sdk::generated::types::Document;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::resolve_issue_id;
use crate::output::{self, Format};

/// Manage documents.
#[derive(Debug, Args)]
pub struct DocumentsCmd {
    #[command(subcommand)]
    pub action: DocumentsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum DocumentsAction {
    /// List documents in the workspace.
    ///
    /// Examples:
    ///   lineark documents list
    ///   lineark documents list --project "My Project"
    List {
        /// Maximum number of documents to return (max 250).
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
    },
    /// Read a specific document by ID (includes content).
    Read {
        /// Document UUID.
        id: String,
    },
    /// Create a new document.
    ///
    /// Examples:
    ///   lineark documents create --title "Design Doc" --content "# Overview\n..."
    ///   lineark documents create --title "Spec" --content "Details" --project PROJECT-UUID
    Create {
        /// Document title.
        #[arg(long)]
        title: String,
        /// Document content in markdown format.
        #[arg(long)]
        content: Option<String>,
        /// Related project UUID.
        #[arg(long)]
        project: Option<String>,
        /// Related issue identifier (e.g., ENG-123) or UUID.
        #[arg(long)]
        issue: Option<String>,
    },
    /// Update an existing document.
    ///
    /// Examples:
    ///   lineark documents update DOC-UUID --title "New Title"
    ///   lineark documents update DOC-UUID --content "Updated content"
    Update {
        /// Document UUID.
        id: String,
        /// New title.
        #[arg(long)]
        title: Option<String>,
        /// New content in markdown format.
        #[arg(long)]
        content: Option<String>,
    },
    /// Delete (trash) a document.
    Delete {
        /// Document UUID.
        id: String,
    },
}

// ── List row ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
struct DocumentRow {
    id: String,
    title: String,
    slug: String,
    updated_at: String,
}

impl From<&Document> for DocumentRow {
    fn from(d: &Document) -> Self {
        Self {
            id: d.id.clone().unwrap_or_default(),
            title: d.title.clone().unwrap_or_default(),
            slug: d.slug_id.clone().unwrap_or_default(),
            updated_at: d.updated_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        }
    }
}

/// Lean result type for document mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Document)]
#[serde(rename_all = "camelCase", default)]
struct DocumentRef {
    id: Option<String>,
    title: Option<String>,
    slug_id: Option<String>,
}

// ── Command dispatch ─────────────────────────────────────────────────────────

pub async fn run(cmd: DocumentsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        DocumentsAction::List { limit } => {
            let conn = client
                .documents::<Document>()
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            match format {
                Format::Json => {
                    let json = serde_json::to_string_pretty(&conn.nodes).unwrap_or_default();
                    println!("{json}");
                }
                Format::Human => {
                    let rows: Vec<DocumentRow> = conn.nodes.iter().map(DocumentRow::from).collect();
                    output::print_table(&rows, format);
                }
            }
        }
        DocumentsAction::Read { id } => {
            let doc = client
                .document::<Document>(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&doc, format);
        }
        DocumentsAction::Create {
            title,
            content,
            project,
            issue,
        } => {
            let issue_id = match issue {
                Some(ref id) => Some(resolve_issue_id(client, id).await?),
                None => None,
            };

            let input = DocumentCreateInput {
                title: Some(title),
                content,
                project_id: project,
                issue_id,
                ..Default::default()
            };

            let doc = client
                .document_create::<DocumentRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&doc, format);
        }
        DocumentsAction::Update { id, title, content } => {
            if title.is_none() && content.is_none() {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --title or --content to specify changes."
                ));
            }

            let input = DocumentUpdateInput {
                title,
                content,
                ..Default::default()
            };

            let doc = client
                .document_update::<DocumentRef>(input, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&doc, format);
        }
        DocumentsAction::Delete { id } => {
            let doc = client
                .document_delete::<DocumentRef>(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&doc, format);
        }
    }
    Ok(())
}
