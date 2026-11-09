---
$schema: feature-review.yaml
ready: false
findings:
    - 'R3-F1 (medium): Schema-root discovery bypasses completion read retention'
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-07T02:23:44-08:00
spec: 2026-11-06-dotted-paths/spec.md
implemented: true
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-06-dotted-paths/log.md
implemented_by: claude/opus
description: 'A **feature** review of `2026-11-06-dotted-paths/spec.md`'
feature: 2026-11-06-dotted-paths/review-3.md
previous: 2026-11-06-dotted-paths/review-2.md
next: 2026-11-06-dotted-paths/review-4.md
---

# Review 3: Dotted Paths

The feature is not production ready. The previous review's specific catalog-read and optional-shell-test defects are fixed. One newly found sibling remains: discovering triggers reads YAML files in a schema directory without sharing those bytes with subsequent schema resolution. Completion returns correct candidates but reads those files twice, contrary to the spec's explicit read-once requirement. The repair can use the existing scoped reader and needs no human decision.

This review covers the spec, both earlier reviews, both implementation logs, the current implementation, and its tests. Neither earlier review contains a Human rulings section. Existing informational decisions are not reopened. Only this review and the requested review/spec frontmatter are changed.

## Previous Finding Verification

The table distinguishes the repaired instances from the newly found sibling. Required instances come from the repair's Finding Ledger in log.md, including its additional sweep rows. Evidence below includes fresh execution and source inspection; completion flags and log summaries are not used as proof.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Setter binding and completion use the wrong baseline selection; carried as custom catalog completion reading files more than once | Original R1-F1.1–R1-F1.24; expanded R1-F1.22.1–R1-F1.22.14 | **verified fixed** | Selected-baseline command/candidate tests pass; all expanded read-once instances are checked below. An independent file-open probe confirms the original catalog case now reads its root and repeated import once. | None of the enumerated instances. R3-F1 is a newly found sibling involving discovery, rather than a failed repair of the catalog loader. |
| R2-F1: Optional shell script tests pass without loading a script | R2-F1.1–R2-F1.8 | **verified fixed** | Both real shell loaders pass; executable shims that exit 127 make both selected tests fail through `expect_level!`. Source inspection and six passing real-terminal tests verify the L2 gates. Details below. | None. The opt-in selection mechanism was already disclosed in review-2.md; it needs no renewed approval. |
| R1-F2: Declared values bypass the shared conversion rules | R1-F2.1–R1-F2.22; already closed in review-2.md | **verified fixed** | Fresh public setter-value, set/compose, declared-shape, persistence, and command-output tests pass. Checked the retained text/quoted/container binding, judged-value persistence, and root-problem filtering routes described below. | None. |
| R1-F3: Union selection loses the origin of nested file setters | R1-F3.1–R1-F3.25; already closed in review-2.md | **verified fixed** | Fresh public and CLI supplied-file tests pass for objects, arrays, dictionaries, unions, recursive children, authored siblings, pending values, and transclusion. Source inspection retains both validation passes, scoped origins, diagnostic provenance, and presentation separation. | None. Fresh cross-OS proof is outside this review's readiness criteria. |
| R1-F4: Elvish and PowerShell lack the required shell verification paths | R1-F4.1–R1-F4.6; already closed in review-2.md | **verified fixed** | Bash, system Bash 3, Zsh, Fish, and Elvish script/word tests and six local L2 cases pass. Native-Windows PowerShell script and ConPTY Tab test paths remain present and correctly selected. | None. PowerShell was inspected, not rerun on this macOS host. The separate prerequisite defect is closed as R2-F1. |

### Read-once repair instances

The darkmatter-cli completion reader now surrounds schema selection with `with_reads_retained`. In simplified-schema, nested retention joins the outer scope, reads reuse active buffers, and restoration drops retained bytes afterward. Its buffer tests pass.

| Required instance | Verification evidence |
|---|---|
| R1-F1.22.1 | Public get-candidate read-count test passes; CLI selection test passes; independent get probe returns `status` and opens outside-root `base.yaml`, `types.yaml`, and `s.yaml` once each. |
| R1-F1.22.2 | Equivalent public set-candidate test and CLI selection test pass; independent probe returns `custom`, with the same one-open counts. |
| R1-F1.22.3 | Equivalent remove-candidate test and CLI selection test pass; independent probe returns `status`, with the same one-open counts. |
| R1-F1.22.4 | `an_ordinary_yaml_baseline_and_its_repeated_import_are_read_once` passes; recognition and resolution share the baseline bytes. |
| R1-F1.22.5 | `a_raw_json_schema_baseline_is_read_once` passes; the raw-schema branch uses the same retaining scope. |
| R1-F1.22.6 | `one_compose_tab_opens_the_baseline_flags_file_once` passes. `compose_baseline` reads through the scoped reader and continues to ignore `BASELINE_SCHEMA` by its existing command contract. |
| R1-F1.22.7 | `a_document_schema_and_its_repeated_import_are_read_once` passes; the independent outside-root document/import control also opens each once. |
| R1-F1.22.8 | Built-in/disabled no-custom-file test passes; disabled-trigger independent control reads only the document and its schema dependencies. |
| R1-F1.22.9 | Trigger-envelope check now uses `buffers::read_to_string` inside effective-schema resolution's retention scope. The document-schema read-count test and independent probe show no second check read. |
| R1-F1.22.10 | `one_edit_tab_opens_each_schema_file_once_across_the_baseline_and_document` passes for get/set/rm; independent outside-root probes confirm a shared catalog/document import opens once. |
| R1-F1.22.11 | Payload memo fingerprint reads through the scoped reader. Payload-memo tests pass; an independent trigger importing outside-root `types.yaml` opens that dependency once across discovery payload preparation and document resolution. |
| R1-F1.22.12 | Source inspection: `host_catalogs` passes its already-read text to `from_host_text`, which calls the shared catalog loader. Catalog tests pass. |
| R1-F1.22.13 | Source inspection: YAML document root-schema resolution and its trigger check share one retention scope. YAML-document tests pass. |
| R1-F1.22.14 | Source inspection: a document declaring `types` is excluded from catalog classification, so self-import follows the ordinary schema resolver and its retained source. Ordinary-baseline repeated-import and resolver buffer controls pass. |

The original R1-F1.1–.21 and .23–.24 remain repaired: the fresh selected-baseline suites exercise custom/disabled/built-in selection, flag/environment policy, document and trigger declarations, raw JSON declarations, named definitions, and failed-selection fallbacks. Source inspection confirms `declared_property_shape` supplies the request-selected view while DMLS retains its separate projection. Request policy reads the captured snapshot. The new finding concerns where discovery obtains bytes, not which schema wins.

### Shell prerequisite repair instances

| Required instance | Verification evidence |
|---|---|
| R2-F1.1 | `just test-optional-shells` runs and passes Fish's script loader. A PATH shim named `fish` exiting 127 produces FAIL with “level 1 requires fish”; recipe exit is 1. |
| R2-F1.2 | Same positive/negative checks for Elvish; the negative run says “level 1 requires elvish.” The positive test invokes the registered function through Elvish and asserts the returned candidate. |
| R2-F1.3 | Modern Bash script loader passes; its selected L1 prerequisite uses `expect_level!`. |
| R2-F1.4 | macOS system Bash 3 loader passes with the same mandatory gate. |
| R2-F1.5 | macOS Zsh loader passes with the same mandatory gate. |
| R2-F1.6 | Source inspection: native-Windows PowerShell uses a noninteractive startup probe and `expect_level!` before loading. |
| R2-F1.7 | All six local L2 completion tests pass. Each gates tmux with a named backend and its shell with `require_level!`; absent shells no longer return success outside the gate. Sessions are detached and take no focus. |
| R2-F1.8 | Source inspection: both PowerShell ConPTY tests retain mandatory L2 prerequisite gates and assert PSReadLine's actual input buffer after Tab. |

`optional_shells` is declared under the live L1 target and the opt-in `optional-shell-tests` feature; `just test-optional-shells` enables it deliberately. It is absent from default/CI feature lists as already documented. Both areas' tier-coverage checks report zero stranded tests.

### Earlier closed value/origin instances

For R1-F2.1–.9 and .13–.15, fresh setter-value and command tests check unknown text, decoded quoted values, declared scalar/union rules, native null, undeclared guesses, candidate spelling, `$schema`, and retained raw input during provisional schema selection. For .10–.12 and .18–.19, declared-shape/pattern-key and raw JSON reference/constraint tests pass; inspection follows `value_as_judged` into the saved edit. For .16–.17 and .20–.22, missing-required-key/root-problem and authored-parity cases pass; `partial_copy` still filters the root problem rather than replacing the whole document. No contrary old assertion was restored.

For R1-F3.1–.9, .11–.17, .20–.21, and .23–.25, fresh public/CLI origin cases cover nested containers, whole overrides, launch-only/document-only paths, authored siblings and copies, children, defaults, templates, pattern dictionaries, presentation, preflight, persisted edits, and refusal messages. Source inspection checks .10's shared preparation in both validation passes, .18's per-leaf classification records, .19's scoped supplied origins, and .22's retained Claudine deferral of nested supplied-file judgments to composition. The prior Claudine results are historical evidence, not a newly executed Claudine run.

## Unblocked Findings

### R3-F1 (medium): Schema-root discovery bypasses completion read retention

**Authority: spec.** Under “What it may read, and how fast,” completion must read “each distinct schema/document content file once per completion invocation, including repeated imports.” The same section explicitly includes configured local schema-root discovery. This finding is a newly found sibling of R1-F1, not a stronger guarantee invented by documentation.

**Class:** a discovery reader bypasses the operation's shared source buffers, so a later consumer rereads content already loaded during the same completion invocation.

In the darkmatter package, [FilesystemRoots::root_file_text](../../lib/src/markdown/schemas/triggers/discovery.rs:518), which reads schema-directory files to determine whether they are triggers, calls `std::fs::read_to_string`. It neither consults nor fills the active retained buffers. Darkmatter-cli's [DocumentSite::load](../../cli/src/args/path_completion.rs:309) correctly wraps schema selection in retention, but discovery bypasses it. Resolution then reads a discovered `$schema` target, named-type import, example, or already-selected custom baseline again.

A normal repository with its schemas in `schemas/` reaches this; no unusual data is needed. The candidates remain correct. Priority is medium because the harm is redundant local I/O rather than a wrong value, but the explicit acceptance rule still blocks readiness. Repair is small relative to the requirement: reuse the existing operation-scoped reader for filesystem discovery, preserving the host-supplied source interface and existing scope cleanup. Do not add another cache or filesystem abstraction.

#### Reproduction

Start with a temporary Git repository, an empty fixture-owned home, and these files. Set `SCHEMAS_DIR` to the fixture's absolute `schemas` directory so no ambient schema configuration participates. This follows the existing completion fixture's schema/type/trigger arrangement, moving the document schema and shared type into the ordinary schema directory.

```yaml
# doc.md frontmatter
$schema: ./schemas/s.yaml
status: custom
lang: rust
```

```yaml
# schemas/s.yaml
$schema:
  status: enum(custom)
  owner: shared@./types.yaml
```

```yaml
# schemas/types.yaml
kind: schema
types:
  shared: {name: string}
```

```yaml
# schemas/lang.trigger.yaml
kind: schema-trigger
match:
  lang: enum(rust; required)
$schema:
  owner: shared@./types.yaml
```

Run the built CLI through its shipped completion entry point:

```sh
COMPLETE=fish md -- md get doc.md sta
COMPLETE=fish md -- md set doc.md status cu
COMPLETE=fish md -- md rm doc.md sta
COMPLETE=fish md -- md compose doc.md status=cu
```

A temporary macOS `open` observer loaded into the existing debug binary recorded the content opens. It changed no repository source. Counts were normalized to full canonical paths, not basenames, and only fixture content files were counted; metadata probes and Git configuration were excluded. Each command opens `doc.md` once, `lang.trigger.yaml` once, `schemas/s.yaml` twice, and `schemas/types.yaml` twice. Candidates are respectively `status`, `custom`, `status`, and `status=custom`. Source inspection identifies both opens as full reads, not selection metadata probes.

#### Complete bounded sweep

Each failing row expects every distinct content file to be opened once. Clean controls retain their stated selection contracts. These child IDs are the repair inventory.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | Filesystem schema-root discovery | Shared YAML schema/type files in `schemas/`, used later in the same Tab | Direct reads bypass retention; later resolver opens each again | Discovery and resolution share the same bytes |
| R3-F1.2 | Get completion → document schema and imports | Fixture above, `sta` | Correct `status`; schema and import twice | Correct candidate; each once |
| R3-F1.3 | Set completion → document schema and imports | Same fixture, value `cu` | Correct `custom`; schema and import twice | Correct candidate; each once |
| R3-F1.4 | Remove completion → document schema and imports | Same fixture, `sta` | Correct `status`; schema and import twice | Correct candidate; each once |
| R3-F1.5 | Compose completion → document schema and imports | Same fixture, `status=cu` | Correct `status=custom`; schema and import twice | Correct candidate; each once |
| R3-F1.6 | Edit-command custom catalog baseline → discovery | Add the catalog from review-2.md under `schemas/base.yaml`; `BASELINE_SCHEMA=./schemas/base.yaml`; run get/set/rm | Correct candidates; catalog, document schema, and shared import each twice | Each once across baseline selection, discovery, and resolution |
| R3-F1.7 | Compose flag baseline → discovery | `schemas/base.yaml` containing `$schema: {status: enum(custom)}`; `--baseline-schema schemas/base.yaml` | Correct `status=custom`; baseline twice, document schema/import twice | Each once |
| R3-F1.8 | Trigger payload import → discovery | Document schema outside the root; trigger imports `shared@./types.yaml` inside it | Correct candidate; trigger dependency twice | Dependency once, regardless of discovery order |
| R3-F1.9 | Example-file resolution → discovery | Outside-root schema includes `greeting: string(example(./schemas/ex.yaml))`; root example declares `kind: example`, `invocation: demo`, `returns: hi`, `description: text` | Correct candidate; `schemas/ex.yaml` twice | Example content once |
| R3-F1.10 | Trigger envelope alone | Trigger does not import its own file | Trigger file once in all probes | Clean: the parsed envelope is reused |
| R3-F1.11 | Disabled trigger discovery | Compose fixture with `--disable schema-triggers` | Document, schema, and shared import each once | Clean: no discovery read |
| R3-F1.12 | Schemas outside discovery roots | Move catalog/schema/types to repository root, update references; leave only the importing trigger in `schemas/`; get/set/rm | Correct candidates; every distinct content path once | Clean: the catalog/read-retention repair works |
| R3-F1.13 | Raw JSON document schema | Outside-root `s.json` with Draft 2020-12 object schema and `status` enum | Correct `status=custom`; JSON file once | Clean: discovery enumerates only YAML/YML, so it cannot reread a JSON target |
| R3-F1.14 | Open editor buffers and host-supplied discovery | Host supplies `SchemaRootSource`; no direct filesystem reader required | Contract inspection: host controls root text; resolver reads active buffers; fresh unsaved-document tests pass | Clean interface; retain host authority rather than replacing its source |

The existing read-count tests do not catch this: they put the schema and imports outside the discovered directory, and `Observed.opened` counts only reads through simplified-schema's buffer reader. The direct discovery read is invisible to that counter. Add a public-result completion test with dependencies inside a discovered root, including a trigger-only file so the expected observed set proves discovery reads are counted too. Cover the root target, baseline overlap, shared import, and example paths using the same shared reader; one fixture can establish the common behavior without a large synthetic matrix.

## Blocked Findings

None. R3-F1 can be repaired within the existing design.

## Recurrence

R3-F1 repeats the duplicate-content-read class of review-1.md R1-F1 and review-2.md's carried R1-F1. It is a **sibling the earlier sweep missed**: filesystem schema-root discovery still bypasses retained reads. The catalog loader, document trigger-envelope check, cross-layer imports, and payload fingerprint changes are repaired; they did not introduce this direct reader.

The earlier sweep should also have followed bytes from schema-root discovery into baseline selection, document schema resolution, named-type imports, trigger payload dependencies, and example resolution when those files live in a discovered root. The instance table now includes those sites and the clean disabled/outside-root/JSON/host-source controls. This does not expand the requirement or call for another protective layer: the existing scoped reader is sufficient.

## Verification and Coverage

Fresh checks on this host:

- Darkmatter `just test path_candidates:: path_completion:: compose_setters:: get_set_rm:: property_paths::`: **404 passed**; no selected L1 test reached 5 seconds.
- Darkmatter `just test path_edit setters::tests output_commands:: context_construction_guard payload_memo yaml_document catalog`: **607 passed**; no selected L1 test reached 5 seconds.
- Simplified-schema `just test setter_values property_paths buffers`: **86 selected tests passed**, followed by the recipe's full L1 run: **1,298 passed**.
- Darkmatter `just test-optional-shells`: **2 passed**. Separate unavailable-shell shim run: **2 expected prerequisite failures**, recipe exit 1.
- Darkmatter `just test-l2 level2_path_completion`: **6 passed**, using detached tmux sessions without taking focus. Other package binaries selected zero matching tests, not additional shell evidence.
- `just check-tier-coverage darkmatter` and `simplified-schema`: **zero stranded tests**. Relevant modules are declared by their consolidated L1/L2 targets.
- Independent CLI file-open probes: positive catalog/outside-root controls and all failing discovery-overlap cases above. No source mutation was used for observation.

Build output reports an existing macOS linker unwind-table size warning. No compile/test failure occurred in the positive runs. No formatting command, source repair, commit, or feature-directory move was performed.

### Requirement-to-verification mapping

| Requirement | Appropriate level | Strongest evidence and limits |
|---|---|---|
| Escapes, empty/Unicode keys, canonical and huge indexes, scalar/null parents, atomic original-index removals | L1 | Public path and normal command tests pass. |
| Nested formatting, comments, fences/body, flow fallback, alias protection, read/write/read persistence | L1 | Editor/command tests pass; inspection confirms fallback reparses and compares the complete expected frontmatter before write. |
| Flat get results, original argument provenance, remove's top-level remaining list, stdout/save/stdin and output envelopes | L1 | Get/set/rm, property-path, and output-command tests pass. |
| Shared value parsing, declared text precision, quoted conversion, container admission, specificity and combined required children | L1 | Public setter adapter and command/pipeline tests pass. |
| Selected schema, trigger ordering, unknown-key errors, supplied-file origins and pending values | L1 | Selected-baseline, public/CLI compose, and validation cases pass; origin plumbing inspected. |
| Passive completion, candidate ordering/escaping, unions, earlier words, fallbacks, file candidates, denied evaluation/network | L1 | Candidate and shipped completion-entry tests pass, including observable denial seams. |
| Each distinct content file read once | L1 | Catalog repair controls pass; independent schema-root overlap probes fail: R3-F1. |
| Registration scripts and shell word handling | L1 | Local Bash/Bash 3/Zsh/Fish/Elvish loaders and word tests pass; native-Windows PowerShell loader inspected. Optional-shell absence now fails when selected. |
| Real Tab replacement, continued paths, spaces, and Elvish acceptance fallback | L2 | Six local real-terminal tests pass; PowerShell ConPTY tests remain implemented and use actual shell buffers. Fresh native-Windows results are not claimed. |

No feature requirement needs OS keyboard injection: these tests verify what shells do with Tab bytes and returned candidates, not a terminal emulator's keyboard encoding or bare-modifier behavior. No new mismatch between required and implemented verification levels was found.

### Input Robustness Matrix

The matrix applies to setter container loaders, not to the dotted-path text grammar, shell words, or the source-preserving YAML writer. The latest read-retention repair adds no configuration field or format parser.

The default remains: admitted containers with duplicate keys or inadmissible numbers fail; syntactically rejected arguments may remain text, as the spec permits. Exceptions preserve absent members, explicit null as data, empty values/collections, declared unknown text, and decoded quoted strings. Both JSON5 and YAML paths retain whole-field, mixed-element, and all-element type failures for validation rather than silently dropping values. Fresh public `SetterValue` tests check these shapes, duplicate keys, trailing/invalid syntax, JSON5 comments/trailing commas, YAML tags/anchors/extra documents, and numeric overflow/underflow. Public command tests verify binding and no-write failures through the invocation path.

The source-smell search found no load-bearing defaulted field or element-filtering parse in the shared setter reader. Its YAML numeric `.ok()` fails the whole container conversion after numeric admission; it does not filter a member. Advisory completion's `.ok()` deliberately omits invalid incomplete words or unavailable schema sources. Catalog field admission still uses the existing strict validator; its loading tests pass. R3-F1 concerns content reuse, not shape validation.
