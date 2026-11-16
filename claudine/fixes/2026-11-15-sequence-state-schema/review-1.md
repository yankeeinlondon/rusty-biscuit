---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Referring-document properties still enter referenced templates'
  - 'R1-F2 (high): Relative task references and file inputs lose their authoring folder'
  - 'R1-F3 (high): Nested templates read raw document values instead of prepared values'
  - 'R1-F4 (high): Referenced document schemas are not checked against current runtime values'
  - 'R1-F5 (high): Edited inline checks reject containers after initialization has already run'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-11-15T19:04:42-08:00
spec: 2026-11-15-sequence-state-schema/spec.md
implemented: true
description: 'A **fix** review of `2026-11-15-sequence-state-schema/spec.md`'
fix: 2026-11-15-sequence-state-schema/review-1.md
next: 2026-11-15-sequence-state-schema/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/log.md
implemented_by: claude/opus
human_review_items:
  - 'R1-F1 repair: research fleets now pass research_prompt through a root initialize set, so a fleet --dry-run (which fires no initialize) needs research_prompt=… as a setter; confirm this trade-off'
---

# Sequence state schema review, iteration 1

The fix is **not production ready**. The document/step schema split works in
the motivating case, but five defects remain in ownership, file resolution,
prepared template values, and checks at container entry. Every finding has a
recommended repair within the existing specification; none needs an author
ruling. This is the first implementation review, so there are no carried
findings.

## Verification and scope

Reviewed the specification, plan, both implementation records, and the changes
from the finalized-spec commit `200a476e5` through the current working tree.
Two independent readers examined the implementation decisions and deferrals
in parallel before the comparison with the specification.

Verification on macOS:

- `just test sequence` in `claudine/`: **1,985 passed**, 10,315 outside the
  filter, 90.006 seconds of test execution; no test reached five seconds.
- `just test-library shipped_artifacts::`: 27 assertion passes, including
  artifact freshness. One test took 5.058 seconds. This run was not initially
  a timing pass: the host load average was 57.35 on 16 cores. Remeasuring
  `shipped_artifacts::the_checked_in_artifact_round_trips_to_uncached_preparation_in_order`
  alone through `just test-library <full-test-name> -j 1` took **2.772 seconds**,
  clearing it under the loaded-host rule. No unresolved slow test was observed
  in these review runs.
- `just check-tier-coverage claudine`: **zero stranded tests**. New modules
  are included by the existing library module tree or CLI L1 target, and the
  portable echo fixture is declared under the enabled `test-fixtures` feature.
- Ran isolated copies of sequence documents through the freshly built
  `target/debug/claudine`, with a temporary home, empty configuration, private
  Rendezvous endpoint, restricted PATH, audio dry run, closed stdin, and
  `--yolo`. Prompt composition used `--dry-run`; execution probes ran only
  fixture shell commands and lifecycle actions. No provider or terminal
  window was launched. Each reproduction below includes a passing control.

The implementer's final full-area test and lint runs are recorded in the log;
this review reran the affected sequence slice and artifact checks, not every
unrelated test. Earlier failed or slow implementation attempts are historical;
the later recorded successful runs and individual timing checks supersede them.
Cross-OS proof remains CI's responsibility and is not a readiness finding.

All requirements in this fix need **Level 1** verification: schema loading,
value conversion, missing-input reports, task boundaries, frozen plans, and
CLI argument wiring. The specification explicitly requires no real provider,
terminal, or focus changes. There is no requirement here whose appropriate
verification level is Level 2 or Level 3. Existing Level 1 tests establish the
motivating frozen reproduction, key placement, state validation, format parity,
reserved writes, initial inline admission, and several retry/loop cases. They
do not establish the behavior contradicted by the reproductions below.

## Findings

### R1-F1 (high): Referring-document properties still enter referenced templates

**Authority — spec:** “The referring document's authored properties do not
enter this list” under document preparation, and “nothing is inherited” under
per-sequence ownership. The implementation deliberately retains the forbidden
layer, and ordinary shared-roster inputs reach it.

**Defect class:** a referenced owner's template lookup includes another
document's authored properties, changing both generated state and the task
selected for execution.

In the Claudine library,
[`plan_referenced_sequence`](../../lib/src/composition/sequence/source.rs)
starts template values with `frontmatter.to_object()` before extending them
with the owner's values. Input discovery independently repeats the merge in
[`Discovery::walk_referenced`](../../lib/src/composition/sequence/preflight/inputs.rs).
This is not fixed by making the owner's own value win: an undefined property
must stay undefined unless a caller setter or runtime write supplies it.

Minimal child, stored as `sub/child.yaml`:

```yaml
kind: sequence
template:
  who: "{{ reviewer || 'none' }}"
sequence:
  - name: work
    shell: "echo {{ seq.who }}"
```

A root with `reviewer: leaked` and `sequence: sub/child.yaml`, run with
`claudine sequence root.yaml --yolo`, prints `leaked`. Running the child
directly prints `none`. The nested, group-leg, and task-file routes also print
`leaked`. These are real executions, not dry-run placeholder comparisons.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Root whole-document reference, shared formal normalizer | Child omits `reviewer`; referrer writes it | `leaked` | `none` |
| R1-F1.2 | Nested whole-document reference | Same child and referrer value | `leaked` | `none` |
| R1-F1.3 | Referenced sequence in a group leg | Same child and referrer value | `leaked` | `none` |
| R1-F1.4 | Referenced sequence reached through a task file | Same child and referrer value | `leaked` | `none` |
| R1-F1.5 | Input discovery and subsequent prompt selection, root and nested routes | Child template selects `prompt_path` or its own fallback; only referrer supplies a different absolute prompt path | Unintended prompt is discovered and composed; its schema fails | Child's fallback prompt is selected |
| R1-F1.6 | Direct owner control | Identical child without a referrer | `none` | `none` — clean |
| R1-F1.7 | Owner-value and setter precedence controls | Child writes `reviewer: own`; then caller supplies `reviewer=caller` | `own`, then `caller` | Same — clean |
| R1-F1.8 | Document required-input decision | Referrer alone supplies a required child input | Existing public-result test `a_referrers_property_never_satisfies_a_referenced_documents_input` rejects it | Rejection — clean; schema input collection itself does not inherit the value |

The prompt-selection probe used absolute prompt paths so the folder defect in
R1-F2 could not explain its result. Both YAML and Markdown reach the same
formal owner merge; JSON and JSON5 whole documents also share that branch.
Data offsets/operators intentionally use their referring document's values
and do not enter this whole-document branch.

**Recommended repair:** remove both fallback merges. Move the affected research
fleets' intentional parameters into the already available initialization-write
channel, then update the fallback-pinning test and the documentation/skills
that describe the exception. The specification already chose independent
ownership and provides the replacement channel; no human decision is needed.

### R1-F2 (high): Relative task references and file inputs lose their authoring folder

**Authority — defect:** a reusable sequence stored in `sub/` cannot resolve
its own neighboring prompt/task/sequence/group files through a root reference,
and a prompt task's authored relative file input is resolved from the launch
folder. These are ordinary `./p.md` and `./spec.md` inputs, not unusual path
spellings. Direct and nested runs of the same documents give different
results.

**Defect class:** authoring-file provenance is lost before a relative file
reference is resolved or validated.

The Claudine library's
[`Loader::walk_plan`](../../lib/src/composition/sequence/preflight/mod.rs)
passes the invoking source path to root tasks even when the plan's
`document_scope` names a different declaring document. The input inventory
makes the same choice in
[`Discovery::walk_referenced`](../../lib/src/composition/sequence/preflight/inputs.rs)
through `rows_origin`. The wrong path subsequently controls task origin,
reference loading, and execution context; agreement between the two readers
therefore does not establish correctness.

For file-valued `with:`, the Claudine CLI's
[`approve_preflight_graph`](../../cli/src/commands/wrap/sequence/mod.rs)
adds overrides as data and supplies the launch-area fallback, producing the
incorrect early rejection. The later
[`WrapperPromptRunner::run`](../../cli/src/commands/wrap/sequence/task_run.rs)
correctly records the authoring source for `with:`. A control with files in
both folders passes approval and then renders the child-adjacent file, proving
that runtime preparation is clean and the two stages disagree.

Example root:

```yaml
kind: sequence
sequence: sub/child.yaml
```

Example `sub/child.yaml`, with `sub/p.md` present and no root-level `p.md`:

```yaml
kind: sequence
sequence:
  - name: work
    prompt: ./p.md
```

`claudine sequence root.yaml --dry-run --yolo` fails with
“`./p.md`: file not found.” Invoking `sub/child.yaml` directly or through a
nested sequence task renders `Child prompt.` successfully.

For the file-input variant, the child task adds
`with: { brief: ./spec.md }`, `sub/spec.md` exists, and the prompt declares
`brief: file(required;eager)`. Direct and nested runs both report that
`./spec.md` did not match from the **launch** folder. Replacing only the
`with:` value with the absolute path succeeds.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Root-reference prompt resolution, inventory then graph | `prompt: ./p.md` beside declaring child | File not found | Resolve child-adjacent prompt |
| R1-F2.2 | Root-reference external-task resolution | `task: ./t.yaml` beside declaring child | File not found | Resolve child-adjacent task |
| R1-F2.3 | Root-reference nested-sequence resolution | `sequence: ./s.yaml` beside declaring child | File not found | Resolve child-adjacent sequence |
| R1-F2.4 | Root-reference external-group resolution | `group: ./g.yaml` beside declaring child | File not found | Resolve child-adjacent group |
| R1-F2.5 | Direct/nested controls for all four executable references | Same files, same relative strings | All eight runs succeed | Success — clean |
| R1-F2.6 | File-input preparation, directly invoked child | `with.brief: ./spec.md`; file beside child | Schema rejects from launch folder | Accept author-relative file |
| R1-F2.7 | File-input preparation, nested whole document | Identical `with.brief` and adjacent file | Same incorrect launch-folder rejection | Accept author-relative file |
| R1-F2.8 | File-input controls, both routes | Only change is absolute `with.brief` | Both render the correct absolute file value | Success — clean |
| R1-F2.9 | Downstream `with:` preparation, direct and nested routes | Same relative input; both launch and child folders contain `spec.md`, allowing approval to finish | Both rendered prompt bodies name the child-adjacent file | Same — clean; runtime honors `with_origin_path` |

Inline/group/task-file prompt inputs converge on the same `PromptReference`
and `PromptTaskRequest` paths; those shapes do not have an independent
file-input validator. Shell commands and side effects do not take `with:`
under the existing task grammar. Offsets/operators remain data selections;
this finding does not change their ownership contract.

**Recommended repair:** retain root task identity and frozen row order while
using the whole document's declaring path for its authored references. Carry
the authoring folder for `with:` values through passive approval using the
existing caller-origin machinery, matching the working runtime preparation;
keep true caller setters anchored to the caller. No new path parser or platform-specific
escaping layer is needed. The recorded deferral of `with:` resolution does
not make this demonstrated defect acceptable.

### R1-F3 (high): Nested templates read raw document values instead of prepared values

**Authority — spec:** “Before evaluating a sequence's template, use its
prepared document values … rather than the raw file alone,” and “Inline
containers use their enclosing document's effective values instead.”

**Defect class:** a value projection drops schema defaults and leaves derived
expressions unevaluated before nested template normalization.

In the Claudine library,
[`Loader::load_sequence_with`](../../lib/src/composition/sequence/preflight/mod.rs)
passes `self.step_frontmatter.clone()` into the inline formal normalizer.
That map comes from authored frontmatter/setters, rather than the prepared
owner values already available during input discovery. A referenced owner's
children similarly receive an authored-value replacement. The root normalizer
correctly prepares its template values, which hides this gap in root-only
coverage.

```yaml
kind: sequence
$schema:
  reviewer: string(default(ada))
sequence:
  - name: nested
    $state:
      who: string(required)
    template:
      who: "{{ reviewer }}"
    sequence:
      - name: work
        shell: "echo {{ seq.who }}"
```

Running this document fails `$state` at `/who`: null is not a string. Moving
only the declarations and list to the document root succeeds and prints
`ada`. Replacing the default with `source: ada` and
`reviewer: "{{ source }}"` is another realistic failure: the nested task
prints the literal `{{ source }}` instead of `ada`.

Each site below was run against four variants of the same fixture: a literal
`reviewer: ada` control, an absent value with a schema default, explicit null
with that default, and a derived value. The expected output is `ada` in every
variant.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | Root inline normalizer | Literal / absent-default / null-default / derived | `ada` in all four | Same — clean |
| R1-F3.2 | Inline nested normalizer | Same four shapes | Literal passes; both default cases fail at `/who`; derived prints expression text | `ada` in all four |
| R1-F3.3 | Group leg's inline sequence | Same four shapes | Same failures and literal derived text | `ada` in all four |
| R1-F3.4 | External task file's inline sequence | Same four shapes | Same failures and literal derived text | `ada` in all four |
| R1-F3.5 | Inline child of a referenced whole document | Same four shapes | Same failures and literal derived text | `ada` in all four |
| R1-F3.6 | Input discovery's inline scope projection | Shared scope values come from `push_owner` preparation | Prepared owner values are retained by `walk_executable`; literal and absent/null-default fixtures pass this stage before graph normalization fails | Prepared values — clean at this projection |
| R1-F3.7 | Subsequent state validation / shell consumption | `who` filled from those snapshots | `$state` rejects null; an accepted derived string reaches the shell unexpanded | Validate and execute the prepared `ada` value |

This is a projection defect, not a new rule for schema nulls: the root
control proves that the existing default engine supplies `ada` for both
absence and explicit null. Existing declaration-shape tests cover wrong
whole-field and union-element types separately.

**Recommended repair:** give every inline owner the enclosing document's
prepared snapshot when the graph is built, with setters/runtime writes
layered as specified and item fields still shadowing that snapshot. Reuse
prepared owner values where available instead of composing once per child.
Keep the resulting rows frozen; later execution must not refill them.

### R1-F4 (high): Referenced document schemas are not checked against current runtime values

**Authority — spec:** “Referenced whole documents instead follow normal
document preparation,” with runtime writes above their own properties, and
“Later document schema checks use current document values through the
existing reread path.” Initial missing-input discovery does not satisfy this
runtime contract.

**Defect class:** schema validation is performed on an initial document
snapshot but omitted at the later preparation boundary where runtime values
have changed.

The Claudine library's
[`prepare_nested_sequence`](../../lib/src/composition/sequence/preflight/mod.rs)
and [`TaskExecution::run_sequence_attempts`](../../lib/src/composition/sequence/task/sequence.rs)
reread referenced declarations/events without performing the document's
schema preparation. Only inline checks get the explicit runtime check.
For a root whole-document reference, the owning scope is retained on the
plan/input inventory, but per-step composition continues to judge the
invoking root. A referenced document containing only shell tasks therefore
has no later prompt preparation that happens to catch the invalid values.

Child `sub/check.yaml`:

```yaml
kind: sequence
$schema:
  rank: number(required)
rank: 1
sequence:
  - name: observe
    shell: echo SHOULD-NOT-RUN
```

Root:

```yaml
kind: sequence
sequence:
  - name: change
    side_effect:
      set: { rank: bad }
  - name: nested
    sequence: sub/check.yaml
```

This exits **0** and prints `SHOULD-NOT-RUN`. The same schema on a directly
invoked document, with the same preceding write, rejects `/rank` and never
runs the observing shell. An inline check rejects it too.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F4.1 | Root whole-document reference | Child's first row writes `rank: bad`, next row runs shell | Shell runs; exit 0 | Current owner schema rejects before that shell |
| R1-F4.2 | Nested whole-document entry | Earlier parent task writes `rank: bad` | Child shell runs; exit 0 | Reject invalid current document values |
| R1-F4.3 | Whole document in group leg | Same earlier write | Child shell runs; exit 0 | Reject invalid current document values |
| R1-F4.4 | Whole document reached through external task file | Same earlier write | Child shell runs; exit 0 | Reject invalid current document values |
| R1-F4.5 | Direct document control | Same schema and write inside direct document | `/rank` rejected; exit 1; no observing shell | Same — clean |
| R1-F4.6 | Inline container check control | Same runtime write and `rank` contract | `/rank` rejected before child | Same — clean for unchanged declarations |
| R1-F4.7 | Valid runtime-value control | Only change: preceding write is `rank: 2` | Child shell runs; exit 0 | Same — clean |
| R1-F4.8 | Initial referenced-owner input inventory | Valid authored `rank: 1` | Initial preparation accepts | Accept — clean, but not evidence about subsequent writes |

The runtime `set` result printed before the marker contains the **prior**
value; it is not evidence that the write failed. The direct and inline
controls verify that the same write is visible to a correctly placed check.

**Recommended repair:** apply normal document schema preparation to whole
referenced owners at their existing execution/reread boundaries, using current
runtime layers and the declaring document's context. Preserve the distinction
from check-only inline schemas, the existing ability for initialization to
supply values, and the frozen children/identities. Add shell-only and empty
referenced-container cases so coverage does not depend on launching a prompt.

### R1-F5 (high): Edited inline checks reject containers after initialization has already run

**Authority — spec:** “If a reread changes a container's check, validate it at
the next entry,” and a failed inline check means the container runs no
“`initialize`, children, or cleanup events.” An earlier task editing a later
container's declaration is a supported source-reread case.

**Defect class:** admission uses a stale preflight declaration and discovers
the current rejecting declaration only after opening the container run.

The Claudine library's
[`TaskExecution::run_validated`](../../lib/src/composition/sequence/task/mod.rs)
checks `sequence.inline_schema` from the approved graph. Only the later
`Start` arm of
[`run_sequence_attempts`](../../lib/src/composition/sequence/task/sequence.rs)
rereads the declaration. Consequently the first initialization runs against
the old check even though the file changed before this task's turn.

Reproduction: a document has `rank: 1`; its later inline container declares
`$schema: { rank: number }`, `initialize: [{ stdout: INIT-RAN }]`, and one
shell child. The preceding task runs a small fixture script that changes only
that declaration to `rank: number(max(0))`. The CLI prints **`INIT-RAN`** and
then fails the container because 1 exceeds 0. No child shell runs, but the
initialization effect has already happened. A separate probe that sets
`initialized: true` during initialization makes the enclosing failure handler
print `initialized=true`, confirming that a real mutation escaped admission.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F5.1 | Inline container after an editing sibling | Declaration tightened before task turn | `INIT-RAN`, then check failure | Failure without initialization output |
| R1-F5.2 | Inline sequence in a group leg | Same edit, same stdout marker | `INIT-RAN`, then check failure | No initialization output |
| R1-F5.3 | Inline sequence in external task file | Same edit to task file | `INIT-RAN`, then check failure | No initialization output |
| R1-F5.4 | Inline child of referenced sequence document | Same edit to declaring document | `INIT-RAN`, then check failure | No initialization output |
| R1-F5.5 | Unchanged invalid-declaration control | Contract already has `max(0)` before planning | Rejected with no marker | Same — clean |
| R1-F5.6 | False-guard control | Same rejecting contract, `when: "false"` | Exit 0; no marker or child | Same — clean |
| R1-F5.7 | Retry/loop start checks | Existing tests exercise current-value rejection on later starts | Check runs before that start's children | Clean for that boundary; it does not prove first admission precedes initialization |

The group result was checked with direct lifecycle stdout, not with mutations
observed outside the group: failed group legs intentionally withhold their
private mutations, which could otherwise hide that initialization ran.

**Recommended repair:** reread and prepare the inline check before admission,
after a true guard and before opening the run. Retain the later start/retry/loop
checks where their existing boundaries require them. Reuse the declaration
cache and frozen graph; do not normalize children again or reopen approval.
Keep a failed admission on the existing unstarted-task failure path.

## Input robustness matrix

This fix changes configuration/declaration readers, so the matrix applies.
The ordinary invalid-declaration outcome is a typed, located error before
commands/providers run. Existing whole-document preparation rules still govern
optional-value dropping, defaults, and initialization; the review does not
invent a stricter document policy.

The shared validators and their intentional exceptions are:

- Schema declaration shape: absence means no additional state/check contract;
  `{}` is an open contract; nonempty references and nonempty root unions are
  supported. Explicit null, empty reference/union, wrong whole-field type,
  and mixed/all-invalid union arms are errors. Root and nested readers use
  the same shape validator. The public resolver tests cover shape errors,
  empty-list errors, reference failures, unions/imports, and owner locations.
- Template shape: absence means no defaults, `{}` is an explicit no-op, and
  a mapping's values are data except that top-level strings are rendered.
  Null, arrays/scalars, and reserved generated/declaration keys are rejected.
  Literal null/list/object values inside the mapping remain data intentionally.
- State values: the shared schema validator rejects required absence/null and
  whole-field/mixed-element wrong types; allowed empty strings/collections
  remain present values, and passive conversion does not rewrite the plan.
  `the_state_validator_judges_every_field_shape_passively` exercises this
  through the public resolver.
- Document values follow the canonical schema default engine: an optional
  default can replace absence or null, and a required null is invalid rather
  than missing. The direct controls in R1-F3 exercise both default shapes.

Loader coverage inspected and exercised in the sequence suite includes both
YAML entry loaders (duplicate root/declaration keys, invalid trailing syntax,
second documents, bare-key null), Markdown frontmatter, the shared JSON/JSON5
loader (duplicate detection, trailing garbage, comments/trailing commas), and
the shared JSONL/NDJSON loader (blank lines and located invalid records).
JSON and JSON5 deliberately share their existing permissive parser; this
review adds no external-format requirement.

The requested smell search found no new serde-default or optional-field
coercion in the declaration readers: they read key presence directly.
`filter_map` collects already-classified missing reports rather than filtering
bad collection elements. The `request.plan(...).ok()` branch chooses between
existing root missing-input report layouts; failure remains surfaced by the
missing-input report. The failures above occur in ownership/projection and
execution boundaries, with their affected shapes and downstream decisions
listed in each finding's table.

## Implementation decisions (informational)

All entries below were assessed against the stopping rules. None requires an
author ruling or a post-cycle interruption. Choices contradicted by verified
behavior receive the ordinary findings above, with a repair recommendation.

The log's 28 `decisions_for_review` entries are accounted for in order:

1. Reject `$schema` in template defaults as well as `$state`/`template`.
   Keep this: declarations should be authored on their owning containers.
2. Give misplaced group declarations a parser-owned typed error. Keep it.
3. Append declaring-file/container provenance to root state errors too. Keep it.
4. Use route-specific typed errors for setters, runtime writes, and `with:`.
   Keep the existing error families and shared reserved-key predicate.
5. Resolve named state types through schema imports, without unsupported
   sibling frontmatter `types`. This satisfies the spec's named-import forms.
6. Retain referrer properties as a lowest template layer to preserve 22
   research fleets. Repair under R1-F1; the spec already chooses the opposite.
7. Prepare owner values only when a string template reads them and a captured
   context exists. Keep the bounded optimization; R1-F3 repairs nested inputs
   that bypass the prepared snapshot altogether.
8. Distinguish steps and documents in missing-input summaries/headings. Keep it.
9. Preserve existing `tasks[i]` graph diagnostic prefixes while inventory uses
   `sequence[i]`. These paths remain distinguishable; renaming is unnecessary.
10. Discover changed commands from reread rows while executing frozen children.
    Keep the existing refusal of commands added after approval.
11. Preserve invoking-file resolution for root-reference rows. Repair under
    R1-F2; row identity/order can stay frozen while origin is corrected.
12. Keep root-reference metadata in the input inventory without duplicating it
    on the graph. Internal storage is flexible; R1-F4 requires consumers to
    retain enough ownership information to perform the promised checks.
13. Leave interpolation in authored executable filenames unchanged; interpolate
    template defaults only. This fix does not introduce new reference syntax.
14. Resolve state/check references during graph building rather than inventory
    discovery, avoiding duplicate preparation before approval. Keep it.
15. Preserve per-step missing-input reports for a nonempty root when no other
    owner is missing values. Keep the existing report ergonomics.
16. Delay non-missing owner errors until structural/provider checks, then
    strictly recheck failed owners. Keep the existing error ordering.
17. Repeat the referrer fallback in input discovery. Repair both readers under
    R1-F1 so they continue to agree on the corrected ownership contract.
18. Include root initialization writes in approval as well as planning. Keep it;
    approval must see values execution will consume.
19. Build the root plan after initialization and input inventory, while checking
    root declaration shape and inline grammar first. Keep this ordering so
    templates can consume supplied inputs without executing them prematurely.
20. Exclude lexical `seq`, `loop`, and `group` bindings from inline schema
    instance properties. Keep the document-property check semantics.
21. Repeat inline checks on the first start and subsequent rereads. Keep those
    boundary checks, and correct stale initial admission under R1-F5.
22. Reuse collected caller values on root retry. Keep it; do not prompt twice.
23. Use refresh-mode command discovery on root retry, retaining frozen state.
    Keep the existing declaration-syntax and command-membership checks.
24. Record caller origins before switching document-resolution context for
    library setters lacking explicit records. Keep caller-relative setters.
25. Defer the relative `with:` file-origin defect as outside this fix. The
    review reproduced it; repair is included in R1-F2, without a scope ruling.
26. Reject declarations in JSONL/NDJSON rows rather than silently activating or
    discarding them. Keep the reserved-key rule; the plan's “inactive data”
    wording does not override the spec's declaration placement restrictions.
27. Add a portable fake echo for shell fixtures. Keep it. The accompanying
    reported `set`/`seq` defect was a misread prior-value result, corrected in
    both implementation records after direct verification; it is not a finding.
28. Summarize declarations in the caller skill instead of transcluding an
    unstable heading selection with executable inline examples. Keep the short
    summary synchronized with the canonical docs during repairs.

Additional implementation choices outside that frontmatter list are also
informational: private source-option builders, shared authored/value snapshots,
borrowed composition views, passive preparation that disables shell expansion,
dependency-aware declaration caching, progress-bounded input collection, and
dry runs that describe an inline check instead of predicting runtime writes.
These fit the existing contracts and need no reaffirmation.

Outstanding/deferred work is accounted for as follows:

- Fleet migration and removal of both template fallbacks are needed for R1-F1;
  initialization writes are now available, so the earlier prerequisite has
  already landed.
- Root-reference file bases and relative `with:` inputs are covered by R1-F2.
- Diagnostic-prefix cleanup, extra graph metadata, unsupported sibling
  `types`, and splitting a topic page solely for transclusion are unnecessary
  to this fix and do not stop the review cycle.
- Dedicated interactive sequence-document collection coverage was omitted.
  The specification explicitly assigns this fix Level 1 coverage and reuses
  existing collection behavior; a new real-terminal test is not required here.
- The log reports 12 pre-existing Windows CLI failures left untouched. This
  review did not independently rerun or inspect those unrelated failing test
  implementations; that report is not converted into new findings or passing
  Windows evidence. CI remains responsible for cross-OS proof.
- Earlier phase deferrals for ownership, CLI collection, context propagation,
  and documentation were subsequently implemented. Earlier slow timings were
  followed by later measurements; the review's own affected test run and
  artifact remeasurement have no unresolved timing failure.

## Implementation record corrections

Corrected the false side-effect/null claim in place in the log's decision and
Windows-probe entries and in `implementation.md`, with dated verification
notes. The printed `set` result contains prior values; direct CLI executions
followed by a shell reading the written value prove `signed=ada` for direct,
root-reference, and nested-reference cases. The library code explicitly
serializes those prior values.

Also narrowed the record's blanket claims about prepared nested template
values and pre-initialization checks, clarified that the relative `with:`
rejection occurs during passive approval while runtime preparation is correct,
and added dated explanations of the reproductions. Those record corrections are informational; the independently
verified implementation defects are R1-F3 and R1-F5. The specification's design
text was not edited; only `review_iterations` was incremented to 1.
