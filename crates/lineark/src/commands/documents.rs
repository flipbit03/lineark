use clap::Args;
use lineark_sdk::generated::inputs::{DocumentCreateInput, DocumentFilter, DocumentUpdateInput};
use lineark_sdk::generated::types::{Document, DocumentSearchResult};
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{resolve_issue_id, resolve_project_id, resolve_team_id};
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
    ///   lineark documents list --issue ENG-123
    List {
        /// Maximum number of documents to return (max 250).
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Filter by project name or UUID.
        #[arg(long)]
        project: Option<String>,
        /// Filter by issue identifier (e.g., ENG-123) or UUID.
        #[arg(long)]
        issue: Option<String>,
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
    ///   lineark documents create --title "Spec" --content "Details" --project "My Project"
    Create {
        /// Document title.
        #[arg(long)]
        title: String,
        /// Document content in markdown format.
        #[arg(long)]
        content: Option<String>,
        /// Related project name or UUID.
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
    /// Full-text search across document titles and content.
    ///
    /// Examples:
    ///   lineark documents search "onboarding"
    ///   lineark documents search "API design" --limit 10
    ///   lineark documents search "spec" --team ENG
    Search {
        /// Search query text.
        query: String,
        /// Maximum number of results (max 250).
        #[arg(short = 'l', long, default_value = "25", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Filter by team key, name, or UUID.
        #[arg(long)]
        team: Option<String>,
    },
}

// ── Lean types ───────────────────────────────────────────────────────────────

/// Lean document type for list views — avoids fetching contentState and other noise.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Document)]
#[serde(rename_all = "camelCase", default)]
struct DocumentSummary {
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
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

/// Lean search result type for `documents search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = DocumentSearchResult)]
#[serde(rename_all = "camelCase", default)]
struct DocSearchSummary {
    pub id: Option<String>,
    pub title: Option<String>,
    pub slug_id: Option<String>,
    pub url: Option<String>,
    pub updated_at: Option<String>,
}

// ── List row ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
struct DocumentRow {
    id: String,
    title: String,
    url: String,
    updated_at: String,
}

impl From<&DocumentSummary> for DocumentRow {
    fn from(d: &DocumentSummary) -> Self {
        Self {
            id: d.id.clone().unwrap_or_default(),
            title: d.title.clone().unwrap_or_default(),
            url: d.url.clone().unwrap_or_default(),
            updated_at: d.updated_at.clone().unwrap_or_default(),
        }
    }
}

// ── Search row ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Tabled)]
struct DocSearchRow {
    id: String,
    title: String,
    slug_id: String,
    url: String,
    updated_at: String,
}

impl From<&DocSearchSummary> for DocSearchRow {
    fn from(d: &DocSearchSummary) -> Self {
        Self {
            id: d.id.clone().unwrap_or_default(),
            title: d.title.clone().unwrap_or_default(),
            slug_id: d.slug_id.clone().unwrap_or_default(),
            url: d.url.clone().unwrap_or_default(),
            updated_at: d.updated_at.clone().unwrap_or_default(),
        }
    }
}

// ── Command dispatch ─────────────────────────────────────────────────────────

pub async fn run(cmd: DocumentsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        DocumentsAction::List {
            limit,
            project,
            issue,
        } => {
            let mut query = client.documents::<DocumentSummary>().first(limit);

            // Build filter from --project / --issue flags
            let has_filter = project.is_some() || issue.is_some();
            if has_filter {
                let mut filter_json = serde_json::Map::new();

                if let Some(ref project_name) = project {
                    let project_id = resolve_project_id(client, project_name).await?;
                    filter_json.insert(
                        "project".into(),
                        serde_json::json!({ "id": { "eq": project_id } }),
                    );
                }

                if let Some(ref issue_ident) = issue {
                    let issue_id = resolve_issue_id(client, issue_ident).await?;
                    filter_json.insert(
                        "issue".into(),
                        serde_json::json!({ "id": { "eq": issue_id } }),
                    );
                }

                let filter: DocumentFilter =
                    serde_json::from_value(serde_json::Value::Object(filter_json))
                        .expect("valid DocumentFilter");
                query = query.filter(filter);
            }

            let conn = query.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

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
            let project_id = match project {
                Some(ref name) => Some(resolve_project_id(client, name).await?),
                None => None,
            };
            let issue_id = match issue {
                Some(ref id) => Some(resolve_issue_id(client, id).await?),
                None => None,
            };

            let input = DocumentCreateInput {
                title: Some(title),
                content,
                project_id,
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
        DocumentsAction::Search { query, limit, team } => {
            let mut builder = client
                .search_documents::<DocSearchSummary>(query)
                .first(limit);

            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                builder = builder.team_id(team_id);
            }

            let conn = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<DocSearchRow> = conn.nodes.iter().map(DocSearchRow::from).collect();
            output::print_table(&rows, format);
        }
    }
    Ok(())
}
