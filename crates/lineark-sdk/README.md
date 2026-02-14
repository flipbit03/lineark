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

    let teams = client.teams().first(10).send().await?;
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

## Queries

Collection queries use a builder pattern with optional pagination and filtering:

```rust
// Paginate with first/last/after/before
let issues = client.issues().first(25).send().await?;
let page2 = client.issues().first(25).after(issues.page_info.end_cursor.unwrap()).send().await?;

// Search with extra filters
let results = client.search_issues("bug")
    .first(10)
    .team_id("team-uuid")
    .include_comments(true)
    .send()
    .await?;
```

| Method | Returns | Description |
|--------|---------|-------------|
| `whoami()` | `User` | Authenticated user |
| `teams()` | `Connection<Team>` | List teams |
| `team(id)` | `Team` | Get team by ID |
| `users()` | `Connection<User>` | List users |
| `projects()` | `Connection<Project>` | List projects |
| `project(id)` | `Project` | Get project by ID |
| `issues()` | `Connection<Issue>` | List issues |
| `issue(id)` | `Issue` | Get issue by ID |
| `search_issues(term)` | `Connection<Issue>` | Full-text issue search |
| `issue_labels()` | `Connection<IssueLabel>` | List labels |
| `cycles()` | `Connection<Cycle>` | List cycles |
| `cycle(id)` | `Cycle` | Get cycle by ID |
| `documents()` | `Connection<Document>` | List documents |
| `document(id)` | `Document` | Get document by ID |
| `issue_relations()` | `Connection<IssueRelation>` | List issue relations |
| `issue_relation(id)` | `IssueRelation` | Get issue relation by ID |
| `workflow_states()` | `Connection<WorkflowState>` | List workflow states |

All collection queries support `.first(n)`, `.last(n)`, `.after(cursor)`, `.before(cursor)`, and `.include_archived(bool)`.

## Custom field selection

All queries are generic over `T: DeserializeOwned + GraphQLFields`. By default they use the generated types (which fetch all scalar fields), but you can define custom lean structs to fetch only the fields you need:

```rust
use lineark_sdk::{Client, GraphQLFields};
use serde::Deserialize;

#[derive(Deserialize, GraphQLFields)]
#[serde(rename_all = "camelCase")]
struct LeanIssue {
    id: Option<String>,
    title: Option<String>,
}

let client = Client::auto()?;
let issues = client.issues::<LeanIssue>().first(10).send().await?;
for issue in &issues.nodes {
    println!("{}", issue.title.as_deref().unwrap_or("?"));
}
```

The `#[derive(GraphQLFields)]` macro generates a `selection()` method from the struct's field names, so the query fetches exactly those fields — no overfetching. For nested objects, annotate with `#[graphql(nested)]`:

```rust
#[derive(Deserialize, GraphQLFields)]
#[serde(rename_all = "camelCase")]
struct IssueWithTeam {
    id: Option<String>,
    title: Option<String>,
    #[graphql(nested)]
    team: Option<LeanTeam>,
}

#[derive(Deserialize, GraphQLFields)]
#[serde(rename_all = "camelCase")]
struct LeanTeam {
    id: Option<String>,
    key: Option<String>,
}
```

## Mutations

```rust
use lineark_sdk::generated::inputs::IssueCreateInput;

let payload = client.issue_create(IssueCreateInput {
    title: Some("Fix the bug".to_string()),
    team_id: Some("team-uuid".to_string()),
    priority: Some(2),
    ..Default::default()
}).await?;
```

| Method | Description |
|--------|-------------|
| `issue_create(input)` | Create an issue |
| `issue_update(input, id)` | Update an issue |
| `issue_archive(trash, id)` | Archive an issue |
| `issue_unarchive(id)` | Unarchive a previously archived issue |
| `issue_delete(permanently, id)` | Delete an issue |
| `comment_create(input)` | Create a comment |
| `document_create(input)` | Create a document |
| `document_update(input, id)` | Update a document |
| `document_delete(id)` | Delete a document |
| `issue_relation_create(override_created_at, input)` | Create an issue relation |
| `file_upload(meta, public, size, type, name)` | Request a signed upload URL |
| `image_upload_from_url(url)` | Upload image from URL |

## File upload and download

The SDK provides high-level helpers for Linear's file operations:

```rust
// Upload a file (two-step: get signed URL from Linear, then PUT to GCS)
let bytes = std::fs::read("screenshot.png")?;
let result = client.upload_file("screenshot.png", "image/png", bytes, false).await?;
println!("Asset URL: {}", result.asset_url);

// Download a file from Linear's CDN
let result = client.download_url("https://uploads.linear.app/...").await?;
std::fs::write("output.png", &result.bytes)?;
```

## Blocking (synchronous) API

For non-async contexts, enable the `blocking` feature:

```toml
[dependencies]
lineark-sdk = { version = "...", features = ["blocking"] }
```

The blocking client mirrors the async API exactly:

```rust
use lineark_sdk::blocking_client::Client;

let client = Client::auto()?;

let me = client.whoami()?;
println!("Logged in as: {:?}", me.name);

let teams = client.teams().first(10).send()?;
for team in &teams.nodes {
    println!("{}: {}",
        team.key.as_deref().unwrap_or("?"),
        team.name.as_deref().unwrap_or("?"),
    );
}

// Mutations work the same way
let payload = client.document_create(input)?;

// File operations too
let result = client.upload_file("file.txt", "text/plain", bytes, false)?;
let downloaded = client.download_url(&result.asset_url)?;
```

## Error handling

All methods return `Result<T, LinearError>`. Error variants:

- `Authentication` — invalid or expired token
- `Forbidden` — insufficient permissions
- `RateLimited` — API rate limit hit (includes `retry_after`)
- `InvalidInput` — bad request parameters
- `GraphQL` — errors returned in the GraphQL response (includes query name for diagnostics)
- `Network` — connection/transport failures
- `HttpError` — non-200 responses not covered above
- `MissingData` — expected data path not found in response
- `AuthConfig` — auth configuration error (no token found)
- `Internal` — internal error (e.g. runtime creation failure)

## Codegen

All types, enums, inputs, and query functions are generated from Linear's official GraphQL schema. The generated code lives in `src/generated/` and is checked in for reproducible builds.

## License

MIT
