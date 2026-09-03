---
implementation_1: "2026-09-02T14:18:28+01:00"
---

## Implementation of Review Findings #1

> **started at:** 2026-09-02T14:18:28+01:00

- this implementation is attempting to implement _all_ of the review findings found in '/Users/ken/.claudine/worktrees/rusty-biscuit/feat-unifi/claudine/fixes/2026-09-01-file-param-anchoring/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Finding 1: anchor the public md compose route' at 14:24:32
        - GitNexus reported LOW upstream risk for `run_compose`: one direct caller, one affected CLI execution flow, and three impacted symbols through depth 3
        - GitNexus reported LOW upstream risk for `caller_input_records`: three direct callers within the compose pipeline, one affected execution flow, and eight impacted symbols through depth 3
        - the public `md compose` route captures launch CWD for `ComposeContext` but does not attach a launch-anchored `FileResolutionContext`; the library projection also ignores raw overrides when only that context is supplied
        - GitNexus reported LOW upstream risk for the `ComposeOptions::with_file_resolution_context` boundary: one direct dependent and seven impacted symbols through depth 3
        - attached one launch-captured file-resolution snapshot and fallback to the public route before both reference validation and final composition
        - made `FileResolutionContext::request_base_dir()` sufficient provenance for raw caller overrides at the library boundary, while preserving the fallback-only compatibility path
        - added Level 1 coverage for the context-only library boundary and real `md compose` processes launched from the repository root and `claudine/` package area against the shipped `prompts/plan.md`
        - focused Level 1 verification passed: one context-only library test and two shipped-prompt `md compose` process tests
        - the first full Level 1 run found two temporary-source transclusion-cycle regressions because the launch repository did not contain their resolved source documents
        - derived the document authoring base from the same launch snapshot and used the existing trusted-external source path for already-resolved inputs outside the launch repository
        - the first trusted-external retry retained an inapplicable launch repository root, so transclusion still rejected the temporary source; rebuilt that external document view from the snapshot's retained launch base, home, and environment without recapturing ambient state
        - the second full Level 1 run exposed two external temporary Git-repository link-normalization regressions because the repository-less document view emitted absolute links
        - added focused source-repository and package-area discovery for external documents while retaining the launch snapshot's caller base, home, and environment
        - the final Darkmatter `just test` gate passed: 7,582 Level 1 tests passed and 50 higher-tier tests were skipped by the canonical filter
        - the Darkmatter `just lint` gate passed for `darkmatter`, `darkmatter-cli`, and `dmls`
        - `git diff --check` passed, and GitNexus change detection reported low risk with no affected execution processes
- work completed for 'Finding 1: anchor the public md compose route' at 14:48:09
- starting the work on 'Finding 2: preserve exactly-one-applicable union semantics' at 14:48:44
        - the current branch already contains property-union and root-union applicability selection that postdates the implementation described by the review; the remaining work is to prove it against the review's exact regressions and correct only any exposed gap
        - GitNexus reported LOW upstream risk for `select_file_mode`, `collect_applicable_root_schema_fragments`, and `root_schema_arm_applies`; their blast radius is confined to the Darkmatter compose/transclusion path
        - GitNexus reported HIGH upstream risk for `ensure_projection_stable`: one direct caller, two affected compose execution flows, and 22 impacted symbols through depth 3; no edit to this symbol will proceed unless the focused drift regressions prove it necessary
        - added Level 1 regressions for an eager-file/plain-string property union, an active non-eager discriminated root arm, bidirectional eager/non-eager root-arm drift after interpolation, and eager-to-non-eager root-arm drift after shell expansion
        - the focused regressions proved that pending interpolation and shell values made all otherwise-inapplicable root arms appear applicable, masking the initial eager classification
        - distinguished exact root-arm matches from matches deferred solely by composition-pending properties; union selection now prefers exactly one exact arm and otherwise requires exactly one pending arm, preserving the same no-guess contract for zero and multiple candidates
        - the post-shell regression also proved that classification stability was rechecked only when trigger schemas were enabled; schema applicability is now revalidated after shell expansion for static root unions as well as trigger-driven schemas
        - the four focused Level 1 regressions passed, including the typed `CallerFileClassificationChanged` fail-closed assertions after interpolation and shell expansion
        - the Darkmatter `just test` gate passed: 7,586 Level 1 tests passed and 50 higher-tier tests were skipped by the canonical filter
        - the Darkmatter `just lint` gate passed for `darkmatter`, `darkmatter-cli`, and `dmls`
        - `git diff --check` passed; GitNexus change detection reported medium aggregate risk across the shared Finding 1 and Finding 2 worktree changes, with one affected compose execution flow and no unexpected affected subsystem
- work completed for 'Finding 2: preserve exactly-one-applicable union semantics' at 14:59:25
- starting the work on 'Finding 3: complete Level 1 verification evidence' at 15:00:14
        - audited the completed Finding 1 and Finding 2 work before adding coverage: shipped `md compose`, property/root union selection, and interpolation/post-shell classification drift were already covered and were not duplicated
        - GitNexus reported HIGH upstream risk for the test-only instrumentation point `prepare_schemas`: two direct callers, two affected compose execution flows, and 21 impacted symbols through depth 2; the added counter is compiled only for tests and increments only at the existing trigger-discovery branch
        - GitNexus reported LOW upstream risk for the shipped-workflow Claudine process test, with no callers or affected execution flows
        - added Level 1 evidence that post-shell expansion and frontmatter interpolation pass 2 retain the same native caller value, portable presentation value, and derived path
        - added a test-only atomic discovery counter and proved that the pre-shell and post-shell schema passes perform exactly one trigger-registry filesystem discovery walk
        - added direct Level 1 proof that installing the same caller projection twice is idempotent
        - added Level 1 coverage for an eager caller file declared by the baseline schema alongside document schema properties, including an absent optional eager caller property that remains unprojected
        - changed the Claudine process regression to invoke the repository's shipped `prompts/plan.md` and real fix specification from both repository-root and `claudine/` package-area launch directories
        - the first Claudine full-suite run exposed that Finding 2's unconditional full post-shell Darkmatter validation bypassed Claudine's established invalid-optional drop-and-diagnostic owner
        - retained full post-shell validation for trigger schemas while adding a classification-only recheck for other schemas, preserving static root-union drift detection without taking ownership of downstream coercion or optional-value scrubbing
        - focused Level 1 verification passed for all three new Darkmatter cases, the shipped Claudine process case, the static root-union post-shell drift case, and Claudine's post-shell optional-drop regression
        - the final Darkmatter `just test` gate passed: 7,589 Level 1 tests passed and 50 higher-tier tests were skipped by the canonical filter
        - the final Darkmatter `just lint` gate passed for `darkmatter`, `darkmatter-cli`, and `dmls`
        - the final Claudine `just test` gate passed: 6,685 Level 1 tests passed and 11 higher-tier tests were skipped by the canonical filter
        - the final Claudine `just lint` gate passed for all five package-area crates and the diagnostic guards
        - `git diff --check` passed; GitNexus change detection reported medium aggregate risk across the shared Finding 1 through Finding 3 worktree, with one affected compose execution flow and no unexpected affected subsystem
- work completed for 'Finding 3: complete Level 1 verification evidence' at 15:19:34

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1 hour, 1 minute, and 6 seconds. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no review findings were deferred
- The files updated by this implementation were:
        - `darkmatter/cli/src/commands/compose.rs`
        - `darkmatter/cli/tests/compose_schema.rs`
        - `darkmatter/lib/src/markdown/compose/context/options.rs`
        - `darkmatter/lib/src/markdown/compose/pipeline/mod.rs`
        - `darkmatter/lib/src/markdown/compose/schema_validation.rs`
        - `darkmatter/lib/src/markdown/compose/tests/schema.rs`
        - `claudine/cli/tests/compose_schema_cli.rs`
        - `claudine/fixes/2026-09-01-file-param-anchoring/review-1.md`
        - `claudine/fixes/2026-09-01-file-param-anchoring/log.md`
