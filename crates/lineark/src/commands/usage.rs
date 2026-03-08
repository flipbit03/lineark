use crate::version_check;

/// Print a compact LLM-friendly command reference (<1000 tokens).
pub async fn run() {
    let env_hint = if std::env::var("LINEAR_API_TOKEN").is_ok() {
        " (set)"
    } else {
        ""
    };
    let file_hint = if std::env::var("HOME")
        .map(|h| std::path::Path::new(&h).join(".linear_api_token").exists())
        .unwrap_or(false)
    {
        " (found)"
    } else {
        ""
    };

    print!(
        r#"lineark — Linear CLI for humans and LLMs

NAME RESOLUTION: Most flags accept names or UUIDs. Team flags accept key, name,
or UUID. --assignee accepts user name/display name. --labels accepts label names.
--project and --cycle accept names. UUIDs always work as fallback.
`me` is a special alias that resolves to the authenticated user at runtime.
It works on --assignee, --lead, and --members (case-insensitive).

COMMANDS:
  lineark whoami                                   Show authenticated user
  lineark teams list                               List all teams
  lineark teams read <KEY-OR-ID>                   Full team detail (members, settings)
  lineark teams create|update|delete ...           Manage teams (--help for flags)
  lineark teams members add|remove ...             Manage membership (--help for flags)
  lineark users list [--active]                    List users
  lineark projects list [--led-by-me]              List all projects (with lead)
  lineark projects read <NAME-OR-ID>               Full project detail (lead, members, status, dates, teams)
  lineark projects create <NAME> --team KEY ...    Create project (--help for flags)
  lineark labels list [--team KEY]                 List labels (group, team, parent, color)
  lineark labels create|update|delete ...          Manage labels (--help for flags)
  lineark cycles list [-l N] [--team KEY]          List cycles
    [--active] [--around-active N]                 Active cycle / ± N neighbors
  lineark cycles read <ID> [--team KEY]            Read cycle (UUID, name, or number)
  lineark issues list [-l N] [--team KEY]          Active issues (done/canceled hidden), newest first
    [--project NAME-OR-ID] [--mine]                Filter by project, assignee
    [--show-done]                                  Include done/canceled issues
  lineark issues read <IDENTIFIER>                 Full issue detail incl. sub-issues, comments, relations
  lineark issues find-branch <BRANCH>              Find issue by Git branch name
  lineark issues search <QUERY> [-l N]             Full-text search
    [--team KEY] [--assignee NAME-OR-ID|me]        Filter by team, assignee, status
    [--status NAME,...] [--show-done]              Comma-separated status names
  lineark issues create <TITLE> --team KEY         Create an issue
    [-p 0-4] [-e N] [--assignee NAME-OR-ID|me]     0=none 1=urgent 2=high 3=medium 4=low
    [--labels NAME,...] [-d TEXT] [-s NAME]        Label names, status name
    [--parent ID] [--project NAME-OR-ID]           Parent issue, project, cycle
    [--cycle NAME-OR-ID]
  lineark issues update <IDENTIFIER>               Update an issue
    [-s NAME] [-p 0-4] [-e N]                      Status, priority, estimate
    [--assignee NAME-OR-ID|me]                     Assignee
    [--labels NAME,...] [--label-by adding|replacing|removing]
    [--clear-labels] [-t TEXT] [-d TEXT]           Title, description
    [--parent ID] [--clear-parent]                 Set or remove parent
    [--project NAME-OR-ID] [--cycle NAME-OR-ID]    Project, cycle
  lineark issues batch-update ID [ID ...]          Batch update (--help for flags)
  lineark issues archive|unarchive|delete ...      Lifecycle ops (--help for flags)
  lineark comments create <ISSUE-ID> --body TEXT   Comment on an issue
  lineark comments update|resolve|unresolve|delete Manage comments (--help for flags)
  lineark relations create|delete ...              Issue relations (--help for flags)
  lineark documents list [--limit N]               List documents (lean output)
    [--project NAME-OR-ID] [--issue ID]            Filter by project or issue
  lineark documents read <ID>                      Read document (includes content)
  lineark documents create|update|delete ...       Manage documents (--help for flags)
  lineark project-milestones ...                   Milestones CRUD (--help for flags)
  lineark embeds upload|download ...               File embeds (--help for flags)
  lineark self update [--check]                    Update lineark / check for updates

GLOBAL OPTIONS:
  --api-token <TOKEN>   Override API token
  --format human|json   Force output format (auto-detected by default)

AUTH (in precedence order):
  1. --api-token flag
  2. $LINEAR_API_TOKEN env var{env_hint}
  3. ~/.linear_api_token file{file_hint}
"#
    );

    // Show update hint (uses cache, goes online at most once per 24h).
    if !version_check::is_dev_build() {
        let latest = version_check::get_latest_version(false).await;
        let hint = crate::format_update_hint(latest.as_deref());
        if !hint.is_empty() {
            print!("{hint}");
        }
    }
}
