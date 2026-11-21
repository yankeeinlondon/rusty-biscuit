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
message_to_agent: |-
  Phase 1 is complete in biscuit-file. `biscuit_file::{Indentation,
  IndentStyle}` are crate-root exports with no feature gate:
  `Indentation { style, width: NonZeroU8 }`, `Default` = 4 spaces, plus
  `Indentation::new`/`spaces`/`DEFAULT_WIDTH`. `IndentStyle` already
  serializes as `space`/`tab` (serde), so Sniff can re-export it directly.
  `Indentation` has no serde derive. `to_yaml_string` and
  `Yaml::to_yaml_string` need the `yaml` feature. Errors are
  `YamlError::Serialize` and `YamlError::Layout`. The writer does NOT use
  `rlsp-yaml-parser`: it cannot read some backend output. See the first
  `decisions_for_review` item. biscuit-file's Cargo.toml has no new
  dependency, so Phase 2 must not assume biscuit-file pulls in that parser.
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
