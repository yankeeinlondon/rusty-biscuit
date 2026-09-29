---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/content-policy/features/2026-09-28-content-policy/spec.md
plan: content-policy/features/2026-09-28-content-policy/plan.md
implemented_by: claude/opus
started_phase: 1
source_files_during_phase_2:
    - content-policy/lib/src/lib.rs
    - content-policy/lib/src/model.rs
    - content-policy/lib/src/diagnostic.rs
    - content-policy/lib/src/grammar.rs
    - content-policy/lib/src/normalized.rs
    - content-policy/lib/src/time.rs
    - content-policy/lib/src/aggregate.rs
    - content-policy/lib/src/reader.rs
    - content-policy/lib/src/evaluate.rs
    - content-policy/lib/tests/evaluation.rs
    - content-policy/lib/tests/robustness_matrix.rs
    - content-policy/lib/tests/fixtures/stamped-note.md
docs_updated_during_phase_2:
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
    - Cargo.lock
    - content-policy/lib/Cargo.toml
    - content-policy/lib/src/lib.rs
    - content-policy/lib/src/evaluate.rs
    - content-policy/lib/src/renew.rs
    - content-policy/lib/tests/common/mod.rs
    - content-policy/lib/tests/evaluation.rs
    - content-policy/lib/tests/renewal.rs
docs_updated_during_phase_3:
    - content-policy/README.md
    - content-policy/docs/topics/policy-lifecycle.md
    - content-policy/docs/dependencies.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages:
    - content-policy
    - content-policy-cli
    - biscuit-file
    - darkmatter
    - darkmatter-cli
    - claudine-cli
---

# Implementation Log for 2026-09-28-content-policy (6 phases)

## Phase 1

### 1.1 Package scaffold

- Created `content-policy/lib` (crate `content-policy`, empty `lib.rs`) and
  `content-policy/cli` (crate `content-policy-cli`, `[[bin]] policy`, a clap
  stub), and added both to the root `Cargo.toml` `members`.
- Library dependencies match R12/AC 22: `serde`, `serde_json`, `chrono`
  (`std`, `clock`, `now`), `biscuit-hash` (`xx_hash`, `blake3`), and
  `biscuit-file` with `default-features = false, features = ["yaml"]`. The
  `file-adapter` feature enables `biscuit-file/file-reference`; the CLI enables
  it. `[package.metadata.ci.tests] all-features = true` on the library so CI's
  L1 covers the adapter.
- `content-policy/justfile` is modeled on `biscuit-hash/justfile` with the test
  scope `content-policy --all-features; content-policy-cli`. `test-l2`,
  `test-l3`, `test-browser`, `test-real`, `fuzz`, and `bench` are "not
  applicable" stubs.
- `just deps-check` (R16) runs `cargo tree -p content-policy -e normal,build`
  for the default features and `--all-features` and fails on any crate whose
  name contains `pdf` or starts with `darkmatter`. Dev-dependencies are
  excluded because they never ship. Verified the pattern is load-bearing:
  the same pipeline over `biscuit-file` (default features) reports `lopdf` and
  `pdf-extract`.
- **CI gap (for the author):** CI's lint job runs `just _lint <package>`
  (`.github/workflows/_package-ci.yml`, "Lint" step), not the area's
  `just lint`, so `deps-check` runs locally but not in CI. R16 chose a just
  recipe over a nextest test deliberately; closing the gap needs either a CI
  hook for area-specific lint extensions or a change of ruling. Recorded in
  the spec's `message_to_agent`; not widened here.
- Docs: added `content-policy/docs/dependencies.md`; root
  `docs/dependencies.md` gains a Recent note, two Structure lines, and two
  Workspace Packages entries.
- Checks: `cargo check -p content-policy -p content-policy-cli` passes;
  `just deps-check`, `just lint`, and `just test` (0 tests) pass in
  `content-policy/`; `sniff repo package-areas` lists `content-policy`;
  `just check-tier-coverage content-policy` reports nothing stranded;
  `just ci-local --plan` shows `lint` (ubuntu) and `L1` (ubuntu, windows,
  macos, wsl2) cells for both crates.

### 1.5 Document migration (subagent)

- Confirmed the set before editing: `git grep -n -E 'Duration\(|update_policy:' -- '*.md'`
  outside `content-policy/` found exactly the 23 documents plus two body-text
  hits that are not frontmatter (a Rust `Duration(chrono::Duration)` in
  `sniff/features/_completed/2026-09-15-recent-commits/implementation-log.md`
  and a mention in `sniff/fixes/_completed/2026-09-14-corrected-perf-flag/plan.md`).
  An `rg` pass including untracked files also found 23.
- Rewrote: 7 `.claude/skills/playa/audio-programming/*.md` and 7
  `sniff/docs/research/audio-programming/*.md` (`update_policy: Duration(6mo)` →
  `content_policy: - ValidFor(6mo)`); 6
  `biscuit-terminal/docs/research/terminal-multiplexing/*.md`
  (`ValidFor(3mo)`, `about.md` `ValidFor(12mo)`); 3
  `claudine/docs/research/acp/*.md` (`ValidFor(6mo)` twice; `json-rpc.md`
  `ValidFor(1yr) # pending: MajorVersion(latest_version)`).
- Only policy lines changed (40 added, 41 removed; `json-rpc.md` lost one item).
  Tab-indented files keep their other tab-indented lines. No CRLF or BOM in
  any of the 23; trailing bytes match `HEAD`. Five `update_policy: ` lines had a
  trailing space; the new key line has none.
- `md get --json5 <file> content_policy last_updated` parses all 23 and returns
  the expected rule and the unchanged `last_updated`.
- Post-check grep finds no frontmatter `Duration(` and no `update_policy:`
  outside specs and spikes.

### 1.2 Biscuit File locator fixes (subagent)

- **Zero-indent lists** (`scan.rs`): a private helper
  `SourceMap::indentless_sequence_key` finds an empty-valued key at the same
  indentation with only sibling `-` lines and deeper lines between; `parent_path`
  consults it for every `-` line. `content_policy:\n- X` is now
  `[Key("content_policy"), Index(0)]`. Works at any indentation. Because
  `block_value_context` and `key_occurrences` share `parent_path`,
  `locate_yaml_key` and duplicate-key analysis get the corrected paths too.
- **Multi-line scalars** (`locate.rs`, private `continues_past_line`):
  `locate_yaml_value` returns `None` when a quoted scalar or flow collection
  closes on a later line, or the next non-blank line is content indented past
  the entry's key (or `-`) column. A comment-only line is not a continuation.
- **Anchor/tag/alias**: new public `YamlValueProperties { anchor, tag, alias:
  Option<SourceSpan> }` with `is_empty()`, as field `properties` of
  `YamlValueLocation`, re-exported from `yaml::analyze`, `biscuit_file::yaml`,
  and the crate root. `span` still starts at the first property token (so
  `md clean` is unchanged); the field doc tells value-only editors to refuse a
  non-empty `properties`. Renewal (Phase 3) must check `properties.is_empty()`.
- Tests (lib unit tests, L1): 8 in `yaml::analyze::tests::scan::`
  (`test_block_value_context_zero_indent_sequence_under_key`,
  `..._following_key_after_zero_indent_sequence_is_root`,
  `..._second_zero_indent_sequence_restarts_index`,
  `..._zero_indent_sequence_of_mappings`,
  `..._indentless_sequence_under_nested_key`,
  `..._root_sequence_stays_at_root`,
  `..._sequence_after_inline_value_key_stays_at_root`,
  `test_key_occurrences_under_zero_indent_sequence`) and 17 in
  `yaml::analyze::tests::locate::` (anchor/tag/alias reporting, either order,
  interior indicator bytes, seven multi-line `None` cases, three still-locates
  controls, two zero-indent list cases); `test_locate_anchored_value_still_locates`
  renamed to `test_locate_anchored_value_reports_anchor`.
- The standalone spike crate `spikes/frontmatter-reader` has a `locate_probe`
  test that pins the old behavior. It is not a workspace member and was left
  as the record of what the spike saw.
- `just lint` in `biscuit-file/` is clean.

### Spike S1 — Embedded schema reference (subagent, isolated worktree)

- Findings: `spikes/embedded-schema-ref/findings.md`; prototype diff:
  `spikes/embedded-schema-ref/prototype.diff`. The throwaway worktree was
  removed after the diff was saved.
- **Answer:** the embedded baseline cannot resolve a relative `Name@file`
  import. `darkmatter_base_schema_ref` only parses; adding the line breaks
  `md compose` ("baseline could not be converted") and panics DMLS at
  `mod.rs:157`. A baseline loaded from disk (`--schema <abs path>`) already
  resolves it.
- **Smallest fix (~110 lines, ~0.5 day):** embed `content-policy.yaml` with
  `include_str!`, give the import engine a table of embedded files keyed by a
  virtual repo-relative path with a lexical join (no filesystem access), and
  expand the baseline once in `darkmatter_base_schema_ref`. Proven in the
  prototype for `md compose`, DMLS validation and completion, and an installed
  binary with `content-policy/` moved away. `CARGO_MANIFEST_DIR` resolution was
  rejected (breaks installed binaries from deleted worktrees, and panics).
- **For task 6.1:** the embedded-resolution change does not depend on
  `2026-09-28-recursive-schema-types` and can land early; only the `policy[]`
  line waits.
- **Ruling needed for AC 21:** `md schema validate` never applies the embedded
  baseline to a document with no `$schema` (documented Darkmatter contract,
  "valid by default"). Recommended: accept `md schema validate --schema
  <path to darkmatter.yaml>` as the evidence and fix the separate existing bug
  where a relative `--schema dir/file.yaml` applies the directory twice
  (`with_baseline_from_file`, `mod.rs:345-348`). Raised as a human-review item
  on the spec; it only affects Phase 6.
- Incidental: `darkmatter markdown::schemas::file_match::tests::conversion_emits_every_root_union_glob`
  failed in the spike worktree too, independent of the base schema (see the
  test-results section below).

### 1.3 Biscuit File tab-indentation repair (subagent)

- New diagnostic code `YamlDiagnosticCode::TabIndentation`
  (`yaml.tab-indentation`, Deterministic); `ALL` grows from 23 to 24 (additive)
  and the pinned-list test is updated. Each tab-indented line gets one
  diagnostic whose single `YamlRepair` replaces its leading whitespace: every
  tab in the prefix becomes two spaces, spaces stay, which is Darkmatter's
  tab normalization. Block scalars therefore read `"Line one\n  Line two"`.
- Runs only when the original does not parse (as Darkmatter normalizes only on
  a failed direct parse), after BOM recovery and before reserved-indicator
  quoting. **Value gate:** the tab-normalized text is built independently, the
  edits must all apply through `apply_edit_set`, and the result must parse to
  the same value as that text; if the normalized text still fails, no repair is
  offered.
- The corpus case `regression/tab-indentation-is-unrepairable` encoded the old
  "report only" decision; the spec reverses it, so it moved to the repaired
  cases as `regression/tab-indentation-is-repaired`.
- Tests: 7 in `yaml::analyze::tests::tab_indentation::` (block scalar inner
  tabs, the `update_policy:\n\t- Duration(6mo)` list case, nested mappings,
  tabs after indentation untouched, no tabs → no repair, parseable document
  with tabs → no repair, repair withheld when normalized text still fails);
  Darkmatter `l1` `tab_indentation_repair_parity::test_tab_repair_matches_darkmatter_for_block_scalar`
  and `..._for_list_and_nested_mapping` (repaired record equals Darkmatter's
  parsed record); `darkmatter-cli` `l1`
  `clean_frontmatter::test_clean_repairs_tab_indented_frontmatter` (`md clean`
  offers and applies the repair).
- Docs: `biscuit-file/lib/README.md` ("YAML Source Analysis and Repair" — the
  repair engine has no separate docs page) and `darkmatter/docs/cli/clean.md`.

### 1.4 UTC `last_updated` stamps (subagent)

- One shared helper `darkmatter::markdown::hash::last_updated_stamp(now:
  DateTime<Utc>) -> String` (in `hash/options.rs`, beside `LAST_UPDATED_KEY`).
  `md hash --save` (`run_hash_save` now takes `now`, `run_hash` passes
  `Utc::now()`), the effects auto-rehash (`effects/verbs.rs`), and Claudine's
  completion write-back (`loop_control.rs`) all call it with `Utc::now()`.
  Claudine CLI already depends on `darkmatter`, so no dependency was added.
- Tests (R14: instant `2026-09-28T23:30:00Z`, the `+10:00` view is asserted to
  be `2026-09-29`, the stamp is asserted `2026-09-28`, no `TZ` mutation):
  `darkmatter markdown::hash::options::tests::last_updated_stamp_uses_the_utc_date`
  (helper) and `darkmatter-cli commands::hash::tests::hash_save_stamps_last_updated_with_the_utc_date`
  (end to end: a temp document with a stale hash is saved and the file holds
  `last_updated: 2026-09-28`).
- **Spec departure — effects writer:** its stamp is never written today.
  `save` calls `plan_hash_save(None, …)`, which always returns
  `bump_last_updated: false`, so the computed date is discarded. The spec's
  "Other writers of the baseline" says effect writes bump `last_updated`; they
  do not. Behavior was not changed (out of scope); the site still uses the UTC
  helper so it is correct if the bump is ever enabled. AC 20 for this writer is
  covered by the helper test only, because there is no observable output.
- **Claudine writer:** observable only by running the whole harness loop, so it
  is covered by the shared helper test, not a Claudine-specific test.
- Docs: `darkmatter/docs/cli/hash.md` ("current local date" → "current UTC
  date") and `claudine/docs/getting-started/index.md` ("(local time)" →
  "(UTC)"). The other "local date" docs describe `ctx.today`/`ctx.now`, a
  separate feature, and were left alone.

### 1.6 Rulings

This session is non-interactive and the plan has `yolo: true`, so no author
answers were received. **R1–R17 are adopted as their recommendations, with no
overrides**, and Phases 2–6 proceed on them unchanged. Additions from Phase 1
that bear on them:

- R16: `deps-check` does not run in CI (see 1.1). Author decision requested.
- R1 / task 6.1: Spike S1 shows the embedded-schema fix can land before the
  gate, and that the `md schema validate` half of AC 21 needs a ruling (see
  Spike S1). Raised as a human-review item on the spec, for Phase 6 only.


## Phase 2

### 2.1 Core types (`model.rs`, `diagnostic.rs`)

- `Rule` (`Evergreen`, `TimeSensitive`, `ValidFor { duration, baseline }`,
  `ValidUntil { deadline }`), `Baseline` (`Inline`, `Reference`, `Defaulted`),
  `Deadline` (`Inline`, `Reference`; the spec lets `@name` fill a deadline
  position), `Duration`/`DurationUnit`, `Action` (`Ord` is the precedence),
  `Renewal`, `PolicyEntry`, `Policy`, `EvidenceRecord` (wraps
  `serde_json::Map`), `PolicyOptions`, `GRAMMAR_VERSION = 1`.
- `Policy` is non-empty and `Evergreen`-alone by construction (`new`,
  `from_declaration`, `from_text`, `from_json` all validate).
  `PolicyOptions` has no way to express "no default" (AC 10): the default is a
  `Policy`, replaceable with `with_default_policy`, never an `Option`.
- **Departure from the plan:** `Policy` has no `defaulted` flag. Whether the
  evaluated policy was declared or defaulted is a property of an evaluation,
  not of a policy (a caller's default is an ordinary `Policy`), so it lives in
  the report as `policy.source` (`PolicySource::{Declared, Defaulted}`). It
  also keeps the serialized form "entries only" (R8).
- Diagnostics carry a `DiagnosticCode` (23 snake_case codes) and a `Location`
  (`policy`, `entry`, `entry_field`, `property {name, entry}`, `frontmatter`).
  `Invalid { document, diagnostics, warnings }` is the no-verdict result.

### 2.2 Declaration parser (`grammar.rs`, `normalized.rs`)

- Compact grammar: `Name` or `Name(args)`, args split at `,` with surrounding
  spaces trimmed (the schema can stay stricter). Durations
  `[1-9][0-9]*(d|wk|mo|yr)`; dates are real `YYYY-MM-DD` dates, a timestamp
  gets the dates-only code; `@name` accepts `[A-Za-z_][A-Za-z0-9_-]*`, and a
  dotted name is `nested_reference`. `FileChanged` is an unknown rule until
  Phase 5.
- Targeted messages: empty list → suggests `Evergreen`; single string → shows
  the list form; `Duration` → suggests `ValidFor`; a case mismatch names the
  right spelling; missing `action` (R3) → points at the compact form;
  unbalanced parentheses → when the neighbouring entry is the other half, the
  message names the joined rule and shows `- ValidFor(3mo, 2026-09-28)`.
- All invalid entries are collected; diagnostics are sorted by entry.
- R8: normalized JSON `{"grammar_version":1,"entries":[{"rule","action"}]}`
  with `deny_unknown_fields`, required `u32` version, version `0` or newer than
  the library is an error. A custom sequence visitor prefixes element errors
  with `entry N` so the diagnostic names the element.
- R7: identity is `xxh64:` + 16 hex over the compact JSON of sorted entries
  `{rule, parameters, action}` plus `grammar_version`. Inline baselines become
  the marker `inline`, the shorthand `default`, a reference `@name`; so the
  baseline *form* counts but its value does not.
- `Policy::from_text` implements R5 for Phase 4's `--default-policy`.

### 2.3 Time semantics (`time.rs`)

- `start_of_day` (00:00 UTC) and `add_duration` (chrono `checked_add_months`
  / `checked_add_days`; `yr` = 12 months with a checked multiply). Table test
  cross-checks Darkmatter's `add_duration_spec` rule. An inline baseline that
  overflows is caught while parsing; a referenced one during evaluation, both
  as `date_out_of_range`.

### 2.4 Frontmatter reader (`reader.rs`, public module)

- `read_frontmatter(bytes) -> Result<ReadOutcome, ReadError>`;
  `ReadOutcome::{NoFrontmatter, Found(Frontmatter)}`. `Frontmatter` exposes the
  record, `block()` and `yaml()` byte ranges in document offsets,
  `fence_line_ending()`, and `tab_repair()` (the Biscuit File repair edits,
  shifted into document offsets) for Phase 3.
- Parses through `biscuit_file::Yaml::from_str` and deserializes the
  `serde_yaml_ng::Value` into `serde_json::Map` without naming
  `serde_yaml_ng`, so the dependency list is unchanged (AC 22). Final line
  terminator stripped before parsing. Duplicate keys (any depth, R4) come back
  as `ReadError::DuplicateKey` with the key name parsed from the parser's
  message; evaluation turns them into a `duplicate_key` diagnostic.
- Tab repair: only after a failed parse, only diagnostics with code
  `yaml.tab-indentation` are applied, and a warning
  `tab_indentation_repaired` is put on the report (R13).
- An unparseable block containing `{{` is `MalformedCause::ExpressionTemplate`
  with a message naming Darkmatter's expression protection.
- **Decision:** evaluation also fails closed on an unterminated block (with or
  without `...`) and on a `----` near-miss fence; both are `ReadError`s, not
  "no frontmatter". The spec lists them only as renewal refusals; reading such
  a document under the default policy would evaluate a document whose
  declaration we could not see.

### 2.5 Aggregation (`aggregate.rs`)

- `aggregate(results) -> Aggregate` is a pure function over
  `(Action, ResultKind)`. Test: all 819 combinations of up to three entries
  against an oracle transcribed row by row from the spec's two tables, plus
  every permutation of every three-entry combination.

### 2.6 Evaluator and report (`evaluate.rs`)

- `evaluate_record(record, context)`, `evaluate_policy(policy, record,
  context)`, `evaluate_document(bytes, context)`. `EvaluationContext` holds the
  explicit `DateTime<Utc>`, the options, and an optional document label (R13,
  never canonicalized). Phase 5 can add the base directory and provider here.
- Every entry is evaluated; any invalid resolved value makes the whole call
  `Err(Invalid)`. A report therefore always has a verdict.
- JSON field names frozen by `report_json_field_names_are_frozen`: `document`,
  `evaluated_at` (RFC 3339, `Z`), `policy {source, grammar_version,
  identity}`, `status`, `action`, `evaluation_complete`,
  `action_resolution_complete`, `results[] {index, rule, action, renewal,
  result, unknown_reason, baseline, deadline, due, reason}`, `warnings[]
  {code, message}`. Absent optional values serialize as `null`, not omitted.

### 2.7 Robustness matrix (`tests/robustness_matrix.rs`)

- Fixture `lib/tests/fixtures/stamped-note.md`: the migrated
  `terminal-multiplexing/zellij.md` after `md hash --save` on a temp copy
  (added `hash:`), body trimmed after stamping. Read with `include_str!`.
- `yaml_frontmatter_matrix`: a control row (fresh, declared) and 48 one-edit
  cells over the policy value, one entry, long-form `rule`, long-form
  `action`, and the date property; each asserts status/source, unknown reason,
  diagnostic code and location, or malformed cause through
  `evaluate_document`. Each edit target is asserted unique and each edit is
  asserted to change the text.
- `serialized_policy_matrix`: a control row plus 19 one-edit cells over
  `grammar_version` and `entries`, starting from `Policy::to_json`.
- Mutation check: flipping one expected outcome in each walk turned both
  tests red; restored.
- Smell grep over `lib/src`: no `#[serde(default)]` or `unwrap_or_default()`.
  `grammar.rs` `NaiveDate::parse_from_str(..).ok()` maps `None` to the
  `invalid_date` diagnostic (justified). A `filter_map(..).ok()` in the
  evaluator that pushed errors before discarding was rewritten as an explicit
  loop.

### Requirement → test map

| Requirement | Test(s) |
| --- | --- |
| AC 1 normalize, defaults and sources visible | `evaluation::compact_and_long_forms_normalize_consistently`, `inline_referenced_and_defaulted_baselines_evaluate_alike`, `an_absent_policy_uses_the_callers_default` |
| AC 2 evidence values, YAML 1.1 rows, quoting | `evidence_values_table_in_every_date_position`, `yaml_1_1_spellings_in_a_date_position`, `quoted_and_unquoted_dates_are_equivalent` |
| AC 3 time | `valid_until_takes_effect_at_utc_midnight`, `valid_for_is_due_on_its_computed_date`, `month_ends_and_leap_years_clamp`, `a_future_baseline_is_unknown_not_fresh`, `a_referenced_baseline_out_of_calendar_range_is_a_validation_error`, `time::tests::*` |
| AC 4 aggregation | `aggregate::tests::every_combination_of_up_to_three_entries_follows_the_tables`, `entry_order_never_changes_the_result`, `named_rows` |
| AC 5 no writes / no capture | `evaluation_never_captures_a_baseline` |
| AC 7 (library, steps 1, 2, 5, 6, 8) | `lifecycle_steps_through_the_library` |
| AC 10 fail closed | `fail_closed_declarations_never_yield_fresh`, `an_absent_policy_uses_the_callers_default`, `grammar::tests::policy_new_enforces_shape` |
| AC 11 plain map | `a_plain_evidence_map_needs_no_document` (deps half: `just deps-check`) |
| AC 16 legacy + migrated docs | `legacy_duration_and_update_policy`, `migrated_documents_evaluate_without_diagnostics` (23 `include_bytes!` reads) |
| AC 17 comma trap | `the_flow_list_comma_trap` |
| AC 18 duplicate keys | `duplicate_top_level_keys_are_validation_errors`, `reader::tests::duplicate_keys_are_rejected_anywhere_in_the_block` |
| AC 23 identity | `normalized::tests::identity_*` |
| AC 26 dotted reference | `a_dotted_reference_is_rejected_even_when_the_literal_key_exists` |
| AC 27 reader half | `tab_indented_frontmatter_is_evaluated_with_a_warning`, `unquoted_templates_are_rejected_with_the_reason`, `reader::tests::tab_indentation_is_repaired_in_memory` |
| AC 28 clip-chomp | `a_clip_chomped_block_scalar_as_the_last_key_reads_like_darkmatter`, `reader::tests::clip_chomped_block_scalar_as_last_key_has_no_trailing_newline` |
| Matrix | `robustness_matrix::yaml_frontmatter_matrix`, `serialized_policy_matrix` |

### Tests, placement, and gates

- 24 unit tests in `lib/src` (`--lib`) and two integration binaries,
  `evaluation` (29 tests) and `robustness_matrix` (2 tests), all L1 with no
  tier marker. The library keeps Cargo's per-file test discovery (no
  `autotests = false`), so both files are compiled. `just check-tier-coverage
  content-policy`: nothing stranded.
- `just test` in `content-policy/`: 55 passed. `just lint` (clippy for both
  crates plus `deps-check` for default and `--all-features`): clean.
- The repository pins `* text=auto eol=lf`, so the `\n`-based fixture edits
  hold on a Windows checkout. `just cross-check content-policy --os windows`
  (native Windows): 55 passed. Linux and WSL2 were not run: Phase 2 has no
  filesystem, path, or process code, and CI's pull-request Linux leg covers
  it. `just doctest`: no doctests; clean.
- No pre-existing or skipped failures in the `content-policy` scope.

### Docs

- `content-policy/README.md` and `docs/topics/policy-lifecycle.md`: the status
  line now says the reader and time-rule evaluation are built, with renewal,
  CLI, schema, and `FileChanged` still planned. The topic page gains a "Read a
  Report" section (the frozen JSON fields, the three evaluation entry points,
  and the strict normalized-policy JSON), as the spec requires field names to
  be documented in the change that implements them.
- **Drift fixed from Phase 1:** the topic page still said the legacy notes
  "are planned to be migrated" and that the three `last_updated` writers
  "currently stamp the local date"; both were done in Phase 1 and now read as
  done.

## Phase 3

### 3.1 Renewal planner (`lib/src/renew.rs`)

- Public API: `plan_renewal(bytes, &RenewalContext) -> Result<RenewalPlan,
  RenewalError>`; `RenewalContext::new(today)` takes the current UTC date as
  an injected value (`RenewalContext::now()` reads the system clock), `.on(date)`
  sets the update date, plus `with_options` / `with_document`. A future update
  date is `RenewalError::FutureDate`.
- Planning validates by running `evaluate_record` at 00:00 UTC of the update
  date, so a malformed declaration or a present non-date baseline is
  `RenewalError::Invalid` with the evaluator's own diagnostics (renewal never
  repairs). Duplicate keys reuse the evaluator's diagnostic through a new
  shared `evaluate::duplicate_key_invalid` (extracted, no behavior change).
- Every `ValidFor` entry yields a write: inline → `BaselineTarget::Inline
  {entry}`, `@name` → `Property {name}`, shorthand → the configured date
  property. Writes to one target consolidate into one `BaselineChange` listing
  every entry. Differing values for one property are a `DifferentWrites`
  conflict (unreachable with time rules, which all write the update date;
  unit-tested on the private `consolidate` so Phase 5's fingerprint writes
  inherit it). A renewed property that a `ValidUntil(@name)` also reads is a
  `MovesDeadline` conflict.
- `ChangeKind`: `renewed`, `new_baseline` (absent or `null`), and
  `unchanged` (already the update date; no byte edit). `nothing_to_renew()`
  is an empty `changes` list; such a plan lists no tab repair and applies to
  the same bytes.
- Everything is planned before any edit: all refusals and conflicts are
  collected, and any one means no plan.
- The plan carries `fingerprint` (`xxh64:` + 16 hex via
  `biscuit_hash::xx_hash_bytes`, public helper `plan_fingerprint`) and the
  evaluated `PolicySummary`. It derives `Serialize` for Phase 4's `--json`;
  field names are **not frozen yet**.
- **Decision (spec silent):** a caller's default policy with an inline
  baseline, such as `ValidFor(3mo, 2026-01-01)`, cannot be renewed because the
  date is not in the document; it is refused as `not_in_document`.

### 3.2 Span-targeted editor

- Values are located with `locate_yaml_value` / `locate_yaml_key` on the
  frontmatter slice. For tab-indented frontmatter the locator runs on the
  **repaired** YAML (valid YAML), and offsets map back to the document through
  the repair (`Yaml::to_document`); targets never fall inside the leading
  whitespace the repair rewrites. Unit test:
  `renew::tests::located_offsets_map_back_through_the_tab_repair`.
- Inline dates: the rule string is located (compact item, or `Key("rule")`
  of a long-form item), unquoted (plain, `'…'`, or `"…"` without `\`), checked
  against the parsed rule text, and the edit narrows to the baseline date.
- Null fill: `~`/`null` spans are replaced; an empty value gets ` <date>`
  right after the colon, so `last_updated:   # todo` becomes
  `last_updated: 2026-09-28   # todo`.
- Missing properties: one insertion holding every new line, at the end of the
  block with the preceding line's terminator (the fence's for an empty block);
  no frontmatter → a new block after any BOM with the body's first terminator
  (LF when there is none).
- **Departure from the spec (logged, not in the spec):** when the block ends
  with a clip- or keep-chomped block scalar (`note: |`, `note: >+`), a line
  appended after it changes that scalar's value (`"text"` → `"text\n"`),
  because the reader parses without the final terminator (AC 28). The safety
  net caught it on the first run. The spike counted 42 repository files ending
  this way, so refusing them was a poor outcome; the new property is inserted
  **in front of that last top-level entry** instead. Strip-chomped (`|-`)
  scalars append at the end as usual. Documented on the topic page.
- Refusals (`RefusalReason`): `flow_list` (the policy value after the key
  starts with `[`), `multi_line_value`, `escaped_string`,
  `anchor_alias_or_tag` (on the value, or on the policy list itself),
  `flow_mapping` (`- {rule: …}` when the rule must change), `unterminated_block`,
  `near_miss_fence`, `span_mismatch`, `not_locatable`, `not_in_document`. The
  first three messages show the block-list form with the entry's rule.
- **Biscuit File finding (not fixed here):** `locate_yaml_value` returns the
  header `>-` as a one-line value for a block-scalar *sequence item*
  (`- >-\n    ValidFor(…)`), although its module docs say block scalars yield
  `None`. Renewal treats any located value that is a block scalar header as
  `multi_line_value`. A Biscuit File fix is a candidate follow-up; it would not
  change renewal's behavior.
- Edits are applied with `apply_edit_set`; rejected (overlapping or
  out-of-range) edits trip the safety net.

### 3.3 Apply and safety net

- `RenewalPlan::apply_to(bytes)` refuses bytes whose fingerprint differs
  (`ModifiedSincePlan`), applies `edits` + `tab_repair`, and verifies the
  result: it must read without a repair, keep every byte before and after the
  frontmatter block, and read as exactly the original record with the planned
  values written (for inline targets, the rule string with its date replaced).
  Otherwise `SafetyNet` names the keys that would change. `plan_renewal` runs
  the same check, so a preview never shows an edit that apply would refuse.
- `apply_renewal(path, &plan)` is the shared file helper: it re-reads the
  file, calls `apply_to`, and writes only when bytes change.
- Atomic write: no helper exists in Biscuit File (checked), and `tempfile`
  would be a new runtime dependency against AC 22, so the write is std-only:
  a sibling `create_new` temporary file, `sync_all`, the original's
  permissions, then `rename`; the temporary is removed on failure. A symlinked
  document is written through to its target, so the link survives.

### Tests, placement, and gates

- New integration binary `lib/tests/renewal.rs` (22 tests, L1, no tier
  marker; the library keeps Cargo's per-file discovery), plus 7 unit tests in
  `renew::tests` (one `#[cfg(unix)]`: the symlink write-through).
- The 23 migrated documents moved from a local array in
  `tests/evaluation.rs` to `tests/common/mod.rs` (`MIGRATED_DOCUMENTS`, still
  `include_bytes!`), used by `evaluation` and `renewal`. Per
  `docs/cicd/test-inputs.md`, a path in a shared helper schedules every L1 test
  in each binary that includes it; only these two binaries include it, and
  both need the documents.
- Smell grep over `renew.rs`: `unwrap_or_default()` on the record when there
  is no frontmatter (no frontmatter *is* an empty record by definition), and
  `.ok()?` in `inline_date`, whose `None` becomes a `span_mismatch` refusal.
  Both justified; no `#[serde(default)]`.
- `just test` in `content-policy/`: 87 passed. `just lint` (clippy
  `--all-targets -D warnings` for both crates, plus `deps-check` for default
  and `--all-features`): clean. `just check-tier-coverage content-policy`:
  nothing stranded.
- `just cross-check content-policy --os windows` (native Windows): 86 passed
  (the Unix-only symlink test is not compiled there). `--os linux`: pass. WSL2
  was not run; the new OS-sensitive code is the rename-based write, which
  Linux and native Windows cover.
- No skipped or pre-existing failures in the `content-policy` scope.

### Requirement → test map

| Requirement | Test(s) |
| --- | --- |
| AC 6 only targets change, settings kept, shared writes consolidated, conflicts and incomplete capture rejected | `renewal::every_renewable_entry_is_renewed_and_nothing_else_changes`, `renewing_a_referenced_deadline_is_a_conflict`, `one_refused_target_means_no_partial_plan`, `a_present_malformed_baseline_is_an_error`, `renew::tests::identical_writes_consolidate_and_different_ones_conflict` |
| AC 7 (library, all eight steps, injected clock) | `renewal::lifecycle_steps_through_the_library` (on a temp file through `apply_renewal`; step 3 and 7 byte-exact) |
| AC 12 bytes outside values unchanged; changed file refused | `darkmatter_comparison_table`, `spike_matrix_accepted_shapes`, `apply_writes_the_planned_bytes_and_refuses_a_changed_file` |
| AC 14 flow list / multi-line / escapes refused naming the block list; evaluation still succeeds; flow list with outside target renews | `refused_shapes_name_their_reason`, `flow_lists_renew_only_outside_the_brackets` |
| AC 17 quoted flow reference renews; unquoted is malformed | `spike_matrix_accepted_shapes` ("quoted flow-list reference"), `invalid_declarations_and_duplicate_keys_are_errors` |
| AC 19 no frontmatter → new block, body unchanged | `a_document_without_frontmatter_gets_a_new_block` |
| AC 23 identity unchanged by renewal | `renewal_keeps_the_policy_identity` |
| AC 25 (library half) new baseline label, nothing to renew | `missing_and_null_targets_are_new_baselines`, `policies_without_a_renewable_entry_have_nothing_to_renew`, `a_plan_with_nothing_to_renew_writes_nothing` |
| AC 27 (renewal half) tab repair listed and applied | `tab_indented_frontmatter_lists_and_applies_the_repair`, `migrated_documents_renew_byte_exactly` (8 tab-repaired) |
| AC 29 (renewal half) zero-indent list edited in place | `spike_matrix_accepted_shapes` ("zero-indent list", "zero-indent long form") |
| AC 30 Darkmatter table + mixed line endings | `darkmatter_comparison_table`, `mixed_line_endings_are_kept_per_line` |
| AC 31 anchor/alias/tag, flow mapping, unterminated, `...`, `----`, span mismatch, safety net | `refused_shapes_name_their_reason`, `renew::tests::a_span_that_does_not_decode_to_the_parsed_value_is_refused`, `the_safety_net_refuses_a_corrupted_edit` |
| Update date rules | `the_update_date_defaults_to_today_and_rejects_the_future`, `a_baseline_already_at_the_update_date_is_unchanged` |
| Plan fingerprint | `the_plan_fingerprints_the_bytes_it_read` |
| Checkpoint: migrated documents | `migrated_documents_renew_byte_exactly` (temp copies; only `last_updated` and tab-indented lines differ; each re-evaluates `fresh` with no warning) |

### Docs

- `docs/topics/policy-lifecycle.md`: status line (renewal built); "Renewal
  is planned to edit" → present tense; new paragraphs on where a missing
  property goes (including the trailing block-scalar rule, with an example),
  the update date, `unchanged`, and malformed baselines; a new "Renew from a
  Library" section with a Rust example, a plan/apply sequence diagram, and the
  plan's fields. The `policy renew` CLI text stays **planned**.
- `README.md`: status line and a paragraph on the renewal library API.
- `docs/dependencies.md` (area): `xx_hash` also names the plan fingerprint;
  the std-only atomic write; `tempfile` as a dev-dependency.
