//! Blocking (synchronous) Linear API client.
//!
//! This module provides a synchronous wrapper around the async [`Client`](crate::Client).
//! Enable it with the `blocking` feature flag:
//!
//! ```toml
//! [dependencies]
//! lineark-sdk = { version = "...", features = ["blocking"] }
//! ```
//!
//! The blocking client creates an internal tokio runtime and runs each
//! operation to completion synchronously. It mirrors the async client's API
//! surface, so the same query builder patterns and mutation methods are
//! available.
//!
//! # Example
//!
//! ```no_run
//! use lineark_sdk::blocking::Client;
//!
//! let client = Client::auto().unwrap();
//! let me = client.whoami().unwrap();
//! println!("Logged in as: {:?}", me.name);
//!
//! let teams = client.teams().send().unwrap();
//! for team in &teams.nodes {
//!     println!("{}: {}", team.key.as_deref().unwrap_or("?"), team.name.as_deref().unwrap_or("?"));
//! }
//! ```

use crate::error::LinearError;
use crate::helpers::{DownloadResult, UploadResult};
use crate::pagination::Connection;
use serde::de::DeserializeOwned;

/// A synchronous Linear API client.
///
/// Wraps the async [`crate::Client`] with an internal tokio runtime. Every
/// method blocks the calling thread until the operation completes.
///
/// Construct it with the same factory methods as the async client:
/// [`from_token`](Client::from_token), [`from_env`](Client::from_env),
/// [`from_file`](Client::from_file), or [`auto`](Client::auto).
pub struct Client {
    inner: crate::Client,
    rt: tokio::runtime::Runtime,
}

impl Client {
    /// Create a blocking client with an explicit API token.
    pub fn from_token(token: impl Into<String>) -> Result<Self, LinearError> {
        Ok(Self {
            inner: crate::Client::from_token(token)?,
            rt: build_runtime()?,
        })
    }

    /// Create a blocking client from the `LINEAR_API_TOKEN` environment variable.
    pub fn from_env() -> Result<Self, LinearError> {
        Ok(Self {
            inner: crate::Client::from_env()?,
            rt: build_runtime()?,
        })
    }

    /// Create a blocking client from the `~/.linear_api_token` file.
    pub fn from_file() -> Result<Self, LinearError> {
        Ok(Self {
            inner: crate::Client::from_file()?,
            rt: build_runtime()?,
        })
    }

    /// Create a blocking client by auto-detecting the token (env -> file).
    pub fn auto() -> Result<Self, LinearError> {
        Ok(Self {
            inner: crate::Client::auto()?,
            rt: build_runtime()?,
        })
    }

    /// Execute a GraphQL query and extract a single object from the response.
    ///
    /// This is the blocking equivalent of [`crate::Client::execute`].
    pub fn execute<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
        data_path: &str,
    ) -> Result<T, LinearError> {
        self.rt
            .block_on(self.inner.execute(query, variables, data_path))
    }

    /// Execute a GraphQL query and extract a [`Connection`] from the response.
    ///
    /// This is the blocking equivalent of [`crate::Client::execute_connection`].
    pub fn execute_connection<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: serde_json::Value,
        data_path: &str,
    ) -> Result<Connection<T>, LinearError> {
        self.rt
            .block_on(self.inner.execute_connection(query, variables, data_path))
    }

    /// Download a file from a URL (blocking).
    ///
    /// See [`crate::Client::download_url`] for full documentation.
    pub fn download_url(&self, url: &str) -> Result<DownloadResult, LinearError> {
        self.rt.block_on(self.inner.download_url(url))
    }

    /// Upload a file to Linear's cloud storage (blocking).
    ///
    /// See [`crate::Client::upload_file`] for full documentation.
    pub fn upload_file(
        &self,
        filename: &str,
        content_type: &str,
        bytes: Vec<u8>,
        make_public: bool,
    ) -> Result<UploadResult, LinearError> {
        self.rt.block_on(
            self.inner
                .upload_file(filename, content_type, bytes, make_public),
        )
    }
}

// ── Generated query method wrappers ──────────────────────────────────────────

/// A blocking query builder. Wraps an async builder and runs `.send()` synchronously.
#[must_use]
pub struct BlockingQuery<B> {
    builder: B,
    rt: *const tokio::runtime::Runtime,
}

// Safety: BlockingQuery is only used on the same thread as the Client that created it.
// The runtime pointer is valid for the lifetime of the Client.
unsafe impl<B: Send> Send for BlockingQuery<B> {}

impl<B> BlockingQuery<B> {
    fn new(builder: B, rt: &tokio::runtime::Runtime) -> Self {
        Self {
            builder,
            rt: rt as *const _,
        }
    }
}

// Implement builder forwarding and send() for each generated query type.
// We use a macro to avoid repeating the pattern for every query builder.

macro_rules! blocking_query_builder {
    (
        query_type = $QueryType:ident,
        return_type = $ReturnKind:ident < $ReturnType:ty >,
        methods = [ $( $method:ident ( $arg_ty:ty ) ),* $(,)? ]
    ) => {
        impl BlockingQuery<crate::generated::queries::$QueryType<'_>> {
            $(
                pub fn $method(mut self, value: $arg_ty) -> Self {
                    self.builder = self.builder.$method(value);
                    self
                }
            )*

            blocking_query_builder!(@send $ReturnKind<$ReturnType>);
        }
    };
    // Connection return type
    (@send Connection<$T:ty>) => {
        /// Execute the query and return the result synchronously.
        pub fn send(self) -> Result<Connection<$T>, LinearError> {
            // Safety: extract rt before moving builder out of self.
            let rt = unsafe { &*self.rt };
            rt.block_on(self.builder.send())
        }
    };
    // Single return type
    (@send Single<$T:ty>) => {
        /// Execute the query and return the result synchronously.
        pub fn send(self) -> Result<$T, LinearError> {
            let rt = unsafe { &*self.rt };
            rt.block_on(self.builder.send())
        }
    };
}

use crate::generated::types::*;

blocking_query_builder! {
    query_type = WorkflowStatesQuery,
    return_type = Connection<WorkflowState>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = UsersQuery,
    return_type = Connection<User>,
    methods = [include_disabled(bool), before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = TeamsQuery,
    return_type = Connection<Team>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = ProjectsQuery,
    return_type = Connection<Project>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = IssueLabelsQuery,
    return_type = Connection<IssueLabel>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = IssuesQuery,
    return_type = Connection<Issue>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = CyclesQuery,
    return_type = Connection<Cycle>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = SearchIssuesQuery,
    return_type = Connection<Issue>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool), include_comments(bool), team_id(impl Into<String>)]
}

blocking_query_builder! {
    query_type = DocumentsQuery,
    return_type = Connection<Document>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = IssueRelationsQuery,
    return_type = Connection<IssueRelation>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

/// Blocking equivalents of the generated query constructor methods on [`Client`].
impl Client {
    /// Query the authenticated user (blocking).
    pub fn whoami(&self) -> Result<User, LinearError> {
        self.rt.block_on(self.inner.whoami())
    }

    /// List workflow states (blocking).
    pub fn workflow_states(
        &self,
    ) -> BlockingQuery<crate::generated::queries::WorkflowStatesQuery<'_>> {
        BlockingQuery::new(self.inner.workflow_states(), &self.rt)
    }

    /// List users (blocking).
    pub fn users(&self) -> BlockingQuery<crate::generated::queries::UsersQuery<'_>> {
        BlockingQuery::new(self.inner.users(), &self.rt)
    }

    /// List teams (blocking).
    pub fn teams(&self) -> BlockingQuery<crate::generated::queries::TeamsQuery<'_>> {
        BlockingQuery::new(self.inner.teams(), &self.rt)
    }

    /// Query a single team by ID (blocking).
    pub fn team(&self, id: String) -> Result<Team, LinearError> {
        self.rt.block_on(self.inner.team(id))
    }

    /// List projects (blocking).
    pub fn projects(&self) -> BlockingQuery<crate::generated::queries::ProjectsQuery<'_>> {
        BlockingQuery::new(self.inner.projects(), &self.rt)
    }

    /// Query a single project by ID (blocking).
    pub fn project(&self, id: String) -> Result<Project, LinearError> {
        self.rt.block_on(self.inner.project(id))
    }

    /// List issue labels (blocking).
    pub fn issue_labels(&self) -> BlockingQuery<crate::generated::queries::IssueLabelsQuery<'_>> {
        BlockingQuery::new(self.inner.issue_labels(), &self.rt)
    }

    /// List issues (blocking).
    pub fn issues(&self) -> BlockingQuery<crate::generated::queries::IssuesQuery<'_>> {
        BlockingQuery::new(self.inner.issues(), &self.rt)
    }

    /// Query a single issue by ID (blocking).
    pub fn issue(&self, id: String) -> Result<Issue, LinearError> {
        self.rt.block_on(self.inner.issue(id))
    }

    /// List cycles (blocking).
    pub fn cycles(&self) -> BlockingQuery<crate::generated::queries::CyclesQuery<'_>> {
        BlockingQuery::new(self.inner.cycles(), &self.rt)
    }

    /// Query a single cycle by ID (blocking).
    pub fn cycle(&self, id: String) -> Result<Cycle, LinearError> {
        self.rt.block_on(self.inner.cycle(id))
    }

    /// Search issues (blocking).
    pub fn search_issues(
        &self,
        term: impl Into<String>,
    ) -> BlockingQuery<crate::generated::queries::SearchIssuesQuery<'_>> {
        BlockingQuery::new(self.inner.search_issues(term), &self.rt)
    }

    /// List documents (blocking).
    pub fn documents(&self) -> BlockingQuery<crate::generated::queries::DocumentsQuery<'_>> {
        BlockingQuery::new(self.inner.documents(), &self.rt)
    }

    /// Query a single document by ID (blocking).
    pub fn document(&self, id: String) -> Result<Document, LinearError> {
        self.rt.block_on(self.inner.document(id))
    }

    /// List issue relations (blocking).
    pub fn issue_relations(
        &self,
    ) -> BlockingQuery<crate::generated::queries::IssueRelationsQuery<'_>> {
        BlockingQuery::new(self.inner.issue_relations(), &self.rt)
    }

    /// Query a single issue relation by ID (blocking).
    pub fn issue_relation(&self, id: String) -> Result<IssueRelation, LinearError> {
        self.rt.block_on(self.inner.issue_relation(id))
    }
}

// ── Generated mutation method wrappers ───────────────────────────────────────

use crate::generated::inputs::*;

/// Blocking equivalents of the generated mutation methods on [`Client`].
impl Client {
    /// Create a comment (blocking).
    pub fn comment_create(
        &self,
        input: CommentCreateInput,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.comment_create(input))
    }

    /// Create an issue (blocking).
    pub fn issue_create(&self, input: IssueCreateInput) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.issue_create(input))
    }

    /// Update an issue (blocking).
    pub fn issue_update(
        &self,
        input: IssueUpdateInput,
        id: String,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.issue_update(input, id))
    }

    /// Archive an issue (blocking).
    pub fn issue_archive(
        &self,
        trash: Option<bool>,
        id: String,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.issue_archive(trash, id))
    }

    /// Delete an issue (blocking).
    pub fn issue_delete(
        &self,
        permanently_delete: Option<bool>,
        id: String,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt
            .block_on(self.inner.issue_delete(permanently_delete, id))
    }

    /// Request a file upload URL (blocking).
    pub fn file_upload(
        &self,
        meta_data: Option<serde_json::Value>,
        make_public: Option<bool>,
        size: i64,
        content_type: String,
        filename: String,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.file_upload(
            meta_data,
            make_public,
            size,
            content_type,
            filename,
        ))
    }

    /// Upload an image from a URL (blocking).
    pub fn image_upload_from_url(&self, url: String) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.image_upload_from_url(url))
    }

    /// Create an issue relation (blocking).
    pub fn issue_relation_create(
        &self,
        override_created_at: Option<serde_json::Value>,
        input: IssueRelationCreateInput,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt
            .block_on(self.inner.issue_relation_create(override_created_at, input))
    }

    /// Create a document (blocking).
    pub fn document_create(
        &self,
        input: DocumentCreateInput,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.document_create(input))
    }

    /// Update a document (blocking).
    pub fn document_update(
        &self,
        input: DocumentUpdateInput,
        id: String,
    ) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.document_update(input, id))
    }

    /// Delete a document (blocking).
    pub fn document_delete(&self, id: String) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.document_delete(id))
    }
}

fn build_runtime() -> Result<tokio::runtime::Runtime, LinearError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| LinearError::AuthConfig(format!("Failed to create tokio runtime: {}", e)))
}
