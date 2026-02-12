Update the Linear GraphQL schema and fix any resulting breakage.

Steps:
1. Run `cargo run -p lineark-codegen -- --fetch` to fetch the latest schema from Linear's public introspection endpoint and regenerate all SDK types.
2. Run `cargo build --workspace` to check if everything compiles.
3. If there are compilation errors (removed fields, renamed types, new required fields, etc.), fix them:
   - If a field was removed from the schema but is referenced in hand-written CLI code (`crates/lineark/src/commands/`), remove the reference.
   - If a field was removed but is still in a generated query's EXCLUDED_FIELDS list (`crates/lineark-codegen/src/emit_queries.rs`), remove it from the list and re-run codegen.
   - If a field exposed in introspection causes API errors (like `featureFlags`), add it to EXCLUDED_FIELDS and re-run codegen.
4. Run `cargo clippy --workspace -- -D warnings` and fix any warnings.
5. Run `cargo fmt` to fix formatting.
6. Run `cargo test --workspace` to verify tests pass.
7. Report what changed: new/removed types, fields that needed fixing, any EXCLUDED_FIELDS changes.
