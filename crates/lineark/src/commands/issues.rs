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
        /// Only sub-issues of this parent issue (identifier or UUID).
        #[arg(long)]
        parent: Option<String>,
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
    /// Create a new issue. Returns the created issue (all fields this command sets, except the description)
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
    /// Update an existing issue. Returns the updated issue (all fields this command sets, except the description)
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
    ///   subIssueSortOrder  The sub-issue list on the parent issue's page (Ordering: Manual).
    ///
    /// Views ordered by priority, the sub-issue list included, group issues by priority
    /// first (Urgent, High, Medium, Low, No priority) and use prioritySortOrder only inside
    /// a group. They can show the wanted order only if no issue is listed above one with a
    /// higher priority; when prioritySortOrder is set (not with --field manual or sub-issue),
    /// reorder prints a warning naming such issues. Dragging an issue in Linear's UI changes
    /// its priority to fit its new neighbours; --set-priority gives every listed issue one
    /// priority before reordering, which does the same explicitly.
    ///
    /// Default (--field all): set all three. subIssueSortOrder only orders siblings: listed
    /// issues that share a parent are ordered among themselves, and issues without a parent
    /// are skipped for it.
    ///
    /// Only the relative order of the listed issues is set. A field already in the wanted
    /// order is left untouched, so `reorder A Z` does nothing when A is already above Z.
    /// List the issues in between to make them adjacent, or use --parent.
    ///
    /// When a field needs changing, the listed issues move as one block to the position of
    /// the topmost of them. For prioritySortOrder, each set of listed issues that share a
    /// priority is its own block, since the field only orders issues inside one group.
    /// Unlisted issues that were above that position stay above the block; the others,
    /// including any that sat between the listed issues, end up below. Unlisted issues are
    /// those of the listed issues' teams (sortOrder; for prioritySortOrder only those with
    /// the block's priority) or the other children of the same parent (subIssueSortOrder).
    /// Archived issues and issues created seconds ago may be missed.
    /// Values are 0.01 apart, or closer when the gap to the issue above is too narrow for
    /// that. If there is no room left, the nearest unlisted issues above are moved too,
    /// keeping their place in the order.
    ///
    /// With --parent: move the listed children to the top of that parent's sub-issue list.
    ///   subIssueSortOrder  Listed children go first, in the given order, 0.01 apart.
    ///                      Children not listed keep their values, so they follow in their
    ///                      current relative order; children with no value yet go last.
    ///   prioritySortOrder  In each priority group, listed children go above every unlisted
    ///                      child with that priority, right above the topmost of them all,
    ///                      unless already in that order. This moves them in the team's
    ///                      Priority views too.
    /// Default: both; --field sub-issue or --field priority sets one.
    ///
    /// Examples:
    ///   lineark issues reorder ENG-101 ENG-102 ENG-103
    ///   lineark issues reorder ENG-101 ENG-102 --field priority
    ///   lineark issues reorder ENG-101 ENG-102 --set-priority medium
    ///   lineark issues reorder --parent ENG-100 ENG-103 ENG-101 ENG-102
    #[command(verbatim_doc_comment)]
    Reorder {
        /// Issue identifiers (e.g., ENG-123) or UUIDs, in the wanted order (top first).
        #[arg(required = true, num_args = 1..)]
        identifiers: Vec<String>,
        /// Parent issue identifier or UUID: move the listed children to the top of its
        /// sub-issue list (subIssueSortOrder, and prioritySortOrder within each priority).
        #[arg(long)]
        parent: Option<String>,
        /// Which order field to set. Default: all (subIssueSortOrder and prioritySortOrder
        /// with --parent).
        #[arg(long, value_enum)]
        field: Option<OrderField>,
        /// First give every listed issue this priority: none, urgent, high, medium, low, or 0-4.
        #[arg(long, value_parser = parse_priority)]
        set_priority: Option<i64>,
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
    /// All order fields (with --parent: subIssueSortOrder and prioritySortOrder).
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
    #[tabled(skip)]
    sort_order: Option<f64>,
    #[tabled(skip)]
    priority_sort_order: Option<f64>,
    #[tabled(skip)]
    sub_issue_sort_order: Option<f64>,
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
            sort_order: i.sort_order,
            priority_sort_order: i.priority_sort_order,
            sub_issue_sort_order: i.sub_issue_sort_order,
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
            sort_order: i.sort_order,
            priority_sort_order: i.priority_sort_order,
            sub_issue_sort_order: i.sub_issue_sort_order,
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
    pub sort_order: Option<f64>,
    pub priority_sort_order: Option<f64>,
    pub sub_issue_sort_order: Option<f64>,
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

/// Result type for `issues create` / `issues update`: every field those commands can set,
/// except the description (often long).
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
struct IssueWriteResult {
    id: Option<String>,
    identifier: Option<String>,
    title: Option<String>,
    #[serde(rename(serialize = "priority"))]
    priority_label: Option<String>,
    estimate: Option<f64>,
    url: Option<String>,
    sort_order: Option<f64>,
    priority_sort_order: Option<f64>,
    sub_issue_sort_order: Option<f64>,
    #[graphql(nested)]
    state: Option<StateRef>,
    #[graphql(nested)]
    assignee: Option<UserRef>,
    #[graphql(nested)]
    team: Option<TeamRef>,
    #[graphql(nested)]
    project: Option<ProjectRef>,
    #[graphql(nested)]
    cycle: Option<CycleRef>,
    #[graphql(nested)]
    parent: Option<RelatedIssueRef>,
    #[graphql(nested)]
    labels: Option<LabelConnection>,
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
            parent,
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
            if let Some(ref parent_val) = parent {
                // issue(id:) accepts identifiers as well as UUIDs, with no search lag.
                let parent_id = client
                    .issue::<IssueIdRef>(parent_val.clone())
                    .await
                    .map_err(|e| anyhow::anyhow!("Parent issue '{}': {}", parent_val, e))?
                    .id
                    .ok_or_else(|| anyhow::anyhow!("Parent issue '{}' not found", parent_val))?;
                filter_map.insert(
                    "parent".into(),
                    serde_json::json!({ "id": { "eq": parent_id } }),
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
                .issue_create::<IssueWriteResult>(input)
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
                .issue_update::<IssueWriteResult>(input, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Reorder {
            identifiers,
            parent,
            field,
            set_priority,
        } => {
            let issues = match parent {
                Some(ref p) => {
                    reorder_sub_issues(client, p, &identifiers, field, set_priority).await?
                }
                None => reorder_issues(client, &identifiers, field, set_priority).await?,
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

/// Spacing between consecutive issues of a block when the gap allows it. sortOrder and
/// prioritySortOrder are shared across the team, and neighbouring issues are often less
/// than 100 apart, so a small step keeps the block next to where it was.
const REORDER_STEP: f64 = 0.01;

/// Smallest spacing a block may use. `tidy` rounds to 6 decimals, so this keeps rounded
/// values apart and inside their gap.
const MIN_SPACING: f64 = 1e-5;

/// Most unlisted issues pulled into a block to make room, before giving up.
const MAX_PULLED: usize = 50;

/// Fractions of the spacing to shift a block by when a value ties a pulled issue's value.
const TIE_SHIFTS: [f64; 4] = [0.0, 0.5, 0.25, 0.75];

/// A uniform random number in [0, 1), from the std library's randomly keyed hasher.
///
/// Blocks are shifted by a random fraction of their spacing, so runs don't land on a shared
/// grid: an issue that is not fetched (archived, or too new for the filter index) keeps its
/// value, and a later run writing exactly that value would trip Linear's tie handling.
fn random_fraction() -> f64 {
    use std::hash::BuildHasher;
    let bits = std::collections::hash_map::RandomState::new().hash_one(0u8);
    (bits >> 11) as f64 / (1u64 << 53) as f64
}

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
    #[serde(rename(serialize = "priority"))]
    #[tabled(rename = "priority", display_with = "output::display_opt")]
    pub priority_label: Option<String>,
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
    #[serde(skip_serializing)]
    #[tabled(skip)]
    #[graphql(nested)]
    pub team: Option<TeamRef>,
    #[serde(skip_serializing)]
    #[tabled(skip)]
    pub priority: Option<f64>,
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

    fn team_id(&self) -> Option<&str> {
        self.team.as_ref().and_then(|t| t.id.as_deref())
    }

    fn label(&self) -> String {
        self.identifier
            .clone()
            .or_else(|| self.id.clone())
            .unwrap_or_default()
    }

    fn priority_name(&self) -> &str {
        self.priority_label.as_deref().unwrap_or("unknown priority")
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

/// Round to 6 decimals to drop binary noise (`-3189.9849999999997`); MIN_SPACING is coarser.
fn tidy(value: f64) -> f64 {
    (value * 1e6).round() / 1e6
}

/// Whether every value is set and they strictly ascend.
fn ascends(values: &[Option<f64>]) -> bool {
    values.iter().all(Option::is_some) && values.windows(2).all(|w| w[0] < w[1])
}

/// Whether `current` ascends and, when `below` is set, its last value is under `below`.
fn in_order(current: &[Option<f64>], below: Option<f64>) -> bool {
    let last = current.last().copied().flatten();
    ascends(current) && below.is_none_or(|b| last.is_some_and(|l| l < b))
}

/// Where a priority puts an issue in views ordered by priority: Urgent (1) first, then
/// High, Medium, Low, and No priority (0) last.
fn priority_rank(priority: Option<f64>) -> f64 {
    match priority {
        Some(p) if p > 0.0 => p,
        _ => 5.0,
    }
}

/// Pairs `(i, j)` where wanted issue `i` (one of the first `listed`) is above issue `j`,
/// but `j` has a higher priority, so views ordered by priority show `j` first. For each
/// such `i`, `j` is the later issue with the highest priority (the first one on ties).
fn priority_conflicts(priorities: &[Option<f64>], listed: usize) -> Vec<(usize, usize)> {
    (0..listed)
        .filter_map(|i| {
            let rank = priority_rank(priorities[i]);
            (i + 1..priorities.len())
                .filter(|&j| priority_rank(priorities[j]) < rank)
                .min_by(|&a, &b| {
                    priority_rank(priorities[a]).total_cmp(&priority_rank(priorities[b]))
                })
                .map(|j| (i, j))
        })
        .collect()
}

/// A warning when views ordered by priority cannot show `issues` in this order (the first
/// `listed` of them as listed, the rest after them): those views group by priority first.
fn priority_warning(issues: &[IssueOrder], listed: usize) -> Option<String> {
    let priorities: Vec<Option<f64>> = issues.iter().map(|i| i.priority).collect();
    let conflicts = priority_conflicts(&priorities, listed);
    if conflicts.is_empty() {
        return None;
    }
    const MAX_PAIRS: usize = 5;
    let mut pairs: Vec<String> = conflicts
        .iter()
        .take(MAX_PAIRS)
        .map(|&(i, j)| {
            let unlisted = if j >= listed { ", not listed" } else { "" };
            format!(
                "{} ({}) is above {} ({}{})",
                issues[i].label(),
                issues[i].priority_name(),
                issues[j].label(),
                issues[j].priority_name(),
                unlisted
            )
        })
        .collect();
    if conflicts.len() > MAX_PAIRS {
        pairs.push(format!("and {} more", conflicts.len() - MAX_PAIRS));
    }
    // Unlisted issues keep their priority: to go above all of them, the listed ones need at
    // least the highest priority among them (any priority is enough against No priority).
    let highest_unlisted = (listed..issues.len())
        .filter(|&j| priority_rank(priorities[j]) < priority_rank(None))
        .min_by(|&a, &b| priority_rank(priorities[a]).total_cmp(&priority_rank(priorities[b])));
    let level = match highest_unlisted {
        Some(j) => format!(
            " To go above the unlisted ones, use {}{}.",
            issues[j].priority_name(),
            if priority_rank(priorities[j]) > 1.0 {
                " or higher"
            } else {
                ""
            }
        ),
        None => String::new(),
    };
    Some(format!(
        "Warning: views ordered by priority show higher priorities first, so they will not show this order: {}. Give the issues priorities that fit the order with --set-priority LEVEL (one for every listed issue) or `issues update ID --priority LEVEL`, then re-run reorder.{}",
        pairs.join("; "),
        level
    ))
}

/// `count` ascending values strictly inside (lo, hi), where a missing bound is open-ended.
///
/// The values are `step` apart and as close to `hi` as possible, or
/// spread evenly across the interval when it is narrower than that. `shift` (in [0, 1))
/// moves them down by that fraction of the spacing. Returns `None` when the values would
/// be closer than MIN_SPACING, or float precision cannot keep them apart and inside.
fn spread(
    count: usize,
    lo: Option<f64>,
    hi: Option<f64>,
    step: f64,
    shift: f64,
) -> Option<Vec<f64>> {
    let slots = (count + 1) as f64;
    let (lo, hi) = match (lo, hi) {
        (Some(lo), Some(hi)) => (lo.max(hi - slots * step), hi),
        (None, Some(hi)) => (hi - slots * step, hi),
        (Some(lo), None) => (lo, lo + slots * step),
        (None, None) => (0.0, slots * step),
    };
    let spacing = (hi - lo) / slots;
    if spacing.is_nan() || spacing < MIN_SPACING {
        return None;
    }
    let values: Vec<f64> = (1..=count)
        .map(|i| tidy(hi - spacing * (slots - i as f64 + shift)))
        .collect();
    let fits = values.iter().all(|v| v.is_finite() && lo < *v && *v < hi)
        && values.windows(2).all(|w| w[0] < w[1]);
    fits.then_some(values)
}

/// Where a block of issues goes.
#[derive(Debug, PartialEq)]
struct Block {
    /// The old position of the block (the topmost listed issue's value, or `below` when that
    /// is higher). Neighbours below it numerically (above it on screen) stay above the block;
    /// the others end up below. `None` when no listed issue had a value: the block then goes
    /// below every neighbour.
    anchor: Option<f64>,
    /// Neighbours pulled into the block to make room, top first. They keep their order and
    /// stay above the listed issues.
    pulled: Vec<usize>,
    /// New values for the pulled neighbours, then the listed issues.
    values: Vec<f64>,
}

/// Plan one group: `current` holds the listed issues' values in the wanted order,
/// `neighbours` the values of the unlisted issues that share an ordering with them.
/// `below`, when set, is the topmost value of unlisted issues that must end up below the
/// block (`--parent`: the other children).
///
/// Returns `None` when the current values already ascend (and sit above `below`). Otherwise
/// the listed issues move as one block into the free gap right above the topmost of them
/// and `below` (the lowest of those values): every neighbour that was above that position
/// stays above the block, and every other neighbour ends up below it. When the gap is too
/// narrow, the nearest neighbours above are pulled into the block, in their current order,
/// until it fits.
///
/// No value ties another fetched issue's current value: Linear moves an issue elsewhere when
/// its new prioritySortOrder or subIssueSortOrder ties one that another issue still holds.
/// `jitter` (in [0, 1)) shifts the block by that fraction of its spacing, so values don't
/// repeat across runs and tie with issues that were not fetched.
fn plan_block(
    current: &[Option<f64>],
    below: Option<f64>,
    neighbours: &[Option<f64>],
    step: f64,
    jitter: f64,
) -> anyhow::Result<Option<Block>> {
    if in_order(current, below) {
        return Ok(None);
    }
    let n = current.len();
    let Some(anchor) = current
        .iter()
        .flatten()
        .chain(&below)
        .copied()
        .reduce(f64::min)
    else {
        let lo = neighbours.iter().flatten().copied().reduce(f64::max);
        let values = spread(n, lo, None, step, jitter).ok_or_else(|| {
            anyhow::anyhow!("Cannot place {n} issues {step} apart after {lo:?}: float precision is too low there.")
        })?;
        return Ok(Some(Block {
            anchor: None,
            pulled: Vec::new(),
            values,
        }));
    };

    // Neighbours above the anchor, nearest first.
    let mut above: Vec<(usize, f64)> = neighbours
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.filter(|v| *v < anchor).map(|v| (i, v)))
        .collect();
    above.sort_by(|a, b| b.1.total_cmp(&a.1));

    for pulled in 0..=above.len().min(MAX_PULLED) {
        let lo = above.get(pulled).map(|&(_, v)| v);
        // Old values inside the gap belong to the pulled neighbours only.
        let inside: Vec<f64> = above[..pulled].iter().rev().map(|&(_, v)| v).collect();
        let tie_free = std::iter::once(&jitter)
            .chain(&TIE_SHIFTS)
            .map_while(|&shift| spread(pulled + n, lo, Some(anchor), step, shift))
            .find(|values| {
                !values
                    .iter()
                    .enumerate()
                    .any(|(i, v)| inside.iter().enumerate().any(|(j, u)| i != j && u == v))
            });
        if let Some(values) = tie_free {
            return Ok(Some(Block {
                anchor: Some(anchor),
                pulled: above[..pulled].iter().rev().map(|&(i, _)| i).collect(),
                values,
            }));
        }
    }
    Err(anyhow::anyhow!(
        "No room to place {n} issues above {anchor}: the issues there are too close together for float precision. Set the values with `issues update --sort-order` etc. instead."
    ))
}

/// Indexes of `issues` grouped by parent, in first-appearance order, with the parent ID.
/// Issues without a parent are left out: Linear keeps their subIssueSortOrder null.
fn group_by_parent(issues: &[IssueOrder]) -> Vec<(String, Vec<usize>)> {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, issue) in issues.iter().enumerate() {
        let Some(parent) = issue.parent_id() else {
            continue;
        };
        match groups.iter_mut().find(|(p, _)| p == parent) {
            Some((_, members)) => members.push(i),
            None => groups.push((parent.to_string(), vec![i])),
        }
    }
    groups
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

/// What must hold on one order field after the writes.
struct Check {
    key: OrderKey,
    /// Plan indexes that must strictly ascend, top first.
    members: Vec<usize>,
    /// Values of issues not written that must stay above the first member.
    above: Vec<f64>,
    /// Values of issues not written that must stay below the last member.
    below: Vec<f64>,
}

/// The issues to write and what must hold afterwards.
#[derive(Default)]
struct ReorderPlan {
    /// The issues to report first, then unlisted issues pulled into a block to make room.
    issues: Vec<IssueOrder>,
    /// How many of `issues` to report.
    reported: usize,
    /// New values per issue (same indexes as `issues`).
    values: Vec<Vec<(OrderKey, f64)>>,
    checks: Vec<Check>,
}

impl ReorderPlan {
    fn new(issues: Vec<IssueOrder>) -> Self {
        let values = vec![Vec::new(); issues.len()];
        Self {
            reported: issues.len(),
            issues,
            values,
            checks: Vec::new(),
        }
    }

    /// Plan `key` for `members` (plan indexes, in the wanted order) against `neighbours`,
    /// the issues sharing that ordering (members among them are ignored). `below`: see
    /// `plan_block`.
    fn plan_group(
        &mut self,
        key: OrderKey,
        members: Vec<usize>,
        below: Option<f64>,
        neighbours: &[IssueOrder],
    ) -> anyhow::Result<()> {
        let neighbours: Vec<&IssueOrder> = neighbours
            .iter()
            .filter(|o| !members.iter().any(|&i| self.issues[i].id == o.id))
            .collect();
        let current: Vec<Option<f64>> = members.iter().map(|&i| key.get(&self.issues[i])).collect();
        let neighbour_values: Vec<Option<f64>> = neighbours.iter().map(|o| key.get(o)).collect();
        let Some(block) = plan_block(
            &current,
            below,
            &neighbour_values,
            REORDER_STEP,
            random_fraction(),
        )?
        else {
            self.checks.push(Check {
                key,
                members,
                above: Vec::new(),
                below: Vec::new(),
            });
            return Ok(());
        };

        let mut block_members = Vec::with_capacity(block.values.len());
        for &n in &block.pulled {
            let issue = neighbours[n];
            let index = match self.issues.iter().position(|i| i.id == issue.id) {
                Some(index) => index,
                None => {
                    self.issues.push(issue.clone());
                    self.values.push(Vec::new());
                    self.issues.len() - 1
                }
            };
            block_members.push(index);
        }
        block_members.extend(members);
        for (&i, &value) in block_members.iter().zip(&block.values) {
            self.values[i].push((key, value));
        }

        let (mut above, mut below) = (Vec::new(), Vec::new());
        for (n, value) in neighbour_values.iter().enumerate() {
            let Some(value) = *value else { continue };
            if block.pulled.contains(&n) {
                continue;
            }
            match block.anchor {
                Some(anchor) if value >= anchor => below.push(value),
                _ => above.push(value),
            }
        }
        self.checks.push(Check {
            key,
            members: block_members,
            above,
            below,
        });
        Ok(())
    }

    /// Whether issue `i` has a planned value that differs from its current one.
    fn changes(&self, i: usize) -> bool {
        self.values[i]
            .iter()
            .any(|&(key, value)| key.get(&self.issues[i]) != Some(value))
    }
}

/// `issues reorder A B C`: move the listed issues, per field, as one block to the position
/// of the topmost of them.
async fn reorder_issues(
    client: &Client,
    identifiers: &[String],
    field: Option<OrderField>,
    set_priority: Option<i64>,
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
    // Validate before --set-priority writes anything.
    if field == Some(OrderField::SubIssue)
        && group_by_parent(&issues).iter().all(|(_, g)| g.len() < 2)
    {
        return Err(anyhow::anyhow!(
            "No two of these issues share a parent, so there is no sub-issue list to order. Use --parent to move a sub-issue within its parent's list."
        ));
    }
    let priority_set = match set_priority {
        Some(priority) => set_priorities(client, &mut issues, priority).await?,
        None => Vec::new(),
    };
    order_issues(client, issues, field)
        .await
        .map_err(|e| note_priority_set(e, &priority_set))
}

/// Plan and write the order for `reorder_issues`, once the listed issues are fetched.
async fn order_issues(
    client: &Client,
    issues: Vec<IssueOrder>,
    field: Option<OrderField>,
) -> anyhow::Result<Vec<IssueOrder>> {
    let keys = field.unwrap_or(OrderField::All).keys();
    if keys.contains(&OrderKey::Priority) {
        if let Some(warning) = priority_warning(&issues, issues.len()) {
            eprintln!("{warning}");
        }
    }
    let mut plan = ReorderPlan::new(issues);
    // sortOrder and prioritySortOrder neighbours: the listed issues' teams, fetched once.
    // sortOrder comes first; when prioritySortOrder is the first to need them, only the
    // issues with a listed priority are fetched.
    let listed_priorities: Vec<f64> = plan.issues.iter().filter_map(|i| i.priority).collect();
    let mut team_issues: Option<Vec<IssueOrder>> = None;
    for &key in keys {
        if key == OrderKey::SubIssue {
            // subIssueSortOrder only orders siblings: order each parent's listed children
            // among themselves, against the parent's other children.
            let groups = group_by_parent(&plan.issues[..plan.reported]);
            for (parent_id, members) in groups {
                if members.len() < 2 {
                    continue;
                }
                let siblings = if needs_writes(&plan, key, &members, None) {
                    let parent = client
                        .issue::<ParentWithChildren>(parent_id.clone())
                        .await
                        .map_err(|e| anyhow::anyhow!("Parent issue '{}': {}", parent_id, e))?;
                    fetch_children(client, &parent_id, parent).await?
                } else {
                    Vec::new()
                };
                plan.plan_group(key, members, None, &siblings)?;
            }
        } else {
            let members: Vec<usize> = (0..plan.reported).collect();
            // prioritySortOrder only orders issues within one priority group.
            let groups = (key == OrderKey::Priority)
                .then(|| priority_groups(&plan.issues[..plan.reported], plan.reported));
            let needed = match &groups {
                Some(groups) => groups
                    .iter()
                    .any(|g| needs_writes(&plan, key, &g.members, g.below)),
                None => needs_writes(&plan, key, &members, None),
            };
            if team_issues.is_none() && needed {
                team_issues = Some(
                    fetch_team_issues(
                        client,
                        &plan.issues[..plan.reported],
                        (key == OrderKey::Priority).then_some(listed_priorities.as_slice()),
                    )
                    .await?,
                );
            }
            let team = team_issues.as_deref().unwrap_or_default();
            match groups {
                Some(groups) => plan.plan_priority_groups(groups, team)?,
                None => plan.plan_group(key, members, None, team)?,
            }
        }
    }

    note_pulled(&plan);
    apply_reorder(client, plan).await
}

/// Whether `members` are out of order on `key` (or not above `below`), so their neighbours
/// are needed.
fn needs_writes(plan: &ReorderPlan, key: OrderKey, members: &[usize], below: Option<f64>) -> bool {
    let current: Vec<Option<f64>> = members.iter().map(|&i| key.get(&plan.issues[i])).collect();
    !in_order(&current, below)
}

/// Tell about unlisted issues pulled into a block to make room.
fn note_pulled(plan: &ReorderPlan) {
    let pulled: Vec<String> = plan.issues[plan.reported..]
        .iter()
        .map(IssueOrder::label)
        .collect();
    if !pulled.is_empty() {
        eprintln!(
            "Note: also moved {} (not listed) to make room, keeping their place in the order.",
            pulled.join(", ")
        );
    }
}

/// Give every issue `priority` (one `issueUpdate` per issue whose priority differs), then
/// read the changed ones back: Linear may move prioritySortOrder when the priority changes.
/// Returns the labels of the issues whose priority changed.
async fn set_priorities(
    client: &Client,
    issues: &mut [IssueOrder],
    priority: i64,
) -> anyhow::Result<Vec<String>> {
    let mut updated: Vec<String> = Vec::new();
    for issue in issues.iter_mut() {
        if issue.priority == Some(priority as f64) {
            continue;
        }
        let id = issue.id.clone().unwrap_or_default();
        let input = IssueUpdateInput {
            priority: MaybeUndefined::Value(priority),
            ..Default::default()
        };
        client
            .issue_update::<IssueRef>(input, id.clone())
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to set the priority of {}: {}{}",
                    issue.label(),
                    e,
                    priority_set_status(&updated)
                )
            })?;
        updated.push(issue.label());
        *issue = client.issue::<IssueOrder>(id).await.map_err(|e| {
            anyhow::anyhow!(
                "Reading back {}: {}{}",
                issue.label(),
                e,
                priority_set_status(&updated)
            )
        })?;
    }
    Ok(updated)
}

/// What a --set-priority failure leaves behind: which priorities changed, no order did.
fn priority_set_status(updated: &[String]) -> String {
    let set = if updated.is_empty() {
        "none".to_string()
    } else {
        updated.join(", ")
    };
    format!("\nPriority already set on: {set}. No order was changed. Re-run the same command to finish.")
}

/// Add to a reorder error which issues --set-priority already changed.
fn note_priority_set(error: anyhow::Error, priority_set: &[String]) -> anyhow::Error {
    if priority_set.is_empty() {
        return error;
    }
    anyhow::anyhow!(
        "{}\nPriority already set on: {}.",
        error,
        priority_set.join(", ")
    )
}

/// The non-archived issues of every team that one of `issues` belongs to, only those with
/// one of `priorities` when given.
async fn fetch_team_issues(
    client: &Client,
    issues: &[IssueOrder],
    priorities: Option<&[f64]>,
) -> anyhow::Result<Vec<IssueOrder>> {
    let mut team_ids: Vec<String> = Vec::new();
    for issue in issues {
        if let Some(team_id) = issue.team_id() {
            if !team_ids.iter().any(|t| t == team_id) {
                team_ids.push(team_id.to_string());
            }
        }
    }
    let mut filter = serde_json::json!({ "team": { "id": { "in": team_ids } } });
    if let Some(priorities) = priorities {
        filter["priority"] = serde_json::json!({ "in": priorities });
    }
    fetch_issues(client, filter).await
}

/// A parent's non-archived children: its `children` connection (first page, read with the
/// parent), plus any child only the paginated filter query returns. The connection's copies
/// win: the filter can lag behind a just-created child or a just-changed field.
async fn fetch_children(
    client: &Client,
    parent_id: &str,
    parent: ParentWithChildren,
) -> anyhow::Result<Vec<IssueOrder>> {
    let mut children = parent.children.map(|c| c.nodes).unwrap_or_default();
    let filter = serde_json::json!({ "parent": { "id": { "eq": parent_id } } });
    for child in fetch_issues(client, filter).await? {
        if !children.iter().any(|c| c.id == child.id) {
            children.push(child);
        }
    }
    Ok(children)
}

/// `issues reorder --parent P A B`: move the listed children to the top of P's sub-issue
/// list. Children not listed keep their values, so they follow in their current order.
async fn reorder_sub_issues(
    client: &Client,
    parent: &str,
    identifiers: &[String],
    field: Option<OrderField>,
    set_priority: Option<i64>,
) -> anyhow::Result<Vec<IssueOrder>> {
    let keys: &[OrderKey] = match field {
        None | Some(OrderField::All) => &[OrderKey::SubIssue, OrderKey::Priority],
        Some(OrderField::SubIssue) => &[OrderKey::SubIssue],
        Some(OrderField::Priority) => &[OrderKey::Priority],
        Some(OrderField::Manual) => {
            return Err(anyhow::anyhow!(
                "--parent orders the parent's sub-issue list: subIssueSortOrder and prioritySortOrder. Drop --field, or use --field sub-issue or --field priority."
            ))
        }
    };

    let parent_issue = client
        .issue::<ParentWithChildren>(parent.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("Parent issue '{}': {}", parent, e))?;
    let parent_id = parent_issue
        .id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Parent issue '{}' not found", parent))?;
    let parent_label = parent_issue
        .identifier
        .clone()
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
    let priority_set = match set_priority {
        Some(priority) => set_priorities(client, &mut listed_issues, priority).await?,
        None => Vec::new(),
    };
    order_children(client, &parent_id, parent_issue, listed_issues, keys)
        .await
        .map_err(|e| note_priority_set(e, &priority_set))
}

/// Plan and write the order for `reorder_sub_issues`, once the listed children are fetched.
async fn order_children(
    client: &Client,
    parent_id: &str,
    parent_issue: ParentWithChildren,
    listed_issues: Vec<IssueOrder>,
    keys: &[OrderKey],
) -> anyhow::Result<Vec<IssueOrder>> {
    let mut children = fetch_children(client, parent_id, parent_issue).await?;
    let mut listed: Vec<usize> = Vec::with_capacity(listed_issues.len());
    for issue in listed_issues {
        let idx = match children.iter().position(|c| c.id == issue.id) {
            Some(idx) => {
                children[idx] = issue;
                idx
            }
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
    let k = listed.len();
    if keys.contains(&OrderKey::SubIssue) {
        plan_sub_issues_on_top(&mut plan, k)?;
    }
    if keys.contains(&OrderKey::Priority) {
        if let Some(warning) = priority_warning(&plan.issues, k) {
            eprintln!("{warning}");
        }
        plan_priority_on_top(client, &mut plan, k).await?;
    }

    note_pulled(&plan);
    apply_reorder(client, plan).await
}

/// `--parent`, subIssueSortOrder: put the first `k` children of the plan (the listed ones)
/// above every other child, and give children with no value yet one at the bottom.
fn plan_sub_issues_on_top(plan: &mut ReorderPlan, k: usize) -> anyhow::Result<()> {
    let key = OrderKey::SubIssue;
    let values: Vec<Option<f64>> = plan.issues.iter().map(|i| key.get(i)).collect();
    let rest_set: Vec<f64> = values[k..].iter().flatten().copied().collect();
    let listed_on_top = ascends(&values[..k])
        && rest_set
            .iter()
            .all(|r| values[..k].iter().flatten().all(|l| l < r));
    // Children with no value yet go last, in their current order.
    let unset: Vec<usize> = (k..plan.issues.len())
        .filter(|&i| values[i].is_none())
        .collect();
    if !listed_on_top || !unset.is_empty() {
        let mut finals: Vec<f64> = rest_set.clone();
        if listed_on_top {
            finals.extend(values[..k].iter().flatten());
        } else {
            // Above every child, REORDER_STEP apart.
            let top = values.iter().flatten().copied().reduce(f64::min);
            let new = spread(k, None, top, REORDER_STEP, random_fraction()).ok_or_else(|| {
                anyhow::anyhow!("Cannot place {k} issues {REORDER_STEP} apart above {top:?}: float precision is too low there.")
            })?;
            for (i, &value) in new.iter().enumerate() {
                plan.values[i].push((key, value));
            }
            finals.extend(new);
        }
        if !unset.is_empty() {
            let bottom = finals.iter().copied().reduce(f64::max);
            let new = spread(unset.len(), bottom, None, REORDER_STEP, random_fraction()).ok_or_else(|| {
                anyhow::anyhow!("Cannot place {} issues {REORDER_STEP} apart below {bottom:?}: float precision is too low there.", unset.len())
            })?;
            for (&i, value) in unset.iter().zip(new) {
                plan.values[i].push((key, value));
            }
        }
    }
    // Untouched children may tie among themselves; only check the moved ones against them.
    let listed_members: Vec<usize> = (0..k).collect();
    if rest_set.is_empty() {
        plan.checks.push(Check {
            key,
            members: listed_members.into_iter().chain(unset).collect(),
            above: Vec::new(),
            below: Vec::new(),
        });
    } else {
        plan.checks.push(Check {
            key,
            members: listed_members,
            above: Vec::new(),
            below: rest_set.clone(),
        });
        if !unset.is_empty() {
            plan.checks.push(Check {
                key,
                members: unset,
                above: rest_set,
                below: Vec::new(),
            });
        }
    }
    Ok(())
}

/// `--parent`, prioritySortOrder: in each priority group, put the listed children (the first
/// `k` of the plan) above every unlisted child with that priority, so a sub-issue list ordered
/// by priority shows them first in their group. prioritySortOrder is not per parent: the other
/// issues with that priority in the listed children's teams are neighbours too.
async fn plan_priority_on_top(
    client: &Client,
    plan: &mut ReorderPlan,
    k: usize,
) -> anyhow::Result<()> {
    let groups = priority_groups(&plan.issues[..plan.reported], k);
    let team = if groups
        .iter()
        .any(|g| needs_writes(plan, OrderKey::Priority, &g.members, g.below))
    {
        // The listed children's teams; unlisted children are added by plan_priority_groups.
        let priorities: Vec<f64> = groups.iter().filter_map(|g| g.priority).collect();
        fetch_team_issues(client, &plan.issues[..k], Some(&priorities)).await?
    } else {
        Vec::new()
    };
    plan.plan_priority_groups(groups, &team)
}

/// Listed issues that share a priority, ordered on prioritySortOrder as one group.
#[derive(Debug, PartialEq)]
struct PriorityGroup {
    priority: Option<f64>,
    /// Indexes of the listed issues with this priority, in the wanted order.
    members: Vec<usize>,
    /// The topmost prioritySortOrder of the unlisted issues with this priority, which must
    /// end up below the group (`--parent`: the other children).
    below: Option<f64>,
}

/// Group the first `listed` of `issues` by priority, in first-appearance order. The issues
/// after them are unlisted ones that must stay below their group.
fn priority_groups(issues: &[IssueOrder], listed: usize) -> Vec<PriorityGroup> {
    let mut groups: Vec<PriorityGroup> = Vec::new();
    for (i, issue) in issues[..listed].iter().enumerate() {
        match groups.iter_mut().find(|g| g.priority == issue.priority) {
            Some(group) => group.members.push(i),
            None => groups.push(PriorityGroup {
                priority: issue.priority,
                members: vec![i],
                below: None,
            }),
        }
    }
    for group in &mut groups {
        group.below = issues[listed..]
            .iter()
            .filter(|o| o.priority == group.priority)
            .filter_map(|o| o.priority_sort_order)
            .reduce(f64::min);
    }
    groups
}

impl ReorderPlan {
    /// Plan prioritySortOrder for each group against `team` issues with the same priority,
    /// plus the reported issues that are not members (`--parent`: the other children, in
    /// case the team query lags behind one of them).
    fn plan_priority_groups(
        &mut self,
        groups: Vec<PriorityGroup>,
        team: &[IssueOrder],
    ) -> anyhow::Result<()> {
        // The team query can lag behind a priority change: take reported issues only from
        // the plan, which holds fresh reads, so none lands in its old priority's group.
        let reported: std::collections::HashSet<String> = self.issues[..self.reported]
            .iter()
            .filter_map(|i| i.id.clone())
            .collect();
        for group in groups {
            let neighbours: Vec<IssueOrder> = self.issues[..self.reported]
                .iter()
                .chain(
                    team.iter()
                        .filter(|o| o.id.as_ref().is_none_or(|id| !reported.contains(id))),
                )
                .filter(|o| o.priority == group.priority)
                .cloned()
                .collect();
            self.plan_group(OrderKey::Priority, group.members, group.below, &neighbours)?;
        }
        Ok(())
    }
}

/// All non-archived issues matching `filter`, following pagination.
async fn fetch_issues(
    client: &Client,
    filter: serde_json::Value,
) -> anyhow::Result<Vec<IssueOrder>> {
    let filter: IssueFilter = serde_json::from_value(filter).expect("valid IssueFilter");
    let mut issues = Vec::new();
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
        issues.extend(conn.nodes);
        match conn.page_info.end_cursor {
            Some(cursor) if conn.page_info.has_next_page => after = Some(cursor),
            _ => break,
        }
    }
    Ok(issues)
}

/// Write the planned values (one `issueUpdate` per issue whose values change), then read
/// every issue back and check each planned group. Returns the reported issues, read back.
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

    for check in &plan.checks {
        let values: Vec<Option<f64>> = check
            .members
            .iter()
            .map(|&i| check.key.get(&result[i]))
            .collect();
        let (first, last) = (
            values.first().copied().flatten(),
            values.last().copied().flatten(),
        );
        let holds = ascends(&values)
            && check.above.iter().all(|&v| first.is_some_and(|f| v < f))
            && check.below.iter().all(|&v| last.is_some_and(|l| v > l));
        if !holds {
            let got: Vec<String> = check
                .members
                .iter()
                .map(|&i| {
                    format!(
                        "{}={}",
                        result[i].label(),
                        output::display_opt(&check.key.get(&result[i]))
                    )
                })
                .collect();
            return Err(anyhow::anyhow!(
                "Verification failed: {} is not in the wanted order after the update: {}",
                check.key.name(),
                got.join(", ")
            ));
        }
    }

    result.truncate(plan.reported);
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
    fn spread_packs_step_apart_against_hi() {
        let values = spread(3, Some(-3100.0), Some(-3090.0), 0.01, 0.0).unwrap();
        assert_eq!(values, vec![-3090.03, -3090.02, -3090.01]);
        assert_eq!(
            spread(2, None, Some(5.0), 0.5, 0.0).unwrap(),
            vec![4.0, 4.5]
        );
        assert_eq!(
            spread(2, Some(5.0), None, 0.5, 0.0).unwrap(),
            vec![5.5, 6.0]
        );
        assert_eq!(spread(2, None, None, 0.5, 0.0).unwrap(), vec![0.5, 1.0]);
    }

    #[test]
    fn spread_splits_a_narrow_gap_evenly() {
        assert_eq!(
            spread(3, Some(1.0), Some(1.01), 0.01, 0.0).unwrap(),
            vec![1.0025, 1.005, 1.0075]
        );
    }

    #[test]
    fn spread_shift_moves_values_down_inside_the_gap() {
        assert_eq!(
            spread(1, Some(1.0), Some(1.01), 0.01, 0.5).unwrap(),
            vec![1.0025]
        );
    }

    #[test]
    fn spread_refuses_gaps_below_min_spacing_or_precision() {
        assert_eq!(
            spread(1, Some(1.0), Some(1.0 + MIN_SPACING), 0.01, 0.0),
            None
        );
        assert_eq!(spread(2, None, Some(1e17), 0.01, 0.0), None);
    }

    #[test]
    fn random_fraction_is_in_unit_interval() {
        for _ in 0..1000 {
            assert!((0.0..1.0).contains(&random_fraction()));
        }
    }

    #[test]
    fn plan_block_jitter_keeps_the_block_inside_the_gap() {
        let neighbours = [Some(0.997), Some(1.5)];
        for jitter in [0.0, 0.3, 0.999] {
            let planned = plan_block(&[Some(2.0), Some(1.0)], None, &neighbours, 0.01, jitter)
                .unwrap()
                .unwrap();
            assert!(planned.values.iter().all(|v| 0.997 < *v && *v < 1.0));
            assert!(planned.values[0] < planned.values[1]);
        }
    }

    #[test]
    fn plan_block_skips_groups_already_in_order() {
        let planned = plan_block(&[Some(1.0), Some(2.0)], None, &[Some(1.5)], 0.01, 0.0).unwrap();
        assert_eq!(planned, None);
    }

    #[test]
    fn plan_block_places_the_block_right_above_the_topmost_issue() {
        // No neighbour above: 0.01 apart, the last one 0.01 above the old topmost value.
        let planned = plan_block(
            &[Some(5.0), Some(-3190.0), None],
            None,
            &[Some(7.0)],
            0.01,
            0.0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(planned.anchor, Some(-3190.0));
        assert_eq!(planned.pulled, Vec::<usize>::new());
        assert_eq!(planned.values, vec![-3190.03, -3190.02, -3190.01]);
    }

    #[test]
    fn plan_block_partial_reorder_does_not_interleave_with_unlisted_issues() {
        // A full reorder packed A..N 0.01 apart; now move N to the top, then A.
        let packed: Vec<f64> = (0..14)
            .map(|i| tidy(-615799.99766 + i as f64 * 0.01))
            .collect();
        let current = [Some(packed[13]), Some(packed[0])];
        // Unlisted: B..M, an issue above the block and one far below.
        let mut neighbours: Vec<Option<f64>> = packed[1..13].iter().copied().map(Some).collect();
        neighbours.push(Some(-615850.0));
        neighbours.push(Some(-615000.0));

        let planned = plan_block(&current, None, &neighbours, 0.01, 0.0)
            .unwrap()
            .unwrap();
        assert_eq!(planned.pulled, Vec::<usize>::new());
        let (first, last) = (planned.values[0], planned.values[1]);
        assert!(first < last);
        for v in neighbours.iter().flatten() {
            assert!(
                *v < first || *v > last,
                "unlisted value {v} falls inside the block {:?}",
                planned.values
            );
        }
        // B..M stay below the block, the issue above stays above.
        assert!(packed[1] > last);
        assert!(-615850.0 < first);
    }

    #[test]
    fn plan_block_splits_the_gap_to_a_close_neighbour() {
        let planned = plan_block(
            &[Some(2.0), Some(1.0)],
            None,
            &[Some(0.997), Some(1.5)],
            0.01,
            0.0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(planned.values, vec![0.998, 0.999]);
    }

    #[test]
    fn plan_block_pulls_neighbours_in_when_the_gap_is_too_small() {
        // Neighbour 0 sits a hair above the topmost listed issue, neighbour 1 further up.
        let neighbours = [Some(1.0 - 1e-6), Some(0.5), Some(3.0)];
        let planned = plan_block(&[Some(2.0), Some(1.0)], None, &neighbours, 0.01, 0.0)
            .unwrap()
            .unwrap();
        assert_eq!(planned.pulled, vec![0]);
        assert_eq!(planned.values, vec![0.97, 0.98, 0.99]);
    }

    #[test]
    fn plan_block_never_ties_a_pulled_neighbours_value() {
        // Every shift of the 4-pulled block puts a listed issue on 0.99996, which pulled
        // neighbour 0 still holds while the listed issues are written.
        let current: Vec<Option<f64>> = (0..10).rev().map(|i| Some(1.0 + i as f64)).collect();
        let neighbours = [
            Some(0.99996),
            Some(0.99995),
            Some(0.99994),
            Some(0.99993),
            Some(0.9994),
        ];
        let planned = plan_block(&current, None, &neighbours, 0.01, 0.0)
            .unwrap()
            .unwrap();
        let pulled_old: Vec<f64> = planned
            .pulled
            .iter()
            .map(|&i| neighbours[i].unwrap())
            .collect();
        for (i, v) in planned.values.iter().enumerate() {
            for (j, u) in pulled_old.iter().enumerate() {
                assert!(i == j || u != v, "value {v} ties pulled neighbour {j}");
            }
        }
        assert!(planned.values.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn plan_block_goes_after_the_neighbours_when_nothing_has_a_value() {
        let planned = plan_block(&[None, None], None, &[Some(3.0), None], 0.5, 0.0)
            .unwrap()
            .unwrap();
        assert_eq!(planned.anchor, None);
        assert_eq!(planned.values, vec![3.5, 4.0]);
    }

    #[test]
    fn plan_block_rejects_lost_precision() {
        assert!(plan_block(&[Some(2e17), Some(1e17)], None, &[], 0.01, 0.0).is_err());
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
        assert_eq!(
            group_by_parent(&issues),
            vec![("p1".to_string(), vec![0, 3]), ("p2".to_string(), vec![2])]
        );
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

    #[test]
    fn plan_block_goes_above_the_topmost_issue_that_must_stay_below() {
        // Listed issues in order, but an unlisted sibling at 1.5 must end up below them.
        let planned = plan_block(&[Some(1.0), Some(2.0)], Some(1.5), &[Some(1.5)], 0.01, 0.0)
            .unwrap()
            .unwrap();
        assert_eq!(planned.anchor, Some(1.0));
        assert_eq!(planned.values, vec![0.98, 0.99]);

        // The sibling is above every listed issue: the block goes right above it.
        let planned = plan_block(
            &[Some(5.0), Some(6.0)],
            Some(3.0),
            &[Some(3.0), Some(2.995)],
            0.01,
            0.0,
        )
        .unwrap()
        .unwrap();
        assert_eq!(planned.anchor, Some(3.0));
        assert!(planned.values.iter().all(|v| 2.995 < *v && *v < 3.0));
    }

    #[test]
    fn plan_block_skips_a_block_already_above_what_must_stay_below() {
        let planned = plan_block(&[Some(1.0), Some(2.0)], Some(2.5), &[Some(2.5)], 0.01, 0.0);
        assert_eq!(planned.unwrap(), None);
    }

    #[test]
    fn priority_rank_puts_no_priority_last() {
        let ranks: Vec<f64> = [Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(0.0), None]
            .into_iter()
            .map(priority_rank)
            .collect();
        assert!(ranks[..5].windows(2).all(|w| w[0] < w[1]), "{ranks:?}");
        assert_eq!(ranks[4], ranks[5]);
    }

    #[test]
    fn priority_conflicts_only_when_a_higher_priority_is_listed_below() {
        let (none, urgent, high, medium, low) =
            (Some(0.0), Some(1.0), Some(2.0), Some(3.0), Some(4.0));
        // Same priority, or priorities that already go Urgent → No priority: no conflict.
        assert!(priority_conflicts(&[medium, medium, medium], 3).is_empty());
        assert!(priority_conflicts(&[urgent, high, high, low, none], 5).is_empty());
        // No priority above Medium and Low (the E-2400 case): Medium wins, it ranks highest.
        assert_eq!(
            priority_conflicts(&[none, low, medium], 3),
            vec![(0, 2), (1, 2)]
        );
        // Ties pick the first of the highest-priority issues below.
        assert_eq!(priority_conflicts(&[low, high, high], 3), vec![(0, 1)]);
    }

    #[test]
    fn priority_conflicts_checks_listed_issues_against_the_rest() {
        let (high, medium, low) = (Some(2.0), Some(3.0), Some(4.0));
        // `--parent`: listed Medium, Low; unlisted High and Medium siblings follow.
        assert_eq!(
            priority_conflicts(&[medium, low, high, medium], 2),
            vec![(0, 2), (1, 2)]
        );
        // The unlisted siblings' own order is not checked.
        assert!(priority_conflicts(&[high, low, medium], 1).is_empty());
    }

    fn prioritized(identifier: &str, priority: f64, label: &str, value: f64) -> IssueOrder {
        IssueOrder {
            id: Some(identifier.to_string()),
            identifier: Some(identifier.to_string()),
            priority: Some(priority),
            priority_label: Some(label.to_string()),
            priority_sort_order: Some(value),
            ..Default::default()
        }
    }

    #[test]
    fn priority_warning_names_the_pairs_and_the_level_to_beat_unlisted_issues() {
        let issues = vec![
            prioritized("E-1", 0.0, "No priority", 1.0),
            prioritized("E-2", 4.0, "Low", 2.0),
            prioritized("E-3", 3.0, "Medium", 3.0),
            prioritized("E-4", 2.0, "High", 4.0),
        ];
        // All listed: no "unlisted" hint.
        let warning = priority_warning(&issues[..3], 3).unwrap();
        assert!(
            warning.contains("E-1 (No priority) is above E-3 (Medium)"),
            "{warning}"
        );
        assert!(
            warning.contains("E-2 (Low) is above E-3 (Medium)"),
            "{warning}"
        );
        assert!(!warning.contains("unlisted"), "{warning}");

        // `--parent` with E-4 (High) unlisted: name High as the level to use.
        let warning = priority_warning(&issues, 3).unwrap();
        assert!(
            warning.contains("E-3 (Medium) is above E-4 (High, not listed)"),
            "{warning}"
        );
        assert!(warning.contains("High or higher"), "{warning}");

        // A (Low) conflicts with listed B (High) only, but unlisted C (Medium) outranks A.
        let issues2 = vec![
            prioritized("A", 4.0, "Low", 1.0),
            prioritized("B", 2.0, "High", 2.0),
            prioritized("C", 3.0, "Medium", 3.0),
        ];
        let warning = priority_warning(&issues2, 2).unwrap();
        assert!(warning.contains("A (Low) is above B (High)"), "{warning}");
        assert!(warning.contains("use Medium or higher"), "{warning}");

        // A (Medium) above B (High), both listed; unlisted C is Medium. Only listed issues
        // conflict, but --set-priority still needs Medium or higher to stay above C.
        let issues3 = vec![
            prioritized("A", 3.0, "Medium", 1.0),
            prioritized("B", 2.0, "High", 2.0),
            prioritized("C", 3.0, "Medium", 3.0),
        ];
        let warning = priority_warning(&issues3, 2).unwrap();
        assert!(warning.contains("use Medium or higher"), "{warning}");

        // Already in priority order: no warning.
        let in_order = [issues[3].clone(), issues[2].clone(), issues[0].clone()];
        assert_eq!(priority_warning(&in_order, 3), None);
    }

    #[test]
    fn priority_groups_group_listed_issues_and_find_the_topmost_unlisted_value() {
        let issues = vec![
            prioritized("C", 3.0, "Medium", 30.0),
            prioritized("D", 4.0, "Low", 40.0),
            prioritized("B", 3.0, "Medium", 20.0),
            // Unlisted: two Medium siblings, one High.
            prioritized("A", 3.0, "Medium", 10.0),
            prioritized("X", 3.0, "Medium", 50.0),
            prioritized("H", 2.0, "High", 5.0),
        ];
        assert_eq!(
            priority_groups(&issues, 3),
            vec![
                PriorityGroup {
                    priority: Some(3.0),
                    members: vec![0, 2],
                    below: Some(10.0),
                },
                PriorityGroup {
                    priority: Some(4.0),
                    members: vec![1],
                    below: None,
                },
            ]
        );
        // Without --parent every issue is listed: nothing must stay below a group.
        assert!(priority_groups(&issues, 6)
            .iter()
            .all(|g| g.below.is_none()));
    }

    #[test]
    fn plan_priority_groups_ignores_stale_team_copies_of_listed_issues() {
        // A was just changed to Medium; the team query still has it as Low, a hair above the
        // Low group's anchor, where it would be pulled into the C, D block.
        let issues = vec![
            prioritized("A", 3.0, "Medium", 7.0),
            prioritized("C", 4.0, "Low", 1.0),
            prioritized("D", 4.0, "Low", 0.5),
        ];
        let team = vec![prioritized("A", 4.0, "Low", 0.5 - 1e-6)];
        let mut plan = ReorderPlan::new(issues.clone());
        plan.plan_priority_groups(priority_groups(&issues, 3), &team)
            .unwrap();
        assert!(
            plan.values[0].is_empty(),
            "A must not move: {:?}",
            plan.values
        );
        assert_eq!(plan.issues.len(), 3, "nothing pulled");
        let (c, d) = (plan.values[1][0].1, plan.values[2][0].1);
        assert!(c < d && d < 0.5, "C, D above the old anchor: {c}, {d}");
    }

    #[test]
    fn priority_warning_caps_the_pairs() {
        let mut issues: Vec<IssueOrder> = (0..7)
            .map(|i| prioritized(&format!("L{i}"), 4.0, "Low", i as f64))
            .collect();
        issues.push(prioritized("U", 1.0, "Urgent", 9.0));
        let warning = priority_warning(&issues, 8).unwrap();
        assert!(
            warning.contains("L4 (Low) is above U (Urgent)"),
            "{warning}"
        );
        assert!(!warning.contains("L5 (Low)"), "{warning}");
        assert!(warning.contains("; and 2 more"), "{warning}");
    }

    #[test]
    fn priority_warning_does_not_say_higher_than_urgent() {
        let issues = vec![
            prioritized("A", 4.0, "Low", 1.0),
            prioritized("U", 1.0, "Urgent", 2.0),
        ];
        let warning = priority_warning(&issues, 1).unwrap();
        assert!(warning.contains("use Urgent."), "{warning}");
    }

    #[test]
    fn plan_priority_groups_moves_only_groups_out_of_order() {
        // M (Medium) at the bottom of its group, L (Low) far up: `reorder M L` already shows
        // in priority views, and each issue is alone in its group, so nothing moves.
        let issues = vec![
            prioritized("M", 3.0, "Medium", 100.0),
            prioritized("L", 4.0, "Low", 5.0),
        ];
        let team = vec![
            prioritized("M2", 3.0, "Medium", 50.0),
            prioritized("L2", 4.0, "Low", 1.0),
        ];
        let mut plan = ReorderPlan::new(issues.clone());
        plan.plan_priority_groups(priority_groups(&issues, 2), &team)
            .unwrap();
        assert!(plan.values.iter().all(Vec::is_empty), "{:?}", plan.values);

        // --parent with M2 as an unlisted sibling: M goes right above it.
        let mut children = issues.clone();
        children.push(team[0].clone());
        let mut plan = ReorderPlan::new(children.clone());
        plan.plan_priority_groups(priority_groups(&children, 2), &team)
            .unwrap();
        let (key, value) = plan.values[0][0];
        assert_eq!(key, OrderKey::Priority);
        assert!(value < 50.0, "{value}");
        assert!(plan.values[1].is_empty() && plan.values[2].is_empty());
    }
}
