---
spec: "/home/build/coding/rusty-biscuit/claudine/features/2026-11-20-contextual-indents/spec.md"
plan: "claudine/features/2026-11-20-contextual-indents/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - biscuit-file/lib/src/indentation.rs
  - biscuit-file/lib/src/lib.rs
  - biscuit-file/lib/src/yaml/mod.rs
  - biscuit-file/lib/src/yaml/types.rs
  - biscuit-file/lib/src/yaml/write.rs
  - biscuit-file/lib/tests/l1/main.rs
  - biscuit-file/lib/tests/l1/yaml_writing.rs
  - biscuit-file/justfile
docs_updated_during_phase_1:
  - biscuit-file/README.md
  - biscuit-file/lib/README.md
  - biscuit-file/docs/design/yaml-design.md
docs_created_during_phase_1:
  - biscuit-file/docs/topics/yaml-writing.md
skills_files_updated_during_phase_1:
  - .claude/skills/biscuit-file/SKILL.md
  - .claude/skills/biscuit-file/references/api.md
  - .claude/skills/biscuit-file/references/architecture.md
packages:
  - biscuit-file
  - sniff
  - sniff-detection-cli
decisions_for_review:
  - |-
    The YAML writer does not use the span parser (rlsp-yaml-parser) the plan
    named. The plan said to read the backend's YAML text back as parse events
    with source spans and copy each scalar token from those spans. In testing,
    that parser could not read some valid YAML the backend writes: an
    explicit "?" key holding a mapping, after the first entry
    ("a: 1\n? k: v\n: map\n" fails with "mapping key is more indented"). It
    also reads U+2028 inside a block scalar as content, where the backend
    reads it as a line break. Instead, the writer parses the backend's text
    with the backend itself (it needs that parse anyway for the final check)
    and walks that value. It gets each scalar's spelling (quoting, block
    style, chomping) by having the backend serialize that scalar alone. The
    backend picks a scalar's style from its content only, so this is the
    spelling it uses inside a document. The caller's value is still
    serialized once, and no dependency was added. Alternatives: keep the span
    parser and return a layout error for mapping-valued keys (they are rare
    and never appear in config files, but the corpus round trip would lose
    cases), or patch the parser upstream. Redoing this means replacing
    `node`/`spell` in `biscuit-file/lib/src/yaml/write.rs`. The renderer,
    tests and docs would stay as they are.
  - |-
    Under a sequence dash, a literal block scalar's content goes one
    requested level past the dash (`- |` at column 4, content at column 8
    for width 4). A tagged collection inside a sequence item puts its
    children two columns past the dash, matching the compact rule. The plan
    said "re-indented to the requested level" without naming the column the
    level counts from. The alternative is to count block-scalar content from
    the compact column (dash + 2), so all item content aligns. Redoing it
    means changing `compact_child` and `literal_block` in `write.rs`, plus
    the expected strings in `tests/l1/yaml_writing.rs`.
  - |-
    `Indentation` gained small constructors beyond the spec's bare struct:
    `Indentation::new(style, width)`, `Indentation::spaces(width)`, and the
    `Indentation::DEFAULT_WIDTH` constant (4). `IndentStyle` derives serde
    as `space`/`tab`, the JSON spelling the spec gives Sniff, so Sniff can
    re-export it without a wrapper. `Indentation` itself has no serde
    derive, because Sniff's JSON reports `indent_using`/`indent` fields
    rather than the struct. Removing or renaming these is a mechanical edit
    in `biscuit-file/lib/src/indentation.rs` and its callers.
  - |-
    EditorConfig section globs are matched by a small hand-written matcher in
    Sniff, not by globset as the plan said. globset cannot nest `{a,{b,c}}`
    alternatives, which EditorConfig allows, and a numeric range such as
    `{1..10}` needs the matched digits to check its bounds. The matcher
    backtracks over a parsed pattern, treats a range as a test on the whole
    digit run (never an expanded list), and matches a pattern without `/` as
    if it were written `**/pattern`, which is EditorConfig's rule.
    Alternatives: expand braces and ranges into many globset patterns (fails
    on large ranges), or use the `ec4rs` crate already in Sniff's manifest
    (it matches OS paths and cannot report a bad pattern). Redoing this means
    replacing `Glob` in `sniff/lib/src/filesystem/indentation/editorconfig.rs`;
    its unit tests describe the expected matches.
  - |-
    Three "declares but gives no opinion" cases select the file and so shadow
    the row's later files: a string `prettier` key in `package.json`/
    `package.yaml` (a shared preset), a `.clang-format` whose documents are
    all for other languages, and a disabled Biome formatter (the spec rules
    this one). The spec says only that a preset "gives no opinion on its own".
    The alternative is to skip the file and try the row's next candidate,
    which could report settings the formatter itself would never read.
    Changing it is one match arm each in `formatters.rs` (`declared_mapping`
    and `clang_format` return `Outcome::NoOpinion`; `Outcome::Skipped` would
    continue), plus the two tests that assert the shadowing.
  - |-
    The `make` convention reports tabs at width 8; the spec's table says only
    "Tabs". 8 is the width a tab displays at by default and matches gofmt's
    stated width. Alternatives: 4, or leaving the width to the fallback (4),
    which would make the convention opinion state only a style. Changing it
    is one number in `catalog.rs` and the conventions test.
  - |-
    An empty YAML configuration file, or one holding only comments or `---`,
    counts as an empty mapping: it declares the formatter with its defaults.
    Strictly it parses as null, and null is otherwise invalid. A literal `~`
    or `null` document is still invalid. The alternative is to report an
    empty file as a wrong-type declaration and try the next candidate.
    Changing it is `parse_yaml` in `inputs.rs` and one test assertion.
  - |-
    In an extension lookup, EditorConfig sections are matched against
    `file.<ext>` with the extension as the caller wrote it (case-sensitive,
    like all EditorConfig matching), while formatter rows and conventions
    use the lowercased extension. So `YAML` selects the YAML formatters but
    not a `[*.yaml]` section. The spec says case is normalized "for file-type
    selection". The alternative is to lowercase for EditorConfig too. Changing
    it is `Target::extension` in `mod.rs` and one test.
  - |-
    API details beyond the spec's listing:
    - `resolve_for_extension(base, ext)` discovers the worktree from `base`
      (the spec names the parameter `repo_root`), so the CLI's `--base` and a
      subdirectory both work;
    - `normalize_extension` is public, so the Phase 3 CLI validates with the
      same grammar;
    - `RepoIndentation::root()` returns the canonical root;
    - two new error variants, `SniffError::IndentationTarget` and
      `SniffError::InvalidExtension`;
    - diagnostic keys are dotted key paths with list indices
      (`overrides[0].files`). An EditorConfig key is the property name, with
      its line and section in the reason, and a bad section pattern's key is
      `[pattern]`;
    - a destination's directories are canonicalized but its file name is
      not, so a symlink at the destination is matched by its own name.
    Renaming or removing any of these is mechanical within
    `filesystem/indentation/` and its tests.
  - |-
    Plain `sniff repo indent` output names sources this way (the spec gives
    only two examples): an EditorConfig source as `.editorconfig [*], [*.rs]`;
    a formatter as its file plus the keys it read (`rustfmt.toml tab_spaces`),
    or `biome.json defaults` when the file sets nothing and the formatter's
    defaults apply; a convention as `gofmt convention`; the fallback as
    `default`. Tabs print as `tabs (width 8)` so the width stays visible. When
    style and width come from different sources, the line reads
    `2 spaces  (style: .editorconfig [*]; width: rustfmt.toml tab_spaces)`.
    YAML's tab adjustment adds `note: tabs requested; using spaces because
    YAML does not allow tabs for indentation`. Unusable configuration is listed
    under `skipped configuration:`. The spec's own example drops the key
    (`also: rustfmt.toml  4 spaces`). Alternatives: drop the keys, or put
    them in brackets. Changing it means editing `label`/`sources` in
    `sniff/cli/src/output/filesystem/indentation.rs` and the expected strings
    in `sniff/cli/tests/l1/repo_indent.rs` and `sniff/docs/cli/repo_indent.md`.
  - |-
    In summary mode (`sniff repo indent` with no extension), a row whose
    sources disagree ends with `sources disagree`. The spec shows only
    `default  4 spaces  (.editorconfig [*])` and does not say whether the
    summary shows consensus. Without the marker, the summary would hide a
    disagreement that the single-extension output reports. Removing it is one
    `if` in `render_summary`; no test asserts it.
  - |-
    Bare `sniff repo --json` builds its `indent` summary from the worktree root
    that the aggregate's detection already found
    (`RepoIndentation::load(root).summary()`). It calls `resolve_summary(base)`,
    the leaf's function, only when no repository was found. The plan said to
    "use the same summary function as the leaf". Calling `resolve_summary`
    directly would discover the repository a second time and break the
    existing one-discovery rule for this command (a test enforces it). Both
    paths run the same library code, and a test checks that the parent's
    `indent` equals the leaf's output. The alternative is to accept a second
    discovery and change that test's expected count. Redoing it means
    replacing the `match observed_root` in `sniff/cli/src/commands/mod.rs`.
source_files_during_phase_2:
  - sniff/lib/Cargo.toml
  - sniff/lib/src/error.rs
  - sniff/lib/src/filesystem/formatting.rs
  - sniff/lib/src/filesystem/mod.rs
  - sniff/lib/src/filesystem/indentation/mod.rs
  - sniff/lib/src/filesystem/indentation/catalog.rs
  - sniff/lib/src/filesystem/indentation/editorconfig.rs
  - sniff/lib/src/filesystem/indentation/formatters.rs
  - sniff/lib/src/filesystem/indentation/inputs.rs
  - sniff/lib/src/filesystem/indentation/tests.rs
  - sniff/lib/src/performance/counters.rs
  - sniff/lib/tests/l1/main.rs
  - sniff/lib/tests/l1/indentation.rs
docs_updated_during_phase_2:
  - sniff/README.md
  - sniff/lib/README.md
docs_created_during_phase_2:
  - sniff/docs/topics/indentation.md
skills_files_updated_during_phase_2:
  - .claude/skills/sniff/SKILL.md
source_files_during_phase_3:
  - sniff/cli/src/args/mod.rs
  - sniff/cli/src/args/repo.rs
  - sniff/cli/src/commands/mod.rs
  - sniff/cli/src/output/mod.rs
  - sniff/cli/src/output/filesystem/mod.rs
  - sniff/cli/src/output/filesystem/indentation.rs
  - sniff/cli/tests/l1/main.rs
  - sniff/cli/tests/l1/repo_indent.rs
  - sniff/cli/tests/l1/snapshots.rs
  - sniff/cli/tests/l1/snapshots/l1__snapshots__repo_aggregate_json.snap
docs_updated_during_phase_3:
  - sniff/README.md
  - sniff/cli/README.md
  - sniff/docs/cli/repo.md
  - sniff/docs/topics/json-output.md
  - sniff/docs/topics/indentation.md
docs_created_during_phase_3:
  - sniff/docs/cli/repo_indent.md
skills_files_updated_during_phase_3:
  - .claude/skills/sniff/SKILL.md
message_to_agent: |-
  Phase 3 added `sniff repo indent` and the `indent` key in `sniff repo --json`.
  It changed only the Sniff CLI; the library API from Phase 2 is unchanged.
  For Phase 4 writers: `sniff::filesystem::indentation::resolve_for_path(dest)`
  takes a destination that need not exist (it finds the worktree from the
  nearest existing ancestor), and `ResolvedIndentation::indentation()` gives
  the `biscuit_file::Indentation` to pass to the writer. Configuration
  problems are never errors; they are in `skipped`. Only a failure to
  identify the destination or worktree is an `Err`, and the plan says to fall
  back to the default with a path-qualified warning then. Use
  `RepoIndentation::load(root)` when one save touches many files. Phase 1
  and 2 edits are still uncommitted in the working tree, so the scope
  calculator reports sniff, biscuit-file, and their narrowed dependent cells
  even for a phase that changes neither. Sniff has no caller skill
  (`sniff/docs/use-skills/` does not exist).
---

# Implementation Log for 2026-11-20-contextual-indents (4 phases)

## Phase 1

- Started 2026-11-20. Phase 1 is confined to the `biscuit-file` package area.
- Probed `rlsp-yaml-parser` 0.11.1 against `serde_yaml_ng` 0.10 output before
  writing the layout engine: scalar spans cover the whole token (quotes and
  block header included), collection start events are zero-width, tags carry
  a source `tag_loc` span, and the backend emitter runs with width `-1`, so it
  never folds a flow scalar across lines.
- Added `Indentation`/`IndentStyle` (`biscuit-file/lib/src/indentation.rs`,
  unfeatured, crate-root exports). Added `to_yaml_string` and
  `Yaml::to_yaml_string` behind `yaml` (`yaml/write.rs`), and
  `YamlError::{Serialize, Layout}`. `Serialize` has no `From`: the existing
  `From<serde_yaml_ng::Error>` stays `Parse`, and the writer maps backend
  failures to `Serialize` explicitly.
- **Departure from the plan (span parser dropped).** The plan said to use
  `rlsp_yaml_parser::parse_events` on the backend's output. Probing showed
  two disagreements with the backend. First, an explicit key holding a
  mapping, after the first entry, does not parse: `"a: 1\n? k: v\n: map\n"`
  fails with "mapping key is more indented than the enclosing mapping".
  Second, U+2028 inside a block scalar is content to rlsp and a line break
  to libyaml. The writer now walks the backend's own reparsed value and
  spells each scalar by serializing it alone through the backend. No
  dependency was added, and `biscuit-file/lib/Cargo.toml` is unchanged.
  Recorded in `decisions_for_review`.
- Layout rules implemented and documented in
  `biscuit-file/docs/topics/yaml-writing.md`:
    - block collections go one level under their key;
    - compact `- key:` items and nested `- -` items put continuation lines
      two columns past the dash, at any width;
    - empty collections keep `[]`/`{}`, and tags are kept;
    - collection keys and keys over 1024 characters use the explicit
      `?`/`:` form, and multiline keys are double-quoted;
    - literal blocks keep their `|`/`|-`/`|+` form, with content one level
      past the key or dash;
    - the double-quoted fallback, with JSON-compatible escapes, applies
      when an indicator would exceed 9, at a root that needs an indicator,
      for content of only line breaks, and for CR/BOM/NEL/LS/PS or
      non-printable characters.
- The final check reparses the output and compares it recursively with the
  parsed backend output: values, tags, non-string keys, mapping order,
  and NaN equal to NaN.
- Requirement → test map. Boundary: the crate's public API. F1, in-process
  L1 tests in the consolidated `l1` binary
  (`biscuit-file/lib/tests/l1/yaml_writing.rs`, declared in `main.rs`).
    - spec example (4 spaces, lists under key, compact items) →
      `the_specification_example_indents_lists_one_level_under_their_key`
    - default 4 spaces, tabs→spaces →
      `the_default_is_four_spaces_and_tabs_become_spaces`
    - widths 1, 2, 4, 8, 10, 255: nested maps and scalar lists →
      `nested_maps_and_scalar_lists_advance_one_level_per_key_at_every_width`
    - compact map/list items and lists of lists at every width →
      `compact_items_align_with_the_first_key_two_columns_past_the_dash_at_every_width`,
      `a_root_sequence_starts_at_column_zero`
    - empty collections → `empty_collections_keep_their_flow_spelling`
    - tags (serde enums, tagged maps/sequences/scalars) →
      `tags_are_retained_on_scalars_and_collections`
    - non-string and complex keys →
      `non_string_and_complex_keys_keep_their_type_and_use_explicit_entries`,
      `multiline_keys_are_double_quoted_and_long_keys_stay_explicit`
    - non-finite numbers →
      `non_finite_numbers_survive_the_equivalence_check`
    - quoted scalars keep spelling and type →
      `quoted_scalars_keep_the_backend_spelling_and_their_type`
    - Unicode → `unicode_is_written_as_itself`
    - block scalars stay block scalars, with chomping, blank lines and
      YAML-looking content, at every width →
      `block_scalars_stay_block_scalars_with_their_chomping_at_every_width`,
      `block_scalars_in_sequences_and_compact_mappings_indent_from_their_indicator`
    - explicit indicators up to 9 →
      `a_leading_space_takes_an_indentation_indicator_up_to_nine`
    - each double-quoted fallback trigger, and determinism →
      `each_double_quoted_fallback_trigger_spells_the_content_escaped`
    - fixed point (repeated write) →
      `rewriting_the_output_is_a_fixed_point`
    - `Yaml` wrapper, and source text unchanged →
      `the_yaml_wrapper_writes_its_value_and_keeps_its_source_text`
    - typed serialize error (negative path) →
      `a_backend_serialization_failure_is_a_typed_serialize_error`
    - corpus round trip at every width, with value, order and layout
      asserted separately →
      `the_yaml_corpus_round_trips_through_the_wrapper_at_every_width`
      (reads `tests/corpus/yaml_corpus.json` through `include_str!` inside
      the test)
    - mapping insertion order → `mapping_order_is_the_insertion_order`
    - private seams (unit): `yaml::write::tests::*` (NaN and order in the
      equivalence check, tags and key types, the escape table)
    - feature-disabled type exports:
      `indentation::tests::the_default_is_four_spaces_and_styles_serialize_as_space_and_tab`,
      now also run by `just test-minimal` under `--no-default-features`.
      That recipe's filter changed from `path_text` to
      `-- path_text indentation`.
- `YamlError::Layout` cannot be reached through the public API for any
  value tested, including the whole corpus. It is the writer's guard
  against its own defects, so no public test triggers it.
- The Input Robustness Matrix does not apply: this phase adds a writer,
  not a configuration reader.
- No dependency was added or removed, so `docs/dependencies.md` and
  `biscuit-file/docs/dependencies.md` need no change. biscuit-file has no
  caller skill (`docs/use-skills/`), so nothing was published.
- Gates, in the biscuit-file area, host load average about 0.4:
    - `just lint`: clean (default, `file-reference-syntax,yaml`-only, and
      `delimited` builds, plus the CLI);
    - `just test`: 1318 passed, and the `--no-default-features` checks (10)
      and `reference_syntax_only` (6) passed. "🐢 L1 run recorded: 1318
      test(s), none at or above 5.0s";
    - `just check-tier-coverage biscuit-file`: 0 stranded;
    - `cargo test -p biscuit-file --doc`: 38 passed.
- Dependent packages: `cargo check --all-targets` over all 21 direct
  dependents of biscuit-file succeeded with no warnings (2m27s). The
  dependents are claudine, claudine-cli, claudine-gen, darkmatter (lib, CLI,
  LSP), sniff, sniff-detection-cli, simplified-schema, schematic-define,
  schematic-gen, and the others. No dependent uses the new symbols, and
  nothing matches exhaustively on `YamlError` or glob-imports
  `biscuit_file::*`. Compiling them was the proportionate downstream test,
  so no dependent's full suite was run for its own sake.
- Gate-skip check: `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gives
  `packages: ["biscuit-file", "test-toolkit", "darkmatter"]` and
  `change_class: "package"`, so the gates were not skipped. darkmatter is
  selected because three of its tests read every shipped Markdown and skill
  file. test-toolkit is selected for its archive-path guard.
- `just ci-local` (25m47s, load average about 1.3) ran every planned cell
  this host can run. All 16 gates passed:
    - the 11 CI-infrastructure Python suites, selected because
      `biscuit-file/justfile` changed;
    - biscuit-file lint and L1;
    - the test-toolkit archive-path guard;
    - darkmatter L1: 12605 passed, none at or above 5 s;
    - the narrowed darkmatter test-input cell: 3 passed
      (`schema_roots::no_doc_or_skill_names_the_singular_schema_variable`
      and the two `shipped_schema_corpus` tests).
- The claudine area's own `just test` was not run. This phase changed no
  claudine source, and the CI scope selects no claudine cell. claudine and
  claudine-cli compiled against the new biscuit-file in the dependent
  check above.
- No local OS test rigs are configured (`BUILD_LINUX`, `BUILD_WIN`,
  `BUILD_WSL`, and `BUILD_MACOS` are unset). macOS, native Windows, and WSL2
  evidence is left to CI. The writer is pure string processing, with no
  paths and no OS-specific code.

## Phase 2

- Started 2026-11-20. Phase 2 is confined to the `sniff` library
  (`sniff/lib`). No CLI change: `sniff repo indent` is Phase 3 and is marked
  **planned** in the new topic page.
- Host: Linux, load average 0.3–1.3 throughout; 32 cores. No local OS test
  rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, and
  `BUILD_MACOS` are unset), so macOS, native Windows, and WSL2 evidence is left
  to CI. The OS-sensitive code is path handling: relative paths are built from
  path components and joined with `/`, worktree containment uses
  `biscuit_file::{canonicalize_simplified, PathIdentity}`, and one test builds
  its path with the host separator to prove `/` matching.
- I wrote all of the work myself rather than in parallel subagents. The
  three "parallel" tasks share the types and diagnostics, and doing them in
  one context kept those consistent.

### What was built

- `sniff::filesystem::indentation` (`sniff/lib/src/filesystem/indentation/`):
    - `mod.rs`: the public types the specification lists (`IndentAuthority`,
      `IndentOpinion`, `ResolvedIndentation`, `IndentAdjustment`,
      `AdjustmentReason`, `IndentationDiagnostic`, `IndentationSummary`), the
      re-exported `biscuit_file::{Indentation, IndentStyle}`,
      `normalize_extension`, the free functions (`resolve_for_path`,
      `resolve_for_extension`, `resolve_summary`), and `RepoIndentation`
      (`load`, `root`, `resolve_for_path`, `resolve_for_extension`,
      `summary`). Every entry point ends in the one `resolve` function: style
      and width are filled independently, consensus is computed on the raw
      opinions, and the YAML adjustment is applied last. Paths serialize with
      `/` through a serde helper.
    - `catalog.rs`: the 15 formatter rows in rank order (candidates, the
      dialect of each, declaration keys, executable configs, extensions,
      defaults) and the 10 conventions.
    - `editorconfig.rs`: the EditorConfig glob matcher and the
      `indent_style`/`indent_size`/`tab_width` fold.
    - `formatters.rs`: candidate selection (first readable, parseable,
      declaring file; shadowing; disabled and no-opinion outcomes) and one
      reader per row, built on shared validators (`Reader::{width, boolean,
      style, globs, mapping}`).
    - `inputs.rs`: the loader. It probes, reads, and parses each distinct path
      once, in parallel (`rayon` `par_iter` with `performance::pooled_worker`),
      into a type-preserving `Node`. Results are keyed by path, so completion
      order never shows.
- `filesystem/formatting.rs`: `parse_editorconfig` is now one ordered scan
  (`scan_editorconfig`: sections and assignments in file order, with lines)
  and two projections. The formatting report projects exactly as before (all
  15 existing `formatting::tests` pass unchanged). The indentation module
  folds the same scan.
- `performance/counters.rs`: `INDENT_CONFIG_PARSES`
  (`filesystem.indentation.config_parses`).
- `error.rs`: `SniffError::IndentationTarget { path, reason }` and
  `SniffError::InvalidExtension { input, reason }`.
- `sniff/lib/Cargo.toml`: biscuit-file's `json5` feature is now named
  explicitly. It was already on through biscuit-file's defaults, so the
  dependency graph is unchanged and `docs/dependencies.md` needs no edit.

### Departures from the plan

- **EditorConfig globs use a hand-written matcher, not `globset`.** globset
  cannot nest `{…}` alternatives, which EditorConfig allows, and a numeric
  range needs the matched digits to test against its bounds. The matcher in
  `editorconfig.rs` backtracks over a small node list. A range is a predicate
  over the whole digit run, never an expanded list (`v{0..200000}` is tested).
  A glob without `/` matches as `**/glob` would, which is EditorConfig's rule.
  Matching against the basename alone was wrong for `a**z`; the unit test
  caught it. Prettier/oxfmt override globs do use `globset`, with literal
  separators and backslash escapes, as the plan says. Recorded in
  `decisions_for_review`.
- **Plan wording "duplicate keys … through the EditorConfig scanner's own
  grammar where duplicate assignments deliberately merge":** a key repeated
  within a section takes the later value, and a line without `=`/`:` is
  ignored, as before. Both are tested.

### Requirement → test map

Boundary: the crate's public API (F1, in-process) for behavior, plus
crate-private work counters for the read-once contract. The tests are L1:
`sniff::l1` binary module `indentation` (declared in `tests/l1/main.rs`) and
the lib's unit tests. There is no CLI or terminal boundary in this phase.

- Spec success criteria (with this repository's `.editorconfig` read through
  `include_str!("../../../../.editorconfig")` inside the test):
  `this_repositorys_editorconfig_gives_yaml_four_spaces_from_editorconfig`,
  `a_rustfmt_tab_spaces_of_two_gives_rust_two_spaces_and_yaml_the_default`,
  `a_bare_biome_json_gives_tabs_and_editorconfig_outranks_it_without_consensus`
  (also asserts the spec's JSON shape exactly and a serde round trip).
- EditorConfig: style-only/size-only partial opinions, `unset`,
  `indent_size = tab`, tab style with `tab_width`, numeric size against a
  different tab width, invalid values (located, earlier value kept), invalid
  section patterns, repeated assignments, path-scoped and filename sections
  against extension lookups (the synthetic `file.<ext>` exclusion), forward
  slashes, case sensitivity. Glob grammar in
  `editorconfig::tests::*` (basename at any depth, anchoring, `/**/`, `*`/`?`
  never crossing `/`, nested braces, `{single}`, signed ranges, classes,
  escapes, malformed patterns).
- Each formatter row: `row!` control from that formatter's documented example
  plus a defaults fixture stating no indentation (`*_control`,
  `*_defaults`), 30 tests, with a guard test that names every authority.
- Conventions: `each_convention_applies_when_no_formatter_speaks`, exact
  basenames and Terraform suffixes in path lookup only, the Make
  distinction (no forced tabs, no adjustment), and convention fallback after
  a manifest with no key and after a malformed file.
- Ranking, filling, consensus (agree, disagree, partial), and the YAML
  constraint (adjustment, consensus before adjustment, JSON spelling,
  `indentation()` value).
- Candidate selection: shadowing, `package.json` before `package.yaml`, a
  manifest without its key, a string Prettier preset, wrong-type declarations
  (`prettier: 42`, `[tool] ruff = 3`), executable configs beside a selected
  file, Biome language-over-global enablement and disabled shadowing, Biome
  and dprint language blocks, unsupported inheritance and path overrides
  (Biome, dprint, CSharpier, Taplo, Tombi, RuboCop), Deno legacy
  `fmt.options`, and the value mappings (Tombi, Taplo, swift-format,
  clang-format modes, presets, language documents).
- Prettier/oxfmt overrides: file order, later value wins per property,
  `excludeFiles`, and extension-mode deferral of path-dependent entries.

### Input Robustness Matrix

Default outcome (every load-bearing setting): an invalid shape is a
`skipped` diagnostic naming the file and the dotted key, that setting is not
applied, and the property falls through to the lower layer, then the row
default. Exceptions:

- absent → the lower layer, then the row default (no diagnostic);
- explicit `null` (JSON `null`, YAML `k:`) → invalid, never treated as absent;
- an empty declaring object (`{}`, an empty or comment-only YAML file) →
  selects the row with its defaults;
- an empty `overrides` list → no overrides; an empty glob string or empty glob
  list → invalid; one bad element invalidates the whole glob field; an invalid
  override entry leaves the other entries usable;
- a malformed document, or a declaration of the wrong type → the candidate is
  skipped with a diagnostic and the row tries its next candidate;
- a string `prettier` key → an unsupported-preset diagnostic, no opinion, and
  it still shadows;
- EditorConfig `unset` → a valid clear.

Field shapes, once per shared validator, through the public result:

- width: `the_width_validator_accepts_exact_integers_from_1_through_255`
  (1 and 255 controls; absent; `null`, `"4"`, `4.0`, `4.5`, `0`, `256`, `-1`,
  `true`, `[4]`, `{}`);
- boolean: `the_boolean_validator_accepts_only_booleans`;
- style words: `the_style_validator_accepts_only_the_formatters_exact_spellings`;
- nested mapping: `a_nested_settings_block_of_the_wrong_type_is_diagnosed_and_defaults_apply`;
- glob field: `the_glob_field_validator_takes_a_glob_or_a_non_empty_list_and_one_bad_element_spoils_it`
  (`null`, `123`, `""`, `[]`, `["*.ts", 1]`, `[1]`, `[""]`, an invalid glob,
  `{}`);
- override entries and the `overrides` field: `override_entries_are_validated_one_by_one`;
- EditorConfig widths (`exact_width_*` unit test, and the public invalid-value
  test).

Wiring: 44 `wiring_*` tests, one per key Sniff reads in every row and every
declaring manifest (`package.json`/`package.yaml`'s `prettier.`,
`pyproject.toml`'s `tool.ruff.`/`tool.tombi.`). Each has a control and one
invalid value, and asserts the diagnostic key and the default fallback.

Document shapes, once per loader, through the public result (a rejected file
visibly falls through to the next candidate):

- strict JSON: duplicate key (located `line 2 column 2`), trailing content,
  comments, trailing comma;
- JSON5: comments and trailing commas accepted, duplicate key and trailing
  content rejected;
- YAML: implicit null, duplicate key, a second document, unclosed flow, and an
  empty file;
- TOML: duplicate key and trailing content;
- the clang-format YAML stream: multi-document selection, a duplicate key, a
  bad document, and a stream with no mapping document;
- extensionless files: JSON when the file opens with `{`/`[`, YAML otherwise;
- EditorConfig: repeated keys merge, lines without an assignment are ignored;
- unreadable candidates (a directory in place of a file).

The same shapes are unit-tested at the parser in `inputs::tests::*`.
Duplicate JSON/JSON5 keys are rejected before any `serde_json::Value` is
built: the grammar is checked first, then `biscuit_file::json5_scan` runs.

Smell audit (`serde(default)`, `Option<T>`, `filter_map`, `.ok()`,
`unwrap_or_default` in the new code):

- `serde(default)` appears only on the output types, so they deserialize
  their own JSON;
- `.ok()` appears only after digit validation (overflow is diagnosed by the
  caller);
- `unwrap_or_default` on swift-format's `indentation` leads to the
  "exactly one of" diagnostic;
- `filter_map` drops non-string YAML keys, which no setting uses;
- no load-bearing value is filtered or discarded silently.

Mutation check: removing the duplicate-key rejection, or the
extension-lookup section filter, failed 3 tests. Both mutations were reverted.

### Repository behavior and read-once

- One read and parse per shared file, and focused reads: unit tests in
  `filesystem/indentation/tests.rs` under `performance::testing::measure`:
    - a snapshot with 5 files records 5 opens and 5 parses, and later queries
      and the summary record 0;
    - `package.json`, `deno.json`, and `pyproject.toml` feed several rows from
      one parse;
    - an `rs` lookup opens only `.editorconfig` and `rustfmt.toml` (3 probes);
    - a generic lookup opens only `.editorconfig`.
- Public tests: linked worktrees, nested repositories (found by the free
  functions, not by a snapshot), new paths through missing directories with
  `.`/`..`, outside-worktree fallback (conventions still apply,
  `.editorconfig` ignored), snapshot refusals (relative, outside, missing
  root), destination errors, repeated-lookup determinism, `/` in serialized
  paths, and the extension grammar.
- Summary: `the_summary_lists_only_extensions_that_differ_from_the_generic_default`
  covers sorted overrides, the EditorConfig literal suffixes (`[*.custom]`
  counts, `[*.t?ml]` does not), the snapshot and free function agreeing, and
  a diagnostic from extensions equal to the default appearing exactly once.
  `a_summary_outside_a_worktree_is_the_fallback_with_no_overrides`.

### Gates and downstream

- `just lint` (sniff): clean; `cargo clippy --tests` over sniff: clean.
- `just test` (sniff area): 3782 passed, 33 skipped. "🐢 L1 run recorded: 3782
  test(s), none at or above 5.0s". The new L1 module is 134 tests in about
  0.2 s.
- `just check-tier-coverage sniff`: 0 stranded.
- Doctests (`indentation`, `formatting`): 3 passed.
- Dependents: `cargo check --all-targets` over all 24 direct reverse
  dependencies of sniff succeeded with no warnings (2m32s). No dependent uses
  the new module or `detect_formatting` (the `formatting` names in Claudine
  and DMLS are their own modules). The one downstream `match` on
  `SniffError` (tree-hugger's `changed.rs`) has a wildcard arm. Compiling the
  dependents is the proportionate downstream test.
- Gate-skip check: `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gives
  `packages: ["sniff", "test-toolkit", "darkmatter", "simplified-schema"]` and
  `change_class: "package"`, so no gate was skipped. `just ci-local --plan`
  narrows the extra cells:
    - darkmatter: 3 tests that read docs and skills. I ran the
      `shipped_schema_corpus` and `schema_roots` filters (13 passed);
    - simplified-schema: `dependency_direction_guard` reads
      `sniff/lib/Cargo.toml`. 4 passed;
    - test-toolkit: the archive-path guard.
      Passed; its 9 violations are all allowlisted.
- `just test` (claudine area): 12651 passed. "🐢 L1 run recorded … none at or
  above 5.0s" (5m01s).
- `just lint` (claudine area): clean.
- `just slow-tests fresh`: `target/slow-tests/current.md` describes the
  working tree as it is now.

## Phase 3

- Started and finished 2026-11-20. Phase 3 is confined to the Sniff CLI
  (`sniff-detection-cli`); the library is unchanged. Host: Linux, 32 cores,
  load average under 3 throughout. No local OS test rigs are configured
  (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset), so macOS,
  native Windows, and WSL2 evidence is left to CI. The command adds no
  OS-specific code: paths in output come from the library's portable `/`
  spelling, and the renderer joins path components with `/`.
- I did the work myself, without subagents: two tasks in one package with a
  shared renderer.

### What was built

- `sniff repo indent [<FILE-EXT>]`: `RepoSubcommand::Indent` /
  `RepoAction::Indent` in `cli/src/args/{repo,mod}.rs`. Clap validates the
  extension with the library's `normalize_extension` (one grammar for both;
  a rejected value is exit 2 with the library's reason). It is dispatched in
  the early-return block of `commands/mod.rs` beside `state`, so it runs no
  detection plan. With an extension it calls `resolve_for_extension(base,
  ext)`, without one `resolve_summary(base)`. `--json` serializes the library
  value unchanged through `print_json_value`.
- Plain rendering is `cli/src/output/filesystem/indentation.rs`, built with
  biscuit-terminal `InlineProse` lines and an `UnorderedList` for the skipped
  notes. Each disagreeing alternative value is highlighted (bold yellow).
- `sniff repo --json` gains `indent`, the same summary. The plan said to attach
  it on the path feeding `repo_json.rs::build_with_outcome`, but bare
  `sniff repo --json` never reaches that builder: `commands/mod.rs`
  intercepts it and calls the pure projection `build_aggregate_value`. The
  summary is inserted next to that call, not inside it, so the projection
  stays pure. A first version called `resolve_summary(base)` there. It broke
  `cli::repo_aggregate_perf_covers_complete_command`, which pins one Git
  discovery per aggregate command. The aggregate now loads
  `RepoIndentation::load(git.repo_root).summary()` from the worktree root
  that detection already found, and calls `resolve_summary` only when
  detection found no repository. Focused commands are unchanged.
- The `l1__snapshots__repo_aggregate_json` snapshot now includes `indent`
  (deterministic: worktree-relative paths, sorted overrides). Its `source:`
  header also updated to the file's current path (`tests/l1/snapshots.rs`).

Sample output (scratch repository with `[*.{yaml,yml}] indent_style = space,
indent_size = 2` only):

```text
$ sniff repo indent
default  4 spaces        (default)
dart     2 spaces        (dart format convention)
ex       2 spaces        (mix format convention)
exs      2 spaces        (mix format convention)
gleam    2 spaces        (gleam format convention)
go       tabs (width 8)  (gofmt convention)
mk       tabs (width 8)  (make convention)
tf       2 spaces        (terraform fmt convention)
tfvars   2 spaces        (terraform fmt convention)
yaml     2 spaces        (.editorconfig [*.{yaml,yml}])
yml      2 spaces        (.editorconfig [*.{yaml,yml}])
```

In this repository:

```text
$ sniff repo indent yaml
4 spaces  (.editorconfig [*], [*.{js,ts,jsx,tsx,vue,json,yaml,yml,toml}])
consensus: yes
$ sniff repo indent rs
4 spaces  (.editorconfig [*], [*.rs])
also: rustfmt.toml tab_spaces  4 spaces
consensus: yes
```

### Requirement-to-test mapping

All CLI tests spawn the binary through `SniffCliFixture` against fixture
worktrees (F2, L1; boundary: the CLI process, filesystem, and Git
discovery). The robustness matrix itself stays in-process in
`sniff/lib/tests/l1/indentation.rs` (Phase 2), as the plan directs.

| Behavior | Test (`sniff/cli/tests/l1/repo_indent.rs` unless noted) |
|---|---|
| This repository's real `.editorconfig` (`include_str!`) gives YAML 4 spaces from `.editorconfig` | `this_repositorys_editorconfig_gives_yaml_four_spaces_from_editorconfig` |
| rustfmt `tab_spaces = 2` → rs 2 spaces from `rustfmt.toml`; `.yaml` (dot form) → default | `a_rustfmt_tab_spaces_of_two_reports_rust_from_rustfmt_and_yaml_from_the_default` |
| Bare `biome.json` → tabs; with the real `.editorconfig` → 4 spaces, Biome alternative, `consensus: false` (JSON and plain) | `a_bare_biome_json_gives_tabs_until_the_editorconfig_outranks_it_without_consensus` |
| Partial opinion: separate style and width sources named; unstated value omitted from JSON | `a_style_only_section_names_a_separate_width_source`; unit `output::filesystem::indentation::tests::a_partial_authority_names_the_source_of_each_value`, `a_value_stated_by_nobody_is_named_as_the_default` |
| YAML tab adjustment in JSON and the plain note | `yaml_reports_the_tab_adjustment_in_json_and_plain_output` |
| Disagreeing values highlighted, agreeing ones not | unit `only_the_disagreeing_part_of_an_alternative_is_highlighted` |
| Summary mode: default, differing overrides only, aligned plain rows | `the_summary_lists_the_default_and_each_differing_extension` |
| Parent `repo --json` embeds exactly the leaf summary under `indent` | `the_parent_repo_json_carries_the_summary_under_indent`; `snapshots::repo_aggregate_json_snapshot` |
| Focused JSON unaffected | `focused_repo_commands_keep_their_json_without_indent`; existing focused-command tests and snapshots |
| One Git discovery for the aggregate | existing `cli::repo_aggregate_perf_covers_complete_command` (caught the first version) |
| Invalid extensions are usage errors (exit 2, nothing on stdout): empty, `.`, `..yaml`, `-yaml`, `y/aml`, `*.yaml` | six `invalid_extension!` tests |
| Base-directory handling: `--base` from outside the worktree (every fixture test), and the launch directory in a subdirectory without `--base` | `the_launch_directory_inside_a_worktree_is_used_without_base` |
| Outside a worktree: convention (`gofmt`), default, config ignored, empty overrides | `outside_a_worktree_a_convention_or_the_default_applies` |
| Help and dynamic completion name the command | `help_and_completion_name_the_command` |
| CLI wiring per loader: control plus malformed file with a located diagnostic, a usable result, and the plain skipped list, for strict JSON, JSON5, YAML, TOML, EditorConfig | `strict_json_loader_is_wired`, `json5_loader_is_wired`, `yaml_loader_is_wired`, `toml_loader_is_wired`, `editorconfig_scanner_is_wired` |

Input robustness smells: the phase adds no reader of a configuration format.
The only new input is the CLI extension, validated by the library grammar.

### Gates and downstream

- `just test` (sniff area): 3807 passed, 33 skipped; "🐢 L1 run recorded:
  3807 test(s), none at or above 5.0s". The first run failed on
  `cli::repo_aggregate_perf_covers_complete_command` (two Git discoveries),
  fixed as described above. The 22 new tests take about 0.15 s together.
- `just lint` (sniff): clean. `just check-tier-coverage sniff`: 0 stranded.
- Gate-skip check: `git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py` gives
  `packages: ["sniff", "sniff-detection-cli", "test-toolkit", "darkmatter",
  "simplified-schema"]`, `change_class: "package"` (the uncommitted Phase 1
  and 2 edits are still in the tree), so no gate was skipped.
- `just test` (claudine area): 12651 passed, 15 skipped; none at or above
  5.0 s (1m46s). `just lint` (claudine area): clean.
- Dependents: no package depends on `sniff-detection-cli`. The narrowed
  darkmatter cell (tests that read docs and skills) reran:
  `shipped_schema_corpus` and `schema_roots`, 36 passed. The
  simplified-schema and test-toolkit cells read only Phase 1 and 2 inputs,
  which Phase 3 did not touch.
- `sniff-detection-cli`'s tmux L2 tier was not run: it covers git-status and
  CI styling in a real terminal, and Phase 3 changes neither.
- `just slow-tests fresh`: fresh.

### Docs and skills

- Created `sniff/docs/cli/repo_indent.md` (arguments, plain output, summary,
  discovery, JSON keys, cost). I checked its examples against real runs.
- `sniff/docs/topics/indentation.md`: the "(planned)" command section now
  describes the real command and links to the CLI page.
- `sniff/docs/cli/repo.md`: new "Formatting" table and an `indent` bullet in
  the aggregate section. `sniff/docs/topics/json-output.md`: `indent` in the
  aggregate example. `sniff/README.md` and `sniff/cli/README.md`: one example
  line each.
- `.claude/skills/sniff/SKILL.md`: CLI wiring paragraph. Sniff has no caller
  skill (`sniff/docs/use-skills/` does not exist), so the plan's "update the
  caller skill" step had nothing to update and nothing to publish.
- The `claudine` skill needs no change: nothing in Claudine changed.
