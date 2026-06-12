---
phases: 5
created: 2026-06-07
start_phase: 1
source_spec: spec.md
packages:
  - biscuit-terminal
  - darkmatter
source_files_during_phase_1:
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/src/components/filesystem/icons.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/file_links/mod.rs
  - darkmatter/lib/src/markdown/compose/file_links/types.rs
  - darkmatter/lib/src/markdown/compose/file_links/parser.rs
  - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/file_links/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/types.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/file_tree/model.rs
  - darkmatter/cli/src/commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .opencode/skill/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/file_links/mod.rs
docs_updated_during_phase_4:
  - biscuit-terminal/docs/components/file_system.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/docs/topics/transclusion.md
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/structs/Markdown.md
  - darkmatter/docs/inline/toc-linking.md
  - darkmatter/docs/transclusion/block-transclusion.md
  - darkmatter/docs/transclusion/transclusion-design.md
  - darkmatter/docs/cli/graph.md
  - darkmatter/docs/lsp/features.md
docs_created_during_phase_4:
  - darkmatter/docs/inline/file-links.md
skills_files_updated_during_phase_4:
  - .claude/skills/biscuit-terminal/components.md
  - .opencode/skill/darkmatter/compose.md
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/tests/reference_integration.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_code:
  - biscuit-terminal/lib/src/components/filesystem/mod.rs
  - biscuit-terminal/lib/src/components/filesystem/icons.rs
  - biscuit-terminal/lib/tests/filesystem_parity.rs
  - darkmatter/lib/src/markdown/compose/file_links/mod.rs
  - darkmatter/lib/src/markdown/compose/file_links/types.rs
  - darkmatter/lib/src/markdown/compose/file_links/parser.rs
  - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/errors/mod.rs
  - darkmatter/lib/src/markdown/reference/mod.rs
  - darkmatter/lib/src/markdown/reference/types.rs
  - darkmatter/lib/src/markdown/reference/graph.rs
  - darkmatter/lib/src/markdown/reference/file_tree/model.rs
  - darkmatter/lib/src/markdown/reference/validate.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/lib/tests/reference_integration.rs
documentation:
  - biscuit-terminal/docs/components/file_system.md
  - darkmatter/docs/darkmatter-compose-pipeline.md
  - darkmatter/docs/topics/transclusion.md
  - darkmatter/docs/cli/compose.md
  - darkmatter/docs/structs/Markdown.md
  - darkmatter/docs/inline/toc-linking.md
  - darkmatter/docs/transclusion/block-transclusion.md
  - darkmatter/docs/transclusion/transclusion-design.md
  - darkmatter/docs/cli/graph.md
  - darkmatter/docs/lsp/features.md
  - darkmatter/docs/inline/file-links.md
packages:
  - biscuit-terminal
  - darkmatter
---

# Execution Plan: File Links Directive

## Overview

Implement `::file-links` as a transclusion-phase directive that discovers a
bounded set of document files and replaces the directive with a linked
`FileSystem` tree. The work starts with reusable `biscuit-terminal` component
capabilities, then adds Darkmatter parsing and secure discovery, pipeline and
reference integration, documentation, and final cross-package validation.

**Success criteria:** glob and `--dir` forms resolve relative to the containing
document; only `.md`, `.txt`, `.doc`, `.docx`, `.xls`, `.xlsx`, and `.pdf`
files are listed case-insensitively; exact glob matches are preserved; the
source document and boundary-escaping paths are excluded; the root label shows
the repository/CWD-relative dimmed prefix and highlighted target directory;
every file is linked; and the operation executes with the other concurrent
transclusion tasks.

## Assumptions

- The malformed closure path containing `darkmatterdarkmatter` is a typo; this
  plan is stored beside `spec.md`.
- `ComposeOptions::fail_fast` is the existing strictness signal: strict mode
  renders the subtle `No matching files` notice, while permissive mode removes
  an empty directive and records a compose warning.
- Symlinked candidates are canonicalized before boundary checks so a path that
  appears in-bounds but resolves outside the repository/CWD is excluded.
- A glob may select only part of a directory. `FileSystem` therefore needs an
  exact path allowlist in addition to extension filtering; a substring filter
  would incorrectly include allowed-extension files that did not match.

## Phase 1 - Extend the FileSystem Component

*Provide the reusable rendering and selection capabilities required by the
directive before Darkmatter depends on them.*

### Tasks

- [x] Add case-insensitive document-extension icon constants and mappings for
  `.txt`, `.pdf`, `.doc`, `.docx`, `.xls`, and `.xlsx` in
  `biscuit-terminal/lib/src/components/filesystem/icons.rs`, with distinct Nerd
  Font mappings and stable Unicode fallbacks.

- [x] Add a typed, case-insensitive extension allowlist builder to `FileSystem`
  and apply it while scanning files; retain ancestor directories only when
  they contain an included file.

- [x] Add an exact included-path allowlist builder to `FileSystem` so glob
  callers can render only matched files while preserving their directory
  hierarchy; define paths relative to the component root and reject or ignore
  entries outside that root.

- [x] Add a root-label API such as
  `with_dimmed_root_prefix(prefix)` plus an explicit root display name/icon
  option if required, and project the prefix and target directory as separate
  styled spans in both the bespoke terminal renderer and canonical render tree.

- [x] Preserve existing `show_root`, file-link, layout, gitignore, dotfile, and
  root-link behavior when the new builders are unused.

- [x] Add focused unit and parity tests covering every new extension in mixed
  case, exact path selection, empty selections, ancestor pruning, dimmed root
  prefix styling, repository icon selection, OSC8/file links, and
  terminal/render-tree parity.

- [x] Review and update affected `FileSystem` rustdoc and inline comments so
  filter and root-rendering behavior match the implementation.

### Validation Checkpoint

- [x] `cargo test -p biscuit-terminal --lib filesystem` passes.

- [x] `cargo test -p biscuit-terminal --test filesystem_parity` passes.

- [x] Existing callers that do not use the new builders produce unchanged
  output in focused snapshots/parity tests.

### Parallelizable Work

- [x] Icon mappings/tests and the root-prefix API/tests can be implemented in
  parallel; extension and exact-path filtering should be implemented together
  because they share scan/pruning behavior.

## Phase 2 - Parse and Discover File-Link Targets

*Build the directive model and deterministic, boundary-safe file discovery
without changing the compose pipeline yet.*

### Tasks

- [x] Add a `markdown::compose::file_links` module with typed directive,
  options, discovery result, and `FileLinksError` definitions following the
  existing `toc_linking` module structure.

- [x] Implement markdown-aware line parsing that ignores fenced/indented code,
  preserves directive spans and indentation, and accepts exactly one source
  form: `::file-links <glob>` or `::file-links --dir <path> [--depth <u32>]`.

- [x] Reject missing targets, simultaneous glob and `--dir` forms, `--depth`
  without `--dir`, duplicate options, invalid depths, unknown options, and
  invalid glob syntax with line-aware diagnostics and a `BlockError`
  `StatusBlock`.

- [x] Resolve glob and directory paths relative to `ComposeSource::File`;
  return a missing-source-context error for inline content that has no
  containing document.

- [x] Discover the security boundary by walking from the source document to
  the repository root, falling back to the current working directory when no
  repository exists, and canonicalize the boundary, source file, target root,
  and candidates before comparison.

- [x] Implement directory discovery with default depth `0` and inclusive
  recursion through the requested depth; implement glob discovery with
  deterministic lexical ordering and derive the common render root from the
  matched paths.

- [x] Filter candidates to regular files with the allowed extensions,
  case-insensitively; exclude the containing document itself; ignore
  boundary-escaping candidates and symlink escapes; deduplicate canonical
  matches.

- [x] Compute rendering metadata from discovery: component root, exact relative
  included paths, boundary-relative dimmed prefix, highlighted target directory
  name, and whether the repository icon applies.

- [x] Add parser/discovery tests for quoted paths and spaces, glob and directory
  modes, depth `0` and nested depths, mixed-case extensions, unsupported
  binaries/media, self-exclusion, duplicate matches, repository and CWD
  fallback boundaries, `..` escapes, and symlinks escaping the boundary.

### Validation Checkpoint

- [x] Focused `file_links` parser and discovery tests pass without invoking the
  full compose pipeline.

- [x] Fixtures demonstrate that a glob never includes an allowed-extension
  sibling that the glob did not match.

### Parallelizable Work

- [x] Parser/error implementation and filesystem discovery can proceed in
  parallel after the directive data model and empty-result contract are fixed.

## Phase 3 - Integrate Concurrent Composition and References

*Wire discovery and `FileSystem` rendering into all operation registries and
observable Darkmatter surfaces.*

### Tasks

- [x] Add `ComposeOperation::FileLinks`, update `COUNT`, stable indexes,
   default order, display text, phase mapping, operation-set tests, and compose
   pipeline documentation comments; place it in the Transclusion phase next to
   `TocLinking`.
 
 - [x] Parse `::file-links` directives during transclusion preparation and add a
   `PreparedTransclusion` variant so discovery/rendering runs through the
   existing Rayon resolution path and replacements are applied in source order.
 
 - [x] Configure `FileSystem` with exact included paths, the allowed extension
   set, requested depth, `.with_file_links()`,
   `.italicize_dot_files(true)`, `.dim_gitignore(true)`, `.show_root(true)`,
   and the computed root prefix/name/icon metadata.
 
 - [x] Render the component through its structured Markdown/render-tree-capable
   API rather than manually constructing tree glyphs or terminal escape codes;
   indent every replacement line to preserve list/container placement.
 
 - [x] Implement empty-result behavior: strict/fail-fast composition inserts a
   subtle `No matching files` notice, while permissive composition removes the
   directive and adds a line-aware `ComposeWarning`.
 
 - [x] Count successful and empty expansions consistently in
   `ComposeReport::transclusions_applied`/`transclusions_skipped`, and expose
   `FileLinksError` through `MarkdownError` and public compose re-exports.
 
 - [x] Extend transclusion detection and reference extraction types for
   `DirectiveFileLinks`; record the directive source expression and discovered
   file links without treating each selected document as a recursively
   followable Markdown transclusion.
 
 - [x] Update graph/file-tree/JSON serializers and CLI syntax labels so
   `md graph`, validation, and reference reports recognize `::file-links`
   without panics or non-exhaustive matches.
 
 - [x] Add compose integration tests for multiple directives, mixed concurrent
   directive kinds, deterministic replacement ordering, indentation, links,
   root styling metadata, strict/permissive empty results, operation disabling,
   report counters, and malformed/out-of-bound inputs.

### Validation Checkpoint
 
- [x] Focused compose tests prove `FileLinks` is in
   `ComposePhase::Transclusion` and executes alongside `::file`, `::code`, and
   `::toc-linking`.
 
- [x] `md compose` fixture output contains links only for the expected files,
   excludes the source file, and uses portable post-normalization paths.
 
- [x] `md graph --json` and text output handle documents containing
   `::file-links` and report the new syntax consistently.

### Parallelizable Work
 
- [x] Compose integration and reference/graph integration can proceed in
   parallel once the parser and public directive types are stable.

## Phase 4 - Update Public Documentation

*Document the final behavior after APIs and output contracts are validated.*

### Tasks

- [x] Update `biscuit-terminal/docs/components/file_system.md` with document
  icons, case-insensitive extension filtering, exact included-path filtering,
  dimmed root prefixes, custom root metadata, and examples that use the new
  builders.

- [x] Add `darkmatter/docs/inline/file-links.md` covering glob and `--dir`
  syntax, default and explicit depth, supported extensions, source-relative
  resolution, self-exclusion, repository/CWD boundaries, symlink handling,
  root rendering, links, empty results, and examples.

- [x] Add `::file-links` to the Darkmatter directive/reference indexes,
  compose-pipeline diagram and operation list, CLI compose guidance,
  transclusion/reference documentation, and any `has_transclusions()` docs
  that enumerate supported directives.

- [x] Update `.claude/skills/darkmatter/SKILL.md` and
  `.claude/skills/biscuit-terminal/SKILL.md` only where the new public
  directive or component APIs change their authoritative workflow guidance.

- [x] Run a comment-quality pass over every changed symbol and remove or
  correct comments that drifted as operation ordering, filtering, or root
  rendering changed.

### Validation Checkpoint

- [x] Documentation examples match executable API names and tested directive
  syntax.

- [x] Repository search finds no exhaustive directive/operation list that
  should include `::file-links` but still omits it.

### Parallelizable Work

- [x] Biscuit-terminal component documentation and Darkmatter directive
  documentation can be written in parallel after Phase 3 behavior stabilizes.

## Phase 5 - Full Validation and Acceptance

*Verify both package areas and the user-visible workflow end to end.*

### Tasks

- [x] Run the canonical Level 1 test recipes for `biscuit-terminal` and
  `darkmatter` from their area `justfile`s, using package-focused commands if
  the full recipes include unrelated failures.

- [x] Run `cargo test -p biscuit-terminal -p darkmatter` and
  `cargo test -p darkmatter-cli`; record and triage any failures caused by the
  new operation enum, reference syntax, snapshots, or docs.

- [x] Run targeted lint/check recipes for both package areas without running
  `cargo fmt`; fix warnings introduced by the feature.

- [x] Exercise `md compose` against repository fixtures for glob mode,
  directory depth `0`, recursive depth, empty results, source self-exclusion,
  out-of-bound paths, and mixed-case supported extensions.

- [x] Render the composed fixture to terminal, browser/HTML, Markdown, and
  MarkdownPlus and verify the tree shape, root prefix emphasis, repository
  icon/fallback behavior, and file hyperlinks degrade correctly by target.

- [x] Confirm no raw ANSI/OSC escape construction was added outside
  `biscuit-terminal`; terminal links and styling must come from
  `FileSystem`/render-tree components.

- [x] Review `git diff` to confirm changes are limited to the two package
  areas, their tests/docs/skills, and this feature directory, with no unrelated
  worktree changes reverted or reformatted.

### Validation Checkpoint

- [x] All focused and package-level tests pass.

- [x] Every success criterion in this plan and `spec.md` has an automated test
  or an explicitly recorded manual rendering check.

- [x] The final implementation is deterministic across repeated runs and does
  not expose or link any canonical path outside the repository/CWD boundary.
