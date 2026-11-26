---
$schema: "feature-review.yaml"
description: "A **feature** review of `2026-11-22-expression-and-mutation-schemas/spec.md`"
feature: "2026-11-22-expression-and-mutation-schemas/review-1.md"
spec: "2026-11-22-expression-and-mutation-schemas/spec.md"
next: "2026-11-22-expression-and-mutation-schemas/review-2.md"
created: "2026-11-24T22:43:22+00:00"
reviewed-by: "codex/gpt-6.2-sol"
ready: false
implemented: true
findings:
    - "R1-F1 (high): Inline YAML comments make valid catalogs fail"
    - "R1-F2 (high): Catalog diagnostics lose declaration locations"
    - "R1-F3 (high): The shared review and fix schema cannot validate documents"
    - "R1-F4 (low): Registration and catalog references still describe the old formats"
human-review: false
human-in-the-loop: null
has-blocked-findings: false
blocked: false
recurrence: false
log: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md"
implemented-by: "codex/gpt-6.2-sol"
---

# Expression and mutation schemas review 1

The feature needs repairs before it is production ready. No finding waits on an author ruling.

## Findings

### R1-F1 (high) Inline YAML comments make valid catalogs fail

**Authority (defect).** Adding a normal YAML comment to a private helper, as in `Label: string # Project display name`, makes `md schema validate` reject the catalog at line 1, column 1. The editor reports the same failure at the start of the document. A host that republishes this valid YAML with refreshed companion and manifest hashes loses its entire registration. The feature's implementation log records this exact failure in a draft of its language guide. [Simplified Schema's declaration source mapper](../../../simplified-schema/lib/src/simplified/source.rs)

The defect class is rejection of valid data while mapping decoded YAML back to its source. A comment-only edit must preserve the result, and a change to the decoded declaration must still change the result. Starting with the shipped project catalog fixture, adding the plain helper `Label: string` passes. Changing it to `Label: strung` fails at the undeclared type. Adding just the comment instead gives the source-projection error across the sites below. [Project fixture](../../lib/tests/fixtures/host_functions/project.yaml)

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F1.1 | Expression catalog declaration mapping in the CLI, editor, and installed host loader | Plain helper, nested mapping leaf, YAML sequence arm, and scalar pipe union, each with an end-of-line comment | CLI and editor reject at document start. Installed host is refused at file level. | Each edited declaration has the same meaning and admission as its comment-free control. |
| R1-F1.2 | Expression catalog callable declaration on the same routes | `function(parameters(void); returns(string); category(Project)) # Project label` as a plain scalar | Same projection error. | The callable declaration parses. A registered host with matching facts can load it. |
| R1-F1.3 | Mutation catalog in the CLI and editor | Built-in mutation fixture with plain `Label: string # A label` appended | Both reject at document start. | The result agrees with the same helper without the comment. Registration independently refuses a mutation member at `kind`, as required. |
| R1-F1.4 | Ordinary tagged schema in the shared parser, outside the callable kinds | Comment after a plain type helper used by the schema export | CLI and editor reject at document start. The same document without the comment passes. | A normal schema with the commented helper is accepted and keeps the same export. |

The quoted equivalent, comments on separate lines, comments after block-scalar headers or flow-mapping values, and category-list comments pass. LF, CRLF, and BOM with either line ending also pass the unchanged project fixture. The whole-document loader reports repeated keys, second documents, and trailing garbage before the declaration mapper runs. A manifest and its facts companion do not use this mapper.

Repair the shared source mapping for plain YAML scalar values. Stop the authored span at a real YAML comment while preserving a quoted hash and any hash that is part of the value. Keep the rest of the decoded value and its offsets available to the grammar diagnostics. An in-process regression can cover the carrier forms once, with one catalog wired through CLI and editor tests.

### R1-F2 (high) Catalog diagnostics lose declaration locations

**Authority (spec).** The catalog validation section says, “Diagnostics point to the declaration or field that failed and include the other location when two declarations conflict.” The acceptance checks also require the editor and `md schema validate` to report the same catalog errors, including invalid overloads, duplicate keys, and case-insensitive collisions. [Catalog format and validation](spec.md#catalog-format-and-validation), [Acceptance checks](spec.md#acceptance-checks)

The defect class is source locations lost during catalog checking and diagnostic reporting. Each refusal must point to the failing declaration. A conflict must identify both claims. Reformatting the same overloads as a sequence or a pipe union must preserve both locations. Renaming a parameter without changing its type keeps the conflict, while changing one parameter type from `string` to `number` makes the fixture valid through all three entry points. Manifest probes republished the project fixture with valid digests. The table uses one-based positions for readability.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F2.1 | Simplified Schema's early overload check, projected by the CLI and editor for both kinds and by installed expression registration | Two identical binding contracts in a YAML sequence, then the same pair in a folded pipe union | Sequence points to the second arm without the first. Pipe union points to the first arm without the second. | The later arm is primary and the first arm is related in both spellings. |
| R1-F2.2 | Simplified Schema's named-type overload check and the same consumers | Change the second arm to use `Label@this` with `Label: string` | CLI points to the start of the entry. Editor points to the document start. Installed registration has a filename only. None gives the first arm's location. | Both arm locations survive the check that resolves the helper. |
| R1-F2.3 | Category validation and its catalog consumers | Repeat a category label with a different description, use an undeclared label in a signature, or leave either side of a category declaration blank. | CLI points to the repeat and names the earlier list index in text. Editor points to the document start and installed registration has a filename only. Undeclared category, a blank label and a missing description after `->` also lose their positions on those paths. | Point to the offending category entry or callable declaration. For the repeated label include the earlier category's physical location. |
| R1-F2.4 | Darkmatter's registered-host report as rendered by `md compose` | Case-folded canonical pair or two valid overloads that disagree on their category | Both catalog checks find the earlier declaration. The CLI warning drops it when it converts the registration diagnostic to a run notice. | Preserve the earlier mark in the CLI notice, as the standalone validation and editor paths do. |
| R1-F2.5 | Installed facts name checks and their registration report | Give `project_label` the alias `is_project_file`, or an alias already claimed by a built-in | The bad alias is located. The earlier callable or built-in declaration is never attached by the facts check. | Retain the earlier declaration under the same rule used for names claimed across catalogs. |
| R1-F2.6 | Built-in name and alias references from standalone CLI and editor | Rename `project_label` to `KEBABCASE` and validate from a directory without a rusty-biscuit checkout | CLI JSON keeps the built-in's repository-relative path and byte span but has no line or column. The editor drops the related location. | Show the built-in declaration's source label and coordinates from its embedded source. Add a navigable link when matching source is present. |

The standalone case-folded name and conflicting-overload-category controls give both positions in CLI and editor. Duplicate YAML keys give the repeated position and the original coordinates in the CLI message, and a related location in the editor. Installed YAML admission likewise carries both positions in the message. The hosted narrowing checks carry both declarations when two overloads disagree on the fact they prove. Across two catalogs, public registration tests locate canonical/canonical, alias/alias and both mixed claim orders, and the editor publishes the related location. The shared CLI notice projection in R1-F2.4 also handles those cross-catalog reports. Distinct aliases and the original fixture pass. Changing an alias element to null is refused at that element before collision checks. Installed members of mutation kind are refused at kind admission, so they do not enter overload or facts checking. [Cross-catalog registration tests](../../lib/tests/l1/host_registration_companions.rs), [Editor registration tests](../../dmls/tests/l1/host_catalog_reload.rs)

Keep typed locations with the check that first knows them and reuse that information in the existing routes. The existing catalog/registration related-location records can carry most of the data. For a built-in, compute coordinates from the embedded declaration used for the conflict. Avoid requiring a local checkout or extending the facts format. Add regression assertions for both positions, including the resolved-helper overload path. The current editor overload test checks only the second arm. [Standalone catalog checks](../../../simplified-schema/lib/src/simplified/standalone.rs), [Darkmatter catalog error locations](../../lib/src/markdown/compose/expression/catalog/checks.rs), [Registration warning conversion](../../lib/src/markdown/compose/report_diagnostics.rs), [Editor overload test](../../dmls/tests/l1/yaml_document_catalog_editor.rs#L120)

### R1-F3 (high) The shared review and fix schema cannot validate documents

**Authority (defect).** The committed `review-fix-cycle.yaml` rejects the repository's own review-cycle document shape before it checks any frontmatter. This was added separately from the callable-catalog implementation. It fails three tests in the shipped schema corpus and reproduces with `md schema validate` on the schema itself and on a Markdown document naming it. [Review and fix schema](../../../schemas/review-fix-cycle.yaml), [Darkmatter shipped-schema tests](../../lib/tests/l1/shipped_schema_corpus.rs)

The defect class is a shipped schema using declaration forms the loader assigns a different meaning. The schema must load for either of its intended arms, with or without pipeline metadata, while omission of a required review field must still fail validation. The probes exercised the declaration reader followed by Markdown schema resolution. Every repair was tested in a temporary copy.

| Instance | Site | Shape tested | Observed | Expected |
| --- | --- | --- | --- | --- |
| R1-F3.1 | Nested `rfc.blast-radius` declaration | Free text in `$description` | Parsed as a property type and fails at line 22, column 31. | Documentation has no effect on the object's declared fields. |
| R1-F3.2 | Root arm with pipeline metadata | Quoted `{ ...rfc, ...pipeline }` in the root sequence | Once the nested declaration is corrected, both validation routes try to resolve this as a filename. | A mapping arm composes the review fields and pipeline fields. |
| R1-F3.3 | Root arm without pipeline metadata | Quoted `{ ...rfc }` in the root sequence | Same filename interpretation for scalar root arms. This arm follows the first failure. | A mapping arm composes the review fields alone. |
| R1-F3.4 | Shipped schema classification | Zero-byte `specification.yaml` reserved in the same commit | Direct validation selects no schema. The corpus classifies the file as loadable by default and its parse and resolution assertions require an export. | Classify the deliberate placeholder with the corpus's existing placeholder rule. |

Moving the prose into a YAML comment and spelling both root arms with `$extends` made the schema validate and accepted Markdown samples with and without pipeline metadata. Removing the required `reviewed` field from those samples still fails at the relevant arm. Retain the declared field requirements and use those two mapping forms. Keep the intentional zero-byte placeholder and classify it explicitly in the corpus so that the tests will detect when it gains content and needs reclassification. [Corpus classification](../../lib/tests/l1/shipped_schema_corpus.rs#L406)

### R1-F4 (low) Registration and catalog references still describe the old formats

**Authority (spec).** Built-in migration asks for the corresponding skills and guidance to reflect the two new kinds, and host registration requires format 3. Two Claudine maintainer skill pages still say the fixed-path publisher writes format 2. The fresh-publication test's name also says format 2 while the assertions exercise the format 3 publisher. The Darkmatter schema corpus describes the built-in catalogs as importable `kind: schema` libraries while its assertions now check that both are private. Correct the stale descriptions and test name to match the behavior they cover. [Claudine architecture skill](../../../.claude/skills/claudine/architecture.md#L36), [Claudine module map](../../../.claude/skills/claudine/module-map.md#L13), [Fresh publication test](../../../claudine/lib/tests/l1/host_publication.rs#L144), [Corpus catalog classification](../../lib/tests/l1/shipped_schema_corpus.rs#L86)

## Implementation decisions (informational)

Both readers audited the spec, plan, and implementation log. The scope expressly leaves schema-level `when` and host-defined lifecycle actions for later work. The plan keeps Claudine's shipped inventory free of callable companions and uses a companion-bearing fixture for verification. All repairs above fit the authorized behavior.

- Keep the shared YAML error variant and provide a separate duplicate-key position lookup. Use it for related locations in the editor and retain the original coordinates in the CLI message. Keep structural findings on existing validation routes and use the new code for callable-specific failures.
- Check authored metadata types before a soft validator can convert them. Keep the allowed key lists as constants guarded by parity tests. Use the same built-in and authored reader with two explicit modes, selecting built-in treatment by source path identity.
- Remove function catalogs from the importable shipped sources while keeping them as preparation inputs. Copy the private `recent-commits` helper into its other owner and compare the definitions in a test.
- Look up reserved names from the static Rust bindings to avoid recursively loading the registry. Carry the underlying catalog error and the shared related-location records. Use a whole-word search inside a declaration for local reference locations, with the declaration key as fallback.
- Keep the established CLI JSON `error: schema` envelope while adding location fields and allowing both existing strings and objects in its related list. Resolve related source from checkout files and open buffers where available. The missing embedded-source reporting and the deferred overload locations are covered by R1-F2.
- Allow invalid catalogs to lose top-level completion and hover until corrected, with type-grammar help still available. Test repeated editor queries by counting worker jobs and preparation work. Warm one-time envelope preparation before measuring each registration load.
- Retain the typed format 2 decoder for publisher cleanup only, and record refused integer versions explicitly. Derive companion candidates from verified YAML. Favor the parse failure when a member cannot be classified and allow same-stem YAML files when only one derives a companion. Attribute a companion refusal to the catalog entry in `files` and to the companion's content when it has a usable position.
- Use typed facts locations, deterministic path order for cross-catalog conflicts, and the existing lock and manifest-last protocol. Coalesce editor reloads and reuse installed revisions instead of keeping a process-wide function cache. Inherit the selected host's declarations in nested composition workers.
- Land the minimal Claudine format update before its full inventory work to keep intermediate builds usable. Use one encoded inventory for publication, listing, and test injection, and keep fresh generation independent of cached runtime payload preparation. Reserve the manifest path before accepting embedded sources.
- Replace the guide's conflicting sample name with `project_slug` and move its inline comment into prose. The example passes, and R1-F1 covers the parser work it deferred. The temporary Phase 1 CLI routing and early Claudine encoder work were superseded within the implementation. Generated documentation and caller skill publication were also completed.
- Claudine's pre-edit baseline and focused Windows attempt had incomplete evidence, as recorded in the implementation log. Later dependent tests ran, and exact-tree cross-OS evidence remains with CI. The log records an unrelated kache host failure and two research-summary caveats. Neither changes the spec contract or needs an author decision.

## Input robustness and verification

The invalid-shape outcome is a located refusal before evaluation or publication. Omitted catalog descriptions and empty description strings are valid. Required catalog collections need contents. An empty manifest `files` list is valid with the lifecycle member still present. Explicit null on the two nullable facts rules means no rule. An empty alias list, narrowing facts list, or source exclusion list describes an explicit empty collection.

- **Catalog YAML.** The shared envelope suite covers absent, null, wrong-type and empty metadata, one or all invalid category entries, and the supported `types` carriers. CLI controls confirmed the category shapes and plain document markers, with duplicate keys, garbage and a second YAML document rejected. Inline comments fail after loading as shown in R1-F1. Duplicate category labels, undeclared categories, and duplicate overload positions are covered by R1-F2. Bare null and quoted `"null"` are exercised separately in the public envelope suite.
- **Manifest YAML.** Public snapshot tests cover the shared YAML loader and the required-field, type, digest and file-list checks. Empty files are accepted. Null or non-array files, non-string entries, empty paths, missing or extra digest entries, non-string entries and hash mismatches are refused. Tests distinguish missing digest, missing bytes, and stale inner catalog hash. The legacy cleanup decoder has its own malformed manifest and valid format 2 controls. Read-only classification and installed preparation share the digest and companion verifier.
- **Facts JSON.** The public host function tests distinguish absent fields, explicit null, wrong whole-field types, invalid elements and allowed empty lists. Registration tests assert refusal positions for bad entries, non-ASCII text, stale hashes, duplicate JSON keys, trailing content, and JSON5 spellings in strict JSON. The facts-name collision check occurs only after these field checks. Its missing earlier locations are in R1-F2.
- **Built-in execution and generated data, Level 1.** Descriptor and binding parity, lifecycle action forms, event gating, expression calls in action operands, and data-reading permissions have in-process coverage. Built-in catalogs differ from the frozen baseline only on their `kind` lines. The generated lifecycle YAML and function pages are byte-identical to that baseline. `just shipped-schemas --check` passed in Simplified Schema, Darkmatter, and Claudine.
- **Catalog validation and privacy, Level 1.** Public validators, `md schema validate`, and language server diagnostics exercise both catalog kinds, passive examples, and refusals as schema or import targets. Direct probes used the shipped CLI and language server over standard input/output. R1-F1 and R1-F2 cover gaps on these normal entry points.
- **Host scope and mutation exclusion, Level 1.** Compose tests distinguish a valid host call that reaches the runtime boundary from a binding refusal and a call on an unselected document. The in-process host handler tests cover actual evaluation and return validation. Editor tests cover the selected profile's facts and recovery when a companion is restored. Snapshot and manifest tests confirm that host mutation catalogs cannot add actions.
- **Publication and upgrade, Level 1.** Publisher, inventory-listing, and doctor tests cover the existing install location, unsupported-version remedy, published bytes, missing or modified companions, and repeat repair. These checks operate on file contents, diagnostics and protocol messages. They require no graphical terminal or host input.

The focused Darkmatter runs passed 115 and 72 tests. The broader Darkmatter run passed 309 and failed three, all from R1-F3. The focused Claudine run passed 156. Simplified Schema passed 133 focused default-feature tests and all 2,193 minimal-feature tests. No selected test reached five seconds. The tier audit found no stranded tests in the three package areas.
