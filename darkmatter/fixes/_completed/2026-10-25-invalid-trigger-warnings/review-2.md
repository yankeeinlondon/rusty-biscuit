---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): early composition exits discard discovered trigger warnings"
    - "R2-F1 (high): preflight-only trigger warnings bypass explicit denial"
observations: []
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-25T19:22:11-07:00"
spec: 2026-10-25-invalid-trigger-warnings/spec.md
implemented: true
description: "A **fix** review of `2026-10-25-invalid-trigger-warnings/spec.md`"
fix: 2026-10-25-invalid-trigger-warnings/review-2.md
previous: 2026-10-25-invalid-trigger-warnings/review-1.md
next: 2026-10-25-invalid-trigger-warnings/review-3.md
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-25-invalid-trigger-warnings/log.md
implemented_by: claude/opus
---

# Review 2

The fix is **not production ready**. The previous stream-routing and
source-location findings are fixed. Warning retention is only partially
fixed: preparation can still lose warnings before approval, after approval,
or when a later child fails. A separate policy defect lets an approved shell
command execute despite an explicitly denied warning already discovered in
a conditionally skipped child.

No human decision or external prerequisite blocks either repair. The repair
log's choices about source-free JSON locations, inspection's source-only
stdout list, failed clean reports, and passive shell inspection remain
accepted. They need no reconfirmation.

## Previous Finding Verification

Required instances come from `review-1.md` and the repair's `### Finding
Ledger` in `log.md`. The ledger's added child rows are included. Completion
claims were checked against code and execution. There are no earlier reviews
in this directory.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: early composition exits discard discovered trigger warnings | R1-F1.1 through R1-F1.14: inspection, lifecycle exits, ordinary/approval failures, library carriers, Claudine, reference refusal, rendering | **partially fixed** | Named inspection, lifecycle, refusal, and rendering regressions pass. The original R1-F1.5 approval-failure case still fails: incident trigger plus unapproved `::shell echo review-probe` gives no trigger warning. | Original R1-F1.5 approval-failure portion; newly found sibling instances R1-F1.15 through R1-F1.20 below. This is a partial repair, not a regression of the repaired bootstrap/inspection cases. |
| R1-F2: inspection and failed clean reports route warnings to the wrong stream | R1-F2.1 through R1-F2.10: inspection, clean, validation, composition, Claudine, DMLS, other-command sweep | **verified fixed** | Inspection warns on stderr; failed clean retains warnings in its version-1 JSON envelope with exit 1. Validation, composition, Claudine, and editor controls pass. Other commands retain their discovery boundaries. | Nothing. Absent warnings on preparation exits remain R1-F1, rather than reopening routing. |
| R1-F3: warning projections discard available source information | R1-F3.1 through R1-F3.18: envelopes, payload/import errors, aliases, embedded identities, normalized path comparisons, editor wording | **verified fixed** | Public discovery retains payload/import identities; compose JSON preserves the available trigger position; DMLS publishes related files. Alias, symlink, and embedded controls pass. | Nothing. Causes without an exact position retain the accepted fallback. |

### Required-instance evidence

These rows also identify the repaired and clean sites in the class sweep.
Test names below belong to the indicated package's existing L1 suite.

| Instance | Verification evidence and result |
| --- | --- |
| R1-F1.1 | darkmatter-cli `compose_shell_report_keeps_the_warning`: inspection succeeds with one stderr warning. |
| R1-F1.2 | `compose_shell_report_failure_keeps_the_warning`: root warning retained before missing authored schema. The earlier-successful-child variant remains below. |
| R1-F1.3 | `compose_lifecycle_skipped_by_initialize_keeps_the_bootstrap_warning`: ordinary skip warns once. |
| R1-F1.4 | Same test's JSON leg: one warning, null document, success, no stderr warning. |
| R1-F1.5 | Ordinary success and schema/composition-failure controls pass. **Approval failure remains broken:** temporary root-trigger/unapproved-shell probe exits 1 with only “Approval required” and zero trigger warnings. |
| R1-F1.6 | `compose_lifecycle_past_initialize_warns_once`: success/failure, ordinary/JSON, one warning. |
| R1-F1.7 | Darkmatter `invalid_triggers` covers composition, authored-schema failure, transclusion, denial, effective-schema results, and successful public preflight. Additional cross-sibling failure is below. |
| R1-F1.8 | Claudine's 13 filtered tests pass, including preliminary failures, inline composition, loops, sequences, and invocation deduplication. |
| R1-F1.9 | Claudine source inspection confirms selected/direct composition and shell preparation do not enable trigger discovery; separate preliminary schema checks own discovery. Accepted boundary preserved. |
| R1-F1.10 | darkmatter-cli lifecycle call rejection retains warnings in ordinary and JSON output. |
| R1-F1.11 | Lifecycle denied-call-warning test retains the separate trigger warning. |
| R1-F1.12 | Root/child-only reference-refusal and recovered-reference tests pass, including lifecycle and both output modes. An additional public clean-reference control with an included child also retains one final CLI warning. |
| R1-F1.13 | Render-failure test retains warnings for invalid page width, with/without lifecycle. |
| R1-F1.14 | Initialize failure and lifecycle-shell-preparation failure tests retain bootstrap warnings in both modes. |
| R1-F2.1 | Inspection tests assert stderr warning and source/code-only stdout trace. |
| R1-F2.2 | Failed clean JSON test asserts version 1, structured warning, exit 1, no human trigger-warning row on stderr. |
| R1-F2.3 | Ordinary clean success and missing-schema failure retain one stderr warning. |
| R1-F2.4 | Successful clean JSON retains its top-level warning collection and clean stderr. |
| R1-F2.5 | JSON validation tests retain warnings on success/failure and first discovering input, including assignments. |
| R1-F2.6 | Multi-file ordinary validation warns once on stderr. |
| R1-F2.7 | Compose success, authored-schema failure, valid-trigger rejection, root denial use the correct stream/severity. Preparation omissions remain R1-F1. |
| R1-F2.8 | Claudine CLI tests preserve stderr delivery through normal/failing paths. |
| R1-F2.9 | DMLS sessions consume framed protocol messages and receive warning diagnostics without human protocol-stdout output. |
| R1-F2.10 | Other-command source sweep confirms discovery remains in compose, clean, schema validation, and trigger inspection. The other commands identified by the ledger do not independently discover triggers. |
| R1-F3.1 | Positioned-envelope library and CLI JSON tests assert trigger file at line 3, column 1. |
| R1-F3.2 | Scalar payload `42` retains its `AmbiguousReferenced` payload path; DMLS asserts related file. |
| R1-F3.3 | Referenced root union and aliased non-object tests retain actual payload identity/message; editor publishes related file. |
| R1-F3.4 | Alias to raw JSON property type `12` retains `raw.json` as compilation source; editor links it. |
| R1-F3.5 | Discovery keeps `(3, 1)`; DMLS uses the corresponding trigger line. |
| R1-F3.6 | Malformed payload keeps separate payload identity; its coordinates are not projected onto the trigger. |
| R1-F3.7 | Constraint failures keep `SchemaDocument`/referenced-file paths through the exhaustive classifier; payload tests pass. |
| R1-F3.8 | Direct raw-JSON compile failure retains its file; alias case proves chained attribution too. |
| R1-F3.9 | Broken imported-type-file tests retain the import and failed-load observations. |
| R1-F3.10 | Unknown imported-type tests retain the searched import or the import containing the rejected reference, and explain its identity. |
| R1-F3.11 | Missing payload/import tests retain lookup observations without invented cause-file locations. |
| R1-F3.12 | Unsupported keys retain file-level/editor and source-free/JSON fallbacks. |
| R1-F3.13 | `finding.rs::related_path` exhaustively classifies every error variant; path-bearing arms, cycle tests, and normalized self-path exclusion preserve the contract. |
| R1-F3.14 | CLI validation/clean warning rows retain their established fields and location text. |
| R1-F3.15 | Embedded discovery/compose tests keep virtual identity without host path, local link, or watch. |
| R1-F3.16 | Unknown-type-inside-import test names `types.yaml`; editor checks the same related source. |
| R1-F3.17 | Symlink-root/self-reference test passes; comparisons use simplified canonical forms and preserve the cause's spelling. |
| R1-F3.18 | DMLS assertions use “the trigger's schema failed to load here,” covering payloads and imports. |

## Unblocked Findings

### R1-F1 (high): early composition exits discard discovered trigger warnings

**Authority: “Visible, accurate warnings” requires warnings independently of
an effective schema and says “Retain already collected warnings if later
authored-schema resolution, validation, or composition fails, and deliver
them with that failure.” Acceptance items 5 and 6 require preservation
across preflight and consuming command paths.** The defect class is loss of
collected discovery warnings across preparation success/failure boundaries.

The original approval-failure portion of R1-F1.5 remains broken. Darkmatter's
[`compose_preflight_approvals_with`](../../lib/src/markdown/compose/preflight/lifecycle.rs:120),
which turns preparation into shell approvals, discards `report.warnings` on
success and subsequent policy errors. Its result has no warning field.
Neither darkmatter-cli consumer can merge those warnings into its report.
This hides warnings found only in a child that actual composition skips.

Also, Darkmatter's
[`collect_effects`](../../lib/src/markdown/compose/preflight/collect.rs:196),
which walks documents before effects, drops its accumulated warning list
when a later child fails. A successful earlier child's local failure guard
has already been disarmed; ancestor/failed-child guards cannot reconstruct
that child's warning.

**Reproductions.** Reuse the existing CLI incident fixture. Keep the root
incident trigger and add `::shell echo review-probe` to its valid document,
without a shell whitelist. Ordinary compose exits 1 with “Approval required”
and no trigger warning. This is the original required approval-failure case.

For child-only cases, reuse `child_only_trigger_repository`: a Cargo
workspace containing package `alpha`, the incident trigger at
`alpha/schemas/memory.yaml`, a valid `alpha/docs/child.md`, and no invalid
trigger in the parent's schema roots. Use this parent body:

```markdown
Parent body

::file ../alpha/docs/child.md when=false
```

`md compose --shell docs/prompt.md` proves discovery by warning once.
Ordinary compose succeeds with no warning; JSON has an empty diagnostic
array. Adding `initialize: [{info: start}]` leaves the omission unchanged.
Remove `when=false` and add an unapproved root shell: ordinary/lifecycle
compose both fail with zero warnings, in ordinary/JSON output. A user
blacklist rejecting that command has the same loss.

For cross-sibling failure, include the alpha child first, then `docs/bad.md`
whose only change from a valid document is `$schema: ./missing.yaml`.
Shell inspection and ordinary/lifecycle compose fail with the schema error
but omit alpha's earlier warning.

The previous-instance evidence table above covers the other sibling sites,
including those repaired or clean. New child IDs below extend R1-F1's
existing inventory; they identify newly found siblings of that finding.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.5 | darkmatter-cli ordinary approval failure | Root incident trigger plus unapproved shell | Exit 1; zero trigger warnings | Original approval error plus one trigger warning |
| R1-F1.15 | Darkmatter recursive collector/failure sink | Earlier child discovers trigger; later child has missing schema | Successful sibling's warning lost on failure; reproduced through inspection and both compose consumers | Complete discovered warning prefix, once, in order |
| R1-F1.16 | Darkmatter approval-wrapper failure projections | Successful collection, then approval-required or user-blacklist rejection; root/child variants | Collected list discarded before policy error | Carry warnings through every error after collection; the same unguarded boundary also covers policy-file errors and handler rejection |
| R1-F1.17 | Darkmatter successful approval-result projection | False-condition child found by preflight | Result has graph/approvals/stats but no warnings | Preserve warnings for invocation delivery |
| R1-F1.18 | darkmatter-cli ordinary approval consumer | Child-only trigger; skipped-child success and approval failure; ordinary/JSON | Zero delivered warnings | Merge successful/failing preparation warnings |
| R1-F1.19 | darkmatter-cli lifecycle approval consumer | Same cases, initialize continuing | Zero delivered warnings; root-bootstrap fixtures mask this omission | Retain child findings independently of bootstrap |
| R1-F1.20 | Darkmatter in-pipeline membership collector | Public composition with exact approval and false-condition child; public preflight control warns once | One shell expansion; successful composition warning list empty | Retain collected warnings in composition report; R2-F1 also requires timely policy checks |
| R1-F1.1 / R1-F1.7 | Public preflight and CLI inspection controls | Same false-condition child, no later failure | One warning; clean | Preserve |
| R1-F1.2 | Failed collector control | Root warning followed by root missing schema | Warning retained; clean | Preserve while retaining successful siblings too |
| R1-F1.12 | Reference-check/invocation controls | Included child with no reference issues; recovered/refused reference variants | Final warning once; public clean-reference report also carries it; clean in these cases | Preserve existing discovery boundaries |
| R1-F1.8 / R1-F1.9 | Claudine preliminary and composition consumers | Existing invocation tests; discovery-disabled composition invariant | Passing/invariant excludes these child findings; clean | Preserve |

**Smallest repair:** finish the warning carriers already introduced here.
Preserve the complete collector prefix on failure, carry warnings through
approval success and every subsequent error, merge them in both CLI
consumers, and retain the membership walk's warnings in composition. Keep
first-observed identity/order when merging. Add focused L1 regressions for
these rows. No new parser or child-discovery rules are needed.

### R2-F1 (high): preflight-only trigger warnings bypass explicit denial

**Authority: acceptance item 9 requires explicit denial to stop composition
“before its next effect.” “Visible, accurate warnings” explicitly includes
warnings learned “during preflight or child composition.”** The defect
class is failure to apply explicit warning policy to discovery results
before permitting effects.

R1-F1 follows warning values to delivery; this finding follows them to the
decisions permitting execution. Displaying the warning after composition
does not repair an effect that already ran. The collection pass deliberately
defers blocking checks, but the approval wrapper never checks its returned
trigger warnings. The independent in-pipeline
[`validate_pre_approved`](../../lib/src/markdown/compose/preflight/mod.rs:312),
which checks supplied shell approvals, discards those warnings too.

Use the child-only fixture above with `when=false`. Add
`::shell echo review-probe-effect` before the child directive and an existing
`docs/.darkmatter-shell-whitelist` containing `prefix echo`. The command is
approved; denial of the trigger warning independently requires stopping it.

```text
md compose --deny dm.schema.invalid_trigger --output report-json docs/prompt.md
md compose --deny all --output report-json docs/prompt.md
```

Both exit 0 with no diagnostics and include `review-probe-effect` in the
document. Repeating with initialize continuing gives the same result.
The public API probe supplies an exact `ShellPreApprovals` entry, enables
discovery and the blocking policy on one request, confirms public preflight
returns one warning, then calls `Markdown::compose_with_warnings`. It succeeds
with `shell_expansions_applied == 1` and no warnings. This is also a library
defect, not solely CLI wiring.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R2-F1.1 | Darkmatter approval wrapper before policy/approval decisions | False-condition child; discovered warning explicitly denied | Warning never checked before approval succeeds | Apply request policy before permitting the next effect |
| R2-F1.2 | darkmatter-cli ordinary composition | Whitelisted root shell plus skipped child; code/all denied | Exit 0, shell output in document, zero warnings | Policy stop, warning severity/source preserved, shell not executed |
| R2-F1.3 | darkmatter-cli lifecycle composition | Same input with initialize continuing; both denial selections | Same success and executed shell | Stop before subsequent shell effect |
| R2-F1.4 | Darkmatter full-document approval-membership gate | Public composition with exact approval and denied child-only warning | Shell executes; success has one shell expansion and no warning | Check/retain warning before execution independently of CLI wrapper |
| R2-F1.5 | Root preparation/bootstrap policy controls | Root incident trigger and potential frontmatter effect; code/all denied | Existing effect-counter/CLI tests stop before effects; clean | Preserve |
| R2-F1.6 | Active-child policy control | Remove `when=false`; denied code; root shell writes temporary marker; ordinary/lifecycle | Policy stop, marker absent; clean | Preserve while fixing preflight-only case |
| R2-F1.7 | Frontmatter-only membership branch | Branch walks no body children; root warnings use preparation checks | Child-only value cannot arise here by contract; clean | Keep discovery boundaries and root enforcement |
| R2-F1.8 | Discovery opt-out and passive inspection | Existing disable-trigger tests; same child's successful `--shell` inspection | Disabled discovery emits nothing; passive inspection warns without effects; clean | Preserve accepted inspection behavior |
| R2-F1.9 | Claudine composition/approval consumers | Trigger discovery disabled in these passes; preliminary checks own findings | This child-only value cannot arise by accepted invariant; clean | Do not expand discovery scope |

**Smallest repair:** enforce the existing blocking policy on collected
invalid-trigger warnings at both the approval and in-pipeline membership
boundaries, before another effect. Use existing blocked-diagnostic errors
and warning carriers, preserving severity/source. Keep default recovery and
passive inspection unchanged. L1 public-API effect counters and a small CLI
wiring test are sufficient; no terminal or approval UI automation is needed.

## Blocked Findings

None.

## Recurrence

R1-F1 repeats the retention class from `review-1.md`. Its original approval
failure is an **unchanged instance**: the earlier review labeled that portion
clean, and the repair did not establish it with an unapproved-shell
regression. Inspection and bootstrap repairs themselves are verified.

R1-F1.15 through R1-F1.20 are **siblings the earlier sweep missed**: successful
collection into approval success/failure, ordinary/lifecycle delivery,
in-pipeline membership collection, and later-sibling failure. Root-only
fixtures let rediscovery/bootstrap findings hide several omissions.

R2-F1 is a newly identified decision-consumer sibling of the lost-warning
class in R1-F1: earlier checks did not follow warnings learned only during
preflight to the policy decisions permitting effects. It is separate because
late warning delivery cannot undo execution. The explicit preflight/effect
requirement gives the repair a concrete boundary; no broader robustness
guarantee or new discovery behavior is requested.

## Verification and coverage

Independently executed package-area nextest recipes:

- Darkmatter `just test invalid_trigger`: 57 passed, repeated after removing
  temporary probes, with the same result.
- Darkmatter `just test trigger_`: 357 passed, including discovery recovery,
  strict input matrices, provenance, refresh, and embedded sources.
- Claudine `just test invalid_trigger`: 13 passed.
- Darkmatter `just test shipped_schema`: 14 passed.
- Both areas' `just check-tier-coverage`: zero stranded tests.

Counts overlap and are not a unique-test total. Temporary probes reused
existing isolated fixtures/public APIs and reproduced the findings above.
Clean-reference and active-child effect-prevention controls passed. An
exploratory TOC-only probe did not establish trigger discovery and is
excluded from findings. All temporary source additions were removed. No
production source changed for this review; no terminal/browser was opened
or focused.

Required tests are declared by consolidated L1 targets; actual nextest
selection proves execution. Recipes enable the CLI fixture features. Full
lint/test and cross-OS results in the repair log are implementer claims, not
substitutes for these focused checks. Cross-OS proof does not block this
review's readiness decision.

| Spec acceptance requirement | Appropriate verification level and evidence | Assessment |
| --- | --- | --- |
| 1: incident recovery | L1 normal CLI/public composition | Passing |
| 2: valid neighbors, ordering, enforcement | L1 discovery/validation and CLI assignments | Passing |
| 3: invalid envelopes/payloads/imports/compile cases; strict parsing | L1 public matrices and payload/provenance tests | Passing |
| 4: all-invalid, matcher-only, ignored YAML, shadowing, infrastructure | L1 discovery/embedded tests | Passing |
| 5: invocation deduplication and repair/creation/deletion refresh | L1 public API and DMLS sessions | Refresh/dedup controls pass; retention remains R1-F1 |
| 6: hosts, reports, preliminary checks, independent failures | L1 isolated CLI/fake-provider runs and editor sessions | R1-F1 remains; routing fixed |
| 7: explicit-schema/valid-schema enforcement; shipped corpus | L1 API, CLI, passive artifact corpus | Passing |
| 8: editor registry, completion, hover, source diagnostics | L1 DMLS protocol sessions/registry tests | Passing, including related payload/import sources |
| 9: denial before effects; opt-out | L1 effect counters, shell-output/marker checks, API/CLI | Root/active-child controls pass; preflight-only case fails under R2-F1 |
| 10: clean and editor formatting | L1 CLI output/JSON and editor protocol tests | Passing, including failed clean JSON warnings |

L1 is sufficient: requirements concern schema results, streams, structured
reports, editor protocol results, and effect prevention. None depends on
terminal encoding, keyboard events, rendered geometry, or focus. There is no
L2/L3 mismatch.

### Input robustness

The repair changes warning carriers/source projection rather than adding a
configuration loader. Existing strict envelope/payload validators remain.
Their public matrices ran in the trigger filter: an invalid classified
trigger yields one finding and no installed payload; infrastructure errors
remain errors. Exceptions are explicit: absent/non-trigger `kind` is ignored,
absent payload is matcher-only, empty schema remains an explicit empty
schema, and omitted optional conditions remove that predicate.

Shared YAML-loader tests cover duplicate keys and trailing invalid content;
embedded block YAML and JSON syntax use the same classifier. Null, wrong
whole-field types, mixed/all-invalid collections, and invalid empty conditions
do not silently become missing/empty values. The generated-cell guard runs.

The smell search over discovery, assembly, and finding projection found no
new permissive field default or element-filtering parse. Assembly's
directory-read `.ok()` makes failed freshness checks miss the cache, after
which strict loading decides the result. Its tally default and discovery's
path/name filters are not configuration-field validators. The findings here
concern loss/non-enforcement of valid warning records, not permissive parsing.

## Observations

None.
