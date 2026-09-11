# Contributing to lineark

## Collaboration

We prefer small, reviewable PRs and keep design discussion in GitHub issues or discussions.

For non-trivial changes only, open an issue or discussion first, agree on the approach, then open a PR.

For trivial changes, such as typo fixes, docs, or small self-contained bug fixes, you can open a PR directly.

Consider a change non-trivial if it:

- Adds or changes a public API or CLI command
- Changes generated code, codegen, or the SDK
- Adds a new crate or dependency
- Has a meaningful design decision

## Pull requests

1. Reference the issue or discussion in the PR description.
2. Keep the change focused and explain what it does and why.
3. Run `cargo fmt`, `cargo clippy --workspace --all-targets`, and `cargo test --workspace`.
4. Update docs and examples when behavior changes.
5. Respond to review feedback and iterate on the PR.

Online tests may require maintainer credentials. For fork PRs, a maintainer can label the PR `safe-to-test` to run the online test workflow.

Thanks for contributing!
