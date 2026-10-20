---
$schema: feature-review.yaml
ready: false
findings:
  - "high: First-task shell sites cannot use template approvals"
  - "high: Referenced prompt lifecycles bypass sequence template mode and late-value validation"
  - "high: Group steps run commands before all late values are checked"
observations:
  - Referenced lifecycle approval discovery already happens after task setup
  - Future container surfaces remain outside the implemented runtime
human_review: false
human-in-the-loop: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-14T20:26:23-07:00"
spec: "2026-10-23-shell-template-approval/spec.md"
implemented: true
next: "2026-10-23-shell-template-approval/review-2.md"
description: "A **feature** review of `2026-10-23-shell-template-approval/spec.md`"
feature: "2026-10-23-shell-template-approval/review-1.md"
log: "claudine/features/2026-10-23-shell-template-approval/log.md"
implemented_by: codex/gpt-6.1-sol
---

# Review 1

The feature is **not production ready**. Three reproducible defects leave required approval modes and late-value checks incomplete. No new human design decision is needed to implement the stated contract.

Reviewed the specification, implementation plan and log, staged and working-tree changes, shared Darkmatter parsing/filling/authorization paths, and Claudine's document, lifecycle, sequence-task, and group adapters. The implementation log correctly acknowledges the first-task shape and group-barrier limitations; documenting those departures does not satisfy the specification. A further gap affects lifecycle commands inside referenced prompt documents.

## Findings

### High: First-task shell sites cannot use template approvals

**Authority — spec:** section 9 requires commands with known placeholders in a sequence's first task to run in template mode when approved through “a template rule or the ‘command shape’ choice.” Section 10 explicitly permits that choice for the first task, section 13 includes its task stacks and every leg of a first group, and acceptance criterion 8 requires the corresponding prompt choice.

**Defect class:** byte-eligible task sites discard their template approval candidate before authorization, making both saved shape rules and the interactive shape choice unavailable.

In the Claudine library, [the sequence site preparation branch](../../lib/src/composition/sequence/preflight/mod.rs) returns a resolved-byte site before adding an approval candidate. Although the parsed template survives on the site, graph authorization sees only its resolved command. Execution also lacks a path to settle these sites into template mode after a shape approval.

Reproduction, using the existing native `argv-recorder` fixture on the fixture's private `PATH`:

```yaml
---
out: dist
sequence:
  - name: probe
    shell: "argv-recorder probe {{ out }}"
---
Body.
```

With `.darkmatter-shell-whitelist` containing `template argv-recorder probe {{ string }}`, run `claudine sequence --goose doc.md` with the existing fake Goose provider and nonterminal stdin. It exits 1, requests approval for `argv-recorder probe dist`, and records no command. The saved shape should authorize the command without prompting.

The sweep moved the same authored command between sites, retaining `out: dist` and the same shape rule. Stack cases also whitelist the literal primary command. Each first-group case used a member in the first root group.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| First task, primary `shell:` | `probe {{ out }}` | Approval unavailable; no spawn | Saved shape authorizes template execution |
| First task, setup `shell` | Same | Same refusal | Same authorization |
| First task, teardown `shell` | Same | Same refusal | Same authorization |
| First task, setup whole-value `set` shell | `$(argv-recorder probe {{ out }})` | Same refusal | Same authorization |
| First task, teardown whole-value `set` shell | Same | Same refusal | Same authorization |
| First-group member, primary `shell:` | `probe {{ out }}` | Same refusal | Same authorization |
| First-group member, setup `shell` | Same | Same refusal | Same authorization |
| First-group member, teardown `shell` | Same | Same refusal | Same authorization |
| First-group member, setup `set` shell | Whole-value substitution | Same refusal | Same authorization |
| First-group member, teardown `set` shell | Whole-value substitution | Same refusal | Same authorization |
| Later task, all five task/stack sites above | Identical command and rule; add a literal predecessor | Each succeeds; recorder receives `probe`, `dist` | Clean |
| First prompt task, document `::shell` | Identical command and rule | Succeeds; `probe`, `dist` | Clean |
| First prompt task, document `::shell-block` | Same | Same | Clean |
| First prompt task, document frontmatter shell | Whole-value substitution | Same | Clean |
| First prompt task, document lifecycle `shell` and `set` shell | Both forms | Both succeed | Clean for this known-value, shape-approved case |
| Single document, the same five document surfaces | Same command and rule | All succeed | Clean |

Keep the default preference for exact approval, but transport an either-mode candidate for eligible first-task sites and apply the chosen mode to primary commands, both stacks, and shell-valued `set`. Add native CLI regressions for these sites and a scripted handler assertion that both exact and shape choices are offered. Existing later-task tests cannot establish this behavior.

### High: Referenced prompt lifecycles bypass sequence template mode and late-value validation

**Authority — spec:** section 10 says every later task's commands with placeholders are approved “only as typed templates” and explicitly includes lifecycle `shell` and shell-valued `set` in task documents. Sections 4 and 6 require one containing argument per value, refusal of arrays, and checking every late value before any command of the step runs. Requirements 4, 6, and 8 and acceptance criteria 6, 7, 11, and 16 cover this transport and execution behavior.

**Defect class:** the document lifecycle adapter does not receive the sequence's required execution mode or deferred-value snapshot, so it falls back to ordinary resolved-byte preparation.

In the Claudine library, [lifecycle shell preparation](../../lib/src/composition/preflight.rs) constructs the new typed sites with resolved-byte eligibility even for later prompt tasks. Its `shell` branch and [shell-valued `set` branch](../../lib/src/composition/preflight.rs) both omit the sequence's deferred roots and static task position. [The new prompt-document barrier](../../lib/src/composition/prepare.rs) delegates to Darkmatter document discovery without validating these Claudine lifecycle sites.

Use this parent document:

```yaml
---
sequence:
  - name: prior
    shell: argv-recorder prior
  - name: consume
    setup:
      - shell: argv-recorder setup
    prompt: child.md
---
Body.
```

And this `child.md`:

```yaml
---
out: a b
start:
  - shell: "argv-recorder probe {{ out }}"
---
Body.
```

With `prefix argv-recorder`, the CLI succeeds and records `["prior"]`, `["setup"]`, `["probe", "a", "b"]`. Required template execution would preserve `["probe", "a b"]`. Exact rules for the three currently resolved commands also let it succeed, although the later placeholder site must reject exact-only authorization. Replacing the action with `set: {result: "$(argv-recorder probe {{ out }})"}` reproduces both failures.

| Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Referenced document, `start.shell` | `out: a b`, prefix rule | Value becomes two arguments | One argument |
| Referenced document, `start.set` shell | Same | Value becomes two arguments | One argument |
| Both sites | Same value, exact-only rules | Both succeed | Refuse template-required approval before execution |
| Both sites | Same value, template rule plus literal predecessor/setup approvals | Both preserve `a b` as one argument | Clean when shape mode happens to be selected |
| Both sites | `{{ outputs }}`, prefix rule | Both run, passing the serialized array as `[]` in the standalone prompt-task probe | Reject the array before `setup` |
| Both sites | `{{ last(outputs).missing }}`, prefix rule | Both fail, but records already contain `prior` and `setup` | Missing-value failure with only `prior` recorded |
| Both sites inside a group's prompt member | `{{ group.missing }}` | Refused as unavailable during lifecycle resolution, after the first member ran | Treat the in-group read as deferred, then report the missing value at the step barrier |
| Referenced document, `success.shell` and `success.set` shell | `out: a b`, exact-only rules | Both succeed and split the value | Refuse exact-only authorization |
| Referenced document, `finalize.shell` and `finalize.set` shell | Same | Same incorrect success and splitting | Same refusal |
| Referenced document, `blocked.shell` and `blocked.set` shell | Same | Run succeeds; these inactive branches pass the audit | Condition-blind template-required refusal |
| Referenced document, `failure.shell` and `failure.set` shell | Same | Run succeeds; these inactive branches pass the audit | Same refusal |
| Referenced document, `loop.gate` containing `shell` or a `set` shell | Same; valid loop object with `until: "loop.count >= 1"` | Run succeeds; the inactive gate passes the audit | Same upfront refusal; these probes do not establish gate dispatch |
| Referenced document, `::shell` | `out: a b`, prefix/template/exact-only/no matching rule | Prefix and template preserve one argument; exact-only and missing rules refuse before `prior` | Clean |
| Referenced document, `::shell-block` | Same four policy variants | Same correct outcomes | Clean |
| Referenced document, frontmatter `$( … )` | Same four policy variants | Same correct outcomes | Clean |
| Task setup/teardown `shell` and `set` shells outside a prompt document | Later-task shape probes from the first finding | Preserve containing arguments | Clean |

The six shell-capable document events were checked in both `shell` and whole-value `set` forms. Initialization remains a separate forbidden surface, covered by the existing initialization refusal tests. `on_error` is rendered warning text in the lifecycle executor, not another executed command; it does not need a new execution adapter.

Carry task position and late-root metadata into document lifecycle preparation, retain authored expressions, prove template membership, and include these lifecycle sites in the prompt task's pre-setup validation. Reuse the shared filler rather than adding another escaping layer. Add retained CLI tests for both action forms under prefix, exact-only, and shape rules, with setup markers proving the late-value failure boundary. The existing standalone lifecycle tests and task-stack tests do not cover this composition of the two features.

The older omission of these commands from graph-wide approval discovery is recorded separately below. This finding's blocking authority is the new sequence template-mode and late-value contract, not a claim that the old discovery omission is a regression.

### High: Group steps run commands before all late values are checked

**Authority — spec:** section 1 requires a missing late value to fail “that step before any of the step's commands run”; section 6 extends the same barrier to invalid late values. Section 13 expressly carries task position and source ownership through today's group traversal. Acceptance criteria 2 and 16 cover late-value failure and group descendants.

**Defect class:** validation stops at each member's entry instead of covering the group step before its scheduler starts work.

In the Claudine library, [task entry validation](../../lib/src/composition/sequence/task/mod.rs) checks the task's own shell sites; [the site's iterator](../../lib/src/composition/sequence/preflight/shape.rs) does not traverse group members. The existing [group scheduler](../../lib/src/composition/sequence/task/group.rs) then starts members and lets each establish its own barrier. The scheduler itself predates this change, but running these newly approved late-bound commands makes the missing group barrier reachable.

```yaml
---
sequence:
  - name: prior
    shell: argv-recorder prior
  - name: bundle
    group:
      name: bundle
      execution: serial
      tasks:
        - name: alpha
          shell: argv-recorder alpha
        - name: beta
          shell: "argv-recorder bad {{ group.missing }}"
---
Body.
```

With `prefix argv-recorder`, this fails for the missing value but records both `["prior"]` and `["alpha"]`. Only `["prior"]` should have run. Changing only `execution` to `parallel` reproduces the effect. Replacing the placeholder with `{{ outputs }}` reproduces the same ordering defect for a non-scalar value.

The sweep moved the invalid command to every executable member surface. Each row below was run with both serial and parallel scheduling and with both missing `group.missing` and invalid-array `outputs` reads.

| Site in member `beta` | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| Primary `shell:` | Missing or array hole | Fails after `alpha` ran, in both schedulers | Fail group step before `alpha` |
| Setup `shell` | Same | Same | Same |
| Teardown `shell` | Same | Same | Same |
| Setup whole-value `set` shell | Same | Same | Same |
| Teardown whole-value `set` shell | Same | Same | Same |
| Prompt document `::shell` | Same | Same | Same |
| Prompt document `::shell-block` | Same | Same | Same |
| Prompt document frontmatter shell | Same | Same | Same |
| Prompt document lifecycle `shell` | Same | `alpha` runs; missing group root is rejected as unavailable; array is actually dispatched | Group barrier, plus the lifecycle repair above |
| Prompt document lifecycle `set` shell | Same | Same | Same |
| Ordinary shell task outside a group | Missing late primary value, with setup marker | Existing CLI regression leaves only the earlier step's record | Clean |
| Ordinary prompt task outside a group | Missing/invalid late document value, with setup marker | Existing CLI regressions leave only the earlier step's record | Clean for document directives |
| Transcluded prompt document outside a group | Missing late value, with setup marker | Existing CLI regression leaves only the earlier step's record | Clean |

Establish the specified group-entry validation boundary before member dispatch, including referenced prompt documents, and retain the validated late values for execution. Test both schedulers by observing recorder output, not merely the final failure status. This is an explicit group-step contract; retaining per-member barriers would require the author to narrow that contract. A documentation caveat alone does not implement it, and a repair should avoid an unrelated scheduler redesign.

## Verification and coverage

Fresh review runs used the canonical nextest recipes, sequentially between package areas:

| Command | Result |
| --- | --- |
| `cd claudine && just test shell_template` | 62 passed |
| `cd claudine && just test approval` | 54 passed |
| `cd darkmatter && just test shell_template` | 74 passed |
| `cd darkmatter && just test approval` | 57 passed |
| `just check-tier-coverage claudine` | No stranded tests |
| `just check-tier-coverage darkmatter` | No stranded tests |
| `git diff --check` | Passed |

These are filtered runs, with overlap between filters, not full-suite or lint reruns. Full area tests and lint results in the implementation log were read as implementation evidence. The fresh selected tests all finished below five seconds. The existing macOS linker warning about compact unwind metadata remained informational.

The additional CLI reproductions copied the shipped native recorder and fake Goose fixtures into a temporary `bin`, used private CWD/home/config and policy files, cleared the child environment, pinned a minimal `PATH`, used nonterminal stdin, disabled rendezvous reporting, and set `PLAYA_DRY_RUN=1` with a private spool. Recorded argument vectors supplied the effect evidence. Initial probe fixtures with the wrong shell-block terminator, an empty composed body, or an invalid loop declaration were corrected before drawing conclusions. No production source or retained test was changed, no real provider or terminal window was launched, and no formatter or commit was run.

All requirements here are appropriately verified at Level 1: passive preparation, scripted approval choices, native subprocess argument boundaries, policy persistence, and runtime state. The spec explicitly calls for scripted prompt sessions and does not promise modifier visibility, terminal styling, or terminal-encoded hotkeys requiring Level 2 or Level 3. Passing the existing Level-1 tests does not cover the integration gaps above.

| Acceptance criterion | Strongest relevant retained verification | Review assessment |
| --- | --- | --- |
| 1. Missing single-document values | Level 1, `shell_template_discovery` and Claudine `shell_missing_values` | Covered on the tested document surfaces |
| 2. Missing sequence values | Level 1, `sequence_shell_templates` native markers | Task boundary covered; group and referenced lifecycle boundaries fail |
| 3. Store round trip | Level 1, `shell_template_policy` | Covered |
| 4. Argument-list matching | Level 1, policy matching tables | Covered |
| 5. Covers table | Level 1, policy type table | Covered |
| 6. Containing argument boundaries | Level 1, native recorder CLI tests | Shared filler covered; later prompt lifecycle prefix path fails |
| 7. Runtime validation and error secrecy | Level 1, execution error-chain and native refusal tests | Shared template path covered; lifecycle mode bypass accepts arrays |
| 8. Approval prompt choices | Level 1, scripted md and Claudine handler sessions | First-task task/stack shape choice missing |
| 9. Late approval end to end | Level 1, sequence native CLI tests | Covered for primary tasks and document directives using current `outputs` spelling |
| 10. Loop re-audit | Level 1, lifecycle/body loop CLI tests | Shape and exact iteration behavior covered |
| 11. First resolved, later templates | Level 1, static-position and native later-task tests | Later prompt lifecycles violate required mode; first-task shape choice incomplete |
| 12. Unchanged paths | Level 1, existing approval/initialization/forbidden-root corpus | No additional regression established |
| 13. Runtime policy | Level 1, whole-chain, blacklist, and cached-result regressions | Covered on the shared structured paths examined |
| 14. Grammar and native arguments | Level 1, native recorder and refusal corpus | Appropriate level; no new terminal verification required |
| 15. Approval versus results | Level 1, changed-fill/constraint, reservation, and persistence tests | Shared paths covered; no independent cache defect established |
| 16. State and discovery | Level 1, mutations, parameters, children, groups, and static-position tests | Group and prompt lifecycle gaps above; future surfaces discussed below |
| 17. Schema provenance | Level 1, named/local schemas, fallback, numeric/union, and malformed-rule tests | Covered on the available typed-template path |

The input-format review is limited to the persisted line-oriented whitelist and blacklist; document templates are text grammar, not configuration fields. Inspected both loader matrix walkers and the public-composition matrix, including their positive controls. The load-bearing fields are rule keyword and parsed shape payload on each policy side. The reviewed matrix outcomes are:

| Input shape | Whitelist keyword | Whitelist payload | Blacklist keyword | Blacklist payload |
| --- | --- | --- | --- | --- |
| Control | Matching template authorizes | Matching shape authorizes | Matching template denies | Matching shape denies |
| Absent | Malformed row warns and is skipped | Missing payload warns and is skipped | Same parsing behavior | Same parsing behavior |
| Explicit `null` text | Unknown keyword warns | A literal program named `null`, not a null field; does not match control | Same parsing behavior | Same parsing behavior |
| Wrong type, whole field | Numeric/collection keyword warns | Invalid collection syntax warns; valid literal commands retain their own meaning | Same parsing behavior | Same parsing behavior |
| Wrong type, one hole | Not applicable to scalar keyword | Invalid hole rejects the entire row with warning | Not applicable | Same rejection |
| Wrong type, every hole | Not applicable | Same whole-row rejection | Not applicable | Same rejection |
| Empty | Malformed row warns; blank file means no rules | Empty payload warns | Same parsing behavior | Same parsing behavior |
| Duplicate | Repeated parsed rule deduplicates | Equivalent parsed shape deduplicates | Same parsing behavior | Same parsing behavior |
| Trailing/invalid content | Unknown keyword warns | Invalid syntax warns; a valid extra argument is a different shape; malformed later rows do not erase earlier rules | Same parsing behavior | Same parsing behavior |

There is no typed null or key duplication in this file format. The tests keep exact/prefix sentinels and exercise whitelist authorization and blacklist denial rather than silently dropping malformed hole elements. No configuration robustness finding was established.

## Observations

### Referenced lifecycle approval discovery already happens after task setup

The Claudine graph approval function already omitted referenced prompt documents' lifecycle actions before this feature. With the parent/child fixture above and no matching child-command rule, all six event stacks in both `shell` and shell-valued `set` forms fail only after the `prior` and `setup` markers have run. The unchanged omission is an observation under this review's scope rules, not a separate blocking regression. The author can schedule its broader approval-discovery repair alongside or separately from the new lifecycle transport defect.

### Future container surfaces remain outside the implemented runtime

Nested sequence tasks and group terminal lifecycle events have no current execution surface; the spec itself says later container work may extend discovery. The implementation log discloses these missing acceptance examples. Do not count them as tested, but do not introduce a container implementation solely to close this shell-approval review. The current `outputs` spelling is explicitly permitted by the spec. No next spec or plan was supplied for the requested refresh, so neither was changed.
