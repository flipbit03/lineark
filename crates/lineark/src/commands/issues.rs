use clap::Args;
use lineark_sdk::generated::inputs::{
    IssueCreateInput, IssueFilter, IssueUpdateInput, WorkflowStateFilter,
};
use lineark_sdk::generated::types::{
    Comment, CommentConnection, Issue, IssueConnection, IssueLabel, IssueLabelConnection,
    IssueRelation, IssueRelationConnection, IssueSearchResult, User, WorkflowState,
};
use lineark_sdk::{Client, GraphQLFields, MaybeUndefined};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{
    parse_priority, resolve_cycle_id, resolve_issue_id, resolve_label_ids, resolve_project_id,
    resolve_team_id, resolve_user_id_or_me,
};
use crate::output::{self, Format};

/// Manage issues.
#[derive(Debug, Args)]
pub struct IssuesCmd {
    #[command(subcommand)]
    pub action: IssuesAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum IssuesAction {
    /// List all issues across the workspace (all teams, statuses, and assignees), newest first. Use --mine to show only your issues. Done/canceled issues are hidden by default.
    List {
        /// Maximum number of issues to return (max 250).
        #[arg(short = 'l', long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Filter by team key, name, or UUID.
        #[arg(long)]
        team: Option<String>,
        /// Filter by project name or UUID.
        #[arg(long)]
        project: Option<String>,
        /// Show only issues assigned to the authenticated user.
        #[arg(long, default_value = "false")]
        mine: bool,
        /// Include done and canceled issues (hidden by default).
        #[arg(long, default_value = "false")]
        show_done: bool,
    },
    /// Show full details for a single issue, including assignee, state, project, cycle, parent, labels, description, sub-issues, and comments.
    Read {
        /// Issue identifier (e.g., E-929) or UUID.
        identifier: String,
    },
    /// Full-text search across issue titles and descriptions. Done/canceled issues are hidden by default.
    Search {
        /// Search query text.
        query: String,
        /// Maximum number of results (max 250).
        #[arg(short = 'l', long, default_value = "25", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Include done and canceled issues (hidden by default).
        #[arg(long, default_value = "false")]
        show_done: bool,
        /// Filter by team key, name, or UUID.
        #[arg(long)]
        team: Option<String>,
        /// Filter by assignee: user name, display name, UUID, or `me`.
        #[arg(long)]
        assignee: Option<String>,
        /// Filter by status names (comma-separated).
        #[arg(long, visible_alias = "state", value_delimiter = ',')]
        status: Option<Vec<String>>,
    },
    /// Create a new issue. Returns the created issue
    ///
    /// Examples:
    ///   lineark issues create "Fix the bug" --team ENG
    ///   lineark issues create "Add feature" --team ENG --priority high --description "Details here"
    ///   lineark issues create "Urgent fix" --team ENG -p urgent --labels Bug,Frontend
    ///   lineark issues create "My task" --team ENG --assignee me
    #[command(verbatim_doc_comment)]
    Create {
        /// Issue title.
        title: String,
        /// Team key, name, or UUID. Required.
        #[arg(long)]
        team: String,
        /// Assignee: user name, display name, UUID, or `me`.
        #[arg(long)]
        assignee: Option<String>,
        /// Comma-separated label names.
        #[arg(long, value_delimiter = ',')]
        labels: Option<Vec<String>>,
        /// Priority: 0-4 or none, urgent, high, medium, low.
        #[arg(short = 'p', long, value_parser = parse_priority)]
        priority: Option<i64>,
        /// Estimate points (valid values depend on the team's estimation scale).
        #[arg(short = 'e', long)]
        estimate: Option<i64>,
        /// Issue description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// Parent issue identifier (e.g., ENG-123) or UUID.
        #[arg(long)]
        parent: Option<String>,
        /// Initial status name (resolved against team's workflow states).
        #[arg(short = 's', long, visible_alias = "state")]
        status: Option<String>,
        /// Project name or UUID.
        #[arg(long)]
        project: Option<String>,
        /// Cycle name, number, or UUID (resolved within the team).
        #[arg(long)]
        cycle: Option<String>,
        /// Raw sortOrder value (Manual ordering, lower appears higher). Prefer `issues reorder`.
        #[arg(long, allow_negative_numbers = true, value_parser = parse_order_value)]
        sort_order: Option<f64>,
        /// Raw prioritySortOrder value (Priority ordering, within a priority group). Prefer `issues reorder`.
        #[arg(long, allow_negative_numbers = true, value_parser = parse_order_value)]
        priority_sort_order: Option<f64>,
        /// Raw subIssueSortOrder value (the parent's sub-issue list). Prefer `issues reorder`.
        #[arg(long, allow_negative_numbers = true, value_parser = parse_order_value)]
        sub_issue_sort_order: Option<f64>,
    },
    /// Archive an issue
    ///
    /// Examples:
    ///   lineark issues archive ENG-123
    #[command(verbatim_doc_comment)]
    Archive {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
    },
    /// Unarchive a previously archived issue
    ///
    /// Examples:
    ///   lineark issues unarchive ENG-123
    #[command(verbatim_doc_comment)]
    Unarchive {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
    },
    /// Find the issue associated with a Git branch name.
    FindBranch {
        /// Git branch name to search for.
        branch_name: String,
    },
    /// Delete (trash) an issue. Use --permanently to delete permanently
    ///
    /// Examples:
    ///   lineark issues delete ENG-123
    ///   lineark issues delete ENG-123 --permanently
    #[command(verbatim_doc_comment)]
    Delete {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
        /// Permanently delete the issue instead of trashing it.
        #[arg(long, default_value = "false")]
        permanently: bool,
    },
    /// Batch update multiple issues at once. Returns the updated issues
    ///
    /// Examples:
    ///   lineark issues batch-update ENG-1 ENG-2 --priority high
    ///   lineark issues batch-update ENG-1 ENG-2 ENG-3 --status "In Progress"
    ///   lineark issues batch-update ENG-1 ENG-2 --assignee me --labels Bug --label-by adding
    #[command(verbatim_doc_comment)]
    BatchUpdate {
        /// Issue identifiers (e.g., ENG-123) or UUIDs. At least one required.
        #[arg(required = true, num_args = 1..)]
        identifiers: Vec<String>,
        /// New status name (resolved against the first issue's team workflow states).
        #[arg(short = 's', long, visible_alias = "state")]
        status: Option<String>,
        /// Priority: 0-4 or none, urgent, high, medium, low.
        #[arg(short = 'p', long, value_parser = parse_priority)]
        priority: Option<i64>,
        /// Comma-separated label names. Behavior depends on --label-by.
        #[arg(long, value_delimiter = ',')]
        labels: Option<Vec<String>>,
        /// How to apply --labels: "replacing" (default), "adding", or "removing".
        #[arg(long, default_value = "replacing")]
        label_by: LabelMode,
        /// Remove all labels from the issues.
        #[arg(long, default_value = "false")]
        clear_labels: bool,
        /// Assignee: user name, display name, UUID, or `me`.
        #[arg(long)]
        assignee: Option<String>,
        /// Remove the assignee (unassign the issues).
        #[arg(long, default_value = "false", conflicts_with = "assignee")]
        clear_assignee: bool,
        /// Project name or UUID.
        #[arg(long)]
        project: Option<String>,
        /// Remove the issues from their project.
        #[arg(long, default_value = "false", conflicts_with = "project")]
        clear_project: bool,
        /// Cycle name, number, or UUID (resolved within the first issue's team).
        #[arg(long)]
        cycle: Option<String>,
        /// Remove the issues from their cycle.
        #[arg(long, default_value = "false", conflicts_with = "cycle")]
        clear_cycle: bool,
    },
    /// Update an existing issue. Returns the updated issue
    ///
    /// Examples:
    ///   lineark issues update ENG-123 --status "In Progress"
    ///   lineark issues update ENG-123 --priority urgent --assignee "John Doe"
    ///   lineark issues update ENG-123 --labels Bug,Frontend --label-by adding
    #[command(verbatim_doc_comment)]
    Update {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
        /// New status name (resolved against the issue's team workflow states).
        #[arg(short = 's', long, visible_alias = "state")]
        status: Option<String>,
        /// Priority: 0-4 or none, urgent, high, medium, low.
        #[arg(short = 'p', long, value_parser = parse_priority)]
        priority: Option<i64>,
        /// Estimate points (valid values depend on the team's estimation scale).
        #[arg(short = 'e', long)]
        estimate: Option<i64>,
        /// Comma-separated label names. Behavior depends on --label-by.
        #[arg(long, value_delimiter = ',')]
        labels: Option<Vec<String>>,
        /// How to apply --labels: "replacing" (default), "adding", or "removing".
        #[arg(long, default_value = "replacing")]
        label_by: LabelMode,
        /// Remove all labels from the issue.
        #[arg(long, default_value = "false")]
        clear_labels: bool,
        /// Assignee: user name, display name, UUID, or `me`.
        #[arg(long)]
        assignee: Option<String>,
        /// Remove the assignee (unassign the issue).
        #[arg(long, default_value = "false", conflicts_with = "assignee")]
        clear_assignee: bool,
        /// Parent issue identifier (e.g., ENG-123) or UUID.
        #[arg(long)]
        parent: Option<String>,
        /// Remove the parent issue relationship.
        #[arg(long, default_value = "false", conflicts_with = "parent")]
        clear_parent: bool,
        /// New title.
        #[arg(short = 't', long)]
        title: Option<String>,
        /// New description (markdown).
        #[arg(short = 'd', long)]
        description: Option<String>,
        /// Project name or UUID.
        #[arg(long)]
        project: Option<String>,
        /// Remove the issue from its project.
        #[arg(long, default_value = "false", conflicts_with = "project")]
        clear_project: bool,
        /// Cycle name, number, or UUID (resolved within the team).
        #[arg(long)]
        cycle: Option<String>,
        /// Remove the issue from its cycle.
        #[arg(long, default_value = "false", conflicts_with = "cycle")]
        clear_cycle: bool,
        /// Raw sortOrder value (Manual ordering, lower appears higher). Prefer `issues reorder`.
        #[arg(long, allow_negative_numbers = true, value_parser = parse_order_value)]
        sort_order: Option<f64>,
        /// Raw prioritySortOrder value (Priority ordering, within a priority group). Prefer `issues reorder`.
        #[arg(long, allow_negative_numbers = true, value_parser = parse_order_value)]
        priority_sort_order: Option<f64>,
        /// Raw subIssueSortOrder value (the parent's sub-issue list). Prefer `issues reorder`.
        #[arg(long, allow_negative_numbers = true, value_parser = parse_order_value)]
        sub_issue_sort_order: Option<f64>,
    },
    /// Put issues in the given order (first = top). Returns the issues with their new order values
    ///
    /// Linear has three independent order fields. A lower value appears higher.
    ///   sortOrder          Views with Ordering: Manual (what drag-and-drop changes).
    ///   prioritySortOrder  Views with Ordering: Priority, inside one priority group.
    ///   subIssueSortOrder  The sub-issue list on the parent issue's page.
    ///
    /// Default (--field all): set all three, so the order holds whatever ordering the
    /// viewer uses. The block starts at (or just after) the position of the topmost
    /// listed issue, with values 0.01 apart, so it stays where that issue was.
    /// subIssueSortOrder only orders siblings: listed issues that share a parent are
    /// ordered among themselves, and issues without a parent are skipped for it.
    /// A field already in the wanted order is left untouched.
    ///
    /// With --parent: reorder that parent's sub-issue list (subIssueSortOrder only).
    /// Listed children go first, in the given order. Children not listed follow, in
    /// their current relative order.
    ///
    /// Examples:
    ///   lineark issues reorder ENG-101 ENG-102 ENG-103
    ///   lineark issues reorder ENG-101 ENG-102 --field priority
    ///   lineark issues reorder --parent ENG-100 ENG-103 ENG-101 ENG-102
    #[command(verbatim_doc_comment)]
    Reorder {
        /// Issue identifiers (e.g., ENG-123) or UUIDs, in the wanted order (top first).
        #[arg(required = true, num_args = 1..)]
        identifiers: Vec<String>,
        /// Parent issue identifier or UUID: reorder its sub-issue list only.
        #[arg(long)]
        parent: Option<String>,
        /// Which order field to set. Default: all (sub-issue with --parent).
        #[arg(long, value_enum)]
        field: Option<OrderField>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LabelMode {
    /// Replace all existing labels with the specified ones.
    Replacing,
    /// Add labels to the existing set.
    Adding,
    /// Remove the specified labels from the existing set.
    Removing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OrderField {
    /// All three order fields.
    All,
    /// sortOrder: views with Ordering: Manual.
    Manual,
    /// prioritySortOrder: views with Ordering: Priority (within a priority group).
    Priority,
    /// subIssueSortOrder: the parent's sub-issue list.
    SubIssue,
}

// ── Flattened row (table display + JSON list output) ────────────────────────

#[derive(Debug, Serialize, Tabled)]
#[serde(rename_all = "camelCase")]
struct IssueRow {
    identifier: String,
    title: String,
    #[serde(rename = "priority")]
    #[tabled(rename = "priority")]
    priority_label: String,
    #[tabled(rename = "status")]
    state: String,
    assignee: String,
    team: String,
    estimate: String,
    labels: String,
    #[tabled(skip)]
    url: String,
    #[tabled(skip)]
    completed_at: Option<String>,
    #[tabled(skip)]
    canceled_at: Option<String>,
}

fn format_labels(labels: &Option<LabelConnection>) -> String {
    labels
        .as_ref()
        .map(|lc| {
            lc.nodes
                .iter()
                .filter_map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

impl From<&IssueSummary> for IssueRow {
    fn from(i: &IssueSummary) -> Self {
        Self {
            identifier: i.identifier.clone().unwrap_or_default(),
            title: i.title.clone().unwrap_or_default(),
            priority_label: i.priority_label.clone().unwrap_or_default(),
            state: i
                .state
                .as_ref()
                .and_then(|s| s.name.clone())
                .unwrap_or_default(),
            assignee: i
                .assignee
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_default(),
            team: i
                .team
                .as_ref()
                .and_then(|t| t.key.clone())
                .unwrap_or_default(),
            estimate: format_estimate(i.estimate),
            labels: format_labels(&i.labels),
            url: i.url.clone().unwrap_or_default(),
            completed_at: i.completed_at.clone(),
            canceled_at: i.canceled_at.clone(),
        }
    }
}

impl From<&SearchSummary> for IssueRow {
    fn from(i: &SearchSummary) -> Self {
        Self {
            identifier: i.identifier.clone().unwrap_or_default(),
            title: i.title.clone().unwrap_or_default(),
            priority_label: i.priority_label.clone().unwrap_or_default(),
            state: i
                .state
                .as_ref()
                .and_then(|s| s.name.clone())
                .unwrap_or_default(),
            assignee: i
                .assignee
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_default(),
            team: i
                .team
                .as_ref()
                .and_then(|t| t.key.clone())
                .unwrap_or_default(),
            estimate: format_estimate(i.estimate),
            labels: format_labels(&i.labels),
            url: i.url.clone().unwrap_or_default(),
            completed_at: i.completed_at.clone(),
            canceled_at: i.canceled_at.clone(),
        }
    }
}

// ── IssueSummary — lean type for `issues list` with nested fields ────────

/// Lean issue type for list views — scalars + the nested fields we display.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct IssueSummary {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    #[serde(rename(serialize = "priority"))]
    pub priority_label: Option<String>,
    pub estimate: Option<f64>,
    pub url: Option<String>,
    pub completed_at: Option<String>,
    pub canceled_at: Option<String>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
    #[graphql(nested)]
    pub assignee: Option<UserRef>,
    #[graphql(nested)]
    pub team: Option<TeamRef>,
    #[graphql(nested)]
    pub labels: Option<LabelConnection>,
}

/// Lean search result type for `issues search` with nested fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueSearchResult)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchSummary {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    #[serde(rename(serialize = "priority"))]
    pub priority_label: Option<String>,
    pub estimate: Option<f64>,
    pub url: Option<String>,
    pub completed_at: Option<String>,
    pub canceled_at: Option<String>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
    #[graphql(nested)]
    pub assignee: Option<UserRef>,
    #[graphql(nested)]
    pub team: Option<TeamRef>,
    #[graphql(nested)]
    pub labels: Option<LabelConnection>,
}

// ── IssueDetail — custom type for `issues read` with nested data ─────────

/// Lean issue type for `issues read` — scalars + the nested fields we display.
/// Zero-overfetch: the struct shape IS the query shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct IssueDetail {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename(serialize = "priority"))]
    pub priority_label: Option<String>,
    pub estimate: Option<f64>,
    pub url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub archived_at: Option<String>,
    pub completed_at: Option<String>,
    pub canceled_at: Option<String>,
    pub sort_order: Option<f64>,
    pub priority_sort_order: Option<f64>,
    pub sub_issue_sort_order: Option<f64>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
    #[graphql(nested)]
    pub assignee: Option<UserRef>,
    #[graphql(nested)]
    pub team: Option<TeamRef>,
    #[graphql(nested)]
    pub project: Option<ProjectRef>,
    #[graphql(nested)]
    pub project_milestone: Option<MilestoneRef>,
    #[graphql(nested)]
    pub cycle: Option<CycleRef>,
    #[graphql(nested)]
    pub parent: Option<RelatedIssueRef>,
    #[graphql(nested)]
    pub labels: Option<LabelConnection>,
    #[graphql(nested)]
    pub relations: Option<RelationConnection>,
    #[graphql(nested)]
    pub inverse_relations: Option<RelationConnection>,
    #[graphql(nested)]
    pub children: Option<ChildrenConnection>,
    #[graphql(nested)]
    pub comments: Option<CommentsConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = WorkflowState)]
#[serde(rename_all = "camelCase", default)]
pub struct StateRef {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
pub struct UserRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::Team)]
#[serde(rename_all = "camelCase", default)]
pub struct TeamRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::Project)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectRef {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::ProjectMilestone)]
#[serde(rename_all = "camelCase", default)]
pub struct MilestoneRef {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::Cycle)]
#[serde(rename_all = "camelCase", default)]
pub struct CycleRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub number: Option<f64>,
}

/// Wrapper around IssueLabelConnection that serializes as a flat list of names:
/// `["Bug", "Feature"]` instead of `{ "nodes": [{ "name": "Bug" }, ...] }`.
#[derive(Debug, Clone, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueLabelConnection)]
#[serde(rename_all = "camelCase", default)]
pub struct LabelConnection {
    #[graphql(nested)]
    pub nodes: Vec<LabelNameRef>,
}

impl Serialize for LabelConnection {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let names: Vec<&str> = self
            .nodes
            .iter()
            .filter_map(|l| l.name.as_deref())
            .collect();
        names.serialize(serializer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
pub struct LabelNameRef {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueRelationConnection)]
#[serde(rename_all = "camelCase", default)]
pub struct RelationConnection {
    #[graphql(nested)]
    pub nodes: Vec<RelationNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueRelation)]
#[serde(rename_all = "camelCase", default)]
pub struct RelationNode {
    pub id: Option<String>,
    pub r#type: Option<String>,
    #[graphql(nested)]
    pub issue: Option<RelatedIssueRef>,
    #[graphql(nested)]
    pub related_issue: Option<RelatedIssueRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct RelatedIssueRef {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
}

// ── Sub-issues (children) and comments for `issues read` ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueConnection)]
#[serde(rename_all = "camelCase", default)]
pub struct ChildrenConnection {
    #[graphql(nested)]
    pub nodes: Vec<ChildIssueRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct ChildIssueRef {
    pub identifier: Option<String>,
    pub title: Option<String>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = CommentConnection)]
#[serde(rename_all = "camelCase", default)]
pub struct CommentsConnection {
    #[graphql(nested)]
    pub nodes: Vec<CommentSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = Comment)]
#[serde(rename_all = "camelCase", default)]
pub struct CommentSummary {
    pub id: Option<String>,
    pub body: Option<String>,
    #[graphql(nested)]
    pub user: Option<UserRef>,
    pub created_at: Option<String>,
}

/// Lean result type for issue mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
struct IssueRef {
    id: Option<String>,
    identifier: Option<String>,
}

// ── Command dispatch ────────────────────────────────────────────────────────

pub async fn run(cmd: IssuesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        IssuesAction::List {
            limit,
            team,
            project,
            mine,
            show_done,
        } => {
            let mut filter_map = serde_json::Map::new();
            if !show_done {
                filter_map.insert(
                    "state".into(),
                    serde_json::json!({ "type": { "nin": ["completed", "canceled"] } }),
                );
            }
            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                filter_map.insert(
                    "team".into(),
                    serde_json::json!({ "id": { "eq": team_id } }),
                );
            }
            if let Some(ref project_val) = project {
                let project_id = resolve_project_id(client, project_val).await?;
                filter_map.insert(
                    "project".into(),
                    serde_json::json!({ "id": { "eq": project_id } }),
                );
            }
            if mine {
                let viewer = client
                    .whoami::<User>()
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                let viewer_id = viewer
                    .id
                    .ok_or_else(|| anyhow::anyhow!("Could not determine authenticated user ID"))?;
                filter_map.insert(
                    "assignee".into(),
                    serde_json::json!({ "id": { "eq": viewer_id } }),
                );
            }

            let filter: IssueFilter = serde_json::from_value(serde_json::Value::Object(filter_map))
                .expect("valid IssueFilter");

            let conn = client
                .issues::<IssueSummary>()
                .filter(filter)
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items: Vec<&IssueSummary> = conn.nodes.iter().collect();
            print_issue_list(&items, format);
        }
        IssuesAction::Read { identifier } => {
            let issue = read_issue(client, &identifier).await?;
            output::print_one(&issue, format);
        }
        IssuesAction::Search {
            query,
            limit,
            show_done,
            team,
            assignee,
            status,
        } => {
            let mut builder = client.search_issues::<SearchSummary>(query).first(limit);

            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                builder = builder.team_id(team_id);
            }

            // Build IssueFilter for assignee and/or status.
            let mut filter_map = serde_json::Map::new();
            if let Some(ref assignee_val) = assignee {
                let user_id = resolve_user_id_or_me(client, assignee_val).await?;
                filter_map.insert(
                    "assignee".into(),
                    serde_json::json!({ "id": { "eq": user_id } }),
                );
            }
            if let Some(ref status_names) = status {
                filter_map.insert(
                    "state".into(),
                    serde_json::json!({ "name": { "in": status_names } }),
                );
            }
            if !filter_map.is_empty() {
                let filter: IssueFilter =
                    serde_json::from_value(serde_json::Value::Object(filter_map))
                        .expect("valid IssueFilter");
                builder = builder.filter(filter);
            }

            let conn = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

            let items = filter_done_search(&conn.nodes, show_done);
            print_search_list(&items, format);
        }
        IssuesAction::FindBranch { branch_name } => {
            let result: Option<IssueDetail> = client
                .issue_vcs_branch_search(branch_name.clone())
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            match result {
                Some(issue) => output::print_one(&issue, format),
                None => {
                    return Err(anyhow::anyhow!(
                        "No issue found for branch '{}'",
                        branch_name
                    ))
                }
            }
        }
        IssuesAction::Create {
            title,
            team,
            assignee,
            labels,
            priority,
            estimate,
            description,
            parent,
            status,
            project,
            cycle,
            sort_order,
            priority_sort_order,
            sub_issue_sort_order,
        } => {
            let team_id = resolve_team_id(client, &team).await?;

            let assignee_id = match assignee {
                Some(ref a) => Some(resolve_user_id_or_me(client, a).await?),
                None => None,
            };

            let label_ids = match labels {
                Some(ref l) => Some(resolve_label_ids(client, l, Some(&team_id)).await?),
                None => None,
            };

            let state_id = match status {
                Some(ref name) => Some(resolve_state_id(client, &team_id, name).await?),
                None => None,
            };

            let parent_id = match parent {
                Some(ref p) => Some(resolve_issue_id(client, p).await?),
                None => None,
            };

            let project_id = match project {
                Some(ref p) => Some(resolve_project_id(client, p).await?),
                None => None,
            };

            let cycle_id = match cycle {
                Some(ref c) => Some(resolve_cycle_id(client, c, &team_id).await?),
                None => None,
            };

            let input = IssueCreateInput {
                title: title.into(),
                team_id,
                assignee_id: assignee_id.into(),
                label_ids: label_ids.into(),
                priority: priority.into(),
                estimate: estimate.into(),
                description: description.into(),
                parent_id: parent_id.into(),
                state_id: state_id.into(),
                project_id: project_id.into(),
                cycle_id: cycle_id.into(),
                sort_order: sort_order.into(),
                priority_sort_order: priority_sort_order.into(),
                sub_issue_sort_order: sub_issue_sort_order.into(),
                ..Default::default()
            };

            let issue = client
                .issue_create::<IssueRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Archive { identifier } => {
            let issue_id = resolve_issue_id(client, &identifier).await?;

            let issue = client
                .issue_archive::<IssueRef>(None, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Unarchive { identifier } => {
            let issue_id = resolve_issue_id(client, &identifier).await?;

            let issue = client
                .issue_unarchive::<IssueRef>(issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Delete {
            identifier,
            permanently,
        } => {
            let issue_id = resolve_issue_id(client, &identifier).await?;
            let permanently_delete = if permanently { Some(true) } else { None };

            let issue = client
                .issue_delete::<IssueRef>(permanently_delete, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::BatchUpdate {
            identifiers,
            status,
            priority,
            labels,
            label_by,
            clear_labels,
            assignee,
            clear_assignee,
            project,
            clear_project,
            cycle,
            clear_cycle,
        } => {
            if status.is_none()
                && priority.is_none()
                && labels.is_none()
                && !clear_labels
                && assignee.is_none()
                && !clear_assignee
                && project.is_none()
                && !clear_project
                && cycle.is_none()
                && !clear_cycle
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --status, --priority, --assignee, --labels, --project, --cycle, or a --clear-* flag to specify changes."
                ));
            }

            // Resolve all identifiers to UUIDs.
            let mut ids = Vec::with_capacity(identifiers.len());
            for ident in &identifiers {
                ids.push(resolve_issue_id(client, ident).await?);
            }

            // If we need to resolve status/labels/cycle, get the team from the first issue.
            let needs_team = status.is_some() || labels.is_some() || cycle.is_some();
            let team_id = if needs_team {
                let detail = read_issue(client, &identifiers[0]).await?;
                detail
                    .team
                    .as_ref()
                    .and_then(|t| t.id.clone())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Could not determine team for issue '{}'", identifiers[0])
                    })?
            } else {
                String::new()
            };

            let state_id = match status {
                Some(ref name) => Some(resolve_state_id(client, &team_id, name).await?),
                None => None,
            };

            let assignee_id = match assignee {
                Some(ref a) => Some(resolve_user_id_or_me(client, a).await?),
                None => None,
            };

            let project_id = match project {
                Some(ref p) => Some(resolve_project_id(client, p).await?),
                None => None,
            };

            let cycle_id = match cycle {
                Some(ref c) => Some(resolve_cycle_id(client, c, &team_id).await?),
                None => None,
            };

            // Build label fields based on mode.
            let (label_ids, added_label_ids, removed_label_ids) = if clear_labels {
                (Some(vec![]), None, None)
            } else if let Some(raw_labels) = labels {
                let tid = if needs_team {
                    Some(team_id.as_str())
                } else {
                    None
                };
                let resolved = resolve_label_ids(client, &raw_labels, tid).await?;
                match label_by {
                    LabelMode::Replacing => (Some(resolved), None, None),
                    LabelMode::Adding => (None, Some(resolved), None),
                    LabelMode::Removing => (None, None, Some(resolved)),
                }
            } else {
                (None, None, None)
            };

            let input = IssueUpdateInput {
                assignee_id: clear_or_set(clear_assignee, assignee_id),
                priority: priority.into(),
                state_id: state_id.into(),
                label_ids: label_ids.into(),
                added_label_ids: added_label_ids.into(),
                removed_label_ids: removed_label_ids.into(),
                project_id: clear_or_set(clear_project, project_id),
                cycle_id: clear_or_set(clear_cycle, cycle_id),
                ..Default::default()
            };

            let issues = client
                .issue_batch_update::<IssueSummary>(input, ids)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items: Vec<&IssueSummary> = issues.iter().collect();
            print_issue_list(&items, format);
        }
        IssuesAction::Update {
            identifier,
            status,
            priority,
            estimate,
            labels,
            label_by,
            clear_labels,
            assignee,
            clear_assignee,
            parent,
            clear_parent,
            title,
            description,
            project,
            clear_project,
            cycle,
            clear_cycle,
            sort_order,
            priority_sort_order,
            sub_issue_sort_order,
        } => {
            if status.is_none()
                && priority.is_none()
                && estimate.is_none()
                && labels.is_none()
                && !clear_labels
                && assignee.is_none()
                && !clear_assignee
                && parent.is_none()
                && !clear_parent
                && title.is_none()
                && description.is_none()
                && project.is_none()
                && !clear_project
                && cycle.is_none()
                && !clear_cycle
                && sort_order.is_none()
                && priority_sort_order.is_none()
                && sub_issue_sort_order.is_none()
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --status, --priority, --estimate, --assignee, --labels, --title, --description, --parent, --project, --cycle, --*-sort-order, or a --clear-* flag to specify changes."
                ));
            }

            let issue_id = resolve_issue_id(client, &identifier).await?;

            // Read the issue to get the team ID (needed for status/labels/cycle resolution).
            let needs_team = status.is_some() || labels.is_some() || cycle.is_some();
            let issue_detail = if needs_team {
                Some(read_issue(client, &identifier).await?)
            } else {
                None
            };
            let team_id = issue_detail
                .as_ref()
                .and_then(|d| d.team.as_ref())
                .and_then(|t| t.id.clone());

            let state_id = match status {
                Some(ref name) => {
                    let tid = team_id.as_deref().ok_or_else(|| {
                        anyhow::anyhow!("Could not determine team for issue '{}'", identifier)
                    })?;
                    Some(resolve_state_id(client, tid, name).await?)
                }
                None => None,
            };

            let assignee_id = match assignee {
                Some(ref a) => Some(resolve_user_id_or_me(client, a).await?),
                None => None,
            };

            let parent_id = match parent {
                Some(ref p) => Some(resolve_issue_id(client, p).await?),
                None => None,
            };

            let project_id = match project {
                Some(ref p) => Some(resolve_project_id(client, p).await?),
                None => None,
            };

            let cycle_id = match cycle {
                Some(ref c) => {
                    let tid = team_id.as_deref().ok_or_else(|| {
                        anyhow::anyhow!("Could not determine team for issue '{}'", identifier)
                    })?;
                    Some(resolve_cycle_id(client, c, tid).await?)
                }
                None => None,
            };

            // Build label fields based on mode.
            let (label_ids, added_label_ids, removed_label_ids) = if clear_labels {
                (Some(vec![]), None, None)
            } else if let Some(raw_labels) = labels {
                let resolved = resolve_label_ids(client, &raw_labels, team_id.as_deref()).await?;
                match label_by {
                    LabelMode::Replacing => (Some(resolved), None, None),
                    LabelMode::Adding => (None, Some(resolved), None),
                    LabelMode::Removing => (None, None, Some(resolved)),
                }
            } else {
                (None, None, None)
            };

            let input = IssueUpdateInput {
                title: title.into(),
                description: description.into(),
                assignee_id: clear_or_set(clear_assignee, assignee_id),
                priority: priority.into(),
                estimate: estimate.into(),
                state_id: state_id.into(),
                parent_id: clear_or_set(clear_parent, parent_id),
                label_ids: label_ids.into(),
                added_label_ids: added_label_ids.into(),
                removed_label_ids: removed_label_ids.into(),
                project_id: clear_or_set(clear_project, project_id),
                cycle_id: clear_or_set(clear_cycle, cycle_id),
                sort_order: sort_order.into(),
                priority_sort_order: priority_sort_order.into(),
                sub_issue_sort_order: sub_issue_sort_order.into(),
                ..Default::default()
            };

            let issue = client
                .issue_update::<IssueRef>(input, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Reorder {
            identifiers,
            parent,
            field,
        } => {
            let issues = match parent {
                Some(ref p) => reorder_sub_issues(client, p, &identifiers, field).await?,
                None => reorder_issues(client, &identifiers, field).await?,
            };
            output::print_table(&issues, format);
        }
    }
    Ok(())
}

/// Explicit null when clearing, the resolved ID when set, undefined otherwise.
fn clear_or_set(clear: bool, id: Option<String>) -> MaybeUndefined<String> {
    if clear {
        MaybeUndefined::Null
    } else {
        id.into()
    }
}

// TODO(phase2): query workflowStates types instead of hardcoding state names
const DONE_STATES: &[&str] = &["Done", "Canceled", "Cancelled", "Duplicate"];

fn format_estimate(estimate: Option<f64>) -> String {
    match estimate {
        Some(v) if v.fract() == 0.0 => format!("{}", v as i64),
        Some(v) => format!("{v}"),
        None => String::new(),
    }
}

fn print_issue_list(items: &[&IssueSummary], format: Format) {
    let rows: Vec<IssueRow> = items.iter().map(|i| IssueRow::from(*i)).collect();
    output::print_table(&rows, format);
}

fn filter_done_search(items: &[SearchSummary], show_done: bool) -> Vec<&SearchSummary> {
    if show_done {
        items.iter().collect()
    } else {
        items
            .iter()
            .filter(|i| {
                let state = i
                    .state
                    .as_ref()
                    .and_then(|s| s.name.as_deref())
                    .unwrap_or("");
                !DONE_STATES.iter().any(|d| d.eq_ignore_ascii_case(state))
            })
            .collect()
    }
}

fn print_search_list(items: &[&SearchSummary], format: Format) {
    let rows: Vec<IssueRow> = items.iter().map(|i| IssueRow::from(*i)).collect();
    output::print_table(&rows, format);
}

/// Read a single issue by identifier (e.g. E-929) or UUID, with full nested details.
/// Uses [`IssueDetail`] — a custom type with exactly the nested fields we display.
async fn read_issue(client: &Client, identifier: &str) -> anyhow::Result<IssueDetail> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        return client
            .issue::<IssueDetail>(identifier.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e));
    }
    if identifier.contains('-') {
        // searchIssues is fuzzy — search with full type, find the UUID, then fetch details.
        let conn = client
            .search_issues::<IssueSearchResult>(identifier)
            .first(5)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let id = conn
            .nodes
            .iter()
            .find(|issue| {
                issue
                    .identifier
                    .as_deref()
                    .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
            })
            .and_then(|n| n.id.clone())
            .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))?;
        client
            .issue::<IssueDetail>(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    } else {
        client
            .issue::<IssueDetail>(identifier.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

/// Resolve a workflow state name to its UUID for a given team.
async fn resolve_state_id(
    client: &Client,
    team_id: &str,
    state_name: &str,
) -> anyhow::Result<String> {
    let filter: WorkflowStateFilter =
        serde_json::from_value(serde_json::json!({ "team": { "id": { "eq": team_id } } }))
            .expect("valid WorkflowStateFilter");

    let conn = client
        .workflow_states::<WorkflowState>()
        .filter(filter)
        .first(50)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for node in &conn.nodes {
        let name = node.name.as_deref().unwrap_or("");
        if name.eq_ignore_ascii_case(state_name) {
            return Ok(node.id.clone().unwrap_or_default());
        }
    }
    let available: Vec<String> = conn.nodes.iter().filter_map(|n| n.name.clone()).collect();
    Err(anyhow::anyhow!(
        "Status '{}' not found for this team. Available: {}",
        state_name,
        available.join(", ")
    ))
}

// ── Reorder ─────────────────────────────────────────────────────────────────

/// Step between consecutive issues when anchoring at the group's current minimum.
/// sortOrder and prioritySortOrder are shared across the team, and neighbouring
/// issues are often less than 100 apart, so a small step keeps the block contiguous.
const REORDER_STEP: f64 = 0.01;

/// Step for a parent's sub-issue list. Every child gets rewritten, so absolute values are safe.
const SUB_ISSUE_STEP: f64 = 1000.0;

/// Random offsets to try before giving up on avoiding ties.
const TIE_ATTEMPTS: usize = 32;

/// Lean issue type for `issues reorder`: identity plus the three order fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Tabled, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct IssueOrder {
    #[tabled(skip)]
    pub id: Option<String>,
    #[tabled(display_with = "output::display_opt")]
    pub identifier: Option<String>,
    #[tabled(display_with = "output::display_opt")]
    pub title: Option<String>,
    #[tabled(rename = "sortOrder", display_with = "output::display_opt")]
    pub sort_order: Option<f64>,
    #[tabled(rename = "prioritySortOrder", display_with = "output::display_opt")]
    pub priority_sort_order: Option<f64>,
    #[tabled(rename = "subIssueSortOrder", display_with = "output::display_opt")]
    pub sub_issue_sort_order: Option<f64>,
    #[serde(skip_serializing)]
    #[tabled(skip)]
    #[graphql(nested)]
    pub parent: Option<IssueIdRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct IssueIdRef {
    pub id: Option<String>,
}

/// A parent issue with the first page of its children, for `issues reorder --parent`.
#[derive(Debug, Clone, Default, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
struct ParentWithChildren {
    id: Option<String>,
    identifier: Option<String>,
    #[graphql(nested)]
    children: Option<ChildOrderConnection>,
}

#[derive(Debug, Clone, Default, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueConnection)]
#[serde(rename_all = "camelCase", default)]
struct ChildOrderConnection {
    #[graphql(nested)]
    nodes: Vec<IssueOrder>,
}

impl IssueOrder {
    fn parent_id(&self) -> Option<&str> {
        self.parent.as_ref().and_then(|p| p.id.as_deref())
    }

    fn label(&self) -> String {
        self.identifier
            .clone()
            .or_else(|| self.id.clone())
            .unwrap_or_default()
    }
}

/// One of Linear's three order fields on `Issue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrderKey {
    Manual,
    Priority,
    SubIssue,
}

impl OrderKey {
    fn name(self) -> &'static str {
        match self {
            OrderKey::Manual => "sortOrder",
            OrderKey::Priority => "prioritySortOrder",
            OrderKey::SubIssue => "subIssueSortOrder",
        }
    }

    fn get(self, issue: &IssueOrder) -> Option<f64> {
        match self {
            OrderKey::Manual => issue.sort_order,
            OrderKey::Priority => issue.priority_sort_order,
            OrderKey::SubIssue => issue.sub_issue_sort_order,
        }
    }

    fn set(self, input: &mut IssueUpdateInput, value: f64) {
        let value = MaybeUndefined::Value(value);
        match self {
            OrderKey::Manual => input.sort_order = value,
            OrderKey::Priority => input.priority_sort_order = value,
            OrderKey::SubIssue => input.sub_issue_sort_order = value,
        }
    }
}

impl OrderField {
    fn keys(self) -> &'static [OrderKey] {
        match self {
            OrderField::All => &[OrderKey::Manual, OrderKey::Priority, OrderKey::SubIssue],
            OrderField::Manual => &[OrderKey::Manual],
            OrderField::Priority => &[OrderKey::Priority],
            OrderField::SubIssue => &[OrderKey::SubIssue],
        }
    }
}

/// Parse a raw order value, rejecting NaN and infinity (serde_json would send them as null).
fn parse_order_value(s: &str) -> Result<f64, String> {
    match s.trim().parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(format!("'{s}' is not a finite number")),
    }
}

/// Round to 6 decimals to drop binary noise (`-3189.9849999999997`); the step is far coarser.
fn tidy(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// A uniform random number in [0, 1), from the std library's randomly keyed hasher.
fn random_fraction() -> f64 {
    use std::hash::BuildHasher;
    let bits = std::collections::hash_map::RandomState::new().hash_one(0u8);
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

/// New values for one group of issues, given in the wanted order.
#[derive(Debug, PartialEq)]
struct GroupValues {
    values: Vec<f64>,
    /// False when every offset tried tied with another issue's current value.
    tie_free: bool,
}

/// Plan the values of one group of issues (`current`, in the wanted order).
///
/// Returns `None` when the current values already ascend in that order. Otherwise the
/// values are `base + offset + i * step`, with `offset` a random fraction of a step.
/// The offset is chosen so that no value ties the current value of another issue in
/// the group: Linear moves an issue elsewhere when its new prioritySortOrder or
/// subIssueSortOrder ties one that another issue still holds, and the writes are
/// sequential. The random offset also keeps runs off a shared grid, so a later run
/// is unlikely to tie with issues an earlier run placed and this one doesn't list.
fn plan_group_values(
    current: &[Option<f64>],
    base: f64,
    step: f64,
    mut random: impl FnMut() -> f64,
) -> anyhow::Result<Option<GroupValues>> {
    let ascending = current.windows(2).all(|w| match (w[0], w[1]) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    });
    if ascending && current.iter().all(Option::is_some) {
        return Ok(None);
    }

    let candidate = |offset: f64| -> Vec<f64> {
        (0..current.len())
            .map(|i| tidy(base + offset + i as f64 * step))
            .collect()
    };
    let ties = |values: &[f64]| {
        values.iter().enumerate().any(|(i, v)| {
            current
                .iter()
                .enumerate()
                .any(|(j, c)| i != j && *c == Some(*v))
        })
    };

    let mut values = candidate(0.0);
    let mut tie_free = false;
    for _ in 0..TIE_ATTEMPTS {
        values = candidate(random() * step);
        if !ties(&values) {
            tie_free = true;
            break;
        }
    }

    if !values.windows(2).all(|w| w[0] < w[1]) || values.iter().any(|v| !v.is_finite()) {
        return Err(anyhow::anyhow!(
            "Cannot space issues {step} apart around {base}: the values are too large for float precision. Set them with `issues update --sort-order` etc. instead."
        ));
    }
    Ok(Some(GroupValues { values, tie_free }))
}

/// Indexes of `issues` grouped by parent, in first-appearance order. Issues without a
/// parent are left out: Linear keeps their subIssueSortOrder null.
fn group_by_parent(issues: &[IssueOrder]) -> Vec<Vec<usize>> {
    let mut groups: Vec<(&str, Vec<usize>)> = Vec::new();
    for (i, issue) in issues.iter().enumerate() {
        let Some(parent) = issue.parent_id() else {
            continue;
        };
        match groups.iter_mut().find(|(p, _)| *p == parent) {
            Some((_, members)) => members.push(i),
            None => groups.push((parent, vec![i])),
        }
    }
    groups.into_iter().map(|(_, members)| members).collect()
}

/// Order `children` so that `listed` (indexes into `children`) come first, in the given
/// order, then the rest sorted by their current subIssueSortOrder (unset last).
fn sub_issue_order(children: &[IssueOrder], listed: &[usize]) -> Vec<usize> {
    let mut rest: Vec<usize> = (0..children.len())
        .filter(|i| !listed.contains(i))
        .collect();
    rest.sort_by(|&a, &b| {
        let (a, b) = (
            children[a].sub_issue_sort_order,
            children[b].sub_issue_sort_order,
        );
        match (a, b) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    listed.iter().copied().chain(rest).collect()
}

/// The issues to write, in output order, and the order each group must end up in.
#[derive(Default)]
struct ReorderPlan {
    issues: Vec<IssueOrder>,
    /// New values per issue (same indexes as `issues`).
    values: Vec<Vec<(OrderKey, f64)>>,
    /// Groups of issue indexes that must ascend on a key after the writes.
    groups: Vec<(OrderKey, Vec<usize>)>,
}

impl ReorderPlan {
    fn new(issues: Vec<IssueOrder>) -> Self {
        let values = vec![Vec::new(); issues.len()];
        Self {
            issues,
            values,
            groups: Vec::new(),
        }
    }

    /// Plan `key` for `members` (indexes, in the wanted order), starting at `base`
    /// (the group's current minimum when `None`).
    fn plan_group(
        &mut self,
        key: OrderKey,
        members: Vec<usize>,
        base: Option<f64>,
        step: f64,
    ) -> anyhow::Result<()> {
        if members.len() < 2 {
            return Ok(());
        }
        let current: Vec<Option<f64>> = members.iter().map(|&i| key.get(&self.issues[i])).collect();
        let base = base.unwrap_or_else(|| {
            current
                .iter()
                .flatten()
                .copied()
                .reduce(f64::min)
                .unwrap_or(0.0)
        });
        if let Some(planned) = plan_group_values(&current, base, step, random_fraction)? {
            if !planned.tie_free {
                eprintln!(
                    "Warning: could not avoid a {} tie with another issue; Linear may place one issue slightly off. Re-run to retry.",
                    key.name()
                );
            }
            for (&i, value) in members.iter().zip(planned.values) {
                self.values[i].push((key, value));
            }
        }
        self.groups.push((key, members));
        Ok(())
    }

    /// Whether issue `i` has a planned value that differs from its current one.
    fn changes(&self, i: usize) -> bool {
        self.values[i]
            .iter()
            .any(|&(key, value)| key.get(&self.issues[i]) != Some(value))
    }
}

/// `issues reorder A B C`: anchor each selected field at the group's current minimum.
async fn reorder_issues(
    client: &Client,
    identifiers: &[String],
    field: Option<OrderField>,
) -> anyhow::Result<Vec<IssueOrder>> {
    if identifiers.len() < 2 {
        return Err(anyhow::anyhow!(
            "Give at least two issues to reorder, or use --parent to move a sub-issue to the top of its parent's list."
        ));
    }

    let mut issues: Vec<IssueOrder> = Vec::with_capacity(identifiers.len());
    for ident in identifiers {
        // issue(id:) accepts identifiers (ENG-123) as well as UUIDs, with no search lag.
        let issue = client
            .issue::<IssueOrder>(ident.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Issue '{}': {}", ident, e))?;
        if issues.iter().any(|i| i.id == issue.id) {
            return Err(anyhow::anyhow!(
                "Issue '{}' is listed more than once",
                ident
            ));
        }
        issues.push(issue);
    }

    let mut plan = ReorderPlan::new(issues);
    for &key in field.unwrap_or(OrderField::All).keys() {
        if key == OrderKey::SubIssue {
            // subIssueSortOrder only orders siblings: order each parent's listed children
            // among themselves.
            let groups = group_by_parent(&plan.issues);
            if field == Some(OrderField::SubIssue) && groups.iter().all(|g| g.len() < 2) {
                return Err(anyhow::anyhow!(
                    "No two of these issues share a parent, so there is no sub-issue list to order. Use --parent to move a sub-issue within its parent's list."
                ));
            }
            for members in groups {
                plan.plan_group(key, members, None, REORDER_STEP)?;
            }
        } else {
            let members = (0..plan.issues.len()).collect();
            plan.plan_group(key, members, None, REORDER_STEP)?;
        }
    }

    apply_reorder(client, plan).await
}

/// `issues reorder --parent P A B`: rewrite the subIssueSortOrder of all of P's children.
async fn reorder_sub_issues(
    client: &Client,
    parent: &str,
    identifiers: &[String],
    field: Option<OrderField>,
) -> anyhow::Result<Vec<IssueOrder>> {
    if !matches!(field, None | Some(OrderField::SubIssue)) {
        return Err(anyhow::anyhow!(
            "--parent reorders the parent's sub-issue list (subIssueSortOrder) only. Drop --field, or use --field sub-issue."
        ));
    }

    let parent_issue = client
        .issue::<ParentWithChildren>(parent.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("Parent issue '{}': {}", parent, e))?;
    let parent_id = parent_issue
        .id
        .ok_or_else(|| anyhow::anyhow!("Parent issue '{}' not found", parent))?;
    let parent_label = parent_issue
        .identifier
        .unwrap_or_else(|| parent.to_string());

    // Check membership on each listed issue itself: the children query below goes
    // through a filter, which can lag behind a just-created child.
    let mut listed_issues: Vec<IssueOrder> = Vec::with_capacity(identifiers.len());
    for ident in identifiers {
        let issue = client
            .issue::<IssueOrder>(ident.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Issue '{}': {}", ident, e))?;
        if issue.parent_id() != Some(parent_id.as_str()) {
            return Err(anyhow::anyhow!(
                "Issue '{}' is not a sub-issue of {}",
                ident,
                parent_label
            ));
        }
        if listed_issues.iter().any(|i| i.id == issue.id) {
            return Err(anyhow::anyhow!(
                "Issue '{}' is listed more than once",
                ident
            ));
        }
        listed_issues.push(issue);
    }

    // Union of the parent's children connection (first page) and the paginated filter
    // query, so a child missing from one source is still rewritten.
    let mut children = fetch_children(client, &parent_id).await?;
    for child in parent_issue.children.map(|c| c.nodes).unwrap_or_default() {
        if !children.iter().any(|c| c.id == child.id) {
            children.push(child);
        }
    }
    let mut listed: Vec<usize> = Vec::with_capacity(listed_issues.len());
    for issue in listed_issues {
        let idx = match children.iter().position(|c| c.id == issue.id) {
            Some(idx) => idx,
            None => {
                children.push(issue);
                children.len() - 1
            }
        };
        listed.push(idx);
    }

    let order = sub_issue_order(&children, &listed);
    let ordered: Vec<IssueOrder> = order.into_iter().map(|i| children[i].clone()).collect();
    let mut plan = ReorderPlan::new(ordered);
    let members = (0..plan.issues.len()).collect();
    plan.plan_group(
        OrderKey::SubIssue,
        members,
        Some(SUB_ISSUE_STEP),
        SUB_ISSUE_STEP,
    )?;

    apply_reorder(client, plan).await
}

/// All non-archived children of a parent issue, following pagination.
async fn fetch_children(client: &Client, parent_id: &str) -> anyhow::Result<Vec<IssueOrder>> {
    let filter: IssueFilter =
        serde_json::from_value(serde_json::json!({ "parent": { "id": { "eq": parent_id } } }))
            .expect("valid IssueFilter");
    let mut children = Vec::new();
    let mut after: Option<String> = None;
    loop {
        let mut builder = client
            .issues::<IssueOrder>()
            .filter(filter.clone())
            .first(250);
        if let Some(cursor) = after.take() {
            builder = builder.after(cursor);
        }
        let conn = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        children.extend(conn.nodes);
        match conn.page_info.end_cursor {
            Some(cursor) if conn.page_info.has_next_page => after = Some(cursor),
            _ => break,
        }
    }
    Ok(children)
}

/// Write the planned values (one `issueUpdate` per issue whose values change), then read
/// every issue back and check that each planned group now ascends in the wanted order.
async fn apply_reorder(client: &Client, plan: ReorderPlan) -> anyhow::Result<Vec<IssueOrder>> {
    let pending: Vec<usize> = (0..plan.issues.len())
        .filter(|&i| plan.changes(i))
        .collect();
    for (n, &i) in pending.iter().enumerate() {
        let mut input = IssueUpdateInput::default();
        for &(key, value) in &plan.values[i] {
            key.set(&mut input, value);
        }
        let id = plan.issues[i].id.clone().unwrap_or_default();
        if let Err(e) = client.issue_update::<IssueRef>(input, id).await {
            let names = |indexes: &[usize]| -> String {
                if indexes.is_empty() {
                    return "none".to_string();
                }
                indexes
                    .iter()
                    .map(|&j| plan.issues[j].label())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            return Err(anyhow::anyhow!(
                "Failed to update {}: {}\nAlready updated: {}\nNot updated: {}\nRe-run the same command to finish.",
                plan.issues[i].label(),
                e,
                names(&pending[..n]),
                names(&pending[n..])
            ));
        }
    }

    let mut result = Vec::with_capacity(plan.issues.len());
    for issue in &plan.issues {
        let id = issue.id.clone().unwrap_or_default();
        let fresh = client
            .issue::<IssueOrder>(id)
            .await
            .map_err(|e| anyhow::anyhow!("Reading back {}: {}", issue.label(), e))?;
        result.push(fresh);
    }

    for (key, members) in &plan.groups {
        let values: Vec<Option<f64>> = members.iter().map(|&i| key.get(&result[i])).collect();
        let ordered = values.windows(2).all(|w| match (w[0], w[1]) {
            (Some(a), Some(b)) => a < b,
            _ => false,
        });
        if !ordered {
            let got: Vec<String> = members
                .iter()
                .map(|&i| {
                    format!(
                        "{}={}",
                        result[i].label(),
                        output::display_opt(&key.get(&result[i]))
                    )
                })
                .collect();
            return Err(anyhow::anyhow!(
                "Verification failed: {} is not in the wanted order after the update: {}",
                key.name(),
                got.join(", ")
            ));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_priority_numeric_values() {
        assert_eq!(parse_priority("0").unwrap(), 0);
        assert_eq!(parse_priority("1").unwrap(), 1);
        assert_eq!(parse_priority("4").unwrap(), 4);
    }

    #[test]
    fn parse_priority_textual_values() {
        assert_eq!(parse_priority("none").unwrap(), 0);
        assert_eq!(parse_priority("urgent").unwrap(), 1);
        assert_eq!(parse_priority("high").unwrap(), 2);
        assert_eq!(parse_priority("medium").unwrap(), 3);
        assert_eq!(parse_priority("normal").unwrap(), 3);
        assert_eq!(parse_priority("low").unwrap(), 4);
    }

    #[test]
    fn parse_priority_case_insensitive() {
        assert_eq!(parse_priority("URGENT").unwrap(), 1);
        assert_eq!(parse_priority("High").unwrap(), 2);
        assert_eq!(parse_priority("LOW").unwrap(), 4);
    }

    #[test]
    fn parse_priority_trims_whitespace() {
        assert_eq!(parse_priority(" urgent ").unwrap(), 1);
        assert_eq!(parse_priority(" 2 ").unwrap(), 2);
        assert!(parse_priority("  ").is_err());
    }

    #[test]
    fn parse_priority_rejects_invalid() {
        assert!(parse_priority("bogus").is_err());
        assert!(parse_priority("5").is_err());
        assert!(parse_priority("-1").is_err());
    }

    #[test]
    fn format_estimate_none_returns_empty() {
        assert_eq!(format_estimate(None), "");
    }

    #[test]
    fn format_estimate_whole_number_omits_decimal() {
        assert_eq!(format_estimate(Some(3.0)), "3");
        assert_eq!(format_estimate(Some(0.0)), "0");
    }

    #[test]
    fn format_estimate_fractional_preserves_decimal() {
        assert_eq!(format_estimate(Some(1.5)), "1.5");
        assert_eq!(format_estimate(Some(0.5)), "0.5");
    }

    fn issue(parent: Option<&str>, sub_issue_sort_order: Option<f64>) -> IssueOrder {
        IssueOrder {
            sub_issue_sort_order,
            parent: parent.map(|p| IssueIdRef {
                id: Some(p.to_string()),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn plan_group_values_skips_groups_already_in_order() {
        let planned = plan_group_values(&[Some(1.0), Some(2.0)], 1.0, 0.01, || 0.5).unwrap();
        assert_eq!(planned, None);
    }

    #[test]
    fn plan_group_values_anchors_at_base_plus_offset() {
        let planned = plan_group_values(&[Some(5.0), Some(-3.0), None], -3.0, 0.5, || 0.5)
            .unwrap()
            .unwrap();
        assert_eq!(planned.values, vec![-2.75, -2.25, -1.75]);
        assert!(planned.tie_free);
    }

    #[test]
    fn plan_group_values_detect_tie_with_the_minimum_holder() {
        // Offset 0 would give the first issue -3.0 while the second still holds it.
        let planned = plan_group_values(&[Some(5.0), Some(-3.0)], -3.0, 0.5, || 0.0)
            .unwrap()
            .unwrap();
        assert!(!planned.tie_free);
    }

    #[test]
    fn plan_group_values_have_no_binary_noise() {
        let planned = plan_group_values(&[None, None, None], -3190.0, 0.01, || 0.0)
            .unwrap()
            .unwrap();
        assert_eq!(planned.values, vec![-3190.0, -3189.99, -3189.98]);
    }

    #[test]
    fn plan_group_values_retries_offsets_that_tie() {
        // Re-run with the order reversed: offset 0 ties every value with another issue's.
        let mut offsets = vec![0.25, 0.0].into_iter();
        let planned = plan_group_values(
            &[Some(3000.0), Some(2000.0), Some(1000.0)],
            1000.0,
            1000.0,
            || offsets.next_back().unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(planned.values, vec![1250.0, 2250.0, 3250.0]);
        assert!(planned.tie_free);
    }

    #[test]
    fn plan_group_values_ignores_an_issues_own_value() {
        let planned = plan_group_values(&[Some(1000.0), Some(5.0)], 1000.0, 1000.0, || 0.0)
            .unwrap()
            .unwrap();
        assert_eq!(planned.values, vec![1000.0, 2000.0]);
        assert!(planned.tie_free);
    }

    #[test]
    fn plan_group_values_reports_unavoidable_ties() {
        let planned = plan_group_values(&[Some(2.0), Some(1.0)], 1.0, 1.0, || 0.0)
            .unwrap()
            .unwrap();
        assert!(!planned.tie_free);
    }

    #[test]
    fn plan_group_values_rejects_lost_precision() {
        assert!(plan_group_values(&[Some(2e17), Some(1e17)], 1e17, 0.01, || 0.0).is_err());
    }

    #[test]
    fn random_fraction_is_in_unit_interval() {
        for _ in 0..1000 {
            let r = random_fraction();
            assert!((0.0..1.0).contains(&r));
        }
    }

    #[test]
    fn parse_order_value_rejects_non_finite() {
        assert_eq!(parse_order_value("-615877.5"), Ok(-615877.5));
        assert!(parse_order_value("nan").is_err());
        assert!(parse_order_value("inf").is_err());
        assert!(parse_order_value("x").is_err());
    }

    #[test]
    fn group_by_parent_skips_parentless_issues() {
        let issues = vec![
            issue(Some("p1"), None),
            issue(None, None),
            issue(Some("p2"), None),
            issue(Some("p1"), None),
        ];
        assert_eq!(group_by_parent(&issues), vec![vec![0, 3], vec![2]]);
    }

    #[test]
    fn sub_issue_order_puts_listed_first_then_rest_by_current_value() {
        let children = vec![
            issue(None, Some(30.0)),
            issue(None, None),
            issue(None, Some(10.0)),
            issue(None, Some(20.0)),
        ];
        assert_eq!(sub_issue_order(&children, &[3, 0]), vec![3, 0, 2, 1]);
    }

    #[test]
    fn order_field_all_covers_three_keys() {
        assert_eq!(
            OrderField::All.keys(),
            &[OrderKey::Manual, OrderKey::Priority, OrderKey::SubIssue]
        );
        assert_eq!(OrderField::SubIssue.keys(), &[OrderKey::SubIssue]);
    }
}
