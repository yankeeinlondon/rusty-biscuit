# Dependencies

`unicode-normalization` 0.1 supplies Rust identifier NFC comparison. Its pure
Rust implementation works across supported platforms, and it was already in
the workspace dependency graph. ECMAScript identifiers use a separate escape
decoder because literal composed and decomposed Unicode identifiers are distinct.

`tree-hugger-cli` depends on `sniff` via `../../sniff/lib` with
`default-features = false` for `GitRepo::discover` and
`GitRepo::changed_paths(ChangedBase)`, plus stats-only
`detect_repo_structure_or_root_package`, `PackageNameSource`, and the reusable
`PackageOwnershipLookup`. Stats captures one catalog and lookup; it requests
no inventory or lockfile enrichment. No `network`, `remote`, or `metrics`
feature is requested. The focused API uses sniff's existing pinned gix
backend for local status and direct commit-tip comparison; it adds no Git
subprocess or new gix feature. The tree-hugger library has no sniff edge.

CLI changed-file fixtures use `git2` 0.20.3 with default features off as a
dev-dependency for commits, branch checkout, and linked worktrees without
host signing or credentials. The `libgit2-sys >=0.18.3, <0.18.6` constraint
matches sniff's Windows MSVC compatibility pin. Neither is a production
Git backend.

The CLI uses `pulldown-cmark` 0.13 as a dev-dependency to parse Markdown export
output in integration tests. Runtime Markdown writing uses the CLI's escaping
adapter and adds no parser dependency to the shipped binary.

`tree-hugger-cli` depends on the workspace `renderable` crate (path
`../../renderable`, default features) for the render-tree node types
(`renderable::tree::{RenderNode, NodeKind}`) that biscuit-terminal's
`InlineProse::to_render_nodes` returns. The Markdown export and plain Prose export
read the listing's generated Prose markup through those nodes instead of
scanning text. biscuit-terminal already depends on `renderable`, so the
build graph gains no new crate; the repository-wide `renderable` Markdown
renderer is not used.
