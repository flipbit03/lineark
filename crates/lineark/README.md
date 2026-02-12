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

## Usage

```
lineark whoami                                   Show authenticated user
lineark teams list                               List all teams
lineark users list [--active]                    List users
lineark projects list                            List all projects
lineark labels list                              List issue labels
lineark cycles list [--limit N]                  List cycles
lineark cycles read <ID>                         Read a specific cycle
lineark issues list [--limit N] [--team KEY]     Active issues, newest first
  [--mine]                                       Only issues assigned to me
  [--show-done]                                  Include done/canceled issues
lineark issues read <IDENTIFIER>                 Full issue detail (e.g., E-929)
lineark issues search <QUERY> [--limit N]        Full-text search
  [--show-done]                                  Include done/canceled results
lineark usage                                    Compact command reference
```

Every command supports `--help` for full details.

## Output format

Output auto-detects: human-readable tables in a terminal, JSON when piped. Override with `--format {human,json}`.

```sh
lineark teams list                  # table in terminal
lineark teams list | jq .           # JSON when piped
lineark teams list --format json    # force JSON
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
