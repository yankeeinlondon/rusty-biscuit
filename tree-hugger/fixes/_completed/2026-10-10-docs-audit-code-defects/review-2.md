---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Prior review and repair-ledger files are unavailable
  - The CLI error documentation promises more than the wrapper changes
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T03:32:28-07:00"
spec: 2026-10-10-docs-audit-code-defects/spec.md
implemented: false
description: "A **fix** review of `2026-10-10-docs-audit-code-defects/spec.md`"
fix: 2026-10-10-docs-audit-code-defects/review-2.md
previous: 2026-10-10-docs-audit-code-defects/review-1.md
---

The implementation meets the available specification. No high, medium, or low findings were identified. The tree-hugger and rust-testing skills were used. The scope includes the committed severity, rule, and API-removal changes plus the uncommitted provenance and CLI changes; the implementation log's completion claims were checked against both.

**Verification limitation:** the requested `review-1.md` and `log.md` do not exist in this worktree. The available ledger is `implementation-log.md`. Neither missing file appears in `git log --all -- <requested paths>`, and neither exists at the corresponding main-checkout location identified in that ledger. Consequently, this review cannot certify closure of an unavailable earlier finding inventory, assess disputed findings, or establish recurrence against it. `ready: true` describes the implementation against the available spec, not a claim that unknown prior findings were repaired.

## Previous Finding Verification

| Finding ID/title | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| `review-1.md`: finding inventory unavailable | Prior body, frontmatter findings, instance tables, required repairs, and any earlier unresolved findings | unresolved | Requested file absent; no history for its path across available refs. The supplied `log.md` is also absent. `implementation-log.md` records four implementation phases, not an earlier review's repair inventory. | Restore the actual earlier review and its ledger to reconcile IDs, disputes, blockers, and recurrence. No known implementation repair can be inferred from missing evidence. |

The following table independently verifies **spec defects F1–F7**. These are specification identifiers, not invented identifiers for the missing review. The available ledger contains no disputed finding or unresolved design blocker. The specification's accepted decisions preceded the implementation: severity follows the registry; unused APIs are removed; modified vendored queries retain their upstream license; unknown CLI rule names warn and continue. None requires reopening.

## Specification and Implementation Verification

| Spec defect | Required instances and consumers | Outcome | Verification evidence | Remaining work or blocker |
|---|---|---|---|---|
| F1: inconsistent default severity | Generic query capture; `dead-code`, `undefined-symbol`, `unused-symbol`, `unused-import`, `undefined-module`; syntax diagnostics; unknown IDs and aliases; cached diagnostics; CLI policy and exit status | verified fixed | All seven emission paths in tree-hugger's [tree_file.rs](../../lib/src/file/tree_file.rs) use `registry_severity`; unknown IDs return `Info`, aliases resolve. [lint_severity_registry.rs](../../lib/tests/l1/lint_severity_registry.rs) checks emitted severity and metadata against every registered rule, with a guard requiring an emitting case for each. [lint_severity_agreement.rs](../../cli/tests/lint_severity_agreement.rs) verifies the shared fixture yields two warnings and CLI success. Extractor revision is 51; library stale-entry and warm-replay controls plus the CLI revision-50 row check rejection, recomputation, and subsequent hits. | None. |
| F2: rules and language coverage disagree | All 16 language lint queries; registry canonical IDs and aliases; six Rust-emitted rules; message table; list-rules output; PHP positive and negative calls; Python case sensitivity; policy consumers | verified fixed | [rule_registry_consistency.rs](../../lib/tests/l1/rule_registry_consistency.rs) compares the registry with every query's captures and the semantic rule list, checks removed IDs and message arms, and proves one-sided additions fail. Both rule versions are 1.1.0; Python is listed for `eval-call`; PHP `exec` metadata and exclusions are present. Public-library tests assert exact diagnostic lines for bare/global mixed-case `exec`, namespaced and other-function exclusions, mixed-case `eval`, and Python controls. Independent shipped-CLI checks below verify policy and status. List-rules tests check both JSON and plain output. | None. |
| F3: empty re-export API | Method definition, library exports, callers/tests, library docs, API skill; surviving Rust `ImportSymbol::is_reexport` | verified fixed | Removed-symbol search finds `reexported_symbols` only in the permitted release note. [tree_file.rs tests](../../lib/tests/l1/tree_file.rs) check `pub use a::B` marks B as a re-export and ordinary `use c::D` leaves D unmarked. Documentation and the API skill explain the surviving Rust-only mechanism. | None; other-language detection remains outside this spec. |
| F4: incorrect original-query license | Original overlay constructor for all kinds, Perl original constructor, vendored constructor including modified queries; inventory and docs | verified fixed | Both original constructors in [provenance.rs](../../lib/src/queries/provenance.rs) use `REPOSITORY_LICENSE`; vendored records use `Apache-2.0`. `every_record_carries_the_license_of_its_origin` checks both nonempty inventory partitions against literal expected licenses. Locals overlays share the same original constructor as lint, references, and comments, so their license does not depend on inventory selection. Provenance docs and the skill agree. | None. |
| F5: incomplete unused drift API | Module and public exports; phase-six test consumers and full audit workflow; retained provenance revisions; query docs and skill | verified fixed | All six removed API names appear only in the release-note exception when searching `lib`, `cli`, `docs`, and the tree-hugger skill. The remaining phase-six workflow checks provenance and compatibility; `all_queries_compile_with_provenance` checks a nonempty revision for every language/kind. Both packages compile in the area gates. No additional dependent package was found in the workspace manifest search. Manual refresh guidance remains in docs and the skill. | None. |
| F6: command errors print internal structure | Every `Err` escaping the shared entry point, including scan, current-directory, adapter, and serialization failures; five command reproductions; control-bearing path; existing usage, invalid-directory, and closed-pipe paths | verified fixed | [main.rs](../../cli/src/main.rs) routes every returned error through one neutralized `Display` wrapper and exit 1. [command_feedback.rs](../../cli/tests/command_feedback.rs) checks `symbols`, `refs`, `lint`, `imports`, and `god-files`, exact expected message content, one line, status, and absence of a struct dump. The Unix path test verifies escaping. Existing god-files missing/non-directory assertions remain unchanged; existing usage and pipe tests exercise their separate exits. | None. No separate implementation was added for individual error producers. |
| F7: unknown rule names silently do nothing | `deny`, `warn`, `allow`; repetitions within/across flags; known IDs, aliases, all/category selectors; warning escaping; selector cloning, policy, diagnostics, and exit status; list-rules dispatch | verified fixed | One check runs immediately after command-kind construction, before list-rules or scanning. It traverses all three parsed vectors, uses alias-aware `has_rule`, and deduplicates names. The library parser remains unchanged. Feedback tests cover each flag, repetition, aliases/special selectors, escaping, clean status, and an actual denied diagnostic. CLI policy clones the already-checked vectors and applies registry policy without another warning pass. Export-test grep found no rule flags affecting their empty-stderr assertions. Docs and release notes describe the warning. | None. `has_rule` is the existing equivalent of the nonexistent `is_known` named in the spec, as recorded in the ledger. |

## Verification Evidence and Test Levels

- `just test`, run from this worktree's absolute `tree-hugger` directory: **2,089 passed**, zero skipped, exit 0 (151.434 seconds). Nextest flagged 35 existing tests as slow; the changed CLI feedback tests each completed in under one second. The unrelated slow tests are outside this spec's changes.
- `just lint`: passed for both tree-hugger packages.
- Root `just check-tier-coverage tree-hugger`: passed, zero stranded tests. The new library modules are declared in `tests/l1/main.rs`; Cargo discovers both new CLI test files through default autotests. No new test has a higher-tier marker or feature gate. The shared fixture is declared as CLI `source-inputs` and read using `manifest_dir!()` in the CLI test.
- An independent scratch-repository CLI reproduction used PHP calls on lines 2–8: `exec`, `\exec`, `EXEC`, `Foo\exec`, `namespace\exec`, `eval`, `EVAL`. Expected and observed results were exactly three `exec-call` warnings and two `eval-call` warnings, with no namespaced findings. Adding `--deny exec-call` changed exactly the three exec severities to `Error` and status to 1. Adding `--deny exec-cal` kept those results and printed one unknown-rule warning.

L1 is the appropriate verification level for every changed acceptance criterion: library return values, cached data, metadata, plain stderr text, and process exit status. This spec adds no styled terminal view, keyboard interaction, or terminal input protocol requirement. No new L2 or L3 check is required. The ledger reports seven passing L2 checks and Windows feedback tests; those are implementer-reported evidence, not runs independently repeated in this review. Cross-OS proof is left to CI and does not block this review.

The input robustness matrix is **not applicable**: the change modifies source-code lint queries, severity lookup, provenance generation, and CLI output, not a configuration, manifest, lockfile, or cache reader. Tests edit cached payloads to verify invalidation; production deserialization behavior is unchanged.

## Unblocked Findings

None.

## Blocked Findings

None. The absent earlier review is a verification limitation, not evidence of a spec violation or an implementation repair awaiting a human decision.

## Recurrence

No recurring implementation defect was identified from the available records. Recurrence against the missing earlier review cannot be established; `recurrence: false` does not certify that historical comparison.

## Observations

### Prior review and repair-ledger files are unavailable

The earlier finding inventory and disputed-finding decisions cannot be reconstructed from completion flags. Restore the actual files before relying on this document as proof of second-iteration closure. The requested `next` update to `review-1.md` could not be made; creating a substitute would fabricate the history and lose the implementer's original metadata.

### The CLI error documentation promises more than the wrapper changes

The new exit-status paragraph says “A command that fails prints one `error: <message>` line.” The accepted scope concerns errors returned to `main`; existing usage errors and per-file failure reporting have separate paths. Narrow the paragraph to errors returned from command execution to the shared wrapper. This is a documentation observation about an overbroad guarantee, not a requirement to redesign unchanged error paths.
