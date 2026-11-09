---
$schema: feature-review.yaml
ready: false
findings:
  - "R3-F1 (high): Nested contract cache hits lose the defining schema's read permissions"
human_review: true
human_review_items:
  - "R3-F1: allowed slot reads are recorded only in the thread-local dependency frames (one inventory replacing the separate per-resolution recorder); confirm, or keep a thread-agnostic recorder alongside"
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-08T15:35:51-08:00"
spec: "2026-11-05-dynamic-schema-types/spec.md"
implemented: true
description: "A **feature** review of `2026-11-05-dynamic-schema-types/spec.md`"
feature: "2026-11-05-dynamic-schema-types/review-3.md"
previous: "2026-11-05-dynamic-schema-types/review-2.md"
next: "2026-11-05-dynamic-schema-types/review-4.md"
log: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/log.md"
implemented_by: claude/opus
---

# Review 3: Dynamic Schema Types

The feature is **not production ready**. Both findings from review 2 are verified fixed, including the additional instances in the repair ledger. One newly found sibling of the first review's permission finding remains: a cached outer schema can reuse a nested contract's result without the permission that contract needs. No human decision is required to repair it.

I read the specification and its investigation outcomes and rulings, both earlier reviews, the implementation log, and both finding ledgers in `log.md`. Neither earlier review contains a `## Human rulings` section. Previously accepted implementation choices remain accepted; this review does not ask the author to reaffirm them.

## Previous Finding Verification

Evidence below comes from the implementation and tests rerun for this review, rather than the repair's completion flags. Required instances include the ledger's additional memo and reference-validation cases. There were no still-open findings from review 1 in review 2.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R2-F1: Cache reuse omits implicit and transitive request inputs | R2-F1.1 clock; .2–.3 command probes; .4–.5 addresses; .6–.7 package/area; .8 installed agent; .9 nested captured input; .10 nested volatile call; .11 later union round; .12 direct volatile control; .13 environment control; .14 file/import/failure/mutation/read-set controls; .15 permission controls; additional contract-memo dependency replay | **verified fixed** | In Darkmatter, `slots/dependencies.rs` collects dependencies during evaluation; the central function dispatcher records captured answers and executed volatile calls. `SlotSession::serves` compares captured answers using the new session's observations. Nested evaluation contributes to enclosing collections, and `DataRuntime::prepare_contract` stores and replays the summary on a memo hit. Every `session::tests::consumed` case passed: unchanged-input hits, changed-input misses, cold controls, nested contract failures, memo replay, and later union rounds. All `reuse` controls also passed. | None of these required instances remains. R3-F1 concerns missing defining-file permission records, rather than captured values or volatile classification. |
| R2-F2: Referenced schemas and validation sessions omit required function captures | R2-F2.1 referenced schema; .2 imported type; .3 selected trigger; .4 validating/clean constructor; .5 editor worker; .6 inline control; .7 clock control; .8 command/PATH control; .9 reference checks; .10 nested contracts; additional referenced validating-session and request-based reference-validation cases | **verified fixed** | In Darkmatter, `SessionObservations::covering` supplies missing Repo, Network, and Agent captures before evaluation. Request sessions extend from their authority; context-only sessions use their supplied environment and repository anchor. `observed_groups` walks function calls and callback bodies for every evaluated slot, including later rounds and nested contracts. All 73 `dynamic_schema_slot_captures` tests passed, exercising the five affected functions and three controls across nine preparation paths, plus the selected-trigger guard. All seven actual DMLS worker cases passed. The matrix is declared in the L1 target, and the work-counter feature is enabled by local and CI metadata. | None. Frozen caller-supplied observations retain their existing contract; absence is not replaced with fabricated empty observations. |

The earlier repairs also remain effective in the rerun coverage: authorization before direct reads and imports (`dynamic_schema_slot_read_order`), pruning overridden trigger properties (`dynamic_schema_slot_precedence`), authored diagnostic positions (`dynamic_schema_slot_positions` and CLI/editor cases), every unsaved-buffer format and close transition, and stale/pending completion, hover, and quick-fix markers. R3-F1 is a newly found nested-cache sibling, not a claim that one of those previously enumerated cases has regressed.

## Unblocked Findings

### R3-F1 (high): Nested contract cache hits lose the defining schema's read permissions

**Authority: spec.** “Check current permissions before using a cached result” and “a result produced with remote or shell permission grants no permission to another request.” The automatic-grant decision also expressly says that a schema outside the request's repository gets no automatic grant and “every read it makes needs a grant.” These rules apply to an imported contract's own slots as well as the outer slot. A realistic shared schema under a user's schema folder can currently borrow a previous command's one-run permission when its contract result is cached.

**Defect class:** nested evaluations preserve input values but lose the defining-file provenance needed to reauthorize those inputs when a different command reuses the result. This is a sibling of R1-F1's cache-permission case, described under Recurrence.

In the darkmatter package, [`resolve_contract_slots`](../../lib/src/markdown/schemas/slots/session.rs:705), which evaluates a data contract's dynamic constraints, builds its host with plain `RunGates` at line 730. Ordinary resolution instead uses `RecordingGates`, which records both the read target and the schema file that made the read. The nested gate checks correctly while evaluating, but its defining-file permission requirements never reach the outer cache entry.

The outer `load_data` call subsequently checks all observed inputs through its own gate in [the function dispatcher](../../lib/src/markdown/compose/expression/functions/mod.rs:1073). That records the nested read as if the outer schema had made it. An outer schema inside the repository qualifies for the automatic grant; the shared contract outside it does not. [`SlotSession::serves`](../../lib/src/markdown/schemas/slots/session.rs:526), which authorizes cache reuse before checking input contents, therefore checks the wrong defining file and accepts the entry. The new [contract-memo replay](../../lib/src/markdown/compose/expression/data/runtime.rs:653) preserves observations and captured/volatile dependencies, but its `ContractSlotInputs` has no defining-file permission records either.

**Public-API reproduction.** Adapt the existing temporary committed sandbox fixture. The repository's schema loads its committed OS list under `Os@<shared-folder>/types.yaml`. That external file defines:

```yaml
kind: schema
types:
  Os: |-
    string(pattern({{ load_data("<repository>/data/pattern.yaml", "string[]")[0] }}))[]
```

The committed `data/pattern.yaml` contains `- '.*'`. Replace the angle-bracket placeholders with absolute paths; the probe used canonical temporary paths. Through the public `DarkmatterSchemas::effective_for` API:

1. Resolve with a fresh session holding one-run file grants for `types.yaml` and `pattern.yaml`. It successfully exports the OS enum.
2. Resolve the unchanged document with a new session granting only `types.yaml`. It incorrectly exports the same enum from the shared cache.
3. Clear the process cache and repeat exactly step 2. It fails with `dm.schema.slot_approval_required` for the shared contract's read of `pattern.yaml`.

The difference is entirely the warm cache: neither schema text, file bytes, environment, nor requested permissions changes between steps 2 and 3. No malicious input or filesystem race is needed. A long-running embedding host can reach this with successive commands that have different one-run grants.

**Class sweep.** The following sites cover the evaluation, memo, projection, and reuse decisions in this feature's resolution path. The content, probe, deeper-contract, memo, and direct-control cases were each exercised through the same public API fixture with initial grants, narrowed grants, and a cold control. Other clean sites below are ruled out by their stated implementation contract.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Nested contract's content-read gate | Shared `Os` contract loads a pattern list from a clean repository file; remove only that file's one-run grant in a new session | Warm success; cold approval-required | Both require the contract's read grant |
| R3-F1.2 | Nested contract's existence-probe gate | Replace its pattern expression with `file_exists(<pattern path>) ? '.*' : '^Windows$'`; same narrowed grants | Warm success; cold approval-required | Both require a grant: the probe's defining schema is outside the repository |
| R3-F1.3 | Further nested contract resolution | `Os` loads the pattern under `Pattern@./inner.yaml`; `Pattern` itself has the existence-dependent dynamic pattern | Warm success; cold approval-required | Preserve permissions through every nesting level |
| R3-F1.4 | Contract memo and its replay | Resolve two different inline schema mappings in the same session and defining context, using the same contract; then request the second mapping from a narrower new session | Second mapping initially runs only its outer slot, proving a contract memo hit; its subsequent warm result succeeds while the cold control refuses | Replay defining-file permission records along with the memo's observations and captured dependencies |
| R3-F1.5 | Outer input-reading function's post-call observation check | The preceding nested reads are projected into the outer call's observations | Read is recorded under the repository-local outer schema; the external contract's stricter requirement is absent | Retain the nested requirement; the outer check cannot replace it |
| R3-F1.6 | Shared-cache permission acceptance | Request any of .1–.4 with the same inputs and narrower permissions | `serves` accepts based on the outer origin and returns a concrete schema | Check every original defining-file/target requirement before rehashing or serving |
| R3-F1.7 | Ordinary top-level template resolution, clean control | Reference the external schema directly and repeat the content-read/grant-removal comparison | Warm and cold both refuse | Correct: `RecordingGates` records the external origin |
| R3-F1.8 | Ordinary imported definitions and later root-union rounds, clean invariant | Slots resolved by `PendingSchema::resolve` through the same host passed by `resolve_fresh` | Every call uses that host's `RecordingGates`; resolving later cannot replace its recorder | Retain this origin recording; the plain-gate constructor is specific to nested contracts |
| R3-F1.9 | Within-command pins, clean contract | Reuse a snapshot in its original session | Grants remain decided for that run, as the spec expressly allows | Retain the once-per-run decision; repair cross-command acceptance |
| R3-F1.10 | Nested listings and remote reads, clean invariant for this origin-loss class | A listing or URL needs an explicit grant at both origins | Automatic-grant code always refuses listings/URLs; explicit grants match the target independently of origin | Changing the recorded origin alone cannot turn these into an automatic grant; retain explicit authorization |

All nested content readers share the same `RunGates` construction, and all ordinary slot positions share `RecordingGates`; this is not a separate escaping or per-function parsing problem. The content and existence cases exercise the two automatic-grant branches that depend on a defining schema being inside the repository. Listing and URL branches never qualify for that automatic grant.

**Recommended repair:** retain each allowed read's original defining file in the existing nested resolution dependency collection, and store/replay those records with contract memo entries. Include them in the enclosing cache entry's permission checks, including at further nesting levels. Keep the outer post-call check, but do not let it substitute for a nested read's permission requirement. Reuse the current grant types and gate logic; no new policy, approval choice, cache, or file format is needed. Add the public-result warm/narrow/cold cases above alongside the existing direct-cache permission controls.

## Blocked Findings

None. R3-F1 has a repair within the existing permission model and requires no author choice.

## Recurrence

R3-F1 repeats the permission-preservation class of **review-1.md, R1-F1: Several slot paths perform reads before approval**, specifically its repair-ledger child **R1-F1.18: cached inputs were rehashed before checking permission coverage**. It is a **sibling the earlier sweep missed**, not an unchanged reported instance or a regression demonstrated to have been introduced by the repair. The earlier fix correctly moved coverage checks before rehashing, but its sweep did not check how the coverage inventory is built by nested contract evaluation and replayed contract memos. Recording only outer reads leaves those checks incomplete.

The current sweep follows the value through the nested host, further nesting, contract memo, outer observation projection, and cache acceptance, with the direct host and nonautomatic target kinds as controls. The captured-input and missing-capture findings from review 2 are independently fixed. This finding has a bounded requirement—preserve existing defining-file permissions across cache reuse—rather than adding another input-protection rule.

## Verification and test rigor

| User-facing requirement | Verification level reviewed | Assessment |
| --- | --- | --- |
| Passive parse/query/export; enum/suggestion shapes, ordering, deduplication, injection resistance; literal/default/bound/pattern positions; imports, recursion, concrete snapshots | L1 grammar, facade, substitution, public-result and static-parity tests | Appropriate; no terminal or keyboard behavior is promised |
| Real sandboxing schema, passive corpus admission, explicit composition and validation | L1 shipped-schema/template and public composition/CLI tests | Relative `load_data("../../../os.yaml", "string[]")` spelling is present; corpus exception is removed |
| Grants, preflight, trigger precedence, reference checks, mutations and cache refresh | L1 scripted approval, read-observation, public composition and CLI tests | Existing cases pass; nested cross-command permission provenance fails under R3-F1 |
| Source positions and typed underlying failures | L1 library results, CLI data output, actual language-server responses | Previous authored-location repair remains effective |
| Unsaved data formats, worker cancellation, stale/pending completion, hover and affected quick fixes | L1 actual language-server protocol and worker tests, with held jobs | Appropriate: these are protocol contracts, not promises about editor painting or physical key encoding |
| Captured functions and their cache inputs across all schema sources | L1 public preparation/composition matrix, changed observations with cold controls, actual editor worker | Both review-2 findings verified fixed |

Commands run:

- Darkmatter: `just test slot` — **437 passed**, none at or above five seconds.
- Simplified-schema: `just test slot` — **87 passed**, followed by the recipe's **1,437 passing minimal-feature tests**.
- Repository root: `just check-tier-coverage darkmatter` — **no stranded tests**.
- Darkmatter: temporary `just test review3_nested` probes — five final cases completed in roughly 0.14–0.16 seconds each. Four confirmed warm success versus cold refusal, and the direct control refused both. The memo case also checked that only the outer slot ran on the initial memo hit. Before converting the probe into an inventory check, its expected-refusal assertion failed on the public API's returned concrete schema.

The probes used an existing declared test target and ordinary L1 names. Initial fixture errors (a scalar `load_data` contract and unquoted YAML ternaries) and a probe-only `Debug` compilation error were corrected before assessing behavior. Final tests used valid container contracts and block-scalar carriers. The temporary additions were removed, and `session_tests.rs` was verified byte-for-byte against its pre-review copy. No implementation repair, commit, formatter, focused window, network request, or external service was used. Full-suite and lint claims in the repair log were not independently rerun; cross-OS proof remains CI's responsibility and does not determine this review's readiness.

## Input robustness matrix

The feature's configuration loader remains the fixed-home, line-based schema read-grant store. Schema expressions are a text grammar; serializers/renderers and the existing data-file loaders are outside this new-loader matrix. Neither review-2 repair added another configuration format.

**Default outcome:** an invalid record rejects the entire store, reports its path and line, and grants nothing. A whole-file read/encoding failure reports the path without inventing a source line. **Format-defined exceptions:** a missing or empty store means no rules; blank/comment lines, CRLF, and a UTF-8 byte-order mark are allowed; identical rules deduplicate; denial overrides allowances.

I inspected `grants/store.rs` and its public loading/matching tests, all rerun by the slot selection. The control covers every action/scope; invalid-record cases cover absent/empty arguments, unknown action/scope, invalid paths/URLs and scope-specific components. Document cases cover duplicate rules, invalid UTF-8, unreadable files, accepted comment/line-ending syntax, and repeated write/read round trips. CLI invalid-store wiring cases also passed. This line format has no native null or collection-element shape. The required scan found no serde defaults, silent element filtering, default-on-parse-failure, or ignored parse result in the loader. No loader-shape finding was found.
