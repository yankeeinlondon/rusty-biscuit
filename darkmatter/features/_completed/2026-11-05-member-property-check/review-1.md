---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): DMLS omits shell ternary conditions and value branches"
    - "R1-F2 (high): Evaluation failures discard already-observed member warnings"
    - "R1-F3 (high): Interpolation hides other member reads in a shell expression"
    - "R1-F4 (low): Loading-data skill guidance still says member typos are silent"
human_review: true
human_review_items:
    - Confirm afterward that a passive slot session is the right unresolved-composition test; normal composition creates a resolving session automatically.
    - Confirm afterward that unresolved composition should fail before expression evaluation, with no member warning; this supersedes the log's earlier expectation.
    - Confirm afterward that pending site schemas carry both their template and unresolved-slot error, avoiding repeated schema assembly.
    - Confirm afterward that the editor stores the template in its existing slot status and uses the small for_template constructor.
    - Confirm afterward that the effective schema stays by value in SiteSchema, avoiding an allocation on the usual path.
    - Confirm afterward that constant projections from function results are typed locally for member checking without changing call-check verdicts.
    - Confirm afterward that the pending-enum test proves unknown typing rather than enum-value rejection, which the call checker does not perform.
    - Confirm afterward that holding the background slot job is the right way to test the editor before its first resolved schema is published.
    - Confirm afterward that separate member-plus-call and call-only entry points preserve the approved lifecycle-stack exclusion.
    - Confirm afterward that body identities use authored byte offsets and frontmatter identities use key paths plus offsets within authored values.
    - Confirm afterward that shell pieces use the top-level key and condition/then/else role as identity; their interpolation exclusion must be repaired under R1-F3.
    - Confirm afterward that shell warning denial is checked before any command in the stage runs.
    - Confirm afterward that transclusion conditions recheck denial before composing their targets.
    - Confirm afterward that collection callback warnings are buffered until the next existing gate or traversal completion, including failure.
    - Confirm afterward that pending template declarations also feed call checking, with pending constraints unknown, as the author already approved.
    - Confirm afterward that acceptance-test smoke cases were added during wiring and extended afterward in the same test file.
    - Confirm afterward that comparison operands exercise short-circuit reachability without direct absence handling suppressing the read.
    - Confirm afterward that the pass-repetition test uses a held-back leaf because current frontmatter passes do not evaluate one leaf twice.
    - Confirm afterward that a reached right-hand operand of fallback remains checked; an earlier implementation note incorrectly said every operand was silent.
    - Confirm afterward that additional topic pages were updated wherever their previous member-check descriptions became false.
    - Confirm afterward that a registered host function is appropriate for the closed-object function-return test.
    - Confirm afterward that exhausted alias projection is tested internally because the resolver rejects authored alias cycles earlier.
    - Note that omitting editor shell-piece checks was an implementation deferral, not an approved scope change; R1-F1 recommends implementing them.
    - Note that preserving warnings only across stages leaves the within-stage contract incomplete; R1-F2 recommends preserving them on error too.
    - Note that separate local cross-platform runs and new terminal tiers were omitted; this review does not treat either as a readiness requirement.
    - Confirm afterward that shell pieces holding an in-string `{{ }}` placeholder join to authored candidates by an authored-text template; a fill containing the enclosing quote or a backslash, or a placeholder outside a string, stays silent at runtime.
    - Confirm afterward that one shared inventory (`member_pieces`) filters placeholder positions for both composition and DMLS, and that the editor warns on in-string-placeholder pieces whatever the runtime fill.
    - Confirm afterward that `md compose`'s passive reference check stays passive, so a body expression failing there reports its error without a member warning; alternatives were deferring the check's expression failures to composition or recording members during discovery.
    - Confirm afterward that a failing shell ternary piece reports its own members before a branch is selected, and that a failing `::file`/`::code` condition's evaluation error wins over a deny stop.
    - Confirm afterward that placeholder-adjacent denial (R1-F3.7) is evidenced at library level only, since CLI shell approval is not cheaply scriptable.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-09T03:31:39-08:00
spec: 2026-11-05-member-property-check/spec.md
next: 2026-11-05-member-property-check/review-2.md
implemented: true
description: "A **feature** review of `2026-11-05-member-property-check/spec.md`"
feature: 2026-11-05-member-property-check/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-member-property-check/log.md
implemented_by: claude/opus
---

# Review 1: Member Property Check

The feature is **not production ready**. Three functional findings and one documentation finding remain. None needs a human decision before repair. This is the first implementation review; there are no previous findings to reconcile.

The closed-object classifier, conservative union rule, shared absence handling, lexical typing, source-site identity, pending-template typing, editor positions, and CLI policy wiring are implemented. The gaps concern omitted shell expressions and diagnostic preservation on failure, rather than the basic member verdict.

## Implementation decisions (informational)

The two independent readers examined the implementation log and specification in parallel. This assessment also includes the author-approved readings recorded at the end of the plan. The following choices are reversible, already authorized, or choices I recommend. They do not stop repair.

1. **Unresolved-composition setup:** use an explicitly passive slot session. An ordinary library composition creates a resolving session automatically, so omitting a session would not test an unresolved schema.
2. **Unresolved-composition result:** expect `dm.schema.slot_pending` and no member warning because schema preparation fails before expressions run. This supersedes the log's initial expectation of a warning in that failure and agrees with acceptance criterion 3's resolve-first contract.
3. **Pending-schema representation:** keep the unresolved error beside the template in `SiteSchema::Template`; retain the editor's existing failed-schema outcome. This avoids assembling the same schema twice.
4. **Editor template storage:** store the template in `SlotStatus.template` and use `DeclaredTypes::for_template`, rather than storing a second copy of the slot error in a whole site-schema value.
5. **Enum size:** keep `EffectiveSchema` by value and allow the large enum variant. Boxing would add allocation to the common successful path; there is no demonstrated problem requiring it.
6. **Function-result projection:** type constant member/index chains locally in the member checker. This covers `load_data(...)[0].nmae` without expanding the call check's verdicts beyond this feature.
7. **Pending-enum evidence:** assert that pending constraints produce unknown types. The call checker does not compare enum-member sets even for concrete enums, so the originally proposed rejection comparison would prove nothing.
8. **Pending editor test:** hold the background slot-resolution job rather than the registry gate. This directly controls first publication and also proves that passive analysis did not evaluate a slot.
9. **Lifecycle exclusion:** split editor checking into member-plus-call and call-only routes. This implements the approved exclusion of lifecycle-stack member observation.
10. **Site identity:** use original body byte offsets, and frontmatter key paths plus interpolation offsets within the authored scalar. This avoids repeated YAML position projection for identity while retaining projection for diagnostics.
11. **Shell identity:** identify condition/then/else pieces by role within each top-level key. That internal identity is reasonable. Omitting an entire piece containing an interpolation is not equivalent to checking its placeholder; R1-F3 repairs the missing ordinary reads.
12. **Shell denial timing:** check member denial before any command in the stage runs. This is a reasonable strengthening within the existing warning policy.
13. **Transclusion denial timing:** check immediately after a condition's member warnings, before composing its target.
14. **Collection denial timing:** buffer callback warnings while evaluation holds a shared borrow, then flush before the next gate and at traversal completion, including failure. The spec requests existing checkpoints, not a new checkpoint between every item; no mutable-engine redesign is required merely for this feature.
15. **Template-backed call checking:** use pending declarations for calls too, treating pending constraints as unknown. The author explicitly approved this; it is not an unresolved decision.
16. **Test sequencing:** add surface smoke tests during wiring, then extend that file for acceptance coverage. No behavior depends on the phase in which the file was created.
17. **Short-circuit cases:** use comparison operands to exercise ordinary reads, since direct logical operands follow existing absence handling.
18. **Frontmatter pass repetition:** test a held-back leaf once, because current passes do not evaluate the same leaf twice. This keeps the intended no-duplicate-warning check without inventing a pipeline state.
19. **Fallback correction:** retain warning behavior on a reached right-hand fallback operand. The implementation log's earlier claim that every operand was silent was corrected to match the code; primary operands retain their absence handling.
20. **Documentation breadth:** update additional current topic pages where root-only or no-member-warning statements became false. This is required drift maintenance, not feature expansion.
21. **Typed function fixture:** register a host function returning a closed object because ordinary built-ins do not supply that return shape. Its handler panics if passive checking accidentally executes it.
22. **Bounded alias evidence:** test exhausted alias projection internally, because authored alias cycles are rejected by the resolver before the public classifier receives them.
23. **Deferred editor coverage:** the implementer omitted shell ternary conditions/value branches to follow the plan's existing editor checkpoint, and documented the omission. The spec still requires those sites; implement the existing-scope repair in R1-F1.
24. **Deferred warning preservation:** the implementer left same-stage loss unchanged because its shared error type has many construction sites, and tested later-stage failure instead. That narrower test does not satisfy the full contract; R1-F2 identifies the affected sites and recommends preserving accumulated diagnostics on error.
25. **Verification scope:** no separate local cross-platform execution or new terminal tier was added. Neither is required for review readiness here. Earlier comment drift was repaired during the final phase, so it is not outstanding deferred work.

The author has already accepted expression-typed frontmatter being checked only in DMLS, interpreting shell-template holes as frontmatter `$()`, registration in the library policy table, and skipping typed host globals. Those decisions are preserved. The pending-schema test correction is consistent with the author's resolve-first decision. No new public format, security default, scope transfer, or human-only verification is needed to repair the findings.

## Findings

### R1-F1 (high): DMLS omits shell ternary conditions and value branches

**Authority — spec:** “DMLS checks authored expressions in all branches and callback bodies” and “shell-template holes” are included in the observed surfaces; acceptance criterion 7 requires library/editor agreement on equivalent sites. The plan's approved reading identifies this surface as frontmatter `$()`. The implementer's log acknowledges this omission; no author decision authorizes it.

**Defect class:** the editor inventories interpolation placeholders but omits the surrounding shell ternary's expression pieces.

In the darkmatter-lsp package, [interpolation_call_diagnostics](../../dmls/src/diagnostics/frontmatter.rs:1158) sends only `{{ ... }}` spans through the shared checker. The darkmatter package's composition precheck separately inventories the shell condition and both value branches. Consequently, ordinary authored typos in those pieces produce composition warnings but no editor warning.

Use this fixture header, append one `pick` row from the table, then close frontmatter and render `{{ pick }}`:

```yaml
$schema:
  server: "{ name: string, port: number }"
server: { name: web, port: 80 }
```

Each reproduction was run through the public compose API and the real in-process language-server connection. Replacing `nmae` with `name` is the clean control.

| Child ID | Site / `pick` value | Observed member warnings: composition / editor | Expected |
| --- | --- | --- | --- |
| R1-F1.1 | Condition: `$( server.nmae == 'x' ? echo yes : '' )` | 1 / 0 | 1 / 1 |
| R1-F1.2 | Then value: `$( server.name ? fm.server.nmae : echo no )` | 1 / 0 | 1 / 1 |
| R1-F1.3 | Else value: `$( server.port < 0 ? echo no : fm.server.nmae )` | 1 / 0 | 1 / 1 |
| R1-F1.4 | Placeholder: `$( echo {{ server.nmae }} )` | 1 / 1; existing agreement test passes | Clean |
| R1-F1.5 | Ordinary frontmatter `{{ server.nmae }}` and expression-typed values | Existing frontmatter agreement and editor surface tests pass | Clean; both already reach the shared checker |
| R1-F1.6 | Body interpolation, block/file conditions, collection callbacks and region bodies | Existing per-surface library/editor agreement tests pass | Clean; no omitted shell-piece reader at these sites |
| R1-F1.7 | Lifecycle-stack expressions | Explicit call-only route; no member warning | Clean under the approved exclusion |

**Recommended repair:** reuse the shell ternary parser to inventory its condition and value expressions in DMLS, pass each through the shared classifier, and project member spans through the frontmatter scalar map. Analyze both value branches passively even when runtime would select only one. Keep lifecycle exclusions and placeholder deduplication. Extend the agreement tests with the three missing rows and editor-only checks for unselected branches. Remove the documented omission from current docs and skills when repaired. No shell execution or data loading is needed in editor analysis.

### R1-F2 (high): Evaluation failures discard already-observed member warnings

**Authority — spec:** acceptance criterion 8 says “A later failure retains previously reported member warnings,” and the reporting contract says to “Preserve structured diagnostics and earlier warnings on failure.” A missing data file is a normal input, and a member selector read before that failure must remain in the returned failure report. The log explicitly defers the same-stage case.

**Defect class:** error propagation returns before accumulated observations or warning vectors are transferred into the composition failure report.

In darkmatter, [interpolation rewriting](../../lib/src/markdown/compose/interpolation/rewrite.rs:399) returns an error without its accumulated warnings. The whole-value frontmatter path has the same problem. Directive conditions and shell ternary pieces additionally apply `?` to evaluation before converting the observations that evaluation already recorded. This is broader than the log's two interpolation callers.

For the header above, define `E = [server.nmae, load_data('absent.yaml', 'string[]')]`, leave `absent.yaml` missing, and use the shapes below. Every control replaces only the `load_data(...)` call with `'ok'` (or an equivalent list in a shell value), and returns one member warning. The failed runs all name the absent file. Collection rows add a schema-typed nonempty `hosts` list as in the existing composition fixture.

| Child ID | Site / shape tested | Observed member warnings on failure | Expected |
| --- | --- | --- | --- |
| R1-F2.1 | Body pass: `{{ load_data('absent.yaml', 'string[]') }} {{ server.nmae }}`; rightmost site evaluates first | 0 | 1 |
| R1-F2.2 | Body expression: `{{ E }}` | 0 | 1 |
| R1-F2.3 | Mixed frontmatter scalar containing the two interpolations from .1 | 0 | 1 |
| R1-F2.4 | Whole-value frontmatter scalar: `title: "{{ E }}"` | 0 | 1 |
| R1-F2.5 | Page-block `when="E"` | 0 | 1 |
| R1-F2.6 | Collection-region `when="E"` | 0 | 1 |
| R1-F2.7 | `::file ./child.md when="E"`, with a valid child | 0 | 1 |
| R1-F2.8 | `::code ./child.md when="E"`, with a valid child | 0 | 1 |
| R1-F2.9 | Shell condition: `$( E ? echo yes : '' )` | 0 | 1 |
| R1-F2.10 | Shell then value: `$( server.name ? length([fm.server.nmae, load_data('absent.yaml', 'string[]')]) : echo no )` | 0 | 1 |
| R1-F2.11 | Shell else value: `$( server.port < 0 ? echo no : length([fm.server.nmae, load_data('absent.yaml', 'string[]')]) )` | 0 | 1 |
| R1-F2.12 | Positional `::map` callback `h => E` | 1 | Clean |
| R1-F2.13 | Positional `::filter` callback `h => E` | 1 | Clean |
| R1-F2.14 | `filter="h => length(E)"` callback | 1 | Clean |
| R1-F2.15 | `sort="h => length(E)"` callback | 1 | Clean |
| R1-F2.16 | `while="h => length(E)"` callback | 1 | Clean |
| R1-F2.17 | `until="h => length(E)"` callback | 1 | Clean |
| R1-F2.18 | Completed frontmatter interpolation or block condition followed by a failing body stage | 1; existing `ac8_later_failure` passes | Clean |

The callback sites all route through [Execution::eval_callback](../../lib/src/markdown/compose/collection_blocks/engine.rs:636), which captures the evaluation result, records observations, then returns the error; traversal flushes pending warnings even on failure. Use that ordering for the condition sites too. Page-block evaluation is at [evaluate_when](../../lib/src/markdown/compose/page_blocks/engine.rs:130), collection gates at [Execution::condition](../../lib/src/markdown/compose/collection_blocks/engine.rs:394), transclusion gates at [condition evaluation](../../lib/src/markdown/compose/transclusion/engine.rs:684), and shell pieces at [ternary preparation](../../lib/src/markdown/compose/frontmatter_shell_expansion.rs:1509). Nested page blocks share `evaluate_when`; map/filter regions share `condition`; file/code share the transclusion gate. These shared paths have the same ordering, not separate clean implementations.

**Recommended repair:** preserve accumulated warnings in the existing interpolation failure path and merge them into the returned composition failure. At observed condition/value sites, save the evaluation result, transfer observations into the report or failure, and then propagate the error. Also preserve warnings collected before a later shell preparation/execution failure. Keep the original error, authored positions, deduplication, and blocking behavior. This needs no new syntax or warning family. Add public failure-result tests for the failing rows and retain the clean callback cases; add one structured CLI failure case proving earlier diagnostics survive with empty stderr. Testing only a later pipeline stage leaves this defect untested.

### R1-F3 (high): Interpolation hides other member reads in a shell expression

**Authority — spec:** the feature must cover shell-template holes and report evaluated constant selectors using known site types; acceptance criteria 7 and 8 require those surfaces and normal denial behavior. In a valid shell ternary, an ordinary member read does not become handled merely because another part of its expression contains a placeholder.

**Defect class:** a whole expression is excluded from member analysis because it contains an independently checked nested interpolation.

In darkmatter, [record_shell_members](../../lib/src/markdown/compose/call_precheck.rs:619) skips every condition or branch containing `{{`. The placeholder observer covers reads *inside* that placeholder. It cannot cover a separate `server.nmae` beside it. [MemberCandidates::warnings](../../lib/src/markdown/compose/member_sites.rs:127) also requires exact expression text, so the repair must preserve the relationship to authored selectors when interpolation changes the surrounding text.

Add `expected: web` to the fixture header. The following cases were run through public composition with the normal shell approval handler. All compose successfully. For each case, replacing `nmae` with `name` is clean; replacing only `{{ expected }}` with `web` restores the missing warning.

| Child ID | Shell piece / `pick` value | Observed warnings | Expected |
| --- | --- | --- | --- |
| R1-F3.1 | Condition: `$( server.nmae == '{{ expected }}' ? echo yes : '' )` | 0 | 1 at `nmae` |
| R1-F3.2 | Then value: `$( server.name ? length([fm.server.nmae, '{{ expected }}']) : echo no )` | 0 | 1 at `nmae` |
| R1-F3.3 | Else value: `$( server.port < 0 ? echo no : length([fm.server.nmae, '{{ expected }}']) )` | 0 | 1 at `nmae` |
| R1-F3.4 | The same three pieces with `web` instead of `{{ expected }}` | 1 each | Clean |
| R1-F3.5 | Member inside a placeholder: `$( echo {{ server.nmae }} )` | 1; library/editor agreement passes | Clean; retain this observation without duplicates |
| R1-F3.6 | Unselected ordinary value branch without interpolation | No composition warning; existing shell branch test passes | Clean; do not replace runtime reachability with unconditional reporting |

**Recommended repair:** preserve analysis and authored identity for ordinary selectors in shell pieces that also contain placeholders. Start with these normally parseable expressions, where the placeholder is inside a quoted string and does not change the surrounding expression structure; preserve the known member node rather than abandoning the entire piece. Use the existing source/provenance machinery to join evaluated selectors back to those candidates. There is no need to add an escaping framework or diagnose arbitrary generated expression text. Add the three rows, their controls, and a denial check showing the typo cannot bypass `--deny dm.expression.undeclared_member` simply by adding a placeholder. Keep the separate placeholder warning once per authored site.

### R1-F4 (low): Loading-data skill guidance still says member typos are silent

**Authority — spec:** acceptance criterion 9 explicitly requires updating the Darkmatter skill's expression/observation guidance. The edited skill still states the opposite of the new behavior in its loading-data section.

The darkmatter skill's [typed literal returns guidance](../../../.claude/skills/darkmatter/expressions.md:621) ends with “No member misspelling check (`h.nmae` is silent), as for any typed value.” This contradicts both its new member-check section and the passing acceptance test for literal `load_data` contracts. Remove that obsolete sentence or replace it with a link to the shared member-check rules. No runtime change or new test is needed.

## Verification and requirement coverage

All behavior in this spec needs **Level 1** verification: passive analysis, in-process composition, language-server protocol messages, and hermetic CLI subprocesses. No terminal rendering, browser interaction, or keyboard input behavior was added, so no Level 2 or Level 3 evidence is missing.

| Requirement | Strongest relevant evidence inspected/run | Assessment |
| --- | --- | --- |
| Nonempty/empty typed data; computed contracts; per-selector deduplication | Library `ac1_rows_and_empty`, editor `ac1_editor`, public classifier contract cases | Level 1; covered |
| Access forms, optional members, function/literal bases | Public classifier type/access/function tests; composition `ac2_forms`; editor surface tests | Level 1; covered |
| Pending schema declarations and unknown pending constraints | Dynamic-slot library cases and held-job editor tests | Level 1; covered; passive composition fails before reads as authorized |
| Union/absence rules, references, narrowing and invalidation | Public classifier tables and scope tests | Level 1; covered |
| Paths, literal dotted keys, computed keys, array projection, no cascade | Classifier `paths_and_keys`, composition `ac5_paths` | Level 1; covered |
| Lexical identity, repeated callbacks/includes, distinct files | Classifier shadowing and composition `ac6_identity` | Level 1; covered |
| Every surface; editor/runtime reachability; authored positions and Unicode | Existing per-surface agreement and position tests | Level 1 is appropriate, but shell coverage is incomplete: R1-F1 and R1-F3 |
| Default success, selected/all denial, structured output, earlier warnings on failure | CLI `compose_member_warnings`, library later-stage failure test | Level 1; policy wiring passes, failure preservation incomplete: R1-F2 |
| Current docs and skill guidance | Edited topics, policy page, editor diagnostics and skill | R1-F4 remains |

Review execution:

- `just test member` in `darkmatter/`: **167 passed**, no selected test took 5 seconds. This includes the new member suites and related existing tests selected by that filter; it is not a claim that every package test was rerun.
- Seven temporary Level-1 review probes exercised public composition and the real in-process language-server connection. Their final run passed, asserting the observed missing diagnostics and the clean controls described above. The longest probe took 0.394 seconds. Initial probe drafts used unsupported shell branch shapes; those were corrected before recording these results.
- Temporary probes were removed by restoring the original test files exactly. Only this review and the spec review counter remain changed.
- `just check-tier-coverage darkmatter`: passed, no stranded tier tests. The new implementation test modules are declared in their packages' consolidated Level-1 targets and were selected by the focused run.
- The implementation log records the earlier full-area `just test` and `just lint` passes. Those passes do not cover the missing cases identified here; no cross-platform proof is used to block readiness.

The input robustness matrix does not apply: this feature changes expression analysis and diagnostic projection, not a file-format or configuration loader. Its pending-schema work reuses existing loaders; no new load-bearing field validator was introduced.

## Repair disposition

All four findings are unblocked. Repair R1-F1 through R1-F3 first, then remove the stale guidance in R1-F4. Retain the conservative type rules, approved surface exclusions, passive editor behavior, and existing warning identity. The implementation decisions above are for the author's later information; none requires pausing the review/fix cycle.
