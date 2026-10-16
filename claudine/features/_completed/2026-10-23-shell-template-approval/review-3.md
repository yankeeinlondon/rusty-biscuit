---
$schema: feature-review.yaml
ready: false
findings:
  - "high: Executable placeholders still offer unrestricted program approval"
observations:
  - Existing executable interpolation restrictions remain unchanged
  - Future containers remain outside the current runtime
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-14T21:48:13-07:00"
spec: "2026-10-23-shell-template-approval/spec.md"
implemented: true
description: "A **feature** review of `2026-10-23-shell-template-approval/spec.md`"
feature: "2026-10-23-shell-template-approval/review-3.md"
previous: "2026-10-23-shell-template-approval/review-2.md"
next: "2026-10-23-shell-template-approval/review-4.md"
log: "claudine/features/2026-10-23-shell-template-approval/log.md"
implemented_by: "codex/gpt-6.1-sol"
---

# Review 3

The feature is **not production ready**. The previous blocking finding is repaired. One explicit approval-choice requirement remains unmet: commands whose executable comes from a placeholder can still offer and save unrestricted program approval. No human decision is needed for this correction.

Reviewed the specification, both earlier reviews, the implementation logs, and the combined staged and working-tree implementation. Existing implementation changes were preserved.

## Previous unblocked findings

Review 2 uses `## Findings`, with one unblocked finding, rather than a separate `## Unblocked Findings` section.

| Previous finding | Implementation and verification | Disposition |
| --- | --- | --- |
| First-task prompt lifecycles incorrectly require template grammar | Claudine's referenced-lifecycle preparation now determines first-task eligibility and direct/derived late reads before requiring template grammar. Fresh tests cover all six events, both shell forms, first-group members, authored chains, passive audit, and later preparation. Later-task, late-read, and forbidden-root controls continue to refuse execution. | Repaired. |

Review 1's first-task shape selection, referenced lifecycle template execution, and serial/parallel group validation repairs also remain covered by the fresh shell-template selection.

## Findings

### High: Executable placeholders still offer unrestricted program approval

**Authority — spec:** section 5 says a command whose program word contains a placeholder is “offered only exact, once, deny, and blacklist when resolved-byte approval is eligible.” Section 8 limits “This program with any arguments” to “fixed program words.” Acceptance criterion 8 requires the specified prompt choices. A configured executable, such as `tool: argv-recorder` with `shell: "{{ tool }} probe"`, is a realistic input explicitly exercised by the previous repair.

**Defect class:** resolved-byte approval loses the authored executable's eligibility restrictions and offers a broader persistent approval than the spec permits.

In the Claudine library, [prompt_resolved_site](../../lib/src/harness/shell.rs:418), which offers choices for an authored shell site, returns to the byte-only approval path when the site has no template shape. An executable placeholder necessarily has no valid template shape. The [byte-only approval request](../../lib/src/harness/shell.rs:212) then uses unrestricted resolved-byte choices and accepts `AllowCommandPersist`, saving a prefix rule. The authored restriction has been lost. Ordinary sequence tasks and standalone/referenced lifecycle actions all reach this fallback. Darkmatter's [ShellCandidate::offered](../../../darkmatter/lib/src/markdown/compose/shell_expansion/template/candidate.rs:159), the shared candidate-choice projection, likewise enables prefix approval whenever resolved bytes exist, even if template parsing failed because the program contains a hole. Correcting that projection alone would not repair Claudine's fallback.

Reproduction uses the existing sequence fixture with one authored command changed:

```yaml
---
tool: argv-recorder
sequence:
  - name: probe
    shell: "{{ tool }} probe"
---
Body.
```

Call Claudine's public graph builder and `resolve_graph_shell_approvals` with a private policy directory and a scripted approval handler, as existing prompt-choice tests do. The handler receives resolved command `argv-recorder probe` with `offered.prefix == true`. Returning `AllowCommandPersist` succeeds and writes:

```text
prefix argv-recorder
```

The spec requires that choice to be absent. Exact, once, deny, and exact blacklist remain available. This is not an unconsented command execution: the user selected the broad choice. It is nevertheless the wrong choice set and a persistent grant the spec explicitly excludes.

The sweep moved that same command between every applicable approval adapter. It also placed the executable placeholder after a fixed first chain segment: `git --version && {{ tool }} probe`. Scripted handlers observe the public approval request and policy-file result; no native process is needed to establish this prompt/persistence defect.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| First ordinary task: primary shell | Configured executable, alone and in the second chain segment | Prefix choice offered; prefix persistence accepted | No prefix choice or acceptance |
| First ordinary task: setup and teardown shell | Both short and long action forms; both program positions | Same | Same |
| First-group member: primary, setup, teardown | Same commands and action forms | Same | Same |
| First prompt document: start, success, finalize, failure, blocked, loop gate | Both shell forms; both program positions | Every stack offers prefix approval and accepts persistence during condition-blind audit | No prefix choice or acceptance |
| First-group prompt member: those same six stacks | Same | Same | Same |
| Standalone document lifecycle: those same six stacks | Same | Same through `resolve_shell_approvals` after lifecycle resolution | Same restriction as first-task lifecycle |
| Later prompt lifecycle: those six stacks, both shell forms | Configured executable, exact-only policy | Refuses template grammar before predecessor effects | Clean; retained native CLI controls |
| First-task lifecycle with a late executable or late argument in unsupported grammar | Late outputs read; prefix policy | Refuses before execution | Clean; retained native CLI controls |
| Document directive, shell block, frontmatter shell; standalone and first-task prompt | Configured executable | Existing inserted-executable rejection, before approval | Unchanged; not affected by this finding |
| Whole-value set shell: all six referenced lifecycle stacks and both task stages, including first-group members | Configured executable | Existing executable-insertion rejection | Unchanged; not affected by this finding |
| Fixed executable with an argument placeholder: standalone, first task, later task | Existing scripted exact/shape/prefix fixtures | Prefix remains offered and saves the program rule | Clean; preserve this behavior |

Retain whether any authored program word contains a placeholder through approval, including grammar-invalid candidates that may still run as resolved bytes. Use it both to hide prefix approval and to reject a handler returning that unoffered decision. Keep supported exact/once/blacklist choices and fixed-program prefix behavior. This is a bounded approval-metadata correction; it does not require new quoting, escaping, or execution machinery. Add scripted public approval regressions for the listed adapter families, including the second program of a chain.

## Recurrence

This repeats the approval-eligibility loss class from review-1's **First-task shell sites cannot use template approvals** finding. That repair preserved either-mode candidates when a valid shape exists, but the fallback for candidates without a valid shape still discards authored restrictions. The earlier sweep should have checked which choices remain available for executable-placeholder commands across primary tasks, setup/teardown, first-group members, and standalone/referenced lifecycle actions. This review sweeps all those sites and the parser/refusal controls. Review 2's grammar-order defect is repaired; the present finding does not ask to impose template grammar on resolved-byte commands again. The failing case is explicitly bounded by section 5.

## Verification and coverage

Fresh canonical nextest selections used four workers:

| Command | Result |
| --- | --- |
| `cd claudine && just test shell_template` | 344 passed, including all 84 byte-grammar CLI regressions |
| `cd claudine && just test sequence_lifecycle` | 15 passed, including the 13 new preparation regressions |
| `cd darkmatter && just test shell_template` | 74 passed |
| `cd claudine && just test preflight` | 160 passed |
| `just check-tier-coverage claudine` | Passed; zero stranded tests |
| `just check-tier-coverage darkmatter` | Passed; zero stranded tests |
| `git diff --check` | Passed |

The temporary public-API diagnostic walkers passed their assertions of the **observed defect** across sequence task/group and lifecycle adapters, including both program positions. A separate Darkmatter diagnostic confirmed the existing executable-insertion rejection for directives, blocks, and frontmatter. The first chain diagnostic incorrectly expected a particular prefix-rule spelling; that expectation was corrected to assert the relevant outcome, acceptance and persistence of a prefix approval. This does not establish that the command's contract passed. All temporary test edits were restored byte for byte. No production code or retained tests were changed by this review.

The implementation log records full-area Claudine L1 (9,561 passing tests), lint, and tier checks after the previous repair. Those are implementation evidence, not fresh full-suite or lint runs by this review. The local compact-unwind linker warning was informational. Cross-OS execution is left to CI and does not determine readiness here.

All applicable behavior requires Level 1: passive discovery, scripted approval choices, policy persistence, process arguments, runtime state, and process cleanup. The spec explicitly requests scripted prompts and has no terminal rendering or keyboard-encoding promise needing Level 2 or Level 3. New regression tests are compiled through existing unit-test modules and the declared CLI L1 target; fixture-enabled recipes select the CLI cases. No focus-taking terminal or real provider was launched.

| Acceptance criterion | Strongest verification reviewed | Assessment |
| --- | --- | --- |
| 1. Missing single-document values | Level 1: discovery and CLI no-spawn cases | Covered |
| 2. Missing sequence values | Level 1: task/document and serial/parallel group barriers | Covered |
| 3. Store round trip | Level 1: repeated policy persistence and loading | Covered |
| 4. Argument-list matching | Level 1: parsed shape tables | Covered |
| 5. Covers relation | Level 1: specified type-relation table | Covered |
| 6. Argument boundaries | Level 1: native recorder CLI tests | Covered |
| 7. Validation and secret-free errors | Level 1: typed rejection/error chains and no-spawn assertions | Covered |
| 8. Prompt choices | Level 1: scripted handlers and persisted policy | Executable-placeholder prefix restriction fails above |
| 9. Late approval end to end | Level 1: fake provider, saved rules, no-handler refusals | Covered with current `outputs` spelling |
| 10. Loop re-audit | Level 1: iteration records and exact-rule refusal | Covered |
| 11. First resolved, later templates | Level 1: first/later/group native and scripted cases | Previous grammar defect repaired |
| 12. Unchanged paths | Level 1: refusal, initialization, standalone and forbidden-root controls | Covered |
| 13. Runtime policy | Level 1: full-chain, user/built-in blacklist, cached/preapproved checks | Covered |
| 14. Grammar and arguments | Level 1: parser corpus, native recorder, target refusal, redirections | Covered |
| 15. Approval versus results | Level 1: changed fills/types/constraints, separate caches, persistence failure | Covered |
| 16. State and discovery | Level 1: current mutations, params, child documents, groups, late snapshots | Covered for current runtime surfaces |
| 17. Schema provenance | Level 1: local/named schemas, fallback, numeric/union, malformed rules | Covered |

The input robustness matrix applies to the line-oriented whitelist and blacklist, not to Markdown template grammar. Fresh Darkmatter tests walk rules written by the real persistence API and test the resulting authorization/denial; another matrix drives public composition. Both policy sides cover the keyword and shape payload:

| Input shape | Whitelist keyword | Whitelist payload | Blacklist keyword | Blacklist payload |
| --- | --- | --- | --- | --- |
| Control | Matching shape authorizes | Matching shape authorizes | Matching shape denies | Matching shape denies |
| Absent | Malformed row warns/skips | Missing payload warns/skips | Same | Same |
| Explicit null text | Unknown keyword warns/skips | Literal program `null`; no typed null | Same | Same |
| Wrong whole-field type | Numeric/collection keyword warns/skips | Invalid grammar warns/skips; literal command retains meaning | Same | Same |
| One wrong element | Scalar field; not applicable | Invalid hole rejects entire row | Same | Same |
| Every element wrong | Scalar field; not applicable | All-invalid holes reject entire row | Same | Same |
| Empty | Empty keyword warns/skips | Empty payload warns/skips; blank file has no rules | Same | Same |
| Duplicate | Duplicate rule deduplicates | Parsed equivalent shape deduplicates | Same | Same |
| Trailing/invalid content | Unknown keyword warns/skips | Invalid syntax warns/skips; valid extra argument changes shape; malformed later row preserves earlier rules | Same | Same |

This format has no typed null fields or duplicate mapping keys. No additional configuration robustness finding was established.

## Observations

### Existing executable interpolation restrictions remain unchanged

Darkmatter document directives, shell blocks, frontmatter shell values, and Claudine shell-valued `set` still reject interpolated executables before approval. Standalone and first-task controls confirm the existing restriction. As earlier reviews noted, expanding those surfaces is separate work; the finding above concerns commands that already reach approval.

### Future containers remain outside the current runtime

Nested sequence execution and group terminal lifecycle events still have no implemented execution surface. Their future examples are not present coverage. The spec leaves container expansion to later work; this does not block correcting the current approval choices.
