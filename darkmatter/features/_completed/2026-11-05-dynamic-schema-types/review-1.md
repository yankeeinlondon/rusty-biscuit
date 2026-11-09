---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F1 (high): Several slot paths perform reads before approval"
  - "R1-F2 (high): Overridden trigger slots still evaluate"
  - "R1-F3 (high): Inline and trigger slot failures lose authored locations"
  - "R1-F4 (high): Unsaved data buffers are ignored outside YAML and JSON"
  - "R1-F5 (high): Stale completion candidates have no stale marker"
human_review: true
human_review_items:
  - "Array-item completion reuses scalar completion, so booleans, files, and literals benefit along with enums; retain this shared implementation."
  - "Unclosed flow lists still receive no completion because their YAML does not parse; retain this existing limit rather than add general YAML recovery here."
  - "Symlink-escape and other-repository approval fixtures moved to the approval phase and were subsequently delivered; no work remains deferred."
  - "The pending resolution handle moved to the resolver phase and was subsequently delivered; no placeholder public field was needed."
  - "Unresolved validation uses the existing preparation-failure result with a typed underlying cause; retain this compatible error representation."
  - "Markdown factories project suggestion positions into document coordinates alongside slot positions; retain consistent coordinates."
  - "Inherited static and dynamic bounds and defaults merge after resolution, when their values are known; retain the existing merge rules."
  - "Subtype queries remain boolean and fail closed for unresolved enum and literal types; retain this bounded internal choice."
  - "Suggestion and union-completion queries expose pending flags while retaining literal candidates; retain this representation."
  - "The lower-level source scanner still lacks block-scalar projection, while file factories support it; retain this pre-existing lower-level limit."
  - "The shipped-schema exemption was removed as soon as passive admission succeeded, earlier than planned; retain the removal."
  - "Every file-defined slot records its defining file, including root schemas and triggers; retain this needed identity."
  - "Resolution replays captured schema text rather than patching an expanded tree; retain the consistent snapshot despite temporary text copies."
  - "Root-union arms resolve in successive rounds; retain this approach unless repair reveals a concrete admission-order defect."
  - "Host failures retain typed errors that Darkmatter can inspect, rather than adding a host-specific data-location type to the schema library; retain the boundary."
  - "Dynamic defaults use the existing structural validator and report the first invalid result; retain this reuse without changing static defaults."
  - "The slot-language page is the canonical language reference, with links from the type reference; retain one source of documentation."
  - "Date functions share a captured clock and its fixed UTC offset; retain this bounded choice, with historical daylight-saving conversion deferred because it needs timezone rules."
  - "Command probes use the captured environment PATH; retain this author-decided behavior, including no bare-name result when PATH is absent."
  - "Host handlers are treated as pure by their existing arguments-only contract, without changing the function-facts artifact; retain this contract."
  - "URL discovery follows actual evaluator aliases and case matching, rather than adding underscore folding; retain matching behavior."
  - "Glob listings enter the observation log rather than the path-only read set because the listing API does not expose searched folders; retain this distinction."
  - "Contract imports and some probes are authorized after observation; repair content and listing reads under R1-F1, while retaining ordinary candidate discovery."
  - "Missing files use the general slot-failure code and reference details, while malformed data retain data-specific diagnostics; retain the existing loading-data distinction."
  - "Rejected dynamic patterns have a dedicated result and diagnostic code; retain the typed distinction."
  - "Admission reports the first refused part of each slot; retain this bounded error presentation."
  - "The early independent slot clock capture was assigned to the session phase; retain the final shared-request approach rather than treat the early note as unfinished work."
  - "Sniff distinguishes files affected by Git filters so they cannot receive clean-file approval; retain the explicit filtered state."
  - "Early preflight support covered only root schemas, with triggers and children assigned to later phases; those later surfaces were delivered."
  - "Discovery leaves templates pending instead of evaluating their expressions; retain passive discovery."
  - "Preflight presents successive read prompts before shell prompts through a batched handler; retain the existing prompt interface."
  - "Without an approval handler, preflight waits for resolution to report the positioned approval failure; retain this failure path."
  - "Bare-name and magic-root targets are checked during evaluation because preflight lacks a passive candidate resolver; retain runtime checks without adding another resolver."
  - "Any applicable denial overrides an allowance, including directory and glob cases; retain the author-decided denial precedence."
  - "Failed persistence keeps the approved answer for the current run, while handler failure grants nothing; retain this bounded failure behavior."
  - "Stored URL grants accept HTTP and HTTPS, reject query-bearing query-independent rules, and omit a line for whole-file read errors; retain these parsing rules."
  - "CLI approval proofs waited for CLI resolution support and were subsequently delivered; no work remains deferred."
  - "Read-nothing passes use existing pins and otherwise defer the entire template; retain this conservative reference-check behavior without adding partial resolution."
  - "Overridden trigger slots are filtered only from preflight questions; repair evaluation under R1-F2 rather than accept a departure from the author’s precedence rule."
  - "URL, listing, and glob results are shared only within their originating command because they cannot be cheaply rechecked; retain conservative reuse."
  - "Cache context dependencies are collected passively and digested again so failures can also be cached; retain this approach without speculative optimization."
  - "Resolution-time grant-store errors appear at slots with the store location in the message; retain this bounded presentation, with preflight retaining direct store locations."
  - "Document-owned data contracts remain unresolved when they name templates; only contracts inside active slot evaluation resolve recursively, avoiding an extra effectful entry point."
  - "Cache keys include the authored schema value and directory or trigger identity, covering inline templates with no source file; retain this identity."
  - "Trigger output uses a pending boolean rather than a state enumeration; retain the additive output field."
  - "Without a session, selected template triggers fail as unresolved instead of disappearing from validation; retain this fail-closed behavior."
  - "Kind-selection failures carry generic optional codes rather than teaching the schema library a Darkmatter-specific error variant; retain the package boundary."
  - "The baseline prohibition code is derived from its typed cause rather than stored twice; retain typed cause preservation."
  - "Global catalogs reject slots at load because their globals apply before resolution; retain passive global catalogs."
  - "Slots in trigger match conditions retain the existing invalid-trigger diagnostic code; retain the existing error family."
  - "Claudine completion offers pending schema property names and falls back to untyped values without evaluating slots; retain passive completion."
  - "Claudine launch reuses its schema-load error for unresolved templates; retain the existing error presentation."
  - "Compose creates its explicit slot session after preflight so run-only approvals can be attached; retain this ordering while keeping earlier reference discovery passive."
  - "Graph and reference validation defer an entire unapproved template layer; retain conservative deferral rather than introduce partial validators in this feature."
  - "Schema validation judges automatic approval against the launch request repository; retain the author’s rule that external schema contexts receive no automatic approval."
  - "Validate and clean sessions omit captured observations; the log claims command probes therefore lose PATH, but public CLI controls found git successfully on all three commands, so no PATH defect is reported."
  - "Approval diagnostics list allowance choices once and persisted, with duplicate file-level choices removed; retain this script-facing description."
  - "Validate and clean convert approval-handler errors to ordinary approval-required failures; retain refusal without inventing another outcome."
  - "Schema-definition reports include a slot count only when positive, counting imported schema definitions without evaluating their slots; retain optional output."
  - "Validation output includes kind and baseline slot-prohibition codes; retain the useful typed diagnostics."
  - "Two documentation stubs received introductions and slot sections rather than unrelated full-topic rewrites; retain the limited documentation scope."
  - "Performance samples measured actual CLI and language-server processes instead of rebuilding the investigation benchmark; accept the measurement method and disclose the missing direct comparison."
  - "Missed warm and cold timing comparisons were recorded without speculative optimization; retain this response because the spec does not make those quick samples release gates."
  - "The authorized whole-type follow-up was created as a draft with unresolved design questions; retain the author-approved sequencing."
  - "Editor snapshots retain worker sessions and their dependency paths rather than copying concrete schemas into another store; retain reuse of the library snapshot logic."
  - "Editor work is bounded by visits and expansion bytes rather than a wall-clock deadline; retain deterministic work limits."
  - "Editor work-limit failures are excluded from the shared cache so commands with larger allowances can retry; retain this separation."
  - "Open buffers force dependency rehashing despite unchanged disk metadata; retain unsaved-edit invalidation."
  - "Editor quick fixes add deny choices separately from allowance metadata, and run-only grants last until the server exits; retain this editor session scope."
  - "Inline editor diagnostic positions are found from authored slot text; repair source preservation under R1-F3 so hosts can share reliable locations."
  - "Completion omits stale annotations while hover and diagnostics include them; repair completion under R1-F5 as the spec explicitly requires stale completion to be identified."
  - "Pending editor diagnostics are informational, approval-required is a warning, and other failures are errors; retain those severities and explicit stale status during refresh."
  - "Pins-only unresolved results retain their replay handle so literal candidates remain inspectable; retain this useful pending information."
  - "Unsaved dependency text is supplied only for YAML and JSON; repair the extension restriction under R1-F4 rather than accept narrower supported data formats."
  - "Mutation invalidation uses the data-memo generation and dependency rechecks instead of per-path write attribution; retain the existing invalidation mechanism."
  - "The mutation test shares one session across compositions to exercise the next validation after a write; retain this test boundary."
  - "Context-cache invalidation tests vary environment inputs rather than a separate captured context group; accept the shared digest path without adding a redundant matrix."
  - "Slot failures are governed by read grants rather than general recovery flags; retain the documented security boundary."
  - "Host allowances match exact case-insensitive hostnames, without wildcard or network-range matching; retain existing host semantics, with IPv6 verification not claimed."
  - "Unrelated proposed language-server commands were left outside the slot documentation changes; retain surgical documentation scope."
  - "Repair R1-F1: `$schema` is now excluded from every frontmatter interpolation pass (matching pre-flight), so other keys can no longer read `$schema` during interpolation; the alternative was passing it raw while still skipping evaluation."
  - "Repair R1-F1: schema imports opened inside a slot (data contracts, `validate_schema`) are judged as content reads under the calling function's name, and listings need a folder grant on the pattern root or the indexed file's folder; listing a missing folder stays an existence probe (Q21)."
  - "Repair R1-F1: nested template slots inside a contract are judged only by their own gates, and a refused contract preparation is not memoized; the alternative applied the outer slot's guard to them, changing per-file automatic grants."
  - "Repair R1-F2: precedence pruning happens inside the library replay (`PendingSchema::without_properties`), and a root-union layer replaces a lower property only when every arm declares it, matching the existing merge."
  - "Repair R1-F2: a root-union template document's declared properties are computed passively across all arms, including referenced arms read without evaluation, so pre-flight and runtime prune identically."
  - "Repair R1-F3: slot spans are projected in the library at parse time and attached only when the located `$schema` text matches the parsed value node for node; otherwise positions stay null. Document slots keep no defining-file origin, so they still evaluate in the request context."
  - "Repair R1-F3: DMLS uses the library's slot span only when the failure was resolved from the buffer's current text, otherwise the `$schema` value range; a refused read written through a YAML escape reports the whole expression rather than a second sub-span mapping."
  - "Repair R1-F3: a contract-template slot failure publishes the inner failure's code (cycle, then grant problems, then work limits, then first failure) at the outer slot's position; no secondary-location field was added because it would change all 17 output schemas."
  - "`validate_schema` naming a page whose schema is a template keeps failing closed as unresolved instead of resolving through the active slot session; resolving there would add a new effectful entry point with grant consequences."
  - "Repair R1-F4: DMLS records every open buffer in one shared buffer set (trigger and kind routing keep their own path checks), and a read served from a buffer records no file `stat` so closing the buffer forces a rehash; the alternatives were a slot-only buffer map or clearing the cache on close."
  - "Outside R1-F4's class and retained: a slot-free Markdown `$schema` still assembles its imports from disk, and a data file existing only as a never-saved buffer is not found by `load_data`, because path resolution checks disk first."
  - "Repair R1-F5: the stale or pending note goes in the completion item's `detail` (appended after any existing detail) for every ordinary frontmatter candidate, keys included; schema-authoring grammar candidates stay undecorated, and a quick-fix title is marked only when it inserts a slot-supplied literal."
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-08T11:38:58-08:00"
spec: "2026-11-05-dynamic-schema-types/spec.md"
implemented: true
description: "A **feature** review of `2026-11-05-dynamic-schema-types/spec.md`"
feature: "2026-11-05-dynamic-schema-types/review-1.md"
next: "2026-11-05-dynamic-schema-types/review-2.md"
log: "/Volumes/coding/wt/rusty-biscuit/feat-better-research/darkmatter/features/2026-11-05-dynamic-schema-types/log.md"
implemented_by: claude/opus
---
# Review 1: Dynamic Schema Types

The feature is **not production ready**. Five findings remain. None needs a human decision before repair: each recommendation follows the existing specification and author decisions.

This review covers the current working tree, including its uncommitted implementation changes. Two agents independently read the spec and implementation log in parallel before the code comparison. Their inventories included the editor decisions accidentally stored under the log’s skill-file list. This is the first implementation review; there are no previous findings to carry forward.

## Findings

### R1-F1 (high): Several slot paths perform reads before approval

**Authority: spec.** The per-resource approval decision says “Everything else asks,” permits automatic content reads only for clean files in the defining repository, and requires computed targets to be checked when evaluated. The required semantics also prohibit using document reads to bypass slot permissions. In a temporary folder with an empty home policy and no repository, the current implementation reads data and schema imports, or lists directories, before returning approval-required. Returning an error afterward does not undo the read.

**Defect class:** authorization happens after I/O on several expression paths, while the ordinary direct-content path correctly authorizes first.

In the darkmatter package, [frontmatter interpolation](../../lib/src/markdown/compose/frontmatter_interpolation.rs:966) includes the inline schema subtree in ordinary expression evaluation. The composition pipeline then judges the schema through the slot resolver, but that check is too late to prevent the earlier read. Inline enum and suggestion arrays also become text and produce a schema-preparation error. In the same package, [data-contract preparation](../../lib/src/markdown/compose/expression/data/load.rs:69) opens imported schema files before the data-source gate; [schema validation and directory functions](../../lib/src/markdown/compose/expression/functions/fs.rs:1300) likewise record inputs before the dispatch wrapper checks them.

The public-API reproduction composes a document against a referenced schema, passes an explicit data runtime, and reads its observation log after failure. A content hash proves the corresponding bytes were read; listing observations prove enumeration occurred. The CLI reproduction uses `md compose doc.md --output json` and inspects its read set. No approval handler, persisted grant, or automatic repository grant is present. The data control is `names.yaml` containing `- Linux`; the document value is `Linux`.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Inline enum | Slot loads an array of strings | Data file appears in read set before schema-preparation failure | Approval-required before data bytes are read |
| R1-F1.2 | Inline suggestion | Same data array in a string suggestion list | Same premature read and preparation failure | Same |
| R1-F1.3 | Inline literal | First member of the data array supplies a literal | File read before approval-required | Same |
| R1-F1.4 | Inline default | First member supplies a string default | File read before approval-required | Same |
| R1-F1.5 | Inline minimum | Array length supplies a number bound | File read before approval-required | Same |
| R1-F1.6 | Inline maximum | Same array length | File read before approval-required | Same |
| R1-F1.7 | Inline pattern | First member supplies a pattern | File read before approval-required | Same |
| R1-F1.8 | Imported data contract | Referenced slot loads data with `Names@./private.yaml`; that file exports `Names: string[]` | Observation log contains the unapproved imported file and its content hash | Authorize each import before opening it |
| R1-F1.9 | Schema-validation expression | Permit reading `page.md`, whose schema names an unapproved `private.yaml` | Both files appear as content observations before refusal | Read the permitted page, then stop before opening its schema |
| R1-F1.10 | Glob listing | Referenced slot calls `find_files('./*.md')` | Hashed glob listing is observed before refusal | Directory approval before enumeration |
| R1-F1.11 | First indexed file | Referenced slot calls `find_first_index('./review-1.md')` | Directory listing is observed before refusal | Same |
| R1-F1.12 | Last indexed file | Corresponding last-index call | Directory listing is observed before refusal | Same |
| R1-F1.13 | Direct data read, clean control | Referenced slot loads `names.yaml` with the inline `string[]` contract | Approval-required; no content observation | Correct; retain |
| R1-F1.14 | Frontmatter reader, clean control | Referenced slot reads `page.md` without a grant | Approval-required; no content observation | Correct; retain |
| R1-F1.15 | Recovering frontmatter reader, clean control | Same file through `try_frontmatter` | Refusal is preserved; no content observation | Correct; retain |
| R1-F1.16 | Markdown title reader, clean control | Same file through `markdown_title` | Approval-required; no content observation | Correct; retain |
| R1-F1.17 | Markdown-body reader, clean control | Same file through `markdown_body_empty` | Approval-required; no content observation | Correct; retain |

The other registered input readers were checked against the dispatch table: path transformations, index arithmetic, skill probes, and file existence checks make existence observations rather than reading file contents or enumerating directory entries. Their candidate probing is distinct from the content/listing failures above. Existing remote-read and shared-memo tests pass and confirm authorization before fetching or borrowing memoized content.

**Recommended repair:** keep schema slot syntax out of ordinary frontmatter interpolation, including later interpolation passes. Add authorization at the schema-import opening boundary and before listing operations. Reuse the existing file-resolution and grant mechanisms; do not build a second candidate resolver. Preserve the post-call observation check as an additional check, rather than relying on it to prevent reads. Add the failing public-API cases beside the existing direct-read controls.

### R1-F2 (high): Overridden trigger slots still evaluate

**Authority: spec.** The effective-schema decision states: “A slot under a property that a higher layer redeclares is never read or prompted for.” A document that replaces a trigger’s dynamic `os` property with `os: string` still fails because the discarded trigger slot requests approval. A later trigger that replaces the same property has the same problem.

**Defect class:** lower-priority properties are resolved before whole-property precedence removes them.

In the darkmatter package, [trigger-layer resolution](../../lib/src/markdown/schemas/mod.rs:1055) resolves every slot of a selected payload. The earlier [preflight collection](../../lib/src/markdown/schemas/mod.rs:943) only filters questions, and considers document overrides rather than every higher-priority trigger. Filtering the question does not prevent evaluation or a later prompt.

Reproduction: copy the existing template-trigger fixture. Its matcher selects `topic: sandbox`, and its payload declares `os: enum({{ load_data('../data/os.yaml', 'string[]') }}; required)`. Use untracked data and no grants. Change only the overriding layer or matcher value:

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Document over trigger | Document declares inline `os: string`, value `Windows` | Exit 1, approval-required for the discarded enum | Success without asking about or reading its data |
| R1-F2.2 | Later trigger over earlier trigger | `b.trigger.yaml` replaces the dynamic property from `a.trigger.yaml` with `os: string` | Same approval-required failure | Same |
| R1-F2.3 | Unmatched payload, clean control | Change `topic` to `other` | Exit 0; no slot read | Correct; retain |
| R1-F2.4 | Effective dynamic payload, clean control | Select the trigger without an override | Approval-required | Correct; retain the gate |

The baseline cannot contain slots, so it has no sibling evaluation site. Schema preparation and each command using it inherit the trigger-layer behavior; fixing only CLI prompting will leave validation and DMLS incorrect.

**Recommended repair:** determine the surviving top-level properties across all layers before resolving their slots, and resolve the surviving template shape. Preserve normal whole-property replacement. This does not require general lazy validation or placeholder values for discarded constraints; the discarded properties should not reach resolution. Use the same surviving-slot selection for preflight and runtime.

### R1-F3 (high): Inline and trigger slot failures lose authored locations

**Authority: spec.** Minimum requirement 3 requires positioned slot errors, with the authored slot as the primary source; the required semantics explicitly include quoted and block YAML carriers. A local inline schema is not an unknown source: the command has the original document and can preserve its authored position. Currently its slot error has no file, line, column, or span. A trigger-inline slot keeps its file but loses the line and column.

**Defect class:** passing schema values without their authored source text and coordinate mapping loses diagnostic provenance.

In the darkmatter package, [diagnostic projection](../../lib/src/markdown/compose/report_diagnostics.rs:752) can position a slot only when the failure retains its defining text. [Trigger payload preparation](../../lib/src/markdown/schemas/triggers/assemble.rs:311) supplies a parsed payload value without retaining the enclosing trigger text. Inline document slots also have no defining file on the slot. The library’s standalone and imported schema paths already retain the necessary text.

Reproduction: use `enum({{ 123 }})`, a realistic accidental scalar result, and run both `md compose` and `md schema validate` with JSON output. For the trigger path, the unapproved data read from R1-F2 also demonstrates the missing coordinates.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Compose inline schema | Wrong enum result | Typed slot error, but source file, line, column, and span are null | Original document and authored slot range |
| R1-F3.2 | Schema-validate inline schema | Same document | Same missing source | Same |
| R1-F3.3 | Compose trigger-inline payload | Selected slot requires approval | Trigger file retained; line, column, and span null | Range of the slot in the trigger file |
| R1-F3.4 | Schema-validate trigger-inline payload | Same document and trigger | Same lost coordinates | Same |
| R1-F3.5 | Compose referenced schema, clean control | Same wrong result in a standalone schema | Correct schema file, line 2, column 12 | Correct; retain |
| R1-F3.6 | Schema-validate referenced schema, clean control | Same referenced schema | Correct file and position | Correct; retain |
| R1-F3.7 | Compose imported named type, clean control | `Name` imported from a kinded schema; wrong result inside its enum | Correct imported file, line 3, column 14 | Correct; retain |
| R1-F3.8 | Schema-validate imported type, clean control | Same imported definition | Correct imported file and position | Correct; retain |
| R1-F3.9 | DMLS simple inline slot, clean control | Existing approval diagnostic protocol test | Correct authored slot range from editor-side lookup | Retain positioned output while consolidating its source mapping |

The pure-library span tests also pass. They cover quoted and block carriers; their mapping should be reused when the host supplies document and trigger sources. Do not solve this by reopening files during diagnostics, which could produce positions from a different revision or add I/O to editor requests.

**Recommended repair:** retain original document/trigger text and its schema-value source mapping in pending templates and failure carriers. Project the slot once into the owning file’s coordinates, including quoted and block carriers, so CLI and DMLS can consume the same proven positions. Add public-result tests for both failing origins and keep referenced/imported controls.

### R1-F4 (high): Unsaved data buffers are ignored outside YAML and JSON

**Authority: spec.** Cache correctness and refresh require watching actual dependencies “regardless of extension” and refreshing unsaved data/schema buffers through the source-buffer mechanism. The supported data formats include JSON5, JSONL, TOML, CSV, and TSV. Editing a loaded data file in one of these formats refreshes the worker but republishes a completed snapshot based on the old disk contents.

**Defect class:** extension-based filtering discards authoritative open-buffer text before dependency resolution.

In the darkmatter-lsp package, the [open-buffer setter](../../dmls/src/overlay/mod.rs:912) returns immediately unless the path is YAML or JSON. The [slot job](../../dmls/src/slot_jobs.rs:77) then receives that restricted buffer snapshot. The same data-loading mechanism supports the other formats, so this is a host input restriction rather than a parser limitation.

I adapted the existing unsaved-data protocol test, keeping its real server, worker, filesystem, persisted file grant, and synchronization. Each control initially rejects `Haiku`; the only edit then adds `Haiku` to the unsaved data buffer. The disk file remains unchanged. A block YAML carrier avoids quoting ambiguity in contracts containing colons. Every row ran as an ordinary L1 protocol test, compiled through the existing declared test target.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F4.1 | JSON5 buffer | Array changes from `["Linux"]` to `["Linux","Haiku"]` | Completed refresh still rejects Haiku | Accept the unsaved member |
| R1-F4.2 | JSONL buffer | Add a second string record | Same stale disk result | Same |
| R1-F4.3 | TOML buffer | Add Haiku under an `os` array; contract reads that field | Same stale disk result | Same |
| R1-F4.4 | CSV buffer | Add a Haiku row; map the `os` column into members | Same stale disk result | Same |
| R1-F4.5 | TSV buffer | Same column/row case in TSV | Same stale disk result | Same |
| R1-F4.6 | YAML buffer, clean control | Add a Haiku sequence member | Unsaved member accepted; disk unchanged | Correct; retain |
| R1-F4.7 | JSON buffer, clean control | Add a Haiku array member | Unsaved member accepted; disk unchanged | Correct; retain |

These are seven readers of the same buffer handoff, not seven separate parser fixes. All five failing cases finished refreshing and then emitted a normal constraint error; this is not merely a stale-marker delay.

**Recommended repair:** retain open text for every actual slot dependency, independently of schema-document extension routing, and supply it to the worker’s source buffers. Preserve dirty-buffer approval checks and invalidation on open, change, and close. Do not add a second set of data parsers.

### R1-F5 (high): Stale completion candidates have no stale marker

**Authority: spec.** During refresh, last-good values may support “explicitly stale completion/hover.” Hover and diagnostics identify stale results, but completion silently returns ordinary candidates from the old snapshot. This occurs during an ordinary edit of a loaded member list.

**Defect class:** the completion projection drops the snapshot’s refresh status.

In the darkmatter-lsp package, the [frontmatter provider](../../dmls/src/providers/frontmatter.rs) decorates hover with the slot status, but does not decorate completion responses. Using the existing held-worker protocol fixture, I loaded Linux, changed the data while holding the refresh, and requested completion at `os: `. It returned:

```json
[{"label":"Linux","kind":20,"textEdit":{"range":{"start":{"line":3,"character":4},"end":{"line":3,"character":4}},"newText":"Linux"}}]
```

There is no detail, documentation, or other field telling the user that the candidates are stale. The separate information diagnostic does not label the completion item being selected.

| Child ID | Site | Shape tested | Observed result | Expected result |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Completion response | Resolved enum while its data refresh is held | Old Linux candidate with no stale indication | Candidate explicitly identifies stale schema values |
| R1-F5.2 | Hover, clean control | Same held refresh | Hover contains a stale note | Correct; retain |
| R1-F5.3 | Diagnostic status, clean control | Same held refresh | Information diagnostic says results come from the last resolution | Correct; retain |

Scalar, union, and array-item candidate builders all feed completion items through this provider without a snapshot-status decoration. The repair belongs at that shared response boundary, rather than inside enum parsing or each type-specific candidate builder.

**Recommended repair:** add a short stale note to the existing completion-item detail or documentation when its values come from a refreshing snapshot. Preserve labels, insertion text, ordering, and the nonblocking worker design. Add a held-refresh completion assertion beside the existing hover/diagnostic assertions.

## Verification and test rigor

| Requirement group | Strongest reviewed evidence | Assessment |
| --- | --- | --- |
| Passive parsing, template inspection/export, result shapes, imported definitions, static parity | L1 library tests; 75 slot-selected tests passed in simplified-schema | Appropriate level; no terminal behavior involved |
| Minimal-feature library support and shipped artifacts | The simplified-schema recipe also ran its minimal-feature suite: 1,435 passed | Appropriate level |
| Composition, approval, cache reuse/invalidation, typed failures, trigger selection | L1 public APIs and shipped CLI tests; 237 slot-selected Darkmatter tests passed | Appropriate level, with missing cases exposed above |
| DMLS background resolution, edits, grant actions, superseded workers, no request-path evaluation, completion/hover | L1 protocol tests drive the actual server and workers | Appropriate level; a real editor window or keyboard injection would add no required evidence |
| Unsaved format coverage and stale completion | Temporary L1 protocol variants: YAML/JSON controls passed; five other formats and stale completion failed | Findings R1-F4 and R1-F5 |
| Content and listing authorization order | Temporary public composition probe observed runtime reads after refusal; CLI read-set reproductions confirm inline behavior | Finding R1-F1 |
| Prompt choices and grant persistence | Existing scripted approval tests, policy public API, repeated write/read tests | Appropriate L1 boundary for this feature’s stated choices; no key-encoder or terminal-color promise |

Commands run from their package areas: `just test slot` in darkmatter and simplified-schema. The former passed 237 selected tests. The latter passed 75 selected tests and its recipe’s 1,435-test minimal-feature pass. Temporary tests used the existing consolidated L1 modules and were selected by ordinary names through `just test review_probe`. Their additions were removed after gathering evidence; implementation files were restored byte-for-byte. The public read-observation probe deliberately printed its result through a failing assertion; that assertion itself is not a product failure. The reported I/O observations are the evidence. The final eight editor probes had two passing controls and six failing behavioral assertions, each under half a second.

I inspected declared test-module wiring and tier selection. The feature’s tests are compiled in the existing L1 targets; no terminal, browser, or keyboard resource is required for the behavior reviewed here. No focus-stealing window was opened. Full-suite and lint results in the implementation log were read as prior evidence; this review did not rerun every area’s lint or full test suite. Cross-OS evidence is left to CI and does not affect this readiness judgment.

The timing samples miss two investigation comparisons and omit the direct slot-versus-lowering comparison. The spec explicitly treats the quick sample as explanatory rather than a pass/fail threshold, so these are informational rather than additional findings. No unmeasured optimization is required by this review.

## Input robustness matrix

The new configuration loader is the fixed-home, line-based read-grant store. The schema expression grammar is a text grammar and is outside this matrix; existing data-file parsers are not redefined by this feature.

**Default outcome:** a malformed grant record rejects the entire store with its path and line and grants nothing. A whole-file read/encoding failure also rejects the store, with no invented source line. **Defined exceptions:** a missing store and an empty store both mean no persisted rules; blank/comment lines and a UTF-8 byte-order mark are allowed; duplicate identical records are deduplicated; denial overrides an allowance. These are explicit format semantics, not silent fallback on malformed fields.

The public store-loading tests cover a valid record for every action/scope combination, absent/empty targets, absent or unknown scope, unknown action, relative paths, malformed URLs, non-HTTP schemes, forbidden URL fragments, query-bearing query-independent rules, and forbidden site components. They also cover invalid UTF-8, unreadable files, CRLF, comments, spaces in paths, duplicate records, and repeated write/read round trips. Line 3 errors are checked with valid records before and after the bad record, proving rejection of the whole file. This line format has no native null, typed scalar, or array representation; collection-element rows do not apply. CLI wiring tests report an invalid store at its actual line and refuse resolution.

The requested smell scan of the store loader found no serde defaults, permissive element filtering, default-on-parse-failure, or ignored parse result. No additional configuration-shape finding was found. Approval ordering is separately recorded in R1-F1 rather than hidden in this section.

## Implementation decisions (informational)

No item requires stopping for a human. The author can inspect or reverse the retained choices after the repair cycle; no response is needed to proceed. Where an implementation choice contradicts an existing requirement, the recommendation is to repair it under the finding named below. Early phase deferrals that were subsequently delivered are marked accordingly, rather than reported as unfinished work. The summaries below correspond one-for-one to `human_review_items`.

1. Array-item completion reuses scalar completion, so booleans, files, and literals benefit along with enums; retain this shared implementation.
2. Unclosed flow lists still receive no completion because their YAML does not parse; retain this existing limit rather than add general YAML recovery here.
3. Symlink-escape and other-repository approval fixtures moved to the approval phase and were subsequently delivered; no work remains deferred.
4. The pending resolution handle moved to the resolver phase and was subsequently delivered; no placeholder public field was needed.
5. Unresolved validation uses the existing preparation-failure result with a typed underlying cause; retain this compatible error representation.
6. Markdown factories project suggestion positions into document coordinates alongside slot positions; retain consistent coordinates.
7. Inherited static and dynamic bounds and defaults merge after resolution, when their values are known; retain the existing merge rules.
8. Subtype queries remain boolean and fail closed for unresolved enum and literal types; retain this bounded internal choice.
9. Suggestion and union-completion queries expose pending flags while retaining literal candidates; retain this representation.
10. The lower-level source scanner still lacks block-scalar projection, while file factories support it; retain this pre-existing lower-level limit.
11. The shipped-schema exemption was removed as soon as passive admission succeeded, earlier than planned; retain the removal.
12. Every file-defined slot records its defining file, including root schemas and triggers; retain this needed identity.
13. Resolution replays captured schema text rather than patching an expanded tree; retain the consistent snapshot despite temporary text copies.
14. Root-union arms resolve in successive rounds; retain this approach unless repair reveals a concrete admission-order defect.
15. Host failures retain typed errors that Darkmatter can inspect, rather than adding a host-specific data-location type to the schema library; retain the boundary.
16. Dynamic defaults use the existing structural validator and report the first invalid result; retain this reuse without changing static defaults.
17. The slot-language page is the canonical language reference, with links from the type reference; retain one source of documentation.
18. Date functions share a captured clock and its fixed UTC offset; retain this bounded choice, with historical daylight-saving conversion deferred because it needs timezone rules.
19. Command probes use the captured environment PATH; retain this author-decided behavior, including no bare-name result when PATH is absent.
20. Host handlers are treated as pure by their existing arguments-only contract, without changing the function-facts artifact; retain this contract.
21. URL discovery follows actual evaluator aliases and case matching, rather than adding underscore folding; retain matching behavior.
22. Glob listings enter the observation log rather than the path-only read set because the listing API does not expose searched folders; retain this distinction.
23. Contract imports and some probes are authorized after observation; repair content and listing reads under R1-F1, while retaining ordinary candidate discovery.
24. Missing files use the general slot-failure code and reference details, while malformed data retain data-specific diagnostics; retain the existing loading-data distinction.
25. Rejected dynamic patterns have a dedicated result and diagnostic code; retain the typed distinction.
26. Admission reports the first refused part of each slot; retain this bounded error presentation.
27. The early independent slot clock capture was assigned to the session phase; retain the final shared-request approach rather than treat the early note as unfinished work.
28. Sniff distinguishes files affected by Git filters so they cannot receive clean-file approval; retain the explicit filtered state.
29. Early preflight support covered only root schemas, with triggers and children assigned to later phases; those later surfaces were delivered.
30. Discovery leaves templates pending instead of evaluating their expressions; retain passive discovery.
31. Preflight presents successive read prompts before shell prompts through a batched handler; retain the existing prompt interface.
32. Without an approval handler, preflight waits for resolution to report the positioned approval failure; retain this failure path.
33. Bare-name and magic-root targets are checked during evaluation because preflight lacks a passive candidate resolver; retain runtime checks without adding another resolver.
34. Any applicable denial overrides an allowance, including directory and glob cases; retain the author-decided denial precedence.
35. Failed persistence keeps the approved answer for the current run, while handler failure grants nothing; retain this bounded failure behavior.
36. Stored URL grants accept HTTP and HTTPS, reject query-bearing query-independent rules, and omit a line for whole-file read errors; retain these parsing rules.
37. CLI approval proofs waited for CLI resolution support and were subsequently delivered; no work remains deferred.
38. Read-nothing passes use existing pins and otherwise defer the entire template; retain this conservative reference-check behavior without adding partial resolution.
39. Overridden trigger slots are filtered only from preflight questions; repair evaluation under R1-F2 rather than accept a departure from the author’s precedence rule.
40. URL, listing, and glob results are shared only within their originating command because they cannot be cheaply rechecked; retain conservative reuse.
41. Cache context dependencies are collected passively and digested again so failures can also be cached; retain this approach without speculative optimization.
42. Resolution-time grant-store errors appear at slots with the store location in the message; retain this bounded presentation, with preflight retaining direct store locations.
43. Document-owned data contracts remain unresolved when they name templates; only contracts inside active slot evaluation resolve recursively, avoiding an extra effectful entry point.
44. Cache keys include the authored schema value and directory or trigger identity, covering inline templates with no source file; retain this identity.
45. Trigger output uses a pending boolean rather than a state enumeration; retain the additive output field.
46. Without a session, selected template triggers fail as unresolved instead of disappearing from validation; retain this fail-closed behavior.
47. Kind-selection failures carry generic optional codes rather than teaching the schema library a Darkmatter-specific error variant; retain the package boundary.
48. The baseline prohibition code is derived from its typed cause rather than stored twice; retain typed cause preservation.
49. Global catalogs reject slots at load because their globals apply before resolution; retain passive global catalogs.
50. Slots in trigger match conditions retain the existing invalid-trigger diagnostic code; retain the existing error family.
51. Claudine completion offers pending schema property names and falls back to untyped values without evaluating slots; retain passive completion.
52. Claudine launch reuses its schema-load error for unresolved templates; retain the existing error presentation.
53. Compose creates its explicit slot session after preflight so run-only approvals can be attached; retain this ordering while keeping earlier reference discovery passive.
54. Graph and reference validation defer an entire unapproved template layer; retain conservative deferral rather than introduce partial validators in this feature.
55. Schema validation judges automatic approval against the launch request repository; retain the author’s rule that external schema contexts receive no automatic approval.
56. Validate and clean sessions omit captured observations; the log claims command probes therefore lose PATH, but public CLI controls found git successfully on all three commands, so no PATH defect is reported.
57. Approval diagnostics list allowance choices once and persisted, with duplicate file-level choices removed; retain this script-facing description.
58. Validate and clean convert approval-handler errors to ordinary approval-required failures; retain refusal without inventing another outcome.
59. Schema-definition reports include a slot count only when positive, counting imported schema definitions without evaluating their slots; retain optional output.
60. Validation output includes kind and baseline slot-prohibition codes; retain the useful typed diagnostics.
61. Two documentation stubs received introductions and slot sections rather than unrelated full-topic rewrites; retain the limited documentation scope.
62. Performance samples measured actual CLI and language-server processes instead of rebuilding the investigation benchmark; accept the measurement method and disclose the missing direct comparison.
63. Missed warm and cold timing comparisons were recorded without speculative optimization; retain this response because the spec does not make those quick samples release gates.
64. The authorized whole-type follow-up was created as a draft with unresolved design questions; retain the author-approved sequencing.
65. Editor snapshots retain worker sessions and their dependency paths rather than copying concrete schemas into another store; retain reuse of the library snapshot logic.
66. Editor work is bounded by visits and expansion bytes rather than a wall-clock deadline; retain deterministic work limits.
67. Editor work-limit failures are excluded from the shared cache so commands with larger allowances can retry; retain this separation.
68. Open buffers force dependency rehashing despite unchanged disk metadata; retain unsaved-edit invalidation.
69. Editor quick fixes add deny choices separately from allowance metadata, and run-only grants last until the server exits; retain this editor session scope.
70. Inline editor diagnostic positions are found from authored slot text; repair source preservation under R1-F3 so hosts can share reliable locations.
71. Completion omits stale annotations while hover and diagnostics include them; repair completion under R1-F5 as the spec explicitly requires stale completion to be identified.
72. Pending editor diagnostics are informational, approval-required is a warning, and other failures are errors; retain those severities and explicit stale status during refresh.
73. Pins-only unresolved results retain their replay handle so literal candidates remain inspectable; retain this useful pending information.
74. Unsaved dependency text is supplied only for YAML and JSON; repair the extension restriction under R1-F4 rather than accept narrower supported data formats.
75. Mutation invalidation uses the data-memo generation and dependency rechecks instead of per-path write attribution; retain the existing invalidation mechanism.
76. The mutation test shares one session across compositions to exercise the next validation after a write; retain this test boundary.
77. Context-cache invalidation tests vary environment inputs rather than a separate captured context group; accept the shared digest path without adding a redundant matrix.
78. Slot failures are governed by read grants rather than general recovery flags; retain the documented security boundary.
79. Host allowances match exact case-insensitive hostnames, without wildcard or network-range matching; retain existing host semantics, with IPv6 verification not claimed.
80. Unrelated proposed language-server commands were left outside the slot documentation changes; retain surgical documentation scope.
