---
$schema: feature-review.yaml
ready: false
findings:
  - "high: First-task prompt lifecycles incorrectly require template grammar"
observations:
  - Existing document and set restrictions on interpolated executables
  - Future container surfaces still have no execution implementation
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-14T21:26:44-07:00"
spec: "2026-10-23-shell-template-approval/spec.md"
implemented: true
description: "A **feature** review of `2026-10-23-shell-template-approval/spec.md`"
feature: "2026-10-23-shell-template-approval/review-2.md"
previous: "2026-10-23-shell-template-approval/review-1.md"
log: "claudine/features/2026-10-23-shell-template-approval/log.md"
implemented_by: codex/gpt-6.1-sol
next: "2026-10-23-shell-template-approval/review-3.md"
---

# Review 2

The feature is **not production ready**. The original three findings' reproductions are repaired, but the referenced lifecycle repair introduces a blocking restriction on first-task commands that should still support resolved-byte approval. No human decision is required to repair it.

Reviewed the specification, both implementation logs, the previous review, the combined staged and working-tree implementation, and retained tests. The working tree already contained implementation changes; this review leaves those changes intact.

## Previous unblocked findings

The previous review uses `## Findings`, rather than a separate `## Unblocked Findings` heading. All three findings were unblocked.

| Previous finding | Implementation checked | Verification and disposition |
| --- | --- | --- |
| First-task shell sites cannot use template approvals | Claudine's sequence discovery retains either-mode candidates; task dispatch settles the audited mode before stamping stacks or running primary commands. | Original first-task and first-group primary, setup, teardown, and shell-valued `set` cases pass. Scripted preflight tests cover exact/shape choices, persistence, and exact preference. Original finding repaired. |
| Referenced prompt lifecycles bypass sequence template mode and late-value validation | Claudine carries sequence position, lexical group availability, template membership, and late snapshots into referenced lifecycle preparation. Graph discovery now includes these lifecycle commands. | Prefix/shape/exact-only cases and missing/array barriers pass across all six events and both action forms. The earlier discovery observation is also repaired. The new first-task grammar regression is reported below. |
| Group steps run commands before all late values are checked | Claudine validates every group member before dispatch, retaining late shell inputs and document requests while rebinding parallel runtime state. | Both schedulers pass the retained missing/array barrier cases across task stacks, document directives, frontmatter, transclusion, and lifecycle actions. Snapshot controls distinguish frozen late outputs from current ordinary mutations. Original finding repaired. |

## Findings

### High: First-task prompt lifecycles incorrectly require template grammar

**Authority — spec:** section 5 says a program-word placeholder is “offered only exact, once, deny, and blacklist when resolved-byte approval is eligible,” and says “Resolved-byte Claudine commands keep their current grammar.” Sections 9, 10, and 13 make known-value commands in the first task, including its source document and first-group members, eligible for resolved-byte execution. Acceptance criteria 11 and 12 require that first-task exception and unchanged exact approval paths. A configured executable name and a shell command containing a semicolon are plausible authored inputs, not adversarial values.

**Defect class:** a lifecycle adapter enforces template-only syntax before determining whether the command requires template execution.

In the Claudine library, [sequence_lifecycle_template](../../lib/src/composition/preflight.rs:956), which prepares referenced prompt lifecycle commands, rejects a failed template parse at lines 978–980. Only afterward, at lines 994–996, does it check whether a first-task command with no late reads should return to resolved-byte preparation. Thus a valid resolved-byte command never reaches that fallback if its authored grammar cannot be a template. Both the upfront graph audit and later document preparation use this helper.

Reproduce with the existing native `argv-recorder` and fake Goose fixtures on a private `PATH`. The parent document is:

```yaml
---
sequence:
  - name: first
    prompt: child.md
---
Body.
```

The referenced `child.md` is:

```yaml
---
tool: argv-recorder
start:
  - shell: "{{ tool }} probe"
---
Prompt.
```

With `.darkmatter-shell-whitelist` containing `exact argv-recorder probe`, `claudine sequence --goose seq.md` exits unsuccessfully with `invalid shell template grammar` and records no command. `claudine compose --goose child.md` succeeds with the same fixture and exact rule, recording `["probe"]`. The first sequence task should also resolve and execute those approved bytes, without offering a shape approval.

A second realistic command demonstrates that this is broader than executable interpolation: replace the lifecycle command with `argv-recorder {{ out }}; argv-recorder tail`, set `out: dist`, and use `exact argv-recorder dist; argv-recorder tail`. Standalone composition records `["dist"]` followed by `["tail"]`; the first prompt task instead fails the template grammar check.

The sweep used these same native CLI fixtures, changing the command's location or execution mode. Event names below identify authored lifecycle stacks. Inactive stacks still participate because discovery must be condition-blind.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| First prompt document: `start.shell` | Known `{{ tool }}` program, exact rule | Template grammar refusal; no recorder call | Resolve and run approved bytes |
| First prompt document: `success.shell` | Same | Same refusal | Accept resolved bytes during audit |
| First prompt document: `finalize.shell` | Same | Same refusal | Accept resolved bytes during audit |
| First prompt document: `failure.shell` | Same | Same refusal, even though inactive | Accept resolved bytes during audit |
| First prompt document: `blocked.shell` | Same | Same refusal, even though inactive | Accept resolved bytes during audit |
| First prompt document: `loop.gate` shell | Same, with a valid loop declaration | Same refusal | Accept resolved bytes during audit |
| First-group prompt member: each of those six shell stacks | Same; first serial group | All six refuse before dispatch | Preserve the first-task exception for every leg |
| First prompt and first-group member: long-form `shell.command` | Same program placeholder | Both refuse | Same byte eligibility as short form |
| First prompt document: `start.shell` | Known argument hole plus authored semicolon | Template grammar refusal | Run the exact-approved shell command |
| Standalone document: each of the six shell stacks, plus long-form `shell.command` | Same program placeholder and exact rule | All succeed; active start/success/finalize commands record `probe` | Clean |
| Standalone document: `start.shell` | Same semicolon command and exact rule | Records `dist`, then `tail` | Clean |
| Later prompt document: each of the six shell stacks | Same program placeholder, with a literal predecessor | All refuse template grammar before predecessor execution | Clean: these sites require templates |
| First ordinary task and first-group member: primary, setup shell, teardown shell | Same program placeholder and exact rule | All six succeed and record `probe` | Clean |
| Referenced lifecycle whole-value `set`: each of the six events; standalone start `set` | Same program placeholder | Rejected earlier by the existing frontmatter-shell executable restriction | Unchanged limitation; see observation |
| First ordinary task and first-group member: setup/teardown whole-value `set` | Same program placeholder | All four reject that existing executable restriction | Unchanged limitation; see observation |
| First prompt and standalone document: `::shell`, `::shell-block`, frontmatter shell | Same program placeholder | All six reject existing inserted-executable restrictions | Unchanged limitation; see observation |

Repair the mode decision before imposing template grammar. Determine first-task eligibility and whether expressions contain late reads independently of a successful template parse, as Claudine's ordinary sequence task preparation already does. Preserve required-template refusals for later tasks and late reads. This is a bounded decision-order correction; it needs no new quoting layer, syntax, or scheduler machinery. Retain representative native CLI regressions for first-task and first-group lifecycle shells, both command forms, and the later-task refusal control. Cover all lifecycle event stacks through the shared preparation boundary.

## Recurrence

This finding recurs in the lifecycle execution-mode distinction from review-1's **Referenced prompt lifecycles bypass sequence template mode and late-value validation** finding. That repair correctly makes later lifecycle commands require templates, but should also have swept the first-task resolved-byte counterparts, including first-group prompt members and commands that cannot be templates. Its retained first-task document controls use valid template grammar, so they cannot catch this opposite mode-selection failure. This review enumerates all six event stacks, both shell command forms, the set/parser siblings, and the standalone/later-task controls. The failing case is explicitly named by section 5 and the first-task acceptance contract; the required fix remains small.

## Verification and coverage

Fresh canonical nextest runs, with four test workers:

| Command | Result |
| --- | --- |
| `cd claudine && just test sequence_shell_templates` | 231 passed |
| `cd claudine && just test shell_template` | 260 passed; overlaps the preceding selection |
| `cd claudine && just test preflight` | 160 passed |
| `cd darkmatter && just test shell_template` | 74 passed |
| `just check-tier-coverage claudine` | Passed; zero stranded tests |
| `just check-tier-coverage darkmatter` | Passed; zero stranded tests |
| `git diff --check` | Passed |

The retained selected tests passed below the five-second per-test budget. The existing macOS compact-unwind linker warning was informational. The repair log additionally records passing full-area L1 and lint runs: 9,464 Claudine tests and 10,113 Darkmatter tests. Those full suites and lint were implementation evidence, not rerun as part of this review.

Temporary diagnostic cases reused the existing `Sequence`/`CliProcessFixture`, native recorder, fake provider, private home/CWD/policy, disabled audio, and nonterminal input. A final 51-case sweep asserted the observed successes and refusals, followed by six document-parser controls. These diagnostic passes confirm the reported behavior; they do **not** mean the failing contract passed. Initial runs stopped on expected contract failures or incorrect control expectations; controls were corrected to reflect the existing parser restrictions and semicolon commands were given their exact rule. All temporary test additions were removed, restoring the original test file byte for byte. Production source and retained tests were left unchanged. No real provider, terminal window, formatter, or commit was used.

All applicable verification is Level 1: passive discovery, scripted approval choices, policy persistence, native process arguments, runtime state, and process cleanup. The spec explicitly requests scripted prompt sessions and promises no terminal rendering or keyboard-encoding behavior that needs Level 2 or Level 3.

| Acceptance criterion | Strongest retained verification | Assessment |
| --- | --- | --- |
| 1. Missing single-document values | Level 1: Darkmatter discovery and Claudine missing-value tests | Covered |
| 2. Missing sequence values | Level 1: native task, document, transclusion, and serial/parallel group barrier cases | Original gaps repaired |
| 3. Store round trip | Level 1: `shell_template_policy` persistence and malformed-rule tests | Covered |
| 4. Argument-list matching | Level 1: parsed shape matching tables | Covered |
| 5. Covers relation | Level 1: every specified type-relation row | Covered |
| 6. Containing argument boundaries | Level 1: native recorders in both CLIs and Claudine adapters | Covered |
| 7. Runtime validation and secret-free errors | Level 1: rejection/error-chain tests and native no-spawn assertions | Covered on template paths |
| 8. Prompt choices | Level 1: scripted approval handlers and graph mode settlement | Covered for valid templates; new byte-eligibility refusal above prevents approval of some first-task commands |
| 9. Late approval end to end | Level 1: fake provider, saved rule, and no-handler refusal cases | Covered with current `outputs` spelling |
| 10. Loop re-audit | Level 1: native lifecycle/body iteration records and exact-rule refusal | Covered |
| 11. First resolved, later templates | Level 1: first/later/first-group native and scripted cases | New first-task lifecycle grammar gap above |
| 12. Unchanged paths | Level 1: existing approval/refusal tests and review's standalone controls | New regression above; forbidden roots and initialization retain coverage |
| 13. Runtime policy | Level 1: filled-chain, user/built-in blacklist, and cached/preapproved refusal tests | Covered |
| 14. Grammar and native arguments | Level 1: native recorder, target refusal, and redirection corpus | Template restrictions covered; first-task byte exception fails above |
| 15. Approvals versus results | Level 1: changed-fill/type/constraint, result cache, reservation, and persistence failures | Covered |
| 16. State and discovery | Level 1: task parameters, derived late values, mutations, child documents, groups, and snapshots | Existing surfaces covered except finding above; future containers remain outside current runtime |
| 17. Schema provenance | Level 1: local/named schema, fallback, numeric/union, malformed-rule and validator tests | Covered |

The input robustness matrix applies to the line-oriented policy files, not Markdown template grammar. The fresh Darkmatter selection includes the whitelist/blacklist matrix walkers and public-composition policy matrix. Load-bearing fields are the rule keyword and shape payload on each policy side:

| Input shape | Whitelist keyword | Whitelist payload | Blacklist keyword | Blacklist payload |
| --- | --- | --- | --- | --- |
| Positive control | Authorizes matching command | Authorizes matching shape | Denies matching command | Denies matching shape |
| Absent | Malformed row warns and is skipped | Missing payload warns and is skipped | Same | Same |
| Explicit `null` text | Unknown keyword warns | Literal program `null`, not a typed null; does not match control | Same | Same |
| Wrong type, whole field | Numeric/collection keyword warns | Invalid syntax warns; valid literal command retains its meaning | Same | Same |
| Wrong type, one element | Not applicable to scalar keyword | Invalid hole rejects whole row with warning | Same | Same |
| Wrong type, every element | Not applicable | Every invalid hole still rejects whole row | Same | Same |
| Empty | Malformed row warns; blank file means no rules | Empty payload warns | Same | Same |
| Duplicate | Repeated rule deduplicates | Equivalent parsed shape deduplicates | Same | Same |
| Trailing/invalid content | Unknown keyword warns | Invalid syntax warns; valid extra argument changes shape; bad later row does not erase earlier rules | Same | Same |

This format has neither typed null fields nor duplicate mapping keys. No permissive-deserialization defect was established.

## Observations

### Existing document and set restrictions on interpolated executables

Darkmatter's document directives, shell blocks, frontmatter shells, and Claudine's shell-valued `set` already reject an interpolated executable before template-mode selection. The same native fixture confirms these refusals in standalone composition and first-task references; they are not introduced by the lifecycle repair. Treat that older restriction separately from the newly broken lifecycle `shell` path. It does not require a broader executable-interpolation implementation during this repair cycle.

### Future container surfaces still have no execution implementation

As review-1 observed, nested sequence tasks and group terminal lifecycle events have no current execution surface. Their acceptance examples remain unverified future work, not evidence of present coverage. This review does not require implementing those containers to repair shell approval.
