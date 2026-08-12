---
total_phases: 5
created: 2026-07-16
phase: 1
agent: codex/default
yolo: true
---

# Execution Plan — Preserve Dynamic Text in Prose Error Surfaces

Source: [`spec.md`](./spec.md)

## Outcome

Dynamic error, warning, path, and provider text must render byte-for-byte through
Claudine's Prose-backed terminal surfaces while static labels retain their
styling. Intentional Prose/Markdown content remains markup-aware.

## Verified implementation facts

- `biscuit-terminal` already exposes `Prose::escape_text` in
  `biscuit-terminal/lib/src/components/prose/prose.rs`; this plan strengthens
  and adopts that API instead of creating a second literal-text facility.
- `claudine/cli/src/log.rs::error` is a HIGH-risk shared sink in the current
  GitNexus index: five direct callers and seven upstream symbols across three
  modules. Its caller contract must be reviewed before the implementation is
  changed.
- The current CLI audit command finds 118 `Prose::new(format!` sites across 30
  files. Several Claudine modules also carry local `escape_prose`/
  `prose_escape` implementations whose character sets have already drifted.
- `biscuit-terminal`'s Prose documentation currently says dynamic interpolation
  usually does not require escaping. The incident disproves that statement for
  punctuation-flanked paths, so the public docs must change with the code.

## Execution conventions

- `[P]` marks work that can proceed in parallel with other `[P]` tasks in the
  same phase after their shared prerequisites are complete.
- Do not change Prose's simplified emphasis-flanking semantics. The boundary is
  literal dynamic data versus intentionally authored Prose, not a Markdown
  parser redesign.
- Before editing any named function, rerun GitNexus upstream impact analysis on
  that symbol. Stop and report any new HIGH or CRITICAL expansion before
  proceeding.
- Use package-area `just` recipes, which run tests through nextest. Do not run
  `cargo fmt` in write mode.

## Phase 1 — Lock the failure and classify the surface area

**Goal:** Establish a failing regression and a complete, reviewable inventory
before changing the shared literal-text contract or any high-fanout sink.

- [ ] Rerun upstream impact analysis for `Prose::escape_text`,
  `claudine-cli::log::warn`, `claudine-cli::log::error`,
  `try_format_api_error`, and each additional formatter selected by the audit;
  record direct callers, affected flows/modules, and risk in the implementation
  notes.
- [ ] [P] Add a focused regression that constructs
  `HarnessError::PathResolutionFailed` with
  `prompts/_reviews/a.md` and `/x/prompts/_reviews/a.md`, routes its `Display`
  text through the production generic error formatter, and proves the current
  output loses or styles the underscores before the fix.
- [ ] [P] Capture the current CLI interpolation inventory with
  `rg -n 'Prose::new\(format!' claudine/cli/src --glob '*.rs'` in a fix-local
  `audit.md`. For every hit, record its data provenance and classify it as
  static/trusted markup, intentional Markdown-aware content, literal dynamic
  text requiring `Prose::escape_text`, a tag attribute requiring
  `Prose::quoted_attr`, or pre-rendered terminal content that must not be parsed
  again.
- [ ] [P] Inventory Claudine-local Prose escape helpers with
  `rg -n 'fn (escape_prose|prose_escape)|Prose::escape_text' claudine --glob '*.rs'`;
  add every helper to `audit.md` with a replace/retain decision and rationale.
- [ ] Explicitly mark help text, hook descriptions, composed-document previews,
  and other authored Markdown surfaces as out of literalization scope so the
  implementation cannot accidentally disable their intentional styling.
- [ ] **Validation checkpoint:** the incident regression fails for the expected
  underscore/italic corruption, the audit has no unclassified rows, and the
  recorded scope distinguishes text nodes, tag attributes, and already-rendered
  output.

## Phase 2 — Make `Prose::escape_text` the authoritative literal contract

**Goal:** Biscuit Terminal owns one parser-aligned facility that round-trips
arbitrary dynamic text at every color depth without changing authored Prose.

- [ ] Keep `Prose::escape_text` as the public API. Consolidate its
  parser-significant character classification with the Markdown preprocessor's
  backslash-escape checks under `components/prose`, so grammar growth updates
  one source of truth; keep tag-attribute quoting as the separate
  `Prose::quoted_attr` contract.
- [ ] Ensure the public facility preserves UTF-8 and its existing CSI/OSC
  pass-through behavior while round-tripping literal backslashes, braces,
  brackets, parentheses, angle brackets, asterisks, and underscores without
  leaking escape backslashes or internal placeholders.
- [ ] Add focused Biscuit Terminal unit tests that apply
  `Prose::escape_text`, render through `Prose`, and assert exact source equality
  for: two underscore-bearing paths in one message, `*`/`**` text,
  `[text](ref)`-shaped text, backtick-bearing text, a recognized `<red>`-like
  token, an unknown `<tag>`-like token, and a Windows-shaped path containing
  backslashes and an underscore-prefixed directory.
- [ ] Run the same literal round-trip matrix with `ColorDepth::None` and a
  styled terminal profile; strip only expected outer styling when comparing the
  styled case, and assert the dynamic span never emits italic or OSC8 control
  sequences.
- [ ] Preserve characterization tests for intentional `_italic_`, `**bold**`,
  links, fenced code, and bracketed tags passed directly to `Prose::new` without
  `escape_text`.
- [ ] **Validation checkpoint:** from `biscuit-terminal/`, focused Prose tests
  and `just test` pass; literal cases are byte-identical and authored markup
  retains its existing semantics.

## Phase 3 — Fix Claudine's shared error and provider-stderr sinks

**Goal:** The high-fanout error paths use literal dynamic segments and styled
static templates, with direct regression coverage at plain and styled depths.

- [ ] Update `claudine/cli/src/log.rs::warn` and `error` so only their static
  labels are Prose markup and every `msg` is passed through
  `Prose::escape_text` before interpolation. Update their rustdoc to remove the
  obsolete contract that callers may embed Prose tags in arbitrary messages.
- [ ] Review all direct callers of `log::error` and `log::warn`; convert any
  intentional styling into an explicit structured/static template at the
  owning call site rather than weakening the literal-by-default sink.
- [ ] Keep `render_top_level_error`'s typed `BlockError` path unchanged, but
  prove its generic `report.to_string()` fallback inherits the fixed literal
  behavior through `log::error`.
- [ ] Escape dynamic fields independently in
  `claudine/cli/src/output/api_errors.rs`: provider message, request ID,
  malformed JSON fallback, and generic CLI error body. Preserve the static
  `Error:`/`API Error` labels and static remediation prose.
- [ ] Confirm unmatched child stderr in
  `commands/wrap/exec/spawn/semantic.rs` remains raw passthrough rather than
  being unnecessarily parsed by Prose; record that safe path in the audit.
- [ ] Add tests around the production formatter for the incident
  `HarnessError` at `ColorDepth::None` and TrueColor. Assert both underscores
  and every punctuation byte survive, no escape backslashes/sentinels leak, no
  italic SGR appears, and the styled case still contains the red/bold `Error:`
  label.
- [ ] Add provider-stderr formatter tests with underscore paths, Markdown link
  shapes, angle-tag shapes, and backticks in structured and generic errors;
  retain the existing prefix-deduplication assertions.
- [ ] **Validation checkpoint:** the Phase 1 incident regression is green in
  plain and styled modes, all direct callers of the HIGH-risk shared sink have
  been reviewed, and typed `BlockError` output is byte-stable outside the new
  fallback tests.

## Phase 4 — Complete the interpolation audit and remove grammar copies

**Goal:** Every dynamic Prose interpolation in Claudine has an explicit policy,
and no Claudine-local helper duplicates Biscuit Terminal's text grammar.

- [ ] Apply the Phase 1 audit decisions to all CLI error/status surfaces,
  including dynamic fields in `output/error_report.rs`, `output/mod.rs`,
  command status lines, log reports, and wrapper/watchdog messages. Escape each
  dynamic text segment before it is combined with static markup; do not escape
  the final composed markup string.
- [ ] Replace Claudine-local text-node helpers such as `escape_prose`,
  `prose_escape`, and `escape_prose_path` with `Prose::escape_text`. Use
  `Prose::quoted_attr` for dynamic tag attributes and retain a local helper only
  when it performs a demonstrably different non-Prose transformation, with that
  distinction documented and tested.
- [ ] Update or delete stale comments beside changed renderers, especially
  comments claiming that escaping only `<`, `>`, `{`, or `\\` is sufficient.
  Preserve comments that explain trusted authored-Markdown boundaries.
- [ ] For every intentionally markup-aware `Prose::new(format!` site left in
  place, finish the `audit.md` row with the trusted data source and an existing
  or new test that proves the styling is intentional.
- [ ] Add focused regressions for any additional dynamic error/status surfaces
  changed by the audit, using punctuation-rich values rather than assertions
  that only check a substring.
- [ ] [P] Update `biscuit-terminal/docs/components/prose.md` and
  `.claude/skills/biscuit-terminal/styling.md` to document
  `Prose::escape_text` as mandatory for untrusted/dynamic text interpolation,
  add it to the public API examples/table, and remove the misleading claim that
  flanking rules usually make dynamic interpolation safe.
- [ ] [P] Review Claudine's README, CLI reference, architecture skill, and
  timeline for a stated dynamic-markup contract; update only documents whose
  public guidance or architecture description is changed by this fix.
- [ ] **Validation checkpoint:** `audit.md` has a resolved action and evidence
  for every original and newly introduced site; repository searches find no
  duplicated Prose text-escape character loops in Claudine; intentional help,
  description, and preview markup tests still pass.

## Phase 5 — Cross-package regression and closure

**Goal:** Prove the fix across package boundaries, terminal capability modes,
and supported path shapes, then verify the final blast radius.

- [ ] Run focused tests while iterating, then run `just test` and `just lint`
  from `biscuit-terminal/`.
- [ ] Run focused CLI/library tests while iterating, then run `just test` and
  `just lint` from `claudine/`; all commands must use the repository's nextest
  recipes.
- [ ] Exercise the incident path through the nearest deterministic Claudine CLI
  integration fixture with `NO_COLOR=1` and `FORCE_COLOR=1`. In both captures,
  assert the two `_reviews` segments survive copy/paste; in the styled capture,
  assert the label is red/bold and the dynamic body has no italic span.
- [ ] Verify `--plain`/`NO_COLOR` output contains no ANSI, backslash-escape, or
  placeholder leakage and that wrapping is the only permitted byte-level
  difference from the input message.
- [ ] Review test fixtures for macOS, Windows, and Linux portability: use temp
  directories or display-only path strings, avoid host-specific `/Users/...`
  assertions, and cover both slash and backslash path shapes without changing
  path-resolution semantics.
- [ ] Re-run both audit searches and confirm every new dynamic interpolation
  uses the Biscuit Terminal facility or has a documented intentional-Markdown
  classification.
- [ ] Run `git diff --check`, inspect the final diff for unrelated formatting or
  behavior changes, and run GitNexus `detect_changes` against `main`. Investigate
  any affected symbol or execution flow not predicted by the Phase 1 impact
  record before handoff or commit.
- [ ] **Final validation checkpoint:** all four specification acceptance
  criteria are covered by automated tests, both package areas pass test/lint,
  the audit has zero unresolved rows, and the final diff contains no Prose
  flanking-rule or proxy path-resolution changes.
