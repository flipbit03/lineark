/// Print a compact LLM-friendly command reference (<1000 tokens).
pub fn run() {
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

COMMANDS:
  lineark whoami                                   Show authenticated user
  lineark teams list                               List all teams
  lineark users list [--active]                    List users
  lineark projects list                            List all projects
  lineark labels list                              List issue labels
  lineark cycles list [--limit N] [--team KEY]     List cycles
    [--active]                                     Only the active cycle
    [--around-active N]                            Active ± N neighbors
  lineark cycles read <ID> [--team KEY]            Read cycle (UUID, name, or number)
  lineark issues list [--limit N] [--team KEY]     Active issues (done/canceled hidden), newest first
    [--mine]                                       Only issues assigned to me
    [--show-done]                                  Include done/canceled issues
  lineark issues read <IDENTIFIER>                 Full issue detail (e.g., E-929) incl. attachments & relations
  lineark issues search <QUERY> [--limit N]        Full-text search
    [--show-done]                                  Include done/canceled results
  lineark issues create <TITLE> --team KEY         Create an issue
    [--priority 0-4] [--assignee ID]               0=none 1=urgent 2=high 3=medium 4=low
    [--labels ID,...] [--description TEXT]         Comma-separated label UUIDs
    [--status NAME] [--parent ID]                  Status resolved against team states
  lineark issues update <IDENTIFIER>               Update an issue
    [--status NAME] [--priority 0-4]               Status resolved against team states
    [--assignee ID] [--parent ID]                  User UUID or issue identifier
    [--labels ID,...] [--label-by adding|replacing|removing]
    [--clear-labels] [--title TEXT] [--description TEXT]
  lineark issues archive <IDENTIFIER>              Archive an issue
  lineark issues unarchive <IDENTIFIER>            Unarchive a previously archived issue
  lineark issues delete <IDENTIFIER>               Delete (trash) an issue
    [--permanently]                                Permanently delete instead of trashing
  lineark comments create <ISSUE-ID> --body TEXT   Comment on an issue
  lineark documents list [--limit N]               List documents
  lineark documents read <ID>                      Read document (includes content)
  lineark documents create --title TEXT            Create a document
    [--content TEXT] [--project ID] [--issue ID]
  lineark documents update <ID>                    Update a document
    [--title TEXT] [--content TEXT]
  lineark documents delete <ID>                    Delete (trash) a document
  lineark embeds upload <FILE> [--public]          Upload file to Linear, returns asset URL
                                                   Embed as markdown [name](url) in issues,
                                                   comments, or documents
  lineark embeds download <URL>                    Download any file by URL (works with
    [--output PATH] [--overwrite]                  Linear CDN URLs and external URLs alike)

GLOBAL OPTIONS:
  --api-token <TOKEN>   Override API token
  --format human|json   Force output format (auto-detected by default)

AUTH (in precedence order):
  1. --api-token flag
  2. $LINEAR_API_TOKEN env var{env_hint}
  3. ~/.linear_api_token file{file_hint}
"#
    );
}
