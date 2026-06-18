---
agent: codex
model: ""
ready: false
---

# Review: File Links Directive, Iteration 2

## Findings

### High: gitignored files cannot receive the required dim styling

The directive enables `dim_gitignore(true)`, but `FileSystem` still assigns
`is_ignored: false` to every file and directory produced by its filesystem
scan:

- [compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:2298)
- [filesystem/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/src/components/filesystem/mod.rs:1481)
- [filesystem/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/biscuit-terminal/lib/src/components/filesystem/mod.rs:1507)

The existing parity fixture explicitly confirms that a real `.gitignore` file
produces no dim SGR in either renderer because ignored status is hardcoded to
false. Therefore the spec's standard Darkmatter styling requirement is
configured but not implemented.

Suggested fix: implement ignore matching during tree construction, or carry
ignored status from file-links discovery into the projected tree. Add a Level 1
test using a real `.gitignore`, then include an ignored document in the Level 2
directive fixture and verify its captured style.

Verification level present: Level 1 tests of builder flags and artificial
`TreeNode { is_ignored: true }` values. Required level: Level 2 for the final
user-visible dim styling. The real-filesystem behavior currently fails even at
Level 1.

### High: the Level 2 test does not verify the full presentation contract

The new real-terminal test is useful and verifies hierarchy text, one dim SGR,
and OSC8 link preservation. It does not verify several distinct observable
requirements from the specification:

- repository icon versus ordinary folder icon
- target directory highlight color/style
- italic styling for dotfiles
- dim styling specifically on gitignored entries
- extension-specific glyphs for `.pdf`, `.doc`, `.docx`, `.xls`, `.xlsx`, and
  `.txt`

The fixture contains only ordinary `.md` files, and `topics` is asserted only
as plain text:

- [level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/level2_render_tree_terminal.rs:1407)
- [level2_render_tree_terminal.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/tests/level2_render_tree_terminal.rs:1456)

These are glyph and terminal-style requirements, so Level 1 render-node or
escape-byte tests are not sufficient. Extend the Level 2 fixture to contain
representative document extensions, a dotfile, and a gitignored document, then
assert the captured icon glyphs and the SGR runs associated with the relevant
names. The root assertion should distinguish the repository icon and target
highlight from incidental styling elsewhere in the frame.

Verification level present: Level 2 for basic hierarchy, root-prefix dimming,
and OSC8 links; Level 1 only or no effective test for the requirements above.
Required level: Level 2.

### Medium: file discovery remains serial and the filesystem is walked twice

`prepare_file_links_transclusions()` still calls `discover()` for each
directive in the serial preparation loop, before prepared transclusions enter
the concurrent resolver. Resolution then constructs `FileSystem` and
`ensure_tree_built()` walks the selected tree again:

- [compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1045)
- [compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:1845)
- [compose/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/mod.rs:2316)

Iteration 2 removed one repeated `discover()` call, but it did not satisfy the
specification's reason for placing this operation in the concurrent
transclusion phase. Multiple directives still serialize their expensive walks,
and each successful directive scans its files once for discovery and again for
rendering. Filesystem changes between those walks can also make the embedded
tree disagree with the discovered allowlist.

Suggested fix: enqueue the parsed directive during preparation and perform
discovery in the concurrent resolver. Build the render tree directly from the
discovered entries, or return enough entry metadata to avoid a second walk.
Add an instrumented test or benchmark that detects duplicate traversal and
demonstrates overlap between multiple file-links resolutions.

### Medium: an unterminated embedded subtree silently truncates the document

After decoding a valid opening marker, the Markdown fold immediately splices
the node and suppresses every subsequent event until it sees the separate close
marker:

- [fold.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/render_tree/fold.rs:190)
- [fold.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/render_tree/fold.rs:638)
- [embed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/renderable/src/tree/embed.rs:66)

If composed Markdown is truncated, edited, or copied without
`<!--bt:render-tree:end-->`, all content after the opening marker disappears
without a diagnostic. The current tests cover valid round trips and corrupt
payloads, but not a valid payload with a missing close marker.

Suggested fix: recognize and validate the complete embedded region before
splicing it, or buffer suppressed events and restore them with a structural
diagnostic when EOF is reached without a close marker. Add Level 1 tests for a
missing close marker, duplicate close marker, and adjacent embedded regions.

## Verification Matrix

| Requirement | Strongest verification | Assessment |
|---|---:|---|
| Glob and `--dir` discovery, depth, filtering, boundaries, self-exclusion | Level 1 | Appropriate |
| Exact directive keyword recognition | Level 1 | Appropriate |
| In-bound and escaping symlink behavior | Level 1 | Appropriate |
| Visible hierarchy and dimmed root prefix | Level 2 | Appropriate |
| OSC8 file links through a real terminal | Level 2 | Appropriate |
| Repository icon and highlighted target directory | Level 1 / partial Level 2 | Gap: needs specific Level 2 assertions |
| Dotfile italics | Level 1 component coverage | Gap: needs directive-level Level 2 capture |
| Gitignored-file dimming | No effective real-filesystem coverage | Broken and under-tested |
| Required document-extension glyphs | Level 1 component coverage | Gap: needs Level 2 capture |
| Concurrent transclusion execution | None | Implementation does not meet requirement |

## Verification

Focused tests could not run because `rustup toolchain list` reports no installed
toolchains. These commands failed before compilation:

```text
cargo test --color=never -p darkmatter --lib file_links
cargo test --color=never -p renderable tree::embed
cargo test --color=never -p darkmatter embedded_render_tree
```

The repository worktree was clean before this review file was added.

## Production Readiness

Not ready for production. A required style is nonfunctional, several terminal
presentation requirements remain below their required verification level, and
the operation still does not execute filesystem discovery concurrently as
specified.
