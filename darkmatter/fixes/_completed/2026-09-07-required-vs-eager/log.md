---
implementation_2: "2026-09-07T12:57:57-07:00"
implementation_3: "2026-09-07T15:40:48-07:00"
---

## Implementation of Review Findings #2

> **started at:** 2026-09-07T12:57:57-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/darkmatter/fixes/2026-09-07-required-vs-eager/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'Finding 1 — Claudine inline prompt eager-vs-required' at 12:59:16
        - GitNexus blast radius for `def_is_required_at_completion` (upstream, `claudine/lib/src/composition/inline_prompt.rs`): at the default depth 3 the radius is 7 symbols / LOW / 1 module (Composition); at depth 6 it is **53 symbols, risk CRITICAL**, 1 direct caller (`property_table`), 0 named execution flows, and 8 modules — Service (17), Composition (16), Schema (6), Harness_orch (4), Compose (4), Looping (2), Tests (2), Prepare (1). The change is deliberate and spec-mandated, so it proceeded after reporting; the affected paths are covered by the `just test claudine` gate below.
        - Darkmatter agrees with the "any arm" union rule this helper already used, so it was kept: `simplified/convert.rs:140` hoists a property-level `required` when `arms.iter().any(PropertyAtom::is_required)`, and `PropertyAtom::is_required` (`simplified/types.rs:254`) reads `property_constraints()` — item constraints chained with postfix `array_constraints`. `phase::project_atom` at `SchemaPhase::Completion` sets `phase_required = authored_required` from the same chained pair and never consults `Constraint::Eager`. The helper now mirrors exactly that: only `Constraint::Required`, item + array constraints, any union arm. `PropertyAtom::is_required` is `pub(crate)` to Darkmatter, so Claudine cannot call it directly.
        - production change — `def_is_required_at_completion` (`claudine/lib/src/composition/inline_prompt.rs`) drops `Constraint::Eager` from its match, and its `///` no longer claims eager implies required-at-completion.
        - second drifted comment found and fixed in the same change: `prepare_inline`'s `///` (`claudine/lib/src/composition/prepare.rs`) said the launch verdict requires "every `eager` property"; the runtime rule is that only `required; eager` must be present at launch. Code was treated as correct, the comment as wrong.
        - tests — `eager_without_required_is_still_required_at_completion` was renamed and inverted to `eager_without_required_is_optional_at_completion`; `only_required_marks_a_property_required_at_completion` was added to cover all four matrix cells plus `file(eager)[]`, `file[](eager)`, `file(required)[]`, `file[](required)`, and both union arms; `inline_prompt_table_marks_only_required_properties_required_at_completion` (`claudine/lib/src/composition/prepare/tests.rs`) pins the exact table through the normal `prepare_inline` path, which is the sole production caller of `build_inline_prompt_header`.
        - the prepare-path fixture must supply the `required; eager` property: `prepare_inline` composes at `SchemaPhase::Launch`, so omitting it aborts with `MissingRequired` before a header is built. The fixture therefore omits the eager-only, required-only, and unconstrained properties instead, which is also the more interesting launch row set.
        - non-vacuity evidence — restoring `Constraint::Required | Constraint::Eager` in the helper and rerunning `BISCUIT_TEST_FILTER='test(/inline_prompt_table_marks_only_required|eager_without_required_is_optional_at_completion|only_required_marks_a_property/)' just test claudine` gives **3 tests run: 0 passed, 3 failed**; with the fix restored the same filter passes. All three new/changed tests discriminate the axis.
        - `claudine/docs/topics/composition.md:681-695` already publishes the independent-axis contract, as the review said. A sweep of the other active Claudine surfaces (`docs/topics/frontmatter-properties.md`, `docs/topics/file-referencing.md`, `docs/pipeline.md`, `.claude/skills/claudine/*`) found no remaining copy of the retired rule.
        - `plan.md` Phase 5 evidence corrected: the inverted test is no longer cited as proof that runtime verdicts are unchanged; the paragraph now records that the prompt table *was* wrong and names the three replacement tests.
        - verification — root `just test claudine`: **7,082 run, 7,082 passed, 13 skipped** (7,080 before, plus the two added tests). The first run of that gate reported one `TIMEOUT` on `composition::sequence::preflight::tests::shell::bracket_target_identity_is_rejected_on_every_graph_shell_surface`; run alone it passes in 6.8s against a 30s budget, and it touches no schema or prompt code, so it is the known load artifact. The clean re-run above is the reported result.
        - verification — `darkmatter/ just test`: **7,704 run, 7,704 passed, 51 skipped**. `darkmatter/ just lint`: clean, including `zed-dmls` for `wasm32-wasip2`. `claudine/ just lint`: clean across `claudine-catalog-types`, `claudine`, `claudine-contract`, `claudine-cli`, and `claudine-gen`.
        - all gates were run with `env -u MODEL -u CLAUDINE_INTERACTIVE -u CLAUDINE_PID -u CLAUDINE_SESSION_ID` and after `cargo build -p darkmatter-cli --bin md`, the same host prerequisites Phase 5 recorded.
        - Finding 2 (the `STRICT_EAGER_ABSENCE_DOC` comment) was out of scope for this task and is untouched.
- work completed for 'Finding 1 — Claudine inline prompt eager-vs-required' at 13:47:50
- starting the work on 'Finding 2 — strict-mode fixture array-placement coverage' at 13:49:12
        - took the stronger of the review's two remedies: added the missing omitted `file[](eager)` sibling (`refs`) to `STRICT_EAGER_ABSENCE_DOC` rather than downgrading the claim, so AC8's array-ownership coverage gains real strict-mode diagnostic parity instead of a corrected comment.
        - the fixture now omits five properties: `spec` (`string(eager)`, eager-only scalar), `items` (`file(eager)[]`, eager owned by the items), `refs` (`file[](eager)`, eager owned by the array property), `notes` (`string`, the no-constraint control), and `plan` (`string(required; eager)`, the sole permitted diagnosed absence). The `///` above the constant was rewritten to say exactly that — the previous wording claimed `spec`/`items` covered "both array placements" when `spec` is a scalar and `items` is only the prefix placement.
        - assertions updated: the eager-only loop is now `["spec", "items", "refs", "notes"]`, and `missing.len() == 1` still holds.
        - the range assertion was genuinely shifted by the added line and remains a real range assertion: a missing key has no value node, so the diagnostic is ranged on the frontmatter mapping, which grew from lines 1–7 to lines 1–8. Only the end line changed.
        - swept the rest of `lsp_session.rs` for other array-placement claims: the only other one is the `HOVER_MATRIX` comment ("`file(eager)[]` owns the items, `file[](eager)` owns the property"), which matches its four array rows exactly. No further drift found, nothing else touched.
        - non-vacuity evidence — mutating `refs` to `file[](required; eager)` (and `plan` to `string(eager)` so exactly one required property remains, see the anomaly below) makes the run **FAIL** with `"refs" is a required property` at `lsp_session.rs:3740`, i.e. the sole missing-required diagnostic switches from `plan` to `refs`. Restoring `file[](eager)` returns the test to **1 passed**. The new coverage discriminates the presence axis at the postfix array placement.
        - anomaly found while proving non-vacuity, NOT fixed (out of scope for this finding, reported to the orchestrator): with DMLS schema strict mode on, a document whose effective schema carries **exactly two** `required` properties publishes **zero** `dm.schema.missing_required` diagnostics. Black-box probes through the real LSP session: 1 required → 1 diagnostic; 2 required → 0; 3 → 3; 4 → 4; 2 required with one supplied → 0. Instrumenting `dmls/src/diagnostics/frontmatter.rs` showed the effective JSON Schema does contain `"required":["a","b"]` yet `EffectiveSchema::validate_with_options` returns `valid=true` with an empty problem list, so the loss is upstream of DMLS's problem-to-diagnostic mapping. `md schema validate` on an equivalent standalone document reports both missing properties, so the two paths disagree. All instrumentation and probe tests were reverted; `git status` for `darkmatter/` shows only `dmls/tests/lsp_session.rs` and this log.
        - the first non-vacuity attempt (`refs` → `file[](required; eager)` alone) also failed, but for the contaminated reason above (two required absences → zero diagnostics), so it was replaced by the single-required mutation reported here.
        - verification — `darkmatter/ just test`: **7,704 run, 7,704 passed, 51 skipped**. `darkmatter/ just lint`: **clean** across `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`, including `zed-dmls` for `wasm32-wasip2`. Both run with `env -u MODEL -u CLAUDINE_INTERACTIVE -u CLAUDINE_PID -u CLAUDINE_SESSION_ID`.
- work completed for 'Finding 2 — strict-mode fixture array-placement coverage' at 13:59:44
- orchestrator verification of the reported anomaly at 14:05:12
        - the two-required anomaly reported under Finding 2 was **independently reproduced** rather than taken on the subagent's word. A temporary probe was appended to `dmls/tests/lsp_session.rs`, driven through the same real `ClientFixture` LSP session (initialize → open → publish-diagnostics) with `[schema] strict = true`, generating documents whose `$schema` declared `n` omitted `string(required)` properties for `n` in 1..=4.
        - probe results, verbatim: `required=1 -> missing_required=1`; `required=2 -> missing_required=0`; `required=3 -> missing_required=3`; `required=4 -> missing_required=4`. The exactly-two case is the only one that loses its diagnostics, and it loses **all** of them.
        - the probe was removed and `dmls/tests/lsp_session.rs` restored to its Finding 2 state (6,117 lines, no `tmp_probe` symbol remaining); `BISCUIT_TEST_FILTER='test(strict_mode_diagnoses_absent_required_eager_but_never_absent_eager_only)' just test` re-confirmed **1 passed** after the restore.
        - this defect is **out of scope for this fix** — it is a presence-counting bug in schema validation, not a `required`/`eager` axis conflation, and it predates this cycle. It is recorded here so it is not lost; it warrants its own fix spec.

### Successful Completion

The implementation of review cycle 2 has completed successfully in 1 hour, 10
minutes, and 54 seconds. During this implementation all 2 review findings were
evaluated to see if they could be fixed as a part of this implementation cycle:
2 were fixed, 0 were deferred.

No finding required deferral. In particular, no performance measurement was
requested by this review, so no measurement was blocked by host CPU load and
`deferred_perf_measurement` remains unset.

The files changed by this implementation cycle are:

- `claudine/lib/src/composition/inline_prompt.rs` — `def_is_required_at_completion`
  now treats only `Constraint::Required` as a completion presence rule across
  item constraints, postfix array constraints, and union arms; its `///` no
  longer claims eager implies required-at-completion; the eager-only regression
  was renamed and inverted and a four-cell matrix control added.
- `claudine/lib/src/composition/prepare.rs` — corrected a second drifted `///`
  on `prepare_inline` that described the launch verdict as requiring every
  `eager` property.
- `claudine/lib/src/composition/prepare/tests.rs` — added
  `inline_prompt_table_marks_only_required_properties_required_at_completion`,
  pinning the exact property table delivered to the agent through the normal
  inline preparation path.
- `darkmatter/dmls/tests/lsp_session.rs` — added the omitted `refs: file[](eager)`
  sibling to `STRICT_EAGER_ABSENCE_DOC`, corrected the constant's `///`, extended
  the eager-only assertion loop, and shifted the frontmatter-mapping range end to
  match the grown fixture.
- `darkmatter/fixes/2026-09-07-required-vs-eager/plan.md` — Phase 5 evidence no
  longer cites the inverted test as proof that runtime verdicts were unchanged.
- `darkmatter/fixes/2026-09-07-required-vs-eager/log.md` — this log.
- `darkmatter/fixes/2026-09-07-required-vs-eager/review-2.md` — closing metadata.

### Follow-up Owed

One defect was discovered during this cycle and deliberately left unfixed
because it is outside the review's scope and unrelated to the `required`/`eager`
axis:

- with DMLS schema strict mode enabled, a document whose effective schema
  declares **exactly two** `required` properties publishes **zero**
  `dm.schema.missing_required` diagnostics. One, three, and four required
  properties all diagnose correctly. `EffectiveSchema::validate_with_options`
  returns `valid=true` with an empty problem list even though the compiled JSON
  Schema carries both names in `required`, and `md schema validate` disagrees
  with the DMLS path on the same input. Reproduced independently through a real
  LSP session by the orchestrator. This needs its own fix spec.

**Resolved in cycle 3.** Review 3 ruled the defect in scope rather than
deferring it to its own spec, and it was fixed under "Implementation of Review
Findings #3" below. It was never a DMLS or Darkmatter defect: `jsonschema`
0.42.2 compiled no `required` validator at all for a two-name `required` array
whose parent object also carried `properties` above an internal 15-property
threshold, and the shipped baseline's 16 properties put every Darkmatter
document over it. The dependency was upgraded to 0.55.0 (upstream fixed the
shape in 0.46.1 and the sibling `additionalProperties` shape in 0.46.2), and the
reproduced matrix is now a permanent Level-1 regression in both
`darkmatter/lib/tests/schemas_required_count_matrix.rs` and
`darkmatter/dmls/tests/lsp_session.rs`. The discovery record above is retained
as written.

## Implementation of Review Findings #3

> **started at:** 2026-09-07T14:48:22-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/darkmatter/fixes/2026-09-07-required-vs-eager/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'Finding 1 — strict missing-required diagnostics vanish for exactly two required properties' at 14:48:34
        - orchestrator triage first, so the subagent starts from a root cause rather than the symptom. The defect is **not** DMLS-local and **not** a `required`/`eager` conflation; it reproduces through the plain `md schema validate` CLI as soon as a baseline schema is supplied.
        - reproduction, no DMLS involved — a document declaring `n` omitted `string(required)` properties validated against a baseline schema of `N` properties:
                - `N` = 12 baseline properties (14 total after merge): `n` = 1/2/3/4 -> 1/2/3/4 diagnostics (correct)
                - `N` = 13 baseline properties (15 total after merge): `n` = 1/2/3/4 -> 1/**0**/3/4 diagnostics
                - the Darkmatter baseline (`docs/schemas/darkmatter.yaml`) has 16 top-level properties, so every DMLS document is over the threshold — which is exactly why DMLS and bare `md schema validate` disagreed.
        - root cause is upstream in **`jsonschema` 0.42.2**, not in Darkmatter or DMLS. `keywords/required.rs::compile` returns `None` (compiling *no* `required` validator at all) whenever `required` lists exactly two names and the parent has `properties`, on the assumption that `keywords/properties.rs` will emit the fused `SmallPropertiesWithRequired2Validator`. But `properties::compile` only emits that fused validator when `map.len() < HASHMAP_THRESHOLD` (15); at or above the threshold it emits `BigPropertiesValidator`, which does not check `required`. The two-key `required` is then never validated by anything.
        - upstream confirms both the defect and its fix in its changelog: **0.46.1** — "`required` not enforced when `properties` has 15 or more entries and `required` lists exactly 2 keys"; **0.46.2** — "`required` not enforced when `additionalProperties` is a schema object and `required` lists exactly 2 keys". The repo currently pins `jsonschema = "0.42"` in `darkmatter/lib`, `claudine/contract`, `unchained-ai/contract`, and `schematic/gen`.
        - verified the triage before acting on it, then upgraded the dependency rather than working around it. **Landed on `jsonschema` 0.55.0** — the latest release, well past the 0.46.1/0.46.2 fixes — because the API churn from 0.42 turned out to be six compiler errors in one file, all of the same shape. There was no reason to settle for an intermediate version.
        - five workspace crates pinned `jsonschema = "0.42"`, not the four the triage named: `darkmatter/lib`, `claudine/contract`, `unchained-ai/contract`, `schematic/gen`, and `biscuit-file/lib` (optional, behind its `schema` feature). All five were moved to `0.55` together; `cargo tree -i jsonschema` reports a **single** `jsonschema v0.55.0` node. The lockfile also picks up `referencing` 0.42.2 -> 0.55.0 and the new `jsonschema-regex` / `jsonschema-value` / `micromap` / `fancy-regex 0.19` transitives.
        - the only source churn is `darkmatter/lib/src/markdown/schemas/format.rs`: upstream's `Keyword` trait gained an instance lifetime (`Keyword<'i, F: Json = SerdeJson>`), so the three keyword factories now return `Box<dyn for<'i> Keyword<'i>>` and the three impls became `impl<'i> Keyword<'i> for ...` with `is_valid(&self, instance: &'i Value)`. No behavior change, no message drift, no assertion had to be relaxed anywhere.
        - the comment at `format.rs:190` naming "jsonschema 0.42's `with_format`" was re-verified rather than assumed: `with_format` still takes `Check: Fn(&str) -> bool + Send + Sync + 'static` in 0.55, so the *claim* was right and only the version name had drifted. The version was dropped from the comment instead of bumped — pinning a dependency version in prose is the treadmill the repo comment rules warn about.
        - `cargo check --all-targets` is clean for `darkmatter`, `claudine-contract`, `unchained-ai-contract`, `schematic-gen`, and `biscuit-file`. The other named API surfaces (`ValidationErrorKind` matching in `schemas/validate.rs`, `paths::LocationSegment` in `compose/schema_validation.rs`, `jsonschema::options()`/`Draft`, `jsonschema::validator_for`) all compile unchanged.
        - Darkmatter Level-1 regression added at the shared seam: `darkmatter/lib/tests/schemas_required_count_matrix.rs`, three tests, all through the ordinary `DarkmatterSchemas` baseline-merge and `EffectiveSchema` path (no hand-built `jsonschema::Validator`).
                - `every_omitted_required_property_is_reported_over_the_baseline` — the 1/2/3/4 matrix over `with_darkmatter_baseline_json_schema()`, whose 16 baseline properties are what push the merged count past the upstream fusion threshold. Asserts the exact set of reported property names, not just the count, and that each message names its property.
                - `one_of_two_required_properties_supplied_still_reports_the_other` — the review's supplied/omitted control.
                - `two_required_properties_are_reported_under_a_schema_additional_properties` — the 0.46.2 shape. This one needed a referenced raw JSON Schema file (`$schema: ./schema.json` in a temp dir): a SimplifiedSchema never emits an `additionalProperties` *schema object*, and `validate_baseline_schema` rejects `additionalProperties` in a baseline outright, so the referenced-file route is the only authorable path to that shape. Still the normal resolution path.
        - DMLS Level-1 real-LSP regression added: `strict_mode_reports_every_absent_required_property_at_any_required_count` (`darkmatter/dmls/tests/lsp_session.rs`), driven through the normal `ClientFixture` initialize -> didOpen -> publishDiagnostics path against a workspace whose `.dmls.toml` sets `[schema] strict = true`. One fixture, six documents, matching the file's existing `concat!` const conventions.
                - the six cases are the 1/2/3/4 matrix (case `two` is also the review's "two required-only properties" control), two `string(required; eager)` properties (both must be diagnosed — adding `eager` must neither add nor remove a diagnosis), and two required with one supplied (only the omitted one).
                - each case asserts the code (`dm.schema.missing_required`), the property name in the message, the severity (`1`), the source (`darkmatter.schema`), and the **exact** LSP range. Per the spec's "Diagnostic ranges" section a missing key has no value node, so the range is the frontmatter mapping: `line 1, character 0` through the closing `---` fence. That fence line differs per case (4/5/6/7/5/6), so the range table is a real assertion rather than a restatement of one document.
                - diagnostics are matched to expected properties **by name, not by index** — publish order is not part of the contract, one diagnostic per omitted property is. Each expected name must be claimed by exactly one diagnostic.
                - `STRICT_EAGER_ABSENCE_DOC` and its cycle-2 assertions were not touched; the fix required no change to them.
        - non-vacuity evidence, verbatim. With `jsonschema` reverted to `0.42` in all five manifests, the lockfile downgraded (`cargo update -p jsonschema --precise 0.42.2`) and `format.rs` restored to its pre-upgrade lifetimes, the filter `binary(schemas_required_count_matrix) + test(strict_mode_reports_every_absent_required_property_at_any_required_count)` gives **`4 tests run: 0 passed, 4 failed, 7755 skipped`**:
                - `two_required_properties_are_reported_under_a_schema_additional_properties` — `left: [] right: ["alpha", "bravo"]`
                - `one_of_two_required_properties_supplied_still_reports_the_other` — `left: [] right: ["bravo"]`
                - `every_omitted_required_property_is_reported_over_the_baseline` — `2 omitted required properties must each be reported`, `left: [] right: ["alpha", "bravo"]`
                - `strict_mode_reports_every_absent_required_property_at_any_required_count` — `[two] expected 2 missing-required diagnostics: []`, `left: 0 right: 2`
        - with the fix restored, the same filter gives **`4 tests run: 4 passed, 7755 skipped`**. Every new test discriminates the defect, and each fails on the exactly-two case specifically — the 1/3/4 legs of the matrix passed even on 0.42, which is what made the bug invisible for so long.
        - no upstream message drift to absorb: `RequiredProperty`'s `Display` is byte-identical in 0.42.2 and 0.55.0 (`{property} is a required property`), and no message-pinning assertion anywhere in the workspace had to be touched.
        - one dependency-tree follow-on, in scope because a comment asserts it: `darkmatter/lib`'s direct `fancy-regex` pin was `0.17` and documented as "already compiled transitively via `jsonschema`, so no added build cost". `jsonschema` 0.55 brings `fancy-regex` 0.19, which would have made that claim false and added a third `fancy-regex` build. The direct pin moved to `0.19` to restore the shared-copy invariant the comment describes; `fancy_regex::Regex::new` is unchanged and `cargo check --all-targets` is clean.
        - documentation drift swept: `docs/dependencies.md` (two `0.42` workspace-pin mentions, plus the Schema Validation catalog entry which was separately stale at `v0.28`), `claudine/contract/docs/dependencies.md`, and `unchained-ai/contract/docs/dependencies.md` all now say `0.55`. `darkmatter/docs/dependencies.md` and `schematic/docs/dependencies.md` record no `jsonschema` version, so neither needed an edit. The only remaining `jsonschema 0.42` mentions in the tree are the two new regression tests' module docs, where naming the defective version is the point.
        - the cycle-2 `### Follow-up Owed` section above now records the resolution and points at this work; its original discovery text is left verbatim.
- orchestration resumed at 15:40:48 after the previous orchestrator run ended before writing its closing entries
        - the resumed run did not take the log on trust. Every claim above that is checkable from the tree was re-checked: the five `jsonschema = "0.55"` manifest pins (`darkmatter/lib`, `claudine/contract`, `unchained-ai/contract`, `schematic/gen`, `biscuit-file/lib`), the single `jsonschema v0.55.0` node in `cargo tree -i`, the presence of `darkmatter/lib/tests/schemas_required_count_matrix.rs`, and `strict_mode_reports_every_absent_required_property_at_any_required_count` in `darkmatter/dmls/tests/lsp_session.rs`. All present as described.
        - the review's single finding was already implemented in full by that earlier run, so no new subagent implementation work was dispatched. The remaining work for this run was independent verification and closure.
        - the root-cause claim was re-derived from the vendored upstream sources rather than from the changelog citation, because the whole fix rests on it. In `jsonschema-0.42.2/src/keywords/required.rs`, `compile` short-circuits to `None` — compiling no `required` validator — under "Case 2: properties + required: [2 items], no additionalProperties: false, no patternProperties", delegating to a fused validator. `keywords/properties.rs:422` only emits that `SmallPropertiesWithRequired2Validator` when `map.len() < HASHMAP_THRESHOLD`, and `properties.rs:59` defines that threshold as `15`; the same function returns `None` outright when `additionalProperties` is `Bool(false) | Object(_)`, which is the second (0.46.2) shape. In `jsonschema-0.55.0/src/keywords/required.rs` no such skip guard exists. The triage recorded above is correct in every particular.
        - gap found and closed in this run: the earlier run verified the downstream Claudine surface with `cargo check` only, but the spec's Verification section mandates the root `just test claudine` gate, and `claudine/contract` is one of the five crates whose `jsonschema` pin moved. That gate was run here.
        - the earlier run also left three package areas carrying the bumped pin untested (`biscuit-file/lib`, `schematic/gen`, `unchained-ai/contract` are covered by neither the darkmatter nor the claudine gate). All three were run here.
        - verification, this run, all with `env -u MODEL -u CLAUDINE_INTERACTIVE -u CLAUDINE_PID -u CLAUDINE_SESSION_ID`:
                - `darkmatter/ just test`: **7,708 run, 7,708 passed, 51 skipped** — four more than cycle 2's 7,704, matching the three new library matrix tests plus the one new DMLS regression.
                - `darkmatter/ just lint`: **clean** across `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`, including `zed-dmls` against `wasm32-wasip2`.
                - root `just test claudine`: **7,082 run, 7,082 passed, 13 skipped**.
                - root `just test biscuit-file`: **813 passed**; `just test schematic`: **1,699 passed, 5 skipped**; `just test unchained-ai`: **367 passed, 1 skipped**.
                - `cargo tree -i jsonschema --workspace`: a single `jsonschema v0.55.0` node, confirming no duplicate-version build.
        - the two new test surfaces were read rather than counted. `schemas_required_count_matrix.rs` asserts the exact *set* of reported property names at each count and that each message names its property, so a silent-drop regression cannot pass by coincidence; `strict_mode_reports_every_absent_required_property_at_any_required_count` asserts the code, the per-name uniqueness, the severity, the source, and the exact LSP range for all six cases, with a per-case fence line that makes the range table a genuine assertion. Both satisfy the review's "assert the diagnostic codes, property names, severities, and stable mapping ranges" requirement.
- work completed for 'Finding 1 — strict missing-required diagnostics vanish for exactly two required properties' at 15:48:29

### Successful Completion

The implementation of review cycle 3 has completed successfully in 1 hour and 6
seconds. During this implementation all 1 review findings were evaluated to see
if they could be fixed as a part of this implementation cycle: 1 were fixed, 0
were deferred.

No finding was deferred. The review requested no performance measurement, so no
measurement was blocked by host CPU load and `deferred_perf_measurement`
remains unset.

Review 3's sole finding is closed at its true root cause rather than at the
symptom. The defect was never a `required`/`eager` conflation and never lived in
Darkmatter or DMLS: `jsonschema` 0.42.2 compiled no `required` validator at all
for a two-name `required` array whose parent object also declared `properties`,
because it assumed a fused properties-plus-required validator would cover it —
and that fusion was only emitted below an internal 15-property threshold, and
never when `additionalProperties` was a schema object. The shipped Darkmatter
baseline declares 16 top-level properties, so every merged document sat above
the threshold and every strict-mode document with exactly two required
properties published a false-clean verdict. The dependency moved to 0.55.0, past
upstream's 0.46.1 and 0.46.2 fixes, and the reproduced 1/2/3/4 matrix is now a
permanent Level-1 regression at both the shared validation seam and the real
DMLS LSP path.

The files changed by this implementation cycle are:

- `darkmatter/lib/Cargo.toml` — `jsonschema` `0.42` -> `0.55`; the direct
  `fancy-regex` pin `0.17` -> `0.19` to preserve the shared-transitive-copy
  invariant its adjacent comment asserts.
- `claudine/contract/Cargo.toml`, `unchained-ai/contract/Cargo.toml`,
  `schematic/gen/Cargo.toml`, `biscuit-file/lib/Cargo.toml` — the same
  `jsonschema` `0.42` -> `0.55` move, so the workspace keeps a single compiled
  copy.
- `Cargo.lock` — `jsonschema` and `referencing` to 0.55.0 plus the new
  `jsonschema-regex`, `jsonschema-value`, `micromap`, and `fancy-regex 0.19`
  transitives.
- `darkmatter/lib/src/markdown/schemas/format.rs` — the only source churn from
  the upgrade: upstream's `Keyword` trait gained an instance lifetime, so the
  three keyword factories return `Box<dyn for<'i> Keyword<'i>>` and the three
  impls carry `<'i>`. No behavior change. A comment that pinned a dependency
  version in prose had the version dropped rather than bumped.
- `darkmatter/lib/tests/schemas_required_count_matrix.rs` — new. Three Level-1
  tests through the ordinary `DarkmatterSchemas` baseline-merge and
  `EffectiveSchema` path: the 1/2/3/4 matrix over the shipped baseline, the
  supplied-one-of-two control, and the `additionalProperties`-schema shape.
- `darkmatter/dmls/tests/lsp_session.rs` — new Level-1 real-LSP regression
  `strict_mode_reports_every_absent_required_property_at_any_required_count`,
  six documents through the normal initialize -> didOpen -> publishDiagnostics
  path under `[schema] strict = true`, asserting code, property name, severity,
  source, and exact range per case.
- `docs/dependencies.md`, `claudine/contract/docs/dependencies.md`,
  `unchained-ai/contract/docs/dependencies.md` — `jsonschema` version drift
  swept to `0.55`; the root Schema Validation catalog entry was separately stale
  at `v0.28` and was corrected in the same pass.
- `darkmatter/fixes/2026-09-07-required-vs-eager/log.md` — this log, including
  the cycle-2 `### Follow-up Owed` resolution note.
- `darkmatter/fixes/2026-09-07-required-vs-eager/review-3.md` — closing
  metadata.
