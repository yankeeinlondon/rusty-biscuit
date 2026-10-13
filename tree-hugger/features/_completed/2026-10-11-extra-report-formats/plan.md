---
kind: plan
feature: extra-report-formats
spec: spec.md
total_phases: 6
created: 2026-10-05
phase: 6
agent: claude/sonnet
yolo: true
implemented: true
source_files_during_phase_1:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/output.rs
source_files_during_phase_2:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/export.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/symbols/reports.rs
  - tree-hugger/cli/tests/symbols/export_plumbing.rs
docs_updated_during_phase_2:
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/export.rs
  - tree-hugger/cli/src/tsv.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
  - tree-hugger/cli/tests/symbols/export_tsv.rs
docs_updated_during_phase_3:
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/markdown.rs
  - tree-hugger/cli/src/output.rs
  - tree-hugger/cli/src/prose.rs
  - tree-hugger/cli/src/render.rs
  - tree-hugger/cli/tests/symbols/export_prose_markdown.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
docs_updated_during_phase_4:
  - tree-hugger/docs/dependencies.md
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - tree-hugger/cli/src/ctags.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/tests/symbols/export_ctags.rs
  - tree-hugger/cli/tests/symbols/main.rs
  - tree-hugger/cli/tests/level2_declarations.rs
docs_updated_during_phase_5:
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/tree-hugger/SKILL.md
docs_updated_during_phase_1:
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/spec.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - tree-hugger-cli
source_files_during_phase_6:
  - tree-hugger/cli/src/ctags.rs
  - tree-hugger/cli/src/declarations.rs
  - tree-hugger/cli/src/main.rs
  - tree-hugger/cli/src/tsv.rs
  - tree-hugger/cli/tests/fixtures/golden/symbols.tsv
  - tree-hugger/cli/tests/symbols/export_ctags.rs
  - tree-hugger/cli/tests/symbols/export_tsv.rs
  - tree-hugger/cli/tests/symbols/golden.rs
docs_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - tree-hugger/cli/README.md
  - tree-hugger/docs/symbol-reports.md
  - tree-hugger/features/2026-10-11-extra-report-formats/plan.md
  - tree-hugger/features/2026-10-11-extra-report-formats/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/tree-hugger/SKILL.md
  - .claude/skills/os/SKILL.md
  - .claude/skills/os/windows-paths.md
---

# Plan: TSV, Markdown, Prose and Tags Output for Symbol Reports

## Summary and Definition of Done

`hug {functions,types,symbols,classes}` gains four `--format` values: `tsv`,
`markdown`, `prose` and `ctags`. They are presentation-only views over the
declarations the existing reports already select. No extraction, cache-key,
record-schema or selector change is made, and `imports`, `lint` and
`god-files` are untouched.

Today (`cli/src/output.rs`) `FormatArg` is `Plain | Table`, `resolve()` maps a
format to a terminal `View`, and `declarations.rs` dispatches on
`ReportFormat::{Json, JsonFull, JsonLines, Count, Stats, Terminal}`. The work
has five moving parts:

1. **Output resolver**: new `FormatArg` variants, a new `OutputMode` family that
   is *not* a terminal layout, and the spec's option-combination table.
2. **TSV** (`cli/src/tsv.rs`, new): eight fixed columns, one-pass escaping, class
   flattening by record ID, per-file streaming.
3. **Prose and Markdown**: extract the renderer's declaration/outline helpers so
   they can emit unrendered markup; Prose emits that markup, Markdown folds the
   same structure through a CLI-owned CommonMark-escaping adapter.
4. **ctags** (`cli/src/ctags.rs`, new): extended-subset tags with a published
   kind/scope mapping, tags-specific encoding, and a named non-interactive
   reader fixture.
5. **Verification and docs**: one golden per format over `multi/`, focused
   cases, README/`docs/symbol-reports.md`/help/completions/skill, and the
   implementation log.

Success looks like:

- [ ] Every row of the spec's option-combination table has a passing resolver
  test, plus the pairwise mode matrix extended to the four new formats.
- [x] All four formats are UTF-8, LF-only, free of ANSI/OSC bytes **before** any
  normalization, and byte-identical across worker counts, warm/cold cache, and
  narrow/wide `COLUMNS`.
- [x] TSV: one header, exactly eight cells on every row, reversible one-pass
  escaping proven by a decode round-trip test.
- [ ] Markdown parses as CommonMark with the expected nesting and no unintended
  links, HTML, entities, emphasis or blocks from source text.
- [ ] Prose: both corrected plain-comparison checks pass; links survive spaces,
  Unicode, parentheses and Windows paths.
- [ ] ctags: the named reader fixture passes for duplicates, container fields,
  spaces, Unicode and the chosen escaping boundary.
- [x] Output selection adds no references, diagnostics, semantics or metrics
  work; containment is requested only for layouts that need it (counter-based
  assertion, not timing), including recorded ownership for ctags and classes.
- [ ] `just test` and `just lint` pass in `tree-hugger/`; docs agree with
  behavior; the implementation log records the ctags choice and any departures.
  Directory left active for author review.

No Input Robustness Matrix is needed: the feature **writes** formats. It reads no
file format or configuration beyond what existing code already parses. (The
tags reader fixture reads our output only to verify it.)

## Phase 1: Rulings, Spike, and Resolver

### Necessary Rules

The author must confirm or overrule these before Phase 5 (ctags); rules 1-3 also
gate Phase 2. Phases 2-4 do not depend on the ctags rulings, so they can proceed
on the recommended answers while rulings are pending.

1. **Tags contract: documented extended subset** (spec Open Question; the spec
   says its recommendation "is not author approval"). Planned on this basis:
   numeric line addresses, `!_TAG_FILE_FORMAT 2`, `!_TAG_FILE_SORTED 0`, kind and
   container extension fields. If the author picks "minimal traditional tags"
   instead, Phase 5 shrinks (no mapping table) but sorted output then needs a
   buffering or ordering ruling; if "Universal Ctags parity", the work is
   re-specified.
2. **Reader fixture.** Spec requires one named, non-interactive reader. Candidates:
   (a) headless Vim/Neovim reading `taglist()`; (b) a Rust tags-reader crate as
   a dev-dependency. Recommendation: decide from the Phase 1 spike. A crate
   avoids installing an editor on four environments but only proves "a parser",
   not "an editor"; headless Vim proves editor navigation but must be present on
   all CI hosts and must never gain window focus. The spec says a crate or a
   headless editor suffices.
3. **Unusual names/paths policy.** Spec: either prove the reader accepts escaped
   names/paths, or **skip with a stderr warning and exit 1**. Recommendation:
   skip-and-warn for any name, container or path containing a tab, CR, LF or
   other control character (the escape syntax is not interoperable across
   readers); spaces and Unicode are emitted raw and verified. Decided finally by
   the spike.
4. **Kind letter vs. word.** Spec says to "publish a complete mapping for every
   supported record kind and language". Recommendation: emit the extended
   `kind:<word>` form using Hug's snake_case kind spelling (consistent with TSV
   and `--kind`) rather than inventing single-letter Ctags kinds; Unknown and
   imprecise Type map to a generic word and are never promoted to a specific
   kind. Author confirms or asks for letters.
5. **Container field source.** Container comes from recorded ownership
   (`SymbolRecord` parent/container ID), never name equality, and uses the
   genuine container-qualified name without the scan-root file prefix. Rule:
   need to confirm the field exists for every language that has containers; where
   a language lacks recorded ownership, the container field is omitted (not
   guessed).
6. **Which explicit flags error with ctags.** Spec: explicit `--signatures` or
   `--comments` is an error. Rule: "explicit" means the flag appears on the
   command line (including `--signatures short`); a default does not trigger it.
   `GivenOutput.signatures` is already `Option<_>`; `comments` must be added to
   `GivenOutput` as an `Option`/bool so the resolver can see it.
7. **`--comments` with `--format tsv`** is an argument error (spec table); with
   `markdown` and `prose` it is accepted and comment text is literal content.
8. **Markdown dependency for tests.** The spec says to use an existing parser
   dependency where available. Rule: before adding anything, check whether
   `pulldown-cmark` already exists in the lockfile (the repo has a
   `pulldown-cmark` skill and `darkmatter` area). Reuse its version as a
   `dev-dependency` of `tree-hugger-cli` and document it in
   `tree-hugger/docs/dependencies.md`. Rule needing author sign-off only if
   adding a crate not already in the workspace.
9. **Markdown escaping adapter location.** CLI-owned (`cli/src/markdown.rs`);
   no change to `renderable`'s Markdown renderer. Matches the spec's scope line.
10. **Wider cross-OS measurement is not scheduled.** The spec adds no
    performance spike and no CI cells; this plan adds none. If the author wants
    Windows/WSL2 timing for the new writers, that is their call.
11. **TSV `signature` with relation labels.** Spec says the Signature cell equals
    the terminal table's Signature cell (`signature_cell`). Relations shown by
    outlines are *not* in TSV. Confirmed as written; no ruling needed beyond
    recording it.
12. **Unresolved prerequisite decisions** in the three `depends-on` specs are not
    decided here (spec: "not silently decided by this export feature"). If a
    kind or relation decision is still open when Phase 3/5 reaches it, stop and
    ask rather than choose.

### Spike

One spike, run once, before Phase 5. It does not recur in later phases.

- [x] **Tags reader spike** (one host, macOS, a quick sample): hand-write a tiny
  tags file with a space in a filename, a non-ASCII name, a duplicate name, a
  `kind:`/`class:` extension field, and `!_TAG_FILE_SORTED 0`. Feed it to
  (a) headless Vim/Neovim via `taglist()` and (b) the candidate Rust crate.
  Record in the implementation log: which reader is chosen, whether raw
  space/Unicode survive, whether any escaping of tab/newline is accepted,
  whether an unsorted file is read correctly, and whether the editor can run
  with no window. Answers rulings 2 and 3. Do not widen to other OSes; if the
  chosen reader is an editor, listing Windows/WSL2 availability is a ruling for
  the author, not spike work.

### Tasks

- [x] **Resolver variants**
  - Add `Tsv`, `Markdown`, `Prose`, `Ctags` to `FormatArg` in
    `cli/src/output.rs` with doc comments (the current doc says "the terminal
    layout"; fix it).
  - Add an `OutputMode::Export(ExportFormat)` variant that carries the chosen
    export plus `signatures` and `plain`; do not route the new formats through
    `OutputMode::Terminal`.
  - Extend `GivenOutput` with `comments` (explicit flag presence) and update
    `GivenOutput::new` and its call sites in `main.rs`.
- [x] **Combination table**
  - Implement every row of the spec's table in `resolve()`: new format with
    `--json/--json-full/--jsonl/--count` and with `--stats` are argument errors
    (exit 2); `--tree` accepted for `markdown`/`prose` on functions, types and
    symbols, rejected for `tsv`/`ctags`; `classes --tree` still rejected;
    explicit `--signatures`/`--comments` with `ctags` rejected; `--comments` with
    `tsv` rejected; global `--plain` accepted with all four.
  - Update the existing `--tree` check that says "`--tree` is terminal output"
    so it still reads correctly.
  - Update the module doc comment that lists modes ("an explicit `--format`").
- [x] **Resolver tests**
  - Extend `MODES` in the `output.rs` tests with the four formats; assert each
    pair is rejected except the already-accepted `--stats` pairs, and that
    `--stats` with each new format is rejected.
  - One test per remaining table row, asserting the error text mentions both
    flags.
  - Help text: `--format` value list shows all six values.

**Checkpoint 1:** `just test` and `just lint` in `tree-hugger/` pass; `hug functions
--format tsv` parses and fails cleanly with "not yet implemented" (or is wired
in Phase 2); the spike result and rulings 1-3 are recorded in the
implementation log.

## Phase 2: Shared Export Plumbing

Establishes what every format uses. Wave 1 tasks are independent; Wave 2
depends on Wave 1.

### Wave 1 (parallel)

- [x] **Record flattening**
  - Add a helper (in `cli/src/classes.rs` or a new `cli/src/export.rs`) that
    turns a file's selected declarations into an ordered `Vec<&SymbolRecord>` for
    TSV/ctags: for `classes`, flatten each selected container and its displayed
    methods and constructors, apply the same per-file order, preserve member
    selectors and static/instance filtering, **de-duplicate by record ID**, never
    synthesize field records from class-summary field metadata, keep synthetic
    prelude records for TSV (but not for ctags).
  - Unit tests: a method reached through two containers is emitted once;
    a field-only summary entry is absent; ordering matches `--sort`.
- [x] **Path projection**
  - Confirm `portable_relative` (in `report.rs`) gives the `/`-separated
    scan-root-relative file for TSV/ctags and the existing outside-root
    fallback; do not invent paths. Add a unit test for an outside-root file
    that asserts the fallback unchanged and no symbol-identity change.
- [x] **Control-character neutralizer reuse**
  - Reuse `neutralized`/`literal` in `render.rs` (they already map C0, DEL and
    C1 to `\u{..}` and are `pub(crate)`). Add a TSV-oriented cell encoder in
    Wave 2 that handles backslash/tab/CR/LF first, then defers to `neutralized`
    for the rest. Test the ordering: `\t` must not become `\u{9}`.

### Wave 2

- [x] **Format dispatch**
  - Add `ReportFormat::Export { format, signatures, plain, ... }` in
    `cli/src/declarations.rs` and a `write_export(...)` arm in the dispatch
    `match scan.format`. Each export writer follows `write_terminal`'s pattern:
    `settle` each file through `Sink`, report failed files to stderr, keep
    exit-1-on-any-failure, `BrokenPipe` ⇒ exit 0, other write errors ⇒ exit 1.
  - Ensure the query requested for exports is the *same* as the equivalent
    listing: no references/diagnostics/semantics/metrics, containment only for
    layouts that need it (`--tree`, `classes`, Markdown/Prose outlines; **not**
    TSV flat or ctags unless ownership is needed for the container field, in
    which case it is requested only then and a test asserts it).
  - Streaming: after each completed batch write rows; hold at most one file's
    rows beyond existing scan/prelude buffering.
- [x] **Shared error behavior tests** (against a stub format or TSV once Phase 3
  lands): one failed file alongside a successful one; empty selection exits 0;
  a scan with no source files keeps the existing discovery error; closed stdout
  pipe exits 0; other write failure exits 1; verbose/cache messages on stderr
  only.

**Checkpoint 2:** the dispatch compiles for all four formats with each writer
returning a minimal, tested stub; `just lint` is clean.

## Phase 3: TSV

Depends on Phase 2. TSV, Prose/Markdown (Phase 4) and the ctags mapping design
(start of Phase 5) touch different files, so Phase 3 and Phase 4 may run in
parallel as separate subagents once Phase 2 is merged, provided the
`render.rs` extraction in Phase 4 does not alter `signature_cell`, which TSV
reuses (Phase 4 must not change its behavior).

- [x] **TSV writer** (`cli/src/tsv.rs`)
  - Header exactly `file\tline\tcolumn\tkind\tname\tqualified_name\tvisibility\tsignature`,
    emitted once, including for an empty selection.
  - `line`/`column` from `declaration_span.start` (1-based, never the
    identifier span); `kind` is the snake_case spelling matching JSON and
    `--kind`; `visibility` snake_case with Unknown and any missing value
    empty (never `null`/`None`); `qualified_name` includes the scan-root prefix
    from the existing query finishing rules.
  - `signature` is `signature_cell(record, mode)`: `none` leaves the cell empty
    but keeps the header, `full` uses the recorded header with the existing
    short fallback.
  - Exactly eight cells on every row, including trailing empty cells.
- [x] **Cell encoder**
  - One pass: `\` → `\\`, TAB → `\t`, CR → `\r`, LF → `\n`; other controls via
    the existing visible `\u{..}` notation; never re-escape already encoded
    text; Unicode, quotes and spaces unchanged.
  - Test `\` followed by `n` encodes as `\\n` and is distinguishable from an LF
    encoded `\n`. Write a one-pass decoder **in the test module** to prove
    reversibility for a corpus including tabs, newlines, CR, backslashes, C1
    controls, and an embedded ANSI sequence.
- [x] **Ordering and grouping**
  - Files in portable path order; `--sort` orders records within each file;
    grouping flags add no headings or blank rows and do not change order.
- [x] **Classes in TSV**
  - Use the Phase 2 flattening helper; assert exact agreement with `classes`
    static/instance rules and member selectors.
- [x] **TSV tests** (`cli/tests/symbols/`, new `export_tsv.rs` registered in
  `main.rs`)
  - Header-only on an empty selection, a row with every empty trailing cell,
    visibility/kind spelling against JSON for the same record, all three
    signature modes, a source name with embedded control characters and ANSI.
  - A cross-check test: TSV rows equal the `--jsonl` records' `file`, `line`,
    `column`, `kind`, `name`, `qualified_name` for the same selectors.
  - `--comments` with TSV is an argument error (assert via the CLI, exit 2).

**Checkpoint 3:** TSV tests pass; `hug functions --format tsv` on a scratch copy
of `multi/` opens as 8 columns in a quick `awk -F'\t' '{print NF}' | sort -u`
spot check (single value `8`).

## Phase 4: Prose and Markdown

Depends on Phase 2. Wave 1 is the shared extraction; Wave 2 runs Prose and
Markdown in parallel after it.

### Wave 1

- [ ] **Extract unrendered markup helpers**
  - In `cli/src/render.rs`, extract the pieces that currently build Prose
    markup and then fold it to a terminal string — declaration text
    (`declaration`), signature formatting (`shown`, `signature_cell`,
    `compact_signature`), location labels (`display`, `link`), grouping
    (`grouped`, `module_key`, `module_title`), class sections
    (`method_section`, `field_section`) and outline ownership (`outline`,
    `outline_node`) — behind functions that return **markup/structure**
    (strings of Prose markup or render-tree nodes) rather than writing to a
    terminal. The terminal renderer must call the same helpers.
  - Do **not** duplicate symbol formatting; do not change the terminal bytes.
    Verification: the existing goldens and `just test` remain unchanged.
  - Update doc comments that say these paths are terminal-only (spec's drift
    note).
  - The `ListingStream` buffering rules (one file / directory / package) must
    stay the single source of buffering for terminal, Prose and Markdown.

### Wave 2 (parallel)

- [ ] **Prose writer** (`cli/src/prose.rs`)
  - Emit the unrendered Prose markup using the extracted helpers, preserving the
    current layout: package sections, optional file/module groups, outline
    indentation, static/instance class sections, doc lines under `--comments`;
    default output stays un-file-grouped.
  - Each generated style/link tag closes within its line.
  - All source text goes through `literal`; attribute values through
    `Prose::quoted_attr`. Source-location links are generated independent of
    terminal detection with the existing file-URL encoder and `#L<line>`.
  - Global `--plain` omits generated styles and links but keeps literal
    escaping and layout.
  - No ANSI/OSC bytes, no wrapping, LF only regardless of `COLUMNS`.
- [ ] **Prose tests**
  - Backtick runs, tags, emphasis and link syntax in names, signatures,
    comments and paths remain inert (render the Prose back through
    `biscuit-terminal` and assert no style/link was introduced).
  - Link URL cases: spaces, Unicode, parentheses, and a native Windows path
    (unit-level, so it runs on every host).
  - **Two comparisons from the spec**, each over matching commands, selectors,
    signature modes, comments and grouping: (1) unwrapped, ANSI-stripped
    rendering of styled Prose on an OSC-8-capable terminal equals
    `--format plain`; (2) `--plain --format prose` rendered with no
    styles/links equals `--format plain`.
- [ ] **Markdown adapter** (`cli/src/markdown.rs`)
  - Build the list structure with `UnorderedList` or the shared render-tree
    nodes and Prose-derived inline nodes; apply **CommonMark literal escaping to
    source text nodes at the output boundary** (not to the finished string),
    covering `[x](url)`, `**x**`, backticks, `<img>`, `&copy;`, `#`, `-`,
    numbered-list prefixes, `|`, and leading indentation.
  - Headings for package/module/file groups with blank-line separation;
    comments and continuation lines indented under their owning item as
    literal text; `(no symbols)` for an empty selection using the existing
    package-section behavior; locations are readable labels, **no** generated
    file links; presentation-only color/dim dropped, semantic emphasis kept;
    global `--plain` does not change Markdown.
  - Overloads stay separate; relation labels (including under
    `--signatures none`) and per-file outline order inherited.
  - Output free of ANSI/OSC; control characters made inert.
- [ ] **Markdown tests**
  - Parse output with the Phase 1 parser dev-dependency; assert the event
    stream has the expected list nesting and **no** link, HTML, entity,
    emphasis or heading/block events originating from source text, for a
    hostile-names fixture (one case per construct above).
  - Flat, `--tree`, `classes`, grouped, filtered-out container context headings
    (never a declaration), empty selection.
  - Stability: same bytes with different `--jobs`/worker counts, warm vs. cold
    cache, `COLUMNS=20` and `COLUMNS=200`.
  - Golden tests alone do not satisfy the escaping contract; the hostile
    fixture must exist.

**Checkpoint 4:** Prose and Markdown tests pass; existing terminal goldens
byte-identical (prove the extraction was behavior-preserving); `just lint`
clean.

## Phase 5: ctags

Gated on rulings 1-6 and the spike. Depends on Phase 2; the flattening helper
and output dispatch exist already.

### Wave 1 (parallel)

- [x] **Kind/scope mapping table**
  - Create the published mapping for every supported record kind **and**
    language (16 in the `multi/` fixture): Hug kind → tags `kind:` value;
    Unknown and imprecise Type never promoted; where a language has no
    container notion the container field is omitted. Encode as a table in code
    with an exhaustive `match` (so a new kind fails compilation) and document it
    in `docs/symbol-reports.md`.
  - Container field from recorded ownership (never name equality), using the
    genuine container-qualified name without the scan-root file prefix; a unit
    test with a method whose name equals its class name proves no name-equality
    inference.
- [x] **Tag line encoder**
  - Separate encoding for name, filename and extension values per the tags
    standard; **not** the TSV encoder. Apply the ruling-3 policy: an entry
    whose name, container or path contains a tab/CR/LF/other control character
    is skipped with a stderr warning naming the file and line, and the run
    exits 1; never insert raw tabs/newlines or fabricate a filename.
  - Numeric address: `line;"` followed by extension fields, never an Ex
    command derived from source text.

### Wave 2

- [x] **ctags writer** (`cli/src/ctags.rs`)
  - Header lines `!_TAG_FILE_FORMAT	2	/extended format/` and
    `!_TAG_FILE_SORTED	0	/0=unsorted/`; empty output retains the headers.
  - One tag per selected record; preserve per-file report order even with
    `--sort name` (no global tag-name sort); retain overloads and duplicates;
    omit synthetic/locationless records and class-summary field-only entries;
    for `classes`, selected containers plus displayed methods/constructors once
    by ID.
  - Paths relative to the scan root with `/` separators on all OSes; outside-root
    paths follow the documented policy.
- [x] **Reader fixture** (`cli/tests/symbols/export_ctags.rs`)
  - Implement the chosen reader (ruling 2) as a non-interactive test: generate
    tags for a scratch tree containing duplicate names, a container-qualified
    method, a directory with a space, and Unicode names; assert lookup and
    numeric-line navigation for each, plus the skip-with-warning path (stderr
    message, exit 1, the skipped entry absent, remaining entries present).
  - If the reader is an editor it must run headless, never gain window focus,
    and the test gates with the existing `available()`-style skip pattern
    (see `rust-testing`) so a host without it reports a governed skip rather
    than failing.
  - Do not assert or document full Universal Ctags compatibility.
- [x] **ctags option tests**: explicit `--signatures`/`--comments` rejected,
  default signature mode accepted, `--tree` rejected, global `--plain` accepted
  and byte-identical output.

**Checkpoint 5:** ctags tests and the reader fixture pass on macOS (local);
the unsorted header and duplicate entries are covered; implementation log
records the accepted choice.

## Phase 6: Goldens, Validation, Documentation

### Wave 1 (parallel)

- [x] **Goldens**
  - One golden per format (`tsv`, `markdown`, `prose`, `ctags`) over the
    existing 16-language `multi/` fixture, using the established scratch-copy
    approach in `cli/tests/symbols/golden.rs`, normalizing only changing root
    paths and IDs.
  - Assert **before normalization** that bytes contain no ESC (`\x1b`) or OSC 8
    sequence and no CR.
  - Review each golden by eye and note the review in the implementation log.
- [x] **Cross-cutting checks**
  - [x] Worker-count, warm/cold-cache and narrow/wide-terminal byte stability for
    all four formats (Markdown covered in Phase 4; extend to the rest).
  - [x] Work-counter test: choosing an export format adds no reference, diagnostic,
    semantic or metrics work beyond the equivalent listing, and containment is
    requested only for outline layouts or recorded ownership (ctags and
    classes). Reuse the existing counters; add no new performance spike.
- [x] **Shared-terminal L2 check**: only if shared terminal behavior changed in
  Phase 4, run `just test-l2` per `rust-testing` with no window focus. The
  `render.rs` extraction is expected to change no terminal bytes, so this is
  conditional.

### Wave 2 (parallel, after Wave 1)

- [x] **Docs**
  - `tree-hugger/cli/README.md`, `tree-hugger/docs/symbol-reports.md` (field and
    escaping rules, option table, literal rendering, class export limits, local
    Prose links, accepted tags-reader subset, kind mapping), `--help` text,
    completions, and `.claude/skills/tree-hugger/SKILL.md`.
  - Docs follow the repo rule: written for a developer new to the repo, a compact
    example per rule, a Mermaid diagram for the format/mode decision flow;
    never mention this feature or its spec by name.
  - `tree-hugger/docs/dependencies.md` if the Phase 1 ruling added a crate.
  - Note: no record/index schema or cache-revision change; do not bump either.
- [x] **Comment drift pass**: review changed renderer/helper comments for claims
  that formats or outlines are terminal-only; fix per the code-comment rule and
  state the fixes in the implementation log.
- [x] **Implementation log**: record the accepted tags choice, reader, spike
  results, any departure from the spec, and the comment-drift findings.

### Final validation

- [x] `just test` and `just lint` in `tree-hugger/` pass.
- [x] macOS and Linux evidence per the `os` skill; native Windows and WSL2
  verified on the available local rigs at the author's request (no new CI
  cells). Check Windows-path URL encoding and
  `/` separators in TSV/ctags with unit tests that run on every host.
- [x] Update spec status/`implemented` fields only as the author's process
  requires; **do not** move the directory to `_completed` and do not run
  `just complete`. Terminal state: implementation complete, ready for review.
  Phase 6 verification does not close the outstanding Phase 4 work; the
  overall spec status stays unchanged until that prerequisite is complete.
