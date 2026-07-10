---
agent: codex/
total_phases: 7
created: 2026-07-09
phase: 1
yolo: false
spec: darkmatter/fixes/2026-07-09-godless-beauty/spec.md
---

# Godless Beauty — Execution Plan

This plan implements the six improvements in the functional specification while preserving public
behavior except for the specified UTF-8 title correction, GPU-only context correction, and atomic
expression-catalog API migration. Mechanical relocations must not include opportunistic cleanup,
formatting, renamed tests, or weakened assertions.

## Dependency and parallelism map

Phase 1 establishes the baseline and removes dead code. Phases 2 and 3 then reduce duplication and
test-file pressure. Phases 4 and 5 both depend on Phase 3 but are independent and may run in
parallel. Phase 6 begins only after both branches land so its cross-workspace migration does not
compete with the lower-risk file moves. Phase 7 is the final integration gate.

## Phase 1 — Establish baselines and delete the dead transform module

### Baseline and inventory

- [ ] Capture `git status --short` and preserve all unrelated worktree changes; do not edit or
  discard pre-existing changes in the specification, render-tree parity tests, schema resolver, or
  local settings.
- [ ] Record the pre-change Darkmatter test inventory with `cargo nextest list -p darkmatter` and
  the Level-2 target inventory with
  `cargo nextest list -p darkmatter --test level2_render_tree_terminal` under this fix directory so
  later mechanical moves can be compared by test name and count.
- [ ] Run the package-area baseline from `darkmatter/`: `cargo check -p darkmatter`, `just test`,
  and `just lint`; record any pre-existing failure before implementation instead of attributing it
  to the refactor.
- [ ] Inventory references to `markdown::transform`, `TransformReport`, `TransformOptions`,
  `TransformContext`, `.transform()`, and `.transform_with()` across source and current
  documentation, distinguishing historical specifications from active architectural guidance.

### Improvement 1 implementation

- [ ] Delete `darkmatter/lib/src/markdown/transform/mod.rs` and
  `darkmatter/lib/src/markdown/transform/types.rs` without copying their implementation or stale
  tests into the live compose tree.
- [ ] Update active documentation that describes the retired transform pipeline—especially
  `darkmatter/docs/topics/darkmatter-expressions.md`—to name the current compose pipeline and its
  live APIs; leave ordinary English uses of “transform” and historical records unchanged.
- [ ] Search compiled source and active docs again and prove no reference to the deleted module or
  API remains; confirm `darkmatter/lib/src/markdown/mod.rs` still has no transform declaration.

### Phase 1 validation checkpoint

- [ ] Run `cargo check -p darkmatter`, `just test`, and `just lint` from `darkmatter/`; require all
  three to pass with no live compose implementation changes and no loss from the recorded test
  inventory.

## Phase 2 — Single-source link and image reference parsing

### Characterization and shared boundaries

- [ ] Inventory the duplicated helpers, caller-specific drivers, error construction, snapshots,
  metadata environment variables, and tests in `darkmatter/lib/src/render/link.rs`,
  `darkmatter/lib/src/render/image_ref.rs`, and `darkmatter/lib/tests/error_snapshots/{link,image_ref}.rs`.
- [ ] Add table-driven parity tests that feed equivalent Markdown and HTML reference cases through
  `Link` and `ImageRef`: non-ASCII display/alt/title text, escaped quotes and backslashes, nested
  parentheses, malformed or unclosed input, uppercase ASCII HTML attributes, inline/strip/lossless
  metadata modes, and metadata encode/decode round trips.
- [ ] Pin existing `LinkError` and `ImageRefError` caret positions, messages, and `AsBlockError`
  status blocks with byte-exact assertions before moving helpers.

### Shared parsing and metadata modules

- [ ] Add crate-private `darkmatter/lib/src/render/reference_parse.rs` and declare it in
  `render/mod.rs`; move HTML-attribute parsing, bracket/parenthesis scanners, URL/title codecs,
  structured-property tokenization, ANSI stripping, escaping, and normalization helpers into it.
- [ ] Implement every text scanner with `char_indices` or `Chars`; return neutral parsed values or
  byte offsets proven to be UTF-8 boundaries, and canonicalize HTML attribute names with
  `to_ascii_lowercase()`.
- [ ] Keep structured-property parsing generic through a per-type apply closure so link-only
  `prompt`/`target` behavior and image-only `srcset`/width behavior remain in their owning modules.
- [ ] Add crate-private `darkmatter/lib/src/render/metadata_codec.rs` and declare it in
  `render/mod.rs`; move base64, generic serde encode/decode, and `MetadataPolicy` into it, with the
  caller supplying `LINK_METADATA` or `IMAGE_REF_METADATA`.
- [ ] Replace the local helper copies in `link.rs` and `image_ref.rs` with the shared APIs while
  retaining caller-owned `LinkError`/`ImageRefError` construction and all genuinely distinct
  link/image behavior.
- [ ] Remove the duplicate helper implementations and add focused regression assertions proving
  multibyte title text is no longer corrupted or skipped; treat this UTF-8 correction as an
  explicit behavior change, separate from the mechanical extraction.

### Phase 2 validation checkpoint

- [ ] Run focused link/image unit and snapshot targets with nextest, then run `cargo check -p
  darkmatter`, `just test`, `just lint`, and `just test-l2` from `darkmatter/`.
- [ ] Require all pre-existing snapshots and status blocks to remain byte-identical, with changes
  limited to the new UTF-8 regression expectations, and confirm both metadata environment
  variables still select the same modes independently.

## Phase 3 — Relocate test ballast without changing behavior

### Standalone inline-suite extraction

- [ ] Record the exact pre-move nextest names and counts for the `layout::page` and
  `frontmatter_shell_expansion` unit suites; retain the Phase 1 full-crate inventory for the final
  comparison.
- [ ] Move the inline `layout/page.rs` test module to `layout/page/tests.rs` (or an equivalent
  explicit path) and replace it with only `#[cfg(test)] mod tests;`, preserving private-item access,
  test names, snapshots, and assertions.
- [ ] Move the inline `compose/frontmatter_shell_expansion.rs` test module to a sibling test file
  and replace it with only its `#[cfg(test)]` declaration, preserving serial annotations,
  environment guards, ignored flags, temporary-directory lifetimes, and all assertions.
- [ ] Compare the two post-move inventories to their baselines by name and count, then run each
  moved suite directly with nextest.

### Compose unit-test tree

- [ ] Classify every test in `compose/tests.rs` into `frontmatter`, `schema`, `shell`,
  `transclusion`, `caching`, `preflight`, or `rendering`, recording the original test name exactly
  once before moving it.
- [ ] Replace `compose/tests.rs` with a `compose/tests/mod.rs` tree containing those seven domain
  modules plus a small `fixtures.rs`; extract only genuinely shared temp-repository builders and
  give fixtures the narrowest working `pub(super)` visibility.
- [ ] Preserve test names where feasible and preserve every assertion, `#[serial]` group, ignore
  marker, environment guard, cache/security setup, and temporary-resource lifetime; do not mix
  production edits into this relocation.
- [ ] Compare the post-split compose inventory with the recorded names and counts and run the
  compose test modules directly with nextest.

### Level-2 integration target tree

- [ ] Inventory `level2_render_tree_terminal` tests by name, count, test-level requirement, harness
  dependency, and helper usage before moving them.
- [ ] Keep `lib/tests/level2_render_tree_terminal.rs` as a thin integration-target module root and
  split tests below `lib/tests/level2_render_tree_terminal/` into code panel, images, file links,
  layout policy/page geometry, public entry points, and basic spans modules using explicit `#[path]`
  declarations where Rust integration-target resolution requires them.
- [ ] Create a target-local `support/` tree for harness, render-probe, ANSI, and image helpers;
  preserve executable discovery, real-terminal semantics, and
  `BISCUIT_TEST_LEVEL_REQUIRED` behavior without introducing a workspace-wide abstraction.
- [ ] Compare the target's post-move nextest inventory by exact name and count and run
  `cargo nextest run -p darkmatter --test level2_render_tree_terminal`.

### Phase 3 validation checkpoint

- [ ] Review the `src/` diff and require production changes to be limited to test-module
  declarations and removal of moved test bodies; require zero assertion edits and zero missing or
  duplicated tests.
- [ ] Run `cargo check -p darkmatter`, `just test`, `just lint`, and `just test-l2` from
  `darkmatter/`; compare the full Darkmatter nextest inventory with the Phase 1 baseline.

## Phase 4 — Split cleanup into an explicit two-stage pass pipeline

This phase is parallelizable with Phase 5 after Phase 3 is complete.

### Cleanup module extraction

- [ ] Characterize the current public cleanup entry points, re-exports, exact Phase A/Phase B pass
  order, emphasis-placeholder state, list-marker state, and the DMLS formatting parity test before
  converting `markdown/cleanup.rs` into `markdown/cleanup/`.
- [ ] Create `cleanup/mod.rs` as the source-compatible public facade and keep
  `cleanup_content_internal` there as one plainly readable orchestrator that explicitly invokes
  every pass in the existing order.
- [ ] Move emphasis preservation/restoration and related unescaping to `cleanup/emphasis.rs`,
  keeping the coupled placeholder lifecycle within that module.
- [ ] Move table stream alignment, `CellWidthCalculator`, and single-table processing to
  `cleanup/tables.rs`.
- [ ] Move list-marker extraction/restoration, spacing normalization, and indentation correction to
  `cleanup/lists.rs`; move blockquote and bracket passes to `cleanup/blockquote.rs` and
  `cleanup/brackets.rs` respectively.
- [ ] Move incidental-newline stripping, fixed-width reflow, wrapping, prefix/line metadata, and
  HTML-block state to `cleanup/reflow.rs` without changing display-column, code-protection, line
  ending, or trailing-newline behavior.
- [ ] Move pass-specific characterization tests beside their owning modules and retain
  ordering-dependent end-to-end tests under `cleanup/tests/` for emphasis through reflow, list
  markers in blockquotes, tables adjacent to lists, fenced/indented code, HTML blocks, CRLF,
  Unicode display width, and trailing-newline modes.
- [ ] Prove `markdown::cleanup` and every existing cleanup re-export/import path remains
  source-compatible; do not introduce a pass trait or implicit pass chaining.

### Phase 4 validation checkpoint

- [ ] Compare cleanup test names and counts before and after the move, run the cleanup tests with
  nextest, and run the DMLS formatting parity test to prove byte-equivalence with `md clean`.
- [ ] Run `cargo check -p darkmatter -p dmls`, `just test`, and `just lint` from `darkmatter/`;
  require no cleanup output or snapshot changes.

## Phase 5 — Split demand-driven context capture by capture group

This phase is parallelizable with Phase 4 after Phase 3 is complete.

### Capture facade and group ownership

- [ ] Characterize `ContextGroup::all`, `ContextGroup::for_key`,
  `scan_needed_groups`, `ContextCapture::new`, population order, sniff probe dependencies,
  diagnostics/timing labels, aliases, and existing tests before converting `context/capture.rs` to
  `context/capture/`.
- [ ] Create `capture/mod.rs` as the crate-private facade for group selection and population
  sequencing; preserve the existing capture entry points and always-on local datetime behavior.
- [ ] Move `ContextGroup`, `all`, demand scanning, and `for_key` delegation to `capture/groups.rs`;
  have each domain own a `KEYS` slice or equivalent `owns_key` predicate instead of repeating a
  central key match.
- [ ] Move `ContextCapture::new` and concurrent sniff orchestration to `capture/snapshot.rs`,
  preserving repo-before-document discovery, overlap of independent probes, stable diagnostic and
  timing labels, and the rule that unrequested probes do not run.
- [ ] Move population code and owned keys into `datetime.rs`, `repo.rs`, `changes.rs`,
  `languages.rs`, `docs.rs`, `host.rs`, and `agent.rs` according to the specification; retain
  `sniff` as the authority for repository, filesystem, OS, hardware, and GPU discovery on macOS,
  Windows, and Linux.
- [ ] Separate `populate_gpu` from hardware population and invoke it for `ContextGroup::Gpu` so a
  `ctx.gpu`-only request inserts the GPU value without forcing CPU/memory capture.

### Context invariants and regression coverage

- [ ] Add a host-independent GPU-only regression test using injected or constructed capture data;
  assert the GPU value is populated and hardware probing is not required.
- [ ] Add invariant tests proving every generated context descriptor maps to exactly one group,
  documented backward-compatible aliases are handled by an explicit allowlist, and unknown keys
  map to no group.
- [ ] Add or retain tests proving content with no relevant `ctx.*` reference performs only the
  documented datetime work and each requested group triggers only its dependency-minimal probe
  set.
- [ ] Move existing tests to their owning capture modules without changing names or assertions and
  compare the context-capture nextest inventory before and after.

### Phase 5 validation checkpoint

- [ ] Run focused context-capture tests with nextest, then run `cargo check -p darkmatter`,
  `just test`, and `just lint` from `darkmatter/`; require stable diagnostics/timing labels and no
  behavior change beyond GPU-only capture.

## Phase 6 — Atomically migrate to domain-owned expression registrations

This phase depends on both Phases 4 and 5 and must land atomically across Darkmatter, DMLS,
Claudine library, and Claudine CLI. Do not leave old and new catalog authorities side by side.

### Registration model and domain split

- [ ] Inventory every pure, context-aware, and lazy callable; canonical name; alias; overload;
  descriptor; handler; evaluator path; and workspace consumer of
  `EXPRESSION_FUNCTION_DESCRIPTORS`, `PURE_FUNCTIONS`, and `FS_FUNCTIONS`.
- [ ] Define crate-private `FunctionRegistration` and `FunctionHandler::{Pure, Context, Lazy}` in
  `compose/expression/functions/mod.rs`, keeping `ExpressionFunctionDescriptor` public and
  handler-free in `catalog.rs`.
- [ ] Split handlers and their directly owned tests into `functions/args.rs`, `predicates.rs`,
  `collections.rs`, `strings.rs`, `dates.rs`, `paths.rs`, `skills.rs`, `markdown_docs.rs`, and
  `terminal.rs`; use explicit imports and keep shared helpers private to `functions` unless an
  existing public path requires otherwise.
- [ ] Define one const registration slice per domain beside its handlers, representing overloads
  as multiple descriptors on one registration and representing aliases only on that registration.
- [ ] Aggregate domain registrations in `functions/mod.rs` and derive runtime dispatch,
  context-aware dispatch, lazy dispatch metadata, canonical-name enumeration, callable signatures,
  and catalog projection from that aggregation.
- [ ] Back `expression_function_descriptors() -> &'static [ExpressionFunctionDescriptor]` with one
  `LazyLock<Vec<_>>`; remove the public raw `EXPRESSION_FUNCTION_DESCRIPTORS`, `PURE_FUNCTIONS`,
  and `FS_FUNCTIONS` constants without introducing compatibility constants of changed types.
- [ ] Update Darkmatter evaluator errors/suggestions, catalog helpers,
  `darkmatter/lib/src/catalog/mod.rs`, generated documentation inputs, and internal comments to use
  the accessor or registration projection as appropriate.

### Workspace-wide consumer migration

- [ ] Update DMLS hover, completion, and lookup code in
  `darkmatter/dmls/src/overlay/expressions.rs` to consume
  `expression_function_descriptors()` while preserving result ordering and content.
- [ ] Update Claudine library and CLI source/tests—including lifecycle-action expression
  validation and context command output—to consume the accessor and preserve their public output.
- [ ] Update active Darkmatter and Claudine expression architecture, engine, drift, and authoring
  documentation to describe domain-owned registrations and the accessor as the sole public catalog
  API; leave historical completed specifications intact unless they are presented as current
  guidance.
- [ ] Update `.claude/skills/darkmatter/SKILL.md` and its expression reference material so future
  work registers a callable in one domain slice and reads descriptors through
  `expression_function_descriptors()`.

### Registry invariants and behavior coverage

- [ ] Add invariant tests proving canonical names are unique; aliases collide with no canonical or
  alias; every registration has descriptors; every descriptor signature begins with its
  registration's canonical name; descriptor signatures are unique; overloads share one handler;
  and Pure, Context, and Lazy handlers dispatch through the intended path.
- [ ] Preserve and run behavior tests for aliases, arity, local-only versus remote path rules,
  injectable date behavior, and lazy `and`/`or` evaluation.
- [ ] Regenerate expression-function documentation with `just regen-expr-doc` from `darkmatter/`
  and review the diff for expected structural wording only; require the callable set, signatures,
  ordering, descriptions, and examples to remain unchanged.

### Phase 6 validation checkpoint

- [ ] Run checks and nextest suites for `darkmatter`, `dmls`, `claudine`, and `claudine-cli`, then
  run `just test` and `just lint` from `darkmatter/` and the canonical test/lint recipes from
  `claudine/`.
- [ ] Search active source and docs for the three removed constant names and require zero remaining
  consumers; permit occurrences only in historical specifications, reviews, or an explicit
  migration note.

## Phase 7 — Integration, cross-platform review, and closeout

- [ ] Compare the final `cargo nextest list -p darkmatter` and
  `level2_render_tree_terminal` inventories with the Phase 1 records; account explicitly for every
  added regression test and require every pre-existing test name/count to remain represented.
- [ ] Review the complete diff for scope: no generic pass trait, plugin framework, direct
  platform-command discovery, production behavior change beyond the two fixes and catalog API
  migration, assertion weakening, test-gate drift, or newly created god-file.
- [ ] Review all touched comments, rustdoc, READMEs, active architecture docs, and the Darkmatter
  skill for stale ownership or pipeline claims; update only claims made stale by this work.
- [ ] Audit path handling, module declarations, fixture paths, environment-variable use, and sniff
  integrations for macOS, Windows, and Linux compatibility; ensure no Unix-only separators,
  commands, or filesystem assumptions entered production or tests.
- [ ] Run final compilation:
  `cargo check -p darkmatter -p darkmatter-cli -p dmls -p claudine -p claudine-cli`.
- [ ] Run final Darkmatter package-area gates from `darkmatter/`: `just test`, `just test-l2`, and
  `just lint`; run the canonical Claudine checks/tests required by its area recipes.
- [ ] Run a repository-wide search proving the deleted transform APIs and removed expression
  constants survive only in historical specifications/reviews or explicit migration notes.
- [ ] Confirm the final diff preserves rendering bytes/snapshots, compose pass order,
  error/status-block text, cache and shell-security behavior, demand-driven capture, and test-level
  gating, with only the UTF-8 title fix, GPU-only capture fix, and documented catalog API break
  called out for review.
