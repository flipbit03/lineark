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
//! use lineark_sdk::blocking_client::Client;
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

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("blocking_client::Client")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
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
pub struct BlockingQuery<'rt, B> {
    builder: B,
    rt: &'rt tokio::runtime::Runtime,
}

impl<'rt, B> BlockingQuery<'rt, B> {
    fn new(builder: B, rt: &'rt tokio::runtime::Runtime) -> Self {
        Self { builder, rt }
    }
}

// Implement builder forwarding and send() for each generated query type.
// We use a macro to avoid repeating the pattern for every query builder.

macro_rules! blocking_query_builder {
    (
        query_type = $QueryType:ident,
        node_type = $NodeType:ty,
        return_type = $ReturnKind:ident < $ReturnType:ty >,
        methods = [ $( $method:ident ( $arg_ty:ty ) ),* $(,)? ]
    ) => {
        impl<'rt> BlockingQuery<'rt, crate::generated::queries::$QueryType<'_, $NodeType>> {
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
            self.rt.block_on(self.builder.send())
        }
    };
    // Single return type
    (@send Single<$T:ty>) => {
        /// Execute the query and return the result synchronously.
        pub fn send(self) -> Result<$T, LinearError> {
            self.rt.block_on(self.builder.send())
        }
    };
}

use crate::generated::types::*;

blocking_query_builder! {
    query_type = WorkflowStatesQueryBuilder,
    node_type = WorkflowState,
    return_type = Connection<WorkflowState>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = UsersQueryBuilder,
    node_type = User,
    return_type = Connection<User>,
    methods = [include_disabled(bool), before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = TeamsQueryBuilder,
    node_type = Team,
    return_type = Connection<Team>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = ProjectsQueryBuilder,
    node_type = Project,
    return_type = Connection<Project>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = IssueLabelsQueryBuilder,
    node_type = IssueLabel,
    return_type = Connection<IssueLabel>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = IssuesQueryBuilder,
    node_type = Issue,
    return_type = Connection<Issue>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = CyclesQueryBuilder,
    node_type = Cycle,
    return_type = Connection<Cycle>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = SearchIssuesQueryBuilder,
    node_type = IssueSearchResult,
    return_type = Connection<IssueSearchResult>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool), include_comments(bool), team_id(impl Into<String>)]
}

blocking_query_builder! {
    query_type = SearchDocumentsQueryBuilder,
    node_type = DocumentSearchResult,
    return_type = Connection<DocumentSearchResult>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool), include_comments(bool), team_id(impl Into<String>)]
}

blocking_query_builder! {
    query_type = SearchProjectsQueryBuilder,
    node_type = ProjectSearchResult,
    return_type = Connection<ProjectSearchResult>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool), include_comments(bool), team_id(impl Into<String>)]
}

blocking_query_builder! {
    query_type = DocumentsQueryBuilder,
    node_type = Document,
    return_type = Connection<Document>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = IssueRelationsQueryBuilder,
    node_type = IssueRelation,
    return_type = Connection<IssueRelation>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

blocking_query_builder! {
    query_type = ProjectMilestonesQueryBuilder,
    node_type = ProjectMilestone,
    return_type = Connection<ProjectMilestone>,
    methods = [before(impl Into<String>), after(impl Into<String>), first(i64), last(i64), include_archived(bool)]
}

/// Blocking equivalents of the generated query constructor methods on [`Client`].
impl Client {
    /// Query the authenticated user (blocking).
    pub fn whoami(&self) -> Result<User, LinearError> {
        self.rt.block_on(self.inner.whoami::<User>())
    }

    /// List workflow states (blocking).
    pub fn workflow_states(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::WorkflowStatesQueryBuilder<'_, WorkflowState>>
    {
        BlockingQuery::new(self.inner.workflow_states::<WorkflowState>(), &self.rt)
    }

    /// List users (blocking).
    pub fn users(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::UsersQueryBuilder<'_, User>> {
        BlockingQuery::new(self.inner.users::<User>(), &self.rt)
    }

    /// List teams (blocking).
    pub fn teams(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::TeamsQueryBuilder<'_, Team>> {
        BlockingQuery::new(self.inner.teams::<Team>(), &self.rt)
    }

    /// Query a single team by ID (blocking).
    pub fn team(&self, id: String) -> Result<Team, LinearError> {
        self.rt.block_on(self.inner.team::<Team>(id))
    }

    /// List projects (blocking).
    pub fn projects(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::ProjectsQueryBuilder<'_, Project>> {
        BlockingQuery::new(self.inner.projects::<Project>(), &self.rt)
    }

    /// Query a single project by ID (blocking).
    pub fn project(&self, id: String) -> Result<Project, LinearError> {
        self.rt.block_on(self.inner.project::<Project>(id))
    }

    /// List issue labels (blocking).
    pub fn issue_labels(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::IssueLabelsQueryBuilder<'_, IssueLabel>> {
        BlockingQuery::new(self.inner.issue_labels::<IssueLabel>(), &self.rt)
    }

    /// List issues (blocking).
    pub fn issues(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::IssuesQueryBuilder<'_, Issue>> {
        BlockingQuery::new(self.inner.issues::<Issue>(), &self.rt)
    }

    /// Query a single issue by ID (blocking).
    pub fn issue(&self, id: String) -> Result<Issue, LinearError> {
        self.rt.block_on(self.inner.issue::<Issue>(id))
    }

    /// List cycles (blocking).
    pub fn cycles(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::CyclesQueryBuilder<'_, Cycle>> {
        BlockingQuery::new(self.inner.cycles::<Cycle>(), &self.rt)
    }

    /// Query a single cycle by ID (blocking).
    pub fn cycle(&self, id: String) -> Result<Cycle, LinearError> {
        self.rt.block_on(self.inner.cycle::<Cycle>(id))
    }

    /// Search issues (blocking).
    pub fn search_issues(
        &self,
        term: impl Into<String>,
    ) -> BlockingQuery<'_, crate::generated::queries::SearchIssuesQueryBuilder<'_, IssueSearchResult>>
    {
        BlockingQuery::new(
            self.inner.search_issues::<IssueSearchResult>(term),
            &self.rt,
        )
    }

    /// Search documents (blocking).
    pub fn search_documents(
        &self,
        term: impl Into<String>,
    ) -> BlockingQuery<
        '_,
        crate::generated::queries::SearchDocumentsQueryBuilder<'_, DocumentSearchResult>,
    > {
        BlockingQuery::new(
            self.inner.search_documents::<DocumentSearchResult>(term),
            &self.rt,
        )
    }

    /// Search projects (blocking).
    pub fn search_projects(
        &self,
        term: impl Into<String>,
    ) -> BlockingQuery<
        '_,
        crate::generated::queries::SearchProjectsQueryBuilder<'_, ProjectSearchResult>,
    > {
        BlockingQuery::new(
            self.inner.search_projects::<ProjectSearchResult>(term),
            &self.rt,
        )
    }

    /// List documents (blocking).
    pub fn documents(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::DocumentsQueryBuilder<'_, Document>> {
        BlockingQuery::new(self.inner.documents::<Document>(), &self.rt)
    }

    /// Query a single document by ID (blocking).
    pub fn document(&self, id: String) -> Result<Document, LinearError> {
        self.rt.block_on(self.inner.document::<Document>(id))
    }

    /// List issue relations (blocking).
    pub fn issue_relations(
        &self,
    ) -> BlockingQuery<'_, crate::generated::queries::IssueRelationsQueryBuilder<'_, IssueRelation>>
    {
        BlockingQuery::new(self.inner.issue_relations::<IssueRelation>(), &self.rt)
    }

    /// Query a single issue relation by ID (blocking).
    pub fn issue_relation(&self, id: String) -> Result<IssueRelation, LinearError> {
        self.rt
            .block_on(self.inner.issue_relation::<IssueRelation>(id))
    }

    /// List project milestones (blocking).
    pub fn project_milestones(
        &self,
    ) -> BlockingQuery<
        '_,
        crate::generated::queries::ProjectMilestonesQueryBuilder<'_, ProjectMilestone>,
    > {
        BlockingQuery::new(
            self.inner.project_milestones::<ProjectMilestone>(),
            &self.rt,
        )
    }

    /// Query a single project milestone by ID (blocking).
    pub fn project_milestone(&self, id: String) -> Result<ProjectMilestone, LinearError> {
        self.rt
            .block_on(self.inner.project_milestone::<ProjectMilestone>(id))
    }
}

// ── Generated mutation method wrappers ───────────────────────────────────────

use crate::generated::inputs::*;

/// Blocking equivalents of the generated mutation methods on [`Client`].
impl Client {
    /// Create a comment (blocking).
    pub fn comment_create<T: DeserializeOwned + crate::GraphQLFields<FullType = Comment>>(
        &self,
        input: CommentCreateInput,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.comment_create::<T>(input))
    }

    /// Create an issue (blocking).
    pub fn issue_create<T: DeserializeOwned + crate::GraphQLFields<FullType = Issue>>(
        &self,
        input: IssueCreateInput,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.issue_create::<T>(input))
    }

    /// Update an issue (blocking).
    pub fn issue_update<T: DeserializeOwned + crate::GraphQLFields<FullType = Issue>>(
        &self,
        input: IssueUpdateInput,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.issue_update::<T>(input, id))
    }

    /// Archive an issue (blocking).
    pub fn issue_archive<T: DeserializeOwned + crate::GraphQLFields<FullType = Issue>>(
        &self,
        trash: Option<bool>,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.issue_archive::<T>(trash, id))
    }

    /// Unarchive an issue (blocking).
    pub fn issue_unarchive<T: DeserializeOwned + crate::GraphQLFields<FullType = Issue>>(
        &self,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.issue_unarchive::<T>(id))
    }

    /// Delete an issue (blocking).
    pub fn issue_delete<T: DeserializeOwned + crate::GraphQLFields<FullType = Issue>>(
        &self,
        permanently_delete: Option<bool>,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt
            .block_on(self.inner.issue_delete::<T>(permanently_delete, id))
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
    pub fn issue_relation_create<
        T: DeserializeOwned + crate::GraphQLFields<FullType = IssueRelation>,
    >(
        &self,
        override_created_at: Option<serde_json::Value>,
        input: IssueRelationCreateInput,
    ) -> Result<T, LinearError> {
        self.rt.block_on(
            self.inner
                .issue_relation_create::<T>(override_created_at, input),
        )
    }

    /// Create a document (blocking).
    pub fn document_create<T: DeserializeOwned + crate::GraphQLFields<FullType = Document>>(
        &self,
        input: DocumentCreateInput,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.document_create::<T>(input))
    }

    /// Update a document (blocking).
    pub fn document_update<T: DeserializeOwned + crate::GraphQLFields<FullType = Document>>(
        &self,
        input: DocumentUpdateInput,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.document_update::<T>(input, id))
    }

    /// Delete a document (blocking).
    pub fn document_delete<T: DeserializeOwned + crate::GraphQLFields<FullType = Document>>(
        &self,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt.block_on(self.inner.document_delete::<T>(id))
    }

    /// Create a project milestone (blocking).
    pub fn project_milestone_create<
        T: DeserializeOwned + crate::GraphQLFields<FullType = ProjectMilestone>,
    >(
        &self,
        input: ProjectMilestoneCreateInput,
    ) -> Result<T, LinearError> {
        self.rt
            .block_on(self.inner.project_milestone_create::<T>(input))
    }

    /// Update a project milestone (blocking).
    pub fn project_milestone_update<
        T: DeserializeOwned + crate::GraphQLFields<FullType = ProjectMilestone>,
    >(
        &self,
        input: ProjectMilestoneUpdateInput,
        id: String,
    ) -> Result<T, LinearError> {
        self.rt
            .block_on(self.inner.project_milestone_update::<T>(input, id))
    }

    /// Delete a project milestone (blocking).
    pub fn project_milestone_delete(&self, id: String) -> Result<serde_json::Value, LinearError> {
        self.rt.block_on(self.inner.project_milestone_delete(id))
    }
}

fn build_runtime() -> Result<tokio::runtime::Runtime, LinearError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| LinearError::Internal(format!("Failed to create tokio runtime: {}", e)))
}
