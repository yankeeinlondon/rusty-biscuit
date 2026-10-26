---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): early composition exits discard discovered trigger warnings"
    - "R1-F2 (high): inspection and failed clean reports route warnings to the wrong stream"
    - "R1-F3 (high): warning projections discard available source information"
observations: []
human_in_the_loop: null
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-25T17:54:19-07:00"
spec: 2026-10-25-invalid-trigger-warnings/spec.md
implemented: true
description: "A **fix** review of `2026-10-25-invalid-trigger-warnings/spec.md`"
fix: 2026-10-25-invalid-trigger-warnings/review-1.md
next: 2026-10-25-invalid-trigger-warnings/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-pipeline/darkmatter/fixes/2026-10-25-invalid-trigger-warnings/log.md
implemented_by: claude/opus
---

# Review 1

The fix is **not production ready**. Individual-trigger recovery, valid-neighbor
enforcement, shadowing, cache refresh, and editor registry replacement work in
the cases exercised. Three warning-delivery defects remain: some composition
paths drop warnings, two CLI paths violate the output-stream contract, and
some projections lose source information that discovery already knows.

No human decision is required. Two independent reviewers first inspected the
spec and implementation log, including `decisions_for_author: []`. The
implementation's choices have recommendations and reversible consequences.
The output exceptions recorded in Phase 2 narrow requested behavior and are
included in the high finding R1-F2; they are not questions for the author.

This is the first implementation review. There are no previous findings to
classify as repaired, partially repaired, or recurring.

## Decisions reviewed

The implementation log records these 34 decisions. They remain settled except
for the two output exceptions identified below, whose repairs follow the spec.
The review's own decisions, alternatives, reasons, and downsides are recorded
under `Review 1` in the implementation log.

| Phase | Implementation decisions and review disposition |
| --- | --- |
| 1: discovery and library | Keep a shared typed error cause; collect failed-load reads and searches in a thread-local scope; compile payloads through the existing validator cache; report composition warnings before effects; reject explicit references to malformed trigger envelopes; temporarily bridge DMLS to its former failure behavior; update tests of the former blocking behavior. Accepted. The temporary bridge was removed in Phase 3. |
| 2: Darkmatter CLI | Share delivery and source deduplication; retain findings on every schema-validation outcome; add a skipped-trigger inspection section; carry clean findings through results and errors; deliver ordinary composition failure warnings; prove embedded report sources without a CLI reproduction because `md` registers no embedded bundle. Accepted except stdout-only inspection warnings and stderr-only failed `clean --json` warnings: R1-F2 repairs both contract narrowings. |
| 3: editor | Derive findings from the installed registry; reconcile sources across scopes; remove obsolete fatal-trigger diagnostics; prioritize strict authoring errors in an open malformed buffer; restore discovery warnings on close; publish related information when a cause identifies a file; keep startup status based on failed scopes. Accepted. Source identification is incomplete for several payload failures: R1-F3. |
| 4: Claudine | Own a shared warning record in the invocation; record discovery before authored-schema resolution; recognize compose warning identities and collect successful/failed reports; drain undelivered warnings at invocation exit; treat a prep target as its own invocation; replace the former launch-blocking test with recovery and shadowing assertions. Accepted within the existing discovery boundaries; see the selected-composition check below. |
| 5: documentation | Keep one complete behavior page per area; show actual warning wording; remove the obsolete printed quoting-hint claim; preserve valid-schema control-value rejection wording; retain last-good registry wording only for whole-scan failures. Accepted. Documentation and tests encoding R1-F2's exceptions must change with that repair. |
| 6: final verification | Prove same-named source identities across two packages in one invocation; prove repeat cached discovery in-process; leave the four tests added in that phase to CI for Windows evidence. Accepted. Cross-OS proof is not a readiness gate in this review. |

The Phase 4 note about the selected-composition hook dropping warnings on
failure was checked, rather than treated as an automatic author escalation.
Claudine's current canonical composition options leave trigger discovery
disabled; its separate effective-schema checks discover triggers and record
their warnings before returning errors. Consequently a child-only invalid
trigger warning cannot originate in that hook under the current configuration.
Public direct/inline preparation probes, with and without a selector, confirmed
that invariant. This review does not expand Claudine's discovery roots or
enable trigger enforcement on previously unscanned child documents. The
generic hook limitation is not an outstanding skipped-trigger warning that
this implementation discovered and then discarded.

## Findings

### R1-F1 (high): early composition exits discard discovered trigger warnings

**Authority: spec, “Visible, accurate warnings”: “Every skipped trigger
produces a warning” and “Retain already collected warnings if later
authored-schema resolution, validation, or composition fails, and deliver
them with that failure.” Acceptance item 6 also requires existing commands
that use composition and warnings retained before independent failures.**
The defect class is loss of collected findings when an earlier preparation
path returns before the ordinary composition reporter runs.

In darkmatter-cli, the shell inspection branch of
[`run_compose`](../../cli/src/commands/compose.rs:429) discards
`ComposePreflightReport.warnings`, and propagates a preflight error without
draining its retained warnings. The lifecycle
[`bootstrap_frontmatter`](../../cli/src/commands/compose/lifecycle.rs:174)
discards its successful report and uses the warning-dropping compose API for
failure. If `initialize` skips the document, there is no later compose pass
to rediscover and display the finding.

Reproduce in an isolated Git repository using the spec's `schemas/memory.yaml`
and this control document:

```markdown
---
title: prompt
---
Prompt body
```

`md compose prompt.md` warns once. `md compose --shell prompt.md` exits 0
with “No shell commands discovered.” and no warning on either stream.
Replacing the frontmatter with the following also exits 0 without a warning:

```yaml
initialize:
  - skip
```

With `--output report-json`, that skipped document returns
`{"outcome":"success","diagnostics":[],"document":null}`. Discovery did
run during bootstrap; its warning was discarded. Replacing `title` with
`$schema: ./missing.yaml` in the `--shell` reproduction exits 1 and also
loses the warning collected before the schema failure.

The sweep followed reports into every composition delivery branch touched
by the fix. Each instance ID identifies a repair-ledger row, including controls.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | darkmatter-cli shell inspection success | Incident trigger; control document; `compose --shell` | No warning | One stderr warning; shell report and exit 0 preserved |
| R1-F1.2 | darkmatter-cli shell inspection failure | Same trigger; authored missing schema; `compose --shell` | Schema error only | Earlier warning plus existing failure |
| R1-F1.3 | darkmatter-cli lifecycle bootstrap, ordinary output | Same trigger; `initialize: [- skip]` | No warning | One stderr warning despite the intentional skip |
| R1-F1.4 | darkmatter-cli lifecycle bootstrap, JSON output | Same skipped document; `--output report-json` | Empty diagnostic array | One warning diagnostic; no warning row on stderr |
| R1-F1.5 | darkmatter-cli ordinary composition and approval failure delivery | Same incident trigger; valid document and missing authored schema | One warning on success and failure; clean | Preserve |
| R1-F1.6 | darkmatter-cli lifecycle continuing past initialize | Same trigger; `initialize: [- info: start]`, with and without a missing authored schema; both output modes | Warning delivered once by the later phase; clean | Preserve without duplicating the bootstrap finding |
| R1-F1.7 | darkmatter library report/failure carriers | Incident-trigger composition, authored-schema failure, transclusion, and denial tests | Warnings retained; clean | Preserve |
| R1-F1.8 | Claudine preliminary checks and invocation drain | Root incident trigger; compose, inline, loop, sequence, validation rejection, authored-schema failure | One invocation warning; clean | Preserve |
| R1-F1.9 | Claudine selected composition and shell-preparation consumers | Public direct/inline preparation with and without a selector; child-only trigger, valid and failing child | Current composition options do not discover triggers; no trigger finding can be dropped here | Preserve existing discovery boundaries; no new child-discovery feature required |

**Smallest repair:** retain bootstrap and shell-inspection findings at the
invocation's delivery boundary, on success and failure. Deduplicate them with
later passes instead of printing each pass independently. Use the existing
warning-preserving composition/failure carriers. Add L1 CLI regressions for
the early exits above, including a continuing lifecycle control that still
warns once. No real terminal is needed to verify this stream/report behavior.

### R1-F2 (high): inspection and failed clean reports route warnings to the wrong stream

**Authority: spec opening contract: “Ordinary CLI output writes the warning
to stderr and keeps stdout available for its normal output. Structured
report modes include the warning in their report instead.” The clean
contract additionally says “`md clean --json` keeps its version-1 envelope
and adds a top-level `warnings` array” and “report mode does not repeat the
warning on stderr.”** The defect class is warning routing that bypasses the
output mode's delivery boundary.

Darkmatter-cli's
[`run_triggers`](../../cli/src/commands/schema/triggers.rs:67) deliberately
prints findings only in the stdout trace. Its new test explicitly asserts
the absence of a stderr warning. Darkmatter-cli's
[`clean`](../../cli/src/commands/clean.rs:269) unconditionally prints retained
findings to stderr on a schema failure, before the caller can emit a JSON
report. These are documented Phase 2 implementation choices that narrowed
the promised scope; they require repair, not approval.

Use the same incident repository and control document as R1-F1.
`md schema triggers prompt.md` exits 0, puts “ignoring invalid schema
trigger” on stdout, and leaves stderr empty. Then change the document's
frontmatter to `$schema: ./missing.yaml` and run
`md clean --json prompt.md`: exit 1, empty stdout, a human-readable trigger
warning on stderr, and no version-1 warning collection to read.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1 | darkmatter-cli trigger inspection | Incident trigger; ordinary `schema triggers` | Warning only in stdout trace | Warning on stderr; ordinary inspection data on stdout |
| R1-F2.2 | darkmatter-cli clean schema-error delivery | Incident trigger plus missing authored schema; `clean --json` | No JSON envelope; warning on stderr | Retained warning in version-1 `warnings`; no human-readable trigger-warning row on stderr; exit 1 preserved |
| R1-F2.3 | darkmatter-cli ordinary clean | Control and missing authored schema | One stderr warning; clean | Preserve |
| R1-F2.4 | darkmatter-cli successful clean report | Control; `clean --json` | Version 1, top-level warning, no stderr warning; clean | Preserve |
| R1-F2.5 | darkmatter-cli schema validation reports | Control and schema failure; JSON, multiple inputs, assignments | Warning in existing collection on first discovering input; clean | Preserve |
| R1-F2.6 | darkmatter-cli ordinary schema validation | Control; multiple files and assignments | Warning once on stderr; clean | Preserve |
| R1-F2.7 | darkmatter-cli ordinary/JSON composition | Control, authored-schema failure, valid-trigger rejection, explicit denial | Correct warning stream and severity; clean where a report reaches delivery | Preserve; R1-F1 covers absent delivery |
| R1-F2.8 | Claudine CLI | Compose, inline-compose, loop, sequence, rejected prompt | Warning once on stderr; clean | Preserve |
| R1-F2.9 | DMLS | Startup and refresh with closed trigger files | Warning diagnostics through the protocol, no human output on protocol stdout; clean | Preserve |

**Smallest repair:** move trigger-inspection warning rows to the shared stderr
renderer without also repeating the full warning on stdout. For failed
`clean --json`, emit retained findings in the existing version-1 envelope
before returning the original failure. Preserve its nonzero exit status and
existing schema-error presentation; no new report mode or version is needed.
Update the inspection test that currently asserts the wrong stream, add the
clean JSON failure regression, and correct documentation that records these
exceptions.

### R1-F3 (high): warning projections discard available source information

**Authority: spec, “Visible, accurate warnings”: “Retain the underlying typed
cause and any available error position and related payload/import source,”
and “If the cause points into a payload or import, preserve that location
separately and include its identity in the explanation.” The editor contract
requires available cause locations as related information.** The defect
class is loss of source provenance while turning a discovery failure into
a warning or projecting that warning into a host report.

Darkmatter's
[`ComposeWarning::from_invalid_trigger`](../../lib/src/markdown/compose/context/report.rs:739)
sets `diagnostic` and `line_number` to `None` even when the discovery finding
has an exact YAML error position. Its
[`warning_diagnostic`](../../lib/src/markdown/compose/report_diagnostics.rs:51)
therefore emits `source: null`. Separately,
[`related_path`](../../lib/src/markdown/schemas/triggers/finding.rs:180)
omits `AmbiguousReferenced`, despite that error carrying the payload path.
The payload
[`resolve_file_payload`](../../lib/src/markdown/schemas/triggers/assemble.rs:318)
also loses the resolved payload identity on merge rejection, and on compile
rejection when an authored schema reference follows another schema file.
DMLS faithfully projects those missing values into missing related links.

For the primary position reproduction, use:

```yaml
kind: schema-trigger
match: [unclosed
```

The public discovery result has `position() == Some((3, 1))`. Yet
`md compose --output report-json prompt.md` emits the corresponding warning
with `source: null`. The source field can carry the real trigger file and
the available line/column; no synthetic exact position is needed.

For payload reproductions, keep a valid envelope and change one payload:

```yaml
kind: schema-trigger
match:
  title: string(required)
$schema: ./payload.yaml
```

With payload `42`, discovery returns `AmbiguousReferenced { path: ... }`
but `cause_path()` is `None`, so DMLS publishes no related payload location.
With a root-union payload, the warning lacks both a related link and the
payload's identity in its explanation. Following an alias schema to raw JSON
Schema with `"type": 12` has the same identity loss on compilation.

The sweep checked library findings, compose projections, JSON delivery, and
actual DMLS warning publications. Payload failures were exercised through the
public discovery API and an L1 DMLS session using the existing test fixture.

| Instance | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | darkmatter compose warning/shared-diagnostic projection | Malformed claimed envelope with known `(3, 1)` position | Position lost; JSON `source: null` | Real trigger file and known position retained |
| R1-F3.2 | darkmatter finding and DMLS related-location projection | Referenced YAML payload containing `42` | Typed cause has payload path; finding and editor related location omit it | Preserve payload path separately and publish related location |
| R1-F3.3 | darkmatter payload merge rejection, finding, and DMLS projection | Referenced root union, `$schema: [{left: string}, {right: number}]` | No related payload identity; explanation names only the trigger | Retain known resolved payload identity in finding, explanation, and editor related information |
| R1-F3.4 | darkmatter payload compilation rejection, finding, and DMLS projection | Trigger → YAML `$schema` alias → JSON object with property type `12` | Compile warning omits both payload identities and related information | Preserve the actual schema source through the reference chain |
| R1-F3.5 | darkmatter discovery position and DMLS primary location | Same malformed envelope | Finding keeps `(3, 1)`; editor uses the known line on the trigger; clean | Preserve |
| R1-F3.6 | darkmatter finding and DMLS projection | Malformed YAML payload | Related payload path present; primary trigger location stays file-level; clean | Preserve without moving payload coordinates onto the trigger |
| R1-F3.7 | darkmatter finding and DMLS projection | Invalid SimplifiedSchema constraint in a payload | Related payload path present; clean | Preserve |
| R1-F3.8 | darkmatter finding and DMLS projection | Direct raw JSON payload compilation failure | Related JSON path present; clean | Preserve when routed through an alias too |
| R1-F3.9 | darkmatter finding and DMLS projection | Malformed imported type file | Related import path present; clean | Preserve |
| R1-F3.10 | darkmatter finding and DMLS projection | Unknown named type imported from a valid type file | Related payload containing the bad reference; namespace also named in message; clean | Preserve |
| R1-F3.11 | darkmatter finding and DMLS projection | Missing referenced payload or import | No exact cause-file position; no invented local related location; clean | Preserve the unresolved reference and lookup observations |
| R1-F3.12 | darkmatter finding and DMLS projection | Unsupported envelope keys | Trigger file-level fallback; no invented exact position; clean | Preserve |
| R1-F3.13 | darkmatter finding's other explicit path-bearing causes | `Io`, `ReferencedSchema`, `SchemaDocument`, `TriggerPayloadCycle`, direct `TriggerPayloadCompile` | Explicit match arms preserve the cause path when distinct from the trigger; clean by construction, with parser/import/compile cases also reproduced above | Preserve; add the omitted path-bearing cause and retain merge/alias provenance |
| R1-F3.14 | darkmatter-cli validation and clean warning rows | Malformed envelope | Existing rows retain trigger path and line/column in the message; clean for their established row shape | Preserve; this finding does not require a new report format |
| R1-F3.15 | Embedded finding and compose projections | Bundle/document source | Library invariant forbids a host `cause_path`; compose has no host path; clean | Keep virtual identity and source-free location; never create a local link or watch |

**Smallest repair:** carry known positions into the existing shared diagnostic
record and preserve resolved payload provenance before merge/compile failures
discard the successful resolution result. Include `AmbiguousReferenced` in
typed cause-path extraction. Use known paths, not the first arbitrary read
observation, and retain file-level/source-free fallbacks where exact positions
are unavailable. Extend the existing discovery, report-JSON, and DMLS tests
with these cases; no new parser, escaping layer, or report version is needed.

## Verification and coverage

Review execution used nextest through the package-area recipes:

- Darkmatter `just test invalid_trigger`: 34 passed.
- Claudine `just test invalid_trigger`: 13 passed.
- Darkmatter `just test trigger_`: 328 passed, including discovery recovery,
  input matrices, embedded triggers, editor refresh, and consuming CLI paths.
- Darkmatter `just test shipped_schema`: 14 passed, including passive corpus
  parsing and loadable shipped-schema resolution.
- `just check-tier-coverage darkmatter` and `claudine`: zero stranded tests.
- Additional isolated CLI and public-library reproductions established the
  results above. Temporary L1 DMLS session probes captured the scalar, union,
  malformed payload/import, constraint, direct/aliased compile, missing file,
  malformed envelope, and unknown-key diagnostics. All temporary source changes
  were removed after recording the results.

These filtered runs overlap; their counts are not a unique-test total. No
production code was changed for the review. Lint and full-area reruns were not
needed to create the review artifact. No terminal or browser window was opened
or focused. Cross-OS evidence remains CI's responsibility for this review.

| Spec requirement | Strongest relevant verification inspected/run | Assessment |
| --- | --- | --- |
| 1: incident recovery | L1 normal `md` and fake-provider Claudine CLI runs | Passing |
| 2: valid neighbors and ordering | L1 public discovery/validation and CLI tests | Passing |
| 3: invalid envelopes, payloads, imports, compile checks; strict passive parsing | L1 public API matrices and payload tests | Recovery passes; source preservation gap is R1-F3 |
| 4: all-invalid, matcher-only, ignored YAML, shadowing, infrastructure errors | L1 public API and embedded-bundle tests | Passing |
| 5: deduplication, repeated invocations, repairs/creation/deletion, distinct identities | L1 public API, CLI, and DMLS session tests | Passing in covered routes; early delivery is R1-F1 |
| 6: consuming hosts, assignments, reports, retained failure warnings | L1 CLI/fake-provider runs and editor protocol sessions | R1-F1 and R1-F2 leave uncovered or incorrectly asserted paths |
| 7: explicit-schema/validation enforcement and shipped corpus | L1 public API, CLI, passive corpus | Passing |
| 8: current editor registry, completion/hover, shared warnings, strict authoring | L1 editor protocol sessions and registry tests | State transitions pass; related-source gaps are R1-F3 |
| 9: explicit denial before effects and discovery opt-out | L1 effect counter/approval-handler tests and CLI reports | Passing |
| 10: clean, JSON warning collection, opt-out, editor formatting | L1 CLI and editor formatting tests | Success paths pass; failed clean JSON delivery is R1-F2 |

L1 is appropriate here: the promises concern passive schema results, process
streams, JSON, editor protocol messages, and prevention of effects. They do not
depend on terminal-emulator encoding, keyboard input, glyph geometry, or host
focus. There is no missing L2/L3 requirement.

Test modules are declared by their consolidated L1 targets, and the recipes
enable the CLI fixture feature needed by the fake provider. The observed
nextest selections prove these tests run; the tier checks found no stranded
coverage.

### Input robustness

The recovery boundary reuses the existing strict envelope/payload parsers;
it does not add a permissive parser. The default for an invalid classified
trigger is one finding and exclusion of its whole payload. Infrastructure
errors still fail discovery.

The public `trigger_input_matrix` suite exercises field shapes for `kind`,
`match`, payload, condition groups, threshold count/candidates, path patterns,
and property conditions. Its exceptions are explicit: absent/non-trigger
`kind` is ignored, absent payload is matcher-only, an explicit empty payload
is preserved, and omitted optional condition nodes remove that predicate.
Null, wrong whole-field types, mixed/all-invalid collection elements, and
invalid empty values take the specified invalid-trigger path rather than
silently becoming empty collections. The generated-cell guard checks that
the table and runnable cases agree.

The shared YAML loader is also exercised for duplicate keys and trailing
invalid content; embedded block-YAML and JSON-syntax documents use the same
classifier with their established malformed-claim rules. These rows passed.
The raw JSON compile control and its one-change alias variant exposed the
provenance gap in R1-F3, rather than a validation/recovery gap. No finding here
claims that unchanged field validators need a second per-format matrix.

## Observations

None. The implementation log's pre-existing Windows shell-quoting failures
are outside this fix and are not used to block readiness or request human
approval.
