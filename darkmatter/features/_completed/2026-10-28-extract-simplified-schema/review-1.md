---
$schema: feature-review.yaml
next: 2026-10-28-extract-simplified-schema/review-2.md
ready: false
findings:
    - 'R1-F1 (high): Root-union validation drops raw JSON Schema assertions'
    - 'R1-F2 (high): Minimal dependency graph retains explicitly excluded globset'
    - 'R1-F3 (high): Default dependency graph includes the HTTP effect stack'
    - 'R1-F4 (high): Shipped model-config documents fail their schema contract'
    - 'R1-F5 (high): Schema factories lose typed grammar errors and document locations'
human_review: true
human_review_items:
    - "Keep the passive frontmatter helper behind biscuit-file’s YAML feature; minimal schema builds already enable it."
    - "Keep resolver-dependent portable-path objects behind full file support; expose only passive identity and strategy types in the syntax feature."
    - "Feature-gate the Git error variant rather than change its existing payload type."
    - "Put the inventory beside the actual October 28 specification, correcting the plan’s stale directory."
    - "Keep terminal error presentation in an optional, nondefault library feature to preserve borrowed error rendering without an unsafe host wrapper."
    - "Move the pending-token codec and plain expression scanner with their passive validation consumers."
    - "Expose scanner functions while keeping the Markdown-aware expression finder in Darkmatter."
    - "Leave the two span helpers used only by Darkmatter in Darkmatter."
    - "Declare optional repository discovery before its factories land; the final implementation uses it."
    - "Keep Darkmatter’s public span and literal-token paths and its crate-private pending-token helper; remove the temporary numeric path."
    - "Keep token round-trip tests at both the plain YAML and actual Markdown frontmatter boundaries."
    - "Use library-owned optional work counters and add their measurements to Darkmatter’s existing counters."
    - "Keep the original Claudine parser-regression fixtures as declared cross-package test inputs rather than copy them."
    - "Test passive syntax checks at their owning boundary during the staged move, then move the checks with their implementation."
    - "Expose the operator-precedence table as a documentation-hidden item so Darkmatter can check catalog parity."
    - "Keep Darkmatter’s runtime expression-error module; passive lexer and parser errors move with the parser."
    - "Keep the moved language modules in a flat layout; the facade supplies the ergonomic entry point."
    - "Rename Darkmatter’s schema-search-root type to avoid collision with the extracted schema AST type."
    - "Pass shipped sources explicitly into library resolution and retain host-aware Darkmatter wrappers."
    - "Use a Darkmatter extension trait for trigger-origin operations instead of free functions."
    - "Add a typed resolution-unavailable file error and map it to the existing missing-context category."
    - "Run filesystem-dependent tests with file support and separately test unavailable checks in the minimal build."
    - "Reuse the existing recursive catalog goldens rather than add duplicate expected output."
    - "Move shared compiled validators and problem collection while leaving phase and trigger policy in Darkmatter."
    - "Move validator-cache environment tuning with the cache and cover it with the library’s ambient-state guard."
    - "Introduce reusable prepared validation before adding the facade’s revision-based cache."
    - "Classify mixed definite violations and unavailable checks as invalid data while preserving each problem."
    - "Classify unavailable file and glob checks using build capability and a private context-availability flag."
    - "Return nonfatal schema-resolution advisories with successful validation."
    - "Use one facade error type and a borrowed function-signature view with planning-selected names."
    - "Accept one convertible file-source argument carrying an optional captured context and unsaved text."
    - "Keep content references unresolved until an explicit file-factory load; omit a separate later-binding method."
    - "Cache a typed unprepared-validation failure when a definition cannot be lowered or compiled."
    - "Treat file checks without a content schema’s file context as unavailable rather than invalid data."
    - "Recognize raw JSON Schema content by its dialect URI while retaining the explicit JSON file factory."
    - "Add named-object selection so callers can validate one type from a catalog."
    - "Keep typed layers beside merged JSON Schema, preserve the receiver’s context, and reuse preparation when the merged definition is unchanged."
    - "Export nested objects and property unions structurally and reject representations that cannot be exported without losing information."
    - "Resolve a final tuple index in the facade while preserving the existing query helper’s unsupported deeper traversal and union-result behavior."
    - "Keep the standalone facade free of Darkmatter’s shipped namespace; host resolution wrappers supply it."
    - "Share repository-scope projection between standalone capture and Darkmatter and preserve discovery failures."
    - "Keep local asynchronous resolution independent of an async runtime and choose the loader by file extension."
    - "Keep capture and read counters private to tests and serialize tests that change ambient process state."
    - "Keep Darkmatter’s shared effective-schema representation and prove parity with facade layering instead of constructing facade objects per page."
    - "Remove all migration-only language re-exports and make schema consumers depend directly on the library."
    - "Keep the YAML loader’s existing limit on very long JSON object keys; the explicit JSON file factory remains available."
    - "Reject the implementer’s acceptance of unused HTTP dependencies in the default graph; repair it through feature narrowing in finding R1-F3."
    - "Reuse existing source-guard engines through declared test inputs rather than move seven consumers to another support crate."
    - "Allow exactly one optional file-source context until factory entry captures or reuses it."
    - "Check single ownership with a table of 45 language symbols, excluding known delegating name collisions."
    - "Check trigger-language parts in the library and host envelopes in Darkmatter; use representative sidecar contracts for the library corpus."
    - "Keep doctests in the existing separate recipe and run that recipe explicitly."
    - "Split language documentation into library topic pages while retaining Darkmatter’s integration page and its host-specific anchors."
    - "Rewrite moved comments in host-neutral terms while retaining useful explanatory host examples."
    - "Resolve the deferred model-config document mismatch by converting scalar notes to one-item lists under the existing contract; finding R1-F4 needs no author decision."
    - "Accept completed intermediate deferrals: check-level tests, shared preparation, revision caching, and the ambient-state guard all landed in later phases."
    - "Keep unsupported serializer improvements, deeper tuple traversal, later content binding, and a neutral guard-support crate outside this extraction; existing behavior or typed unsupported results define the boundary."
    - "Keep full research-document enumeration out of the library corpus; repair the seven known-invalid documents at their owning consumer boundary."
    - "Keep doctest automation consistent with existing archive recipes; the recorded manual run supplies this review’s evidence."
    - "Accept the recorded benchmark results as showing no material regression; no author disposition is needed for the measured noise."
    - "Reuse earlier cross-platform evidence for the final documentation-only phase and do not make additional OS evidence a readiness gate."
    - "Accept use of the canonical scope calculator during implementation without a separate local-plan command; no push is requested in this review."
    - "Leave the pre-existing links to unwritten schema-index pages outside this code review; no read code produces an incorrect result from those links."
    - "Keep a schema CLI, remote-schema fetching, expression execution extraction, and code generation outside the agreed release scope."
    - "R1-F1: carry a raw root's sibling assertions into each root-union arm (arm under `allOf` only when needed) instead of compiling the whole root; generated unions keep identical arms and schema paths."
    - "R1-F1: reject a malformed root `anyOf` (empty or a non-schema arm) through jsonschema's existing `Unprepared(BuildValidator)` instead of a hand-written error."
    - "R1-F1.17: convert a caller candidate against the arm, then against the root's sibling keywords, rather than against the `allOf`-wrapped arm, which drives no conversion."
    - "R1-F5: extend `SchemaError::SchemaDocument` with `source` and `span` rather than add a wrapping variant, keeping DMLS attribution and `md schema` output unchanged."
    - "R1-F5: the facade's Markdown frontmatter grammar failure is now `SchemaDocument` wrapping `Grammar` instead of a bare `Grammar`."
    - "R1-F5: `span` is the rejected token when projectable, otherwise the whole rejected value; successful layers do not retain standalone source maps."
    - "R1-F3: gate sniff's `schematic-schema` on `remote` rather than `network`, because only remote code uses it."
    - "R1-F4: repair all ten model-config documents (three beyond the review's seven) and teach the claudine-gen gate to expand the sidecar's own `types:`; catalog regenerated."
human_in_the_loop:
    - |-
        The specification requires the minimal in-memory library to exclude `globset` while preserving existing file-reference grammar. The existing parser uses `globset` to reject malformed glob patterns, even when no files are read. The implementation retained that dependency. This meets the "Contradicts a decision" test: a simple feature switch either retains an expressly forbidden dependency or changes which schemas parse, and the spec does not authorize either departure. No bounded feature-only repair satisfies both requirements without replacing or factoring the parser.

        Options:

        - Permit `globset` for passive syntax validation in the minimal build, while still excluding filesystem discovery and resolution. This preserves one parser and current behavior, at the cost of the small parsing dependency. Recommended: it protects compatibility without creating another grammar to maintain.
        - Require a shared extraction of the existing glob parser so the minimal build excludes the `globset` package. This can preserve grammar but expands the dependency refactor and creates a maintenance obligation beyond the current feature split.
        - Make glob validation unavailable in minimal builds. This reduces dependencies but changes which schemas can be used there and needs an explicit compatibility exception.

        Please choose which contract to retain or explicitly authorize the larger parser refactor. Only finding R1-F2 waits on this ruling; the other findings have concrete repair recommendations.
human_resolved: true
has_blocked_findings: true
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-30T15:18:17-07:00
spec: 2026-10-28-extract-simplified-schema/spec.md
implemented: true
description: 'A **feature** review of `2026-10-28-extract-simplified-schema/spec.md`'
feature: 2026-10-28-extract-simplified-schema/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-28-extract-simplified-schema/log.md
implemented_by: claude/opus
---

# Review 1: Extract SimplifiedSchema

The feature is **not production ready**. Five findings require repair. Only the minimal-build glob-parser dependency needs an author ruling; the other four are actionable now. This is the first review, so there are no previous findings or recurring classes to close.

Two agents independently read the implementation decisions and the specification in parallel. Their inventories were reconciled against the code and acceptance criteria. Routine API names, file placement, internal structure, and equivalent convenience methods were explicitly delegated to planning. That permission does not waive the dependency requirements.

## Findings

### R1-F1 (high): Root-union validation drops raw JSON Schema assertions

**Authority — defect:** In the simplified-schema package, [compiled validation](../../../simplified-schema/lib/src/prepared.rs:52) treats every root `anyOf` as a generated SimplifiedSchema union. Its [arm projection](../../../simplified-schema/lib/src/validate.rs:1418) keeps definitions but discards sibling assertions. A realistic raw JSON Schema declaring a required `name` beside `anyOf` therefore accepts an object without `name`. This affects the extracted library and Darkmatter. It is a defect in the reviewed code, not a claim that extraction introduced a regression.

**Class:** projecting a schema to its alternatives changes the validation contract by dropping assertions or replacing non-object alternatives with unconstrained objects.

Reproduce through the public library API:

```rust
use simplified_schema::SimplifiedSchema;
use serde_json::json;

let schema = SimplifiedSchema::try_from(r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["name"],
  "anyOf": [
    {"properties": {"kind": {"const": "a"}}},
    {"properties": {"kind": {"const": "b"}}}
  ]
}"#).unwrap();
let data = json!({"kind": "a"});
// Both currently return Ok, although name is required.
assert!(schema.validate_strict(&data).is_ok());
assert!(schema.validate_soft(&data).is_ok());
```

The same projection turns `false` arms into `{}` and accepts malformed arms such as `7`. An empty `anyOf` also produces no collected problems and succeeds. These are siblings of the realistic missing-required failure, not separate findings.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Content factory, strict validation | Reproducer above | Accepted | Missing-required failure |
| R1-F1.2 | Content factory, soft acceptance shortcut | Same schema and value | Accepted unchanged | Missing-required failure |
| R1-F1.3 | JSON file factory | Same bytes in `schema.json`, both modes | Both accepted | Both reject |
| R1-F1.4 | YAML file factory, raw-schema classification | Same JSON-compatible YAML bytes, both modes | Both accepted | Both reject |
| R1-F1.5 | Asynchronous `resolve`, JSON and YAML loaders | Same files, both modes | Both accepted | Both reject |
| R1-F1.6 | Public `Prepared` validation | Same schema as an `Arc<Value>`, both modes | Both accepted | Both reject |
| R1-F1.7 | Darkmatter effective-schema normal validation | Document references that `schema.json`; data is the same object | `valid: true` | Missing-required problem |
| R1-F1.8 | Darkmatter effective-schema raw validation | Same document and data | `valid: true` | Missing-required problem |
| R1-F1.9 | Root arm wrapper and downstream problem collector | `anyOf: [false, false]` | Accepted | Reject every value |
| R1-F1.10 | Root arm wrapper | `anyOf: [7, {required: [name]}]` and `[7]` | Accepted | Typed schema-preparation failure |
| R1-F1.11 | Root-union compilation and problem collection | `anyOf: []` | Accepted | Typed schema-preparation failure |
| R1-F1.12 | Ordinary object compilation, clean control | Remove `anyOf`, retain root `required` | Rejected with `/required` problem | Rejected |
| R1-F1.13 | Object union-arm validation, clean control | Arm itself declares `required: [name]` | Rejected | Rejected |
| R1-F1.14 | Generated SimplifiedSchema root unions, clean control | Existing valid union and coercion fixtures | Existing library tests pass | Preserve their acceptance and diagnostics |

The value reaches two independent decisions in `CompiledValidators`: `accepts` controls the soft fast path, while `collect_problems` controls strict validation and the final verdict after conversion. Both must retain the complete schema contract. Darkmatter uses the same builder and collector, including after phase projection. Its rewrite/selection code and the coercion pending-value fallback also call `wrap_arm_as_root_schema`; audit those callers when changing the shared helper. With no pending values the coercion fallback checks its complete structural validator, so that branch does not itself introduce an additional projection loss.

**Recommended repair:** validate raw schemas against their complete root, preserving valid boolean arms and rejecting malformed schemas before arm projection. Keep specialized per-arm reporting for generated unions only where its invariants are proved, or combine root sibling assertions with each projected arm without changing reference scope. Do not build a new general validation engine. Add public strict/soft and host-path regressions for the primary reproducer and the sibling shapes above. Review the projection comments with the behavior change.

### R1-F2 (high): Minimal dependency graph retains explicitly excluded globset

**Authority — spec:** Requirement R2 and acceptance criterion 14 explicitly require the no-default-features graph to exclude `gix`, `walkdir`, and `globset`. [Biscuit-file's syntax feature](../../../biscuit-file/lib/Cargo.toml:31) enables `globset`, and [the schema library](../../../simplified-schema/lib/Cargo.toml:41) requires that feature. The implementation log acknowledges the departure but cannot substitute its own acceptance ruling for the spec.

**Class:** an optional-feature boundary still enables a dependency explicitly excluded from its minimal configuration.

Reproduction: `cargo tree -p simplified-schema --no-default-features -e normal --prefix none`. This is the normal dependency graph of the schema-only example as well as the library.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Library minimal build | Default features disabled | `globset v0.4.18` present | No `globset` under the current spec |
| R1-F2.2 | Biscuit-file passive-reference feature | Transitively selected syntax feature | Explicitly enables `dep:globset` | Boundary consistent with the final ruling |
| R1-F2.3 | Library minimal filesystem dependencies, clean | Same graph | No `gix`, `walkdir`, or `sniff` | Absent |
| R1-F2.4 | Default file build, clean | Default features enabled | Resolver dependencies present | Permitted in this configuration |

**Blocked only on Human item 1.** Retaining parse-time checking is a sound compatibility goal, and silently skipping the check is not a repair. My recommendation is to permit this passive parser dependency explicitly. If the author instead retains the exact dependency exclusion, implement and verify the authorized parser refactor. Update the graph evidence and feature documentation to the ruling; do not describe the current graph as satisfying the unmodified criterion.

### R1-F3 (high): Default dependency graph includes the HTTP effect stack

**Authority — spec:** Requirement R2 says to use the smallest required dependency features and avoid Darkmatter's rendering and effect stack. Acceptance criterion 14 requires that boundary in both configurations. [The default library dependency on sniff](../../../simplified-schema/lib/Cargo.toml:42) reaches its unconditional [schematic-schema dependency](../../../sniff/lib/Cargo.toml:44), which brings in an HTTP client and runtime even though the file adapter uses repository discovery only.

**Class:** an unrelated unconditional transitive feature defeats the standalone dependency boundary.

Reproduction: `cargo tree -p simplified-schema -e normal --prefix none` includes `reqwest v0.13.2`, `hyper v1.9.0`, and `tokio v1.52.1`. The implementation's source guards establish that the library does not call them; they do not establish their absence from its Cargo graph.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Default schema-only build | Normal dependencies | HTTP client and runtime present | No HTTP effect stack |
| R1-F3.2 | Repository-discovery dependency | `sniff` without remote features | Unconditional `schematic-schema` still reaches its runtime | Repository-only use avoids remote dependencies |
| R1-F3.3 | Minimal schema-only build, clean | Default features disabled | No `sniff`, `reqwest`, `hyper`, or `tokio` | Absent |
| R1-F3.4 | Rendering and host dependencies, clean | Default and minimal normal graphs | No Darkmatter, DMLS, Claudine, Content Policy runtime, or biscuit-terminal | Absent; opt-in terminal presentation is separate |

**Recommended repair:** make sniff's remote-only schema dependency optional and enable it from its existing remote feature, after checking all uses. Retain Sniff for repository discovery; duplicating its discovery code is unnecessary. Verify repository-only and remote builds and repeat both normal graph checks. This is a concrete feature correction consistent with the spec and needs no human ruling. The recorded decision to defer it to another package does not satisfy the delivered library's graph requirement.

### R1-F4 (high): Shipped model-config documents fail their schema contract

**Authority — defect:** Claudine's shipped [model-config sidecar](../../../claudine/docs/research/model-config/_schema.yaml:76) declares `notes: string[]` in three record types, while its seven provider documents supply scalar strings. The real generator validates these documents and returns `ResearchInvalid`, preventing catalog generation. All four failures named in the implementation log reproduced through the existing public-pipeline tests. This is a pre-existing data/contract defect found during review, not an extraction regression.

**Class:** shipped data uses a scalar where its consuming schema requires a collection.

The bounded sweep loaded the real sidecar through `SimplifiedSchema::try_from_yaml`, extracted each real document's frontmatter with the shared helper, and called `validate_soft` on that data. Every failure was a `notes` field. A control changed only scalar `notes` values into one-item arrays in memory; all seven documents then passed. No production files were changed for this experiment.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F4.1 | `model-config/claude.md` | 11 scalar notes across the three record arrays | Rejected; corrected control passes | Shipped document satisfies its sidecar |
| R1-F4.2 | `model-config/codex.md` | 11 scalar notes | Rejected; corrected control passes | Same |
| R1-F4.3 | `model-config/gemini.md` | 11 scalar notes | Rejected; corrected control passes | Same |
| R1-F4.4 | `model-config/goose.md` | 13 scalar notes | Rejected; corrected control passes | Same |
| R1-F4.5 | `model-config/kimi.md` | 12 scalar notes | Rejected; corrected control passes | Same |
| R1-F4.6 | `model-config/opencode.md` | 16 scalar notes | Rejected; corrected control passes | Same |
| R1-F4.7 | `model-config/qwen.md` | 15 scalar notes | Rejected; corrected control passes | Same |
| R1-F4.8 | Three named record validators | `model_config_paths[].notes`, `api_standards[].notes`, `local_runners[].notes` | All report the expected array-type error | Keep validation; repair the data |
| R1-F4.9 | Generator's normal research gate | Existing control, repeated-spelling, legacy-revision, and compiled-provider tests | All four fail before switch generation | Valid shipped inputs reach their intended behavior |
| R1-F4.10 | Array-form notes, clean control | Same documents, only 89 scalars wrapped as one-item arrays | All seven pass | Pass |

**Recommended repair:** wrap the 89 scalar notes in one-item arrays in their owning documents, preserving every note and the existing collection contract. Do not add coercion or weaken the schema. This is mechanical data repair; the implementer's statement that choosing a repair is reserved for the author is unnecessary. Re-run the four generator tests and the seven-document public validation check after repair. The existing field validator correctly rejects the scalar shape; this finding requires no new validator or synthetic field matrix.

### R1-F5 (high): Schema factories lose typed grammar errors and document locations

**Authority — spec:** Requirement R6 says errors must retain typed causes, source identities, and spans, and that standalone offsets refer to the exact supplied text, including CRLF. In the simplified-schema package, [standalone parsing](../../../simplified-schema/lib/src/simplified/standalone.rs:191) formats grammar failures into `SchemaDocument.message`; [Markdown loading](../../../simplified-schema/lib/src/schema/layer.rs:176) returns a type-string-relative `Grammar.span` without a source document or the frontmatter offset. The facade also discards the standalone source maps when constructing its layer.

**Class:** a loading adapter erases typed diagnostic provenance or exposes a local substring offset as the only available location.

Use valid YAML containing the malformed type string `age: 'number(min('`. For the Markdown case, put it after `title: test` in frontmatter. Use CRLF throughout. YAML factories return `Schema(SchemaDocument { path, message: "... expected argument, found Eof" })` with no retained grammar cause or span. Markdown factories return `Schema(Grammar { property: "age", span: 11..11, ... })`; even the file factory returns no filename, and `11` indexes the type expression rather than the supplied document. Callers cannot identify and locate the error without reloading and reparsing the input themselves.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F5.1 | Content YAML, root `$schema` payload | Malformed `age` type in CRLF text | Grammar failure reduced to message | Retained typed grammar cause and supplied-text location |
| R1-F5.2 | YAML file factory, root payload | Same text in a bound source | File path retained; cause/span lost | File identity plus typed located cause |
| R1-F5.3 | Content YAML, separate `types` table | Same malformed type under `types.Person.age` | `invalid named types` message only | Typed cause and correct nested source span |
| R1-F5.4 | YAML file factory, separate `types` table | Same named-type document | Path retained; cause/span lost | Typed located cause |
| R1-F5.5 | Content Markdown factory | Same type after another frontmatter field | Only local `11..11` span | Offset into the original CRLF document |
| R1-F5.6 | Markdown file factory | Same text in a bound source | Same local span; file identity absent | Bound file identity and document span |
| R1-F5.7 | Referenced/imported schema reader | Main schema imports the malformed named-type file | Imported path retained but failure still stringified | Original typed grammar cause with referenced-file location |
| R1-F5.8 | Direct type-string parser, clean by contract | Same malformed type expression | Grammar variant retains its substring-relative span | Correct for this lower-level API, which accepts a type string |
| R1-F5.9 | Raw JSON file loader, clean for location | Existing malformed/trailing JSON cases | Error names the source and parser line/column | Preserve that location behavior |

Both root-payload and separate-types error mappings in the standalone parser were checked. Both content and file entry points were reproduced, and the import path was followed to its public failure. Asynchronous resolution delegates directly to these loaders; it introduces no additional diagnostic projection. The existing tests assert error variants or messages but do not prove document offsets or preserved grammar causes.

**Recommended repair:** retain the existing source-aware parse results and the envelope's offsets in the facade, and expose a located error carrying the original typed cause. Preserve the old low-level parser surfaces where callers depend on them; do not require a caller to reread a schema or reconstruct locations. Add public factory tests that assert filename, original grammar variant, and exact LF/CRLF source ranges for both root declarations and named types. This is the source-preservation work already required by the spec, not a new diagnostic framework.

## Verification and acceptance coverage

All new user-facing behavior here is a library operation or passive file adapter. Level 1 in-process and hermetic filesystem tests are the appropriate evidence. The extraction adds no keyboard, mouse, scrolling, or new terminal-style interaction requiring Level 2 or Level 3. Moving existing optional error presentation does not introduce a new terminal behavior claim.

| Requirement | Strongest evidence reviewed | Result |
| --- | --- | --- |
| Standalone parsing, inspection, resolution, lowering | Library example, moved language suites, supplied-source shipped corpus; Level 1 | Implemented; graph gaps below remain |
| One implementation and host-neutral boundary | Ownership, dependency-direction, context, and glob guards; Level 1; normal Cargo graphs | Ownership guards pass; dependency exclusions fail in R1-F2/R1-F3 |
| Strict/soft acceptance, borrow/own result, unchanged inputs | `validation_contract` and facade validation suites; Level 1 | Core examples pass; raw-schema root unions fail R1-F1 |
| Host parity, baseline/trigger ordering and shared preparation | Effective-schema parity, shipped invocation, ownership guards, unsaved DMLS buffer; Level 1 | Targeted run: 10 passed |
| Atomic layering, list precedence, revision/cache identity | Facade layering and private prepared-state tests; Level 1 | Passing in applicable configurations |
| One-argument factories, bound context, one capture/read | `schema_files` and private context-boundary tests; Level 1 | Passing; located diagnostic contract fails R1-F5 |
| Dotted/bracket/quoted queries and full function views | Facade query suite; Level 1 | Passing; existing unsupported tuple traversal is deliberately retained |
| Simplified YAML export and recursive/function metadata | Facade parse/export/parse suite; Level 1 | Passing for supported forms; unsupported exports are typed failures |
| Existing consumers | Host parity tests, implementation's migration evidence, four re-run generator tests | Four generator failures reproduced; R1-F4 |
| Performance and reuse | Recorded same-host before/after measurements and cache-identity assertions; Level 1 plus benchmarks | Recorded changes −2.7% to +0.9%; no material regression identified |
| Default and minimal builds | `just test` from `simplified-schema/` | 1,216 passed with `work-counters,terminal`; 1,080 passed without default features |
| Documentation and skills | New language/reference pages and updated host skill; direct source comparison | Boundary recorded; dependency exceptions must follow the eventual repair/ruling |
| Cross-platform behavior | Implementation's recorded macOS/Linux/native-Windows/WSL2 runs | Reviewed as existing evidence; not rerun and not an independent readiness blocker |

The new library uses `autotests = false`; `tests/l1/main.rs` declares its test modules, its layout guard runs, and its CI feature list enables the work-counter and terminal-gated tests. Its filesystem tests are selected in the default build. No new terminal/browser tier is claimed. The implementation records tier-coverage checks for both areas.

Review commands also included the normal dependency graphs in both configurations and temporary in-process public-API reproducers for findings R1-F1, R1-F4, and R1-F5. Temporary example sources were removed. The review did not run formatting, commit changes, or change implementation behavior. The four failing generator tests are `control_row_types_dash_c_as_a_string`, `a_spelling_may_repeat_at_disjoint_command_paths`, `a_revision_one_document_generates_an_explicit_gap`, and `every_compiled_provider_has_switches_or_a_gap`, all under `cli_switches` in claudine-gen's L1 target.

## Input robustness review

The structural YAML schema envelopes and raw JSON schema loader are file-format readers, so this review considered their load-bearing fields and document shapes. Type-expression grammar, Markdown body parsing, and export writers are outside the configuration matrix. The intended default is a typed error for malformed schema structure with no evaluation, network access, or document mutation. Explicit exceptions include a missing schema producing `MissingSchema`, an empty schema mapping being a valid empty object schema, unresolved external references being retained by content parsing, and optional metadata remaining optional.

Existing characterization tests exercise the standalone envelope, `types`, and raw JSON loading through their public results, including absent/null/wrong-type structure, duplicate keys, trailing content, and second documents. Facade tests add missing/null/numeric/empty `$schema` controls; file tests cover duplicate YAML and duplicate/trailing JSON. These preserve the existing admission behavior rather than impose a new grammar. Error location/cause fidelity remains the gap in R1-F5.

For the `anyOf` validator hit by R1-F1, the full field column was exercised once through the public facade:

| Shape | Observed public result | Required outcome |
| --- | --- | --- |
| Absent | Root required-property failure | Same |
| Explicit null | `Unprepared(BuildValidator)` | Same |
| Wrong whole type (`7`) | `Unprepared(BuildValidator)` | Same |
| One malformed element (`[7, {required: [name]}]`) | Success | Preparation error |
| Every element malformed (`[7]`) | Success | Preparation error |
| Empty (`[]`) | Success | Preparation error |
| Valid false-schema alternatives (`[false, false]`) | Success | Invalid data |
| Valid object arm requiring `name` | Missing-required failure | Same |
| Valid object arms with required `name` only at the root | Success | Missing-required failure |

Document-level JSON duplicate/trailing-content controls use the existing strict loader tests. YAML's shared loader rejects duplicate keys and additional documents. R1-F1 is downstream projection loss after loading, rather than a proposal to build a second loader matrix for every facade.

The smell search covered the facade and standalone readers: no `#[serde(default)]` was added there; the standalone non-string-key filter still rejects the envelope through its key-count check; view fallbacks affect only unavailable typed inspection; capture's `.ok()` cases preserve existing repository-scope behavior. None silently admits the malformed schema shapes above. This does not excuse the separate projection or diagnostic failures.

## Implementation decisions (informational)

The following choices need no decision before repair. The recommendation is to retain each choice unless it explicitly names a finding to repair. After the fix cycle, the author may confirm or reverse these recorded choices; that review never blocks a repair. The matching one-line summaries are in `human_review_items`. The minimal-build dependency conflict is separately pending under Human item 1.

- Keep the passive frontmatter helper behind biscuit-file’s YAML feature; minimal schema builds already enable it.
- Keep resolver-dependent portable-path objects behind full file support; expose only passive identity and strategy types in the syntax feature.
- Feature-gate the Git error variant rather than change its existing payload type.
- Put the inventory beside the actual October 28 specification, correcting the plan’s stale directory.
- Keep terminal error presentation in an optional, nondefault library feature to preserve borrowed error rendering without an unsafe host wrapper.
- Move the pending-token codec and plain expression scanner with their passive validation consumers.
- Expose scanner functions while keeping the Markdown-aware expression finder in Darkmatter.
- Leave the two span helpers used only by Darkmatter in Darkmatter.
- Declare optional repository discovery before its factories land; the final implementation uses it.
- Keep Darkmatter’s public span and literal-token paths and its crate-private pending-token helper; remove the temporary numeric path.
- Keep token round-trip tests at both the plain YAML and actual Markdown frontmatter boundaries.
- Use library-owned optional work counters and add their measurements to Darkmatter’s existing counters.
- Keep the original Claudine parser-regression fixtures as declared cross-package test inputs rather than copy them.
- Test passive syntax checks at their owning boundary during the staged move, then move the checks with their implementation.
- Expose the operator-precedence table as a documentation-hidden item so Darkmatter can check catalog parity.
- Keep Darkmatter’s runtime expression-error module; passive lexer and parser errors move with the parser.
- Keep the moved language modules in a flat layout; the facade supplies the ergonomic entry point.
- Rename Darkmatter’s schema-search-root type to avoid collision with the extracted schema AST type.
- Pass shipped sources explicitly into library resolution and retain host-aware Darkmatter wrappers.
- Use a Darkmatter extension trait for trigger-origin operations instead of free functions.
- Add a typed resolution-unavailable file error and map it to the existing missing-context category.
- Run filesystem-dependent tests with file support and separately test unavailable checks in the minimal build.
- Reuse the existing recursive catalog goldens rather than add duplicate expected output.
- Move shared compiled validators and problem collection while leaving phase and trigger policy in Darkmatter.
- Move validator-cache environment tuning with the cache and cover it with the library’s ambient-state guard.
- Introduce reusable prepared validation before adding the facade’s revision-based cache.
- Classify mixed definite violations and unavailable checks as invalid data while preserving each problem.
- Classify unavailable file and glob checks using build capability and a private context-availability flag.
- Return nonfatal schema-resolution advisories with successful validation.
- Use one facade error type and a borrowed function-signature view with planning-selected names.
- Accept one convertible file-source argument carrying an optional captured context and unsaved text.
- Keep content references unresolved until an explicit file-factory load; omit a separate later-binding method.
- Cache a typed unprepared-validation failure when a definition cannot be lowered or compiled.
- Treat file checks without a content schema’s file context as unavailable rather than invalid data.
- Recognize raw JSON Schema content by its dialect URI while retaining the explicit JSON file factory.
- Add named-object selection so callers can validate one type from a catalog.
- Keep typed layers beside merged JSON Schema, preserve the receiver’s context, and reuse preparation when the merged definition is unchanged.
- Export nested objects and property unions structurally and reject representations that cannot be exported without losing information.
- Resolve a final tuple index in the facade while preserving the existing query helper’s unsupported deeper traversal and union-result behavior.
- Keep the standalone facade free of Darkmatter’s shipped namespace; host resolution wrappers supply it.
- Share repository-scope projection between standalone capture and Darkmatter and preserve discovery failures.
- Keep local asynchronous resolution independent of an async runtime and choose the loader by file extension.
- Keep capture and read counters private to tests and serialize tests that change ambient process state.
- Keep Darkmatter’s shared effective-schema representation and prove parity with facade layering instead of constructing facade objects per page.
- Remove all migration-only language re-exports and make schema consumers depend directly on the library.
- Keep the YAML loader’s existing limit on very long JSON object keys; the explicit JSON file factory remains available.
- Reject the implementer’s acceptance of unused HTTP dependencies in the default graph; repair it through feature narrowing in finding R1-F3.
- Reuse existing source-guard engines through declared test inputs rather than move seven consumers to another support crate.
- Allow exactly one optional file-source context until factory entry captures or reuses it.
- Check single ownership with a table of 45 language symbols, excluding known delegating name collisions.
- Check trigger-language parts in the library and host envelopes in Darkmatter; use representative sidecar contracts for the library corpus.
- Keep doctests in the existing separate recipe and run that recipe explicitly.
- Split language documentation into library topic pages while retaining Darkmatter’s integration page and its host-specific anchors.
- Rewrite moved comments in host-neutral terms while retaining useful explanatory host examples.
- Resolve the deferred model-config document mismatch by converting scalar notes to one-item lists under the existing contract; finding R1-F4 needs no author decision.
- Accept completed intermediate deferrals: check-level tests, shared preparation, revision caching, and the ambient-state guard all landed in later phases.
- Keep unsupported serializer improvements, deeper tuple traversal, later content binding, and a neutral guard-support crate outside this extraction; existing behavior or typed unsupported results define the boundary.
- Keep full research-document enumeration out of the library corpus; repair the seven known-invalid documents at their owning consumer boundary.
- Keep doctest automation consistent with existing archive recipes; the recorded manual run supplies this review’s evidence.
- Accept the recorded benchmark results as showing no material regression; no author disposition is needed for the measured noise.
- Reuse earlier cross-platform evidence for the final documentation-only phase and do not make additional OS evidence a readiness gate.
- Accept use of the canonical scope calculator during implementation without a separate local-plan command; no push is requested in this review.
- Leave the pre-existing links to unwritten schema-index pages outside this code review; no read code produces an incorrect result from those links.
- Keep a schema CLI, remote-schema fetching, expression execution extraction, and code generation outside the agreed release scope.

The intermediate deferrals were resolved in later implementation phases. Remaining omissions are either explicitly outside scope, preservation of existing lower-level behavior, or the actionable findings above. In particular, neither unused HTTP dependencies nor known-invalid shipped research data should be carried forward merely as a request for author review.

## Human rulings

### Item 1

Ruling: Permit `globset` in the minimal build as a passive syntax-validation dependency, while still excluding filesystem discovery and resolution (`gix`, `walkdir`).

Why: Author chose the recommended option. `globset` performs no I/O; it validates `match(...)` glob pattern text at parse time, so keeping it preserves identical schema parsing across builds (R3) with one grammar. Its marginal cost in the minimal graph is only `globset` and `bstr`: its regex dependencies are already present through `jsonschema`/`fancy-regex`.

Changes:
- Spec: amend R2 and acceptance criterion 14 so the minimal build excludes `gix` and `walkdir` resolver dependencies; `globset` is permitted as passive glob-pattern syntax validation. Record the departure in the implementation log as an author ruling.
- Code: none required; `biscuit-file`'s `file-reference-syntax = ["dep:globset", "url"]` and the schema library's use of it already match the ruling.
- Evidence and docs: update the minimal-graph checks (and any source or Cargo-graph guard asserting the exclusion list) to assert no `gix`/`walkdir`, with `globset` expected; update `biscuit-file` and `simplified-schema` feature documentation to state that `globset` is a passive syntax dependency, not a resolver dependency. Do not describe the graph as satisfying the unamended criterion.

