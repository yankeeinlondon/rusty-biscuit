---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - Existing executable interpolation restrictions remain unchanged
  - Future containers remain outside the current runtime
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-14T22:22:08-07:00"
spec: "2026-10-23-shell-template-approval/spec.md"
implemented: false
description: "A **feature** review of `2026-10-23-shell-template-approval/spec.md`"
feature: "2026-10-23-shell-template-approval/review-4.md"
previous: "2026-10-23-shell-template-approval/review-3.md"
---

# Review 4

The feature is **production ready** for its current execution surfaces. Review 3's blocking finding is repaired, the earlier repairs remain covered, and this review found no new high, medium, or low findings. No human decision or manual test is required to close this review.

Reviewed the specification, all three previous reviews, implementation records, the combined staged and working-tree changes, and the relevant tests. Existing implementation changes were preserved; this review changes only the requested review and specification documents.

## Previous unblocked findings

Review 3 uses `## Findings`, rather than a separate `## Unblocked Findings` section. Its one finding was unblocked.

| Previous finding | Implementation checked | Verification and disposition |
| --- | --- | --- |
| Executable placeholders still offer unrestricted program approval | Darkmatter retains authored command text even when template parsing fails. Claudine carries that evidence through ordinary tasks, group members, and lifecycle preparation. Site approval withdraws the prefix choice and rejects an unoffered prefix or shape decision before persistence. | Repaired. Fresh public approval tests cover both program positions, primary/setup/teardown, first groups, standalone and referenced lifecycle stacks, both action forms, and repeated lifecycle resolution. Exact, once, deny, and exact blacklist retain their expected outcomes. |

The relevant boundaries are Darkmatter's [ShellCandidate](../../../darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs), which determines eligible approval choices from the authored command, and Claudine's [prompt_resolved_site](../../lib/src/harness/shell.rs), which presents those choices and persists eligible decisions. Claudine's [lifecycle preparation](../../lib/src/composition/preflight.rs) and [sequence graph preparation](../../lib/src/composition/sequence/preflight/mod.rs) now preserve the evidence those decisions need.

The repair's retained tests sweep the complete class from review 3:

| Site or control | Input checked | Result |
| --- | --- | --- |
| First ordinary task and first-group member: primary, setup, teardown | Executable placeholder alone and after a fixed first chain segment; short and long shell forms | Exact offered; prefix and shape absent; returning either unoffered choice fails without writing a grant |
| First referenced prompt and first-group prompt member: start, success, finalize, failure, blocked, loop gate | Same executable-placeholder commands and both shell forms | Same restrictions during condition-blind graph approval |
| Standalone lifecycle: those same six stacks | Same commands and forms, resolved twice before approval | Authored restrictions survive repeated resolution |
| Supported leading redirects and native-shell LF/CRLF boundaries | Executable placeholder following each curated redirect, including redirects after a line break | Prefix withdrawn; unoffered prefix decision rejected without persistence |
| Fixed-program counterparts | Argument placeholders, quoted line breaks, chains, and redirects | Prefix remains eligible; scripted first-task control saves `prefix git` |
| Later-task and late-read counterparts | Executable placeholders and unsupported template grammar | Refused before predecessor effects; prefix and once do not bypass validation |
| Document directives, shell blocks, frontmatter shells, and whole-value set shells | Executable interpolation | Existing earlier rejection retained; see observations |

Each adapter test exercises the six handler decisions and asserts the resulting approved membership and policy-file effects, rather than only checking the displayed flags. Shared candidate tests independently distinguish fixed and variable program words.

Review 2's first-task lifecycle grammar repair remains covered by the native CLI cases: first-task and first-group commands retain resolved-byte approval, while later tasks retain template-only restrictions. Review 1's shape selection, referenced lifecycle transport, and group-wide late-value barriers also pass the fresh selection.

## Findings

None. No additional abstraction, performance change, or test layer is justified by a defect established in this review.

## Recurrence

No finding recurs in this review. The recurring approval-eligibility class reported in review 3 is repaired across its enumerated adapters and controls.

## Verification and coverage

Fresh canonical nextest runs used four test workers. Selections overlap; the counts below are not a count of distinct tests.

| Command | Result |
| --- | --- |
| `cd claudine && just test shell_template` | 442 passed |
| `cd claudine && just test preflight` | 258 passed |
| `cd claudine && just test sequence_lifecycle` | 15 passed |
| `cd claudine && just test shell_approval_prompt` | 2 passed |
| `cd darkmatter && just test shell_template` | 75 passed |
| `cd darkmatter && just test shell_expansion::site` | 3 passed; cached-result blacklist enforcement and failed-persistence reservation recovery |
| `cd darkmatter && just test approval_prompt` | 9 passed |
| `just check-tier-coverage claudine` | Passed; zero stranded tests |
| `just check-tier-coverage darkmatter` | Passed; zero stranded tests |
| `git diff --check` | Passed |

No selected test exceeded five seconds. The macOS linker emitted its existing compact-unwind size warning; it did not fail compilation or testing. Implementation records additionally report full-area L1 and lint success for both affected areas; those full suites and lint were not rerun by this review. Cross-OS evidence remains a CI responsibility and does not determine readiness here.

All applicable requirements need **Level 1** verification: in-process discovery and validation, scripted approval input, policy files, native child argument recording, state transitions, and process supervision. The specification asks for scripted prompt sessions and makes no promise about terminal styling, emulator input encoding, or physical keyboard behavior requiring Level 2 or Level 3. No terminal window, real provider, or audio playback was used.

| Acceptance criterion | Strongest applicable verification reviewed | Assessment |
| --- | --- | --- |
| 1. Missing single-document values | Level 1: discovery and no-spawn CLI cases across shell surfaces | Covered |
| 2. Missing sequence values | Level 1: task/document checks and serial/parallel group barriers | Covered |
| 3. Store round trip | Level 1: persistence, reload, deduplication, and malformed rules | Covered |
| 4. Argument-list matching | Level 1: literal/hole positions, quotes, chains, and ordered redirects | Covered |
| 5. Type coverage relation | Level 1: each specified declaration/rule pairing | Covered |
| 6. Containing argument boundaries | Level 1: native recorders in both CLIs, lifecycle, and sequence tasks | Covered |
| 7. Runtime validation and errors that omit values | Level 1: type/constraint rejection, error causes, and no-spawn assertions | Covered |
| 8. Prompt choices | Level 1: scripted public handlers, CLI prompt tests, and resulting policy files | Previous finding repaired |
| 9. Late approval end to end | Level 1: fake providers, saved templates, and no-terminal refusal before step one | Covered using the permitted `outputs` spelling |
| 10. Loop re-audit | Level 1: native iteration records; reusable shape succeeds and exact approval refuses iteration two | Covered |
| 11. First resolved, later templates | Level 1: scripted mode choices and native first/later/first-group cases | Covered |
| 12. Unchanged paths | Level 1: byte-grammar controls, exact/prefix cases, forbidden roots, and initialization refusals | Covered |
| 13. Runtime policy | Level 1: filled-chain checks, built-in/user blacklists, cached results, and preapproved membership | Covered |
| 14. Grammar and argument boundaries | Level 1: parser corpus, native recorder payloads, redirects, and interpreter-target refusal | Covered |
| 15. Approval versus results | Level 1: changed values/types/constraints, distinct cache modes, failed persistence, and waiting-peer release | Covered |
| 16. State and discovery | Level 1: current mutations, parameters, child ownership, groups, late snapshots, and passive discovery | Covered for current execution surfaces; future containers noted below |
| 17. Schema provenance | Level 1: named/local schemas, untyped fallback, scalar conversion, numeric/union disclosure, and malformed rules | Covered |

The test targets and module declarations include the new recorder binaries and consolidated L1 modules. Their fixture features are enabled by the local recipes and CI metadata. The fresh tier audits found no stranded tests. Test inputs use private fixtures rather than live internal prompts.

### Input robustness matrix

The line-oriented whitelist and blacklist are the changed file format. Authored shell templates are a text grammar and are outside this matrix. The load-bearing fields are each rule's keyword and shape payload on both policy sides. The fresh Darkmatter selection runs the persisted-rule matrix and the public-composition matrix, starting from rules written by the real persistence API and observing authorization or denial.

| Shape | Whitelist keyword | Whitelist shape payload | Blacklist keyword | Blacklist shape payload |
| --- | --- | --- | --- | --- |
| Positive control | Matching rule authorizes | Matching shape authorizes | Matching rule denies | Matching shape denies |
| Absent | Malformed row warns and skips | Missing payload warns and skips | Same | Same |
| Explicit null text | Unknown keyword warns and skips | Literal program `null`, not typed null; does not match control | Same | Same |
| Wrong whole-field type | Numeric/collection keyword warns and skips | Invalid collection syntax warns and skips; valid literal command keeps its meaning | Same | Same |
| One wrong element | Scalar field; not applicable | Invalid hole rejects entire row | Same | Same |
| Every element wrong | Scalar field; not applicable | All-invalid holes reject entire row | Same | Same |
| Empty | Empty keyword warns and skips | Empty payload warns and skips; empty file contains no rules | Same | Same |
| Duplicate | Duplicate rule deduplicates | Equivalent parsed shape deduplicates | Same | Same |
| Trailing/invalid content | Unknown keyword warns and skips | Invalid syntax warns and skips; valid extra argument changes shape; malformed later row preserves earlier rules | Same | Same |

The format has no typed null fields or mapping keys. No permissive parsing defect was established.

## Observations

### Existing executable interpolation restrictions remain unchanged

Darkmatter's document directives, shell blocks, and frontmatter shells, plus Claudine's whole-value set shells, already reject interpolated executables before approval. Existing refusal controls still pass. As in the earlier reviews, expanding those surfaces is separate work and is not required to close this approval-choice repair.

### Future containers remain outside the current runtime

Nested sequence execution and group terminal lifecycle events still have no execution surface. Their future acceptance examples are not counted as tested here. The specification expressly leaves container expansion to later work; the current group traversal and first-task behavior are covered.
