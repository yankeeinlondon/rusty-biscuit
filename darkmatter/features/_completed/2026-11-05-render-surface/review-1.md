---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Early failures and selective denial discard style diagnostics"
  - "R1-F2 (high): Whole-style setter type failures use the wrong diagnostic family"
  - "R1-F3 (medium): Superseded duplicate-key setters still fail parsing"
  - "R1-F4 (low): Typo hints ignore the document’s effective style schema"
human_review: true
human_review_items:
  - "Optional confirmation or reversal after repair: Prerequisites — Accepted the completed, reviewed prerequisite features and verified their APIs instead of rerunning their implementation plans; recommend keeping this choice."
  - "Optional confirmation or reversal after repair: Execution order — Ran overlapping Phase 2 and Phase 3 work serially and moved Phase 5 migration/removal before final parity testing; the final scope is unchanged."
  - "Optional confirmation or reversal after repair: Inactive notices — Preserved existing informational notices for some component fields that already render. The spec explicitly asks to preserve these notices; they are not evidence that rendering is absent."
  - "Optional confirmation or reversal after repair: Page aliases — Removed snake-case aliases for renamed/new page fields, while retaining older deprecated aliases in documents only; files exclude them and setters reject them. This follows the canonical-setter and removal rules."
  - "Optional confirmation or reversal after repair: Mode and theme parsing — Made code mode case-sensitive to match its schema; kept theme aliases and case handling in the shared theme parser, with runtime validation. Recommend keeping this choice."
  - "Optional confirmation or reversal after repair: Document type validation — Used the effective schema for document unknown-key admission and retained existing runtime type validation rather than adding full document schema enforcement to render. This preserves the specified existing behavior."
  - "Optional confirmation or reversal after repair: Document diagnostic codes — Kept dm.style.unknown_key for authored style warnings and compose errors, and dm.schema.* for setter admission failures. Recommend keeping the source distinction."
  - "Optional confirmation or reversal after repair: Border union replacement — A nested border setter replaces a preceding scalar border with the object form without translating the scalar into a weight; this follows the specified union replacement rule."
  - "Optional confirmation or reversal after repair: Theme capture timing — Moved environment theme defaults into the captured request earlier than planned; this satisfies the one-request-context requirement."
  - "Optional confirmation or reversal after repair: Stylesheet targets — Validated stylesheet reference syntax on all targets, read local CSS only for HTML and MarkdownPlus, and used launch context for stdin. This follows the target and origin rules."
  - "Optional confirmation or reversal after repair: Empty metadata — Treated an empty authored page.meta object as absent and required nonempty metadata to be an object. Empty metadata has no observable rendering payload; recommend keeping this behavior."
  - "Optional confirmation or reversal after repair: Alignment and padding — Broadcast page alignment to every existing component, including horizontal rules, and exposed already-implemented image/hyperlink padding in CSS. These retain the intended render capabilities."
  - "Optional confirmation or reversal after repair: Temporary compatibility code — Kept hidden legacy flags and transitional presentation code only between implementation phases, then removed both. No compatibility bridge remains to approve."
  - "Optional confirmation or reversal after repair: Root help wording — Used ARGS instead of INPUT in the alias sentence to match the actual positional definition; showed shared controls and only --style in root render help. Recommend retaining this cheap wording choice."
  - "Optional confirmation or reversal after repair: Ineffective code-block option — Removed md code-block --code-block MODE, which had been accepted globally but had no effect; preserved the real local schema-about flag. This follows removal of the global rendering flag."
  - "Optional confirmation or reversal after repair: Offset escapes — Treated property-path escapes before the offset separator as escapes rather than path separators, and a leading @ as a magic reference. Recommend this reading because it preserves both the required escaped-dot offset and explicit-reference rules."
  - "Optional confirmation or reversal after repair: File failure codes — Reused input-read and malformed-data codes, and used invalid-root for missing/nonmapping file offsets. The spec does not prescribe replacement file-failure codes; keep these typed failures."
  - "Optional confirmation or reversal after repair: Warning registry — Left always-fatal style errors outside the --deny warning registry. They already fail without a policy switch; recommend keeping this choice."
  - "Optional confirmation or reversal after repair: Diagnostic locations — Added an explicit located-failure marker so setter and style-file errors do not acquire the Markdown document path. Recommend keeping this shared reporting mechanism."
  - "Optional confirmation or reversal after repair: Schema fallback — Preserved document-only rendering when schema selection fails, with built-in style admission and a debug log; CLI styles still fail because they need the selected schema. Recommend preserving this existing document-render behavior."
  - "Optional confirmation or reversal after repair: Source outputs — Retried preparation with CLI layers alone when source outputs encounter ignorable document style problems; data outputs retain document warnings and formatted Markdown suppresses them. This implements the specified target-dependent behavior."
  - "Optional confirmation or reversal after repair: Authored positions — Added document-line positions to block-mapping style warnings; flow mappings retain unknown positions rather than invented coordinates. Recommend keeping this improvement."
  - "Optional confirmation or reversal after repair: Non-render commands — Made clean/set rendered previews use only document style; removed root render options no longer influence those commands. This follows root isolation."
  - "Optional confirmation or reversal after repair: Terminal capture — Built page terminals from resolved output options rather than redetecting the host. Recommend retaining the deterministic output context."
  - "Optional confirmation or reversal after repair: Denied severity — Represented denied style warnings as error records while retaining their diagnostic codes, and documented that exception. The spec requires blocking, not a particular severity conversion; keep this choice. Retaining other warnings is separately required by R1-F1."
  - "Optional confirmation or reversal after repair: Trigger warning delivery — Added render schema-trigger warnings but deferred preserving them on later load failures. Recommend repairing that omission with R1-F1; no author decision is needed."
  - "Optional confirmation or reversal after repair: No-input testing seam — Captured whether stdin is a terminal in OutputEnvironment and used the existing input-read code for its data failure. Recommend retaining the deterministic testing seam."
  - "Optional confirmation or reversal after repair: Theme-list representation — Kept list contents/order and dark-mode descriptions, used bare arrays for JSON/JSON5/YAML and a value wrapper for TOML, and added setter guidance to formatted output. Recommend keeping the existing data-family conventions."
  - "Optional confirmation or reversal after repair: Completion implementation — Added a render mode to the shared schema completer, offered only files after --, and changed the shell-test prefix to style.pag to avoid the newly ambiguous padding prefix. These are internal choices consistent with the spec."
  - "Optional confirmation or reversal after repair: Image padding schema — Added image padding keys to the schema/descriptor because the model already implemented them and removed image-fill flags needed replacements. This stays within the required capability migration."
  - "Optional confirmation or reversal after repair: Color spellings — Kept the existing style color grammar and documented hex/current replacements for RGB triples/currentColor. The capability remains available; recommend avoiding an unrelated grammar expansion."
  - "Optional confirmation or reversal after repair: Missing stylesheet origin — Made missing stylesheet provenance a typed error instead of guessing the document context. Preparation supplies an origin, so this protects an internal invariant without changing valid invocations."
  - "Optional confirmation or reversal after repair: Union and path diagnostics — Fixed unknown members inside union objects to use normal unknown-key policy and prefixed schema messages with the canonical style path. These implement the existing contract."
  - "Optional confirmation or reversal after repair: Failure artifacts — Preserved existing --show behavior for data failure artifacts; interpreted the plan’s no-artifact rule as no rendered-page artifact. Recommend preserving the spec’s existing output contract instead of changing global artifact behavior."
  - "Optional confirmation or reversal after repair: Characterization helpers — Kept explicit temporary-directory contexts in older style-chain test helpers rather than changing them to full preparation. This preserves the narrower behavior those tests characterize."
  - "Optional confirmation or reversal after repair: Typo hints — Used built-in sibling names for bounded typo suggestions, without custom-schema siblings. Recommend repairing the mismatch with effective-schema admission as R1-F4."
  - "Optional confirmation or reversal after repair: Malformed inline diagnostic — Used dm.style.invalid_root for malformed inline whole-style values and changed docs to match. Recommend restoring the spec’s schema diagnostic contract as R1-F2; the spec supplies the answer."
  - "Optional confirmation or reversal after repair: Documentation exclusions — Left a design-exploration note with old flag syntax and three preexisting source comments naming specs unchanged. These are outside the current-doc migration, and no functional repair depends on them."
  - "Optional confirmation or reversal after repair: Test file size — Kept the 852-line parity test together despite an advisory file-size limit. Recommend keeping its cohesive inventory and guards rather than splitting it for line count alone."
  - "Optional confirmation or reversal after repair: Color-terminal parity — Deferred a dedicated root-versus-render parity test on a real color terminal. Parser/prepared-value and output parity are covered at L1, and actual styling has migrated L2 coverage; this does not independently require a duplicate terminal parity matrix."
  - "Optional confirmation or reversal after repair: Browser timeout — Left the existing embedded-Mermaid browser font test timing out at 30 seconds on this host; the log records a direct pass in about 42 seconds and attributes it to CoreText scanning a huge build directory. This is a disclosed validation limit, not a demonstrated render-surface defect; no timeout increase is recommended."
  - "Optional confirmation or reversal after repair: Other operating systems — Deferred Windows, Linux, and WSL2 execution to CI because the planned transport creates a commit; did not repeat dependent compile checks after the final test-only phase. Cross-OS proof does not block this review, and no commit is authorized here."
  - "Repair decision (R1-F2.12): with no effective schema at all, a non-mapping whole-style setter (`style=4`) also reports dm.schema.type_mismatch, while a nested container setter in that mode (`style.page=4`) keeps dm.style.invalid_value from the built-in descriptor. Alternative: use invalid_value for the root in schemaless mode too; chose the spec's schema code for every whole-style setter."
  - "Repair decision (R1-F1): style warnings now precede the run's error record in diagnostics when both are reported; reading still stops at the first failing --style input, so a second bad file is not reported in the same run."
  - "Repair decision (R1-F4.5): a rename-only hint (old page key → new name) is now dropped when the document's effective schema does not admit the new name; under the baseline it is unchanged."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-07T18:15:40-08:00"
spec: 2026-11-05-render-surface/spec.md
implemented: true
description: "A **feature** review of `2026-11-05-render-surface/spec.md`"
feature: 2026-11-05-render-surface/review-1.md
next: 2026-11-05-render-surface/review-2.md
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-render-surface/log.md
implemented_by: claude/opus
---
# Render surface review, iteration 1

The feature is **not production ready**. Four findings need repair. None needs a human decision before repair; the spec supplies safe answers. This is the first review, so there are no previous findings to close or recurring classes to report.

The core architecture matches the design: root and explicit rendering share argument preparation and dispatch; sparse layers normalize before merging; CLI style does not modify authored frontmatter; file origins survive to stylesheet resolution; and legacy claim machinery is removed. The findings concern error reporting and replacement semantics, rather than a need to redesign the layer model.

## Findings

### R1-F1 (high): Early failures and selective denial discard style diagnostics

**Authority — spec:** “Keep document warnings and their original positions; a CLI override must not conceal an authored unknown or deprecated key.” The file rules also say excluded unknown keys produce warnings. **Defect:** schema-trigger warnings already collected by the new render schema-selection path are dropped if a subsequent input fails to load.

**Class:** preparation and policy early returns discard diagnostics from other inputs instead of carrying them into the final failure.

In darkmatter-cli, [prepare_cli_style_inputs](../../cli/src/render_style.rs#L253) returns immediately from file loading or setter-path parsing, before returning the selected trigger findings or preparing the document layer. Later, [Render::stop_if_denied](../../cli/src/commands/render.rs#L198), which turns selected style warnings into a run failure, retains only blocked records; the caller has not yet published any style warnings. A common case is fixing a style-file path while the Markdown also contains an old style spelling: the resulting report hides the document issue entirely.

Reproduction fixture, based on the existing render-dispatch trigger test:

```yaml
# doc.md frontmatter
style:
  magin: 4
  page:
    max_width: 50
```

The body is `# Notes`. In a temporary Git repository, add `schemas/broken.trigger.yaml` containing only `kind: schema-trigger`. Without overrides, JSON output reports the invalid trigger, deprecated max_width, and unknown magin. Change just the CLI input as below. All failure cases still exit 1 and emit one JSON envelope with empty stderr; the defect is missing records.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F1.1 | File opening | `--style missing.yaml` | Only input-read failure | Preserve document warnings and already collected trigger warning alongside the failure |
| R1-F1.2 | Shared data loader | `--style bad.yaml`, containing `margin: [` | Only malformed-data failure | Same preservation, with authored document positions unchanged |
| R1-F1.3 | Offset selection | `--style theme@offset.yaml`, containing `theme: 2` | Only invalid-root failure | Same preservation |
| R1-F1.4 | Argument classification | Remote YAML reference; separately an unsupported `.css` reference | Only input-read failure | Same preservation; no network read |
| R1-F1.5 | Setter-path parser | `style..x=1` | Only invalid-setter failure | Same preservation |
| R1-F1.6 | Earlier file layer | Valid `good.yaml` has `margin: 2` and `bogus: 1`, followed by malformed `bad.yaml` | Earlier file warning also disappears | Keep the warning from the earlier supplied layer when reporting the later failure |
| R1-F1.7 | Selective denial | `--deny dm.style.unknown_key` | Only unknown-key error; deprecation warning disappears | Retain deprecation warning, trigger warning, and blocked unknown-key record |
| R1-F1.8 | Selective denial, reverse | `--deny dm.style.deprecated` | Only deprecation error; unknown-key warning disappears | Retain unknown-key warning, trigger warning, and blocked deprecation record |
| R1-F1.9 | Layer validation, clean sibling | `style.margin=abc` | Type error plus document and trigger warnings | Correct; retain this behavior |
| R1-F1.10 | Shared setter-value parse failure, clean sibling | `style={margin:2,margin:3}` | Invalid-setter error plus document and trigger warnings | Correct; retain this behavior |
| R1-F1.11 | Deny whole family, clean sibling | `--deny style` | Both style warnings appear as errors | Correct; no record disappears merely because another is blocked |
| R1-F1.12 | Successful preparation, clean sibling | No CLI override, or valid `style.margin=2` | Authored warnings and trigger warning appear | Correct |

The sites cover the input readers, their offset/classification decisions, layer preparation, and the policy decision consuming those warnings. These are shared before target output; JSON exposes the records directly. Formatted output must likewise retain the applicable warnings on stderr without printing a page on failure.

**Smallest repair:** carry pending diagnostics through failed input preparation, including the document layer and earlier loaded layers, and preserve all style records when selective denial makes the run fail. Keep denied records marked according to the existing chosen severity convention. Add focused L1 public-result tests for the rows above; no new logging framework or output format is needed.

### R1-F2 (high): Whole-style setter type failures use the wrong diagnostic family

**Authority — spec:** under “Style input preparation and errors,” the malformed inline example `style="{ margin: "` is an ordinary schema failure with the matching `dm.schema.*` code. The Tests section explicitly requires that example to exit 1 with that family and its key path. Documenting a different family does not supersede that requirement.

**Class:** the whole-style setter takes a special type-error branch that bypasses the schema diagnostic contract used by nested setters.

In darkmatter, [assemble_setters](../../lib/src/style/layer.rs#L1037), which constructs the final CLI setter layer, reports `dm.style.invalid_root` whenever the bound whole-style value is not an object. The exact specified malformed example exits 1 with that code, not a schema code. This matters to consumers categorizing JSON diagnostics as schema failures. Its message also says only `style`, rather than a misleading file location, which should be retained.

The same temporary `# Notes` document was used for this complete whole-root shape sweep and adjacent setter paths:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.1 | Whole-style branch | `style={ margin: ` | `dm.style.invalid_root`, exit 1 | `dm.schema.type_mismatch`, naming `style`, exit 1 |
| R1-F2.2 | Whole-style branch | `style=4` | `dm.style.invalid_root` | Schema type mismatch for a nonmapping setter root |
| R1-F2.3 | Whole-style branch | `style=null` | `dm.style.invalid_root` | Schema type mismatch; root null is invalid, unlike null fields |
| R1-F2.4 | Whole-style branch | `style=[]` | `dm.style.invalid_root` | Schema type mismatch; root array is invalid |
| R1-F2.5 | No root supplied, clean sibling | No setter | Success/default style | Correct |
| R1-F2.6 | Empty root, clean sibling | `style={}` | Success/no-op | Correct; do not turn empty into absent or a failure |
| R1-F2.7 | Nested container, clean sibling | `style.page=4` | `dm.schema.type_mismatch` | Correct |
| R1-F2.8 | Scalar field, clean sibling | `style.margin=abc` | `dm.schema.type_mismatch` | Correct |
| R1-F2.9 | Union field, clean sibling | `style.table.border={ left:` | `dm.schema.constraint` | Correct rejection through the union schema |
| R1-F2.10 | File-root adapter, clean sibling | Style file containing a scalar or array | `dm.style.invalid_root` | Keep: the spec does not require setter schema codes for file-root failures |

Null members inside valid CLI objects already mean absence by explicit spec exception. Valid mappings, absent fields, empty mappings, scalar roots, null roots, array roots, and malformed whole-object text are accounted for above; collection-element shapes do not apply to the required mapping root. File document syntax/duplicates use the separate shared loaders and should retain their existing rules.

**Smallest repair:** report whole-style setter type mismatches with the shared schema type-mismatch code while preserving the setter origin and message. Update the tests and current CLI error docs that currently pin invalid-root for this setter case. Leave file/offset invalid-root codes unchanged. This is an ordinary contract repair, not an unresolved choice of public format.

### R1-F3 (medium): Superseded duplicate-key setters still fail parsing

**Authority — spec:** “Within the combined setter layer, discard superseded occurrences of the same exact path before conversion, as the shared setter parser does.” The repeated-path rule explicitly includes whole-object setters.

**Class:** duplicate assignment selection runs after fallible value parsing, so an assignment that should be discarded can still reject the command.

In darkmatter, [assemble_setters](../../lib/src/style/layer.rs#L1002) calls the shared value parser before selecting the last occurrence at a path. A duplicate object key in an earlier assignment therefore fails even when the user provides a valid final replacement. This can occur when shell defaults supply an inline object and an invocation appends a corrected value.

Run each tail below after `md render doc.md --output json`, with `doc.md` containing `# Notes`:

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F3.1 | Whole-style assignment | `'style={ margin: 1, margin: 2 }' 'style={ margin: 3 }'` | Invalid-setter duplicate-key failure, exit 1 | Success; only final assignment is parsed |
| R1-F3.2 | Union-object assignment | `'style.table.border={left:true,left:false}' style.table.border=thin` | Same failure | Success; final scalar border wins |
| R1-F3.3 | Open metadata assignment | `'style.page.meta={a:1,a:2}' 'style.page.meta={b:3}'` | Same failure | Success; final metadata wins |
| R1-F3.4 | Cleanup object assignment | `'style.markdown={unwrap:true,unwrap:false}' 'style.markdown={unwrap:true}'` | Same failure | Success; final valid cleanup object remains inert for render |
| R1-F3.5 | Scalar conversion, clean sibling | `style.margin=abc style.margin=2` | Success | Correct: schema binding is already after replacement |
| R1-F3.6 | Malformed text fallback, clean sibling | `'style={ margin: ' 'style={ margin: 2 }'` | Success | Correct: shared parser preserves this malformed text as a value until the replacement discards it |
| R1-F3.7 | Surviving duplicate object, clean sibling | `'style={margin:1,margin:2}'` alone | Duplicate-key rejection | Correct; do not weaken duplicate-key validation of surviving assignments |
| R1-F3.8 | Separate files, clean sibling | Earlier file has invalid margin and later file replaces margin | Existing public-result test rejects earlier file | Correct: files remain separate checked inputs, unlike repeated exact setter paths |

Every raw setter family reaches this same parser/replacement loop, including custom fields; pre-parsed values cannot hit the fallible raw parse by their enum variant. The problem is not silently accepting invalid field shapes: it is checking an assignment the contract says to discard. Keep the duplicate-key document rule for the surviving object and keep different normalized spellings conflicting.

**Smallest repair:** select the last assignment per decoded exact path before calling the shared value parser or binding values. Retain current parent-before-child application and equivalent-path conflict detection. Add public preparation tests for these replacement families and one CLI wiring test.

### R1-F4 (low): Typo hints ignore the document’s effective style schema

**Authority — spec:** a setter error “suggests the nearest valid key,” and a document declaring `style` replaces the built-in definition. Thus a built-in key removed by the document is not a valid correction.

In darkmatter, [report_unknown](../../lib/src/style/layer.rs#L748), which formats rejected setter keys, asks the built-in descriptor for suggestions. With `$schema: { style: { accent: string } }`, `style.accent=blue` succeeds, `style.acent=blue` fails without suggesting `accent`, and `style.magin=4` suggests `style.margin` even though that correction also fails under this schema. All three outcomes were reproduced through JSON output of the shipped CLI.

**Smallest repair:** derive sibling candidates from the already-selected effective schema when present, using the existing shared schema/path facilities and the current bounded distance calculation. Fall back to descriptor siblings only when there is no schema. Keep file/document rename-only hints unchanged. Add one custom-schema positive suggestion and one assertion that a rejected built-in field is never suggested.

## Verification and acceptance coverage

The following checks were rerun for this review through nextest recipes. Every selected test passed and none took five seconds:

| Check | Result |
|---|---|
| `just test render_style` | 90 passed |
| `just test style_layer` | 140 passed |
| `just test render_alias_parity` | 42 passed |
| `just test render_dispatch` | 22 passed |
| `just check-tier-coverage darkmatter` | Zero stranded tests |
| `just test-l2 level2_render_no_input` | One CLI real-terminal test passed; no matching library/DMLS tests |

The CLI and library both disable automatic test discovery, but their declared L1 binaries include the new style, matrix, acceptance, and parity modules. The listed test paths select L1 as intended. No review tests or production code were added.

A combined filter attempt using shell pipe characters was rejected by the recipe shell; separate supported recipe filters above completed successfully. Compiler output included an existing linker unwind-size warning. Neither is counted as a feature failure.

| User-facing requirement group | Strongest relevant evidence present | Assessment |
|---|---|---|
| Alias arguments, defaults, root isolation, placement, terminators, command-named files, removed flags and help | L1 clap inventory guards, prepared-result comparisons, CLI byte/status/diagnostic tests | Appropriate; both forms share the typed render entry point |
| No-input terminal behavior | L1 injected stdin-terminal environment plus L2 `level2_render_no_input` real-terminal wiring | Appropriate; redirected stdin remains L1 |
| Layer order, normalization, sparse borders, width replacement, null/zero/false, union selection, custom admission and required fields | L1 public library/CLI preparation tests and typed-value assertions | Appropriate; missing cases are listed as findings rather than inferred from passing counts |
| File forms, data syntax, offsets, origin contexts, passive preparation, stylesheet reads, failures and stream contracts | L1 public-result loader matrices and CLI wiring | Appropriate for these file and process boundaries |
| Rendered dimensions, alignment, horizontal-rule padding, code theme/mode, line numbers and plain capability degradation | L1 deterministic output checks plus migrated L2 layout/horizontal-rule/code-style tests | Actual terminal rendering has L2 coverage; piped terminal parity alone proves only plain output |
| HTML and MarkdownPlus structure and page effects | L1 HTML assertions plus existing headless browser style/geometry tests, inspected and reported in the implementation log | Appropriate browser boundary; browser run limitation disclosed below |
| Completion keys and enum values in supported shells, passive schema selection, no style-file preparation | L1 completion-engine tests plus migrated L2 real-shell path tests | Appropriate; no OS keyboard-event behavior was introduced |
| Theme listing, command-local survivor flags, immutable authored output and unchanged cleanup | L1 command-output schema conformance, save tests, cleanup/source-output assertions | Appropriate |

No new requirement depends on bare modifier events, IME, mouse encoding, or OS keyboard injection, so L3 is not required. A spawned CLI reading manufactured fixtures is L1; it does not become native-input evidence merely because the real binary ran.

The implementation log reports full L1, lint, L2 and dependent compilation results. It also explicitly reports a browser font-test timeout and the absence of cross-OS runs. I did not reinterpret the timeout as a passing recipe or cross-OS evidence. The timeout is in unchanged Mermaid/font behavior, with a reported direct pass after the configured deadline; there is no inspected render-surface defect to file from that fact alone. CI remains responsible for cross-OS proof. A dedicated color-terminal alias comparison is absent, but shared dispatch, L1 prepared/output equality, and existing L2 style behavior cover the specified responsibilities without requiring a second terminal matrix solely for the alias spelling.

## Input robustness review

The matrix applies to `--style` data files, not Markdown grammar or renderer output. Loaders are shared with load_data: JSON, JSON5, YAML/YML, and TOML branches. The existing CLI public-result tests exercise their controls, duplicate-key policy, invalid/trailing content, format syntax, and located failures. JSON keeps its specified last duplicate value; inline object parsing still rejects duplicate keys.

The shared layer matrix tests validator families once, then controls and invalid wiring cases for related fields: row counts, horizontal lengths, alignment, colors, backgrounds, border unions, booleans, themes/modes, metadata, required fields, and custom arrays. CLI null fields explicitly mean absence; zero and false remain values. Empty inline roots are a no-op, while files with no supplied leaf fail. R1-F2 records all root-shape siblings; R1-F3 records the complete repeated-assignment parsing boundary. Existing custom-array tests cover absent, null, wrong whole type, mixed/every wrong element and empty replacement; root document shapes are handled by the separate loaders.

I searched the changed layer/normalization/presentation and CLI style-reader code for serde defaults, filter_map, unwrap_or_default, and swallowed parse errors. The layer filter_map accumulates problems in its report; diagnostic filtering deliberately excludes informational records; position conversion can leave a position unknown. Invalid alignment is retained for later validation rather than accepted by its parse probe. No additional silent-default finding is supported by those sites.

## Implementation decisions (informational)

Two independent readers examined the implementation log and spec in parallel before this review comparison. The inventory below includes their combined choices, flagged author items, and deferred work. Every item has a recommended path within the author’s decisions, or is a reversible implementation/testing choice. None passes the stopping-for-a-human test. Optional confirmation or reversal happens after the repair cycle; these items never block repair.

1. **Prerequisites.** Accepted the completed, reviewed prerequisite features and verified their APIs instead of rerunning their implementation plans; recommend keeping this choice.
2. **Execution order.** Ran overlapping Phase 2 and Phase 3 work serially and moved Phase 5 migration/removal before final parity testing; the final scope is unchanged.
3. **Inactive notices.** Preserved existing informational notices for some component fields that already render. The spec explicitly asks to preserve these notices; they are not evidence that rendering is absent.
4. **Page aliases.** Removed snake-case aliases for renamed/new page fields, while retaining older deprecated aliases in documents only; files exclude them and setters reject them. This follows the canonical-setter and removal rules.
5. **Mode and theme parsing.** Made code mode case-sensitive to match its schema; kept theme aliases and case handling in the shared theme parser, with runtime validation. Recommend keeping this choice.
6. **Document type validation.** Used the effective schema for document unknown-key admission and retained existing runtime type validation rather than adding full document schema enforcement to render. This preserves the specified existing behavior.
7. **Document diagnostic codes.** Kept dm.style.unknown_key for authored style warnings and compose errors, and dm.schema.* for setter admission failures. Recommend keeping the source distinction.
8. **Border union replacement.** A nested border setter replaces a preceding scalar border with the object form without translating the scalar into a weight; this follows the specified union replacement rule.
9. **Theme capture timing.** Moved environment theme defaults into the captured request earlier than planned; this satisfies the one-request-context requirement.
10. **Stylesheet targets.** Validated stylesheet reference syntax on all targets, read local CSS only for HTML and MarkdownPlus, and used launch context for stdin. This follows the target and origin rules.
11. **Empty metadata.** Treated an empty authored page.meta object as absent and required nonempty metadata to be an object. Empty metadata has no observable rendering payload; recommend keeping this behavior.
12. **Alignment and padding.** Broadcast page alignment to every existing component, including horizontal rules, and exposed already-implemented image/hyperlink padding in CSS. These retain the intended render capabilities.
13. **Temporary compatibility code.** Kept hidden legacy flags and transitional presentation code only between implementation phases, then removed both. No compatibility bridge remains to approve.
14. **Root help wording.** Used ARGS instead of INPUT in the alias sentence to match the actual positional definition; showed shared controls and only --style in root render help. Recommend retaining this cheap wording choice.
15. **Ineffective code-block option.** Removed md code-block --code-block MODE, which had been accepted globally but had no effect; preserved the real local schema-about flag. This follows removal of the global rendering flag.
16. **Offset escapes.** Treated property-path escapes before the offset separator as escapes rather than path separators, and a leading @ as a magic reference. Recommend this reading because it preserves both the required escaped-dot offset and explicit-reference rules.
17. **File failure codes.** Reused input-read and malformed-data codes, and used invalid-root for missing/nonmapping file offsets. The spec does not prescribe replacement file-failure codes; keep these typed failures.
18. **Warning registry.** Left always-fatal style errors outside the --deny warning registry. They already fail without a policy switch; recommend keeping this choice.
19. **Diagnostic locations.** Added an explicit located-failure marker so setter and style-file errors do not acquire the Markdown document path. Recommend keeping this shared reporting mechanism.
20. **Schema fallback.** Preserved document-only rendering when schema selection fails, with built-in style admission and a debug log; CLI styles still fail because they need the selected schema. Recommend preserving this existing document-render behavior.
21. **Source outputs.** Retried preparation with CLI layers alone when source outputs encounter ignorable document style problems; data outputs retain document warnings and formatted Markdown suppresses them. This implements the specified target-dependent behavior.
22. **Authored positions.** Added document-line positions to block-mapping style warnings; flow mappings retain unknown positions rather than invented coordinates. Recommend keeping this improvement.
23. **Non-render commands.** Made clean/set rendered previews use only document style; removed root render options no longer influence those commands. This follows root isolation.
24. **Terminal capture.** Built page terminals from resolved output options rather than redetecting the host. Recommend retaining the deterministic output context.
25. **Denied severity.** Represented denied style warnings as error records while retaining their diagnostic codes, and documented that exception. The spec requires blocking, not a particular severity conversion; keep this choice. Retaining other warnings is separately required by R1-F1.
26. **Trigger warning delivery.** Added render schema-trigger warnings but deferred preserving them on later load failures. Recommend repairing that omission with R1-F1; no author decision is needed.
27. **No-input testing seam.** Captured whether stdin is a terminal in OutputEnvironment and used the existing input-read code for its data failure. Recommend retaining the deterministic testing seam.
28. **Theme-list representation.** Kept list contents/order and dark-mode descriptions, used bare arrays for JSON/JSON5/YAML and a value wrapper for TOML, and added setter guidance to formatted output. Recommend keeping the existing data-family conventions.
29. **Completion implementation.** Added a render mode to the shared schema completer, offered only files after --, and changed the shell-test prefix to style.pag to avoid the newly ambiguous padding prefix. These are internal choices consistent with the spec.
30. **Image padding schema.** Added image padding keys to the schema/descriptor because the model already implemented them and removed image-fill flags needed replacements. This stays within the required capability migration.
31. **Color spellings.** Kept the existing style color grammar and documented hex/current replacements for RGB triples/currentColor. The capability remains available; recommend avoiding an unrelated grammar expansion.
32. **Missing stylesheet origin.** Made missing stylesheet provenance a typed error instead of guessing the document context. Preparation supplies an origin, so this protects an internal invariant without changing valid invocations.
33. **Union and path diagnostics.** Fixed unknown members inside union objects to use normal unknown-key policy and prefixed schema messages with the canonical style path. These implement the existing contract.
34. **Failure artifacts.** Preserved existing --show behavior for data failure artifacts; interpreted the plan’s no-artifact rule as no rendered-page artifact. Recommend preserving the spec’s existing output contract instead of changing global artifact behavior.
35. **Characterization helpers.** Kept explicit temporary-directory contexts in older style-chain test helpers rather than changing them to full preparation. This preserves the narrower behavior those tests characterize.
36. **Typo hints.** Used built-in sibling names for bounded typo suggestions, without custom-schema siblings. Recommend repairing the mismatch with effective-schema admission as R1-F4.
37. **Malformed inline diagnostic.** Used dm.style.invalid_root for malformed inline whole-style values and changed docs to match. Recommend restoring the spec’s schema diagnostic contract as R1-F2; the spec supplies the answer.
38. **Documentation exclusions.** Left a design-exploration note with old flag syntax and three preexisting source comments naming specs unchanged. These are outside the current-doc migration, and no functional repair depends on them.
39. **Test file size.** Kept the 852-line parity test together despite an advisory file-size limit. Recommend keeping its cohesive inventory and guards rather than splitting it for line count alone.
40. **Color-terminal parity.** Deferred a dedicated root-versus-render parity test on a real color terminal. Parser/prepared-value and output parity are covered at L1, and actual styling has migrated L2 coverage; this does not independently require a duplicate terminal parity matrix.
41. **Browser timeout.** Left the existing embedded-Mermaid browser font test timing out at 30 seconds on this host; the log records a direct pass in about 42 seconds and attributes it to CoreText scanning a huge build directory. This is a disclosed validation limit, not a demonstrated render-surface defect; no timeout increase is recommended.
42. **Other operating systems.** Deferred Windows, Linux, and WSL2 execution to CI because the planned transport creates a commit; did not repeat dependent compile checks after the final test-only phase. Cross-OS proof does not block this review, and no commit is authorized here.

Intermediate deferrals for flag removal, actual style application, no-input terminal wiring, input matrices, and terminal-suite execution were completed in later phases; they are not outstanding scope. The remaining reporting omissions have findings above, rather than being reserved for an author decision. No follow-up work is moved to another spec, and no feature directory is moved to completed.
