---
agent: codex
model: ""
ready: false
---

# Review: File Links Directive

## Findings

### High: compose discards the required root and file styling

The directive configures the requested `FileSystem` options, but then serializes
the component through `render_markdown()`:

- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:2298)
- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/src/components/filesystem/mod.rs:3230)

That renderer explicitly drops color, dim, italic styling, and both Nerd Font
and Unicode icons. Consequently the composed document cannot retain the spec's
dimmed root prefix, highlighted target directory, repository icon, italicized
dotfiles, or dimmed gitignored files. The compose tests only assert that file
names occur, so they pass without verifying any of this required presentation.

Suggested fix: preserve a structural `FileSystem` render node through the
Darkmatter render pipeline, or introduce a Markdown representation that can
round-trip the required semantics. Add an end-to-end test that composes the
directive and then renders the resulting document, rather than testing the
component configuration in isolation.

Verification level present: Level 1 metadata/component tests only. The
user-observable styling requirement is not implemented end to end.

### High: terminal rendering and OSC8 behavior have no Level 2 verification

The feature requires specific root glyphs/styles and OSC8 links, but the only
terminal-oriented coverage constructs a `Terminal` in process and inspects
generated escape bytes:

- [filesystem_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/tests/filesystem_parity.rs:473)
- [filesystem_parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/tests/filesystem_parity.rs:601)

There is no `level2_*` test for `::file-links`, and no real-terminal capture of
the composed Darkmatter output. Level 1 is insufficient to prove that the root
icon, width, SGR dim/highlight styling, nested tree layout, and hyperlinks
survive the complete renderer and terminal emulator.

Suggested fix: add a Level 2 test, run through `just test-l2`, that composes a
fixture in a real terminal and captures the pane. Assert the visible root path,
repository/folder glyph, file hierarchy, and styling; inspect captured terminal
state or raw pane output as supported by the harness to verify OSC8 targets.

Verification level present: Level 1. Required level: Level 2.

### Medium: filesystem discovery runs serially and is repeated

`prepare_file_links_transclusions()` performs full discovery while preparing
the transclusion list, before work is sent to the concurrent resolver:

- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1843)
- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:2285)

The resolver then repeats discovery and `FileSystem` scans the selected tree a
third time while rendering. This contradicts the specification's concurrency
goal and makes multiple directives pay serial filesystem traversal cost before
parallel resolution begins. Results can also differ if the filesystem changes
between preparation and resolution.

Suggested fix: preparation should validate syntax and enqueue the directive
without walking the filesystem. Resolve discovery once in the concurrent stage
and carry its result directly into rendering. Ideally build the render tree from
the discovered entries so `FileSystem` does not rescan the same subtree.

Verification level present: Level 1 functional tests; no test or benchmark
proves concurrent discovery or guards against duplicate traversal.

### Medium: canonicalizing matches breaks in-bound symlink links

Discovery inserts each candidate's canonical target into the result set:

- [discovery.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/file_links/discovery.rs:269)
- [discovery.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/file_links/discovery.rs:293)

`included_paths` is therefore based on the target path, while `FileSystem`
matches its allowlist against the directory entry's lexical relative path:

- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/src/components/filesystem/mod.rs:1402)
- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/src/components/filesystem/mod.rs:1491)

For an in-repository symlink such as `docs/alias.pdf -> ../assets/report.pdf`,
the glob can match `docs/alias.pdf`, but rendering looks for the canonical
`assets/report.pdf` path. The alias is omitted or the target is shown under a
different tree, so the displayed path and hyperlink no longer represent the
matched file.

Suggested fix: retain both the lexical matched path and canonical path. Use the
canonical path only for boundary checks and deduplication; use the lexical path
for component roots, included paths, labels, and links. Add Level 1 tests for an
in-bound symlinked file as well as the existing escape case.

### Medium: names beginning with `::file-links` are parsed as directives

Directive detection uses `starts_with("::file-links")` and the parser does not
require whitespace or end-of-line after the directive name:

- [parser.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/file_links/parser.rs:36)
- [parser.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/file_links/parser.rs:135)

Text such as `::file-links-extra` is accepted as a file-links directive with
`extra` interpreted as its glob. This can unexpectedly remove or replace prose
and makes directive recognition inconsistent with an exact DSL keyword.

Suggested fix: require the keyword to be followed by ASCII whitespace or the
end of the line before parsing it. Add Level 1 tests for near-miss names such as
`::file-link`, `::file-links-extra`, and `::file-links2`.

## Verification

The focused test commands could not run in this environment because `rustup`
has no installed/default toolchain. Both attempted commands failed before
compilation:

```text
cargo test -p darkmatter --lib file_links
cargo test -p biscuit-terminal --test filesystem_parity
```

Static review also found no Level 2 test whose name or contents exercise
`::file-links`.

## Production Readiness

Not ready for production. The required terminal presentation is discarded by
the compose path, its appropriate Level 2 verification is absent, and discovery
does not meet the specified concurrent execution model.
