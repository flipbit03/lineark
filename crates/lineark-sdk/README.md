# lineark-sdk

Typed, async-first Rust SDK for the [Linear](https://linear.app) GraphQL API.

Part of the [lineark](https://github.com/flipbit03/lineark) project — an unofficial Linear ecosystem for Rust.

## Install

```sh
cargo add lineark-sdk
```

## Quick start

```rust
use lineark_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::auto()?;

    let me = client.whoami().await?;
    println!("Logged in as: {}", me.name.as_deref().unwrap_or("?"));

    let teams = client.teams(None, None, None, None, None).await?;
    for team in &teams.nodes {
        println!("{}: {}",
            team.key.as_deref().unwrap_or("?"),
            team.name.as_deref().unwrap_or("?"),
        );
    }

    Ok(())
}
```

## Authentication

Create a [Linear API token](https://linear.app/settings/account/security) and provide it via any of these methods (in priority order):

| Method | Example |
|--------|---------|
| Direct | `Client::from_token("lin_api_...")` |
| Env var | `export LINEAR_API_TOKEN="lin_api_..."` then `Client::from_env()` |
| File | `echo "lin_api_..." > ~/.linear_api_token` then `Client::from_file()` |
| Auto | `Client::auto()` — tries env var, then file |

## Available queries

| Method | Returns | Description |
|--------|---------|-------------|
| `viewer()` | `User` | Authenticated user |
| `teams(...)` | `Connection<Team>` | List teams |
| `team(id)` | `Team` | Get team by ID |
| `users(...)` | `Connection<User>` | List users |
| `projects(...)` | `Connection<Project>` | List projects |
| `project(id)` | `Project` | Get project by ID |
| `issues(...)` | `Connection<Issue>` | List issues |
| `issue(id)` | `Issue` | Get issue by ID |
| `search_issues(...)` | `Connection<Issue>` | Full-text issue search |
| `issue_labels(...)` | `Connection<IssueLabel>` | List labels |
| `cycles(...)` | `Connection<Cycle>` | List cycles |
| `cycle(id)` | `Cycle` | Get cycle by ID |
| `workflow_states(...)` | `Connection<WorkflowState>` | List workflow states |

Connection queries accept optional pagination parameters (`first`, `after`, `last`, `before`).

## Error handling

All methods return `Result<T, LinearError>`. Error variants:

- `Authentication` — invalid or expired token
- `Forbidden` — insufficient permissions
- `RateLimited` — API rate limit hit (includes `retry_after`)
- `InvalidInput` — bad request parameters
- `GraphQL` — errors returned in the GraphQL response
- `Network` — connection/transport failures
- `HttpError` — non-200 responses not covered above

## Codegen

All types, enums, inputs, and query functions are generated from Linear's official GraphQL schema. The generated code lives in `src/generated/` and is checked in for reproducible builds.

## License

MIT
