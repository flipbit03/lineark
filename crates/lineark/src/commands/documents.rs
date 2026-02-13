use clap::Args;
use lineark_sdk::generated::inputs::{DocumentCreateInput, DocumentUpdateInput};
use lineark_sdk::Client;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

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

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DocumentListItem {
    id: Option<String>,
    title: Option<String>,
    slug_id: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Serialize, Tabled)]
struct DocumentRow {
    id: String,
    title: String,
    slug: String,
    updated_at: String,
}

impl From<&DocumentListItem> for DocumentRow {
    fn from(d: &DocumentListItem) -> Self {
        Self {
            id: d.id.clone().unwrap_or_default(),
            title: d.title.clone().unwrap_or_default(),
            slug: d.slug_id.clone().unwrap_or_default(),
            updated_at: d.updated_at.clone().unwrap_or_default(),
        }
    }
}

const DOCUMENT_LIST_QUERY: &str = "query Documents($first: Int) { documents(first: $first) { \
nodes { id title slugId createdAt updatedAt url } \
pageInfo { hasNextPage endCursor } } }";

// ── Detail ───────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DocumentDetail {
    id: Option<String>,
    title: Option<String>,
    slug_id: Option<String>,
    content: Option<String>,
    url: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    project: Option<NestedProject>,
    creator: Option<NestedUser>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedProject {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedUser {
    id: Option<String>,
    name: Option<String>,
}

const DOCUMENT_READ_QUERY: &str = "query Document($id: String!) { document(id: $id) { \
id title slugId content url createdAt updatedAt icon color \
project { id name } creator { id name } \
} }";

// ── Command dispatch ─────────────────────────────────────────────────────────

pub async fn run(cmd: DocumentsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        DocumentsAction::List { limit } => {
            let variables = serde_json::json!({ "first": limit });
            let conn = client
                .execute_connection::<DocumentListItem>(DOCUMENT_LIST_QUERY, variables, "documents")
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
            let variables = serde_json::json!({ "id": id });
            let doc: DocumentDetail = client
                .execute(DOCUMENT_READ_QUERY, variables, "document")
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
            let input = DocumentCreateInput {
                title: Some(title),
                content,
                project_id: project,
                issue_id: issue,
                ..Default::default()
            };

            let payload = client
                .document_create(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            check_success(&payload)?;
            let doc = payload.get("document").cloned().unwrap_or_default();
            output::print_one(&doc, format);
        }
        DocumentsAction::Update { id, title, content } => {
            let input = DocumentUpdateInput {
                title,
                content,
                ..Default::default()
            };

            let payload = client
                .document_update(input, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            check_success(&payload)?;
            let doc = payload.get("document").cloned().unwrap_or_default();
            output::print_one(&doc, format);
        }
        DocumentsAction::Delete { id } => {
            let payload = client
                .document_delete(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            check_success(&payload)?;
            output::print_one(&payload, format);
        }
    }
    Ok(())
}

fn check_success(payload: &serde_json::Value) -> anyhow::Result<()> {
    if payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
        return Err(anyhow::anyhow!(
            "Operation failed: {}",
            serde_json::to_string_pretty(payload).unwrap_or_default()
        ));
    }
    Ok(())
}
