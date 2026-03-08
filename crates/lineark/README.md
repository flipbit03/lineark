# lineark

CLI for the [Linear](https://linear.app) issue tracker — for humans and LLMs.

Part of the [lineark](https://github.com/flipbit03/lineark) project — an unofficial Linear ecosystem for Rust.

## Install

### Pre-built binary (fastest)

```sh
curl -fsSL https://raw.githubusercontent.com/flipbit03/lineark/main/install.sh | sh
```

### Via cargo

```sh
cargo install lineark
```

### Download manually

Grab a binary from the [latest release](https://github.com/flipbit03/lineark/releases/latest).

## Setup

Create a [Linear API token](https://linear.app/settings/account/security) and save it:

```sh
echo "lin_api_..." > ~/.linear_api_token
```

Or use an environment variable (`LINEAR_API_TOKEN`) or the `--api-token` flag.

### Multiple profiles

Store tokens for different workspaces in named files:

```sh
echo "lin_api_..." > ~/.linear_api_token            # "default" API token
echo "lin_api_..." > ~/.linear_api_token_work       # "work" API token
echo "lin_api_..." > ~/.linear_api_token_freelance  # "freelance" API token
```

Then switch with `--profile`:

```sh
lineark whoami                             # uses ~/.linear_api_token (default)
lineark --profile work issues list --mine  # uses ~/.linear_api_token_work
lineark --profile freelance whoami         # uses ~/.linear_api_token_freelance
```

## Usage

Most flags accept human-readable names or UUIDs — `--team` accepts key/name/UUID, `--assignee` accepts user name/display name, `--labels` accepts label names, `--project` and `--cycle` accept names. `me` is a special alias that resolves to the authenticated user on `--assignee`, `--lead`, and `--members`.

```
lineark whoami                                   Show authenticated user
lineark teams list                               List all teams
lineark teams read <ID>                          Full team detail (members, settings)
lineark teams create <NAME>                      Create a team
  [--key KEY] [--description TEXT] ...           See --help for all options
lineark teams update <ID>                        Update a team
  [--name NAME] [--key KEY] ...                  See --help for all options
lineark teams delete <ID>                        Delete a team
lineark teams members add <TEAM> --user USER     Add a user to a team
lineark teams members remove <TEAM> --user USER  Remove a user from a team
lineark users list [--active]                    List users
lineark projects list [--led-by-me]              List all projects (with lead)
lineark projects read <NAME-OR-ID>               Full project detail (lead, members, status, dates, teams)
lineark projects create <NAME> --team KEY        Create a project
  [--description TEXT] [--lead NAME-OR-ID|me]    Description, lead, dates
  [--members NAME,...|me]                        Project members (comma-separated)
  [--start-date DATE] [--target-date DATE]       Priority, content, icon, color
  [-p 0-4] [--content TEXT] ...                  See --help for all options
lineark labels list [--team KEY]                 List labels (group, team, parent)
lineark labels create <NAME>                     Create a label
  [--team KEY] [--color HEX]                     Team, color
  [--description TEXT]                           Description
  [--parent-label-group ID]                      Nest under a group label
  [--make-label-group]                           Create as a group label
lineark labels update <ID>                       Update a label
  [--name TEXT] [--color HEX]                    Name, color
  [--parent-label-group ID]                      Nest under a group label
  [--clear-parent-label-group]                   Remove parent group
  [--make-label-group] [--clear-label-group]     Promote/demote group
lineark labels delete <ID>                       Delete a label
lineark cycles list [-l N] [--team KEY]          List cycles
  [--active] [--around-active N]                 Active cycle / ± N neighbors
lineark cycles read <ID> [--team KEY]            Read cycle (UUID, name, number)
lineark issues list [-l N] [--team KEY]          Active issues, newest first
  [--mine] [--show-done]                         Filter by assignee / state
lineark issues read <IDENTIFIER>                 Full issue detail incl. sub-issues & comments
lineark issues find-branch <BRANCH>              Find issue by Git branch name
lineark issues search <QUERY> [-l N]             Full-text search
  [--team KEY] [--assignee NAME-OR-ID|me]        Filter by team, assignee, status
  [--status NAME,...] [--show-done]
lineark issues create <TITLE> --team KEY         Create an issue
  [-p 0-4] [-e N] [--assignee NAME-OR-ID|me]     Priority, estimate, assignee
  [--labels NAME,...] [-s NAME] ...              Labels, status — see --help
lineark issues update <IDENTIFIER>               Update an issue
  [-s NAME] [-p 0-4] [-e N]                      Status, priority, estimate
  [--assignee NAME-OR-ID|me]                     Assignee
  [--clear-parent] [--project NAME-OR-ID] ...    See --help for all options
lineark issues batch-update ID [ID ...]          Batch update multiple issues
  [-s NAME] [-p 0-4] [--assignee NAME-OR-ID|me]  Status, priority, assignee
lineark issues archive <IDENTIFIER>              Archive an issue
lineark issues unarchive <IDENTIFIER>            Unarchive an issue
lineark issues delete <IDENTIFIER>               Delete (trash) an issue
lineark comments create <ISSUE-ID> --body TEXT   Comment on an issue
lineark comments update <COMMENT-UUID>           Update a comment
  --body TEXT                                    New body in markdown
lineark comments resolve <COMMENT-UUID>          Resolve a comment thread
lineark comments unresolve <COMMENT-UUID>        Unresolve a comment thread
lineark comments delete <ID>                     Delete a comment
lineark relations create <ISSUE>                 Create an issue relation
  --blocks <ISSUE>                               Source blocks target
  --blocked-by <ISSUE>                           Source is blocked by target
  --related/--duplicate/--similar <ISSUE>        Other relation types
lineark relations delete <RELATION-UUID>         Delete an issue relation
lineark documents list [--limit N]               List documents (lean output)
  [--project NAME-OR-ID] [--issue ID]            Filter by project or issue
lineark documents read <ID>                      Read document (includes content)
lineark documents create --title TEXT            Create a document
  [--project NAME-OR-ID] [--issue ID]
lineark documents update <ID>                    Update a document
lineark documents delete <ID>                    Delete a document
lineark project-milestones list --project NAME   List milestones for a project
lineark project-milestones read <ID>             Read a milestone
lineark project-milestones create <NAME>         Create a milestone
  --project NAME-OR-ID [--target-date DATE]
lineark project-milestones update <ID>           Update a milestone
lineark project-milestones delete <ID>           Delete a milestone
lineark embeds upload <FILE> [--public]          Upload file, get asset URL
lineark embeds download <URL> [--output PATH]    Download a file by URL
lineark self update                              Update to latest release
lineark usage                                    Compact command reference
```

Every command supports `--help` for full details.

## Output format

Output auto-detects: human-readable tables in a terminal, JSON when piped. Override with `--format {human,json}`.

```sh
lineark teams list                               # table in terminal
lineark teams list | jq .                        # JSON when piped
lineark teams list --format json                 # force JSON
```

## LLM / AI agent setup

Add this to your LLM's context (e.g. `CLAUDE.md`, `.cursorrules`, system prompt):

```
We use the `lineark` CLI tool for communicating with Linear. Use your Bash tool
to call the `lineark` executable. Run `lineark usage` to see usage information.
```

`lineark usage` gives your agent a complete command reference in under 1,000 tokens.

## License

MIT
