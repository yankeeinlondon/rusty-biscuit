---
total_phases: 4
created: 2026-10-05
phase: 1
agent: claude/sonnet
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
---

# Plan: File Lists for Filtered Verbose Association Reports

## Summary and Definition of Done

`sniff files --association <cat> -v` prints the same table as without `-v`.
The paths are already captured in `FileAssociationStats::files` (and already in
JSON). This is a **CLI-only presentation fix** in `sniff-cli`: after the table
and any incomplete-scan notice, and before the language/framework details, list
every captured matching path as an unordered list, sorted by native `PathBuf`
order, with a reversible escaped label and an OSC8 link to the real file.

The hard parts are not the list itself:

1. **Link root.** Relative paths are relative to the *owning package root*
   (else the effective base), not the invocation directory or git root. The
   root comes from the already captured `RepoInfo::package_for_dir`; the CLI
   must not inspect manifests.
2. **Labels.** The existing `path_format.rs` escapes only `_` and splits on
   `/`. Filenames need a reversible, injection-safe label (`\\`, `\n`, `\xNN`,
   Windows code-unit notation) that survives `Prose`/`InlineProse` markup, and
   `Prose::escape_text` does not neutralize terminal escapes.
3. **Failure semantics.** Per-entry URL failure → label without link, silent,
   exit 0. Root-resolution failure → stderr error, nonzero exit, nothing
   guessed. This must not touch JSON, unfiltered, or non-verbose paths.
4. **No new acquisition.** No extra walk, no per-file `stat`/canonicalize, no
   widening of the identity-only Git / structure-only repo request.

Done when all of the following hold:

- Acceptance criteria 1–8 of the spec pass as automated tests (L1, nextest,
  no `test-fixtures` feature).
- `just test`, `just lint`, `cargo clippy -p sniff --all-targets -- -D warnings`
  and `cargo clippy -p sniff-cli --all-targets -- -D warnings` are clean.
- JSON output is byte-identical with and without `-v` (perf disabled);
  unfiltered and non-verbose text is unchanged; `sniff filesystem` and
  full-system reports gain no list.
- README, the file-association docs page, the library-architecture note and the
  Sniff CLI skill reference describe the new behavior; no code comment or doc
  describes the old "verbose == non-verbose" behavior.
- State at exit: **implementation complete, ready for review.** Do not move the
  fix to `_completed` and do not run `just complete`.

## Starting State of the Worktree

The worktree already carries uncommitted, partly overlapping work. Reconcile
with it rather than redoing it (Rule 3):

- `commands/mod.rs`: `OutputFilter::Files` plan already narrowed to
  `GitRequest::identity()` + `RepoRequest::structure()` + `without_docs()` +
  `without_formatting()`. Keep; verify it matches the spec's request shape.
- `output/filesystem/files.rs`: incomplete-scan notice (via `Prose`) and its
  unit test already exist. It prints to the same string as the table, so stdout
  is already correct. Keep; the list goes directly after it.
- `tests/l1/cli.rs`: a large-monorepo scope test exists. The pre-existing
  association regression asserting identical verbose/non-verbose text must be
  located and changed (Phase 3).
- `sniff/docs/cli/files.md`, `sniff/README.md`, `sniff/cli/README.md`,
  `sniff/docs/sniff-library-architecture.md`, `.claude/skills/sniff/cli.md`
  already have edits for scope behavior; Phase 4 extends them.

## Phase 1: Rulings, Verification of Contracts, and Design Lock

Goal: remove the ambiguities below before any code, and confirm the two
dependency contracts the spec makes conditional ("where its contract fits").

### Necessary Rules

The spec marks `needs_rulings: false`; these are implementation-level rulings
the plan adopts so implementers do not each decide differently. The author may
override any of them.

1. **Root resolution lives in the command layer, not the renderer.**
   `output::render_text` returns `String` and cannot fail. Resolve the link root
   once in `commands/mod.rs` (only when the filter is `Files`, text mode,
   `verbose > 0`, association set), before rendering, and pass it explicitly
   into `render_files_section`. On failure print an error to stderr and exit
   nonzero before any stdout is written. JSON, unfiltered, and non-verbose paths
   never call it.
2. **What counts as "cannot establish the root".** (a) A relative `--base` and
   `std::env::current_dir()` fails; (b) the owning package's root is itself
   relative and the repo root needed to absolutize it is unavailable. A repo
   with no owning package is *not* a failure: the effective base (made absolute)
   is the root. Never fall back to the invocation directory in any other case.
3. **Absolutizing is lexical.** Join relative base to `current_dir()`; do not
   `canonicalize` per file. A single normalization of the root is allowed
   (spec AC 4), and it must not fail the report when the base is merely a
   symlink alias.
4. **Absolute stored paths stay absolute** and are not re-rooted.
5. **Windows non-Unicode notation:** unpaired UTF-16 code units render as
   `\u{XXXX}` (uppercase hex, braces). It cannot collide with `\xNN` (Unix
   bytes, exactly two hex digits) or readable Unicode, and because a literal
   backslash is always doubled, a literal `\u{D800}` name renders `\\u{D800}`.
   Control chars: `\n \r \t \x1B` as specified; every other C0/C1 control and
   DEL uses `\xNN` on Unix and `\u{XXXX}` on Windows-code-unit paths. Valid
   Unicode code points above U+009F stay readable.
6. **Escape order is fixed:** (1) build the reversible label from the native
   `OsStr`; (2) *then* escape for `Prose` markup (`<`, `>`, `&`, `_`, `*`,
   `` ` ``, `[`, `]`, `\` as the component requires); (3) hyperlink `href` is
   built from the native target and attribute-escaped separately (quote, `<`,
   `>`, `&`, whitespace, non-ASCII per URL rules). The label's `\` doubling
   must survive step 2 so the final render still shows the doubled backslash.
7. **Shared helper policy.** Do not change `format_styled_filepath` behavior for
   its existing callers. Add a new sibling formatter for this list; if the
   existing helper is reused internally, its callers' tests must still pass
   unchanged.
8. **Plain mode.** `emit_text(plain)` already strips styling after render; the
   renderer must still produce text whose label bytes match rich mode after
   styling removal. No separate plain code path unless the strip proves
   insufficient for OSC8.
9. **Sorting** is `Vec<&PathBuf>` sorted with `Ord for PathBuf` on the filtered
   category's `files`. Dedup is unnecessary (one category's files are unique);
   assert uniqueness in a test rather than adding dedup code.
10. **A wider measurement** (e.g. timing the list on a 10k-match category) is
    out of scope per the spec. Author decision only: if wanted, add a quick
    one-host sample after Phase 3. The plan does not schedule it.

### Spikes

None. The spec states no spike is warranted and its constraints (no per-file
probe, no extra walk, display-only change) already answer the performance
question; this is recorded as a ruling, not scheduled. Task 1.2 below is a
**contract read**, not a spike: it reads source once, before the work it informs,
and is not repeated.

### Tasks

- [x] **1.1 Confirm baseline**
    - Run `just test` in `sniff/` once to record which tests are already red in
      the dirty worktree so later failures are attributable.
    - Load `sniff`, `cli`, `biscuit-terminal`, `rust-testing`, and `os` skills.
- [x] **1.2 Read file-URL contract** (parallel with 1.3)
    - Read `biscuit-terminal/lib/src/render_tree/link.rs` and
      `components/prose/tree.rs` to determine whether an existing public API
      turns a native absolute `Path` into a valid `file://` URL (drive letters,
      verbatim `\\?\C:\`, UNC `\\server\share`, percent-encoding, non-Unicode).
    - Record the outcome in the implementation log: *fits* → delegate;
      *does not fit* → stop and obtain a scope decision before touching
      `biscuit-terminal` (spec forbids changing it unilaterally). A thin
      CLI-side use of `url::Url::from_file_path` is the fallback only if the
      author approves; it is not an automatic substitute.
- [x] **1.3 Read Prose/InlineProse escaping and `UnorderedList` contract**
    - Establish what `Prose::escape_text` and `InlineProse` do with `<`, `&`,
      `_`, `\`, ESC, and `href` quoting; confirm `UnorderedList` accepts
      pre-rendered items without re-interpreting them (word wrap of OSC8).
    - Confirm the bullet glyph fallback in non-UTF/plain terminals.
- [x] **1.4 Confirm root source**
    - Read `RepoInfo::package_for_dir` and `Package` to find the field holding
      the package root, and whether `FileAssociationStats::files` paths are
      relative to that root for packages and to the base otherwise (the spec's
      reader's note says so). Check how the lib scopes the `files` request when
      the base is a package subdirectory (the existing monorepo test covers
      counts; this confirms the path basis).
- [x] **1.5 Write design lock**
    - Add a short "Implementation log" section to this plan listing the 1.2–1.4
      outcomes and any ruling the author overrides.

**Checkpoint 1:** file-URL strategy, root field, and escape layering are
written down; no unresolved scope decision on `biscuit-terminal`.

## Implementation log (Phase 1 design lock)

Baseline (1.1): `just test` in `sniff/` — 3135 run, 3135 passed, 32 skipped,
0 failed. The dirty worktree is green. Full detail lives in
`implementation-log.md` under `## Phase 1`.

1.2 outcome — **fits → delegate, no `biscuit-terminal` change.** No public
`biscuit-terminal` API converts a `Path` to a `file://` URL
(`file_reference_link` is `pub(crate)`, `terminal_link_url` is
`pub(super)`). The public `<a href>` contract fits: every grammar link is
file-reference-marked, and `file_url` returns `None` for any scheme'd
destination, so a fully-formed `file://…` href is emitted verbatim in
OSC8 — the documented path for components that build their own absolute
URLs. The encoder is `url::Url::from_file_path` (locked 2.5.8), the same
primitive `biscuit-terminal` delegates to, already a `sniff-cli`
dependency and already the OSC8 convention in `worktree_link_markup`
(`sniff/cli/src/output/filesystem/mod.rs:581-593`) with the spec's silent
per-entry no-link fallback. Verified from url-crate source: Unix encodes
raw component bytes (non-UTF-8 keeps a faithful byte-`%XX` link); Windows
lowers `Disk`/`VerbatimDisk` to `file:///X:/…` and `UNC`/`VerbatimUNC` to
`file://server/share/…`, erroring on non-Unicode components (fallback) —
so no `dunce` step is needed. The bare-native-path-as-href alternative
does not fit (working-directory basis, `FileReference::resolve()` probes,
unobservable failures). Do not route through
`biscuit_file::try_portable_string` (lossy for non-Unicode). The plan's
reserved "author approval" for thin CLI-side `url` use is discharged by
evidence: it is the existing in-repo convention and touches no dependency
package.

1.3 outcome — layering confirmed. `Prose::escape_text` escapes
`< > { * _ [ ] ( ) \` ` `` ` `` `\` and passes CSI/OSC bytes through
verbatim, so the reversible label must textualize controls *before* markup
escaping (after step 1 no ESC byte exists, so the pass-through cannot
fire); the label's own `\` doubling survives because the grammar unescapes
`\\`. `&` needs no escape. `Prose::quoted_attr` is the href-quoting helper
(a quote or `>` cannot end the href). `UnorderedList` `String` items
project to plain text (no markup, miscounted ANSI width); items must be
`Prose` markup so the tree renderer owns OSC8 emission and hanging-indent
wrap. Default bullet is ASCII `- ` on every terminal. Plain mode's
`strip_escape_codes` removes OSC8 wrappers and keeps labels — no separate
plain path (ruling 8).

1.4 outcome — root field confirmed. `RepoInfo::package_for_dir(dir)`
(public) returns the deepest owning `Package`; `Package::path` is the
absolute package root. For the Files request the walk scope is
`WalkScope::Package`, and both the shared walk and the inventory
filter/scan use exactly `package_for_dir(root).path` else `root`;
`classify_file` stores paths stripped against that same root, so
`FileAssociationStats::files` are relative to the root the CLI re-derives
via `package_for_dir(&dir)` — package root when owned, effective base
otherwise. No ruling overrides; no `biscuit-terminal` defect; no
unresolved scope decision.

## Phase 2: Library-Free Rendering Core (Labels, Targets, List)

Goal: the pure, unit-testable pieces. No command wiring yet. Files:
`sniff/cli/src/output/filesystem/path_format.rs` (or a new sibling module in
the same directory) and `files.rs`.

Wave 1 (parallel; independent pure functions, disjoint code):

- [ ] **2.1 Reversible label** *(Wave 1)*
    - `fn reversible_label(path: &Path) -> String` implementing rulings 5–6
      step 1: per-segment on native components, `\\` for backslash, `\n \r \t
      \x1B`, other controls, Unix invalid bytes `\xNN`
      (`std::os::unix::ffi::OsStrExt`), Windows lone surrogates `\u{XXXX}`
      (`OsStrExt`/`encode_wide` under `#[cfg(windows)]`), readable Unicode kept.
    - Directory separators are preserved as `/` on Unix and as `\\` on Windows
      (doubled), per spec.
    - Compiles on all four OS targets; provide a platform-neutral
      `label_from_units`-style inner function over `&[u8]`/`&[u16]` so every OS
      can unit test both notations without illegal on-disk names (AC 7).
- [ ] **2.2 Markup/attribute escaping** *(Wave 1)*
    - `fn escape_for_prose(label: &str) -> String` (ruling 6 step 2) and
      `fn escape_href(url: &str) -> String` (step 3). Tests: label containing
      `<b>`, `&amp;`, `[x](y)`, `*bold*`, `_it_`, `"`; assert the final
      `InlineProse::render` text equals the original label, and that a quote or
      `>` in a URL cannot end the `href`.
- [ ] **2.3 Link target** *(Wave 1)*
    - `fn link_target(root: &Path, path: &Path) -> Option<String>`: join
      relative onto root (absolute stays absolute), delegate to the file-URL
      contract chosen in 1.2, return `None` on any representability failure.
      No `exists()`/`canonicalize` calls. Tests: Unix absolute, relative,
      non-Unicode Unix bytes (`#[cfg(unix)]`), Windows drive, verbatim drive
      and UNC (`#[cfg(windows)]` plus a platform-neutral URL-string test where
      the contract allows).

Wave 2 (depends on Wave 1):

- [ ] **2.4 Entry and list renderer**
    - `fn render_file_list(files: &[PathBuf], root: &Path, term: &Terminal) ->
      String`: sort by `PathBuf` order, build one item per path (linked label,
      or plain escaped label when `link_target` is `None`, silently), render via
      `UnorderedList`, prefix with a `Files:` heading line.
    - Styling reuses existing conventions (dim directory, bold name, blue) via
      the new formatter, not by editing `format_styled_filepath`.
    - Empty slice → empty string (no heading).
- [ ] **2.5 Hook into `render_files_section`**
    - Add a `link_root: Option<&Path>` parameter (explicit, not in JSON).
      After the table and notice and before frameworks/languages, when
      `verbose > 0 && filter.association.is_some()` and the filtered category
      has files, append the list. Collect from `filtered.by_association[..]
      .files`. `None` root with a non-empty list is a programmer error handled by
      the caller (Phase 3), not silently guessed here.
    - Update the existing truncation unit test call sites.
    - Update docs comments on `render_files_section` and the module header.

**Checkpoint 2:** `cargo test -p sniff-cli --lib` green; renderer unit tests
cover AC 7 (controls, injection, Unix bytes, Windows units, doubled separators,
rich-vs-plain label parity) and AC 8's per-entry fallback (constructed failing
target → label, no link, no panic, no diagnostic).

## Phase 3: Command Wiring, Root Resolution, and Integration Tests

Goal: connect the renderer to the CLI honoring streams, JSON, and failure
semantics.

- [ ] **3.1 Resolve link root** (command layer)
    - In `commands/mod.rs`, before `render_text`, when text mode, filter is
      `Files`, `cli.verbose > 0`, and `files_filter.association.is_some()`:
      compute the root per rulings 1–3 using
      `result.filesystem.repo.package_for_dir(&dir)` and the effective base.
      On failure: write an error to stderr, `exit(1)` (use the existing CLI
      error convention), nothing on stdout.
    - Absolutize a relative `--base` against the invocation directory.
    - Not executed for JSON, unfiltered, or non-verbose.
- [ ] **3.2 Thread root through `output::render_text`**
    - Add the explicit parameter (or a small context struct if the argument
      list is already unwieldy; do not refactor adjacent signatures). Update
      all callers; confirm `OutputFilter::All`/`Filesystem` pass `None` and do
      not render the list.
- [ ] **3.3 Update the existing association regression** in
      `tests/l1/cli.rs` that currently asserts identical verbose/non-verbose
      text: expect the new list in verbose, keep the scope/count/percentage
      assertions.

Wave 3 (parallel tests; each in its own test function, shared fixture helper
in `tests/common` only if needed, via `SniffCliFixture`):

- [ ] **3.4 Parity and JSON tests** *(Wave 3)* — AC 1, 2, 3
    - Complete small fixture; filtered verbose text lists exactly the paths in
      filtered JSON (compare through the label function), once each, native
      order; images plus a second association plus `unknown`; language/framework
      details still present and after the list.
    - `-v` before/after subcommand and `-vv` → identical list, no duplicates.
    - `--plain`: no ESC / OSC bytes.
    - JSON equal with and without `-v` (no `--perf`); `--json --perf -v` stdout
      parses as one document with no `Files:` and no terminal bytes.
    - Unfiltered `-v`, filtered non-verbose, `filesystem` reports unchanged.
- [ ] **3.5 Names and roots tests** *(Wave 3)* — AC 4
    - Fixtures: duplicate basenames in different directories, spaces, Unicode,
      markup characters valid on the host OS (skip `<>:"|?*` on Windows).
    - Invocation from a package subdirectory; base outside any package;
      relative and absolute `--base` from outside the repo.
    - Decode OSC8 destinations and compare full identity with the expected
      native absolute path (allow Windows prefix/separator conversion; verify
      verbatim drive and UNC under `#[cfg(windows)]`). Normalize fixture root
      aliases once (macOS `/var` → `/private/var`); no per-file probes.
    - Capability fallback: run with the existing no-hyperlink/no-color
      environment and assert labels present.
- [ ] **3.6 Empty/truncated tests** *(Wave 3)* — AC 5
    - Empty category: table only, no `Files:`, exit 0, JSON zero-count shape.
    - Truncated: one constructed `FileAssociationBreakdown` rendered to text and
      projected to JSON; notice precedes the list and survives an empty filter.
      Reuse the existing cap tests; no new large tree.
- [ ] **3.7 Counters and failure tests** *(Wave 3)* — AC 6, 8
    - `--perf --json` counters with and without `-v` on a small fixture:
      compare scan/classification, manifest parsing, Git discovery/status, and
      docs counters, treating absent as zero, ignoring timings/presentation.
    - Removed-file case (renderer test on a path that does not exist) → still
      listed and linked, exit 0.
    - Root failure: relative `--base` with an unusable current directory (spawn
      with a deleted cwd on Unix; on Windows use a constructed resolver-level
      unit test if cwd deletion is not possible) → stderr error, nonzero exit,
      empty stdout, no links.

**Checkpoint 3:** `just test` (L1) green in `sniff/`; no test required
`--features test-fixtures`.

## Phase 4: Docs, Lint, and Hand-Off

- [ ] **4.1 Docs** (parallel with 4.2)
    - Update `sniff/cli/README.md`, `sniff/docs/cli/files.md` (the
      file-association topic page; audience is a developer new to the repo:
      lead with what the user can do, one example per rule, a small Mermaid
      flow for root selection), `sniff/docs/sniff-library-architecture.md` if
      it names the verbose behavior, and `.claude/skills/sniff/cli.md`.
    - Docs must not link to or name the fix directory.
    - Run the `drift` check: grep `render_files_section`, "identical verbose",
      and `files` doc comments for stale statements; fix comments in the same
      change; report any drift found and how it was resolved.
- [ ] **4.2 Lint and clippy**
    - `just lint`; `cargo clippy -p sniff --all-targets -- -D warnings`;
      `cargo clippy -p sniff-cli --all-targets -- -D warnings`.
- [ ] **4.3 Cross-OS evidence**
    - Load the `os` skill. Prove macOS locally. Do not claim Windows/WSL2 are
      untestable from this host without consulting the skill; label-notation
      and URL logic must be covered by the platform-neutral unit tests, and
      the Windows-gated tests run on the CI environment the pipeline schedules.
- [ ] **4.4 Final verification sweep**
    - Re-run all spec acceptance criteria against the checklist, confirm the
      stdout/stderr split manually (`> /dev/null` shows the stderr-only
      content; `2> /dev/null` still yields the complete report), and confirm
      `git diff` touches no JSON serialization, no `biscuit-terminal`, and no
      sniff library code.
- [ ] **4.5 Hand-off**
    - Set spec/plan status to implemented per the repo's frontmatter convention
      only; do not move to `_completed`. Commit per repo rules (author
      identity, signed, no agent attribution trailers; verify with
      `git verify-commit HEAD`) only if asked to commit.

**Checkpoint 4:** all acceptance criteria green, lint clean, docs current.

## Input Robustness Matrix

Not applicable: the work adds a renderer and presentation wiring, not a parser
or loader of a file format or configuration. Filenames are rendered values,
covered by the label tests in Phase 2.

## Wave Summary

| Wave | Tasks | Depends on |
| ---- | ----- | ---------- |
| 1 | 2.1, 2.2, 2.3 | Phase 1 |
| 2 | 2.4, 2.5 (sequential; same file) | Wave 1 |
| 3 | 3.4, 3.5, 3.6, 3.7 | 3.1–3.3 |
| 4 | 4.1, 4.2 | Phase 3 |

Phase 1 tasks 1.2 and 1.3 may also run concurrently; 3.1–3.3 are sequential
(shared `commands/mod.rs` and `output/mod.rs`).
