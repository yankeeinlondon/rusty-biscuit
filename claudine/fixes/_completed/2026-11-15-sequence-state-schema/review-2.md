---
$schema: feature-review.yaml
ready: false
findings:
  - 'R2-F1 (high): Derived document values reach first-position shell sites as literal template text'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-11-15T21:48:22-08:00
spec: 2026-11-15-sequence-state-schema/spec.md
implemented: true
description: 'A **fix** review of `2026-11-15-sequence-state-schema/spec.md`'
fix: 2026-11-15-sequence-state-schema/review-2.md
previous: 2026-11-15-sequence-state-schema/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/log.md
implemented_by: claude/opus
next: 2026-11-15-sequence-state-schema/review-3.md
---

# Sequence state schema review, iteration 2

The fix is **not production ready**. All five findings from the first review
are verified fixed at their original sites. One additional sibling remains:
shell preparation can use an unexpanded document property even though normal
document preparation has resolved it. This produces wrong command arguments,
or rejects an unchanged inline container as if it introduced a new command.
The repair log noticed this behavior but disputed it; the realistic input
below establishes defect authority without needing a new design decision.

## Previous Finding Verification

Read `review-1.md`, `log.md`'s Finding Ledger, both implementation records,
and the implementation before assessing the completion claims. There is no
Human rulings section in the previous review and no earlier review in this
directory. None of the previous findings was blocked.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — Referring-document properties still enter referenced templates | R1-F1.1–R1-F1.4: root, nested, group-leg, task-file references; R1-F1.5: discovery and prompt selection; R1-F1.6–R1-F1.8: direct, owner/setter precedence, required-input controls; R1-F1.9: fleets and documentation | verified fixed | Both referrer merges are removed from the Claudine library's `source.rs` and `preflight/inputs.rs`. Passing `sequence_referenced_state` route/control tests print the child's fallback, own value, or setter as appropriate; `a_referenced_template_never_selects_a_prompt_from_its_referrers_values` checks discovery; `a_referrers_property_never_satisfies_a_referenced_documents_input` checks the downstream collection decision. All 22 affected fleets now use an initialization write; `research_sequence_artifacts` tests pass. Topic docs and skills describe that channel and the dry-run setter. | None of the original instances. |
| R1-F2 — Relative task references and file inputs lose their authoring folder | R1-F2.1–R1-F2.4: prompt/task/sequence/group references; R1-F2.5: direct/nested controls; R1-F2.6–R1-F2.9: relative and absolute `with:` inputs, approval, barrier, execution | verified fixed | `Loader::walk_plan` and discovery use the whole document's declaring path. `a_documents_relative_references_resolve_beside_it_through_a_root_reference` checks inventory and graph for all four executable kinds; the CLI route test passes. `with_file_provenance` tests pass for relative, absolute, and competing launch-folder files. Read both approval's `with_origin` records and `task_caller_records`, shared by validation and execution. Nested sequence/group rereads retain the invocation's resolution context. | None. Data selections keep their separate, existing origin rule. |
| R1-F3 — Nested templates read raw document values instead of prepared values | R1-F3.1–R1-F3.5: root, inline, group-leg, task-file, referenced-owner lists; R1-F3.6: discovery; R1-F3.7: state validation and shell consumption | verified fixed | `Loader::owner_template_values` reuses discovered prepared values, or prepares once per owner. `every_inline_template_reads_its_owners_prepared_values` passes for six placements, four value variants, and three builders; setter/item-shadowing and no-expression controls pass. CLI absent-default, null-default, derived-value, and referenced-owner cases print `ada`. `$state` validates the filled row. | The original template-normalization instances are closed. R2-F1 is a newly identified sibling in shell preparation, not an incomplete repair of these template sites. |
| R1-F4 — Referenced document schemas are not checked against current runtime values | R1-F4.1–R1-F4.4: four reference routes; R1-F4.5–R1-F4.8: direct/inline, valid-write, authored-value controls; R1-F4.9: empty/shell-only/loop cases; repair-added R1-F4.10–R1-F4.13: row owner, root events, defaults, initialized required inputs; R1-F4.14: disputed derived-shell case | verified fixed | `prepare_whole_document` prepares current values in the declaring context. Nested references judge at start and later loop/retry entries; root references judge at each composing step. Passing library `referenced_owner_schema`, `prepared_shell_values_tests`, and CLI rejection/control tests verify these consumers, empty containers, initialization correcting a value, and prepared event/child values. YAML/Markdown root initialization supplies required inputs, including empty roots and reference routes. Independently reproduced repair-added R1-F4.14: its disputed disposition does not justify leaving the separate shell defect; it is fully inventoried as R2-F1 below. | No remaining defect in the original runtime-schema checks or repaired R1-F4.10–R1-F4.13. The newly identified shell sibling has its own finding. |
| R1-F5 — Edited inline checks reject containers after initialization has already run | R1-F5.1–R1-F5.4: inline, group-leg, task-file, referenced-owner placements; R1-F5.5–R1-F5.7: invalid, false-guard, retry/loop controls; repair-added R1-F5.8: whole-document proxy | verified fixed | `run_validated` calls `admit_inline_container`; `reread_inline_check` reads the current declaration before opening the run. Library `stale_admission` and CLI tightened-check tests pass at all four placements, asserting no initialization effect. Loosened and malformed edits, false guards, post-initialization start checks, and declaration-cache counts also pass. Both proxy-target tests preserve whole-document schema preparation. Frozen-graph and retry/loop tests pass. | None. |

## Verification and requirement coverage

Used the `claudine` and `rust-testing` skills. Reviewed the schema split and
declaration validators as well as this repair's changes to discovery, graph
construction, runtime preparation, file provenance, fixtures, topic docs, and
skills. The work remains in the session's original worktree; no production
code was changed during this review.

Verification on macOS:

- `just test sequence --no-fail-fast` in `claudine/`: **2,069 passed**,
  10,315 outside the filter; 83.658 seconds of test execution. No test reached
  five seconds.
- `just test-library shipped_artifacts::`: **27 passed**, none at or above
  five seconds, including checked-in artifact freshness and parity.
- `just check-tier-coverage claudine`: **zero stranded tests**. The added
  library files are declared by their existing module trees; the CLI tests
  belong to the existing L1 target and fixture feature.
- Ran the built CLI against private temporary sequence documents, with an
  isolated home/configuration, restricted executable directory, private
  unreachable Rendezvous endpoint, audio dry run, closed stdin, and `--yolo`.
  Only local fixture commands ran; no provider or terminal window launched.
  Literal-value and later-position controls accompany the failing probes.

The first sequence run was invalidated by this review's concurrent
`check-tier-coverage` invocation. That recipe runs `cargo nextest list`
without fixture features and replaces `target/debug/claudine` while the
running CLI tests still use that path. Three pre-warm tests consequently
observed no fixture recordings, and one waited 13.987 seconds for them.
This was an incorrect verification procedure, not valid evidence about a
fixture-enabled execution. All three passed when run alone (0.892–1.237
seconds), and in the subsequent complete sequence run without overlapping
builds. The other ten initial timings of 5.237–7.337 seconds occurred at load
135.89 on 16 cores (`sniff cpu --json`, `uptime`); individual remeasurements
were 0.931–1.959 seconds. The final valid run has no unresolved timing failure.

The implementer's full-area `just test` and `just lint` results are recorded
in `log.md`; this review reran the affected sequence and artifact checks.
Cross-OS evidence remains CI's responsibility.

| User-facing requirement | Appropriate and present verification |
|---|---|
| Root `$schema` describes document inputs; YAML/Markdown direct and reference routes agree; supplied files convert before derived values read them | Level 1 public preparation/resolver tests and CLI frozen-reproduction/setter cases in `sequence_sources_cli` |
| `$state` validates only immediate normalized steps, passively; both declarations coexist; malformed declarations fail even on empty lists | Level 1 `declarations`, `document`, and formal-normalization tests, including typed causes and locations |
| Templates use prepared owner values and per-item shadowing; parents do not supply a child's authored properties | Level 1 owner-value matrices plus CLI tests listed in the verification table |
| Referenced owners and empty documents join required-input collection; inline checks and `$state` never prompt | Level 1 input-inventory and CLI combined-report/empty-list tests |
| Inline checks follow the guard, precede initialization, and recheck current declarations at existing entry boundaries | Level 1 library execution tests and CLI edited-check, false-guard, retry/loop tests |
| Reserved writes, declaration placement, schema references/imports, whole-document formats, and inactive data selections | Level 1 declaration-write, loader, resolver, grammar-matrix, and CLI wiring tests |
| Children, order, identifiers, and template-derived state stay frozen through rereads/retries/loops; approval never reopens | Level 1 frozen-graph, declaration-cache, shell-approval, and retry/loop tests |
| Shipped and disk contracts agree and generated artifacts are fresh | Level 1 shipped-schema and artifact tests |

These are data preparation and execution-boundary requirements. None requires
terminal-emulator rendering or OS keyboard events, so Level 2 or Level 3
evidence would not answer an additional requirement here. The passing tests
above do not cover the derived-value shell defect below.

## Unblocked Findings

### R2-F1 (high): Derived document values reach first-position shell sites as literal template text

**Authority — defect:** in the Claudine library's shell-preparation code,
an ordinary derived document property produces the wrong command argument.
For example, `name: ada`, `greeting: "hi {{ name }}"`, and
`shell: "echo {{ greeting }}"` print **`hi {{ name }}`**, with exit 0.
Normal document preparation produces `greeting: "hi ada"`; a later-position
shell prints that correct value. An inline container using the same property
can instead fail with “nested source introduced a command after sequence
approval closed,” even though no source was edited. This is a realistic
derived-value input, not an adversarial template or a proposed new feature.

**Defect class:** the value view used to approve and freeze a shell command
retains authored expression text in place of its resolved document value.

The Claudine library's
[`layer_prepared_values`](../../lib/src/composition/sequence/preflight/mod.rs:113)
deliberately skips every key containing a template, including ordinary
document-only expressions. Its comment says early binding evaluates that
text when read, but
[`early_binding_state`](../../lib/src/composition/sequence/preflight/reapproval.rs:169)
does not resolve those derived properties. A command reading `greeting`
therefore receives its authored string. The first-position command freezes
those bytes; runtime verification repeats that authored basis and accepts
them. Inline-container rereads can instead see the prepared value and
incorrectly classify the different command as newly introduced.

Minimal direct reproduction:

```yaml
kind: sequence
name: ada
greeting: "hi {{ name }}"
sequence:
  - name: work
    shell: "echo {{ greeting }}"
```

Run `claudine sequence child.yaml --goose --yolo`. Expected shell output is
`hi ada`; observed output is `hi {{ name }}`. Changing only `greeting` to
literal `hi-ada` prints `hi-ada`. Keeping the derived property and placing
the shell after a harmless `side_effect: { set: { started: true } }` prints
`hi ada`. Wrapping the original row in an inline `sequence: [...]` produces
the approval-change failure rather than running it.

The following sweep uses that same derived property and command at every
affected consumer. Reference fixtures keep the property in the declaring
child document. Each literal control changes only the derived property to
its literal value.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.1 | Direct root shell row | First row reads derived `greeting` | Exit 0; prints `hi {{ name }}` | Print `hi ada` |
| R2-F1.2 | Root whole-document reference shell row | Same child through root `sequence: sub/child.yaml` | Exit 0; prints literal expression | Print `hi ada` |
| R2-F1.3 | Nested whole-document shell row | Same child as a sequence task | Exit 0; prints literal expression | Print `hi ada` |
| R2-F1.4 | Referenced sequence in a group leg | Same child and first shell | Exit 0; leg prints literal expression | Print `hi ada` |
| R2-F1.5 | Referenced sequence through a task file | Same child and first shell | Exit 0; prints literal expression | Print `hi ada` |
| R2-F1.6 | Inline child of a directly invoked owner | First shell inside inline sequence | Refuses expanded `echo hi ada` as a new command | Run approved command with `hi ada` |
| R2-F1.7 | Inline child through a root reference | Same inline child in referenced owner | Same incorrect refusal | Run with `hi ada` |
| R2-F1.8 | Inline child through a nested reference | Same inline child in referenced owner | Same incorrect refusal | Run with `hi ada` |
| R2-F1.9 | Inline child through a group-leg reference | Same inline child in referenced owner | Same incorrect refusal | Run with `hi ada` |
| R2-F1.10 | Inline child through a task-file reference | Same inline child in referenced owner | Same incorrect refusal; rendered diagnostic truncates its final explanation | Run with `hi ada` |
| R2-F1.11 | Direct root lifecycle shell | `start: [{ shell: "echo {{ greeting }}" }]` | Prints literal expression | Print `hi ada` |
| R2-F1.12 | Referenced lifecycle shell | Same start action, nested/group-leg/task-file routes | All three print literal expression | Print `hi ada` |
| R2-F1.13 | Inline-container lifecycle shell | Start action on inline container, direct and nested owners | Both refuse expanded command as newly introduced | Run with `hi ada` |
| R2-F1.14 | Group lifecycle shell | Start action on inline group, direct and nested owners | Both print literal expression | Print `hi ada` |
| R2-F1.15 | Direct root lifecycle shell-valued write | `set: { captured: "$(echo {{ greeting }})" }`, followed by stdout of `captured` | Captured value is `hi {{ name }}` | Capture `hi ada` |
| R2-F1.16 | Referenced lifecycle shell-valued write | Same write, nested/group-leg/task-file routes | All three capture literal expression | Capture `hi ada` |
| R2-F1.17 | Later-position shell control | Unchanged derived property; harmless preceding write; direct/root-reference/nested/group-leg/task-file routes | All five print `hi ada` | Same — clean |
| R2-F1.18 | Literal property controls | Original shell across all six routes; root/referenced/group lifecycle shells; root/referenced shell-valued writes; inline-child shells | Correct literal output; no approval-change refusal | Same — clean |
| R2-F1.19 | Side-effect shell placement control | `side_effect: { shell: ... }` on all six routes | Typed refusal: use a `shell:` task | Same — clean; this is not an additional shell consumer |
| R2-F1.20 | Template/state consumers of the derived owner value | Existing owner-template matrix and CLI derived-template case | Filled state contains `ada`; consuming shell prints `ada` | Same — clean; R1-F3's original sites remain fixed |

The three shared projections are the root lifecycle view, root step seed,
and referenced event/child view, all through `prepared_view`. Their consumers
include shell-row bytes and approval records, container command shells,
shell-valued writes, and the later membership check. The table follows the
value through those decisions, including the clean later-position path.
Root references keep the existing invoking-root lifecycle contract; the
referenced child's lifecycle is not an additional event site on that route.

**Recommended repair:** resolve ordinary document-derived values consistently
at the existing shell preparation and verification boundaries. Keep the
authored dependency information needed to recognize genuinely late
`seq`/`loop`/`group`, provider/environment, and shell-derived reads; do not
blindly replace every templated property with its launch-time value. Reuse
the existing preparation/dependency machinery rather than adding an escaping
layer or a new syntax. A plain one-property derivation is enough to pin the
defect. Cover the shared projections in-process, with a few CLI cases proving
correct output, a correct captured value, and no false membership refusal.
Update the inaccurate early-binding comment with the repair.

The repair log's R1-F4.14 establishes that this behavior was already noticed;
its claim that pre-existing behavior has no review authority is not an author
ruling. The review instructions explicitly include realistic pre-existing
defects. No public format, scope change, or new policy decision is required.

## Blocked Findings

None. R2-F1 has a recommended repair within the existing design.

## Recurrence

R2-F1 is a **newly identified sibling** of `review-1.md`'s R1-F3, which
reported value projections retaining raw document expressions instead of
prepared values. That review and repair covered template normalizers and
discovery; the missing consumers were first-position shell binding, lifecycle
command and shell-value preparation, approval verification, and inline
reread membership. This review sweeps them together above. The original
R1-F3 instances are verified fixed; this is not an unchanged template instance
or a claimed regression caused by the repair. The repair-added R1-F4.14 row
is accounted for by this new finding, rather than silently accepting its
disputed disposition.

## Input robustness matrix

The readers added by this fix remain covered through their shared validators
and loaders. Invalid declaration shapes produce typed, located errors before
the rejected task launches. Existing document preparation rules, including
defaults and initialization-supplied values, remain the default policy.

- Schema declarations: absence adds no contract; `{}` is open. Nonempty
  references and supported root unions/imports resolve through the shared
  machinery. Null, empty reference/union, wrong whole-field type, and mixed
  or entirely invalid union elements fail. `declaration_shapes_are_judged_by_presence_on_an_empty_list`,
  `task_declaration_shapes_are_located_at_the_task`, and schema-form tests
  exercise the public resolver, including empty lists and owner locations.
- Templates: absence adds no defaults; `{}` is a no-op. Null and non-mappings
  fail. Mapping values remain data except for top-level string evaluation;
  literal null/list/object values inside the mapping are intentional data.
  Reserved declaration/generated keys fail. The resolver tests distinguish
  these shapes and preserve item shadowing.
- Step state: the public resolver test
  `the_state_validator_judges_every_field_shape_passively` checks required
  absence/null, whole-field type failures, mixed/all-invalid collection
  elements, permitted empties, and passive numeric conversion. Document
  preparation separately tests schema defaults for absence and null,
  supplied-value conversion, and required-input error classification.
- Loaders: the sequence tests cover both YAML entry loaders' duplicate keys,
  invalid trailing syntax, second documents, and bare-key null; Markdown
  frontmatter duplicates/null; shared JSON/JSON5 duplicate detection,
  trailing garbage, comments and trailing commas; shared JSONL/NDJSON blank
  lines and located bad records. These tests passed in the sequence run.

The smell search found no new serde default or optional-field deserializer
that merges declaration absence and null: readers use key presence.
`filter_map` in input reports selects already-classified missing owners.
The new file-read `.ok()` feeds the source memo: a failed read triggers the
ordinary loader and its error, not a cached successful document. The
prepared-view fallback is followed by the existing owner verdict; R2-F1
instead occurs on valid, successfully prepared values. Its derived/literal
and first/later-position controls identify the failing decision rather than
inventing additional invalid-input requirements.

## Implementation record corrections

Corrected `implementation.md` in place, with dated verification notes: nested
templates now use prepared owner values, admission now rereads before
initialization, and referenced templates no longer retain a referrer fallback.
Recast the corresponding correction paragraphs in `implementation-log.md` as
the historical first-review behavior and added the verified current behavior.
These record corrections are not findings or human-review requests. The
specification's design body was not changed.

The fleet preview trade-off mentioned by the previous review needs no new
author interruption: retaining initialization writes and supplying their
equivalent setter for dry runs follows the existing no-lifecycle dry-run
contract and the spec's independent-owner decision. This review recommends
keeping it.
