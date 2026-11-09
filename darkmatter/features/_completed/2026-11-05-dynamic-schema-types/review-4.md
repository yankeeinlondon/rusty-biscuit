---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-08T16:09:24-08:00"
spec: "2026-11-05-dynamic-schema-types/spec.md"
implemented: false
description: "A **feature** review of `2026-11-05-dynamic-schema-types/spec.md`"
feature: "2026-11-05-dynamic-schema-types/review-4.md"
previous: "2026-11-05-dynamic-schema-types/review-3.md"
---

# Review 4: Dynamic Schema Types

The feature is **production ready within the specification's scope**. The remaining finding from review 3 is verified fixed, including every instance and the additional cache/pin replay instance in the repair ledger. No new finding or pending decision remains.

I read the specification, including its investigation outcomes and author rulings, all three previous reviews, and the repair ledgers in `log.md` before assessing the implementation. None of the earlier reviews contains a `## Human rulings` section. Their accepted implementation choices remain accepted; this review does not request reaffirmation of them. In particular, the accepted size/mtime shortcut, once-per-run grants, whole-type sequencing, and bounded performance measurements are not reopened.

## Previous Finding Verification

The required instances below come from review 3 and its repair's Finding Ledger. Outcomes are based on code inspection and tests run for this review, rather than completion flags. Reviews 2 and 3 left no other earlier finding open. The earlier findings are also listed to make the repair history explicit.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R3-F1: Nested contract cache hits lose the defining schema's read permissions | R3-F1.1–R3-F1.11, checked individually below | **verified fixed** | In darkmatter, the nested contract host uses `RecordingGates`; allowed reads retain their defining file in `SlotDependencies.reads`. Contract memos, shared-cache hits, and pins replay that inventory. `SlotSession::serves` checks all original permissions before checking file contents. All six `nested_grants` tests pass. | None. This closes the previously reported sibling of R1-F1; it is not a new sibling or a partial repair. |
| R1-F1: Several slot paths perform reads before approval | Previously closed R1-F1.1–.20: inline positions, contract/schema readers, listings, direct controls, cache ordering, listing grants and diagnostic listing | **verified fixed** | The rerun `dynamic_schema_slot_read_order`, approval and cache-order tests pass. Inspection retains schema exclusion from ordinary interpolation, gates before reads, and permission checks before cache rehashes. R3-F1 now preserves the nested permission inventory as well. | None. |
| R1-F2: Overridden trigger slots still evaluate | Previously closed R1-F2.1–.12: layer overrides, preflight/runtime agreement, caller surfaces, cache identity, named types and root-union documents | **verified fixed** | The rerun precedence tests, CLI caller tests and editor discarded-trigger case pass. Shared surviving-property selection remains in use before slot resolution. | None. |
| R1-F3: Inline and trigger slot failures lose authored locations | Previously closed R1-F3.1–.28: callers, carriers, imported/referenced schemas, repeated slots, typed data/contract failures and unproven positions | **verified fixed** | The rerun position tests, CLI JSON diagnostic checks and editor authored-range cases pass. Typed nested causes and authored source mapping remain intact. | None. |
| R1-F4: Unsaved data buffers are ignored outside YAML and JSON | Previously closed R1-F4.1–.11: all supported data formats, Markdown, unusual-extension imports, closing buffers and dirty-buffer grants | **verified fixed** | The rerun editor `unsaved_buffers` tests and library buffer-close case pass; the format guard remains compiled. Buffer reads still avoid the disk-stat shortcut. | None. |
| R1-F5: Stale completion candidates have no stale marker | Previously closed R1-F5.1–.10: completion forms, hover, diagnostics, pending literals and value-inserting quick fixes | **verified fixed** | The rerun held-refresh and `stale_responses` protocol tests pass. Pending/stale notes remain shared across the affected projections. | None. |
| R2-F1: Cache reuse omits implicit and transitive request inputs | Previously closed R2-F1.1–.15 plus contract-memo dependency replay: clock, commands, addresses, packages, agents, nested and later-round inputs, volatility and controls | **verified fixed** | All rerun `session::tests::consumed` and `reuse` cases pass. Dependency collection still surrounds the entire resolution; nested memo replay carries captured calls, context inputs and volatility alongside the newly consolidated read inventory. | None. |
| R2-F2: Referenced schemas and validation sessions omit required function captures | Previously closed R2-F2.1–.10 plus referenced validating-session and request-based reference checks | **verified fixed** | All 73 `dynamic_schema_slot_captures` cases and seven actual editor-worker capture cases pass. `SessionObservations::covering` still provides missing captures from the supplied request authority; frozen supplied observations retain their existing contract. | None. |

### Required instances of R3-F1

The test fixture places the outer schema and data in a committed temporary repository and the contract in a shared folder outside it. Each read case first succeeds with broad grants, then requires approval with narrower grants in a new session, and also requires approval in an equivalent cold fixture. This distinguishes permission enforcement from an unrelated resolution failure.

For an independent caller-boundary check, I temporarily adapted the fixture's resolution helper to build a Markdown document with an inline copy of the schema and call darkmatter's public `DarkmatterSchemas::effective_for` API. A harmless additional schema property distinguishes the memo test's two template identities. All six cases passed again, including the memo-hit evaluation count. The direct external-template control retained its original path. The test file was restored byte-for-byte afterward; no probe remains in the implementation.

| Child ID | Required instance | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R3-F1.1 | Nested contract content read | **verified fixed** | `a_nested_contracts_content_read_needs_its_own_grant_warm_and_cold` passes: broad success, warm refusal and cold refusal. The read is recorded under the external contract's file. | None. |
| R3-F1.2 | Nested contract existence probe | **verified fixed** | `a_nested_contracts_existence_probe_needs_its_own_grant_warm_and_cold` passes with the same three outcomes. A repository-local target does not give an external defining schema an automatic grant. | None. |
| R3-F1.3 | Further nested contract resolution | **verified fixed** | `a_deeper_nested_contracts_probe_needs_its_own_grant_warm_and_cold` passes with `Pattern@./inner.yaml`. Recording reaches enclosing dependency collections at every depth. | None. |
| R3-F1.4 | Contract memo and replay | **verified fixed** | `a_memoized_nested_contract_replays_its_reads_for_another_command` passes. Its one-evaluation assertion proves the second template used the contract memo; warm and cold narrower requests both refuse. `ContractSlotInputs.dependencies` stores/replays the original reads. | None. |
| R3-F1.5 | Outer post-call observation check | **verified fixed** | The dispatcher still calls `authorize_slot_observations`. Its outer-origin record supplements the nested-origin record; `SlotDependencies::merge` does not replace one with the other. The warm refusals in .1–.4 demonstrate that the stricter requirement survives. | None. |
| R3-F1.6 | Shared-cache permission acceptance | **verified fixed** | `serves` checks every `entry.dependencies.reads` record with its own `defining` path before `InputCheck::holds`. The narrower warm requests refuse; the existing refused-before-rehash test also passes. | None. |
| R3-F1.7 | Ordinary top-level template control | **verified fixed** | `a_shared_template_read_directly_needs_its_grant_warm_and_cold` passes: broad success and both narrower refusals. | None. |
| R3-F1.8 | Imported definitions and later root-union rounds | **verified fixed** | `resolve_fresh` supplies one recording host to `PendingSchema::resolve`; the library's successive rounds use that same host. Imported/read-set controls and the later-round volatile-input test pass. The separate nested-host construction also now records. | None. |
| R3-F1.9 | Within-command pins | **verified fixed** | The once-per-command snapshot and rewrite-under-earlier-grant tests pass. `pinned` replays dependencies without asking for the same run's grant again, preserving the author's once-per-run decision. | None. |
| R3-F1.10 | Nested listings and remote reads | **verified fixed** | The nested listing test passes with broad success and both narrower refusals. Code invariant for URLs: automatic grants refuse them, and URL observations make a cache entry session-local. Explicit remote authorization therefore cannot become an automatic grant by changing origin. | None. |
| R3-F1.11 | Cache-hit and pin replay inside an enclosing collection | **verified fixed** | Both hit paths call `dependencies::replay`; its merge includes the same `reads` inventory that contract replay uses. This is a structural check: no additional production route that nests a shared-template cache lookup was found. The reachable contract-memo route is exercised by .4. | None. |

The owning code is darkmatter's [slot session](../../lib/src/markdown/schemas/slots/session.rs), which evaluates and authorizes snapshots, [dependency collector](../../lib/src/markdown/schemas/slots/dependencies.rs), which preserves nested input provenance, and [data runtime](../../lib/src/markdown/compose/expression/data/runtime.rs), which replays memoized contract inputs. Their comments now describe the consolidated inventory and same-thread evaluation invariant. Resolution remains synchronous on the thread that entered the collector; the editor invokes it on its dedicated worker. Removing the old separate read recorder therefore does not discard a supported cross-thread evaluation path.

## Unblocked Findings

None.

## Blocked Findings

None. No repair requires an author decision.

## Recurrence

No new recurring finding. R3-F1 was the previously reported permission-preservation sibling of review 1's R1-F1.18; its required instances are now verified fixed. The review-2 captured-input and capture-availability repairs remain effective.

## Spec coverage and verification level

| User-facing requirement | Verification reviewed and rerun | Assessment |
| --- | --- | --- |
| Passive template parse/query/export; explicit immutable resolution; imports and recursion; typed unresolved errors; static parity | L1 simplified-schema grammar, facade, resolution and parity tests, including the minimal-feature build | Appropriate. The host-neutral library still has no Darkmatter evaluator dependency. |
| Mixed enum/suggestion lists, shape checks, ordering/deduplication and delimiter-like literal data | L1 typed substitution and public schema-result tests | Appropriate; values enter the typed representation rather than being reparsed as source. |
| Literal/default/bound/pattern slots and pattern limits | L1 supported-position, invalid-result and pattern-limit tests | Appropriate. Whole-type slots remain explicitly sequenced into later work. |
| Positioned errors and preserved data failure codes/locations | L1 library results, CLI data output and actual language-server responses | Appropriate; previous authored-location and nested-cause fixes pass. |
| Real sandboxing schema, passive corpus admission and explicit document validation | L1 shipped-artifact, composition and CLI tests | Appropriate; schema-relative `../../../os.yaml` spelling remains present and the corpus exception is removed. |
| Permission scopes, grant store, preflight, computed targets and cache permissions | L1 scripted approval, read-order, warm/narrow/cold, composition and CLI wiring tests | Appropriate; all nested permission cases now enforce the original defining file. |
| Effective-layer precedence, passive triggers/kind checks/reference checks, mutation invalidation and read-set replay | L1 public-result, observation and evaluation-counter tests | Appropriate; retained behavior is exercised without external services. |
| Captured functions and changing implicit/transitive inputs | L1 changed-observation/cold-control cases, caller matrix and actual editor worker | Appropriate; the review-2 repairs pass. |
| Unsaved formats, dependency refresh, superseded workers, pending/stale completion/hover/diagnostics and affected quick fixes | L1 actual language-server protocol tests with controlled worker scheduling | Appropriate. These requirements concern protocol responses; they do not promise physical-key encoding, terminal painting or browser behavior. |

Commands run for this review:

- In `darkmatter`: `just test slot` — **443 passed**, none at or above five seconds.
- In `simplified-schema`: `just test slot` — **87 passed**, followed by the recipe's **1,437 passing minimal-feature tests**.
- At the repository root: `just check-tier-coverage darkmatter` — **no stranded tests**. The feature's integration modules are declared in their L1 targets, and the counting tests' feature is enabled by the test recipe and package CI metadata.
- Temporary public-API adaptation, `just test nested_grants` — **6 passed**, each below 0.24 seconds; original source restored afterward.

The linked review and repair logs contain additional full-suite and lint evidence; those claims were not independently rerun here. Cross-OS proof is left to CI and does not determine readiness. No formatter, implementation repair, commit, focused window, external network service or human-only verification was needed.

## Input robustness matrix

The feature's new configuration loader is the fixed-home, line-based schema read-grant store. Slot expressions are a text grammar; writers/renderers and existing data-file parsers do not add another configuration loader in this repair.

**Default outcome:** an invalid grant record rejects the entire store, grants nothing, and reports its path and line. A file read failure reports the path without inventing a line; invalid UTF-8 reports its detected line. **Format-defined exceptions:** a missing or empty store means no rules; comments, blank lines, CRLF and a UTF-8 byte-order mark are allowed; identical records deduplicate; denial overrides allowances.

I inspected the loading and parsing branches in [the grant store](../../lib/src/markdown/schemas/slots/grants/store.rs) and reran its loading/matching and CLI invalid-store wiring tests as part of the slot selection. Control cases cover all actions/scopes. Invalid cases cover missing/empty arguments, unknown verbs/scopes, invalid paths/URLs, query/site restrictions, encoding and unreadable files. Document cases cover duplicate records, accepted line syntax and write/read round trips. Null and collection-element shapes do not exist in this line format. The requested scan found no serde defaults, silent element filtering, default-on-parse-failure or ignored parse result in this loader. No input-shape defect was found.
