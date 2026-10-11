---
area: biscuit-file
feature: 2026-09-30-glob-reference
spec: biscuit-file/features/2026-09-30-glob-reference/spec.md
total_phases: 8
created: 2026-10-01
phase: 7
agent: "claude/sonnet"
yolo: "true"
packages:
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - dmls
    - claudine
    - claudine-cli
source_files_during_phase_1:
    - biscuit-file/lib/Cargo.toml
    - Cargo.lock
docs_updated_during_phase_1:
    - biscuit-file/docs/dependencies.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/glob/error.rs
    - biscuit-file/lib/src/file_reference/glob/parse.rs
    - biscuit-file/lib/src/file_reference/glob/roots.rs
    - biscuit-file/lib/src/file_reference/glob/list.rs
    - biscuit-file/lib/src/file_reference/glob/matches.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/glob_reference/mod.rs
    - biscuit-file/lib/tests/l1/glob_reference/grammar.rs
    - biscuit-file/lib/tests/l1/glob_reference/order.rs
    - biscuit-file/lib/tests/l1/glob_reference/boundary.rs
    - biscuit-file/lib/tests/l1/glob_reference/literal.rs
    - biscuit-file/lib/tests/l1/glob_reference/unfiltered.rs
docs_updated_during_phase_2:
    - biscuit-file/docs/topics/file-references.md
    - biscuit-file/docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/lib/src/markdown/schemas/validate.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/file_links/discovery.rs
    - darkmatter/lib/src/markdown/compose/file_links/types.rs
    - darkmatter/lib/src/markdown/compose/file_links/mod.rs
    - darkmatter/lib/src/markdown/compose/file_links/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/glob_listing.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/glob_consumers.rs
    - darkmatter/lib/tests/l1/find_files_and_try_frontmatter.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/schemas_grammar_proptest.rs
    - darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/tests/l1/glob_reference/literal.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
docs_updated_during_phase_3:
    - darkmatter/docs/inline/file-links.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/schemas/expression-functions.yaml
    - darkmatter/docs/topics/darkmatter-expressions.md
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/os/windows.md
source_files_during_phase_4:
    - darkmatter/lib/src/markdown/schemas/roots.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/lint.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/src/commands/clean/frontmatter_repair.rs
    - darkmatter/cli/tests/l1/schema_triggers.rs
    - darkmatter/cli/tests/common/protected_env.rs
    - darkmatter/dmls/src/context.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/tests/l1/lsp_session.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - darkmatter/docs/topics/schemas/authoring-schemas.md
    - darkmatter/docs/topics/schemas/schema-activation.md
    - darkmatter/example-docs/schemas/external.md
    - claudine/docs/rollout-strategy.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/os/windows.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/schemas/file_match.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/tests/l1/main.rs
    - darkmatter/dmls/tests/l1/schema_roots_parity.rs
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/glob/list.rs
    - biscuit-file/lib/src/file_reference/glob/matches.rs
    - biscuit-file/lib/tests/l1/glob_reference/boundary.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/src/commands/schema_interactive/mod.rs
    - claudine/cli/tests/l1/compose_schema_cli.rs
    - claudine/cli/tests/l1/level1_review_router_partial_pty.rs
    - claudine/cli/tests/common/review_router.rs
    - claudine/cli/tests/level2/level2_provided_partial_file_capture.rs
    - claudine/cli/tests/level2/level2_windows_provided_partial_file_capture.rs
docs_updated_during_phase_5:
    - biscuit-file/docs/topics/file-references.md
    - darkmatter/docs/topics/schemas/dmls-schema-support.md
    - claudine/docs/topics/completions/shell-completions.md
    - claudine/docs/topics/completions/auto-complete.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/darkmatter/dmls.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/claudine/SKILL.md
source_files_during_phase_6:
    - darkmatter/lib/tests/l1/glob_implementation_guard.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/Cargo.toml
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/os/build-hosts.md
source_files_during_phase_7:
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/tests/common/entry_point_parity/mod.rs
    - darkmatter/lib/tests/l1/entry_point_parity.rs
    - darkmatter/cli/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/tests/l1/entry_point_parity.rs
    - claudine/cli/tests/l1/entry_point_parity.rs
    - claudine/cli/src/completion/schema_completion/mod.rs
    - claudine/cli/src/completion/schema_completion/parity_tests.rs
docs_updated_during_phase_7:
    - darkmatter/docs/errors/file-reference-failures.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
---

# Plan: Glob References Share the File-Reference Grammar

## Summary of the Work

Three consumers each reimplement "reference prefix plus glob", and one
(`file(match(...))`) ignores the prefixes silently (Incident 2). This feature
adds one implementation, `GlobReference`, to biscuit-file and moves every
consumer onto it:

| Consumer | Today | After |
|----------|-------|-------|
| `file(match(...))` (`schemas/file_match.rs`, Claudine walks) | strips `!`, hands the rest to `globset`; fixed anchor list; `admits` reads the CWD | thin wrapper over `GlobReference`; `roots()` for completion, `matches` for validation |
| `find_files()` (`compose/expression/functions/mod.rs`) | own prefix splitter, first root | `list_files`, merged across roots, native order |
| `::file-links <glob>` (`compose/file_links/discovery.rs`) | prefix-unaware, `resolve_boundary` reads CWD | `list_files`, boundary from the prepared context |
| schema `$path` triggers (`schemas/triggers/`) | plain globs, boundary-relative | `GlobReference` + `matches`, new definition errors |
| `%` in `FileReference` (`resolve_recursive_core`) | global lexical winner | `take_first` on `**/<payload>`, local-first |

Three behavior changes ride along: the five **schema roots** replace the
ancestor walk (`schema_roots`) for trigger discovery and bare-name `$schema`
lookup, including `SCHEMAS_DIR` and `~/schemas`; completion candidates render
through `PortablePath`; and bound globs skip file symlinks that leave the
tree, with a compose warning.

Both current-directory reads handed off by `2026-09-30-file-refs-use-magic`
(`resolve_boundary`, `admits`) are deleted here (Decision 22), so the branch
satisfies that fix's Acceptance Criterion 5 with no exceptions.

### Definition of Success

Success is observable, not asserted:

- [ ] All 34 acceptance criteria of the spec are met, each by a named test
      (Phase 8 holds the criterion-to-test table).
- [ ] `glob_implementation_guard.rs` passes: no glob-crate use in production
      source outside the four-entry allowlist.
- [ ] The ambient-state guard of `2026-09-30-file-refs-use-magic` has no
      handed-off allowlist entries left.
- [ ] `just test` and `just test-l2` are green in every affected package area;
      `just lint` is clean.
- [ ] `just cross-check <pkg> --os windows` passes for each affected package
      and is recorded in the implementation log.
- [ ] Docs and skills describe the new behavior; no `SCHEMA_DIR` remains.
- [ ] The implementation log lists every changed output and every departure
      from the spec.
- [ ] Status is "implementation complete, ready for review". An agent never
      moves the spec to `_completed` or runs `just complete`.

### Conventions for Every Task

- Non-interactive session tree: every subagent brief must say so, name a
  narrow bounded task, and forbid `gpg`, `ssh`, `sudo`, and prompts.
- Never run `cargo fmt`. Never commit unless the author asks (commits are a
  separate operation; when asked, they are signed, carry no agent
  attribution, and each is checked with `git verify-commit HEAD`).
- Tests use nextest through `just test` / `just test-l2`; L2/L3 tests never
  take window focus. Load the `rust-testing` skill before writing tests.
- Any change to a symbol's behavior includes a pass over its `///`/`//!` docs
  and inline comments in the same change.
- Every Windows-sensitive path comparison uses biscuit-file's `PathIdentity`;
  load the `os` skill before touching `#[cfg(windows)]` or path comparison.
- Test fixtures build throwaway git repositories and a fixture `HOME` in
  temporary directories and pass them through the request snapshot; the test
  process's environment is never mutated.
- Wave size: subagents within a wave own disjoint files. A task that edits a
  file another task in the same wave edits is sequenced, not parallelized.

---

## Phase 1: Rulings, Spikes, and Readiness

Goal: settle every planning-owned decision, retire the two real unknowns, and
confirm the branch is ready before any signature changes.

### Necessary Rules

The spec leaves these to planning or leaves them unclear. Each is decided here
so implementers do not guess. **R4, R6, and R9 change observable behavior in
ways the spec does not pin; the author may overturn them before Phase 2.**

- **R1. Names and locations.**
  - biscuit-file module: `biscuit-file/lib/src/file_reference/glob/`
    (`mod.rs`, `parse.rs`, `roots.rs`, `list.rs`, `matches.rs`), re-exported
    from the crate root beside `FileReference`, behind the existing
    `file-reference` feature.
  - Types: `GlobReference`, `GlobReferenceError` (typed, `thiserror`),
    `GlobListing { matches: Vec<PathBuf>, skipped: Vec<SkippedEntry> }`,
    `SkippedEntry { link: PathBuf, target: PathBuf }`.
  - `list_files` returns `Result<GlobListing, GlobReferenceError>`;
    `take_first` returns `Result<Option<PathBuf>, _>`; `matches` returns
    `bool` (it cannot fail once the reference is constructed); `roots`
    returns `Vec<PathBuf>`.
  - Escaping helper for `%`: `GlobReference::escape(&str) -> String`.
- **R2. Optional first-root list mode: not built.** The spec calls it
  optional and no Darkmatter consumer uses it. Rule 2 (no speculative
  features) applies. Record in the implementation log as a deliberate omission.
- **R3. Skipped-symlink warning code: `compose.glob.skipped_symlink`.** The
  implementer first reads the existing compose warning codes and matches
  their naming convention; if the convention differs, use the closest
  spelling and log it. `find_files()` and `::file-links` each emit one
  warning per skipped entry, naming link and target.
- **R4. `*` does not cross `/`; `**` does.** `globset` must be built with
  `literal_separator(true)` and case-sensitive matching. Rationale:
  `^docs/*.md` and the "no `*.md` at other depths" example in criterion 10
  require it. Matching runs on the `/`-spelled path (`to_portable_string`) on
  every OS. The implementer first confirms what `FileMatchGlobs::compile` and
  the trigger matcher do today; if they differ, existing-test results decide,
  and the departure is logged.
- **R5. Guard test home and name.**
  `darkmatter/lib/tests/l1/glob_implementation_guard.rs`, reusing the sanitizer
  and counting style of `semantic_results_never_persist.rs` and
  `darkmatter/cli/tests/common/source_scan.rs`. Other packages' source roots
  are declared in `darkmatter/lib/Cargo.toml`
  `[package.metadata.ci.tests] source-inputs`.
- **R6. A pattern list with no positive pattern is a typed error**
  (`GlobReferenceError::NoPositivePattern`), not "admit everything" and not
  "admit nothing". Spec basis: "diagnostics, never silence". The implementer
  searches existing `match()` and trigger tests for an all-negation list
  first; if one exists, the existing behavior is kept and the departure from
  this ruling is logged.
- **R7. Walk pruning.** A walk starts at the literal directory prefix of the
  glob under each root and never follows directory symlinks. The walker
  applies no content filters (spec), but it must not descend a subtree the
  glob cannot match (a pattern with literal first segment `docs/` walks
  `docs/` only).
- **R8. Containment, deduplication, and ownership share one helper.** One
  function in `roots.rs` decides "first root that contains this file"
  using `canonicalize_simplified` + `PathIdentity::strip_prefix`; `list.rs`
  and `matches.rs` both call it. No second implementation.
- **R9. `SCHEMAS_DIR` input shapes** (the schema-root reader is a config
  read; see the matrix in Phase 4). Unset: no fourth root. Set to an empty or
  whitespace-only string: **not** treated as unset; it is reported by
  `md schema triggers` as "invalid (empty)" and adds no root. Set to a
  relative path: invalid, reported the same way, adds no root (a relative
  schema root would depend on launch directory). Set to an absolute path that
  does not exist: skipped as "absent", as for any root. `SCHEMA_DIR`
  (singular) is never read.
- **R10. Trigger-definition error location.** Trigger pattern errors are
  produced by one function in the trigger grammar layer
  (`schemas/triggers/grammar.rs` / `lint.rs`, wherever existing trigger
  definition errors originate) and consumed identically by `md` and DMLS.
  `match()` definition errors are produced where `match()` globs are
  compiled today and surfaced by the existing schema-definition check path.
- **R11. Nearest-root judgment per pattern, not per list.** Each pattern
  (positive or negative) is judged against its own roots. A file is a match
  when some positive pattern admits it and no negative pattern rejects it.
  The file-name view is decided per pattern from the glob text after the
  prefix (spec); it is not a `GlobReference`-wide flag. `GlobReference`
  exposes the file-name view through a constructor option
  (`GlobReference::with_file_name_view()`), set by `match()` and triggers and
  left off for `find_files()` and `::file-links`.
- **R12. Wider measurement left to the author.** The spec asks only for a
  quick performance sample (Spike 2). A multi-OS or large-repository
  benchmark is **not** scheduled; if the author wants one, say so before
  Phase 2.

### Spikes

Each spike runs once, here, before the work it informs.

- [x] **Spike 1: `globset` semantic probe** (one host, ~15 minutes).
  Write a throwaway test (not committed) that builds `globset` with
  `literal_separator(true)`, case-sensitive, and asserts: `*` vs `/`;
  `**/` matching zero directories (does `**/intro.md` match `intro.md`?);
  `[id]` as a class; `{a,b}` alternation; backslash escaping and
  `globset::escape` for a literal `[id].md`; leading `!` handling (we strip it
  ourselves). Output: a 6-line table appended to the implementation log
  under "Spike 1". Informs R4 and the escape helper. If `**/x` does not match
  top-level `x`, the `%` rewrite and criterion 15 (`{pkg}/intro.md` is
  matched by `^**/intro.md`) need an explicit zero-depth alternative; record it.
- [x] **Spike 2: unfiltered-walk cost** (one host, one sample). Time a
  `walkdir` pass under this repository's root with directory-symlink
  following off and no filters, listing `**/*.md`, with and without a
  warm `target/` present. Report wall-clock and entry count. Informs R7:
  if an unfiltered walk of the repository root exceeds ~2 s, `list_files`
  gets a documented note, and `find_files()` docs warn about `&**/…`; it
  does **not** add filters (spec forbids them). No other OS is measured
  (R12).

### Tasks

- [x] **Readiness check.** Confirm `git status` shows only the two
  expected modified test files (`darkmatter/dmls/tests/l1/repository_contexts.rs`,
  `darkmatter/lib/tests/l1/entry_point_parity.rs`); report any others to the
  user before proceeding. Confirm `darkmatter/fixes/2026-09-30-file-refs-use-magic`
  is at "Phase 8 close" in its implementation log. Run `just test` in
  `biscuit-file` and `darkmatter` for a green baseline; record baseline
  failures, if any.
- [x] **Baseline inventory.** Using `grep` (and GitNexus `context` where it
  answers faster), list: every caller of `FileMatchGlobs`, `admits`,
  `admits_path`, `file_match_admits`, `find_files_fn`, `resolve_boundary`,
  `schema_roots`, and `resolve_recursive_core`; every test asserting current
  output of those. Save as an appendix to the implementation log. This list
  drives Phase 8's "changed outputs" record.
- [x] **Create the implementation log** at
  `biscuit-file/features/2026-09-30-glob-reference/implementation-log.md`
  with sections: Spikes, Changed Outputs, Departures from Spec, Windows
  Evidence, Handed-off Reads. Later phases append to it as they go.
- [x] **Dependency note.** Add `globset` to the `file-reference` feature of
  `biscuit-file/lib/Cargo.toml` (already used by darkmatter; match its
  version) and update `biscuit-file/docs/dependencies.md`.

**Checkpoint 1:** baseline green, spikes recorded, rulings reviewed. Proceed.

---

## Phase 2: `GlobReference` in biscuit-file

Goal: the only implementation of prefix-plus-glob exists, fully tested,
before any consumer moves. Package: `biscuit-file`. Load the `biscuit-file`,
`rust`, `rust-testing`, and `os` skills.

### Wave 1 (sequential; one subagent): grammar and types

- [x] **Pattern parser.** `GlobReference::new(impl IntoIterator<Item: AsRef<str>>)`.
  Each pattern is `[!][prefix]glob`. Reuse `FileReference`'s prefix parser,
  `{{VAR}}` interpolation, and sigil-to-roots mapping by extracting what is
  needed from `parse.rs`/`resolve.rs` into shared `pub(super)` helpers
  (extract, do not copy). The text after the prefix is the glob.
  - `!` is meaningful only here; `FileReference` keeps rejecting it.
  - Rejected: `http(s)://` and `%`, as typed errors naming the pattern.
  - A pattern with no metacharacters is valid and matches one path.
  - R6: no positive pattern is `NoPositivePattern`.
  - Invalid glob text is `InvalidGlob { pattern, source }`.
- [x] **Glob compile.** Build one `GlobMatcher` per pattern (R4: case-sensitive,
  `literal_separator(true)`). `compile` is the only place in biscuit-file that
  names `globset`.
- [x] **Error enum** with `Display`/`source` and variants: `RejectedPrefix`,
  `MalformedPrefix`, `InvalidGlob`, `NoPositivePattern`, `RelativeTreeEscape`,
  `OutsideRepository`, plus a `From` for the existing `FileReferenceError`
  prefix failures so `ResolutionFailure` comparison works for the parity matrix.
- [x] **`GlobReference::escape`** (for `%`), tested with `[id].md`, `*`, `?`,
  `{`, `\`.
- [x] **Constructor option** `with_file_name_view()` (R11). Rule: applies per
  pattern when the glob text after the prefix has no `/`; applies to
  negations the same way.

### Wave 2 (parallel; disjoint files, after Wave 1)

- [x] **Task: Roots and ordering** (`roots.rs`, `list.rs`).
  - `roots(&ctx) -> Vec<PathBuf>` per the spec's root table: bare = value
    `cwd` then repository root (Decision 19); `./`/`../` = `cwd`; `&` =
    repository root; `^` = package, package area, repository; `@` = magic
    chain in tier order; `~` = home; `vault:` = configured vault roots then
    captured `$VAULT` paths; absolute = verbatim. Literal directory segments
    before the first metacharacter narrow each root.
  - Relative boundary: when `base_dir_is_boundary()` and not
    `external_relative_allowed()`, a bare/`./`/`../` root that leaves
    `base_dir()` is `RelativeTreeEscape`. `&`/`^` outside a repository are
    `OutsideRepository`. `~`, `@`, absolute, vault, `{{VAR}}` are allowed.
  - `list_files(&ctx) -> GlobListing`: run the glob under each root in
    precedence order; the R8 ownership helper makes a later root skip every
    file under an earlier root, whether the earlier root included or excluded
    it; order = root precedence, then component count, then component-wise
    `PathIdentity::components()` comparison (never `/`-string comparison).
    Files are canonicalized before comparison so `/var` vs `/private/var` is
    one file.
  - `take_first(&ctx)`: same order, stops at the first root with a match and
    never walks a later root.
  - Walk: R7 pruning, no directory-symlink following, no filters.
  - Symlinks (Decision 25): in a boundary-bound glob, a matched **file**
    symlink whose real target lies outside the tree is excluded from `matches`
    and reported in `skipped`; a symlink whose target stays inside is listed
    as before. Unbound globs (`~`, `@`, absolute, `&`, `^`, vault) skip nothing.
- [x] **Task: Lexical `matches`** (`matches.rs`; same crate, different file).
  - `matches(&path, &ctx) -> bool`: canonicalize the longest existing prefix
    of `path`, append the rest unchanged, then judge by containment (R8
    helper) and glob checks with no walk and no further filesystem access.
  - Nearest-root judgment per pattern (R11), file-name view per option,
    negation per its own roots.
  - Must be defined for a path that does not exist, under an existing
    directory and under a missing one (criterion 26).
  - Because Wave 2's two tasks both call the R8 helper, **Wave 1 ships the
    helper's signature** (`fn owning_root(roots, path) -> Option<(usize, PathBuf)>`)
    and the Roots task provides the implementation; the matches task writes
    against the signature and its tests wait for it (merge point at the end
    of the wave).

### Wave 3 (parallel; after Wave 2)

- [x] **Task: `%` rebuilt on `take_first`.** Replace `resolve_recursive_core`
  in `file_reference/resolve.rs` with `GlobReference::take_first` on
  `**/<escaped payload>` under the same prefix roots (Decision 15). Delete
  the old walk and its filters/ordering. Update the existing `%` tests; each
  changed expected result is logged under "Changed Outputs".
- [x] **Task: Literal-miss hint.** When a `FileReference` finds nothing and
  its text contains glob metacharacters, append a hint naming the
  glob-accepting form (`::file-links`). Typed error unchanged; the hint is
  part of its `Display`.
- [x] **Task: Input-grammar test** (`biscuit-file/lib/tests/l1/glob_reference.rs`),
  one test walking every shape in the pattern grammar from a real-fixture
  repository, one edit per cell, with a control row proving the unedited
  list is the positive result:

  | Shape | Pattern input | Expected outcome |
  |-------|---------------|------------------|
  | control | `["^**/*spec*.md"]` | ordered list |
  | empty list | `[]` | `NoPositivePattern` |
  | only negations | `["!&x/**"]` | `NoPositivePattern` (R6) |
  | empty string | `[""]` | `MalformedPrefix` (never matches everything) |
  | remote | `["https://x/*.md"]` | `RejectedPrefix` |
  | `%` | `["%**/*.md"]` | `RejectedPrefix` |
  | invalid glob | `["^**/[x"]` | `InvalidGlob` naming the pattern |
  | `!` in `FileReference` | `FileReference::new("!x")` | still the removed-sigil error |
  | duplicate patterns | `["*.md", "*.md"]` | each file once |
  | unknown sigil | `["$x/*.md"]` | `MalformedPrefix` |

### Wave 4 (parallel; tests, after Wave 3)

Each test names its acceptance criterion in a comment; tests assert through the
public API (`list_files`, `take_first`, `matches`, `roots`, `FileReference`).

- [x] **Local-first order and ownership** (criteria 15, 16, 18): the fixture
  package `pkg` in area `area` with the five-file expectation, tie-break
  (`a/intro.md` before `b/intro.md`, `a/x.md` before `a-b/x.md`), ownership
  with exclusion (`["^**/*spec*.md", "!x/**"]`), and symlinked-spelling
  deduplication (macOS `/var` vs `/private/var` via a symlinked temp dir).
- [x] **Boundary and symlinks** (criteria 14 biscuit-file parts, 25): bare/`./`
  escape = `RelativeTreeEscape`; `allow_external_relative()` succeeds; `&`/`^`
  outside a repository = `OutsideRepository`; `~`/`@`/absolute/vault allowed;
  directory symlink out of the tree lists nothing behind it; file symlink out
  of the tree is `skipped`, and `FileReference::new("docs/leak.md")` still
  fails `RelativeTreeEscape`. Windows: symlink creation needs privilege;
  gate with the repository's existing skip pattern for symlink tests, and
  record in the log if the cross-check cannot exercise it.
- [x] **Literal vs class, `%`, case** (criteria 19, 23, 26): `[id].md`;
  `%^README.md` local-first; `%vault:notes.md`; `matches` on missing paths;
  `*.MD` does not match `x.md`.
- [x] **Unfiltered** (criterion 13 biscuit-file part): hidden, gitignored, and
  `_`-prefixed files are returned.

**Checkpoint 2:** `just test` and `just lint` green in `biscuit-file`;
nothing in the workspace has changed behavior yet except `%` (log it). Run
`just cross-check biscuit-file --os windows` now so a Windows-only path bug
surfaces before three packages depend on it.

---

## Phase 3: Darkmatter Consumers

Goal: `match()`, `find_files()`, and `::file-links` run on `GlobReference`;
both handed-off current-directory reads are gone. Package: `darkmatter`.
Load the `darkmatter` skill.

### Wave 5 (parallel; one subagent per track, disjoint files)

- [x] **Track A: `FileMatchGlobs`** (`schemas/file_match.rs`).
  - Becomes a thin wrapper over `GlobReference` built with
    `with_file_name_view()`; keep only what is `match()`-specific.
  - Delete `admits`, `admits_path`, and the anchor list (this removes the
    CWD fallback). `file_match_admits` calls `matches` with the value's
    context.
  - Expose `roots()` for Claudine (Phase 5).
  - Definition errors (criterion 8): `%`, `https://`, and invalid glob each
    produce a schema definition error naming property and pattern, via the
    existing definition-check path (R10). `match(vault:notes/*.md)` is valid.
  - Update `///` docs; existing bare-pattern tests (`*.png`, `src/**/*.rs`,
    `!_*.md`) must pass unchanged (criterion 9).
- [x] **Track B: `find_files()`** (`compose/expression/functions/mod.rs`,
  `find_files_fn`).
  - Delete the prefix splitter; call `list_files` for every prefix; return
    the full merged list in native order (not sorted).
  - File-name view is **off** (criterion 11).
  - Turn each `GlobListing.skipped` entry into one compose warning with code
    `compose.glob.skipped_symlink` (R3).
  - Relative-boundary and `OutsideRepository` errors surface as typed
    expression errors.
  - Update `find_files()` docs, replacing "sorted absolute paths".
- [x] **Track C: `::file-links <glob>`** (`compose/file_links/`).
  - Delete `resolve_boundary` (and its git discovery and CWD fallback). The
    boundary is read from the prepared context via `GlobReference`.
  - Use `list_files` for every prefix; a bare glob searches the containing
    document's folder, then the repository root, merged. `--dir` mode is
    unchanged.
  - Keep the directive's own rules: extension allowlist, exclude the
    containing document, skip symlinked directories; render through the
    existing tree (rooted at the matches' common ancestor).
  - Skipped entries become compose warnings (R3).
  - Parser (`parser.rs`) accepts reference prefixes in the glob argument.

### Wave 6 (sequential; after Wave 5)

- [x] **Existing-test reconciliation.** Run `just test` in `darkmatter`,
  classify each red test as an intended output change (update it, log it
  under "Changed Outputs") or a bug (fix the code). Include
  `find_files_and_try_frontmatter.rs` (must still find `_completed/` files),
  `file_links` tests (merged bare globs, criterion 10), and CLI tests.
- [x] **New consumer tests** (criteria 9–14, 25 consumer side, 20 rows are
  Phase 7): merged `::file-links *.md` from a nested document; `^docs/*.md`
  from package, area, repository; bare `find_files('spec.md')` does not
  return `x/spec.md`; `match(spec.md)` and `match(^spec.md)` do; file-name
  negation (`match(*.md, !_*.md)` rejects `docs/_x.md`); symlink warning
  count and wording; `RelativeTreeEscape` for bare/`./` globs in both
  consumers with and without `allow_external_relative()`.
- [x] **Handed-off reads.** Run the ambient-state guard test of
  `2026-09-30-file-refs-use-magic`; remove its two handed-off allowlist
  entries; the guard must pass with none.

**Checkpoint 3:** `just test` / `just test-l2` / `just lint` green in
`darkmatter`; Incident 2 reproduces as a passing test at the `match()`
validation level (completion half lands in Phase 5).

---

## Phase 4: Schema Roots and `$path` Triggers

Goal: five schema roots replace the ancestor walk; triggers run on
`GlobReference`; `md` and DMLS-visible behavior is defined. Package:
`darkmatter` and `darkmatter-cli`.

### Wave 7 (sequential; one subagent): the root list

- [x] **Root list function** in `schemas/triggers/discovery.rs` (or a new
  `schemas/roots.rs` if discovery grows unwieldy): given the prepared
  context, return the ordered, deduplicated list of schema roots:
  1. package root `/schemas`
  2. package-area root `/schemas`
  3. `base_dir()` `/schemas`
  4. the folder named by `SCHEMAS_DIR` from the request snapshot environment
     (the folder itself, no `schemas/` appended; Decisions 26 and 27)
  5. `{home}/schemas` from the snapshot's home
  - Package and area are those of the **document being checked**, not of the
    launch directory.
  - A root coinciding with an earlier one is kept once, at its first position
    (`canonicalize_simplified` + `PathIdentity`). A missing folder is skipped
    but remembered as "absent" for `md schema triggers` output.
  - R9 shapes for `SCHEMAS_DIR` are reported as invalid, never silently
    ignored and never read from the process environment.
  - Replace `schema_roots(document_dir, boundary)` and its ancestor walk;
    update the module doc.
- [x] **Bare-name `$schema` lookup** (`schemas/resolve.rs`): a single file name
  with no separator resolves to the first root holding it. Path-qualified
  `$schema` is untouched.
- [x] **Input matrix for the root reader** (one test, fixture `HOME`, request
  snapshot only):

  | Field | Shape | Outcome |
  |-------|-------|---------|
  | `SCHEMAS_DIR` | control: absolute existing folder | root 4 present |
  | | absent from snapshot (even if set in the test process) | no root 4 (criterion 29) |
  | | empty / whitespace | reported invalid (R9), no root |
  | | relative | reported invalid (R9), no root |
  | | absolute, nonexistent | skipped as absent |
  | | `SCHEMA_DIR` (singular) set | no root |
  | | equals `~/schemas` | one root, first position |
  | home | absent from snapshot | root 5 absent, no panic |

### Wave 8 (parallel; disjoint files; after Wave 7)

- [x] **Task: Trigger matching on `GlobReference`**
  (`triggers/matcher.rs`, `assemble.rs`, `grammar.rs`/`lint.rs`).
  - Parse `$path` patterns as `GlobReference` with `with_file_name_view()`;
    judge a document path with `matches` against the prepared context.
  - Allowed prefixes: `&`, `^`, `~`, absolute, bare, `./`. Trigger-definition
    errors (R10), each naming the pattern: `@`, `%`, `vault:`, and `{{VAR}}`
    **anywhere** in the pattern (including after `!`).
  - A bare/`./` pattern's `cwd`: the folder holding the trigger's `schemas/`
    when it is the package, area, or tree root; for a trigger from
    `SCHEMAS_DIR` or `~/schemas`, the checked document's tree root
    (`base_dir()`).
  - Shadowing by file name across roots is as today; a shadowed trigger is
    reported, not evaluated (criterion 28).
- [x] **Task: `$path` field matrix.** One test walking the `$path` field of a
  real trigger-schema fixture, one edit per cell, with a control row:

  | Shape | Edit | Expected through `md schema validate` / trigger report |
  |-------|------|----------------------------------------------------------|
  | control | `$path: "^docs/**"` | trigger applies to a document under `docs/` |
  | absent | key omitted | existing "missing `$path`" definition error, not "matches nothing" |
  | explicit null | `$path: null` and `$path:` | same error as absent is **not** allowed; distinct "null" error, or the existing one if it already distinguishes; log which |
  | wrong type, whole field | `$path: 123` | definition error, not empty |
  | wrong type, one element | `$path: ["^docs/**", 123]` | definition error, not filtered |
  | wrong type, every element | `$path: [123]` | definition error, not empty |
  | empty | `$path: []` | definition error (R6), not "matches all" |
  | duplicate key | `$path` twice | the YAML layer's existing duplicate-key behavior; assert whichever the repository already guarantees, and log it |
  | invalid content | `$path: "^**/[x"` | `InvalidGlob` naming pattern |
  | forbidden prefix | `@x/**`, `%x.md`, `vault:x/**`, `{{X}}/**`, `docs/{{X}}/*.md` | definition error (criterion 21) |
  | `~` | `~/notes/**` | valid; matches a document under fixture `HOME`'s `notes/` |

  Before declaring the matrix done, grep the trigger envelope for
  `#[serde(default)]`, `filter_map(.. as_str())`, `unwrap_or_default()`,
  and `.ok()` on `$path` parsing, and fix any that conflate shapes.
- [x] **Task: `md schema triggers` output** (`darkmatter/cli/src/commands/schema/triggers.rs`).
  Print the five roots in search order, marking absent folders, an unset
  `SCHEMAS_DIR`, and R9-invalid values, instead of "Boundary" and walked roots.
  Render with `biscuit-terminal` `TerminalRenderable` components
  (`UnorderedList`/`Prose`), per repository convention.
- [x] **Task: Shipped example.** Rewrite
  `darkmatter/example-docs/schemas/external.md` to `$schema: ./external.yaml`
  (criterion 33); verify it validates.

### Wave 9 (parallel tests; after Wave 8)

- [x] **Schema-root tests** (criteria 27–31): order in a fixture monorepo with
  `SCHEMAS_DIR` and `HOME` in the snapshot; shadowing both directions
  (repository file wins; remove it and the home file wins); `SCHEMAS_DIR`
  names the folder itself (`dm-defs` outside `HOME` and every repository: a
  file directly inside is found, one only in `dm-defs/schemas/` is not);
  in-between folders (`{package}/docs/schemas/`) neither apply nor resolve;
  `~/schemas` with a fixture `HOME` and `$schema: user.yaml`.
- [x] **Trigger/`match()` parity** (criterion 21 last sentence): for a shared
  set of patterns and paths, trigger matching and `match()` validation give
  identical verdicts, including nearest-root, negation, and file-name view.
- [x] **CLI tests** for `md schema triggers` output (criteria 27, 29) and
  definition errors from `md schema validate`.
- [x] **Repository check.** Re-run `ls -d */schemas */*/schemas` and confirm
  the spec's claim (root 3 and area-root schemas keep working; documents that
  name `feature-review.yaml` or `review.yaml` by bare name still resolve, by
  running `md schema validate` over them). Log any document that now fails.

**Checkpoint 4:** `just test`, `just test-l2`, `just lint` green in
`darkmatter` and `darkmatter-cli`; `md schema triggers` output reviewed
manually once and pasted into the implementation log (changed output).

---

## Phase 5: DMLS and Claudine

Goal: the editor and the shell agree on schema roots and completion. Two
disjoint tracks.

### Wave 10 (parallel; disjoint crates)

- [x] **Track A: DMLS** (`darkmatter/dmls`). Load the `darkmatter` skill.
  - Effective-schema cache (`overlay/mod.rs` ~148) keys on content hash **plus**
    the context: repository root, document `cwd`, `base_dir` origin, and a
    hash of the request snapshot (`HOME`, environment, extra `@` roots).
    Hash through `biscuit-hash` (xxHash), never a hand-rolled hasher.
  - The five schema roots come from the same function `md` uses (Phase 4
    Wave 7); no duplicate list in DMLS. The snapshot comes from the
    environment the editor launched the server with.
  - Trigger definition errors are the same diagnostics as `md` (R10).
  - DMLS's two unrelated glob uses (`workspace/discover.rs`,
    `overlay/schema.rs`) are left alone and are allowlisted in the guard.
  - Tests (criteria 22, 32): same content under two contexts whose `^` roots
    differ, and under two snapshots whose `SCHEMAS_DIR` differs, are judged
    separately (no stale cache hit); `md` and DMLS give the same roots,
    applied triggers, and bare-name resolution for the documents of
    criteria 27–31. L2 tests use no real editor windows.
- [x] **Track B: Claudine completion** (`claudine/cli/src/completion/schema_completion/candidates.rs`,
  and `file_match_admits`' caller in `claudine/lib` if present).
  Load the `claudine` skill.
  - `file_candidates` (TAB) and `file_candidate_paths` (ENTER chooser) walk
    `FileMatchGlobs::roots()` in precedence order instead of
    `scopes::property_value_root`, visiting each file once under its most
    local root (ownership by R8's helper or a public biscuit-file equivalent;
    do not reimplement containment in Claudine).
  - Walk filters (hidden, gitignored, `_`-prefixed, `SKIP_DIRS`) and the
    typed-partial substring filter stay in Claudine's walks.
  - Skipped-symlink entries are omitted silently (Decision 25).
  - Render each candidate through `PortablePath` with `with_ctx`, restricted to
    `SameDirRelative`, `ChildDir`, `ImmediateParentDir`, `PeerDir`, then
    `RepoRoot(None)`, then `HomeDir`, then `AbsolutePath` (no `EnvRootedPath`).
    A candidate under the launch directory passes its bare launch-relative
    path so the spelling stays `fixes/…/spec.md`, never `./…`.
  - Candidates are emitted in native order, not re-sorted by rendered string.
  - Fix `file_match_admits` to use `matches`.
  - Tests: Incident 2 regression (criterion 1: exactly
    `spec='fixes/2026-09-29-ts-review-improvements/spec.md'`), criteria 3, 4,
    5 (`../other/z/spec.md`), 12, 17, 24 (shadowing falls through to a later
    form and every inserted value resolves back to the walked file), and
    one-way parity (criterion 6).

**Checkpoint 5:** `just test`, `just test-l2`, `just lint` green in `dmls`,
`claudine`, and `claudine-cli`.

---

## Phase 6: Source-Scan Guard

Goal: no second glob implementation can creep back in.

### Wave 11 (sequential)

- [x] **Write `glob_implementation_guard.rs`** (R5). Sanitized source,
  identifier-boundary matches, exact per-file counts, stale entries fail.
  Scans production source (not `tests/`, not `benches/`) of biscuit-file,
  darkmatter, darkmatter-cli, dmls, claudine, and claudine-cli for `Glob`,
  `GlobBuilder`, `GlobSet`, `GlobSetBuilder`, and any other glob crate
  (`glob::`, `wax`, `ignore::overrides`), outside this exact allowlist with
  counts:
  - biscuit-file's `GlobReference` module (`file_reference/glob/parse.rs` only)
  - `darkmatter/lib/src/markdown/compose/toc_linking/filter.rs`
  - `darkmatter/dmls/src/workspace/discover.rs`
  - `darkmatter/dmls/src/overlay/schema.rs`
- [x] **Resolve remaining hits.** Scan will flag
  `compose/toc_linking/types.rs`, `compose/file_links/types.rs`, and any other
  file that names a glob type. For each: delete the use if it moved to
  `GlobReference`, or add it to the allowlist only if it is genuinely
  heading-text or configuration glob (justify in the allowlist comment). The
  spec names four allowed entries; a fifth needs a log entry.
- [x] **Declare scanned paths** of other packages in `darkmatter/lib/Cargo.toml`
  `[package.metadata.ci.tests] source-inputs`
  (`docs/cicd/test-inputs.md`; spell reads in the forms the `rust-testing`
  skill lists).
- [x] **Negative control.** The guard has a test proving it fails on a
  synthetic `GlobBuilder` use and on a stale allowlist entry.
- [x] **Dependency cleanup.** Remove `globset` from any crate whose production
  source no longer uses it; update each affected `docs/dependencies.md`.

**Checkpoint 6:** guard green in `darkmatter`; `just lint` green.

---

## Phase 7: Entry-Point Parity Matrix

Goal: the glob form appears in both tables of the parity matrix from
`2026-09-30-file-refs-use-magic`, on all OSes. Extend the existing shared
module `entry_point_parity` and its per-package runners; do not create a
parallel matrix.

### Wave 12 (sequential; one subagent)

- [x] **Add the `GlobReference` form row** to the shared tables
  (`^**/*spec*.md`, `!&**/_completed/**`), and a glob cell expectation type
  (ordered file list in native order, or `ResolutionFailure`).
- [x] **Table 1 consumers** (document-authored, fixed launch directory):
  `::file-links <glob>`, `find_files()`, and `file(match(...))` validation of
  a frontmatter value.
- [x] **Table 2 consumers** (caller-supplied, two launch directories: repo
  root and nested package): `file(match(...))` completion (both Claudine
  walks) and validation of a caller-supplied value.

### Wave 13 (parallel; one runner per package; after Wave 12)

- [x] **`darkmatter` runner** (`darkmatter/lib/tests/l1/entry_point_parity.rs`,
  currently modified in the working tree; preserve those edits).
- [x] **`dmls` runner** (`darkmatter/dmls/tests/l1/`; `repository_contexts.rs`
  is modified in the working tree; preserve those edits).
- [x] **`claudine-cli` runner** for the completion cells.

### Wave 14 (after Wave 13)

- [x] **Cross-entry comparison.** For each cell, assert every consumer that
  shares a table row returns the same ordered list or the same
  `ResolutionFailure`; any divergence is fixed in code, not excused in the test.

**Checkpoint 7:** criterion 20 verified on macOS and Linux locally; Windows
via the cross-check in Phase 8.

---

## Phase 8: Docs, Skills, Evidence, and Hand-off

Goal: the `docs/` tree and skills describe the new behavior; the Definition of
Done is demonstrably met.

### Wave 15 (parallel; disjoint files)

Docs audience is a developer with no experience of this repository: lead with
what the reader can do, give a compact example per rule, and use a Mermaid
diagram where a flow is easier to see than to read. A `docs/` page never links
to or names a feature or fix (by path or `{date}-{name}).

- [x] **biscuit-file docs.** `biscuit-file/docs/topics/file-references.md`:
  `GlobReference` section (constructor, the five calls, native order with a
  diagram, `take_first`, relative boundary for globs, skipped symlinks, when
  to choose it over `FileReference`, literal-vs-class brackets); rewrite
  "Recursive Search" as local-first. Update `biscuit-file/README.md` and
  `biscuit-file/docs/dependencies.md`.
- [x] **Darkmatter schema docs.**
  - `darkmatter/docs/topics/schemas/definition.md`: `file` row and
    `match(globs)` section (one example per prefix, the file-name view, the
    advice to use `&`, `^`, or `@` for a fixed meaning, vault example),
    "Repository Trigger Schemas" rewritten (five roots, `$path` allowed and
    forbidden prefixes, user-level triggers), the bare-name `$schema`
    paragraph near line 951.
  - `dmls-schema-support.md` (diagram near 57; statements near 76 and 183 that
    DMLS does not read the schema-folder variable; the editor-environment
    caveat and server-restart note).
  - `authoring-schemas.md` (discovery list near 185, `{root}` outside a
    repository, `SCHEMA_DIR` mentions incl. near 266).
  - `schema-activation.md` (sentence near 105).
  - `claudine/docs/rollout-strategy.md` (rename near 191).
  - Rename every `SCHEMA_DIR` to `SCHEMAS_DIR` and describe it as the schemas
    folder itself, any folder the user chooses (criterion 34); verify with
    `grep -rn "SCHEMA_DIR" docs .claude/skills` returning nothing.
  - Module doc of `triggers/discovery.rs` (the ancestor-walk text).
- [x] **Darkmatter inline/expression docs.**
  `darkmatter/docs/inline/file-links.md` (prefixes, merged roots, boundary,
  skipped-symlink warning); `darkmatter/docs/topics/darkmatter-expressions.md`
  and `darkmatter/docs/schemas/expression-functions.yaml` (native order over
  all roots replaces "sorted absolute paths"; the warning).
- [x] **Claudine docs.** Any topic page describing `match()` completion
  (order, `PortablePath` rendering, the omitted skipped symlinks).
- [x] **Skills.** Update `.claude/skills/biscuit-file/`, `darkmatter/`, and
  `claudine/` for the new types and behavior; add any OS-specific fact learned
  to `.claude/skills/os/` in the same change. Keep each `SKILL.md` under 200
  lines, linking to details. Add the new `GlobReference` names to the
  `biscuit-file` skill description triggers.
- [x] **Doc-comment drift pass.** For every symbol whose behavior changed
  (`FileMatchGlobs`, `find_files_fn`, file-links discovery, `schema_roots`
  replacement, `resolve_recursive_core` replacement), confirm `///`/`//!` and
  inline comments match; where code and comment disagree, the code is right;
  report each drift fixed.
- [x] **CLAUDE.md check.** Only edit if a repo-wide convention changed; expect
  no change.

### Wave 16 (sequential; after Wave 15)

- [ ] **Full local validation** in every affected area: `just lint`, `just test`,
  `just test-l2` for `biscuit-file`, `darkmatter`, `darkmatter-cli` (same
  area), `dmls`, `claudine`, `claudine-cli`. Report each failure with output;
  fix or escalate.
- [x] **Criterion-to-test table.** Append to the implementation log one row per
  acceptance criterion (1–34) naming its test file and function, and mark any
  criterion without a test as a gap to close before proceeding.
- [ ] **Windows cross-check** (pre-authorized, Decision 21):
  `just cross-check <pkg> --os windows` for each of `biscuit-file`,
  `darkmatter`, `darkmatter-cli`, `dmls`, `claudine`, and `claudine-cli`;
  load the `os` skill first. Record each pass (or failure and fix) under
  "Windows Evidence". `ci:all-os` and extra CI cells are **not**
  pre-authorized; ask first.
- [x] **`just ci-local --plan` review.** Confirm the plan's executing cells
  match expectations (per the CI discipline in `CLAUDE.md`); do not push.
- [ ] **Finalize the implementation log:** every changed output
  (`find_files()`, `::file-links`, `%`, schema discovery, `md schema triggers`,
  shipped docs and tests), every departure from the spec (including R2, R3,
  R4, and R6 outcomes), the handed-off reads deleted, and the deliberate
  omission of the first-root list mode.
- [ ] **Set spec status** to "implementation complete, ready for review"
  (frontmatter `status: implemented`, `implemented: true`,
  `implemented_by: claude/sonnet`); do **not** move the spec to `_completed`
  and do **not** run `just complete`.

### Hand-off (author-owned, not agent tasks)

- Commit (signed, no agent attribution), open the pull request, watch Linux and
  macOS green, merge, and watch the post-merge push to `main` (which adds
  Windows) to completion; Decision 22: one merge for both specs.

**Checkpoint 8 (done):** every Definition of Success box above is checked.
