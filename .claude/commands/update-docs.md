Review and update all project documentation to reflect the current codebase.

Documentation files to update (in order):

1. `crates/lineark-sdk/README.md` — SDK crate README (published to crates.io)
2. `crates/lineark/README.md` — CLI crate README (published to crates.io)
3. `README.md` — Top-level project README (GitHub landing page)

Steps:

1. Read all three README files listed above.
2. Read these source files to understand the current state of the codebase:
   - `crates/lineark-sdk/src/field_selection.rs` — GraphQLFields trait, FieldCompatible
   - `crates/lineark-sdk/src/lib.rs` — public re-exports
   - `crates/lineark-derive/src/lib.rs` — derive macro capabilities
   - `crates/lineark/src/commands/mod.rs` — CLI command list
   - `crates/lineark/src/commands/viewer.rs` — example of lean type with full_type
   - `crates/lineark-sdk/src/generated/client_impl.rs` — available query/mutation methods
   - `crates/lineark-sdk/src/blocking_client.rs` — blocking API surface
3. For each README, check that:
   - Code examples compile against the current API (generic queries need turbofish or type inference)
   - All available commands/methods are listed
   - Feature descriptions match actual capabilities (e.g., compile-time validation, custom field selection, FullType constraints)
   - The three READMEs are consistent with each other (don't contradict)
4. Update each README with accurate information. Keep the existing structure and tone. Don't bloat — be concise.
5. Run `cargo fmt` if any code examples were changed.
6. Report what changed in each file.

Key things to verify are accurate:
- Query methods table (SDK README) — return types, generic signatures
- Mutation methods table (SDK README) — generic signatures
- CLI command table (top-level README + CLI README) — all subcommands listed
- Custom field selection examples — `#[graphql(full_type = X)]`, `#[graphql(nested)]` attributes
- The `search_issues` return type is `Connection<IssueSearchResult>`, NOT `Connection<Issue>`
- Mutations are generic: `client.issue_create::<Issue>(input)` or with type inference
- Blocking client reflects async client's full surface
