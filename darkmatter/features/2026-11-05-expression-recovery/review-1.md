---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Missing sources bypass invalid-request checks"
    - "R1-F2 (medium): Stale remote cache hides a denied redirect"
    - "R1-F3 (low): Value-only try calls return a generic handler error"
human_review: true
human_review_items:
    - "For later review: accept the guarded-read fixture for unreadable files; it exercises the real library read path without platform-specific permissions."
    - "For later review: accept using the shared data-error accessor in schema slots, although nested composition cannot currently reach that reader."
    - "For later review: accept the unrelated DMLS timing-test repair and leave elimination of duplicate production validations outside this feature."
    - "For later review: accept preserving contract-import errors' existing fatality grading while keeping them nonrecoverable by try."
    - "For later review: accept preserving Claudine's existing error codes for callback-wrapped file failures."
    - "For later review: accept inspecting the explicit infrastructure-error constructors rather than adding hooks to force runtime and HTTP-client startup failures."
    - "For later review: accept the warning-analysis exception that lets try accept null without hiding undeclared-property warnings."
    - "For later review: accept regenerating the function table before the documentation phase to keep catalog checks passing."
    - "For later review: accept unsupported-format precedence for missing files; R1-F1 asks for consistent enforcement of that decision."
    - "For later review: accept the real local provider failure instead of a provider-specific test hook."
    - "For later review: accept testing schema-slot classification through reads, cache behavior, and admission instead of adding a public accessor."
    - "For later review: accept library coverage of unreadable files in place of an additional spawned-command fixture."
    - "For later review: accept existing interpolation-level runtime locations; the fallback tests still execute a failing fallback inside try."
    - "For later review: accept malformed-reference tests at call checking and binding, where those inputs are actually rejected."
    - "For later review: accept the malformed contract-option fixture for runtime binding failure, because a numeric type argument converts to text."
    - "For later review: accept collection recovery tests in callbacks, predicates, and conditions; source positions retain their existing restricted grammar."
    - "For later review: accept inline child content as the nested-composition fixture, matching as_markdown's input contract."
    - "For later review: accept the additional lifecycle test showing that a side effect can create a previously missing source."
    - "For later review: accept the explicit u16 parse type used to repair an unrelated Windows test compilation error."
    - "For later review: accept full schema-slot documentation in the existing slot topic with a link from the constraints overview."
    - "For later review: accept the existing catalog fallibility convention and generated label; the recovery topic explains propagated argument failures."
    - "For later review: accept leaving the README and Claudine lifecycle overview unchanged because their summaries do not contradict recovery."
    - "For later review: replace the generic value-only rejection with a contract error as recommended in R1-F3; no author choice is needed before repair."
    - "For later review: accept combining tightly coupled implementation tasks and rerunning verification when a worker ended without a report."
    - "For later review: accept discarding verification from the wrong checkout and using the subsequent runs in this worktree."
    - "R1-F1 repair decision: a missing source whose candidates name different formats stays a recoverable miss when any candidate accepts the request; only when every candidate refuses is the first refusal (resolution order) reported. Alternative was rejecting on the first candidate's refusal."
    - "R1-F1 repair decision: an explicit-format applicability error now outranks a missing file (an existing load_data_options test asserted the opposite and was rewritten); a recursive reference is judged by its authored extension and named as written."
    - "R1-F2 repair decision: stale-on-failure is gated by the shared RemoteFetchFailure kind table (Transport only), so HTTP error statuses still serve stale while refusals, invalid references, and infrastructure failures propagate."
    - "R1-F3 repair decision: callback-evaluated functions (map, filter) invoked through value-only entry points also become ContractViolation, so a direct evaluate/call_function caller can no longer downgrade them to a lenient evaluation_failure warning."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-09T00:09:00-08:00
spec: 2026-11-05-expression-recovery/spec.md
implemented: true
description: "A **feature** review of `2026-11-05-expression-recovery/spec.md`"
feature: 2026-11-05-expression-recovery/review-1.md
next: 2026-11-05-expression-recovery/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-expression-recovery/log.md
implemented_by: claude/opus
---

# Expression recovery review, iteration 1

The feature is **not production ready**. Most recovery behavior is implemented
and verified, but missing sources can hide invalid requests, and a warm remote
cache can hide a policy refusal. A smaller error-classification mismatch also
remains. All three findings can be repaired within the existing decisions;
none needs a human ruling.

## Findings

### R1-F1 (high): Missing sources bypass invalid-request checks

**Authority: spec.** The error-boundary table says
`dm.data.invalid_options`, `dm.data.row_shape_conflict`, and
`dm.data.unsupported_format` must propagate because “the requested operation
is invalid.” The first acceptance criterion also explicitly requires invalid
`load_data` options and unsupported extensions to remain nonrecoverable.

**Defect class:** returning a recoverable missing-source error before checking
the request's format and row requirements hides author mistakes behind a
successful fallback.

In the **darkmatter** package,
[load_data_fn](../../lib/src/markdown/compose/expression/data/load.rs:100)
returns a missing-file error before the applicability check at line 141 and
row checks at lines 153 and 155. Literal mistakes often fail the passive call
check first, but file names, contracts, and options supplied through
frontmatter are ordinary runtime inputs. For those inputs, an invalid request
returns `[]` while the file is absent, then starts failing as soon as it exists.

For example, save this as `doc.md` in a temporary folder and run
`md compose doc.md --output json`:

```markdown
---
src: optional.tsv
delimiter: ";"
---
{{ as_json(try(load_data(src, "tuple([string, string])[]", {delimiter: delimiter}), [])) }}
```

With no `optional.tsv`, composition exits 0 and returns `[]`. Add a TSV file
containing `a\tb\n1\t2\n`: the same call exits 1 with
`dm.data.invalid_options`. A TSV delimiter other than a tab is an invalid
request in both cases. Incidental undeclared/uncertain-call warnings do not
change this distinction; the review's larger reproduction declared the
frontmatter fields as well.

The missing-source extension check has a sibling gap:
[unreadable_candidates](../../lib/src/markdown/compose/expression/data/load.rs:238)
returns immediately for recursive references. Consequently,
`try(load_data("%optional.xyz", "string[]"), [])` succeeds with `[]`, while
the nonrecursive `optional.xyz` correctly fails with unsupported format.

The sweep below used the shipped `md compose` path, temporary files, computed
`src` and `contract` frontmatter values, and one changed input at a time.
Every present-file control used valid source syntax so the indicated request
check, rather than parsing, determined its failure. “Fallback” means exit 0
with `[]`; “error” means exit 1 with no document. Child IDs are repair-ledger
entries, including clean controls.

| Child ID | Site / decision | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | Format applicability: YAML | Missing/present `.yaml`, computed `delimiter: ";"` | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.2 | Format applicability: JSON | Same, `.json` | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.3 | Format applicability: JSON Lines | Same, `.jsonl` | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.4 | Format applicability: JSON5 | Same, `.json5` | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.5 | Format applicability: TOML | Same, `.toml`, object contract | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.6 | Header applicability | Missing/present YAML, computed `header: false` | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.7 | TSV delimiter restriction | Missing/present TSV, computed semicolon | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.8 | Positional-row requirement | Missing/present CSV, object-row contract, computed `header: false` | Missing: fallback; present: row-shape conflict | Row-shape conflict in both |
| R1-F1.9 | Row classification: CSV | Missing/present CSV, computed `string[]` contract | Missing: fallback; present: row-shape conflict | Row-shape conflict in both |
| R1-F1.10 | Row classification: TSV | Same, TSV | Missing: fallback; present: row-shape conflict | Row-shape conflict in both |
| R1-F1.11 | Explicit format selection | Missing/present `.xyz`, computed `format: yaml` and semicolon | Missing: fallback; present: invalid options | Invalid options in both |
| R1-F1.12 | Recursive missing-source format check | `%optional.xyz`, no explicit format | Fallback | Unsupported-format error |
| R1-F1.13 | Nonrecursive missing-source format check | `optional.xyz`, missing/present | Unsupported-format error in both | Same; clean |
| R1-F1.14 | Supported recursive miss | `%optional.yaml` | Fallback | Same; clean |
| R1-F1.15 | Delimiter value validator | Missing/present CSV, computed `";;"` | Invalid options in both | Same; clean |
| R1-F1.16 | Container contract admission | Missing/present YAML, computed scalar `string` contract | Not-a-container error in both | Same; clean |
| R1-F1.17 | Document-local contract import | Missing/present YAML, computed `Name@this` | This-import-rejected error in both | Same; clean |
| R1-F1.18 | Contract-file resolution | Missing/present YAML, missing `Name@./types.yaml` | Contract-import error in both | Same; clean |
| R1-F1.19 | Valid request controls | Valid YAML array / CSV tuple rows, missing/present | Missing: fallback; present: loaded values | Same; clean |
| R1-F1.20 | Passive call checking | Literal invalid options or row request | Rejected before evaluation by existing options tests | Same; clean |
| R1-F1.21 | Recovery classifier and both executors | Missing source emitted by this loader | Correctly classify the emitted miss as recoverable | Keep classifier behavior; fix the premature loader decision |

The format aliases share canonical `DataFormat` values and the same
applicability validator; they have no independent decision to repair. Both
the ordinary evaluator and lifecycle executor consume the same typed result,
so changing their recovery classifiers would incorrectly stop valid misses.

**Recommended repair:** validate the effective format, option applicability,
and tabular contract wherever those facts are known before returning a
recoverable miss. Cover recursive references without inventing a resolved
identity or adding filesystem reads merely for validation. Preserve the
existing slot-approval ordering and valid missing-file behavior. Extend the
public-library recovery tests with computed inputs and keep one normal CLI
regression proving an invalid request cannot silently select its fallback.
This needs no new syntax or broad error-protection layer.

### R1-F2 (medium): Stale remote cache hides a denied redirect

**Authority: spec and defect.** The error-boundary table requires “Policy
refusal, including denied remote hosts” to propagate. The changed shared cache
code returns stale content after an actual redirect-policy refusal, so the
typed refusal never reaches expression recovery. This is an existing defect
in a site modified by this feature, rather than a regression caused by `try`.

**Defect class:** treating every failed revalidation as a transport outage
turns a denied request into a successful stale-cache read.

In the **darkmatter** package,
[fetch_with_cache](../../lib/src/markdown/compose/cache/remote_cache.rs:511)
matches `Err(_)` whenever freshness mode is `Fallback` and the stored response
does not require successful revalidation. It discards `RedirectBlocked` and
other refusal causes along with network errors. Its new typed error adapter
therefore cannot preserve the policy classification on this branch.

Reproduction used a local HTTP fixture with an allowed `127.0.0.1` origin:

1. Serve `["loaded"]` at `/data.json` with `Cache-Control: max-age=0`.
2. Compose `{{ as_json(try(load_data(URL, "string[]"), [])) }}` with
   `--allow host=127.0.0.1 --cache-root CACHE --remote-refresh --output json`.
   This positive control returns `["loaded"]` and seeds the transport cache.
3. Change that same endpoint to HTTP 302 with a location on `localhost`,
   which the request has not allowed. Repeat the command with the same cache.

The second run exits 0, returns `["loaded"]`, and has no diagnostics. With
an empty cache or `--remote-freshness strict`, it correctly exits 1 and reports
that the redirect was blocked by fetch policy. No denied target was fetched;
the defect is hidden refusal and successful publication of stale content.

The reader sweep repeated this fixture with cached Markdown for document
readers. Each had a successful warm-up control, followed by the same denied
redirect. The runtime's typed and string adapters share this one cache branch.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | `load_data` through typed remote adapter | Warm cache, fallback freshness, denied redirect | Cached array, success, no diagnostics | Propagate policy refusal |
| R1-F2.2 | `file_exists` through typed remote adapter | Same with Markdown URL | `true` | Propagate policy refusal |
| R1-F2.3 | `frontmatter` through `load_markdown` | Same | Cached object | Propagate policy refusal |
| R1-F2.4 | `markdown_title` through `load_markdown` | Same | Cached title | Propagate policy refusal |
| R1-F2.5 | `markdown_body_empty` through `load_markdown` | Same | Cached-body result `false` | Propagate policy refusal |
| R1-F2.6 | `validate_schema` through `load_markdown` | Same | `true` from cached document | Propagate policy refusal |
| R1-F2.7 | `try_frontmatter` through `load_markdown` | Same | `ok: true` and cached frontmatter | Receive the refusal and preserve this function's existing error-as-data behavior |
| R1-F2.8 | `::file` through string adapter | Same | Cached transclusion, exit 0 | Report failed remote read |
| R1-F2.9 | `::code` through string adapter | Same | Cached code block, exit 0 | Report failed remote read |
| R1-F2.10 | Approval discovery's remote child reader | Same `::file`, `md compose --shell` | Exit 0, no commands discovered | Report refused child read |
| R1-F2.11 | Cold-cache branch | Denied redirect, no stored body | Policy error, exit 1 | Same; clean |
| R1-F2.12 | Strict revalidation branch | Warm cache, denied redirect | Policy error, exit 1 | Same; clean |
| R1-F2.13 | Mandatory revalidation branch | Stored `Cache-Control: no-cache`, denied redirect | Error, exit 1 | Same; clean |
| R1-F2.14 | Initial host admission | Warm cache, origin host not allowed | Error, exit 1 | Same; clean |
| R1-F2.15 | Transport fallback control | Warm cache, allowed origin returns HTTP 500 | Cached array, success | Same; clean |

Fresh-cache reuse without any revalidation does not encounter a refusal and
is not this defect. The downstream recovery classifier correctly excludes
`RemoteReadKind::Policy`; it cannot correct a cache decision that already
converted the failure into success.

**Recommended repair:** restrict the shared stale-on-failure branch to typed
transport failures. Return policy, invalid-reference, and infrastructure
failures through the existing adapter. Add an in-process public-runtime
regression that warms a cache and then refuses a redirect, plus a transport
failure control. One representative `md compose` check proves command wiring;
the repair need not add a separate test for every reader in this table or
change `try_frontmatter`'s independent recovery contract.

### R1-F3 (low): Value-only try calls return a generic handler error

**Authority: spec.** “Direct value-only call APIs … must reject such an
invocation as a call contract error, as appropriate for their existing
lazy-function API.” In the **darkmatter** package,
[call_entry](../../lib/src/markdown/compose/expression/functions/mod.rs:1052)
returns `ExpressionError::Other` for `try`, representing a generic handler
failure rather than a violated call contract. The implementation log explicitly
records leaving this requirement unfinished.

The current test `direct_value_call_of_try_is_rejected` passes by asserting
the generic `Other` variant for `call_function("try", [null, 7], None)`.
Rejection and nonrecovery are correct; the error classification is not. The
same shared branch serves existing lazy functions, so keep their established
behavior in view when choosing the smallest consistent correction.

**Recommended repair:** use a typed contract rejection, such as the existing
`ExpressionError::ContractViolation`, for evaluator-owned invocation through
the value-only entry point, and update the test to assert that classification
and nonrecoverability. Preserve the useful explanation and leave normal
expression evaluation unchanged. No new public error variant or human choice
is necessary.

## Verification and acceptance coverage

This review inspected the implementation commits, spec, plan, implementation
log, changed error producers and projections, ordinary and lifecycle lazy
evaluators, static typing, slot admission, DMLS tests, generated documentation,
and the unrelated timing-test repair. The two requested parallel readers
independently gathered implementation choices before the code review.

These commands ran in this worktree during review:

| Command, from `darkmatter/` unless noted | Result |
|---|---|
| `just test expression_recovery` | 93 passed |
| `just test direct_value_call_of_try_is_rejected` | 1 passed; confirms the R1-F3 mismatch |
| `just test try` | 568 passed |
| `just test load_data_options` | 110 passed |
| Root: `just check-tier-coverage darkmatter` | Passed; zero stranded tests |
| Shipped CLI, temporary local and HTTP fixtures | Reproduced R1-F1 and R1-F2, including the controls above |

These filtered runs overlap; their counts are not a count of distinct tests.
A combined filter attempt failed in the recipe's shell quoting before running
tests; the successful named-filter runs above supplied the relevant coverage.
Linking emitted the existing large unwind-table warning, with no build failure.
No source changes, full-suite rerun, formatting, terminal windows, or live
external services were needed for this review. The implementation log records
full package tests and lint; those are recorded implementation evidence, not
commands rerun by this reviewer.

| User-facing requirement | Strongest relevant verification inspected | Assessment |
|---|---|---|
| Optional-data example through the normal reference check and composition | L1 shipped CLI positive, missing, malformed, and mismatch tests; guarded library unreadable-source test | Appropriate level; R1-F1 extends invalid-request cases |
| Lazy selection, falsy successes, one evaluation, failed/nested fallbacks, shared budget | L1 instrumented evaluator and composition tests | Appropriate |
| Fatal request, programming, policy, context, and resource failures | L1 classifier, binding, composition, and command tests | Appropriate level; R1-F1 and R1-F2 identify gaps |
| All enabled expression surfaces, lifecycle hosts, and schema slots | L1 composition surface, slot, md lifecycle, and Claudine lifecycle tests | Appropriate; restricted collection-source grammar stays restricted |
| Passive analysis, callback completion, hover, arity, unknown functions, and host-name collisions | L1 shared analysis and DMLS protocol tests | Appropriate; no real editor or keyboard behavior is promised |
| Reporting, retained warnings, blocking warnings, and callback provenance | L1 structured report and CLI envelope tests | Appropriate; fallback tests really execute the failing fallback inside `try` |
| Read set, memo reuse, different fallbacks, invalidation after effects | L1 public composition and data-runtime observations | Appropriate; existing author-approved missing-probe identities are retained |
| Union typing and the bounded empty-array exception | L1 passive type tests and DMLS callback-member completion | Appropriate |
| Catalog consistency and current documentation | L1 catalog/generated-page checks and review of the topic changes | Appropriate; preserve the catalog's existing fallibility convention |

The new recovery test modules are declared by the consolidated L1 targets,
and tier coverage confirms they are selected. These requirements concern
expression evaluation and structured output, not emulator rendering or key
encoding; Level 2 or Level 3 evidence is not required.

### Input robustness scope

Recovery adds a text-language form and typed error handling, rather than a new
configuration format or field validator. It does not change JSON, YAML,
JSON5, TOML, or delimited-document decoding. The expression arguments and
inserted text therefore do not acquire a configuration-field matrix from
this review. Existing source and options validation remains relevant to the
changed loader ordering: the options suite passed, and R1-F1 sweeps the
load-bearing request decisions that can be bypassed on a miss.

The requested smell search found `.ok()` in candidate planning and passive
contract classification, a table-row `filter_map`, and a display-only
`unwrap_or_default`. None newly coerces a configuration field to absence or
an empty collection. Candidate-planning uncertainty must remain conservative
when repairing R1-F1; it is not permission to suppress a proven-invalid
format. No unrelated document-format requirements are introduced here.

## Implementation decisions (informational)

Every item below can be accepted or repaired within the spec and prior author
decisions. The frontmatter exposes each for later review; none blocks a repair.

| Decision or deferred work | Review judgment |
|---|---|
| Simulate unreadability with the public guarded-read API after a directory fixture resolved as missing | Accept. Exercises the actual library read failure and works without permission tricks. Memo injection remains an alternative, not unfinished required work. |
| Apply the shared data-error accessor to schema slots as well as the three planned projection sites | Accept. A small internal consistency change; nested composition is currently forbidden there. |
| Repair the unrelated DMLS settings-change test by waiting for all validations and checking every event | Accept. The test no longer mistakes a second event for the next setting's result. Avoiding duplicate production validations was deferred as unrelated optimization. |
| Preserve prior contract-import fatality grading | Accept. Typed provenance still makes contract failures nonrecoverable; preserving existing unguarded grading follows the spec's compatibility requirement. |
| Preserve Claudine callback-wrapped file-failure codes | Accept. Changing those codes was not required; the wrapper-aware helper preserves existing output. |
| Inspect two explicit infrastructure-failure constructors rather than force startup failures with new hooks | Accept. The mapping is direct; the fetch-library client-build error has a test. Additional startup injection seams are not necessary for this feature. |
| Add a special absence-warning rule for `try`'s nullable operands | Accept. Required to admit successful null without suppressing undeclared-property warnings. |
| Regenerate the function table in Phase 3 rather than Phase 5 | Accept. Timing of generated-artifact maintenance has no product consequence. |
| Prefer unsupported format over a missing-file diagnostic when no supported format can apply | Accept the direction under the author's unsupported-format ruling. R1-F1 repairs incomplete enforcement, including recursive references. |
| Use a genuine local provider error rather than add a provider-specific test hook | Accept. It exercises the actual provider variant without external requests. |
| Test slot classification through admission, read observations, and invalidation rather than expose a private class | Accept. No public accessor is needed merely for a test; source inspection also confirms classification walks both arguments. |
| Cover unreadable sources in-process rather than manufacture permissions for a spawned CLI | Accept. The library test reaches the real read site; command-path wiring is covered with other failures. |
| Retain interpolation-level runtime locations rather than add per-call spans | Accept. The spec requires proven locations and preserved existing precision. Contrary to a potentially confusing log summary, both CLI location cases contain an actual failing fallback inside `try`; a separate earlier interpolation also recovers successfully. |
| Test malformed references at passive checking and computed binding | Accept. Those are the real rejection boundaries; inventing a later loader failure would test an unreachable path. |
| Use a malformed contract-option object for runtime binding rejection | Accept. A numeric type argument converts to text and reaches a different failure, so it would be the wrong fixture. |
| Test collection callbacks, predicates, and conditions; reject `try` in the restricted collection-source grammar | Accept. The spec explicitly forbids enabling expressions where they were disabled. |
| Use inline content for nested `as_markdown` failure | Accept. This matches the function's content argument. The author already accepted the nested-data marker during plan review. |
| Add a lifecycle test where a side effect creates a missing source | Accept. Directly protects recovery followed by legitimate regeneration. |
| Add an explicit `u16` parse type to an unrelated Windows test | Accept. The one-line correction resolves a genuine compile ambiguity without changing behavior. |
| Put full slot guidance in the existing schema-slots topic and link from the constraints overview | Accept. Keeps the detailed rules together for new readers. |
| Keep the catalog description and generated “Not fallible” label | Accept the existing convention shared by lazy functions: the form introduces no independent handler error. The surrounding documentation explains propagated operand failures. No catalog-wide metadata redesign is needed. |
| Leave the README and Claudine lifecycle overview unchanged | Accept. Their existing summaries do not meet the plan's conditional edit triggers. |
| Leave value-only lazy invocation classified as a generic error | Do not retain this departure: repair it under R1-F3. The recommendation is within the spec, so no author ruling is needed. |
| Combine tightly coupled implementation tasks and collect a worker's unreported work with independent reruns | Accept. This changes orchestration, not the product contract. |
| Discard verification accidentally run in the main checkout and rerun using absolute worktree paths | Accept. Only the replacement worktree results count. |

The earlier author decisions about missing-file probe dependencies, fatal
nested composition, typed failures from all read functions, lifecycle template
composition failures, and absent in-repository slot targets remain applicable.
No new evidence requires reopening them. Broader recovery of generic or
provider failures remains explicitly outside scope, not deferred required work.

This is the first review; there are no prior findings to carry forward.
