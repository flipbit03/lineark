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
  lineark teams create <NAME>                      Create a team
    [--key KEY] [--description TEXT]               Key (auto-generated if omitted)
    [--icon ICON] [--color COLOR]                  Icon, color
    [--timezone TZ] [--private]                    Timezone, private flag
    [--cycles-enabled] [--triage-enabled]          Enable cycles/triage
  lineark teams update <KEY-OR-ID>                 Update a team
    [--name NAME] [--description TEXT] ...         (same flags as create, all optional)
  lineark teams delete <KEY-OR-ID>                 Delete a team
  lineark teams members add <TEAM> --user NAME     Add member to team
  lineark teams members remove <TEAM> --user NAME  Remove member from team
  lineark users list [--active]                    List users
  lineark projects list [--led-by-me]              List all projects (with lead)
  lineark projects read <NAME-OR-ID>               Full project detail (lead, members, status, dates, teams)
  lineark projects create <NAME> --team KEY[,KEY]  Create a new project
    [--description TEXT] [--lead NAME-OR-ID|me]    Description, project lead
    [--members NAME,...|me]                        Project members (comma-separated)
    [--start-date DATE] [--target-date DATE]       Dates (YYYY-MM-DD)
    [-p 0-4] [--content TEXT]                      Priority, markdown content
    [--icon ICON] [--color COLOR]                  Icon, color
  lineark labels list [--team KEY]                 List issue labels (includes team key)
  lineark cycles list [-l N] [--team KEY]          List cycles
    [--active]                                     Only the active cycle
    [--around-active N]                            Active ± N neighbors
  lineark cycles read <ID> [--team KEY]            Read cycle (UUID, name, or number)
  lineark issues list [-l N] [--team KEY]          Active issues (done/canceled hidden), newest first
    [--mine]                                       Only issues assigned to me
    [--show-done]                                  Include done/canceled issues
  lineark issues read <IDENTIFIER>                 Full issue detail incl. sub-issues, comments, relations
  lineark issues search <QUERY> [-l N]             Full-text search
    [--team KEY] [--assignee NAME-OR-ID|me]        Filter by team, assignee, status
    [--status NAME,...] [--show-done]              Comma-separated status names
  lineark issues create <TITLE> --team KEY         Create an issue
    [-p 0-4] [--assignee NAME-OR-ID|me]            0=none 1=urgent 2=high 3=medium 4=low
    [--labels NAME,...] [-d TEXT] [-s NAME]        Label names (team-scoped), status name
    [--parent ID] [--project NAME-OR-ID]           Parent issue, project, cycle
    [--cycle NAME-OR-ID]
  lineark issues update <IDENTIFIER>               Update an issue
    [-s NAME] [-p 0-4] [--assignee NAME-OR-ID|me]  Status, priority, assignee
    [--labels NAME,...] [--label-by adding|replacing|removing]
    [--clear-labels] [-t TEXT] [-d TEXT]           Title, description
    [--parent ID] [--clear-parent]                 Set or remove parent
    [--project NAME-OR-ID] [--cycle NAME-OR-ID]    Project, cycle
  lineark issues archive <IDENTIFIER>              Archive an issue
  lineark issues unarchive <IDENTIFIER>            Unarchive a previously archived issue
  lineark issues delete <IDENTIFIER>               Delete (trash) an issue
    [--permanently]                                Permanently delete instead of trashing
  lineark comments create <ISSUE-ID> --body TEXT   Comment on an issue
  lineark comments delete <COMMENT-UUID>           Delete a comment
  lineark relations create <ISSUE>                 Create an issue relation
    --blocks <ISSUE>                               Source blocks target
    --blocked-by <ISSUE>                           Source is blocked by target
    --related <ISSUE>                              Mark issues as related
    --duplicate <ISSUE>                            Mark as duplicate
    --similar <ISSUE>                              Mark as similar
  lineark relations delete <RELATION-UUID>         Delete an issue relation
  lineark documents list [--limit N]               List documents (lean output)
    [--project NAME-OR-ID] [--issue ID]            Filter by project or issue
  lineark documents read <ID>                      Read document (includes content)
  lineark documents create --title TEXT            Create a document
    [--content TEXT] [--project NAME-OR-ID]        Project name or UUID
    [--issue ID]
  lineark documents update <ID>                    Update a document
    [--title TEXT] [--content TEXT]
  lineark documents delete <ID>                    Delete (trash) a document
  lineark project-milestones list --project NAME   List milestones for a project
  lineark project-milestones read <ID>             Read a milestone (UUID or name with --project)
    [--project NAME-OR-ID]
  lineark project-milestones create <NAME>         Create a milestone
    --project NAME-OR-ID [--target-date DATE]      DATE = YYYY-MM-DD
    [--description TEXT]
  lineark project-milestones update <ID>           Update a milestone
    [--project NAME-OR-ID] [--name TEXT]
    [--target-date DATE] [--description TEXT]
  lineark project-milestones delete <ID>           Delete a milestone
    [--project NAME-OR-ID]
  lineark initiatives list [-l N]                  List all initiatives
  lineark initiatives read <NAME-OR-ID>            Full initiative detail (owner, projects, dates)
  lineark initiatives create <NAME>                Create an initiative
    [--description TEXT] [--owner NAME-OR-ID|me]   Description, initiative owner
    [--status Planned|Active|Completed]            Status
    [--target-date DATE] [--color HEX] [--icon IC] Target date, color, icon
  lineark initiatives update <NAME-OR-ID>          Update an initiative
    [--name TEXT] [--description TEXT]             Name, description
    [--owner NAME-OR-ID|me] [--status STATUS]      Owner, status
    [--target-date DATE]                           Target date
  lineark initiatives archive <NAME-OR-ID>         Archive an initiative
  lineark initiatives unarchive <NAME-OR-ID>       Unarchive an initiative
  lineark initiatives delete <NAME-OR-ID>          Delete an initiative
  lineark initiatives projects add <INIT>          Link a project to an initiative
    --project NAME-OR-ID
  lineark initiatives projects remove <INIT>       Unlink a project from an initiative
    --project NAME-OR-ID
  lineark embeds upload <FILE> [--public]          Upload file to Linear, returns asset URL
                                                   Embed as markdown [name](url) in issues,
                                                   comments, or documents
                                                   --public only works for images (not SVG)
  lineark embeds download <URL>                    Download any file by URL (works with
    [--output PATH] [--overwrite]                  Linear CDN URLs and external URLs alike)
  lineark self update                              Update lineark to the latest release
  lineark self update --check                      Check if an update is available

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
