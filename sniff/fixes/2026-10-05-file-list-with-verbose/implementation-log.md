---
spec: /Volumes/coding/wt/rusty-biscuit/fix-file-association/sniff/fixes/2026-10-05-file-list-with-verbose/spec.md
plan: sniff/fixes/2026-10-05-file-list-with-verbose/plan.md
implemented_by: opencode/zai-coding-plan/glm-5.3
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages: []
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
