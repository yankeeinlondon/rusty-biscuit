# `more-is-more` into `darkmatter`: Conflict and Merge-Safety Report

## Executive Summary

The merge is large but not uniformly conflict-heavy. At the pinned heads used for
this report:

- merge base: `d672388dd0fed4196295e7f21514cac6fa59f0ae`;
- `darkmatter`: `14dd391f45206d58383ba9d84adbf53c65520534`;
- `more-is-more`: `0584d8297f57f5eb30b52d03b1241ba55184bb44`;
- divergence after the merge base: 82 commits on `darkmatter` and 100 commits on
  `more-is-more`;
- net branch deltas: 152 files on `darkmatter` and 201 files on
  `more-is-more`; and
- only ten paths have net changes on both branches.

A three-way `git merge-tree` preview predicts five textual conflicts and one
modify/delete conflict. The more important risk is semantic: three production
files auto-merge even though they join independently developed behavior in the
compose and schema pipelines. The merge must therefore be treated as an
integration exercise, not as a conflict-marker cleanup.

The recommended strategy is to merge the pinned `more-is-more` head into a new,
clean integration branch based on the pinned `darkmatter` head; resolve the six
predicted conflicts deliberately; audit every automatically merged shared path;
then verify feature acceptance criteria by package area in a serial,
CPU-bounded sequence. Do not run workspace-wide Cargo gates or concurrent gates
from the two source worktrees.

Two bodies of work still have documented evidence gaps that the merge cannot
legitimately erase:

- `2026-07-15-performance-followup` still needs its two admissible quiet-host
  integrated compose-regression captures before acceptance criteria 5 and 6 can
  be closed.
- `2026-07-14-invalid-frontmatter` still needs its admissible common-case timing
  bracket and native Linux/Windows runtime evidence. Its latest review found no
  remaining functional defect, so these are evidence gates rather than a reason
  to alter the implementation during conflict resolution.

## Evidence and Scope

This report uses the two authored branch logs as the primary history summaries,
then checks their claims against the pinned refs with `git merge-base`,
`git diff`, and a three-way `git merge-tree` preview. Repository package
discovery was performed with `sniff repo packages`; the relevant package scope
is:

- Darkmatter area: `darkmatter`, `darkmatter-cli`, and `dmls`;
- Sniff area: `sniff` and `sniff-cli`;
- Biscuit File area: `biscuit-file` and, for public-surface compilation,
  `biscuit-file-cli`;
- Claudine area: `claudine` and `claudine-cli`;
- Biscuit Terminal area: `biscuit-terminal` and its CLI where the area recipes
  include it; and
- repository CI/workflow, shared-schema, skill, prompt, and commit-guidance
  files.

`biscuit-terminal` has no incoming post-fork change from `more-is-more`.
Nevertheless, it remains acceptance scope because the shared pre-fork
performance feature changed terminal discovery/caching and the divergent
branches both touched Darkmatter's real-terminal test integration.

## Predicted Mechanical Conflicts

### 1. `.claude/skills/darkmatter/SKILL.md`

Both branches update the authoritative Darkmatter skill. The textual conflict
is currently limited to the frontmatter `hash` and `last_updated`, but the bodies
contain independent, valid knowledge:

- `darkmatter` documents the corrected cleanup/list behavior, fresh versus
  checked reference-graph validation, invalid-frontmatter analysis, and the
  later package architecture; and
- `more-is-more` documents Git context capture, conflict prediction, expression
  literals/functions, remote providers, and meta-schema semantics.

Resolution must retain both bodies. Neither branch's hash represents the merged
content, so choosing either hash is incorrect. After resolving the body, update
`last_updated` and recompute the Markdown-aware hash with Darkmatter. Read the
merged skill end-to-end afterward to remove duplicated or contradictory
authority statements.

### 2. `.claudine/memory/commits.md`

The conflict places two independent safety rules at the same insertion point:

- `darkmatter` adds non-interactive signing/pinentry precautions; and
- `more-is-more` prohibits bypassing repository hooks.

Keep both. They are complementary. Consolidate them as separate bullets without
weakening either rule, and retain the incoming `--only` plus `-F -` argument-order
guidance that auto-merges elsewhere in the file.

### 3. `CLAUDE.md`

The conflict is a generated GitNexus symbol/relationship count. Neither side's
count will describe the merged tree. Resolve the code and documentation first,
refresh the GitNexus index once, and record the post-merge counts. Do not select
one branch's stale count merely to clear the marker.

The current `darkmatter` worktree already has an unrelated modification to
`CLAUDE.md`; preserve and identify that change before creating the integration
worktree or attempting any merge.

### 4. `darkmatter/cli/tests/level2_code_block_styling.rs`

This is the most substantial textual conflict. `more-is-more` contains a local
shared-tmux harness, sentinel loop, fixture writer, and `run_md_in_tmux` helper.
The `darkmatter` branch moved this responsibility into
`darkmatter/cli/tests/common/level2.rs` and imports the shared helper, as part of
the fixed-width-list/L2 harness correction.

Use the `darkmatter` structural direction: keep the centralized common helper
and do not reintroduce the duplicated local harness. Port any incoming test
cases or behavior that are not already represented, especially the Cargo build
shim and terminal-discovery coverage. The resolved file should have one harness
authority and must preserve serialization of real-terminal tests.

### 5. `darkmatter/cli/tests/level2_errors.rs`

The conflict is a duplicate placement of the `md_shim` import. Keep one import
in the surrounding file's canonical ordering. Confirm that the merged tests use
the current shared harness and build shim rather than an obsolete local path.

### 6. `darkmatter/features/2026-07-15-performance-followup/review-8.md`

This is a modify/delete conflict and must be resolved in favor of retaining the
`darkmatter` version.

`more-is-more` deleted Review 8 when Review 7 was temporarily treated as the
trailing review. The `darkmatter` branch subsequently completed Review 8's
implementation record and added Reviews 9 and 10, both of which explicitly
refer to Review 8. Accepting the deletion would break the audit chain and erase
the record of the deferred quiet-host performance gate.

Retain `review-8.md`. Also audit the automatically merged
`review-7.md`: the incoming branch removes its `next: review-8.md` pointer, but
the final merged history requires that pointer to be restored. Verify the final
chain as Review 7 -> Review 8 -> Review 9 -> Review 10 and preserve the
not-production-ready evidence status.

## Shared Paths That Auto-Merge but Require Manual Audit

### `darkmatter/cli/src/commands/compose.rs`

The changes are textually independent: `darkmatter` exposes
`env_disables_baseline_schema` to the clean command, while `more-is-more`
removes obsolete approval-error bindings. The merged result should contain both.
The semantic check is that compose and clean still share exactly the intended
baseline-disable rule and that provider/approval errors keep their focused
classification and rendering.

### `darkmatter/lib/Cargo.toml`

The merged manifest must contain all three independent changes:

- `sniff` with the `remote` feature for provider functionality;
- the `git2` dev-dependency used only as the merge-prediction test oracle; and
- the `clean_hot_paths` benchmark target.

Run `cargo metadata --no-deps --format-version 1` after resolution. Confirm that
`git2` remains a dev-only oracle and is not introduced into production, where
Sniff's pure-Rust Git implementation is authoritative.

### `darkmatter/lib/src/markdown/schemas/mod.rs`

This path has the highest silent-merge risk. The `darkmatter` branch extracts a
large inline test module and adds the invalid-frontmatter `clean` facade,
`effective_for_with_override`, raw validation, and ordered problem codes. The
`more-is-more` branch adds meta-schema references and a much wider set of
source-aware SimplifiedSchema exports.

The final module must preserve all of the following:

- the extracted test-module layout rather than restoring the removed god-file;
- the public `clean` analysis exports and schema override/raw-validation seams;
- `SchemaReference` classification and bounded reference resolution;
- all source-aware cursor, declaration, value, span, and parser exports needed
  by DMLS; and
- the `PartialOrd`/`Ord` behavior used for deterministic clean diagnostics.

Compilation alone is insufficient: exercise clean schema overrides, meta-schema
references/cycles, DMLS source projection, and deterministic repair diagnostics.

### `darkmatter/lib/src/markdown/schemas/validate.rs`

The `darkmatter` branch widens two helpers to `pub(super)` for schema-clean
analysis. The `more-is-more` branch registers the `type-definition` and `schema`
custom validators. The merged validator builder must retain the URL-scheme
keyword and both new semantic keywords, while the clean module retains access
to nullable-arm helpers. Test raw and coercing validation paths separately so a
schema-proven clean repair is not hidden by coercion.

### Auto-merged tests and metadata

The remaining overlapping paths still deserve explicit review:

- `darkmatter/cli/tests/level2_code_block_styling.rs` and
  `level2_errors.rs` after manual resolution;
- `.claude/skills/darkmatter/SKILL.md` after hash regeneration;
- `.claudine/memory/commits.md` for policy preservation; and
- `CLAUDE.md` after the single post-merge GitNexus refresh.

## Functional Changes Outside the Darkmatter Package Area

### Sniff and Sniff CLI

The largest incoming non-Darkmatter implementation is in Sniff:

- trusted Git repository opening, status, worktree, remote, and branch
  observation;
- a read-only, in-memory branch merge-conflict predictor with parity fixtures
  and no worktree/index/ref/object-database mutation;
- preferred-remote resolution;
- provider/flavor discovery for GitHub, GitLab, Gitea/Forgejo, Bitbucket, Azure
  DevOps, and self-managed variants;
- provider-aware, host-bound credential handling;
- canonical provider URL and web-link normalization;
- exact and paginated pull-request and CI/CD job models, filters, capabilities,
  bounded traversal, and typed errors; and
- Sniff CLI snapshots, test recipes, documentation, and cross-platform CI
  coverage.

The earlier shared performance work also changed Sniff's timezone API boundary:
the public bare API retains full timezone/NTP behavior while Darkmatter uses the
no-NTP seam. The post-fork incoming work must not regress that compatibility
fix.

### Biscuit File

The `darkmatter` branch adds the reusable, schema-agnostic invalid-YAML analysis
and repair foundation to `biscuit-file`:

- a shared byte-offset `SourceSpan` vocabulary;
- source-retaining YAML parsing and structured locations;
- deterministic diagnostics and non-overlapping UTF-8-safe edit sets;
- normalization, whitespace cleanup, bounded reserved-indicator recovery, and
  report-only duplicate/anchor/multi-document/lint detection;
- parse-count, safety, mutation/property, and pinned YAML Test Suite coverage;
  and
- public API and README documentation.

This work is darkmatter-only, so it should enter the merge unchanged. Its risk
is downstream integration with the incoming meta-schema parser and validator,
not a direct file conflict in `biscuit-file`.

### Claudine and Claudine CLI

The incoming branch updates Claudine to understand the expanded expression and
schema values used in composed lifecycle inputs:

- traversal and validation of array/object expression containers;
- classification and structural rendering of semantic `schema` and
  `type-definition` values;
- lifecycle execution/preflight/looping compatibility; and
- CLI context formatting for the new nominal schema types.

These are production downstream consumers, not incidental test changes. They
must be included in the build/test/lint and cross-platform compile scope.

### Biscuit Terminal

The shared pre-fork performance feature changed terminal OSC-query discovery
and caching, tracing-based query-count evidence, and real-emulator tests. No
`biscuit-terminal` file changed on `more-is-more` after the fork, but the merged
Darkmatter L2 harness must continue to prove single terminal discovery and
stable code-block rendering against that shared implementation.

### Repository-wide support files

The incoming branch also changes root CI workflows, testing strategy, feature
review schemas, review-workflow prompts, Rust/lessons skills, and commit
guidance. These files carry real process behavior. Review workflow changes for
duplicate jobs or reduced OS coverage, and validate the modified review schemas
with the merged meta-schema implementation.

## Recommended Merge Procedure

### Phase 0: Freeze and protect the inputs

1. Stop all Cargo/Nextest jobs in every related worktree. Assign one validation
   owner for the duration of the integration.
2. Confirm the two branch heads and merge base still equal the pinned SHAs in
   this report. If either head moves, regenerate the overlap and merge-tree
   preview before proceeding.
3. Make both source worktrees clean, or explicitly preserve every unrelated
   change. At report time the `darkmatter` worktree contains a modified
   `CLAUDE.md`, untracked `.claude/settings.local.json`, and the untracked merge
   report directory. Do not let any of these enter a merge commit accidentally.
4. Create lightweight backup refs for the two pinned heads.
5. Create a dedicated clean integration worktree and branch from the pinned
   `darkmatter` head. Do not perform the first attempt directly on either source
   branch.

### Phase 1: Perform and resolve the merge

1. Merge the pinned `more-is-more` commit without committing immediately.
2. Resolve the six predicted conflicts according to this report.
3. Review all ten shared paths, including the four production auto-merges.
4. Review directory-level unions around these convergence points even when Git
   reports no overlap:
   - `markdown/compose/context` and `markdown/compose/expression`;
   - `markdown/schemas`, especially `clean`, `simplified`, `reference`,
     `resolve`, `format`, and `validate`;
   - DMLS frontmatter/expression overlays and providers;
   - `sniff::filesystem::git` and `sniff::remote`; and
   - Darkmatter CLI clean/compose command plumbing.
5. Search the entire worktree for unresolved conflict markers and run
   `git diff --check`.
6. Run `cargo metadata --no-deps --format-version 1` and inspect the affected
   manifests before starting compilation.
7. Refresh GitNexus only after the source tree is resolved. Run change detection
   against `main` before the final commit and confirm that affected symbols and
   execution flows match the recorded package scope.

Do not resolve production conflicts by taking whole-file `ours` or `theirs`.
The correct result is additive at the authority boundaries: Darkmatter's later
cleanup/reference/repair corrections plus More-Is-More's expression/provider/
meta-schema additions.

### Phase 2: Cheap, deterministic verification first

Before broad area recipes, run focused tests that compile and exercise the
convergence seams:

- schema module exports, validator construction, schema references/cycles, and
  meta-schema parse/source-projection tests;
- invalid-frontmatter clean analysis, raw versus coercing validation, JSON
  envelope, delimiter/span preservation, and schema flags;
- compose context capture, structured literals, indexed-file endpoints,
  provider error fatality, and remote-runtime propagation;
- reference fresh/checked validation and changed-child behavior;
- fixed-width/default/preserve cleanup, opaque directives, CLI/DMLS parity,
  and idempotency;
- Sniff Git parity, conflict prediction, remote resolution/observation, focused
  provider, and credential-isolation tests;
- DMLS no-side-effects, completion/hover, diagnostics, document links, and LSP
  session tests; and
- Claudine container traversal and semantic-schema lifecycle tests.

This ordering localizes failures before expensive L2 or whole-area recipes.

### Phase 3: Area gates without CPU contention

Run one package area at a time. For each affected area, use its `just build`,
`just test`, and `just lint` recipes, with exact package selectors where the
recipe supports them. Do not run bare workspace Cargo commands or an unscoped
root `just` lifecycle recipe.

Set conservative process-wide limits, for example `CARGO_BUILD_JOBS` and
`NEXTEST_TEST_THREADS`, based on an agreed host budget. Keep those limits
consistent for all areas and do not start a second worktree's build while one
is active. A high-spec host can still be saturated by several independent Rust
link/codegen workloads, and concurrent worktrees make benchmark evidence
inadmissible even when functional tests eventually pass.

Suggested sequence, chosen to fail near the changed authorities:

1. `biscuit-file` area;
2. `sniff` area;
3. Darkmatter area (`darkmatter`, `darkmatter-cli`, `dmls`);
4. Claudine area;
5. Biscuit Terminal area and Darkmatter's real-terminal L2 gates; and
6. cross-platform CI compile/test jobs for the recorded scope.

Run Darkmatter's L2 and browser recipes only when their requirements apply;
retain the real-terminal serialization already built into the tests. Do not
run L2 terminal suites from multiple worktrees concurrently.

### Phase 4: Performance and cross-platform evidence

Functional gates and benchmark gates must be kept separate.

- Rerun deterministic mechanism/performance guards for redundant-walk and
  fixed-width cleanup after functional gates pass.
- Run invalid-frontmatter common-case timing only on an admissible host and
  retain the baseline/candidate artifacts.
- Run the performance-followup integrated compose suite only under its
  committed quiet-host contract. If the one-minute load average exceeds 2.0,
  decline the capture and record that fact; do not replace the missing evidence
  with a noisy run.
- Obtain native Linux and Windows runtime evidence for invalid-frontmatter and
  retain the already required macOS/Linux/Windows compilation and behavior
  evidence for More-Is-More, Meta-Schema, Sniff, and Claudine.

## Acceptance-Criteria Assurance Matrix

### `2026-07-13-more-is-more`

Assure all 30 criteria through the following grouped evidence:

- **Git context and safety (AC 1-16):** schema/catalog assertions; demand-driven
  capture counters; detached/unborn/bare/non-repository cases; git2 and Git
  parity fixtures; direction-sensitive merge cases; dirty/corrupt-index
  independence; caller-anchor checks; and before/after snapshots proving no
  HEAD/ref/index/worktree/object mutation.
- **Local expression language (AC 17-18):** indexed-family edge cases and
  executable catalog examples; immutable nested object/array literals,
  computed values, spans, duplicate-key rejection, and postfix-indexing
  regressions.
- **Remote/provider behavior (AC 19-30):** preferred-remote selection, branch
  existence, vendor discovery, exact/list PR and CI/CD queries, filter and bound
  enforcement, capability/version errors, provider-aware credential isolation,
  CommonMark-safe output, deny-by-default host policy, one run-wide executor,
  frontmatter/body/`$()` parity, DMLS catalog parity, Wiremock-only provider
  tests, and cross-platform gates.

The DMLS no-side-effects suite is mandatory because editor intelligence must
remain passive despite the merged network-capable runtime.

### `2026-07-13-meta-schema`

Map the 13 criteria to parser/serializer/descriptor tests for both nominal
types; valid and invalid property/schema declaration matrices; portable
carrier/custom-keyword JSON Schema assertions; array postfix behavior;
source-aware parser parity across quotes, CRLF, UTF-8, mappings, sequences,
aliases, unions, and explicit mapping pairs; base `$schema: schema` behavior;
DMLS activation/completion/hover/diagnostic and last-good recovery tests;
no-side-effects tests; shared recursion-limit tests; byte-compatibility baseline
replay; and Darkmatter L1/L2 plus three-OS compilation.

Pay particular attention to the merged `schemas/mod.rs` exports and
`validate.rs` keyword registration: these are where an apparently successful
auto-merge could invalidate AC 1, 3, 5, 7, 8, or 9.

### `2026-07-15-performance-followup`

Reconfirm the audit table's final disposition for all 35 findings, directory
hash membership, graph-feature ownership, fixture hashes, option-identity
classification, terminal OSC caching, single compose terminal discovery,
shell ordering/timeouts, replacement/interpolation/remote-discovery behavior,
render/hash compatibility, and the retained Linux/Windows behavioral evidence.

The existing raw evidence must not be rewritten merely because a merge
occurred. Acceptance criteria 5 and 6 remain open until the required two
quiet-host captures pass the committed drift/load/threshold contract. The final
merge report and review chain must state that honestly.

### `2026-07-16-redundant-walk`

Run focused reference tests proving:

- fresh validation and `FileTree::ensure_built` skip compatibility/dependency
  verification;
- caller-supplied graphs remain fail-closed;
- both paths share one validation engine;
- changed-child behavior distinguishes fresh snapshot use from stale-graph
  rejection;
- `PreparedHeadingSnapshot` keeps fragment validation coherent;
- public errors/reports/CLI/JSON remain unchanged; and
- the recorded mechanism, at-least-100-microsecond improvement, no-regression,
  and prebuilt-gap guards remain satisfied.

Then run the scoped Darkmatter build/test/lint and GitNexus change audit.

### `2026-07-13-fixed-width-lists`

Run the acceptance matrix across default, preserve, and fixed-width modes for
ordered/unordered/task/nested/blockquoted lists; marker-width changes;
configured indentation; Unicode display widths; atomic overflows; paragraph,
sibling, nested-list, hard-break, code, table, HTML, and directive boundaries;
opaque mixed shell/page blocks; list-spacing modes; idempotency; parse counts;
and structural fingerprints.

Prove byte/structure parity across direct library cleanup, compose inline-post,
CLI stdout, CLI `--save`, and DMLS formatting. The resolved L2 harness must not
reintroduce duplicated tmux machinery. Run the Darkmatter area build/L1/L2/lint
gates. The deferred noisy-host Criterion vector may be captured later, as its
final review classifies it as non-blocking, but no performance claim should be
invented in its absence.

### `2026-07-14-invalid-frontmatter`

Use the ratified acceptance matrix rather than the early spec's placeholder
definition of done:

- **A/S1-S4:** normalization, whitespace, bounded reserved-indicator quoting,
  schema-proven quoting, and safe combined edits;
- **B:** duplicate/anchor/multi-document/lint/schema suggestions remain
  report-only and never mutate;
- **C:** value/schema safety proofs, untouched bytes, precise authored spans,
  retained source/no TOCTOU reread, shared spans, and parse-count invariants;
- **D:** file/stdin/`--save`/verbose/JSON/human error modes, zero-work bypass,
  byte idempotency, schema flags/triggers, and fenced-YAML-body protection;
- **E:** LF, CRLF, lone CR, BOM, UTF-8, Windows paths, and final-newline forms;
  and
- **F:** pinned YAML Test Suite corpus, mutation/property invariants,
  no-frontmatter zero-work, clean-frontmatter parse-once, common-case timing,
  and macOS/Linux/Windows gates.

The schema-clean tests must be run after Meta-Schema tests because both features
meet in schema preparation and validation. Preserve the stable version-1 JSON
envelope as the sole stdout payload. Keep the outstanding timing and native
Linux/Windows runtime evidence visible until captured.

## Additional Risk Reduction Before the Merge

- Pin the three SHAs in a short merge checklist and regenerate this report's
  preview if a ref moves.
- Preserve a patch or commit for the current unrelated dirty files before
  creating the integration worktree.
- Ensure no Cargo, rustc, linker, Nextest, Criterion, or real-terminal harness
  process is running in any related worktree.
- Use a single integration worktree, a single validation owner, bounded Cargo
  jobs, bounded Nextest threads, and serial package-area gates.
- Capture the current feature fixture hashes and retained benchmark artifact
  identities before the merge; compare them afterward so conflict resolution
  cannot silently rewrite evidence.
- Record the ten shared paths and six expected conflicts in the merge checklist;
  investigate any additional conflict rather than treating it as routine.
- Review the final history documents for broken `previous`/`next` links,
  especially performance Reviews 7-10.
- Recompute generated metadata only once from the resolved tree: Darkmatter
  skill hash, GitNexus counts/index, and any snapshots whose semantic output
  intentionally changes.
- Do not update snapshots wholesale. Inspect each delta and associate it with a
  named acceptance criterion.
- Before committing, run GitNexus change detection against `main`, inspect
  `git status` and the staged diff from the integration worktree, and verify
  that no source-worktree-local settings or unrelated edits are staged.

## Merge Completion Definition

The merge is complete only when all of the following are true:

1. all six predicted conflicts and all ten shared paths have documented
   resolutions;
2. no conflict marker or broken review-chain link remains;
3. the merged compose, clean, schema, expression, provider, reference, and DMLS
   authority boundaries each have focused passing tests;
4. affected package-area build/test/lint gates pass without an unscoped
   workspace run;
5. macOS, Linux, and Windows evidence exists where each feature requires it;
6. the More-Is-More, Meta-Schema, redundant-walk, and fixed-width functional
   acceptance criteria remain green;
7. invalid-frontmatter functional criteria remain green and its outstanding
   performance/platform evidence is either completed or explicitly carried
   forward;
8. performance-followup's quiet-host evidence is completed or explicitly
   remains an open production-readiness gate; and
9. GitNexus change detection and the final staged diff show only the intended
   integrated scope.

Passing compilation is necessary but not sufficient. The decisive evidence is
that every feature's original behavioral, safety, performance, passive-analysis,
and cross-platform contract survives in the combined tree.
