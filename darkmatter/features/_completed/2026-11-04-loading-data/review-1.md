---
$schema: feature-review.yaml
ready: false
findings:
    - "R1-F1 (high): Lifecycle file writes leave later steps reading cached data"
    - "R1-F2 (high): Collection and shell expressions are frozen before earlier commands run"
    - "R1-F3 (high): Failed schema expansion drops dependencies already read"
    - "R1-F4 (high): Several malformed YAML paths omit required source positions"
    - "R1-F5 (high): Allowed-host transport failures omit structured reference details"
    - "R1-F6 (high): Quoted bracket unions bypass the required container diagnostic"
human_review: true
human_review_items:
    - "Kept the plan’s old line-number citations and recorded corrected locations in the implementation log; no product change was needed in the initial audit."
    - "Accepted lone carriage returns as CSV/TSV record separators while counting diagnostic lines by LF; retain this documented compatibility choice rather than expand this review over an uncommon input."
    - "Defined blank JSONL lines using JSON whitespace (space, tab, and CR); retain that interpretation rather than silently admit other characters."
    - "Classified JSONL, CSV, and TSV as structured UTF-8 formats; retain these format classifications."
    - "Distinguished an absent header from a header with no data rows internally and retained a defensive reader-synchronization error; both empty forms still load as an empty list."
    - "Returned structured failure records from shared validation, treated unresolved recursive references provisionally, and allowed the container option in nested property declarations; retain these choices."
    - "Extended frontmatter failure reporting to descend into one reachable union arm and validate a partially converted copy, while keeping write-back atomic; this implements the authorized shared-validation repair."
    - "Enabled the new delimited-reader feature in biscuit-file’s local test and lint recipes so its declared tests actually run; retain this extension to the plan."
    - "Restarted unfinished editor regions at a new top-level expression opener and omitted the complete-expression parse gate there; retain the bounded completion ambiguity after broken prose."
    - "Allowed the scanner corpus test to expose an additional legacy span after repairing an earlier span; retain the precise invariant that previously valid spans remain unchanged and replacements parse."
    - "Placed root locations at the first token except TOML at byte zero, relied on prior parsing for JSON escape validity, decoded escaped JSON5 keys, and used json-five’s byte error index directly; retain these locator choices."
    - "Allowed existing YAML-envelope callers to gain list/scalar-root positions through the shared position map; frontmatter still rejects non-mapping roots before this point."
    - "Cleared prepared imported contracts alongside cached data after commands, preventing an updated schema from retaining its old interpretation; retain this invalidation rule."
    - "Kept authored contract text in schema errors, boxed large preparation errors, and classified missing/unreadable imports as file-reference failures; retain this internal error structure."
    - "Normalized missing file_exists candidates through their existing parent directories so missing and later-existing files use consistent path spelling; retain this observation behavior."
    - "Exposed read sets on every root composition report, made an unused shortcut constructor test-only, and added a condition evaluator accepting the shared request context; retain this integration."
    - "Initially deferred validate_schema schema dependencies to a later implementation phase; that gap was repaired in Phase 7 and is no longer deferred."
    - "Preserved Claudine’s previously accepted whole-value interpretation before using the shared scanner as fallback, made complete scanner regions opaque, and retained existing literal/token closing behavior; retain these compatibility choices."
    - "Added a binding-error refinement hook so computed invalid contracts receive the same data-specific errors as literal contracts; retain the mechanism."
    - "Left quoted bracket-union contracts on generic errors because the shared string parser rejects them; repair the explicit acceptance gap described in R1-F6, the bracket-union diagnostic finding."
    - "Fetched through the existing remote runtime before consulting cached parsed data so policy, transport reuse, and denied-host checks remain authoritative; retain this ordering."
    - "Classified rejected numeric spellings as malformed data, checked JSON5 spellings before rounding, and rejected YAML tags only on the data-loading path; retain these parsing choices."
    - "Cached parsed CSV/TSV tables independently of their contracts, built object/list rows during binding, retained raw positions in errors, and boxed remote identities; retain this separation."
    - "Expanded shipped imports passively only when no file import is present, left unknown shipped names to execution, preserved multiple argument errors in argument order, and projected named types to unknown without erasing known sibling properties; retain these bounded static-check choices."
    - "Added load_data to lifecycle verb lists and Claudine’s mirrored catalogs as well as the main function catalog; retain this function-registration parity."
    - "Introduced a distinct typed denied-host failure and inferred it through a passive policy check rather than redesign the transport cache; retain the classification and preserved message wording."
    - "Filled structured reference fields for all file-reference expression errors, including absolute and relative, while omitting a reason where existing vocabulary supplies none; retain that projection without inventing unrelated phrases."
    - "Used per-location data-mismatch messages, preserved data codes across callbacks and nested composition, and retained typed graph-command failures while keeping existing stderr wording; retain these reporting choices."
    - "Ordered failures by authored position, then unlocated and missing values, used the header for a CSV root location, suppressed whole-file YAML positions where required, and reparsed per distinct pointer; retain correctness choices and consider the measured locator cost described below."
    - "Updated all 17 shared diagnostic schemas, used existing numeric coordinate types, added read sets to compose/render envelopes, and read the final dependency set when emitting output; retain this common-record integration."
    - "Kept the existing Markdown-mode error-block layout rather than add structured data line/column display to stderr; the spec does not require that additional presentation."
    - "Reused catalog-selected type-definition editor routines, withheld document-local types and host catalogs, limited unfinished-call recovery to the cursor’s line, and added callback-member suggestions from shared typing; retain these editor choices."
    - "Used the existing load_data_* test-module names rather than introduce nearly identical loading_data_* names; retain the test layout."
    - "Cleared cached data after every nonempty lifecycle event, including guarded-off and informational stacks; replace this broad end-of-event rule with executed-step invalidation under R1-F1, the stale lifecycle-read finding."
    - "Deferred later body and top-level-shell reads that observe earlier file changes as possible future shell-stage work; complete the specified ordering in this feature under R1-F2, the eager-read finding."
    - "Added schema dependencies to the public preparation summary so validate_schema can report transitive reads; retain the added field on the existing non-exhaustive type."
    - "Used the parser’s mapping-start location for duplicate YAML keys and omitted unavailable locations for second documents and non-string keys; keep proven duplicate positions and repair missing positions under R1-F4, the malformed-input-location finding."
    - "Embedded real catalog example-data files, wrote them into temporary example workspaces, and printed their contents in generated documentation; retain this shared example source."
    - "Added a general catalog function-page generator with a page list and separate write modes instead of a load_data-only renderer; retain this small reusable documentation surface."
    - "Added newcomer type-definition examples and a container section in Darkmatter’s page while linking to grammar material already moved into simplified-schema; retain this documentation ownership."
    - "Recovered failed-import dependencies from final schema errors but deferred dependencies read earlier in the failed expansion; complete that work under R1-F3, the incomplete-read-set finding."
    - "Documented remaining differences instead of rewriting the spec: repair transport reference fields and bracket-union errors under R1-F5 and R1-F6; retain bare-name rejection, whole-file YAML position suppression, and exclusion of unresolved data targets as consistent with the stated contract."
    - "Removed stale spec references, repaired nearby documentation links and scanner prose, and corrected the README hash command; retain these encountered documentation repairs."
    - "Filed the four authorized active follow-ups for expression recovery, tabular input options, member-property checking, and YAML locator hardening, and corrected the future dynamic-schema example; these remain explicitly outside this feature."
    - "Left editor invalidation from read sets to the later dynamic-schema work, as the spec directs; no editor dependency watcher is required here."
    - "Left final Linux, native-Windows, and WSL2 evidence to CI rather than use a cross-check command that creates a commit; this does not block readiness."
    - "Used a relative-link/anchor script when documentation reference validation tried to execute inline examples, including unchanged examples; retain this bounded documentation verification method."
    - "Updated Claudine’s expression-list snapshot when the final full suite exposed its missing load_data entry and recorded that dependency in the skill; retain the corrected snapshot."
    - "R1-F2 repair: body expressions calling a read-side function that follow a command now wait and run after it (alternatives: defer every later expression, or only load_data); confirm the scope."
    - "R1-F2 repair: a command whose arguments changed after an earlier command runs under pre-approvals only via an exact whitelist rule or an approved template shape, else NotPreApproved (alternative: strict pre-flight membership); confirm."
    - "R1-F3 repair: failed-expansion reads come from the resolver's existing thread-local read observer rather than a partial-dependencies error wrapper; PreparationSummary::dependencies is retained but no longer consumed by validate_schema."
    - "R1-F4 repair: YAML refusal positions come from a second serde_yaml_ng event pass (yaml_documents/marks.rs); an aliased non-string key reports its anchor's position; the first bad key is reported in document order."
    - "R1-F4 sweep: biscuit-file json5_parse_error_offset now also locates parsed-but-unreadable JSON5 values (oversized/negative/hex integers, bad escapes); this extends a public biscuit-file contract."
    - "R1-F5 repair: allowed-host fetch and UTF-8 failures project reason unreachable and failure io (existing vocabulary), sharing one classification; file_exists(url) without a remote runtime now uses the typed message 'remote reads are not enabled'."
    - "R1-F5.12 decision: validate_schema schema/import file read failures are also typed (SchemaFileRead) with the four reference fields, unchanged message and code (alternative: leave untyped because R5 names only the argument file)."
    - "R1-F6 repair: simplified_schema::parse_type_definition_value reads a quoted bracket list as a YAML-sequence union for definition values only (schema declarations unchanged); items containing [] must be quoted; non-string carriers report canonical contract text."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-11-07T08:54:26-08:00"
spec: 2026-11-04-loading-data/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-04-loading-data/log.md
description: "A **feature** review of `2026-11-04-loading-data/spec.md`"
feature: 2026-11-04-loading-data/review-1.md
next: 2026-11-04-loading-data/review-2.md
---
# Loading Data — Review 1

The feature is **not production ready**. Six findings remain. All are unblocked: the spec supplies a direction for repair, so none requires an author ruling. There is no previous implementation review to carry forward.

The review covered the finalized spec and three spikes, the implementation log and plan, data parsing and binding, shared schema changes, command-wide caching and dependencies, lifecycle integration, diagnostic projection, CLI output schemas, editor providers, scanner consumers, tests, and current documentation. The starting working tree was clean. Production code was not edited during this review.

## Findings

### R1-F1 (high): Lifecycle file writes leave later steps reading cached data

**Authority: spec R7.** “The memo is emptied each time a command has run: … or a lifecycle step” and “Invalidate after each executed command attempt … before any next expression.” An ordinary lifecycle that reads a file, appends a record, and reads it again returns the old count.

**Class:** invalidation occurs after a group of operations instead of immediately after the operation that can change a dependency.

In the darkmatter package, [md_host::evaluate](../../lib/src/lifecycle/md_host.rs) clears cached data only after `StackExecutor::execute_event` returns (lines 733–773). The shared [side-effect dispatcher](../../lib/src/lifecycle/executor.rs) can write a file and continue to the next expression without this hook. The event-end clearing also discards successful reads for a stack whose guarded actions never ran.

Reproduction, with `d.yaml` initially containing `- a\n`:

```yaml
initialize:
  - info: "before {{ length(load_data('d.yaml', 'string[]')) }}"
  - append_line: [d.yaml, "- b"]
  - info: "after {{ length(load_data('d.yaml', 'string[]')) }}"
```

Put this frontmatter above `body` in `doc.md`, then run `md compose doc.md --output json`. It exits successfully, writes both records, and emits lifecycle messages `before 1`, **`after 1`**. The second message must be `after 2`.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | Shared non-shell lifecycle dispatch, `append_line` | Read/write/read in initialize, composed, finalize, and failed events | Every event reports `after 1`; disk has two records | Later expression reads two records |
| R1-F1.2 | Same dispatch, `append_jsonl` | Start with `"one"` as one JSONL record; append `two`; read with `string[]`, in initialize/composed/finalize | `after 1` after the second record is written | `after 2` |
| R1-F1.3 | Lifecycle shell runner | Executed shell attempt | Clears the shared runtime immediately; code places invalidation before inspecting the outcome | Clean: failures that may have written also invalidate |
| R1-F1.4 | Frontmatter and body shell execution | Existing rewrite and failed-write regressions | Focused tests observe fresh reads after executed attempts | Clean |
| R1-F1.5 | Lifecycle event completion | Nonempty stack with all actions guarded off | Code clears solely because the authored stack is nonempty | Preserve reuse when no step executes |

The non-shell dispatch is one shared site for `append_line`, `append_jsonl`, `ensure_file`, `ensure_dir`, all seven frontmatter mutation verbs, and `http_post`; there is no intervening invalidation hook for any of them. The four tested events and the cancellation event all use the same `md_host::evaluate` implementation. Remote transport snapshots still obey R7’s explicit exception; this repair must not force remote refetching.

**Repair:** notify the request runtime after each actually dispatched lifecycle step that can change files, including a failed attempt, before another expression runs. Use that shared dispatch boundary rather than special-case `append_line`, and remove the unconditional end-of-event substitute. Add a public-API regression for read/write/read, a failed writing attempt, and guarded-off steps. The current bootstrap/initialize test checks only a read after the whole event and misses this case.

### R1-F2 (high): Collection and shell expressions are frozen before earlier commands run

**Authority: spec R7 acceptance criterion.** “A shell step inside a `::map` body causes the next item's call to read again.” The implementer explicitly deferred the plain-body case, but the requirement does not limit it to expressions inside command templates.

**Class:** reads for later execution positions are evaluated before an earlier file-changing command, so invalidating the cache cannot refresh their already-rendered values.

The darkmatter package expands collection regions and interpolates their bodies before shell execution in [pipeline phases](../../lib/src/markdown/compose/pipeline/phases.rs), [collection expansion](../../lib/src/markdown/compose/inline/collection_blocks.rs), and [body interpolation](../../lib/src/markdown/compose/inline/interpolation.rs). A later top-level command can likewise retain an earlier resolved argument through [shell preparation](../../lib/src/markdown/compose/inline/shell_expansion.rs).

Use `d.toml` containing `[a]\nb = 1\n` and approve `git config --file d.toml a.b 2` in the fixture’s `.darkmatter-shell-whitelist`:

```markdown
---
$schema:
  items: number[]
items: [10, 20]
---
::map items "i => i"
value: {{ load_data('d.toml', '{ a: { b: number } }').a.b }}
::shell git config --file d.toml a.b 2

::end-map
```

`md compose doc.md --output json` succeeds with **`value: 1` twice**, though the first iteration’s command sets the value to 2. The second iteration must see 2.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Plain text inside `::map` and `::filter` bodies | Reproduction above; repeat with `::filter items "i => true"` / `::end-filter` | Both produce `1`, then `1` | `1`, then `2` |
| R1-F2.2 | Later top-level `::shell` argument | First command writes `d.toml` value 2; next sets `result.toml` from `load_data(...).a.b`; approve both possible result commands | Result file contains 1 | Later command uses 2 |
| R1-F2.3 | Per-item command-template filling | Existing `a_shell_step_inside_a_map_body_makes_the_next_item_read_again` | Starts at 1; adds 10 and then 20; final value 31, reads twice | Clean |
| R1-F2.4 | Frontmatter command followed by body read | Existing `a_frontmatter_command_that_rewrites_the_data_is_read_again` | Body sees rewritten data | Clean |
| R1-F2.5 | Failed command followed by next item’s template | Existing `a_failed_command_that_wrote_a_file_ends_reuse` | Next template sees the write despite failure | Clean |

Both collection forms returned the same stale output in the CLI reproduction. These are the distinct changed read/execution boundaries: collection/plain interpolation, resolved top-level commands, late command templates, and frontmatter execution. A read intentionally performed before a command may retain its old value; this finding concerns the *next iteration or command’s* reads.

**Repair:** preserve the R7 execution ordering for these dependent reads and fills, rather than merely clear cached entries after their results have been substituted. Keep approval checks and inserted-data protection intact; a refreshed command must still be approved. Add the two failing public-API cases alongside the existing command-template control. This is an existing-stage integration change, but moving the requirement to a future spec would leave an explicit acceptance criterion unfinished.

### R1-F3 (high): Failed schema expansion drops dependencies already read

**Authority: spec R8.** “Record schema files actually read while resolving a `load_data` contract or running `validate_schema`, including transitive imports,” and failed envelopes retain those reads. Both functions lose dependencies when a later import fails.

**Class:** dependency collection is attached to a successful return value, so earlier reads disappear on an error return.

The darkmatter package’s [DataRuntime::prepare_contract](../../lib/src/markdown/compose/expression/data/runtime.rs) reconstructs dependencies from the final error only. [Contract preparation](../../lib/src/markdown/compose/expression/data/contract.rs) may also convert that error into a file-reference cause, losing earlier imported paths. [validate_schema_fn](../../lib/src/markdown/compose/expression/functions/fs.rs) records `PreparationSummary::dependencies` only after `validate_summarized` succeeds.

Create `good.yaml` exporting `Good: string` and `bad.yaml` exporting `Other: string`, each under `kind: schema` / `types`. Then compose:

```markdown
{{ load_data('data.json', '{ a: Good@./good.yaml, b: Missing@./bad.yaml }') }}
```

The resolver visits the authored properties in order. It reads `good.yaml` and then reads but rejects `bad.yaml`. The failed envelope lists **only `bad.yaml`**. Replacing the second import with a missing file yields **an empty read set**, dropping the successfully read first import too.

| Child ID | Site | Shape tested | Observed read set, filenames only | Expected |
|---|---|---|---|---|
| R1-F3.1 | `load_data`, successful resolution | Only `a: Good@./good.yaml`, data `{}` | `data.json`, `good.yaml` | Clean |
| R1-F3.2 | `load_data`, rejected later export | `Good` import, then `Missing@./bad.yaml` | `bad.yaml` | `good.yaml`, `bad.yaml`; no data read yet |
| R1-F3.3 | `load_data`, missing later file | `Good` import, then `Missing@./absent.yaml` | Empty | `good.yaml`; invent no absent target |
| R1-F3.4 | `validate_schema`, successful resolution | Subject frontmatter imports `Good` | `subject.md`, `good.yaml` | Clean |
| R1-F3.5 | `validate_schema`, failed resolution | Subject imports `Good`, then missing export in `bad.yaml` | `subject.md` only | Subject plus both schema files |
| R1-F3.6 | Both functions, successful transitive/embedded imports | Existing transitive-import and embedded-schema tests | All host imports retained; embedded schemas add no path | Clean |
| R1-F3.7 | `load_data`, failed transitive expansion | `Outer@./outer.yaml`; its definition imports `Good`, then `Missing` | `bad.yaml` only | `outer.yaml`, `good.yaml`, `bad.yaml` |
| R1-F3.8 | `validate_schema`, failed transitive expansion | Subject imports the same `Outer` | `subject.md` only | Subject plus all three schema files |

The successful contract-import list, its error-path reconstruction, the file-reference error conversion, and the validation-summary consumer are all the dependency projections in this boundary. Repeated cached failures currently retain the same incomplete dependency list; replaying the final error cannot recover lost reads.

**Repair:** retain actual resolver read provenance on failure as well as success and pass it to both expression functions, including errors converted to file-reference failures. A small partial-dependencies error wrapper or read observer is sufficient; do not rescan imports or add paths that were never read. Test a successful import followed by a failed one through each function, including a transitive failure and a repeated failed call. This is the work the log explicitly left unfinished, not a request for a new dependency watcher.

### R1-F4 (high): Several malformed YAML paths omit required source positions

**Authority: spec R3 acceptance criteria** require every malformed-input row to name its position; **R5** limits pointer-only YAML to contract violations in successfully parsed data, explicitly preserving malformed-input locations even when aliases occur. A second YAML document, non-string mapping keys, and a custom tag in a file containing an alias violate that contract.

**Class:** semantic parse refusals have no parser span and fall through to an empty location; the successfully parsed tree’s unavailable position map also suppresses malformed-input positions.

The darkmatter package’s [whole-YAML loader](../../lib/src/markdown/schemas/yaml_documents/load.rs) relies on an optional parser location or projected mapping range. [parse_yaml](../../lib/src/markdown/compose/expression/data/parse.rs) substitutes a default location when neither exists. Its custom-tag check relies on the same position map, which refuses aliases.

Run `load_data('data.yaml', 'object')` through `md compose doc.md --output json`. Every failing shape below returns `dm.data.malformed`, no document, and the data file in `read_set`; the defect is missing coordinates, not accepting invalid data.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F4.1 | YAML document count | `a: ok\n---\na: next\n` | File only | Located second-document refusal |
| R1-F4.2 | YAML non-string keys | Root `42: text`; nested numeric, boolean, and explicit sequence keys; numeric key after an alias | File only in every case | Proven location of offending key or containing mapping |
| R1-F4.3 | YAML custom-tag check with alias | `base: &b ok\na: *b\nb: !tag text\n` | File and `/b`, no coordinates | Tag position, line 3 column 4 |
| R1-F4.4 | YAML custom-tag controls | Root tag, nested tag, multiline-flow tag without alias | Proven positions, including line 2 column 3 for flow tag | Clean |
| R1-F4.5 | YAML parser failure and duplicates | Unclosed sequence, with/without alias; repeated `a` key | Located parser errors; duplicate points at mapping start | Clean: use proven parser position |
| R1-F4.6 | YAML numeric admission | Inexact integer, including after alias; nonfinite value after alias | Located malformed errors at numeric token | Clean |
| R1-F4.7 | JSON loader | Invalid token, trailing garbage, empty document; duplicate-key control | Located errors; duplicate is last-wins | Clean |
| R1-F4.8 | JSONL/NDJSON shared loader | Bad token/trailing content in record; blank-line and duplicate controls | Located errors; blanks skipped, duplicates last-wins within record | Clean |
| R1-F4.9 | JSON5 loader | Bad value, trailing garbage, empty document; comments/trailing-comma control | Located errors; permitted syntax accepted | Clean |
| R1-F4.10 | TOML loader | Missing value, duplicate key, trailing invalid statement; empty control | Located errors; empty is `{}` | Clean |
| R1-F4.11 | CSV/TSV shared reader plus object-header check | Unclosed quote, ragged row, duplicate header; quoted multiline control | Located errors for both delimiters; valid quoted cells accepted | Clean |
| R1-F4.12 | Local byte decoder | Invalid UTF-8 | Original-byte offset without invented line/column | Clean |

The YAML column was also checked against valid controls, blank/null documents, anchors, custom tags, duplicate keys, invalid trailing syntax, non-string keys, and a second document. The other loaders’ corresponding duplicate/trailing/format-specific states are listed under the robustness matrix below. No additional location defect was observed there.

**Repair:** preserve token/event marks for these semantic YAML refusals and use them for the data error. Do not invent a location or remove the spec’s intentionally pointer-only contract-mismatch rule. The smallest repair needs document-start/key/tag marks, not general alias-aware value-tree location. Add public-result tests for all three failing rows; the existing malformed-YAML execution tests mostly check the error category, so they pass without the required coordinates. Update any comments promising located errors to match the repaired behavior.

### R1-F5 (high): Allowed-host transport failures omit structured reference details

**Authority: spec R5.** A “failed fetch” stays `dm.expression.evaluation_failure` and gains `reason`, `construct`, `reference`, and `failure`; “The rule covers every read-side expression function.” The implementation documents an exception for allowed-host fetch/UTF-8 errors which the spec does not authorize.

**Class:** a typed read failure is flattened to a message before diagnostic projection, dropping its reference identity and classification.

In the darkmatter package, [remote_text_error](../../lib/src/markdown/compose/expression/functions/fs.rs), line 75, converts `RemoteTextError::Failed` to `ExpressionError::Other`. [load_data_fn](../../lib/src/markdown/compose/expression/data/load.rs) and the other read functions share that helper. [Diagnostic projection](../../lib/src/markdown/compose/report_diagnostics.rs) can attach reference fields only when the typed cause survives.

A loopback HTTP fixture was allowed with `--allow host=127.0.0.1`; one route returned HTTP 500 and another returned HTTP 200 with byte `0xff`. For example, `load_data('http://127.0.0.1:PORT/fail.json', 'object')` exits 1 with the right evaluation code but **none of the four reference fields**. Invalid UTF-8 has the same omission. Neither should acquire `data_location`, because both fail at transport.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F5.1 | `load_data` | HTTP 500; invalid UTF-8 | Four reference fields absent | Typed reference details, unchanged code/message, no data location |
| R1-F5.2 | `frontmatter` | Same two responses | Same omission | Same rule, construct `frontmatter()` |
| R1-F5.3 | `file_exists` | Same two responses | Same omission | Same rule, construct `file_exists()` |
| R1-F5.4 | `markdown_title` | Same two responses | Same omission | Same rule, construct `markdown_title()` |
| R1-F5.5 | `markdown_body_empty` | Same two responses | Same omission | Same rule, construct `markdown_body_empty()` |
| R1-F5.6 | `validate_schema` | Same two responses | Same omission | Same rule, construct `validate_schema()` |
| R1-F5.7 | `try_frontmatter` | Same two responses | Successful `{ok:false,...}` recovery value; no failure diagnostic | Clean: preserve existing recovery behavior |
| R1-F5.8 | Shared denied-host/missing-file projection | Existing per-function denial/missing-file tests | Reference identity and failure classification retained | Clean |

This covers all seven members of `REMOTE_READ_FUNCTIONS`, the shared remote-error adapter, and its final projection. For each failing function both response shapes were tried; the read set retained the URL and stderr remained empty in JSON mode.

**Repair:** preserve a typed transport cause with the function and authored reference through the shared adapter, and fill the existing reference fields using the existing failure vocabulary. Keep existing human messages and the evaluation code; do not misclassify a contacted host as policy-denied or create a new `dm.data.*` code. Test HTTP failure and UTF-8 failure through the public library result once at the shared adapter, with a control and one wiring case per caller; retain `try_frontmatter`’s intentional recovery. Correct the documented exception when repaired.

### R1-F6 (high): Quoted bracket unions bypass the required container diagnostic

**Authority: spec R2 acceptance criterion.** “`\"[string, number]\"` is rejected (a union of scalars)” with `dm.data.not_a_container` and the `as` argument span. R2 also requires a computed scalar contract to fail with the same code. The implementation instead treats this explicitly specified type spelling as malformed syntax.

**Class:** one supported type carrier takes a different parser path before the shared semantic container check, producing inconsistent public errors.

In the darkmatter package, [judge_written_contract](../../lib/src/markdown/compose/expression/data/contract.rs) passes the entire quoted bracket text to the scalar-string type parser. [refine_binding_error](../../lib/src/markdown/compose/expression/data/load.rs) only replaces the generic error if that parse already succeeded. The simplified-schema package already interprets a YAML sequence as a union; the missing route is the quoted definition used by this API.

| Child ID | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F6.1 | Literal passive call check | `load_data('data.json', '[string, number]')` | `dm.expression.incompatible_call`, no reads | `dm.data.not_a_container` at `as`, no reads |
| R1-F6.2 | Computed argument binding | Frontmatter `t: "[string, number]"`, then `load_data('data.json', t)` | `dm.expression.evaluation_failure`, no reads | `dm.data.not_a_container`, no reads |
| R1-F6.3 | Literal passive call check | Scalar `string`; textual union `string | number` | Correct container error | Clean |
| R1-F6.4 | Computed argument binding | Same scalar and textual union | Correct container error | Clean |
| R1-F6.5 | Shared union/container predicate | Existing YAML-sequence union test | Recognizes union and rejects its scalar arms | Clean |
| R1-F6.6 | Literal/computed execution controls | Existing valid array/object/tuple contracts and null-file tests | Valid values load; null reads neither data nor imports | Clean |

These are the literal check, ordinary binding/refinement, execution preparation, and shared predicate paths that decide this classification. The current test `a_bracketed_scalar_union_is_rejected_as_malformed_syntax` asserts the departure, so its passing result does not establish acceptance.

**Repair:** route the quoted bracket-union form through the existing union/type-definition interpretation before judging container eligibility, consistently for literal and computed contracts. Keep genuinely malformed type text on its existing generic error path. Replace the contrary test expectation and add the computed case. Do not rewrite the spec or require new unquoted expression syntax.

## Verification and requirement coverage

All verification performed here was **Level 1**: public library results, the real shipped CLI with isolated temporary inputs, and in-process language-server protocol requests. This feature adds no keyboard, focus, terminal glyph/style, or scrolling requirement; Level 2/3 terminal evidence would not answer an additional acceptance question. Editor behavior is verified at its protocol boundary and needs no focused desktop window.

| Requirements | Strongest relevant evidence inspected/run | Result |
|---|---|---|
| R1–R2: signature, null, imports, containers | `load_data_execution`, `load_data_passive`, `container_contracts`, editor protocol tests | Pass apart from R1-F6 |
| R3–R4: formats, numeric admission, strict tabular input | Public composition matrices, per-loader CLI controls/failures, reader tests inspected | Parsing outcomes pass; positions have R1-F4 |
| R5: mismatch pointers, order, unions, cap, reference fields | `load_data_diagnostics`, CLI envelope conformance cases, manual error-class sweeps | Contract records pass; R1-F4 and R1-F5 remain |
| R6: host policy and remote reads | Mock-server success, denial, cache modes, query/fragment, and invalid-response cases | Authorization controls pass; failed-response projection has R1-F5 |
| R7: shared reads, commands, invalidation | Public runtime counters, map/template tests, failed command and generation tests; CLI read/write/read reproductions | Basic reuse passes; R1-F1 and R1-F2 remain |
| R8: read set on every surface and failure | Body/frontmatter/condition/callback/target/lifecycle tests and failed-import sweep | Surface wiring passes; failed expansion has R1-F3 |
| R9: optional-file pattern | Missing/present/malformed guard and non-recovering `||` cases | Pass |
| R10: literal result typing | Static type tests and 11 editor protocol tests | Pass within stated named-reference/member-check limits |
| R11: scanner and consumers | Frozen scanner corpus; consumer regressions inspected in Darkmatter/Claudine | Pass; existing accepted malformed spans remain unchanged |
| R12: catalog/docs | Generated-page assertions in focused run; topic, data-contract, and schema documentation inspected | Required pages exist; repair documented exceptions with findings |

Commands run successfully:

- `just test load_data` in darkmatter: **279 passed**, no selected test at or over 5 seconds.
- `just test type_definition_arguments` in darkmatter: **11 passed**.
- `just test data_origin` in darkmatter: **18 passed**.
- `just test scanner_corpus` in simplified-schema: the corpus test passed; the recipe also ran its full no-default-features suite, **1,359 passed**.
- `just test container_contracts` in simplified-schema: **16 passed**; its recipe again ran the no-default-features suite, **1,359 passed**.
- `just check-tier-coverage darkmatter`: **0 stranded**; new library, CLI, and editor test modules are declared by their consolidated targets.

The first attempt to pass a compound expression through `just test -E` failed in shell argument forwarding before running tests; the successful commands above used simple filters. Linker output included the existing macOS large unwind-section warning. No source edits or formatter run were needed. The implementation log’s broader full-suite/lint evidence was inspected, not claimed as newly rerun. Cross-OS proof is left to CI as requested.

## Input robustness matrix

Default: malformed syntax and values failing the declared contract return located errors, stop composition, and publish no document. Exceptions defined by the spec are preserved: optional object members may remain absent or explicitly null; compatible scalar text converts; empty valid collections remain empty; JSON duplicate keys are last-wins; null/empty documents follow each format’s stated rules. The only new exceptions requiring repair are in the findings above.

The data files have caller-defined fields, rather than a fixed application configuration schema. Their load-bearing validators are the shared object-property, array-element, tuple-slot, dictionary-value, and union binders. The public composition tests in `load_data_execution` exercise absent versus null required/optional members, a wrong whole-field type, mixed/all-invalid elements, null elements, tuple slot absence/null/type errors, dictionary-value failure/conversion, explicit empties, and refusal to publish a partial result. These checks occur once per shared validator rather than being repeated for each format.

The document-loader sweep used a valid control for every format, then changed one shape at a time. It produced this result through `md compose --output json`:

| Shared loader | Duplicate keys/headers | Invalid trailing content | Format-specific syntax and empty control |
|---|---|---|---|
| YAML/YML | Rejects with parser location | Rejects with location | Anchors expand; empty becomes null; second document/non-string keys/tag-with-alias have R1-F4 |
| JSON | Last value wins | Rejects with location | Empty rejects; native null accepted by nullable container contract |
| JSONL/NDJSON | Last value wins within each JSON record | Rejects invalid record with position | Blank JSON-whitespace lines skipped; empty becomes `[]`; null record remains data and fails `object[]` |
| JSON5 | Last value wins in observed control; spec does not forbid it | Rejects with location | Comments/trailing commas accepted; empty rejects; native null preserved |
| TOML | Rejects with location | Rejects invalid statement with location | Empty becomes `{}`; numeric/nonfinite rejection inspected in public-result tests |
| CSV/TSV | Object mode rejects duplicate/empty headers; list mode skips them | Ragged record/quote violations reject with location | Quoted delimiters/newlines accepted; blank/empty input becomes `[]`; headers/cells are not trimmed |

The field/document intersection in R1-F4 was swept for the entire YAML column and all corresponding loader rejection paths. R1-F6 sweeps both literal and computed carrier rows, including their scalar and union controls. Neither finding is a request to add a CLI-spawn matrix for every field: repairs should use the public library result and a small number of CLI wiring checks.

The requested code-smell search covered the new data parser, contract preparation, row construction, JSONL reader, and delimited reader. No load-bearing `#[serde(default)]` or `filter_map(as_str)` path silently drops data. `headers: Option<_>` deliberately distinguishes empty input from a header; optional scalar fields follow the shared schema contract. The consequential `unwrap_or_default()` location fallbacks are captured by R1-F4. Error-only schema dependency collection is captured by R1-F3.

## Bounded performance observation

The log explicitly chose to reparse source text per distinct diagnostic pointer in the darkmatter package’s [SourceLocator](../../lib/src/markdown/compose/expression/data/locate.rs). CLI measurements for 100 versus 1,000 rows with one bad `port` field were approximately: JSON 0.34/0.64 seconds, JSONL 0.22/1.76, YAML 0.22/0.29, TOML 0.25/4.45, JSON5 0.24/2.21, CSV 0.22/0.24, TSV 0.21/0.24. Each emitted the required 20 records plus overflow. These are debug-build observations including startup, not performance guarantees.

This supports the implementer’s stated optimization opportunity: build a reusable source index or postpone full location work until after selecting the reported records, while retaining authored ordering and the overflow count. No additional readiness finding is imposed from timing alone: the spec promises no fixed large-input bound, and this observation establishes cost rather than a wrong public result.

## Implementation decisions (informational)

Two independent agents read the implementation log and spec in parallel. The list below accounts for all 43 entries in `decisions_for_review`, including their component choices, plus the closure decisions and authorized deferrals. The author may confirm or reverse these choices after the repair cycle; no response is required to continue. Items with a finding are recorded here for visibility, not approved as departures from the spec. Every recommended repair stays within the existing decisions, so no human ruling is needed.

1. Kept the plan’s old line-number citations and recorded corrected locations in the implementation log; no product change was needed in the initial audit.
2. Accepted lone carriage returns as CSV/TSV record separators while counting diagnostic lines by LF; retain this documented compatibility choice rather than expand this review over an uncommon input.
3. Defined blank JSONL lines using JSON whitespace (space, tab, and CR); retain that interpretation rather than silently admit other characters.
4. Classified JSONL, CSV, and TSV as structured UTF-8 formats; retain these format classifications.
5. Distinguished an absent header from a header with no data rows internally and retained a defensive reader-synchronization error; both empty forms still load as an empty list.
6. Returned structured failure records from shared validation, treated unresolved recursive references provisionally, and allowed the container option in nested property declarations; retain these choices.
7. Extended frontmatter failure reporting to descend into one reachable union arm and validate a partially converted copy, while keeping write-back atomic; this implements the authorized shared-validation repair.
8. Enabled the new delimited-reader feature in biscuit-file’s local test and lint recipes so its declared tests actually run; retain this extension to the plan.
9. Restarted unfinished editor regions at a new top-level expression opener and omitted the complete-expression parse gate there; retain the bounded completion ambiguity after broken prose.
10. Allowed the scanner corpus test to expose an additional legacy span after repairing an earlier span; retain the precise invariant that previously valid spans remain unchanged and replacements parse.
11. Placed root locations at the first token except TOML at byte zero, relied on prior parsing for JSON escape validity, decoded escaped JSON5 keys, and used json-five’s byte error index directly; retain these locator choices.
12. Allowed existing YAML-envelope callers to gain list/scalar-root positions through the shared position map; frontmatter still rejects non-mapping roots before this point.
13. Cleared prepared imported contracts alongside cached data after commands, preventing an updated schema from retaining its old interpretation; retain this invalidation rule.
14. Kept authored contract text in schema errors, boxed large preparation errors, and classified missing/unreadable imports as file-reference failures; retain this internal error structure.
15. Normalized missing file_exists candidates through their existing parent directories so missing and later-existing files use consistent path spelling; retain this observation behavior.
16. Exposed read sets on every root composition report, made an unused shortcut constructor test-only, and added a condition evaluator accepting the shared request context; retain this integration.
17. Initially deferred validate_schema schema dependencies to a later implementation phase; that gap was repaired in Phase 7 and is no longer deferred.
18. Preserved Claudine’s previously accepted whole-value interpretation before using the shared scanner as fallback, made complete scanner regions opaque, and retained existing literal/token closing behavior; retain these compatibility choices.
19. Added a binding-error refinement hook so computed invalid contracts receive the same data-specific errors as literal contracts; retain the mechanism.
20. Left quoted bracket-union contracts on generic errors because the shared string parser rejects them; repair the explicit acceptance gap described in R1-F6, the bracket-union diagnostic finding.
21. Fetched through the existing remote runtime before consulting cached parsed data so policy, transport reuse, and denied-host checks remain authoritative; retain this ordering.
22. Classified rejected numeric spellings as malformed data, checked JSON5 spellings before rounding, and rejected YAML tags only on the data-loading path; retain these parsing choices.
23. Cached parsed CSV/TSV tables independently of their contracts, built object/list rows during binding, retained raw positions in errors, and boxed remote identities; retain this separation.
24. Expanded shipped imports passively only when no file import is present, left unknown shipped names to execution, preserved multiple argument errors in argument order, and projected named types to unknown without erasing known sibling properties; retain these bounded static-check choices.
25. Added load_data to lifecycle verb lists and Claudine’s mirrored catalogs as well as the main function catalog; retain this function-registration parity.
26. Introduced a distinct typed denied-host failure and inferred it through a passive policy check rather than redesign the transport cache; retain the classification and preserved message wording.
27. Filled structured reference fields for all file-reference expression errors, including absolute and relative, while omitting a reason where existing vocabulary supplies none; retain that projection without inventing unrelated phrases.
28. Used per-location data-mismatch messages, preserved data codes across callbacks and nested composition, and retained typed graph-command failures while keeping existing stderr wording; retain these reporting choices.
29. Ordered failures by authored position, then unlocated and missing values, used the header for a CSV root location, suppressed whole-file YAML positions where required, and reparsed per distinct pointer; retain correctness choices and consider the measured locator cost described below.
30. Updated all 17 shared diagnostic schemas, used existing numeric coordinate types, added read sets to compose/render envelopes, and read the final dependency set when emitting output; retain this common-record integration.
31. Kept the existing Markdown-mode error-block layout rather than add structured data line/column display to stderr; the spec does not require that additional presentation.
32. Reused catalog-selected type-definition editor routines, withheld document-local types and host catalogs, limited unfinished-call recovery to the cursor’s line, and added callback-member suggestions from shared typing; retain these editor choices.
33. Used the existing load_data_* test-module names rather than introduce nearly identical loading_data_* names; retain the test layout.
34. Cleared cached data after every nonempty lifecycle event, including guarded-off and informational stacks; replace this broad end-of-event rule with executed-step invalidation under R1-F1, the stale lifecycle-read finding.
35. Deferred later body and top-level-shell reads that observe earlier file changes as possible future shell-stage work; complete the specified ordering in this feature under R1-F2, the eager-read finding.
36. Added schema dependencies to the public preparation summary so validate_schema can report transitive reads; retain the added field on the existing non-exhaustive type.
37. Used the parser’s mapping-start location for duplicate YAML keys and omitted unavailable locations for second documents and non-string keys; keep proven duplicate positions and repair missing positions under R1-F4, the malformed-input-location finding.
38. Embedded real catalog example-data files, wrote them into temporary example workspaces, and printed their contents in generated documentation; retain this shared example source.
39. Added a general catalog function-page generator with a page list and separate write modes instead of a load_data-only renderer; retain this small reusable documentation surface.
40. Added newcomer type-definition examples and a container section in Darkmatter’s page while linking to grammar material already moved into simplified-schema; retain this documentation ownership.
41. Recovered failed-import dependencies from final schema errors but deferred dependencies read earlier in the failed expansion; complete that work under R1-F3, the incomplete-read-set finding.
42. Documented remaining differences instead of rewriting the spec: repair transport reference fields and bracket-union errors under R1-F5 and R1-F6; retain bare-name rejection, whole-file YAML position suppression, and exclusion of unresolved data targets as consistent with the stated contract.
43. Removed stale spec references, repaired nearby documentation links and scanner prose, and corrected the README hash command; retain these encountered documentation repairs.
44. Filed the four authorized active follow-ups for expression recovery, tabular input options, member-property checking, and YAML locator hardening, and corrected the future dynamic-schema example; these remain explicitly outside this feature.
45. Left editor invalidation from read sets to the later dynamic-schema work, as the spec directs; no editor dependency watcher is required here.
46. Left final Linux, native-Windows, and WSL2 evidence to CI rather than use a cross-check command that creates a commit; this does not block readiness.
47. Used a relative-link/anchor script when documentation reference validation tried to execute inline examples, including unchanged examples; retain this bounded documentation verification method.
48. Updated Claudine’s expression-list snapshot when the final full suite exposed its missing load_data entry and recorded that dependency in the skill; retain the corrected snapshot.
