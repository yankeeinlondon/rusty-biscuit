---
spec: /Volumes/coding/wt/rusty-biscuit/feat-osc-9-4-protocol/content-policy/features/2026-09-28-content-policy/spec.md
plan: content-policy/features/2026-09-28-content-policy/plan.md
implemented_by: claude/opus
started_phase: 1
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

