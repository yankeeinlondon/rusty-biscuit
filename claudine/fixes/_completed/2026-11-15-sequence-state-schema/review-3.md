---
$schema: feature-review.yaml
ready: false
findings:
  - 'R2-F1 (medium): Prepared provider dependencies are restored only one level deep'
  - 'R3-F1 (medium): Dependency restoration imports unrelated document properties'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-11-15T22:30:19-08:00
spec: 2026-11-15-sequence-state-schema/spec.md
implemented: true
description: 'A **fix** review of `2026-11-15-sequence-state-schema/spec.md`'
fix: 2026-11-15-sequence-state-schema/review-3.md
previous: 2026-11-15-sequence-state-schema/review-2.md
next: 2026-11-15-sequence-state-schema/review-4.md
log: '/Volumes/coding/wt/rusty-biscuit/feat-state-schema/claudine/fixes/2026-11-15-sequence-state-schema/log.md'
implemented_by: claude/opus
---

# Sequence state schema review, iteration 3

The fix is **not production ready**. The original shell-output failures are
fixed. The repair's additional provider-dependency check is incomplete: it
loses longer dependency chains and can import a different document's reads.
These produce opposite mistakes: refusing a runnable sequence before it starts,
and running an earlier task before rejecting a provider that was already known
to be unavailable. Both have small, local reproductions and need no author
decision.

## Previous Finding Verification

Read `review-2.md`, the Finding Ledgers in `log.md`, `review-1.md`, both
implementation records, and the implementation. Neither earlier review has a
Human rulings section. No finding was blocked. The first review's findings
were closed by iteration 2; they are included here to distinguish those repairs
from the new dependency-layer failures.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1 — Referring-document properties still enter referenced templates | R1-F1.1–R1-F1.9: four reference routes, discovery, direct/precedence/input controls, fleets/docs | verified fixed | Removed fallback merges remain absent from `source.rs` and `preflight/inputs.rs`. Referenced-template, discovery, missing-input, and research artifact cases passed in the sequence run. | None of the original instances. R3-F1 concerns a new dependency projection, not template values. |
| R1-F2 — Relative task references and file inputs lose their authoring folder | R1-F2.1–R1-F2.9: four executable kinds, route controls, relative/absolute `with:`, approval/barrier/execution | verified fixed | Declaring origins still reach `Loader::walk_plan`, discovery, `PromptReference::with_origin`, and `task_caller_records`. Corresponding `sequence_sources_cli` and `with_file_provenance` tests passed. | None. |
| R1-F3 — Nested templates read raw document values instead of prepared values | R1-F3.1–R1-F3.7: root, inline, group, task-file, referenced owner, discovery, validation/consumption | verified fixed | `owner_template_values` still uses the owning document's prepared map. The owner-value matrix and CLI template-fill cases passed, including absent/null defaults, derived values, setter precedence, and item shadowing. | None of the original template sites. |
| R1-F4 — Referenced document schemas are not checked against current runtime values | R1-F4.1–R1-F4.13: reference routes, controls, empty/loop cases, row/event owners, defaults, initialized inputs; repair-added R1-F4.14 transferred to R2-F1 | verified fixed | `prepare_whole_document`, referenced owner preparation, root-reference judgment, and initialization input layering remain wired. `referenced_owner_schema`, prepared shell values, and CLI runtime-schema tests passed. R1-F4.14's shell output is now correct in every original route. | None of the original checks or the transferred shell-output case. |
| R1-F5 — Edited inline checks reject containers after initialization has already run | R1-F5.1–R1-F5.8: four placements, invalid/guard/retry/loop controls, whole-document proxy | verified fixed | `admit_inline_container` rereads before opening the run; `stale_admission` and `sequence_frozen_graph` tests passed, including no initialization effect on refusal, false guards, retries, loops, and proxy controls. | None. |
| R2-F1 — Derived document values reach first-position shell sites as literal template text | R2-F1.1–R2-F1.20 from review 2, plus repair-ledger R2-F1.21–R2-F1.23 | partially fixed | All original shell cases passed. The full child accounting is below. Independently reproduced that repair-added R2-F1.23 restores only one link of a prepared provider dependency. | R2-F1.23 remains incomplete for two or more derived properties. The separate owner-collision sibling is R3-F1. |

The following accounts for every R2-F1 ledger child, including clean controls:

| Required children | Verified result |
|---|---|
| R2-F1.1, .2, .3, .4, .5 | First shell prints `hi ada` directly and through root, nested, group-leg, and task-file references. |
| R2-F1.6, .7, .8, .9, .10 | Inline child prints `hi ada` through all five routes; no false approval-membership refusal. The old refusal's truncated status line is no longer reached. |
| R2-F1.11, .12 | Root and nested/group-leg/task-file lifecycle shells print `hi ada`. |
| R2-F1.13, .14 | Inline sequence and group lifecycle shells print `hi ada`, directly and under referenced owners. |
| R2-F1.15, .16 | Root and nested/group-leg/task-file shell-valued writes capture `hi ada`. |
| R2-F1.17 | Later shells remain correct. CLI direct/nested controls passed; the other routes use the same later-position template policy. |
| R2-F1.18 | Literal controls remain correct; public graph route matrices and CLI controls passed. |
| R2-F1.19 | Side-effect shell spelling still receives the typed instruction to use a shell task. |
| R2-F1.20 | Template/state consumers still receive prepared owner values; owner-value matrices and CLI cases passed. |
| R2-F1.21 | Late-root/environment/runtime-write/shell-derived graph tests passed; `layer_prepared_values` uses the shared transitive classifier and explicitly seeds `seq`, `loop`, `group`, `env`, `agent`, and `model`. The start-write CLI control prints its updated value. |
| R2-F1.22 | The early-binding comment, topic docs, and repo skill now describe prepared ordinary values and retained unstable expressions accurately. |
| R2-F1.23 | One-link provider control passes, but two/three-link `with.agent` and `with.model` dependencies remain static on all six tested placements. **Partially fixed.** |

## Verification and requirement coverage

Used the `claudine`, `rust-testing`, and `just` skills. Reviewed declaration
presence/shape checks, document preparation, normalization, discovery,
composition views, runtime admission/rereads, frozen children, shell approval,
and the repair's new dependency bookkeeping. No production code was changed.

Verification on macOS, with builds run sequentially:

- `just test sequence` in `claudine/`: **2,103 passed**, 10,315 outside the
  filter; 81.493 seconds of test execution; none at or above five seconds.
- `just test-library shipped_artifacts::`: **27 passed**; none at or above
  five seconds, including artifact freshness and uncached parity.
- `just check-tier-coverage claudine`: **zero stranded tests**. New library
  files are declared in their module trees; CLI cases belong to the existing
  fixture-enabled L1 target.
- Two temporary in-process diagnostic probes inspected the returned
  `PreflightGraph` and public `HintState` values: 96 dependency-chain cases
  and 56 ownership/control cases, taking 0.832 and 0.705 seconds respectively.
  They used private temporary fixtures and the existing graph test helpers.
  The diagnostic additions were removed and the original file restored byte
  for byte. They are observations, not retained regression coverage.
- Independent shipped-CLI probes used private homes/configs, a restricted
  executable path, closed stdin, audio dry run, and a private Rendezvous
  endpoint. Goose was a local script returning `answer`; Claude was absent.
  No actual provider, network service, terminal window, or focus change was
  involved.

The repair log records a full-area run of 12,403 passing tests with no slow
tests and a clean lint run. This review reran the affected sequence and schema
artifact scope, rather than unrelated packages. Cross-OS evidence is left to CI.

| User-facing requirement | Appropriate verification present |
|---|---|
| Document `$schema`, caller collection/coercion, direct/reference and YAML/Markdown agreement, empty documents | Level 1 public preparation/discovery tests and CLI frozen reproduction, setters, missing-input, empty-list, and reference tests |
| `$state` judges only normalized immediate state; templates use prepared owners and item shadowing | Level 1 declaration/resolver, owner-value matrix, and CLI filled-state tests |
| Inline checks follow guards, precede initialization, and reread at existing entry boundaries | Level 1 library execution and CLI edited-check, guard, retry/loop tests |
| Placement/reserved writes, references/imports, supported document formats and inactive data selections | Level 1 declaration, loader, resolver, grammar-matrix, and CLI wiring tests |
| Approved children, order, identities, and template state remain frozen | Level 1 frozen-graph, cache, retry/loop, and approval tests |
| First-position shells consume ordinary prepared values while genuinely late reads stay late | Level 1 graph tests and actual CLI output/capture/membership assertions; original failures verified fixed |
| Shipped/disk schemas agree and artifacts are fresh | Level 1 shipped-schema and artifact tests |

These requirements concern data preparation and execution boundaries. They
make no terminal-rendering or keyboard-encoder promise requiring Level 2 or 3.
The new observations below expose cases the retained dependency tests miss.

## Unblocked Findings

### R2-F1 (medium): Prepared provider dependencies are restored only one level deep

**Authority — defect:** the Claudine library's
[`Loader::finish`](../../lib/src/composition/sequence/preflight/mod.rs:2939)
can classify a runtime-dependent provider as fixed and abort a runnable
sequence. The repair ledger's R2-F1.23 requires following a replaced derived
key's authored reads to runtime writes. It does so for one property, but not
for two. This is the incomplete repair-added instance of R2-F1; the original
shell-output instances are closed.

**Defect class:** dependency reconstruction restores only the first edge of a
transitive read graph after preparation replaces expression text with values.

Minimal runnable fixture, with Goose installed and Claude absent:

```yaml
# seq.yaml
kind: sequence
pick: claude
mid: "{{ pick }}"
choice: "{{ mid }}"
sequence:
  - { name: writer, prompt: writes.md }
  - { name: reader, prompt: plain.md, with: { agent: "{{ choice }}" } }
```

```markdown
---
agent: goose
success: [{ set: { pick: goose } }]
---
Write.
```

Save the Markdown as `writes.md`; `plain.md` contains `Plain.`. Run
`claudine sequence seq.yaml --yolo`. **Observed:** exit 1, “Claude is not
installed,” and “No task has started.” **Expected:** defer the reader's
provider until its turn; the writer sets `pick`, and both tasks use Goose.
The control changing only `choice` to `"{{ pick }}"` exits 0 with two local
stub launches. Two ordinary derived settings are sufficient to reach this.

`prediction.dependencies` initially contains `choice` with its prepared
literal. `finish` appends authored `choice: "{{ mid }}"`, but its filter ran
before that append and never appends authored `mid: "{{ pick }}"`.
Consequently
[`defer_runtime_written`](../../lib/src/composition/sequence/preflight/providers.rs:202)
cannot reach the write to `pick`. The shared logic handles both provider
and model selection; fixing only the provider field would leave its sibling.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R2-F1.23 | Direct root step provider/model dependency restoration | `with.agent` and `with.model`; two/three derived links to a prompt's runtime write | Both remain `Static`; provider CLI aborts before writer | Both `DecidedAtLaunch` |
| R2-F1.24 | Root whole-document reference | Same two fields and chains | Both static; provider CLI aborts before writer | Both deferred |
| R2-F1.25 | Nested whole-document reference | Same | Both static; provider CLI aborts before writer | Both deferred |
| R2-F1.26 | Inline sequence child | Same | Both static; provider CLI aborts before writer | Both deferred |
| R2-F1.27 | Referenced sequence in group leg | Same | Both static; provider CLI aborts before writer | Both deferred |
| R2-F1.28 | Referenced sequence through task file | Same | Both static; provider CLI aborts before writer | Both deferred |
| R2-F1.29 | Prompt-owned expression classifier, all six routes | Prompt owns the complete provider/model chain; zero through three derived links | Both correctly deferred | Same — clean |
| R2-F1.30 | Direct-read and one-link controls, both fields on all six routes | `with` reads `pick` or one alias | Both correctly deferred | Same — clean |
| R2-F1.31 | Shell approval and state/template consumers | Original R2-F1.1–.22; later shell policy | Passing original outputs; later shell holes are shape-approved; template state deliberately freezes at graph construction | Same — clean; these decisions do not use the new provider dependency restoration |

The CLI sweep establishes the premature refusal on all six placements. It
does not promise that a nested container recomposes its derived document
properties between children: the spec retains its existing pass boundaries.
The in-process sweep checks the prediction policy itself at those sites.

**Recommended repair:** retain the complete authored read closure for each
prediction, using the existing dependency analysis, before prepared values
erase it. Ensure the restoration reaches all dependencies, rather than doing
a single append. Keep the ownership rule in R3-F1 when making this change.
A small in-process chain test for both selection fields and one CLI
writer/reader test are enough; no new expression syntax or protective layer
is needed.

### R3-F1 (medium): Dependency restoration imports unrelated document properties

**Authority — defect:** the Claudine library's new
[`prepared_replacements`](../../lib/src/composition/sequence/preflight/mod.rs:1154)
is one list for the entire walk. `Loader::finish` attaches entries to a
prediction by property name alone. A sequence's derived `choice` can therefore
replace the dependency of a prompt's unrelated, literal `choice`. This moves
a fixed, unavailable provider past preflight and lets an earlier task run.
The static provider path normally refuses it before any task starts, as the
literal-root control below confirms.

**Defect class:** dependency identity loses its owning scope when a global
name-based restoration overwrites a prediction's own dependency value.

Use the same `writes.md` fixture, then:

```yaml
# seq.yaml
kind: sequence
pick: goose
choice: "{{ pick }}"
sequence:
  - { name: writer, prompt: writes.md }
  - { name: reader, prompt: reads.md }
```

```markdown
---
choice: claude
agent: "{{ choice }}"
---
Read.
```

Save the latter as `reads.md`. With Goose's local stub installed and Claude
absent, **observed:** the writer runs and succeeds, then the reader fails
because Claude is absent. **Expected:** refuse the fixed unavailable provider
before the writer runs. The prompt's `choice` is the literal `claude`; it
reads no `pick`. Changing only the root's `choice` to literal `goose` restores
the correct early refusal with zero launches. Root and prompt values have
different meanings even when their property names match.

This is also reachable when the replacement came from an entirely separate
referenced sequence: an empty sibling document defining derived `choice`
can change the reader's classification. No value inheritance authorizes that
dependency inheritance.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | Direct sequence owner versus prompt owner | Root derived `choice`; prompt literal `choice`, expression `agent` or `model`; graph writes `pick` | Both hints incorrectly deferred; agent CLI runs writer before refusal | Both remain static; unavailable agent refuses before launch |
| R3-F1.2 | Root-reference owner versus prompt | Same | Both incorrectly deferred | Both static |
| R3-F1.3 | Nested referenced owner versus prompt | Same | Both incorrectly deferred | Both static |
| R3-F1.4 | Inline enclosing owner versus prompt | Same | Both incorrectly deferred | Both static |
| R3-F1.5 | Group-leg sequence owner versus prompt | Same | Both incorrectly deferred | Both static |
| R3-F1.6 | Task-file sequence owner versus prompt | Same | Both incorrectly deferred | Both static |
| R3-F1.7 | Independent referenced sibling owner versus prompt | Empty sibling owns derived `choice`; reader is outside it | Both incorrectly deferred | Both static |
| R3-F1.8 | Literal-root controls at all seven sites | Change only unrelated owner's `choice` to literal | Both static | Same — clean |
| R3-F1.9 | Literal `with` overrides at all seven sites | Literal provider/model replaces the prompt expression | Both static | Same — clean; no dependency read |
| R3-F1.10 | Caller-setter overrides at all seven sites | Literal provider/model setter | Both static | Same — clean; no dependency read |

**Recommended repair:** preserve the dependency's lexical owner and layer.
Reconstruct reads only from the authored values that actually supplied that
prediction; never replace a prompt-local or `with`-local dependency using
another owner's same-named property. Prefer retaining that authored closure
at prediction construction to repairing a scope-free list afterward. This
can share the small dependency fix for R2-F1; add one owner-shadowing
in-process case for each selection field and an early-refusal CLI control.

## Blocked Findings

None. Both repairs fit the existing value ownership and prediction contracts.
There is no format, public API, scope, or author-policy decision to make.

## Recurrence

R2-F1 is a **partial repair**, confined to repair-added ledger child .23.
The one-link test passes, but the new restoration machinery misses the
transitive closure. Original R2-F1.1–.22 are verified fixed. This is not a
claim that the original shell-output failures persist.

R3-F1 is a **new sibling introduced by that repair**, related to
`review-1.md`'s R1-F1 owner-isolation class and R1-F3/R2-F1 value-projection
class. The new global dependency restoration needed to sweep prompt-local
shadowing and independent referenced owners, as well as root-derived
`with` reads. The earlier template fallback repairs remain fixed.

Successive repairs have added another projection and then another recovery
step for its lost information. Both findings are medium under the recurrence
rule: no acceptance criterion specifically names chained provider aliases or
same-name dependency collisions. The repair should give this requirement a
boundary: carry the correct authored dependency closure with a prediction,
while keeping prepared values for evaluation. A growing collection of
post-processing exceptions would prolong this class of failures.

## Input robustness matrix

Default policy: invalid declaration shapes produce typed, located errors
before the rejected task launches; collection/preparation launches nothing.
Existing document defaulting and initialization rules remain in force.

- Shared schema-declaration validator: absence adds no declaration; `{}` is
  open. Nonempty supported references/unions/imports resolve. Null, empty
  string/union, wrong whole-field types, and mixed/all-invalid union elements
  fail. Root public-resolver matrix tests check these on empty and nonempty
  lists; inline checks and state declarations have wiring/location controls.
- Template validator: absence supplies no defaults; `{}` is a no-op. Null
  and non-mappings fail. Mapping values intentionally permit null, arrays,
  and objects as literal data; top-level strings evaluate. Reserved keys fail.
- State validator: `the_state_validator_judges_every_field_shape_passively`
  covers absence/null, whole-field and mixed/all-invalid element types,
  permitted empties, and passive conversion. Document tests separately cover
  absence/null defaults and input coercion.
- Loaders: the passing sequence run includes duplicate keys and malformed
  trailing content for both YAML entry paths, YAML bare-key null and second
  documents, Markdown frontmatter, shared JSON/JSON5 duplicate detection and
  syntax, and JSONL/NDJSON blank lines and located invalid records.
- Smell search: declaration `Option` fields come from presence-based map
  readers, not serde optional fields that erase null. `document.rs`'s empty
  fallback handles absence of a supplied preparation context; it does not
  swallow a parse error. No new load-bearing serde default or element-filtering
  parser was found. The present findings are dependency-analysis defects on
  valid inputs, not additional configuration-shape requirements.

## Implementation record corrections

Corrected historical passages in `implementation.md` and
`implementation-log.md`, with dated iteration 3 notes and current test
evidence: the referrer fallback was removed, relative `with:` approval has
been repaired, and the original derived-shell failure is now fixed. The remaining provider-dependency defects
are recorded above. No specification design text or repair-ledger completion
claim was rewritten; the verification table supplies this review's assessment.
