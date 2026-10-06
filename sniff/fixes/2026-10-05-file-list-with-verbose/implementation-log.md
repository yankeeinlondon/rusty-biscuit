---
spec: /Volumes/coding/wt/rusty-biscuit/fix-file-association/sniff/fixes/2026-10-05-file-list-with-verbose/spec.md
plan: sniff/fixes/2026-10-05-file-list-with-verbose/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - sniff/cli/src/output/filesystem/file_list.rs
    - sniff/cli/src/output/filesystem/files.rs
    - sniff/cli/src/output/filesystem/mod.rs
    - sniff/cli/src/output/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - sniff/cli/src/commands/mod.rs
    - sniff/cli/src/output/mod.rs
    - sniff/cli/src/output/filesystem/files.rs
    - sniff/cli/tests/l1/cli.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4: []
docs_updated_during_phase_4:
    - sniff/README.md
    - sniff/cli/README.md
    - sniff/docs/cli/files.md
    - sniff/docs/sniff-library-architecture.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/sniff/cli.md
packages:
    - sniff-cli
---

# Implementation Log for 2026-10-05-file-list-with-verbose (4 phases)

## Phase 1

Phase 1 is the contract-verification and design-lock phase: no implementation
code was written, no tests were added, and no packages were touched. Every
task below is a source read whose outcome is recorded here so Phases 2–4 do
not re-decide settled questions.

### Task 1.1 — Baseline

- `just test` in `sniff/` (2026-10-05, this worktree): **3135 run, 3135
  passed, 32 skipped, 0 failed.** The dirty worktree is green; any later
  failure is attributable to this fix's own changes.
- Dirty state matches the plan's "Starting State of the Worktree": modified
  `.claude/skills/sniff/cli.md`, `sniff/README.md`, `sniff/cli/README.md`,
  `sniff/cli/src/commands/mod.rs`, `sniff/cli/src/output/filesystem/files.rs`,
  `sniff/cli/tests/l1/cli.rs`, `sniff/docs/sniff-library-architecture.md`;
  untracked `sniff/docs/cli/files.md` and this fixes directory.
- Verified the pre-existing narrowing in `sniff/cli/src/commands/mod.rs:1326`
  (`OutputFilter::Files` → `GitRequest::identity()` + `RepoRequest::structure()`
  + `without_docs()` + `without_formatting()`, file inventory left enabled)
  matches the spec's request shape. No widening is needed for the list.
- Skills loaded: `sniff`, `cli`, `biscuit-terminal`, `rust-testing`, `os`.

### Task 1.2 — file-URL contract: **fits → delegate; no `biscuit-terminal` change**

Read `biscuit-terminal/lib/src/render_tree/link.rs`,
`components/prose/tree.rs`, and `components/prose/tokens.rs`.

- **No public API converts a `Path` to a `file://` URL.**
  `file_reference_link` is `pub(crate)` (link.rs:27) and `terminal_link_url`
  is `pub(super)` (link.rs:38). Nothing in `biscuit_terminal`'s public
  surface takes a native path and returns a URL.
- **The public `<a href>` contract fits exactly.** Every link built by the
  Prose grammar is marked as a file-reference link (tokens.rs:478).
  `terminal_link_url` passes a destination through unchanged when `file_url`
  returns `None`, and `file_url` returns `None` for any destination with a
  URL scheme of length > 1 (link.rs:64, `has_url_scheme`). So a
  fully-formed `file://…` URL authored as the href is emitted verbatim as
  the OSC8 destination — the link module's documented intent for
  "components that build their own absolute URLs" (link.rs:9-11).
- **The conversion primitive is `url::Url::from_file_path`** — the same
  primitive `biscuit-terminal`'s own `file_url` delegates to (link.rs:80).
  `sniff-cli` already depends on `url = "2"` (locked 2.5.8) and already uses
  `Url::from_file_path(...).ok()` in four places, most relevantly
  `worktree_link_markup` (`sniff/cli/src/output/filesystem/mod.rs:581-593`):
  URL → `<blue><a href="{href}">{label}</a></blue>` markup, with a silent
  no-link styled-label fallback on `None`. That is the established in-repo
  OSC8 convention and already implements the spec's per-entry fallback
  semantics (AC 8). The "thin CLI-side use of `url::Url::from_file_path`"
  the plan reserved for author approval is therefore not a new fallback
  being introduced — it is the existing convention; recorded here as
  discharged by evidence. No `biscuit-terminal` change, no scope decision.
- **Encoder contract, verified from url 2.5.8 source** (so task 2.3's
  Windows tests assert real behavior, not guesses):
  - Unix (`url-2.5.8/src/lib.rs:2964-2975`): each component's **raw bytes**
    are percent-encoded. A non-UTF-8 path does *not* fail; it yields a
    faithful byte-`%XX` URL. Per the spec, such a target "can be
    represented faithfully", so the hyperlink is retained. Only a
    non-absolute path errs.
  - Windows (`lib.rs:3006-3043`): `Disk` and `VerbatimDisk` prefixes both
    lower to `file:///X:/…` (the verbatim prefix is consumed by the
    component walk); `UNC` and `VerbatimUNC` lower to
    `file://server/share/…`; a component that is not valid Unicode
    (`to_str().ok_or(())?`, lib.rs:3043) or a non-Unicode UNC server/share
    errs → per-entry fallback to the escaped label with no link.
  - Consequently **no `dunce` step is needed** for verbatim-disk roots;
    `Url::from_file_path` handles them itself.
- **The alternative — a bare native path as href, resolved by
  `file_url`'s `FileReference` grammar — does not fit**: it resolves
  relative destinations against the *process working directory* and then
  the repository root (link.rs:50-52), which is the wrong basis for
  package-relative paths; `FileReference::resolve()` touches the filesystem
  (the spec forbids per-file probes); and its outcome is unobservable, so
  the per-entry representability fallback of AC 8 cannot be implemented on
  top of it. This confirms the plan's ruling that the CLI carries an
  explicit root and passes absolute URLs.
- **Do not** pipe the native path through
  `biscuit_file::try_portable_string` first (as `worktree_link_markup`
  does): it is lossy for non-Unicode values (`to_string_lossy`,
  `biscuit-file/lib/src/path_text.rs:27`), which would silently produce
  URLs with `%EF%BF%BD` replacement bytes — unfaithful targets the spec
  forbids linking. The new `link_target` (task 2.3) calls
  `Url::from_file_path` on the native absolute `PathBuf` so
  representability failure is detected, not laundered.

**Checkpoint item resolved: file-URL strategy = CLI builds the absolute
native target, converts with `url::Url::from_file_path` (Err → silent
per-entry no-link fallback), and passes the URL string as the `<a href>`
through the Prose grammar, which emits it verbatim.**

### Task 1.3 — Prose/InlineProse escaping and `UnorderedList`: layering confirmed

Read `components/prose/prose.rs`, `components/prose/tree.rs`,
`components/list.rs`, `render_tree/projection.rs`, and
`utils/escape_codes.rs`.

- `Prose::escape_text` (prose.rs:200-257) backslash-escapes exactly
  `< > { * _ [ ] ( ) \` ` `` ` `` and `\` (prose.rs:248). `&` needs no
  escape — the grammar has no entities — so a filename `&amp;` renders
  literally. `InlineProse` shares the same grammar (`blocks`/`tokens`), so
  one escaping function serves both.
- **The ESC pass-through trap is real**: `escape_text` forwards CSI
  (`ESC[…`) and OSC (`ESC]…BEL|ST`) sequences verbatim (prose.rs:205-241).
  Markup escaping alone therefore does not neutralize terminal escapes —
  the spec's warning is confirmed. Ruling 6's ordering is mandatory and
  sufficient: step 1 (reversible label) textualizes every control
  character, so the string handed to step 2 contains no ESC byte and the
  pass-through branch can never fire.
- **The label's own backslashes survive step 2**: `escape_text` doubles
  each `\` to `\\` and the grammar unescapes it on parse, so the final
  render shows the label's own `\\`, `\x1B`, `\n` spellings verbatim
  (ruling 6's "doubling must survive step 2" holds by construction).
- `Prose::quoted_attr` (prose.rs:336-357) is the public href-quoting
  helper: it escapes `< > \`, wraps in the least-colliding quote, and
  escapes the chosen quote, and the tag parser resolves those escapes
  inside attribute values. URL strings from `Url::from_file_path` are
  already free of `"` `<` `>` `\` (percent-encoded), so a quote or `>` in
  a URL cannot end the href (task 2.2's assertion target). The new
  formatter uses `quoted_attr` rather than raw interpolation.
- `UnorderedList` (list.rs:539+) items are `RenderableTerminalContent`. A
  `String` item projects to a plain text node (projection.rs:278) — no
  markup interpretation, no OSC8 generation, and any embedded ANSI bytes
  would be width-miscounted by wrapping. **Items must be `Prose`**: the
  grammar parses `<a href="{url}">{escaped label}</a>`, projects a `Link`
  node (tree.rs:20-22), and the terminal renderer owns OSC8 emission and
  word wrap under the list's hanging indent (list.rs:649-657,
  `configure_component_wrap`). No pre-rendered ANSI is ever passed, which
  is how "word wrap of OSC8" is delegated safely. Existing tests
  (`test_unordered_prose_gets_automatic_wrap`) show the `Prose`-item
  pattern working with wrap + hanging indent.
- **Bullet glyph fallback**: the default bullet is ASCII `"- "` (list.rs:
  553), normalized to `None` in the tree so the terminal renderer falls
  back to `- ` everywhere (list.rs:730-738). Non-UTF/plain terminals get
  an ASCII bullet; the spec's "without requiring a particular bullet
  glyph" holds with the default. Keep the default.
- **Plain mode** (ruling 8): `emit_text(plain)` strips via
  `ANSI_ESCAPE_RE`, which covers CSI, OSC (BEL and ST terminators), and Fe
  escapes (escape_codes.rs:6-13). OSC8 wrappers are removed and the label
  text between them is preserved, so rich-vs-plain label parity needs no
  separate plain code path. Styling wraps the label (`<blue><b>…`) and
  does not interleave bytes inside it, so stripping cannot corrupt it.

**Checkpoint item resolved: escape layering = label (controls textualized)
→ `escape_text` for markup → URL through `quoted_attr` for the href;
list items are `Prose` markup, one per path; default `- ` bullet.**

### Task 1.4 — Root source: `Package::path`; path basis confirmed

Read `sniff/lib/src/filesystem/repo/types.rs`, `filesystem/mod.rs`, and
`filesystem/file_types/{classify.rs,aggregate.rs,model.rs}`.

- `RepoInfo::package_for_dir(dir)` is public (types.rs:279), returns the
  deepest owning `Package`, and `Package::path` is documented "Absolute
  path to the package" (types.rs:158-159). That is the root field. The
  lookup canonicalizes `dir` (fs canonicalize with lexical fallback,
  detection.rs:1375-1378), so symlink-aliased invocation dirs still match.
- For the Files request the walk scope is `WalkScope::Package`
  (filesystem/mod.rs:344-352 — `repo_full` is false because
  `RepoRequest::structure()` is `structure_only`, docs are off, inventory
  is on), and the walk root is exactly
  `repo.package_for_dir(root).map(|p| p.path).unwrap_or(root)`
  (mod.rs:542-558).
- The inventory branch re-resolves the same `package_for_dir(root)`
  (mod.rs:579-621): owning package → filter/scan rooted at `package.path`
  (excluding nested packages); no package → filter/scan rooted at `root`
  itself. `classify_file` stores `entry.path().strip_prefix(scan_root)`
  (classify.rs:260-265), so **`FileAssociationStats::files` are relative
  to exactly the root the CLI will re-derive in Phase 3** — the owning
  package root when one exists, otherwise the effective base as passed to
  detection. The spec's reader's note is confirmed against source.
- Consequence for Phase 3: calling
  `result.filesystem.repo.package_for_dir(&dir)` in the command layer
  reproduces the scan's own basis — no manifest inspection, no guessing.
  When no package owns the base and the base was relative, stored paths
  are relative to that relative root; ruling 3's lexical absolutize
  (join onto `current_dir()`, no per-file canonicalize) reconstructs
  correct absolute targets. Absolute stored paths (never produced by this
  scan, but defended per ruling 4) stay absolute and are not re-rooted.

**Checkpoint item resolved: root field = `Package::path` via
`package_for_dir`; files' relative basis = that same root, else the
effective base.**

### Task 1.5 — Design lock

Recorded in the plan's "Implementation log" section (below in `plan.md`)
and here. **No ruling overrides; no `biscuit-terminal` defect found; no
unresolved scope decision.** The plan's ten rulings stand as written; the
contract reads above pin their details:

1. Root resolution in the command layer, explicit `link_root` parameter
   (plan ruling 1) — consistent with `render_text` returning `String`.
2. `Url::from_file_path` on the native absolute join is the encoder;
   Unix non-UTF-8 keeps its link (faithful byte-`%XX`), Windows
   non-Unicode falls back per entry (1.2).
3. Labels are built before any markup escaping; `escape_text`'s ANSI
   pass-through can then never fire (1.3).
4. List items are `Prose` markup through `UnorderedList`; default `- `
   bullet; plain mode needs no separate path (1.3).

### Requirement-to-test mapping (Phase 1)

Phase 1 changed no behavior, so it adds no tests by design; the plan's
test tasks begin in Phase 2 (renderer units) and Phase 3 (integration).
The gate run for this phase is the baseline itself: `just test` in
`sniff/` — 3135 passed / 32 skipped / 0 failed (Task 1.1). No skipped or
pre-existing failures to report beyond the 32 routine tier/env skips
reported by the suite.

## Phase 2

Phase 2 built the library-free rendering core (plan tasks 2.1–2.5): the
reversible label, the two escape layers, the file-URL link target, the
list renderer, and the `render_files_section` hook. No command wiring —
`output::render_text` still passes `link_root: None` (interim), so CLI
behavior is unchanged until Phase 3, and the pre-existing
verbose==non-verbose association regression still passes as-is.

### Files

- **Created** `sniff/cli/src/output/filesystem/file_list.rs` — tasks
  2.1–2.4: `reversible_label` (dispatching to `label_from_bytes` on Unix /
  `label_from_units` on Windows), `escape_for_prose` (`Prose::escape_text`),
  `escape_href` (`Prose::quoted_attr`), `link_target`
  (`url::Url::from_file_path`, Err → `None`), `file_list_item_markup`, and
  `render_file_list` (sort, `Prose`-item `UnorderedList`, `Files:` heading).
  28 unit tests including test-side decoders that prove exact round-trips.
- **Updated** `sniff/cli/src/output/filesystem/files.rs` — task 2.5:
  `render_files_section` gained `link_root: Option<&Path>`; after the table
  and incomplete-scan notice, before framework/language details, a filtered
  verbose report with a root appends the list of captured
  `filtered.by_association[..].files`. `None` renders no list (the caller,
  Phase 3, resolves the root or reports why it cannot — never guessed
  here). Module header and function docs updated; 6 section-level tests.
- **Updated** `sniff/cli/src/output/filesystem/mod.rs` — registered
  `mod file_list;`.
- **Updated** `sniff/cli/src/output/mod.rs` — the `OutputFilter::Files`
  call site passes `None` pending Phase 3 threading.

`path_format.rs` is untouched (ruling 7: existing formatter's callers
unchanged); the new sibling module owns the list's styling
(dim directory, bold name, blue, OSC8 through the Prose grammar).

### Design decisions recorded

1. **Ruling-5 precision (the one deviation from a plan ruling's letter).**
   A C1 control *code point* in a Unix path (valid UTF-8, e.g. U+0085 as
   `C2 85`) labels as `\u{0085}`, not `\x85`. Ruling 5's sentence "every
   other C0/C1 control and DEL uses `\xNN` on Unix" is written in the
   native-unit (byte) domain; applying `\xNN` to a C1 *scalar* would
   collide with the invalid byte of the same value (`a C2 85 b` and
   `a 85 b` would both label `a\x85b`), violating the spec's
   "distinct native filename spellings must remain distinguishable". The
   ruling's own first sentence supplies the collision-free notation
   (`\u{XXXX}` cannot collide with `\xNN` or readable Unicode), so the
   ruling's intent — visible, reversible, collision-free — is preserved
   and only the notation for this one case is adjusted. C0 controls, DEL,
   `\n \r \t \x1B`, invalid bytes, and readable Unicode follow the ruling
   exactly; on Windows everything follows the ruling exactly. Documented
   in `file_list.rs`'s module docs; round-trip tests
   (`bytes_label_escapes_c1_code_points_without_byte_collision`,
   `bytes_label_round_trips_*`) pin it. No AC distinguishes the two
   spellings, so no spec change is needed; flagged here for the author's
   review-cycle override if wanted.
2. **Dim/bold styling splits the label, not the native path.** The label
   is split at its last separator (`/` on Unix; `\\` or `/` on Windows) so
   the styled parts concatenate back to exactly the full label — a native
   `parent()`/`file_name()` split would double the separator for
   root-only parents (`/x.png` → `//x.png`).
3. **Spacing.** The section already emits one blank line after the
   table/notice; `render_file_list` starts directly with `Files:` so the
   spec example's single blank line holds, and an empty category adds
   nothing (no heading, no blank).
4. **Platform-neutral inner functions** are `#[cfg(any(unix, test))]` /
   `#[cfg(any(windows, test))]`: every OS runs both notations' unit tests
   (AC 7 without illegal on-disk names), while platform lib builds compile
   only their own notation (no dead code).
5. **Per-entry fallback construction.** A relative root makes every
   `link_target` fail (`Url::from_file_path` rejects non-absolute), which
   gives a deterministic, platform-neutral way to exercise the silent
   no-link fallback; Windows-only tests additionally use a lone-surrogate
   path for a real representability failure.

### Test-design requirement mapping (Phase 2)

| Changed behavior | Tests |
| --- | --- |
| 2.1 reversible label — controls visible | `bytes_label_spells_controls_visibly`, `units_label_spells_controls_visibly` |
| 2.1 literal escape-lookalikes distinguishable (original hostile inputs: `re\nport` vs `re\nport`, `e\x1b` vs `e\x1B`) | `bytes_label_distinguishes_literal_escape_lookalikes`, `units_label_keeps_lone_surrogates_distinct_and_readable_astral` |
| 2.1 invalid Unix bytes / Windows lone surrogates (representation variants: paired vs unpaired, C1 scalar vs byte) | `bytes_label_preserves_invalid_bytes_hex_escaped`, `bytes_label_escapes_c1_code_points_without_byte_collision`, `units_label_keeps_lone_surrogates_distinct_and_readable_astral` |
| 2.1 reversibility (round trip both notations; every byte value; arbitrary unit sweeps) | `bytes_label_round_trips_every_single_byte`, `bytes_label_round_trips_mixed_sequences`, `units_label_round_trips_arbitrary_units` |
| 2.2 markup escaping neutralizes injection (labels `<b>`, `&amp;`, `[x](y)`, `*bold*`, `_it_`, `"`, backslash labels) — final render equals original label | `escape_for_prose_renders_markup_sensitive_labels_literally`, `escape_for_prose_survives_styling_wrappers` |
| 2.2 href cannot be terminated (URLs with `"`, `>`, `'`, `<`) | `escape_href_cannot_be_terminated_by_quotes_or_angle_brackets` |
| 2.3 join/absolute/no-probe/None semantics (neutral + `#[cfg(unix)]` + `#[cfg(windows)]`) | `link_target_joins_relative_and_keeps_absolute_platform_neutral`, `link_target_never_probes_the_filesystem`, `link_target_returns_none_when_result_is_not_absolute`, `link_target_percent_encodes_non_utf8_unix_bytes`, `link_target_encodes_spaces_and_unicode_on_unix`, `link_target_encodes_drive_verbatim_and_unc_paths` |
| 2.4 sort order, heading, once-each, empty→nothing, rich/plain parity, list-level injection | `file_list_sorts_paths_in_native_order`, `file_list_marks_heading_and_bullets`, `file_list_lists_each_captured_path_once`, `file_list_empty_renders_nothing`, `file_list_rich_and_plain_labels_agree_after_stripping`, `file_list_label_cannot_inject_lines_or_terminal_sequences` |
| AC 8 per-entry fallback: failing target → label, no link, no panic, no diagnostic | `file_list_falls_back_to_label_silently_when_target_fails`, `file_list_falls_back_per_entry_for_non_unicode_windows_values` (Windows-gated) |
| 2.5 section integration: list after table, before framework/language details; notice precedes list; `unknown` category; no list when non-verbose / unfiltered / empty category / no root | `verbose_filtered_report_lists_captured_paths_after_table`, `file_list_precedes_framework_and_language_details`, `unknown_association_lists_paths_and_notice_precedes_list`, `no_list_without_link_root`, `no_list_when_not_verbose_unfiltered_or_empty`, `truncated_files_report_discloses_partial_sample_even_without_matches` (updated call site) |

Downstream state assertions: the captured `files` vectors are unchanged
by verbosity (`no_list_when_not_verbose_unfiltered_or_empty` re-asserts
the input stats), and stripped rich renders equal the plain-mode label
bytes (`file_list_rich_and_plain_labels_agree_after_stripping` mirrors
`emit_text(plain)`'s strip, ruling 8).

### Evidence (gates run, 2026-10-05)

- `cargo test -p sniff-cli --lib` — **503 passed / 0 failed**
  (Checkpoint 2; baseline 468 → +35 tests).
- `just test` in `sniff/` — **3170 run, 3170 passed, 32 skipped, 0 failed**
  (baseline 3135).
- `just lint` in `sniff/` — clean.
- `cargo clippy -p sniff --all-targets -- -D warnings` — clean.
- `cargo clippy -p sniff-cli --all-targets -- -D warnings` — clean.
- Windows compile evidence from macOS:
  `cargo check -p sniff-cli --all-targets --target x86_64-pc-windows-gnu`
  — clean; `cargo clippy -p sniff-cli --lib --target …gnu -- -D warnings
  -A clippy::result_large_err` — clean.
- Behavioral cross-rig evidence (`just cross-check sniff-cli`):
  **windows pass** (filters `file_list`, `filesystem::file`), **linux
  pass** (filter `filesystem::file`). These runs caught and fixed a real
  defect: two tests had hard-coded the Unix-spelled root `/work`, which is
  *relative* on Windows, so no links rendered; both now anchor on the live
  working directory.

### Pre-existing findings (not from this change, no action)

- `darkmatter` fails `clippy::result_large_err` and
  `sniff/cli/tests/l1/cli.rs:4731` fails `clippy::permissions_set_readonly_false`
  under the `x86_64-pc-windows-gnu` clippy target only. No gate runs
  clippy for that target (CI lint is Linux-only; local lint is the host
  OS), so neither can fail CI; both predate this fix.
- The 32 routine tier/env skips in `just test` are the suite's normal
  skips.

### Skipped or out-of-scope work

None for Phase 2. Command wiring, root resolution, CLI integration tests
(plan 3.1–3.7) and docs (4.1) belong to later phases by design.

## Phase 3

### Session recovery note

This phase began from an interrupted prior session: the worktree carried a
complete-looking Phase 3 implementation (plan checkboxes 3.1–3.7 already
ticked, `commands/mod.rs` wiring, and 12 new integration tests in
`tests/l1/cli.rs`) but no `## Phase 3` log section and no frontmatter. This
session therefore *verified* that implementation task by task against the
plan rather than redoing it (Rule 3), fixed what it found, added the
cross-OS evidence, and wrote this record.

Verification found the implementation sound; two defects were corrected,
both stray comment edits the interrupted pass had introduced in code it was
not supposed to touch (Rule 3 / comment-drift policy — the original
comments were correct):

1. `print_completions` doc comment in `commands/mod.rs` had been rewritten
   to the nonsensical "the CLI sources a command that calls back to the
   CLI"; restored "the shell sources a command that calls back to the CLI".
2. A comment in `run_isolated_software` (`tests/l1/cli.rs`) had been
   grammar-mangled to "version-probe"; restored "version-probed".

### What Phase 3 delivered

- **3.1 Root resolution (command layer).** `resolve_files_link_root` in
  `commands/mod.rs`: called only on the text path when the filter is
  `Files`, `cli.verbose > 0`, and an association filter is set — never for
  JSON, unfiltered, or non-verbose. Root = owning package root via
  `result.filesystem.repo.package_for_dir(&dir)` (a relative package root
  anchors on the absolute repo root; if none, that is a failure), else the
  effective base, with a relative base absolutized lexically against
  `current_dir()` (ruling 3; no per-file canonicalize). Failures propagate
  through `run()`'s `Result` to `main()`'s existing convention —
  `Error: …` on **stderr**, `exit(1)` — before any stdout is written
  (`render_text`/`emit_text` are strictly after the resolution). Six unit
  tests in `commands::tests::files_link_root`.
- **3.2 Threading.** `output::render_text` gained `link_root:
  Option<&Path>` (parameter list already carries `#[allow(clippy::
  too_many_arguments)]`; no context-struct refactor needed). It is the only
  caller of `render_files_section`; every other filter arm passes `None`
  and cannot render a list.
- **3.3 Regression updated.** `files_association_is_stable_inside_a_large_
  monorepo` now expects the captured list in verbose (labels derived from
  the same per-base relative basis the scan uses) and asserts **no**
  `Files:` heading in non-verbose, keeping its scope/count/percentage
  assertions.
- **3.4–3.7 Wave 3 tests** in `tests/l1/cli.rs` (all L1, no tier markers,
  no features) plus two section-level tests in `files.rs`
  (`truncated_breakdown_projects_one_observation_to_text_and_json`,
  `removed_file_remains_listed_and_linked_after_discovery`), which satisfy
  the plan's "constructed breakdown" and "removed-file renderer test"
  bullets at the level the plan asked for.

Test-harness design decisions worth recording:

- A piped subprocess has no OSC8 capability, so the hyperlink renders as
  the `[label](url)` fallback. `assert_files_list_entries` matches entries
  in whitespace-flattened text (the hanging-indent wrap can break anywhere
  inside an entry), and `file_list_destinations` decodes the fallback form
  back to URLs — including undoing the wrap (a line-final `-` is the
  renderer's break marker; the fixtures contain no literal hyphens) and
  the `\]` escaping of a literal `]` in a label. `file_url_to_native_path`
  converts back to a native path, allowing exactly the Windows URL forms
  the encoder produces (forward slashes, lowered drive letter,
  `file://server/share`); `identity_path` normalizes Windows
  `\\?\`-canonicalize output and case for comparison. macOS `/var` →
  `/private/var` fixture alias is normalized once via
  `canonical_fixture_path` (plan: "normalize fixture-root aliases once").
- The unix root-failure integration test deletes the cwd between fork and
  exec (`pre_exec` chdir + rmdir). With the cwd gone, the Git discovery
  every `files` request performs fails first with its own cwd error, so
  the test asserts the boundary contract (nonzero exit, empty stdout,
  `Error:` on stderr, no `file://` anywhere) rather than one specific
  message; the resolver's own failure branches are pinned by the
  `files_link_root` unit tests (rulings 2(a) message included, 2(b)
  constructed without needing an unreadable cwd — the Windows fallback the
  plan prescribed, since cwd deletion is not possible there).
- Counters comparison treats an absent counter as zero, ignores timings,
  and additionally asserts the scoped request keeps
  `filesystem.docs.documents_parsed` and `git.status_walks` at zero in
  both modes (no request widening with `-v`).

### Requirement-to-test mapping (Phase 3)

| Changed behavior | Tests |
| --- | --- |
| 3.1 owning package root wins; base without package; relative base absolutizes; failure cases 2(a)/2(b) | `owning_package_root_wins_over_base`, `base_without_owning_package_is_the_root`, `no_repository_uses_the_absolute_base`, `relative_base_absolutizes_lexically_against_cwd`, `relative_package_root_anchors_on_absolute_repo_root`, `relative_package_root_without_absolute_anchor_fails` |
| 3.1 root failure → stderr error, nonzero exit, empty stdout, no guessed links | `files_verbose_root_failure_errors_on_stderr_without_stdout` (`#[cfg(unix)]`, deleted cwd) |
| 3.2 list renders only for the Files filter with root; nothing for others | `files_unfiltered_nonverbose_and_filesystem_reports_gain_no_list`; lib `no_list_without_link_root` (Phase 2) |
| 3.3 verbose regression expectation | `files_association_is_stable_inside_a_large_monorepo` |
| AC 1 verbose text lists exactly filtered-JSON paths, once each, native order; image + `unknown` + `programming-language`; language details after list | `files_filtered_verbose_text_lists_exactly_the_captured_json_paths` |
| AC 2 JSON byte-identical ±`-v`; `--json --perf -v` one document, no `Files:`, no terminal bytes; unfiltered/filtered-non-verbose/`filesystem` unchanged | `files_json_is_byte_identical_with_and_without_verbose`, `files_json_perf_verbose_stdout_is_one_json_document`, `files_unfiltered_nonverbose_and_filesystem_reports_gain_no_list` |
| AC 3 `-v` position-independent, `-vv`, once each; `--plain` no ANSI/OSC | `files_verbose_flag_position_and_level_show_one_list`, `files_plain_verbose_output_has_no_terminal_escape_sequences` |
| AC 4 duplicate basenames, spaces, Unicode, markup names (Windows-safe subset); package-subdirectory invocation; base outside any package; relative + absolute `--base` from outside the repo; decoded destination identity; capability fallback | `files_verbose_link_destinations_match_native_absolute_paths`, `files_verbose_keeps_duplicate_and_markup_sensitive_names_distinguishable`, `files_verbose_capability_fallback_still_shows_labels` |
| AC 5 empty category: table only, no `Files:`, exit 0, JSON zero-count shape | `files_filtered_verbose_empty_category_keeps_table_and_zero_counts` |
| AC 5 truncated: one constructed breakdown shared by text + JSON projections; notice precedes list; survives an empty filter | lib `truncated_breakdown_projects_one_observation_to_text_and_json`, `unknown_association_lists_paths_and_notice_precedes_list`, `truncated_files_report_discloses_partial_sample_even_without_matches` |
| AC 6 work counters identical with/without `-v` (absent = 0); docs/status counters stay zero | `files_verbose_work_counters_match_non_verbose` |
| AC 8 removed file still listed and linked, no probe, success | lib `removed_file_remains_listed_and_linked_after_discovery`; `link_target_never_probes_the_filesystem` (Phase 2) |

Tier/placement audit: every new test lives in the declared `l1` target
(`tests/l1/main.rs` → `mod cli;`) or the `sniff`/`sniff-cli` lib/bin
`#[cfg(test)]` modules; none carries a tier marker, so nothing is stranded
(`just check-tier-coverage` unchanged by this phase). No test reads a
repository file, so no `include_str!`/`repo_root()` declaration is needed.

### Evidence (gates run, 2026-10-06)

- `just test` in `sniff/` — **3190 run, 3190 passed, 32 skipped, 0
  failed** (Phase 2 close: 3170; +8 lib/bin tests, +12 integration tests).
- `cargo nextest run -p sniff-cli --lib` — **511 passed / 0 failed**
  (Phase 2 close: 503).
- `just lint` in `sniff/` — clean.
- `cargo clippy -p sniff --all-targets -- -D warnings` — clean.
- `cargo clippy -p sniff-cli --all-targets -- -D warnings` — clean.
- Cross-rig (`just cross-check sniff-cli`):
  - **windows pass** — 12/12 `cli::files_` integration tests and 6/6
    `files_link_root` resolver unit tests (the Windows substitute for the
    unix cwd-deletion test, per plan 3.7).
  - **linux pass** — 13/13 `cli::files_` (includes the `#[cfg(unix)]`
    root-failure test). First attempt failed to build on the standing
    clone's `target/release` — the documented kache-hardlink poisoning
    (os skill, build-hosts.md); retried on the native path with
    `--features test-fixtures`, per the documented workaround. No action
    needed on this fix's part.
- `git diff` scope re-checked: only `sniff-cli` sources/tests and this
  fixes directory; no JSON serialization, no `biscuit-terminal`, no sniff
  library code touched.

### Pre-existing findings (not from this change, no action)

- The 32 routine tier/env skips in `just test` (unchanged since baseline).
- `build-linux` standing clone's `target/release` remains kache-poisoned
  (documented in the `os` skill; affects any archive-path cross-check,
  workaround is a feature flag).

### Skipped or out-of-scope work

- WSL2 not exercised locally: the new code paths are shared with Linux
  (same `#[cfg(unix)]` branch), and nightly CI covers WSL2; the plan
  schedules no WSL2-specific evidence for Phase 3.
- Docs, skills, final lint sweep, and hand-off are Phase 4 by design
  (plan 4.1–4.5); none were updated in this phase.

## Phase 4

Phase 4 is the docs, lint, and hand-off phase: no source code, no tests
added, no behavior changed. All plan checkboxes 4.1–4.5 are complete; the
fix's terminal state is **implementation complete, ready for review**.

### Session context discovered at phase start

The worktree no longer carried the Phases 2–3 source changes as uncommitted
edits — they had been committed between sessions as `e3bff2e57` (scope
narrowing, pre-existing work), `b04f2df9c` / `1521b0504` (scope docs/skill),
`df23b3306` (fix(sniff-cli): list captured paths in filtered verbose
association reports), and `92229d0a4` (Phase 2/3 planning records). The
implementation-diff scope check (4.4) therefore runs against
`e3bff2e57^..HEAD` plus the working tree. Nothing was re-done; this phase
verified the committed state and added the documentation sweep on top.

### Task 4.1 — Docs

- **`sniff/docs/cli/files.md`** (the file-association topic page) extended
  for a repo-newcomer audience: a new "Listing the matching files" section
  leads with what `-v` on a filtered report does (with a real invocation and
  output sample captured from the built CLI), followed by one rule per
  bullet (ordering, relative labels, hyperlink/plain behavior, literal-text
  labels, presentation-only verbosity); a new "Link roots" section explains
  package-root-vs-base selection with a Mermaid flowchart covering both
  failure branches (stderr error, nonzero exit, nothing on stdout) and the
  no-probe/no-canonicalize guarantee; "Incomplete scans" now states the
  verbose list covers only the captured sample, after the notice.
- **`sniff/cli/README.md`** — the `files` paragraph now states that `-v` on
  a filtered report lists every matching file hyperlinked against the
  scan's own root, that the scan/percentages/JSON are unchanged, and the
  capped-verbose-list caveat.
- **`sniff/docs/sniff-library-architecture.md`** — the focused-`files`
  paragraph (it names the verbose behavior) now says verbosity does not
  change acquisition because a filtered verbose report only renders the
  matching paths the captured observation already holds.
- **`sniff/README.md`** — one example line added:
  `sniff files --association image -v` (each matching file, hyperlinked).
- **`.claude/skills/sniff/cli.md`** (the Sniff CLI skill reference) — the
  "File association reports" section gained the full list contract: source
  of the list (captured `files` field, never a second walk), link-root
  authority (`package_for_dir`, not the git root), command-layer resolution
  and its failure semantics, the JSON/unfiltered/non-verbose exclusion, the
  reversible-label layering, and the silent per-entry no-link fallback.
  Verified `.opencode/skill/sniff/cli.md` is a hardlink (same inode) to the
  `.claude` path, so the single edit covers both locations named in the
  spec's Phase-4 message.
- No doc links to or names the fix directory (checked by inspection of the
  edited files).

**Drift check (plan 4.1):** greps for `render_files_section`, "identical
verbose", and verbose-behavior phrases across source and docs found **no
stale statements**. The `render_files_section` / `render_text` /
`resolve_files_link_root` doc comments and the `files.rs` / `file_list.rs`
module headers were already updated by Phases 2–3 and re-read this phase;
the only "identical verbose/non-verbose" matches are this fix's own
spec/plan/log, an unrelated `repo_recent-commits.md` sentence about a
different subcommand's `-v`/`-c` interaction, and a completed fix in
another worktree. Nothing to resolve.

### Task 4.2 — Lint and clippy (2026-10-06, all clean)

- `just lint` in `sniff/` — clean.
- `cargo clippy -p sniff --all-targets -- -D warnings` — clean.
- `cargo clippy -p sniff-cli --all-targets -- -D warnings` — clean.

### Task 4.3 — Cross-OS evidence

Loaded the `os` skill. Phase 4 changed Markdown docs only; no source under
test changed since Phase 3's cross-rig runs, so that evidence still
describes the exact code under verification:

- **macOS (this host): proven.** `just test` in `sniff/` — **3190 run,
  3190 passed, 32 skipped, 0 failed** (identical counts to Phase 3 close),
  plus the manual runs below in 4.4.
- **Windows (build-win-native) and Linux (build-linux):** Phase 3 close ran
  `just cross-check sniff-cli` with **windows 12/12 `cli::files_` + 6/6
  `files_link_root`** and **linux 13/13 `cli::files_`** (see Phase 3
  evidence; linux needed the native path per the documented kache-poisoned
  `target/release` workaround). Platform-neutral label/URL unit tests run
  in the L1 suite on every OS; the `#[cfg(windows)]`-gated tests (lone
  surrogates, drive/verbatim-drive/UNC targets) run in the Windows
  environment the CI pipeline schedules (push-to-main and the `ci:all-os`
  label; pull requests prove Linux and macOS).
- **WSL2:** not exercised locally; the new code paths are shared with Linux
  (same `#[cfg(unix)]` branch) and nightly CI covers WSL2. The plan
  schedules no WSL2-specific evidence.

### Task 4.4 — Final verification sweep

- **Acceptance criteria re-run.** Every AC maps to green automated tests
  (mapping in the Phase 3 section; targeted re-run this phase: 45 lib tests
  matching `file_list|files_link_root|truncated_breakdown|removed_file|
  unknown_association|no_list` — 45/45 passed; 13 integration `cli::files_`
  tests — 13/13 passed; all inside the 3190/3190 suite). AC checklist:
  1 ✓ paths-vs-JSON parity, 2 ✓ JSON unchanged/no list bytes in
  `--json --perf -v`, 3 ✓ `-v` position/`-vv`/`--plain`, 4 ✓ names/roots/
  destinations/capability fallback, 5 ✓ empty+truncated, 6 ✓ counters,
  7 ✓ label notation units, 8 ✓ fallback + root failure.
- **Manual stream split** (built CLI, fixture workspace):
  `--base <pkg> files --association image -v 2> /dev/null` prints the
  complete report — table, `Files:` heading, both linked entries — on
  stdout; `> /dev/null` exits 0 with **zero stderr bytes** (the success
  path emits nothing to stderr). The root-failure stderr path is pinned by
  `files_verbose_root_failure_errors_on_stderr_without_stdout` (Unix,
  deleted cwd) and the `files_link_root` unit tests (including the
  Windows-substitute failure 2(b)).
- **JSON parity spot check:** filtered JSON with and without `-v` on the
  same fixture is byte-identical (`cmp` clean).
- **Diff scope:** `git diff --name-only e3bff2e57^..HEAD` plus working tree
  touches only `sniff-cli` source/tests, sniff docs/README, the sniff CLI
  skill, and this fixes directory. **No `biscuit-terminal` file, no
  `sniff/lib` file, no JSON-serialization file** (`repo_json.rs` et al.
  absent) is in the diff.

### Task 4.5 — Hand-off

- Plan and log frontmatter updated with the Phase 4 keys, `source_code`,
  `documentation`, `completed_phase: 4`, `implemented: true`; packages
  remain `[sniff-cli]` (docs live in the sniff area but belong to the
  `sniff-cli` behavior they document).
- Spec frontmatter set to `status: implemented`, `implemented: true`,
  `human_review: false`, with a completion `message_to_agent`.
- Not committed (no request to commit); fix directory **not** moved to
  `_completed`; `just complete` not run. Terminal state: implementation
  complete, ready for review.

### Requirement-to-test mapping (Phase 4)

Phase 4 changed documentation only; by design it adds no tests. The gates
run for this phase are the evidence: `just test` 3190/3190, `just lint`,
both `-D warnings` clippy gates, the 45+13 targeted AC re-runs, and the
manual stream-split/JSON-parity/fixture runs above.

### Pre-existing findings (not from this change, no action)

- The 32 routine tier/env skips in `just test` (unchanged since baseline).
- `build-linux` standing clone's `target/release` remains kache-poisoned
  (documented in the `os` skill; workaround is a feature flag).
- The worktree carries unrelated dirty files from other work
  (kache/os skills, `docs/initialization.md`, `docs/kache-strategy.md`,
  `scripts/cross-check.sh`, `tools/test-kit` CI contract test); none
  intersect this fix.

### Skipped or out-of-scope work

- Author-only actions deliberately not performed: closing the review
  cycle, moving the fix to `_completed`, `just complete`, and the optional
  Phase-1-ruling-10 measurement sample. The Phase 2 ruling-5 notation
  precision (C1 scalars as `\u{XXXX}` on Unix) remains flagged for the
  author's review cycle only; it is not a blocker and needs no decision
  before any next step (there is no next phase).
