---
$schema: feature-review.yaml
ready: false
findings:
    - 'R1-F1 (high): DMLS suppresses required fallback validation'
    - 'R1-F2 (high): Unsaved schema sources do not consistently refresh YAML validation'
    - 'R1-F3 (high): Standalone YAML loses bare-name schema lookup roots'
    - 'R1-F4 (high): YAML schema preparation still runs on the editor request thread'
human_review: true
human_review_items:
    - Accept the selective revert and temporary corpus classifications; retained extraction and navigation work was preserved and temporary classifications were replaced.
    - Keep declaration options in the existing accepted-constraints catalog rather than generalizing the alphanumeric options field.
    - Keep non-null validation based on existing null-preserving conversion and schema lowering; an additional conversion guard is unnecessary.
    - Keep immediate rejection of numeric, boolean, and empty kind literals by constructor narrowing as well as preparation.
    - Keep root closure determined by the winning layer, including support for closed baselines.
    - Keep non-null flags and canonical presence-flag export for nested block-form objects.
    - Accept nearest-authored-ancestor locations for claim failures behind referenced union arms; exact referenced-arm provenance remains deferred.
    - Keep the existing not-a-schema error with the selected-by-kind reason for forbidden yaml-schema references.
    - Keep document-kind identity derived from the envelope enum rather than duplicating a field.
    - Keep trigger envelopes limited to kind, match, and $schema, matching the existing trigger contract.
    - Keep envelope errors ahead of declaration errors and carrier-shape errors, preserving precise declaration-token locations.
    - Keep existing unrestricted conversion shapes for schema declaration carriers without adding a redundant guard.
    - Keep the private process-lifetime cache for compiled envelope validators.
    - Keep the public helpers for embedded kind preparation, referenced-schema admission, and exact problem byte ranges.
    - Keep kind definitions distinct from importable named schemas in corpus classification and editor counts.
    - Keep shipped kind definitions in a separate embedded table from importable catalogs and partials.
    - Keep unreadable roots as selection failures for all string kinds because the unread root could contain a duplicate claim.
    - Keep lexical identification of malformed kind definitions so unrelated broken YAML does not disable unclaimed kinds.
    - Keep category checks in the library parser and reuse them in whole-document semantic validation.
    - Accept trigger-error location through existing field-prefixed messages instead of threading positions through every grammar function.
    - Keep template strings as ordinary data everywhere in whole-YAML validation, including deferred schema positions.
    - Accept text explanations in selection-error causes while the registry retains the underlying typed errors.
    - Accept duplicate-key errors at the containing mapping position supplied by the YAML parser.
    - Keep separate load and validate-loaded entry points for distinguishing input failures from schema failures.
    - Keep unopened arguments classified by their spelled extension for YAML preflight and baseline decisions.
    - Keep the actual validating definition or inline-schema label in YAML JSON result records.
    - Keep YAML-specific load-error wording and available line and column fields.
    - Keep kind registries in existing per-document background registry entries rather than adding a parallel cache.
    - Keep existing saved-file trigger behavior; extending unsaved buffers to triggers is outside this feature.
    - Follow the spec by publishing fallback validation in DMLS; repair the informal-schema exception under R1-F1 without exempting those files in the library.
    - Keep diagnostic ownership rules that report each envelope, declaration, or semantic problem once.
    - Keep established envelope parse-error codes and the ordinary YAML loader code for other documents.
    - Keep unknown YAML keys as errors without requiring editor strict mode.
    - Keep the seven diagnostic-test helper corrections that retain the newest publication.
    - Keep current YAML documentation in schema, indexing, and configuration pages rather than the proposed feature inventory or Markdown key reference.
    - Keep the Messenger migration across roster, override, and mapping files because all three share one loader.
    - Retain existing deliberately invalid drafts and frozen historical fixtures as corpus exceptions rather than redesigning unrelated schemas.
    - Keep the err.yaml correction, deletion of its empty placeholder, and documentation transclusion of the real envelope definition.
    - Keep removal of the stray empty type field from the current Claudine provider schema.
    - Keep yaml-schema in the repository kind catalog and shared-descriptor completion for object constraints.
    - Retain existing translator disagreement for native scalar meta-type arguments as separate pre-existing work; this review found no new realistic failure requiring it here.
    - Review the selection-unavailable wording, union-envelope error wording, kind-check diagnostic name, and accepted required/non-null redundancy after repairs; no change is needed to proceed.
    - Keep broader embedded-schema reference support and inline-code escaping outside this implementation, as the spec explicitly directs.
    - 'R1-F1 repair: an informal schema whose `$schema` names its own `types` now shows the same "unknown named type" error `md schema validate` reports (exit 2); confirm, or make the fallback route read the file''s `types`.'
    - 'R1-F2 repair: unsaved buffers reach resolution through a thread-scoped `simplified_schema::resolve::buffers` snapshot rather than a reader parameter on every resolver function; confirm the seam.'
    - 'R1-F3 disputed: YAML bare-name `$schema` lookup follows the frontmatter policy, which searches schema roots only in a repository (md) or workspace folder (DMLS); decide whether explicit lookup should ignore that gate for both routes.'
    - 'R1-F4 repair: an unprepared YAML contract is a new `Pending` outcome, contracts live in a cache beside the registry entry, and their folders join the DMLS watch set; confirm.'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: '2026-10-31T22:16:23-07:00'
spec: 2026-10-30-yaml-document-schemas/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-30-yaml-document-schemas/log.md
description: 'A **feature** review of `2026-10-30-yaml-document-schemas/spec.md`'
feature: 2026-10-30-yaml-document-schemas/review-1.md
next: 2026-10-30-yaml-document-schemas/review-2.md
---

# Review 1

The feature is **not production ready**. Four high-priority findings remain. All have repairs within the existing spec; none needs an author decision before repair. This is the first review, so there are no previous findings to carry forward.

## Findings

### R1-F1 (high): DMLS suppresses required fallback validation

**Authority: spec**, Design 3: “Otherwise a root `$schema` is the contract,” and Design 4 requires diagnostics from both selection routes. The exception in DMLS's [overlay assembly](../../dmls/src/overlay/mod.rs:469) deliberately discards the whole-YAML result when the same buffer is recognized as an informal standalone schema. The implementation log acknowledges this departure.

**Class:** a secondary content classifier overrides the selected validation route, hiding real validation failures and changing editor assistance.

Open this plausible, unfinished data file through normal protocol messages:

```yaml
$schema:
    owner: string(required)
```

The CLI exits 1 for the missing `owner`; DMLS publishes no schema error. Adding `owner: [ops]` makes DMLS publish a type error. Removing that field makes the error disappear even though the required property is still missing. A user filling out a schema-backed document receives contradictory feedback as they edit.

The sweep follows selection into the report and into the editor overlay used by diagnostics, completion, and hover:

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F1.1 | CLI fallback report | Inline schema with required owner, no data | Exit 1, names owner; clean | Missing-property failure |
| R1-F1.2 | DMLS overlay and diagnostic publication | Same inline schema, no data | No schema error; YAML result discarded | Publish missing-property error from selected schema |
| R1-F1.3 | DMLS fallback with a data property | Same schema plus owner sequence | Type error; clean | Type error |
| R1-F1.4 | DMLS assistance selection | Same no-data buffer | Standalone authoring overlay replaces selected-data overlay | Keep selected-schema data assistance while preserving declaration assistance inside $schema |
| R1-F1.5 | Claimed-kind route | Existing roster and shipped-schema fixtures | Definition diagnostics and assistance retained; clean | Claiming definition remains authoritative |

The library's fallback selection is unconditional after kind selection and returns its report; the suppression occurs in DMLS, after that result exists. Existing library selection tests and the CLI reproduction confirm that the underlying validator is not omitting the required field.

**Repair:** attach and publish the YAML result for informal envelopes too. Preserve schema-authoring assistance within declaration values and prevent duplicate parse reports. Update tests that currently depend on the exception. Do not adopt the log's proposed library exemption: that would contradict the decided selection rule. Add a protocol test that deletes the last data property and checks diagnostics, completion, and hover.

### R1-F2 (high): Unsaved schema sources do not consistently refresh YAML validation

**Authority: spec**, “Editor lifetime”: “Watch definitions, imported dependencies, and searched missing locations; creation, deletion, rename, or an unsaved edit must refresh affected open YAML documents.” Discovery also requires cached source text, including open unsaved definitions, through the shared discovery path.

**Class:** editor buffers are applied only to existing root candidates, while dependency resolution and root enumeration continue to use saved files.

DMLS's [BufferedRoots](../../dmls/src/overlay/registry.rs:273) substitutes buffer text only after disk enumeration finds a root file. [Registry preparation](../../dmls/src/overlay/registry.rs:532) explicitly reads imports from disk. The [fallback YAML overlay](../../dmls/src/overlay/yaml_document.rs:202) also resolves targets through the filesystem. These paths do not share one view of the editor's source text.

Reproduce with a roster whose `owner` is imported from a normal `kind: schema` type library. Start with `Owner: string` and data `owner: ops`. Open the type library and change it to `Owner: number` without saving. The roster remains error-free. Save the identical text and send a watched-file change: the roster immediately reports a type mismatch. The contract changed before the save, but the dependent document kept the old result.

Normal protocol probes swept all three source boundaries and both import locations:

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F2.1 | Existing root definition buffer | Change owner string to number in roster definition | Dependent error refreshes; clean existing session test | Refresh before save |
| R1-F2.2 | Imported named types inside a schema root | Change Owner string to number in schemas/types.yaml | No dependent error until saved | Refresh from unsaved imported text |
| R1-F2.3 | Imported named types outside schema roots | Same change in shared/types.yaml, imported relatively | No dependent error until saved | Same outcome as root-local import |
| R1-F2.4 | Explicit fallback schema target | Change owner string to number in contract.yaml | No dependent error until saved | Refresh from unsaved target text |
| R1-F2.5 | New root definition buffer | Open schemas/roster.yaml before it exists on disk | Kind remains unclaimed until file is created on disk | Discover open definition and validate dependent data |
| R1-F2.6 | Saved dependency and definition changes | Save the same edits and send change/create events | Expected type error appears in every probe; clean | Same validation result before and after save |
| R1-F2.7 | Existing definition close without save | Existing session fixture | Saved definition restored; clean | Refresh back to saved text |

Each failing probe used a passing saved control and changed only one source declaration or its presence on disk. The dependent diagnostic is the public result; root-local and external imports were tested separately. The new-file case used an existing schemas directory, so the result does not depend on creating a directory too.

**Repair:** give preparation a source reader that overlays open buffers for definitions, imports, and explicit schema targets; merge eligible open paths into root candidate enumeration. Track those buffer versions as dependencies, including creation and closing. Reuse the library resolver rather than adding a second parser. Add separate protocol tests for the four failing rows, including completion/hover refresh and restoration on close.

### R1-F3 (high): Standalone YAML loses bare-name schema lookup roots

**Authority: spec**, Discovery: “Shipped kinds remain available everywhere, and an explicit root `$schema` still works outside those gates,” and “Schema-file lookup continues to use the shared selector and its existing policy.” Design 3 requires frontmatter's declaration syntax and reference resolution for the fallback route.

**Class:** the gate on automatic kind discovery also removes the roots required for explicit schema-reference lookup.

The library [takes lookup roots from the kind registry](../../lib/src/markdown/schemas/yaml_documents/mod.rs:151). A shipped-only registry contains no roots. Outside a repository, or outside an editor workspace, `$schema: contract.yaml` therefore searches as a local file instead of finding `SCHEMAS_DIR/contract.yaml` or `~/schemas/contract.yaml`.

Use a normal schema export:

```yaml
kind: schema
$schema:
    owner: string(required)
```

Put it in `SCHEMAS_DIR/contract.yaml`, and validate a YAML document outside a Git repository containing `$schema: contract.yaml` and `owner: ops`. The CLI exits 2. Using the schema's absolute path passes. Moving the document into the fixture repository, with the same captured `SCHEMAS_DIR`, also passes through the library API.

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F3.1 | Library, repository discovery gate closed | Bare reference to SCHEMAS_DIR file | Resolution failure | Resolve explicit contract using captured roots |
| R1-F3.2 | CLI, no repository | Same reference and environment | Exit 2 | Exit 0 |
| R1-F3.3 | DMLS, no workspace folders | Bare reference to fixture HOME/schemas file | Schema diagnostic | Resolve explicit contract successfully |
| R1-F3.4 | Library, gate open | Same SCHEMAS_DIR reference inside repository | Valid; clean | Valid |
| R1-F3.5 | Library, CLI, DMLS explicit-path route | Absolute path to same schema outside gates | Valid; clean | Valid |
| R1-F3.6 | Claimed-kind discovery outside gates | Existing shipped-only registry fixture | User kind not discovered; clean | Automatic discovery remains disabled |

**Repair:** enumerate explicit-reference lookup roots from the document context independently of whether automatic kind discovery is enabled. Supply those same roots to validation and editor reference assistance. Keep the repository/workspace gates on discovery. Add API and wiring coverage outside both gates with a real basename including its extension.

### R1-F4 (high): YAML schema preparation still runs on the editor request thread

**Authority: spec**, “Editor lifetime”: “Prepare and compile outside the request dispatch thread, and reuse the result while its sources and context are unchanged.” Acceptance also says to reuse prepared registries and compiled validators rather than scan or compile on each editor request.

**Class:** only registry discovery moved to the background worker; final schema preparation and compilation remain synchronous in document-overlay construction.

DMLS calls [yaml_document](../../dmls/src/overlay/mod.rs:465) synchronously under the overlay-cache lock. Its [build function](../../dmls/src/overlay/yaml_document.rs:202) calls Darkmatter's whole-document validator, which resolves fallback schemas and invokes [EffectiveSchema::for_yaml_document](../../lib/src/markdown/schemas/mod.rs:1197). That function builds compiled validators on a cache miss. Registry preparation stores resolved definitions, not those final compiled validators. The implementation log itself describes the YAML overlay as “dispatch-thread only.”

A normal first open of a new schema-backed service file reaches this path; no large or hostile input is required. Editing only a data value invalidates the document cache and repeats fallback resolution even while the contract and context are unchanged. The shared compiler cache can avoid recompilation on a hit, but does not move a first compilation off the request thread or cache fallback preparation independently of data revisions.

The sweep uses the roster and service protocol fixtures exercised in this review. Thread placement below is established by the synchronous call chain, not a latency benchmark:

| Instance | Site | Shape tested | Observed | Expected |
|---|---|---|---|---|
| R1-F4.1 | Kind registry discovery/preparation | Open roster with discovered definition | Background registry job; clean existing thread test | Background work |
| R1-F4.2 | Claimed-kind validator construction | First open of roster with a new contract | Final compiled-validator build called on request thread | Compile during background preparation |
| R1-F4.3 | Inline fallback preparation | Open service with inline root $schema | Resolve/lower and final compile on request thread | Background preparation/compilation |
| R1-F4.4 | Referenced fallback preparation | Open service naming contract.yaml | File resolution, preparation, and final compile on request thread | Background preparation/compilation |
| R1-F4.5 | Reuse after data-only edit | Change service data while contract/context stay fixed | Re-enters fallback resolver because full text/revision keys the cache | Reuse prepared contract; only validate changed data |
| R1-F4.6 | Built-in kind semantic imports | Open kind schema/yaml-schema with named imports | Semantic checks can resolve imports through the synchronous validation call | Prepare dependency work outside request dispatch |

The current thread test observes `registry_scan_threads()` only. It cannot establish where validator compilation or fallback resolution happens, so its passing result does not prove this acceptance criterion.

**Repair:** extend existing background preparation to produce and cache the selected YAML contract and compiled validators, including fallback references and dependency checks. Key contract reuse on declaration, context, and sources, separately from data text. Keep passive validation against that prepared state on the request path. Extend the thread evidence to actual resolver/compiler work and prove a data-only edit performs no schema-file preparation. No multi-host performance benchmark or new test tier is needed.

## Verification and input robustness

All requirements here concern passive parsing, validation, CLI records, and language-server protocol results. **Level 1 is appropriate**; no keyboard encoder, terminal rendering, browser, or focus-dependent behavior is promised by this feature. No Level-2/Level-3 mismatch was found.

Executed during this review:

- `just test yaml_` in darkmatter: **316 passed**, including API selection/loading/registry tests, CLI YAML wiring, protocol refresh tests, and Unicode/CRLF ranges.
- `just test kind_semantic_checks` in darkmatter: **9 passed**.
- `just test shipped_schema_corpus` in darkmatter: **12 passed**, including current kinded artifacts, shipped definitions, and explicit draft exceptions.
- `just test` in simplified-schema: **1,396 default-build tests and 1,209 minimal-feature tests passed**, including declaration narrowing, non-null behavior, closure, envelopes, canonical export, and layout gates.
- `just check-tier-coverage darkmatter`: **zero stranded tests**. The changed YAML test modules are declared in each package's L1 target.
- Eight temporary L1 review probes exercised public validation, the normal CLI fixture, and normal editor protocol messages. Their assertions confirmed the wrong current outcomes and saved/explicit-path controls described above; all eight passed. They were removed after review, leaving production code and tests unchanged. These are diagnostic evidence, not permanent regression tests.

The admitted-YAML loader is shared. Its public tests check duplicate keys, malformed content, a second document, non-string keys, inadmissible numbers, document markers, BOM/CRLF, and scalar/sequence/empty roots. Invalid documents produce load errors before selection; valid non-mapping roots remain unvalidated. The CLI tests prove parse versus preparation exit codes and mixed-batch precedence.

Default field outcome is a located error for an invalid declaration or selected-schema value. Exceptions are explicit: absent/null/non-string kind selects no claim; missing $schema permits no selection; a present invalid fallback $schema fails; absent optional non-null properties pass but explicit null fails; empty values are judged by their declared type. Existing optional unmarked properties retain their null behavior. Soft conversion preserves declaration carriers and never executes template/shell strings.

The public matrix in `yaml_document_selection`, the envelope admission matrix, and the full SimplifiedSchema suite cover absent, null, wrong whole-field type, mixed/all-invalid collection elements, and empty values at their shared validators. The fallback suppression finding concerns the editor's projection after those validators: the same declaration with no data is suppressed, while adding invalid data publishes a failure. Library/CLI controls and the ordinary-data editor control are included in its sweep. The root-lookup finding concerns context gating rather than an untested field coercion.

The required code-smell search was run over envelope admission, claim extraction, YAML loading/selection/registry, and the editor YAML overlay. The defaulting in the key traversal follows an explicit rejection of non-string keys; optional numeric position conversions and failed prepared-shipped lookups do not coerce configuration values into permissive defaults. Failed-reference union arms in claim recovery mark the registry incomplete rather than silently succeeding. No additional finding arose from that search.

Cross-OS proof is left to CI as requested. No formatting command, commit, terminal window, or browser was used.

## Implementation decisions (informational)

Two agents independently read the implementation log and spec in parallel before the comparison above. The spec explicitly permits selective reversion of mixed commits, and excludes broader embedded-resolution work and inline-code escaping. It does not authorize changing fallback selection or ignoring unsaved dependencies. Every item below can be accepted or repaired within the existing decisions; none passes the threshold for stopping for a human. The frontmatter gives one-line summaries for later author inspection. The author may confirm or reverse these choices after the repair cycle; no response is required now.

| Choice or deferred work | Review disposition and reason |
|---|---|
| Selective revert; temporary unclaimed/draft corpus classes | Accept. Preserved unrelated work and was explicitly permitted; temporary exceptions were replaced. |
| Declaration options listed as accepted constraints | Accept existing catalog representation; no new catalog abstraction is needed. |
| No extra non-null conversion guard or wrapper | Accept. Public null-preservation and strict/soft tests establish the intended rejection. |
| Reject non-string/empty kind constructors immediately | Accept. Matches the spec's finite nonempty string-claim rule. |
| Winning layer controls closure; closed baselines admitted | Accept. Merged declarations are visible before closure rejects unknown keys. |
| Nested block-form non-null and presence-flag export | Accept. Makes the stated nested-object contract expressible and round-trippable. |
| Approximate referenced-union claim locations | Accept nearest authored reference location; exact arm provenance remains deferred. This avoids inventing inaccessible inlined source locations. |
| Reuse not-a-schema error with selected-by-kind reason | Accept. Clear refusal using the established typed classification. |
| Derive kind from envelope enum | Accept. Avoid redundant identity state. |
| Trigger envelope has only kind/match/$schema | Accept. No new trigger metadata contract is required. |
| Envelope/declaration/carrier error precedence | Accept. Reports an actionable envelope error first and keeps precise token locations when available. |
| No additional declaration-carrier conversion guard | Accept existing unrestricted carrier shape and tests proving preservation. |
| Private persistent envelope-validator cache | Accept. Avoids coupling language admission to host benchmark-cache resets. |
| Three added preparation/location helpers | Accept. They serve current library/host consumers. |
| Distinct corpus/editor classification and embedded kind table | Accept. Kind definitions are not importable named schemas. |
| Unreadable roots fail all string-kind selection | Accept conservative duplicate-claim handling; no-kind documents remain unaffected. |
| Malformed candidate requires lexical yaml-schema identification | Accept. Unrelated broken YAML should not disable unclaimed kinds. |
| Reuse library category checks | Accept. Keeps direct library loads enforcing the same rules. |
| Recover trigger-error location from field-prefixed message | Accept bounded reuse of the existing grammar's location convention. |
| Disable all template deferral for YAML | Required by the spec; accept. |
| Text in selection causes, typed defect retained in registry | Accept current diagnostic representation; no new loss requiring a separate repair was demonstrated. |
| Duplicate keys point to parser's containing mapping | Accept located error without adding a lexical duplicate-key finder. Exact second-occurrence highlighting is deferred. |
| Separate load/validate-loaded APIs | Accept. Correctly distinguishes document parse failure from referenced-schema failure. |
| Unopened argument classified by extension | Accept deterministic preflight when no resolved path exists. |
| Actual definition label in JSON schema field | Accept useful source identification within the existing record format. |
| YAML-specific load-error wording and positions | Accept. Same failure categories and exit-code contract. |
| Kind cache stored in existing background registry entries | Accept internal structure; actual source coverage and compilation placement still require R1-F2/R1-F4. |
| Unsaved kind buffers without changing trigger behavior | Accept scope boundary. Unsaved kind dependencies and explicit targets must still work under R1-F2. |
| Suppress informal-schema data diagnostics | Reject through R1-F1. Recommend publishing the decided fallback result, rather than changing library selection. |
| Editor diagnostic ownership/deduplication | Accept the intent to publish one problem; preserve it while fixing R1-F1. |
| Existing envelope parse codes retained | Accept compatibility and single reporting. |
| Unknown YAML keys always errors | Accept agreement with CLI and the selected closed definition. |
| Seven newest-diagnostic helper fixes | Accept. They observe current editor results rather than older buffered publications. |
| Documentation pages chosen by current subject | Accept relevant current pages rather than proposed features/Markdown-only keys. |
| Messenger shared-loader migration covers three contract families | Accept coherent whole-YAML behavior across the same existing loader, including fixtures. |
| Eleven failing drafts and frozen historical inputs left unchanged | Accept as intentionally non-current/negative corpus artifacts, not shipped usable definitions. Designing unrelated lifecycle sketches or rewriting frozen invalid inputs would not improve this feature's correctness. Their existing exception status remains explicit. |
| err.yaml cleanup, placeholder deletion, real-definition transclusion | Accept removal of an empty exported contract; current examples now show the real definition. |
| Remove empty type field from current provider schema | Accept fixing a real current corpus input, without redesigning draft files. |
| Kind catalog and descriptor-driven constraint completion | Accept required discoverability of the new kind and constraints. |
| Existing native-scalar meta-type argument translator disagreement deferred | No additional finding: the log identifies pre-existing behavior, and this review did not establish a realistic new wrong result through it. This is not a guarantee that the translators agree. |
| Selection-unavailable wording, union-envelope wording, diagnostic name | Informational only. Recommend current wording/naming; no author gate. |
| required plus non-null redundancy | Recommend accepting the redundant combination, as the implementation/tests do; it has an unambiguous result and needs no special rejection. |
| Broader embedded resolver refusals and inline-code escaping deferred | Accept explicit spec exclusions. |

## Readiness

Repair R1-F1 through R1-F4 and add the specified regression evidence before marking this feature ready. Every finding is unblocked. There are no pending human rulings and no recurring findings from an earlier review.
