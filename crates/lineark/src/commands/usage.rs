/// Print a compact LLM-friendly command reference (<1000 tokens).
pub fn run() {
    print!(
        r#"lineark — Linear CLI for humans and LLMs

COMMANDS:
  lineark whoami                                   Show authenticated user
  lineark teams list                               List all teams
  lineark users list [--active]                    List users
  lineark projects list                            List all projects
  lineark labels list                              List issue labels
  lineark cycles list [--limit N]                  List cycles
  lineark cycles read <ID>                         Read a specific cycle
  lineark issues list [--limit N] [--team KEY]     Active issues (done/canceled hidden), newest first
    [--mine]                                       Only issues assigned to me
    [--show-done]                                  Include done/canceled issues
  lineark issues read <IDENTIFIER>                 Full issue detail with assignee, state, labels (e.g., E-929)
  lineark issues search <QUERY> [--limit N]        Full-text search across titles and descriptions
    [--show-done]                                  Include done/canceled results

GLOBAL OPTIONS:
  --api-token <TOKEN>   Override API token
  --format human|json   Force output format (auto-detected by default)

AUTH (in precedence order):
  1. --api-token flag
  2. $LINEAR_API_TOKEN env var
  3. ~/.linear_api_token file

OUTPUT:
  Terminal → human-readable tables
  Piped    → JSON
"#
    );
}
