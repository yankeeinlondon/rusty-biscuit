---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
    - Optional confirmation after the cycle; retain full policy loading during approval so an invalid policy root still denies the command.
    - Optional confirmation after the cycle; let hover and completion discover independently, sharing a host only within combined diagnostics.
    - Optional confirmation after the cycle; accept the reordered implementation, with public ownership changes completed separately from dependent work.
    - Optional confirmation after the cycle; accept the catalog and verb-list wrapper names and content-based equality.
    - Optional confirmation after the cycle; preserve slice access to owned global names and adapt them through a private type-projection helper.
    - Optional confirmation after the cycle; read files before locking, then synchronize cache lookup, construction, and replacement.
    - Optional confirmation after the cycle; keep separate cached outcomes for successful profiles, refused artifacts, and non-host files.
    - Optional confirmation after the cycle; evict invalid catalogs rather than retain their parse errors, allowing a later visit to retry.
    - Optional confirmation after the cycle; remove vanished paths when visiting a root, treat an unreadable root as empty, and leave present shadowed paths cached.
    - Optional confirmation after the cycle; reuse the existing test instrumentation feature for per-thread discovery observations and injected failures.
    - Optional confirmation after the cycle; use two default provider methods to supply one shared diagnostics host while retaining standalone entry points.
    - Optional confirmation after the cycle; log a shared discovery panic for each affected provider and omit only those providers' diagnostics.
    - Optional confirmation after the cycle; skip host discovery for documents without frontmatter.
    - Optional confirmation after the cycle; inject a private atomic writer to test both ElevenLabs publications on their production paths.
    - Optional confirmation after the cycle; report an audio blocking-task failure as a temporary-file error with publication context.
    - Optional confirmation after the cycle; use asynchronous writer-entry notifications and a separate 1.5-second synchronous watchdog in publication tests.
    - Optional confirmation after the cycle; observe policy reads per thread and use relative policy roots to test path-derivation failure.
    - Optional confirmation after the cycle; split policy loading and shell tests into separate files to meet repository file-size limits.
    - Optional confirmation after the cycle; inject a capacity-one cache through the production assertion builder and prove eviction through shared-reference ownership.
    - Optional confirmation after the cycle; enable long paths during the initial Windows verification clone to prevent checkout failure.
    - Optional confirmation after the cycle; reuse unchanged-source test, lint, example, and benchmark evidence from the preceding implementation phase.
    - Optional confirmation after the cycle; add the missing Tokio dependency documentation entry and leave unaffected READMEs unchanged.
    - Optional confirmation after the cycle; let two timing-sensitive DMLS tests recognize both normal and slow completion logs while still requiring exactly one completion.
    - Optional confirmation after the cycle; accept the following host-function-catalog plan's clarification of its distinct cache policy and reuse of discovery instrumentation.
    - Optional confirmation after the cycle; accept that Linux and WSL2 execution was left to CI, with macOS and native Windows evidence recorded by the implementer.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-10-29T12:50:50-07:00
spec: 2026-10-25-fix-now/spec.md
implemented: false
description: A **fix** review of `2026-10-25-fix-now/spec.md`
fix: 2026-10-25-fix-now/review-1.md
---

# Review 1: Fix Now

The fix is production ready. No findings were identified against the seven specified changes. The implementation preserves the required registration, artifact, validation, and approval behavior, with the two diagnostic changes the spec explicitly permits.

## Findings

None. This is the first implementation review, so there are no earlier findings to reconcile.

## Scope and verification

Reviewed the specification, implementation plan and log, the implementation commits, current production code, test wiring, and affected documentation. Later work has also changed some shared schema files; this review checks the current implementation against this fix's contract without treating separately scoped host-function work as missing scope.

All requirements need Level 1 verification: ownership, file reads, parsing, cache eviction, runtime scheduling, and in-process command dispatch. No requirement involves terminal rendering, keyboard encoding, or browser behavior, so Level 2 and Level 3 evidence is unnecessary.

| Spec requirement | Verification reviewed | Result |
| --- | --- | --- |
| Owned profiles and separately retained registries release obsolete data | Weak references to the profile, registry, catalog, and owned verb list across catalog and artifact replacement; old holders retain their original behavior | Meets the contract |
| Artifact failures are cached without partial catalogs; unusable catalogs invalidate prior success | Discovery build counts, cached-outcome inspection, injected read failures, removal and content changes, followed by artifact repair | Meets the contract |
| Static registrations and artifact bytes remain compatible; loading uses observed bytes | Owned/static round-trip tests, committed-host artifact checks, shared text-loader tests with absent or different on-disk content; runtime leak search | Meets the contract |
| Discovery stops at its first usable profile and diagnostics share one host | Production read observations, root order and filename shadowing, combined-versus-standalone diagnostic comparison, next-request repair, and injected discovery panic | Meets the contract |
| Audio and voice-record publications leave the async worker free | Current-thread runtime tests gate the actual blocking closure, including a record on an audio cache hit; ordinary errors, task panics, caller cancellation, and staging cleanup are exercised | Meets the contract |
| Validator eviction preserves live assertions | Capacity-one cache supplied to the production assertion-building helper; eviction proved independently of retained assertion behavior; syntax-only file checks, custom formats, and error context checked | Meets the contract |
| Named imports retain useful errors with one preliminary parse removed | Public resolver tests load malformed, numerically inadmissible, non-schema, and unsupported-envelope files; inspection confirms the redundant deserialization is gone | Meets the contract |
| Definition projection preserves identity without cloning the replaced body | Explicit field construction retains keys, origin, JSON name, order, and lookup map; existing recursive-definition and lowering coverage | Meets the contract |
| Runtime policy reads only fresh blacklist rules while approval still reads both files | Actual lifecycle dispatch with file-load observations, changed rules between dispatches, missing/unreadable/underivable roots, built-in refusals, malformed-line warnings, and full approval controls | Meets the contract |
| Scoped implementation gates and compiled consumers | Log records full area tests/lints, example and benchmark checks, and native Windows focused checks; new integration tests are declared in their consolidated targets and instrumentation features are enabled | Meets the implementation verification requirement |

The implementation log records final full-suite passes of 12,593 Darkmatter-area tests, 11,654 Claudine-area tests, and 597 Biscuit Speaks-area tests, plus passing area lints. These are implementation evidence, not new runs claimed by this review. Cross-OS proof is left to CI as the review instructions require.

### Review-time checks

Focused review-time L1 executions on macOS:

- `cd biscuit-speaks && just test elevenlabs`: 58 passed, including all five new publication tests.
- `cd darkmatter && just test catalog::`: 174 passed, including lifecycle discovery and the global-catalog validation matrix.

Remaining focused diagnostics, import, assertion, and policy executions are recorded after completion. No production source was modified during review.

## Input robustness

The changed lifecycle artifact fields, `host_globals` and `unknown_verbs.ignore_unprovided`, use strict required-array and element-string validators. Through the public profile result, tests separately reject absent, explicit null, wrong whole-field type, mixed elements, and wholly invalid elements. Empty arrays are valid and their resulting behavior is asserted; valid owned collections round-trip through serialization and parsing.

The host-catalog text helper retains the strict catalog loader as its authority. Its permissive name probe cannot turn a malformed field into an accepted catalog: public loader tests cover missing, null, mistyped, empty, duplicate, and trailing forms, and helper parity cases cover duplicate and non-string keys, invalid declarations, and forbidden built-in globals. The named-import tests exercise the public resolver with valid controls, invalid YAML, duplicate keys, trailing content, a second YAML document, implicit null, numeric admission failure, and incompatible envelopes. Shared field validators retain their existing coverage; this fix adds no permissive deserializer defaults.

The requested code-smell search found permissive operations in the catalog name probe, discovery skips, and policy fallbacks. Each has a defined outcome: strict loading still rejects invalid catalog content; unreadable discovery candidates are skipped and invalidate their old entry; unavailable user rules are empty while built-in rules remain enforced. None introduces a load-bearing silent coercion.

## Implementation decisions (informational)

Two independent readers examined the log and spec in parallel before the comparison. The log has no `decisions_for_review` list or “Decided during implementation” heading; its three “Design Decisions” sections provide the inventory, and `decisions_for_author` is empty. The following choices are recommended or permitted within the spec. The author may confirm or reverse them after the cycle; none needs a ruling before repair or release readiness can be assessed.

| Implementer choice | Review assessment and consequence |
| --- | --- |
| Keep full policy loading during approval | Recommended. It preserves the required denial for an underivable root; only runtime blacklist loading changes. |
| Leave hover/completion discovery independent | Required scope boundary. Only combined diagnostics must share a host; broader caching was excluded. |
| Complete independent work before the public ownership migration, then finish discovery and dispatcher changes sequentially | Reasonable execution order; avoids verification against half-edited dependencies. |
| Name the wrappers [CatalogHandle](../../../darkmatter/lib/src/markdown/schemas/catalog.rs) and [VerbList](../../../darkmatter/lib/src/lifecycle/parse.rs), and compare their contents | Reasonable names within the spec's authorized API migration; content equality preserves comparisons between compiled and rebuilt registrations. |
| Retain slice access to owned host-global names and add a private projection adapter | Recommended. Existing public projection callers keep their interface. |
| Read files outside the mutex, then lock lookup/build/replacement | Meets observed-byte and synchronized replacement requirements. It serializes builds across paths, a small and explicit trade-off. |
| Store successful profiles, artifact refusal reasons, and non-host classifications as distinct outcomes | Matches the required lifetime and classification rules. |
| Evict malformed catalogs instead of caching their parse errors | Permitted. The spec requires caching failed artifacts, while catalog failures must invalidate prior success and remain skipped. |
| Prune removed paths from visited root listings, including unreadable roots; retain present shadowed paths | Reasonable. It releases obsolete entries without reading catalog contents beyond the winner. Unvisited roots may retain entries as the spec allows. |
| Reuse `work-counters` for per-thread discovery observation and injected failures | Recommended bounded test instrumentation; production reads remain unchanged. |
| Add default provider hooks for host participation and shared-host diagnostics | Reasonable plumbing that preserves standalone entry points and per-provider panic isolation. |
| Log a discovery panic for each host-dependent provider and drop only those contributions | Preserves the existing observable panic treatment and unaffected-provider results. |
| Skip discovery when no frontmatter exists | Preserves the earlier consumers' behavior and avoids needless reads. |
| Inject a private atomic writer into ElevenLabs and pass it into snapshot publication | Recommended. Scheduling tests observe the actual publication closures rather than substitute their behavior. |
| Give audio task-join errors temporary-file error context | Matches the spec's error category; record failures remain warning-only. |
| Combine asynchronous entry notification with a 1.5-second synchronous watchdog | Meets the bounded scheduling-test requirement, including failure when the writer runs inline. |
| Observe policy reads per thread and test an underivable root with a relative path | Reasonable deterministic alternatives to permissions-based tests and ambient state. |
| Split shell policy and inline tests into separate files | Justified by the repository's file-size guard; public paths remain available. |
| Inject capacity one through a private assertion builder and inspect shared-reference ownership | Meets the eviction proof without adding a cache inspection API. |
| Enable long paths on the initial native Windows verification clone | Small repair to a real checkout failure; it changes no application behavior. |
| Reuse previous-phase gates and consumer compilation where source did not change | Proportionate evidence reuse. The changed DMLS tests were rerun with the Darkmatter gates. |
| Add a missing Tokio dependency paragraph; leave unaffected READMEs alone | Corrects the plan's mistaken assumption that an entry already existed; current behavior is documented in topic pages. |
| Accept normal or slow-request completion messages in two existing DMLS tests | Fixes load-sensitive assertions while retaining exactly-one-completion verification. |
| Clarify the following host-function-catalog spec/plan's different caching policy and reuse the observer | Keeps the next plan accurate without changing this fix's scope or adding application behavior. |
| Leave Linux and WSL2 execution to CI | Reported evidence limitation, not a readiness finding. The log separately records macOS and native Windows verification. |

No requested functionality was deferred. Unrepeated unchanged-source checks and unexecuted OS checks are the verification omissions listed above. Host-function catalogs, broader editor caching, asynchronous existence probes, other speech providers, group targets through `prep`, and the other pipeline findings were explicit exclusions rather than new deferrals.

The noted lifecycle-profile test near the five-second budget is existing coverage. Its recorded profile attributes the time to schema compilation and validation, so the log does not establish a new performance defect in this fix.
