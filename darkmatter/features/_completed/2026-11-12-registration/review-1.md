---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F1 (high): Foreign composition discards host-event schema errors'
  - 'R1-F2 (high): Sequence-only Claudine documents miss the main host schema'
  - 'R1-F3 (high): Claudine leaves configured URL transclusion disabled'
  - 'R1-F4 (high): Per-run host approvals bypass invalid permissions'
  - 'R1-F5 (high): DMLS ignores template shell approvals'
  - 'R1-F6 (high): Union validation skips conflicting default candidates'
  - 'R1-F7 (high): Recorded Windows source-scan timings fail the L1 speed gate'
  - 'R1-F8 (medium): Eager default failures lose their schema-default origin'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-14T03:24:17-08:00
spec: 2026-11-12-registration/spec.md
implemented: true
description: 'A **feature** review of `2026-11-12-registration/spec.md`'
feature: 2026-11-12-registration/review-1.md
log: /Volumes/coding/personal/rusty-biscuit/darkmatter/features/2026-11-12-registration/log.md
implemented_by: claude/opus
next: 2026-11-12-registration/review-2.md
---

# Registration review, iteration 1

The feature is not production ready. Eight findings remain; all have a recommended repair within the specification and none needs an author decision before repair. There are no earlier review findings to carry forward.

The strongest parts are the independent configuration settings, committed/working-copy permission intersection, locked YAML edits, registration publication, and editor invalidation. The findings below concern observable gaps between those parts, plus a recorded failing speed gate.

## Findings

### R1-F1 (high): Foreign composition discards host-event schema errors

**Authority — spec, section 6:** “`md compose` validates a Claudine document against Claudine's schemas and composes its Markdown body.” Withholding host actions from execution does not authorize skipping their schema validation.

**Defect class:** deferring execution also suppresses final validation of values that no later stage validates.

In Darkmatter, [the schema problem filter](../../lib/src/markdown/compose/schema_validation.rs:675) discards every problem under an excluded key. The CLI [excludes every event of the selected host](../../cli/src/commands/compose/lifecycle.rs:121), including events it will never run. The ordinary relay-host fixture, changed only to `start: 5`, gives exit 1 from `md schema validate` but exit 0 and a successful document from `md compose`. The same defect reaches the other foreign events and Claudine's `loop` setting.

The sweep used the existing registered relay fixture with `sequence: []`, changed one event at a time, and ran both CLI commands with JSON output. A second public-API reproduction used Claudine's actual published profile and the CLI's selected-host/excluded-key options for `loop: [1, 2]`.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Foreign `start` | `5` | Validate rejects; compose succeeds | Compose rejects |
| R1-F1.2 | Foreign `blocked` | `5` | Validate rejects; compose succeeds | Compose rejects |
| R1-F1.3 | Foreign `success` | `5` | Validate rejects; compose succeeds | Compose rejects |
| R1-F1.4 | Foreign `failure` | `5` | Validate rejects; compose succeeds | Compose rejects |
| R1-F1.5 | Claudine `loop` | `[1, 2]`, with `agent: claude` selecting the main schema | Passive validation rejects; selected-host composition succeeds | Composition rejects |
| R1-F1.6 | Shared `initialize` | `5` | Both CLI commands reject | Reject; clean because md parses this stack |
| R1-F1.7 | Shared `cancelled` | `5` | Both reject | Reject; clean |
| R1-F1.8 | Shared `finalize` | `5` | Both reject | Reject; clean |
| R1-F1.9 | Renamed-away `composed` | `5` | Both reject | Reject; clean |
| R1-F1.10 | Renamed-away `failed` | `5` | Both reject | Reject; clean |
| R1-F1.11 | Ordinary `timeout` | Numeric scalar `5` | Both accept the permitted string conversion | Accept; clean; the existing array-valued timeout test also rejects |

**Recommended repair:** keep host event values out of interpolation and execution, but retain their structural schema problems in the final foreign-document verdict. Do not globally remove the caller-owned validation escape hatch that Claudine uses for its own runtime. Add CLI regressions for the foreign-only events and the published Claudine loop case.

### R1-F2 (high): Sequence-only Claudine documents miss the main host schema

**Authority — spec, section 6:** Claudine's identifying keys include `sequence`; an identified document receives the host's schemas, and the host schema must reject the md-only `composed` spelling. The spec only removes shared-event-only identification, not sequence documents' host contract.

**Defect class:** host identification and the selectors that supply that host's main schema disagree.

Claudine's [main schema trigger](../../../claudine/schemas/claudine.yaml:21) tests the eight non-sequence identifying keys. Its [sequence trigger](../../../claudine/schemas/sequence.yaml:17) supplies only sequence-specific fields. A document with `sequence: []` and `timeout: [1, 2]` is identified as Claudine but validates and composes successfully. Replacing `sequence: []` with `agent: claude` correctly rejects the same timeout. Likewise, a sequence-only document accepts `composed: [stop]`, while an agent-identified document correctly rejects it.

The sweep published Claudine through its existing setup fixture, loaded that installation through Darkmatter's public registration API, and changed only the identifying key. Every row retained the invalid `timeout: [1, 2]`. Both passive schema validation and composition were checked.

| Instance | Identification/selection site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | `sequence` → sequence trigger, missing main trigger | `sequence: []` | Host selected; timeout accepted by both readers | Host selected; timeout rejected |
| R1-F2.2 | `start` → main trigger | `start: []` | Both reject timeout | Reject; clean |
| R1-F2.3 | `blocked` → main trigger | `blocked: []` | Both reject | Reject; clean |
| R1-F2.4 | `success` → main trigger | `success: []` | Both reject | Reject; clean |
| R1-F2.5 | `failure` → main trigger | `failure: []` | Both reject | Reject; clean |
| R1-F2.6 | `loop` → main trigger | `loop: {max: 1}` | Both reject | Reject; clean |
| R1-F2.7 | `prompt` → main trigger | `prompt: hi` | Both reject | Reject; clean |
| R1-F2.8 | `agent` → main trigger | `agent: claude` | Both reject | Reject; clean |
| R1-F2.9 | `model` → main trigger | `model: sonnet` | Both reject | Reject; clean |
| R1-F2.10 | Sequence-selected renamed-event validation | `sequence: []`, `composed: [stop]` | Composition accepts | Reject the md spelling |
| R1-F2.11 | Agent-selected renamed-event validation | `agent: claude`, `composed: [stop]` | Composition rejects | Reject; clean |
| R1-F2.12 | Shared-event-only host selection | No host-specific key | Existing CLI/editor tests select md | Select md; clean |

**Recommended repair:** add the same non-null `sequence` selectors to the main Claudine trigger. This keeps the shared-event narrowing and supplies the common contract without duplicating its payload in the sequence trigger. Regenerate the shipped artifacts and test the published profile through the passive reader as well as composition.

### R1-F3 (high): Claudine leaves configured URL transclusion disabled

**Authority — spec, section 2.3:** `network` governs composition reads, explicitly including `::file` and `::code`. The permission-enforcement criterion also requires the same decisions in md and Claudine.

**Defect class:** a consumer projects permission consent but never enables the composition operation that uses it.

Claudine's [canonical composition options](../../../claudine/lib/src/composition/prepare.rs:405) never enable remote transclusion. Darkmatter separately gates [directive URL targets](../../lib/src/markdown/compose/transclusion/engine.rs:822) and [prologue/epilogue URL targets](../../lib/src/markdown/compose/transclusion/engine.rs:992) on that option. Consequently, `network: allow` enables URL expression reads but leaves these composition reads disabled in Claudine.

The reproduction changed the existing network fixture to omit md's extra enable switch, then exercised Claudine's public direct-preparation API using the existing shared permission fixture and loopback responder. No agent or external service was launched.

| Instance | Site | Shape tested | Observed | Expected under effective `network: allow` |
|---|---|---|---|---|
| R1-F3.1 | Claudine body preparation → `::file` | Loopback Markdown URL | Remote-transclusion-disabled error | Include the remote body |
| R1-F3.2 | Claudine body preparation → `::code` | Same URL | Disabled error | Include the remote text as code |
| R1-F3.3 | Claudine preparation → `prologue` | Same URL in frontmatter | Disabled error | Include the remote prologue |
| R1-F3.4 | Claudine preparation → `epilogue` | Same URL in frontmatter | Disabled error | Include the remote epilogue |
| R1-F3.5 | Read-side expression | `length(load_data(url, 'string[]'))` | Returns `2` without the extra switch | Success; clean |
| R1-F3.6 | md/public composition with remote transclusion enabled | Same remote file and network policy | Existing network tests succeed | Success; clean |
| R1-F3.7 | Inline, loop, sequence, and preflight preparation options | The common options builder | All inherit the unset switch; no separate Claudine enable call exists | Use the same configured operation policy |

**Recommended repair:** enable remote transclusion in Claudine's composition preparation, leaving actual consent to the captured network policy. Preserve the library's ability to disable an operation deliberately. Cover all four targets through Claudine's public preparation path; keep denied-host and redirect tests.

### R1-F4 (high): Per-run host approvals bypass invalid permissions

**Authority — spec, section 1:** “A file that fails validation is an error for any command that reads the failing property, never a silent fall back to the default.” Section 2.8 also requires a command needing an invalid referenced block to stop. The per-run host exception is specified for valid `network: deny`, not malformed configuration.

**Defect class:** operation-specific approval is checked before the captured configuration error, turning invalid policy into a permissive result.

Darkmatter's [remote policy gate](../../lib/src/markdown/compose/remote_fetch.rs:566) returns success for an allowed host before inspecting the invalid-configuration state. The [schema-slot URL gate](../../lib/src/markdown/schemas/slots/grants/gate.rs:130) makes the same choice. The loader correctly rejects the field; its error is lost at these later decisions.

A shipped-CLI reproduction used an isolated home containing `permissions: {network: null}`, a loopback `::file` URL, and `md compose doc.md --allow host=127.0.0.1 --output json`. It exited 0, fetched the URL once, returned the child body, and emitted no diagnostics. The valid `network: deny` control also succeeded, as it should.

The default outcome for invalid network shapes must be a located configuration error and no fetch, including when a run names an allowed host. Exceptions are absent → schema default `deny`, and valid `allow`/`deny` → the specified consent rules.

| Instance | Site | Shape tested | Observed with an exact per-run host | Expected |
|---|---|---|---|---|
| R1-F4.1 | Remote `::file` gate | `null`, `123`, `[]`, `""`, `sometimes` | Every case succeeds | Configuration error; no request |
| R1-F4.2 | Remote `::code` gate | Same five field shapes | Every case succeeds | Error; no request |
| R1-F4.3 | Remote expression gate | Same five field shapes | Every case succeeds | Error; no request |
| R1-F4.4 | Schema-slot URL gate | Same five field shapes | Every case succeeds | Error; no request |
| R1-F4.5 | Remote gates, valid controls | `allow`, `deny` | All four surfaces succeed for the explicitly allowed host | Success; clean |
| R1-F4.6 | Network field validator | Absent; explicit null; empty string; wrong scalar/list; unknown enum | Absent defaults; invalid shapes are located errors in existing public-loader tests | Same; clean loader |
| R1-F4.7 | Local schema-slot gate | Invalid captured permissions | Refuses before consulting current-run grants, by its explicit first branch | Refuse; clean |
| R1-F4.8 | Claudine shell permission reader | Invalid captured permissions | Existing public parity test returns configuration error | Refuse; clean |
| R1-F4.9 | YAML document loader | Duplicate keys, malformed/trailing YAML, second document | Existing public-loader tests reject before policy construction | Refuse; clean |
| R1-F4.10 | Whole permissions validator → URL gate | `permissions: null` | Shipped CLI fetches once, exits 0, no diagnostic | Configuration error; no request |
| R1-F4.11 | File-read validator → URL gate | `file-read: [123]` | Same silent success | Error; no request |
| R1-F4.12 | Shell-rule validator → URL gate | `shell-commands: [frobnicate echo]` | Same silent success | Error; no request |
| R1-F4.13 | Permission reference loader → URL gate | `permissions: ./missing.yaml` | Same silent success despite missing target | Reference error naming both files; no request |
| R1-F4.14 | Omitted permissions control | `{}` | Shipped CLI succeeds for the explicitly allowed host | Success; clean |

These are scalar-field shapes; collection-element cases do not apply to `network`. File-read and shell-command lists use their own validators and are covered separately by `config_fields` and the permission suites.

**Recommended repair:** return the captured configuration error before considering host grants in both URL gates. Keep exact-host exceptions for valid deny policies. Add one shared public-result matrix and a CLI wiring case that asserts the configuration location and zero requests.

### R1-F5 (high): DMLS ignores template shell approvals

**Authority — spec, permission-enforcement criterion:** “shell chains/templates take the same decision in `md`, Claudine, and DMLS.”

**Defect class:** the editor projects only a subset of the shared permission rule grammar.

DMLS's [shell verdict](../../dmls/src/overlay/shell.rs:85) consults the exact/prefix whitelist helper but never template coverage. For `template echo {{ string }}`, a document with a declared string `title` and `::shell echo {{ title }}` is pre-approved by md, while DMLS reports unknown. This is a routine documented rule, not an unusual payload.

The reproduction changed only the rules and command in the existing DMLS policy fixture. A CLI control using the same rule and a string-valued title exited 0 and printed `hello` without prompting.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F5.1 | DMLS positive template projection | Template allow, `echo {{ title }}` | Unknown | Approved when the declared shape is covered |
| R1-F5.2 | DMLS positive template projection | Same template allow, literal `echo hi` | Unknown | Approval required; clean, confirmed by md rejecting the literal command |
| R1-F5.3 | DMLS exact rule | `exact echo hi` | Approved | Approved; clean |
| R1-F5.4 | DMLS prefix rule | `prefix echo` | Approved | Approved; clean |
| R1-F5.5 | DMLS template exclusion | `prefix echo` plus `!template echo {{ string }}`, literal command | Unknown | Approval required; clean |
| R1-F5.6 | md template execution | String title, same template rule | Exit 0, `hello` | Approved; clean |
| R1-F5.7 | md/Claudine template and chain enforcement | Shared permission fixture | Existing public parity tests agree | Agree; clean |

**Recommended repair:** use the shared template parser, type projection, and coverage checks in passive editor classification. Preserve the built-in blacklist and exclusion checks. Extend the shared parity fixture to assert the actual editor decision rather than recording template rules as unsupported.

### R1-F6 (high): Union validation skips conflicting default candidates

**Authority — spec, section 3.2:** “Accept only an unambiguous candidate (identical candidates count as one); conflicting successful candidates produce a located ambiguity diagnostic instead of choosing whichever branch runs first.”

**Defect class:** accepting the original value early bypasses the required comparison of complete alternatives.

For this schema and input `{b: y}`, one successful alternative is `{b: y}` and another is `{b: y, a: 1}`:

```yaml
$schema:
  - a: number(required; default(1))
  - b: string(required; default(z))
```

Simplified Schema's [strict and soft entry points](../../../simplified-schema/lib/src/prepared.rs:284), the [shared default preparation entry](../../../simplified-schema/lib/src/defaults.rs:95), and the [nested-alternative walk](../../../simplified-schema/lib/src/defaults.rs:320) all have an already-valid shortcut. Both public modes accept this input without defaults or an ambiguity error. The existing `a_value_one_arm_accepts_as_written_needs_no_default` test pins that departure.

The sweep used the existing required-default fixture, changing only the input or nesting the same alternatives beneath `shape`.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F6.1 | Strict whole-document alternatives | `{b: y}` | Success, no defaults | Ambiguity |
| R1-F6.2 | Soft whole-document alternatives | Same input | Success, unchanged value | Ambiguity |
| R1-F6.3 | Strict nested alternatives | `{shape: {b: y}}` | Success, no defaults | Ambiguity at `shape` |
| R1-F6.4 | Soft nested alternatives | Same nested input | Success | Ambiguity at `shape` |
| R1-F6.5 | Whole-document settlement | Empty input, two differing defaulted candidates | Located ambiguity in both modes | Ambiguity; clean |
| R1-F6.6 | Nested settlement | Empty `shape`, two differing candidates | Located ambiguity in both modes | Ambiguity; clean |
| R1-F6.7 | Equal candidates and literal-tag selection | Existing public required-default controls | Equal candidates settle once; a distinguishing literal selects its branch | Same; clean |
| R1-F6.8 | Darkmatter effective-schema default projection | Same root/nested alternatives in a referenced schema, `{b: y}` or `{shape: {b: y}}` | Shipped `md schema validate` exits 0 and reports valid for both | Located ambiguity |

**Recommended repair:** retain fast acceptance where no alternative can add defaults, but compare complete candidates when alternatives can supply them. Include the unchanged candidate in the comparison. Update the test and documentation that describe the shortcut as intended behavior. No author ruling is needed to implement the explicit contract.

### R1-F7 (high): Recorded Windows source-scan timings fail the L1 speed gate

**Authority — review readiness rule:** an L1 test at 10 seconds or longer fails even on a loaded host, regardless of its isolated rerun or whether this feature changed it.

**Defect class:** a whole-repository source scan exceeds the L1 budget, with per-test thread fan-out competing against nextest's parallel execution.

Darkmatter's [current-root migration guard](../../lib/tests/l1/current_root_migration_guard.rs:101) reads the repository corpus and starts up to 16 threads inside one test. The implementation record reports it at over 10 seconds in Phase 2, 11.7 seconds in Phase 3, 10.7 seconds in Phase 4, 12.6 seconds in Phase 5, and 12.3 seconds in Phase 6. Phase 3's 7.4-second narrow rerun does not clear the 11.7-second package result and itself remains above the isolated limit. Calling the test pre-existing does not clear it either.

This finding uses the implementer's recorded Windows executions; no new Windows run is claimed. I read the scan and its sibling control, and ran both in the targeted macOS package run.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F7.1 | `current_root_migration_guard::the_removed_nesting_appears_only_where_the_allowlist_expects` | Recorded full Windows repository/package runs | Repeated 10.7–12.6 seconds; one narrow rerun 7.4 seconds | Within the applicable package-run budget and at most 5 seconds alone |
| R1-F7.2 | Same production scan on the review host | Targeted macOS package run | 0.245 seconds | Under 5 seconds; clean on this host, does not invalidate Windows evidence |
| R1-F7.3 | Same module's bounded fixture control | Targeted macOS package run | 0.073 seconds | Under 5 seconds; clean |
| R1-F7.4 | Related glob ownership scan | Targeted macOS package run | 1.455 seconds; implementation's loaded 5.23-second run was followed by 1.37 seconds alone | Cleared under the stated loaded-host rule |

**Recommended repair:** reduce the guard's repository-read/decoding work and remove the nested thread fan-out while preserving what the guard checks. Measure it alone and in the Windows package run. Do not hide it behind a different tier, ignore, timeout increase, or dropped coverage. The incorrect passing-gate statements in the implementation log have been corrected separately; that record correction is not an additional finding.

### R1-F8 (medium): Eager default failures lose their schema-default origin

**Authority — spec, section 3.3:** “attribute an inserted value's error to its schema default rather than inventing an authored position.” Section 3.2 also requires invalid defaults to identify their schema location.

**Defect class:** validating a completed value loses the provenance of values inserted from schema defaults.

For an absent `doc` with `file(required; eager; default(./note.md))`, where `note.md` does not exist, Simplified Schema reports an ordinary file-reference problem at instance `/doc` and schema `/properties/doc/format`, rather than identifying `/properties/doc/default`. Darkmatter similarly returns `dm.schema.invalid_file_reference` with an unlocated document source and no indication that the value came from a schema default. This matters especially for a default imported from a separate schema: the author needs to repair the default, not look for a nonexistent frontmatter value.

The reproduction changed the existing eager-file fixture to omit the field and add the default, then used both shared-library modes and the CLI's validation and composition commands. No nonexistent authored line was fabricated; the missing part is attribution to the default.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F8.1 | Simplified Schema strict validation | Absent field, missing eager default target | Ordinary `/doc` file error, schema pointer ends in `/format` | Retain the default's schema origin |
| R1-F8.2 | Simplified Schema soft validation | Same input | Same ordinary file error | Retain default origin |
| R1-F8.3 | Darkmatter passive schema validation | Same Markdown fixture | Exit 1; file problem under `/doc` | Identify the schema default |
| R1-F8.4 | Darkmatter composition | Same fixture | Exit 1; generic file diagnostic, null source line/column, no default attribution | Identify the schema default |
| R1-F8.5 | Eager default whose target exists | Same schema, create `note.md` | Success and reported applied default | Success; clean |
| R1-F8.6 | Lazy file default | Same absent field, no `eager`, target absent or present | Success in both cases | Success; clean |
| R1-F8.7 | Structurally invalid numeric default | Existing composition-default regression | Error names the default and `/properties/n/default` | Correct origin; clean |

**Recommended repair:** retain default-source metadata through completed-value validation and reattribute problems at inserted paths to the responsible default, including in the CLI diagnostic projection. Keep ordinary authored eager-file checks and avoid manufacturing an authored position.

## Input robustness and verification levels

The new loaders are configuration readers, so their public-result shape tests were reviewed. Their intended default is rejection of invalid documents/fields with a location, without authorizing an operation. Documented exceptions are missing configuration/settings using schema defaults, empty permission lists replacing defaults, and a bad registration entry being omitted with a diagnostic while independent permissions remain usable.

- The admitted YAML loader is shared by Darkmatter configuration, referenced blocks, host manifests, and Claudine YAML configuration. Existing tests cover duplicate keys, malformed content, multiple documents, YAML null syntax, empty documents, and supported markers/line endings. Referenced blocks add missing-target, nonmapping, missing-key, and forbidden-chain cases.
- `config_fields` covers absence, null, whole-field type errors, mixed/all-invalid elements, and explicit empty lists for the permission and registration validators. The manifest tests cover required fields, null/type errors, invalid list elements, closed fields, and member refusal before profile publication. The later network decision bypass is R1-F4, rather than a missing parser rejection.
- Migration receipts use a separate strict JSON reader with document and field-shape tests. The checker-input JSON uses closed typed deserialization and rejects malformed or empty input lists. Claudine's temporary JSON5 conversion has its own conversion/failure tests.
- The smell sweep found deliberate filtered iteration only after validation, or registration entry recovery with diagnostics. Retained nested serde defaults in Claudine sit behind schema validation. No additional silent wrong-type-to-empty policy finding was established.

| User-facing requirement | Appropriate level | Evidence reviewed/run |
|---|---|---|
| Configuration shape/defaults, permission intersection, references, persistence, migration | L1 public API plus CLI wiring | Existing named suites and targeted package runs; R1-F4 identifies the uncovered downstream decision |
| Host identification, layering, publication, foreign composition | L1 public API plus CLI | Registered-host and published Claudine fixtures; R1-F1/R1-F2/R1-F3 identify gaps |
| DMLS configuration completion/hover, host diagnostics, atomic-replacement/deletion invalidation | L1 real LSP protocol session | Existing editor suites, including `host_registration_watches`; template classification gap is R1-F5 |
| Required defaults, conversion, strict/soft behavior, editor narrowing | L1 shared-library and CLI/LSP paths | Required-default and composition suites; R1-F6/R1-F8 identify gaps |
| Approval-environment workflow and permission comparison | L1 comparison and mocked workflow/API contracts | Existing Rust tests; 32 Python helper tests passed |
| New keypress, terminal encoder, or real-emulator styling behavior | None introduced by this feature | No L2/L3 mismatch identified; existing interactive components are reused |

Review execution results:

- Darkmatter canonical targeted run: **322 passed**, none at or above five seconds. An earlier narrower run passed 286, also with none slow.
- Simplified Schema canonical run: **28 required-default tests passed**, plus its **1,822-test minimal-feature suite**, none slow.
- Claudine canonical targeted run: **31 passed**, none slow; this count includes one temporary review probe of the foreign loop case.
- CI helper tests: **32 passed** with `PYTHONPATH=scripts/ci`.
- Temporary probes exercised the findings through existing declared L1 targets and normal public APIs/CLI paths, then were removed. They checked current behavior; their successful assertions do not mean the spec is satisfied. No production code was repaired during review.
- Cross-OS proof was not rerun for this review and is not a readiness requirement. The already-recorded Windows speed failure is a concrete finding, not a request for generic platform evidence.

## Implementation decisions (informational)

Two independent sub-agents read the log and spec in parallel. They found 56 `decisions_for_review` entries, no separate implementation record, and no broad autonomy clause in the spec. The following accounts for every entry, in its original order. None needs a new human ruling. Choices that depart from the contract are ordinary repairs identified above; the rest are reasonable implementation choices or already decided by the author.

1. **Ownership imports:** remove Darkmatter re-exports of the moved schema facilities and retain only host-specific wrappers. Keep.
2. **Supplied schema sources:** retain sources on registries and include their identity in memoization; use a host-input object for kind selection. Keep.
3. **Prepared trigger definition:** retain its prepared record in Darkmatter's artifact while the source and semantics live in Simplified Schema. Keep; no behavior departure was established.
4. **Minimal build:** gate filesystem discovery, fail path matching closed without it, and retain caller-supplied-source APIs. Keep; the minimal suite passes.
5. **Already-valid alternatives:** skip default candidate comparison when an alternative accepts the original value. Repair under R1-F6.
6. **Default checking time:** reject an invalid default when a document needs it, rather than eagerly rejecting every schema at preparation. Keep.
7. **Default error variants:** reuse existing unprepared-schema and constraint diagnostics. Keep; R1-F8 concerns lost provenance after insertion, not the variant name.
8. **Default insertion order:** insert before conversion, mark default text as data, and retain first-pass interpolation before the schema stage. Keep.
9. **Default output:** emit the JSON defaults list only when nonempty and omit it from formatted reports. Keep; no additional display requirement is imposed.
10. **YAML edit limits:** refuse edits through anchors, aliases, or tags; use indexed append; keep existing list indentation. Keep.
11. **Writer behavior:** refuse missing parent directories/read-only destinations, preserve restrictive new-file modes, re-read under locks, report partial multi-file publication, and retain lock files. Keep; cross-file atomicity was not promised.
12. **Migration receipts:** write them inside the transaction before publication and constrain cleanup to known paths. Keep.
13. **Committed-tree API:** require portable slash-separated paths, return hexadecimal identities, reject unrepresentable names, and enumerate files/symlinks only. Keep.
14. **Claudine default status:** show a defaulted state, add bounded launch validation, and leave non-eager nested validation to completion. Keep the existing phase distinction.
15. **Collection default syntax:** accept strict JSON arrays/mappings inside defaults and require quoting for literal bracketed text. Keep; configuration defaults need these values.
16. **Parent defaults:** spell the complete permissions default in each parent and pin equality with child defaults. Keep; observable missing-setting behavior matches the required defaults without a second default engine.
17. **Permission reference restrictions:** reject search/vault patterns, lexical repository escapes and committed nonfiles; derive unknown-key checks from the schema; combine user and working-copy registrations. Keep.
18. **Captured configuration plumbing:** use a weak context extension, prefer explicit snapshots, and avoid config reads in local-only expression contexts. Keep.
19. **Fixture and notification defaults:** give hermetic fixtures `{}` user config and keep machine-facing Claudine streams free of missing-config notices. Keep the stream contract.
20. **Claudine schema details:** enumerate provider/event keys, retain guarded nested serde defaults, require Protect pattern fields, reject duplicate/conflicting aliases, and prepare this configuration schema once outside the shipped artifact. Keep.
21. **JSON conversion location:** recognize the `.claudine/config.yaml` path shape and fail on receipt-cleanup errors. Keep.
22. **Per-key writes:** re-read under locks, keep empty repository mappings, write null when clearing a defaulted override, manage setup's selected keys, and normalize quoting only on changed keys. Keep.
23. **No-home fallback:** preserve Claudine's existing relative fallback, resolving its write target against the request directory. Keep the established behavior.
24. **Broken permission references:** isolate their failure to permission consumers and use the referring scope's defaults for omitted target fields. Keep.
25. **Saved file approvals:** persist escaped home-relative exact files only; otherwise report not-saved and retain the explicit approval for the run; refuse concurrent-change saves. Keep the conservative persistence limit.
26. **Network operation boundaries:** retain deliberate library operation switches and separate lifecycle HTTP-write consent. Claudine's failure to enable configured transclusion is repaired under R1-F3; continuing past invalid policy is R1-F4.
27. **Migration outcomes:** fail malformed sources, preserve changed/unpublished sources with notices, avoid migrating into references, and retain the repository commit rule. Keep.
28. **Shell approval lifetime:** retain explicit approve-once decisions, separately check newly excluded filled commands, remove retired policy-root identity, and include rules in cache identity. Keep.
29. **Retired grant forms:** retain them only for migration; keep reading-context checks when saving and URL choices limited to this run. Keep.
30. **Consumer permissions:** allow uncaptured-policy library contexts that approve/save nothing and share invocation approvals; repair DMLS's missing template classification under R1-F5.
31. **Setup entry point:** use Claudine's existing first-run setup rather than resurrecting a removed command; preserve original policy on transfer failure; fill only missing top-level settings; keep explicit inline repair. Keep.
32. **Identification proof:** retain ordinary negative/optional-guard matching and presence-only required host keys, refusing anything the proof cannot establish. Keep; conservative refusal is expressly authorized.
33. **Legacy fallback:** suppress an already registered host and its neighboring function catalogs when it did not identify the document. Keep; otherwise registration could leak that host into unrelated documents.
34. **Trigger disable switch:** let the existing trigger-disable option cover registrations too, while supporting registration outside repositories. Keep.
35. **Layer identity and deduplication:** include manifest/revision meaning in identity, compare digests in merge memoization, and deduplicate resolved equivalent trigger content. Keep.
36. **Inspection passivity:** use non-migrating repository projections for schema inspection/cleaning; retain compose's writing-CLI migration. Keep.
37. **Foreign deferred keys:** defer host event values without inventing a runtime for host data. Keep the execution separation; repair the dropped validation problems under R1-F1.
38. **Foreign notice/function typing:** emit one root notice, declare unavailable foreign runtime values, and scope handler-free function typing to the composing thread. Keep; worker threads still execute no host handler.
39. **Configuration buffers:** allow an empty new editor buffer to complete as `{}`, treat authored `kind`/`$schema` as data, prefer user scope when paths coincide, and expose the simplified contract for editor features. Keep.
40. **Editor warning placement:** put loading failures on the file that caused them and selection failures on the document; use unlocated artifact warnings when no authored position is known. Keep.
41. **Registration positions:** add source coordinates to entries so a missing manifest points at its registration. Keep; this public addition is the recommended implementation of located diagnostics.
42. **Documents without frontmatter:** avoid blocking editor startup on host loading, use md until facts are available, and use md for YAML documents. Keep; there is no host-specific document data to analyze in those cases.
43. **Unknown-host/watching details:** limit warnings to known host events, include host revision in analysis identity, watch HEAD/branch refs, notify once per session, and use disk permissions for hover. Keep; unsaved-buffer permissions were optional in the plan.
44. **Immutable revision names:** use version plus a short content digest, verify all bytes before reuse, and publish a new suffixed directory when an existing one is damaged. Keep.
45. **Claudine identification syntax:** enumerate the nine keys across non-null types and publish the identifying trigger outside the runtime bundle. Keep.
46. **Sequence main-schema selection:** omit sequence from the main trigger after removing shared-only selectors. Repair under R1-F2.
47. **Setup registration scope:** register the local Claudine installation in user config, keep successful setup quiet, fail publication before writing config, and report a later registration failure. Keep.
48. **Launch refresh routes:** refresh the normal user-facing routes while excluding machine streams, completion, uninstall, and early help exits. Keep; these excluded exits do not run a document with stale host behavior.
49. **Published-host verification:** use the shared library to read the actual publication and independent protocol fixtures for DMLS. Keep; a new binary dependency is unnecessary for that boundary.
50. **Checker input inventory:** centralize it in JSON, include policy/checker sources, and omit lockfile/deep transitive changes. Keep the bounded conservative comparison; no specific missed widening was established.
51. **Environment protection bootstrap:** use a trusted inline API check when the base lacks the new helper and test agreement with the helper. Keep.
52. **Default widening comparison:** compare parent and child defaults for both scopes. Keep.
53. **Claudine policy comparison:** compare its repository permissions block even before a Darkmatter reference reaches it. Keep; the spec expressly lists that input.
54. **Trusted build caching:** build the checker without candidate-writable Rust caches. Keep.
55. **Push comparison:** check direct main pushes unless matching approved PR evidence is reused, accepting a possible second approval for a different tree. Keep.
56. **Eager defaults:** distinguish the structural, passive check of a default definition from normal eager validation of the completed value. Keeping the latter check is reasonable; its lost default provenance is repaired under R1-F8. This review does not request a new default-execution exemption.

Additional recorded choices and deferred work:

- Adding an `info` step to the unavailable-`seq` test is appropriate: md intentionally skips an unsupported `say` verb without evaluating its arguments. The added supported verb tests the specified unavailable-value behavior.
- Creating the live permission-widening environment and approving the bootstrap deployment remain the operational steps the spec explicitly assigns to the author. Their options and ownership were already decided; this review does not ask for that decision again or block code repairs on it.
- Unsaved configuration buffers in permission hover, moving Claudine's config schema into its prepared artifact, broader worker-thread function typing, and automatic old-revision cleanup are not required to repair a demonstrated wrong result here. Keep the bounded implementation.
- Cross-OS scheduling stays with the established CI process. Other worktrees' build-directory cleanup is outside this feature and no change there was made.
- Deferring the Windows slow guard to another fix is not accepted under this review's readiness rule; R1-F7 includes it in this repair cycle.

## Implementation record corrections

The implementation log was corrected in place on 2026-11-14. Its Windows gate summaries previously described assertion-successful runs as having no failures despite recording the guard at or above ten seconds. Its network and schema-slot summaries also claimed unconditional refusal of invalid permissions, which the exact-host reproductions disproved. The dated notes give the measurements, CLI inputs, observed results, and verification method. Historical measurements were retained. No spec requirements were rewritten, and the record correction is not a separate finding or human-review item.
