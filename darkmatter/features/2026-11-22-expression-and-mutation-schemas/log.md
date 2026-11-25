---
implementation_1: "2026-11-24T22:52:40+00:00"
implementation_2: "2026-11-25T00:38:46+00:00"
implementation_3: "2026-11-25T01:35:06+00:00"
implementation_4: "2026-11-25T02:33:46+00:00"
implementation_5: "2026-11-25T03:42:27+00:00"
implementation_6: "2026-11-25T04:20:51+00:00"
implementation_7: "2026-11-25T04:50:10+00:00"
implementation_8: "2026-11-25T05:49:49+00:00"
implementation_9: "2026-11-25T06:59:30+00:00"
implementation_10: "2026-11-25T16:14:39+00:00"
implementation_11: "2026-11-25T17:37:04+00:00"
implementation_12: "2026-11-25T19:57:40+00:00"
---
## Implementation of Review Findings #1

> **started at:** 2026-11-24T22:52:40+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-1.md'
- this is iteration 1 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Inline YAML comments preserve declarations | fixed | Shared YAML projection and standalone decoding repaired. All required children verified. |
| R1-F1.1 | Expression helpers in plain, mapping, sequence, and pipe forms pass CLI, editor, and installed admission | fixed | Shared public parser compares all four carriers to controls. CLI, editor, and a digest-verified host accept the commented catalog. |
| R1-F1.2 | Commented callable passes CLI, editor, and installed admission with matching facts | fixed | Public source map returns the exact callable span. CLI, editor, and installed facts admit the callable. The installed names contain exactly the callable. |
| R1-F1.3 | Commented mutation helper passes CLI and editor, and host admission still refuses at kind | fixed | CLI and editor accept the commented mutation catalog. Registration refuses it at the kind declaration. |
| R1-F1.4 | Ordinary tagged schema preserves its export in the shared parser, CLI, and editor | fixed | Shared parser preserves the export, accepts text, and rejects a number. CLI and editor both validate the document. |
| R1-F1.5 | Quoted hashes, separate comments, block and flow comments, category comments, LF/CRLF/BOM, invalid syntax and duplicate keys | clean | The carrier matrix preserves quoted and joined hashes. The loader suite rejects duplicate keys, trailing garbage, and second documents. Line-ending and comment changes found by the sweep are recorded below. |
| R1-F1.6 | Inline comments in flow collections, explicit keys, anchors, document markers, and sequence headers preserve meaning and source spans | fixed | The 14-carrier public parser matrix covers nested holders, dash-only items, anchors, explicit keys, and comments inside flow collections. Document marker and header cases are also checked. |
| R1-F1.7 | Standalone and whole-document admission agree on BOM with CRLF and preserve source coordinates | fixed | Four LF/CRLF × BOM/no BOM combinations assert exact callable and helper spans. The CLI accepts the BOM+CRLF catalog. |
| R1-F1.8 | Darkmatter's standalone YAML loader matrix accepts a single BOM-prefixed document and still rejects a second YAML document | fixed | The old rejection reproduced in isolation. The corrected 47-test matrix verifies accepted BOM, accepted CRLF, and refused second documents. |
| R1-F2 | Catalog diagnostics preserve primary and related locations | fixed | All required routes carry primary and related positions. Focused gates passed 44 Simplified Schema, 151 Darkmatter, and 74 Claudine tests. |
| R1-F2.1 | Direct duplicate overload sequence and pipe arms in CLI, editor, and registration | fixed | Public parser and CLI/editor/installed regressions assert both arms in sequence and folded pipe forms. Both standalone catalog kinds share the checks. |
| R1-F2.2 | Duplicate overload after named helper resolution in the same consumers | fixed | The same routes assert both authored arms when `Label@this` resolves to the first parameter type. Number is the distinct overload control. |
| R1-F2.3 | Repeated, undeclared, and malformed categories retain precise sites and related locations in the same consumers | fixed | Repeated, undeclared, blank-label, and blank-description cases point to the offending declaration. Repetition carries exactly one earlier position. |
| R1-F2.4 | Compose notices preserve registered name and category conflicts, including cross-catalog claims | fixed | Compose tests assert both positions for canonical-name and overload-category conflicts in JSON and plain output. Cross-catalog notices retain both files. |
| R1-F2.5 | Facts aliases conflicting with another callable or built-in retain the earlier location through registration | fixed | Public installed-bundle checks assert the facts alias and the earlier catalog key or embedded built-in declaration. |
| R1-F2.6 | Standalone CLI and editor report embedded built-in coordinates with navigation when source is present | fixed | The standalone CLI and editor cases assert built-in coordinates outside a checkout. The checkout control verifies the source link. |
| R1-F2.7 | Standalone name/category conflicts, duplicate YAML keys, hosted narrowing, cross-catalog claim orders, invalid alias, and mutation-kind refusal controls | clean | Control tests cover standalone name/category errors, both duplicate-key locations, narrowing, all cross-catalog claim orders, malformed facts entries, and host mutation-kind refusal. |
| R1-F2.8 | Grouped pipe overloads and grouped leaves within YAML sequences preserve the distinct arm ranges | fixed | The shared mapper retains each flattened pipe arm and the scalar token in a grouped YAML-sequence item. The four-form matrix asserts both marks for direct and resolved contracts. |
| R1-F2.9 | Within-catalog canonical and alias claim orders use the correct catalog or facts file for each location | fixed | Installed test checks canonical/alias, alias/canonical, and alias/alias orders within one catalog, plus the built-in case. Each mark is checked in its own file. |
| R1-F2.10 | The editor uses a matching checkout file for navigation and retains embedded coordinates when a file is missing or differs | fixed | Editor tests assert exact source links for matching Git and no-Git workspace copies and embedded coordinates for an absent or different copy. |
| R1-F2.11 | Editor dependencies and probes refresh the related location when a matching source appears, changes, or disappears | fixed | A single open catalog gains a link when the source buffer matches and loses it on replacement and deletion. |
| R1-F2.12 | Verified-bundle inspection retains both catalog conflict positions in its public refusal | fixed | The verified refusal preserves the related records when it converts the installed refusal. The digest-consistent doctor fixture exercises that public boundary. |
| R1-F2.13 | Claudine doctor exposes the primary and related positions when a verified installation has conflicting declarations | fixed | The isolated doctor test and 74-test registration slice assert that the report contains the later category at 4:7 and the earlier one at 3:7. |
| R1-F2.14 | Editor publishes one catalog conflict diagnostic when the named-type view finds the same failure | fixed | Editor tests assert exactly one error for each category and overload conflict and a single distinct loader error for duplicate YAML. |
| R1-F2.15 | The Darkmatter kind-registry API returns the located catalog error for an undeclared category | fixed | The old expectation failed alone. The replacement asserts the primary file, exact callable span, category message, and empty related list. All 95 routing tests passed. |
| R1-F3 | Shared review-cycle schema validates both document arms and corpus classifies the reserved placeholder | fixed | Both root arms load and validate Markdown. The 27-test corpus slice and public validation regression passed. |
| R1-F3.1 | Nested blast-radius prose does not declare a property | fixed | The comment carries the prose. Validation accepts a packages list and rejects a mapping at `/blast-radius/packages`. |
| R1-F3.2 | Mapping root arm with pipeline metadata resolves and enforces required review fields | fixed | The resolved first arm requires `name`. Markdown with pipeline metadata validates and fails when `reviewed` is absent. |
| R1-F3.3 | Mapping root arm without pipeline metadata resolves and enforces required review fields | fixed | The resolved second arm has no pipeline field. Markdown without pipeline metadata validates and fails when `reviewed` is absent. |
| R1-F3.4 | Empty specification schema is explicitly classified and detectable if populated | fixed | The explicit placeholder exception requires emptiness and no export. Test-input markers name both root schema files. |
| R1-F3.5 | Other root schema artifacts use supported declarations and keep their corpus dispositions | clean | Corpus parse, resolution, and registry checks passed for the other root schemas. The similar Claudine action draft is already classified as unloaded. |
| R1-F4 | Maintainer references and test names match registration and catalog formats | fixed | Updated all current-format descriptions discovered in the sweep. The 60 focused tests and encoder inspection verified the repairs. |
| R1-F4.1 | Claudine architecture skill describes format 3 | fixed | Architecture skill now says format 3. Verified against the current publisher encoder. |
| R1-F4.2 | Claudine module map describes format 3 | fixed | Module map now says format 3. Verified against the current publisher encoder. |
| R1-F4.3 | Fresh-publication test name reflects format 3 | fixed | The renamed publication test asserts integer format 3 from emitted YAML and a warning-free registration. |
| R1-F4.4 | Corpus classification describes private callable kinds | fixed | The callable-catalog corpus class describes both kinds as private. Corpus tests assert schema-target and named-type import refusals. |
| R1-F4.5 | Publication test module and previous-manifest test comments describe current admission and cleanup | fixed | The publication test header now names format 3. The malformed previous-manifest matrix describes cleanup admission and passed. |
| R1-F4.6 | Maintainer, caller, and canonical documentation distinguish current publishing from legacy format cleanup | clean | Canonical publishing documentation and caller skills identify format 3. Remaining format 2 references describe cleanup, upgrade, or unsupported-version behavior. |
| R1-F4.7 | Installed inventory test documentation names the manifest format exercised by its fixture | fixed | The installed inventory fixture header says format 3 and the inventory tests passed. |
| R1-F4.8 | Setup test describes the publisher's current format in its expectation | fixed | The setup expectation names format 3. The isolated setup test compared the installed manifest with the current inventory and passed. |

-    starting the work on 'R1-F1 Inline YAML comments make valid catalogs fail' at 22:53:12
        - Mapping decoded YAML scalars back to authored source can reject valid declarations when a comment is mistaken for part of the value.
        - The sibling sweep found the shared single-line decoder, the multiline scalar decoder, block values and sequence arms, explicit keys, document markers, and flow entries. Standalone catalogs and ordinary schemas share strict projection, while frontmatter lookup also uses the scalar decoders. The YAML document loader and facts loader validate their own formats before projection.
        - The strict mapper now excludes YAML comments from plain values and follows comments at block holders, anchors, explicit keys, document markers, and flow entry boundaries. The multiline decoder already stopped at YAML comments and is covered through the same public parser.
        - A regression exposed standalone decoding of BOM plus CRLF as a second document. Standalone decoding now skips the BOM as the whole-YAML loader already does, while projected byte spans remain relative to the original text.
        - Commented expression and mutation catalogs and a tagged schema passed CLI and editor checks. Installed expression registration exposed exactly the expected function, and a real helper edit still failed at the helper type. Installed mutation admission still refused the member at its kind.
        - The language guide and Darkmatter maintainer skill still limited comments inside flow collections to frontmatter lookup. Both now describe the shared comment rule.
        - Simplified Schema's source, scalar, and envelope slice passed 71 tests. Darkmatter's comment, schema ownership, and effective layering slice passed 12 tests. Neither run had a test at or above five seconds. `git diff --check` passed.
- work completed for 'R1-F1 Inline YAML comments make valid catalogs fail' at 22:59:08

-    starting the work on 'R1-F2 Catalog diagnostics lose declaration locations' at 22:59:32
        - Checks identify the declarations that caused a failure, and early parsing, source projection, or reporting can discard that location evidence.
        - Swept grammar sequence and pipe unions, grouped alternatives, resolved named types, shared YAML kind checks, catalog compilation, facts name admission, host admission, CLI validation and run notices, and both editor projections. The existing duplicate-key loader and host mutation admission keep their own diagnostic rules.
        - Grouped pipe unions exposed the same loss after flattening. The grammar now retains exact arm spans, and the source map projects them when a group contributes multiple options. The shared overload test covers both catalog kinds, four carriers, and direct and resolved contracts.
        - Switched facts name checking to return both typed claims from the same admission decision. Registration uses the verified YAML or JSON for positions and the embedded catalog for a built-in claim. No companion format change was needed.
        - Used the existing diagnostic schemas to publish related positions in structured run output. The editor falls back to the embedded file label without a matching source and watches candidate paths so open diagnostics gain or lose a link when the source changes.
        - The downstream sweep found the same lost secondary location in verified snapshot reporting. It now survives both the registration diagnostic conversion and the doctor check on a modified, digest-consistent installation. Doctor lists both physical positions without quoting authored values.
        - The editor initially published both its catalog judgment and its standalone envelope error for the same conflict. The YAML overlay now owns catalog and load failures, and the editor tests assert one finding with exact ranges. Duplicate YAML keeps its loader code and both key positions.
        - The focused Linux runs passed 151 Darkmatter tests, 44 Simplified Schema tests, and 74 Claudine registration and doctor tests. No selected test reached five seconds. `just publish-skills` published the current `use-darkmatter` and `use-dmls` guidance, and `git diff --check` passed.
- work completed for 'R1-F2 Catalog diagnostics lose declaration locations' at 23:22:55

-    starting the work on 'R1-F3 The shared review and fix schema cannot validate documents' at 23:23:38
        - Shipped schema declarations use YAML carrier forms that the schema reader interprets as a property type or a file reference.
        - The sweep found both quoted root spreads and the nested prose entry in the active review-cycle contract. The other root contracts use supported carriers. The only other prose-shaped declaration in the shipped corpus is in Claudine’s documented action draft, where separate corpus and recursive-fixture entries already identify the draft as unloaded.
        - Replaced both quoted root arms with mapping-form extensions and moved the blast-radius prose into a YAML comment. The resolved pipeline arm still requires its name, while the review-only arm omits the field.
        - Classified the reserved zero-byte specification schema under the existing placeholder rule. Both schema paths are named in the corpus input index markers.
        - The public Markdown validator accepted review documents with and without pipeline metadata, reported the missing reviewed property in both variants, and rejected a mapping in place of the blast-radius packages list.
        - The focused Darkmatter corpus run passed all 27 tests, with no test at or above five seconds. The corpus covered the other root schema artifacts and confirmed the documented Claudine action draft remains separately classified.
- work completed for 'R1-F3 The shared review and fix schema cannot validate documents' at 23:25:49

-    starting the work on 'R1-F4 Registration and catalog references still describe the old formats' at 23:26:11
        - Maintainer references and test descriptions drifted after the published envelope format and callable document kinds changed.
        - The format and kind sweep found two more live format 2 references in tests for the installed inventory and setup publication. Their fixtures both install format 3. The legacy cleanup decoder, unsupported-version tests, and upgrade guidance correctly describe format 2.
        - Updated both Claudine maintainer pages and the three test surfaces to identify the current format. The renamed fresh-publication test asserts the emitted integer format version and verifies that Darkmatter loads the bundle without diagnostics.
        - Updated the corpus classification to describe standalone expression and mutation catalogs and their private declarations. The corpus exercises both document-schema and named-type import refusal.
        - Focused Linux checks passed in Claudine library (32 publication and inventory tests), Claudine CLI (1 setup migration test), and Darkmatter (27 shipped-schema corpus tests). No test reached five seconds. The renamed test ran as L1 through the declared library test target.
- work completed for 'R1-F4 Registration and catalog references still describe the old formats' at 23:29:32

-    starting the work on 'R1-F1.8 BOM characterization control' at 23:33:50
        - Full Linux L1 and an isolated rerun both reproduced a stale characterization of one leading BOM as a second document. The separate second-document, invalid-content, and JSON BOM controls have distinct contracts.
        - The corrected 47-test Darkmatter YAML loader matrix passed in 0.147 s. The BOM case expects a tagged schema, and the second-document control still expects a located YAML refusal.
- work completed for 'R1-F1.8 BOM characterization control' at 23:34:56

-    starting the work on 'R1-F2.15 kind-registry diagnostic expectation' at 23:36:08
        - Full Darkmatter L1 and an isolated rerun reproduced an obsolete expectation that an undeclared category comes back as a kind-validation problem. The public API now returns the shared callable-catalog error. The shape control still exercises the kind-definition report.
        - The kind-registry test now asserts the typed error and the exact signature span. The 95-test routing slice passed, including both YAML loader and editor controls. Darkmatter lint also passed.
- work completed for 'R1-F2.15 kind-registry diagnostic expectation' at 23:37:07

-    Linux full-gate fixture drift reproduced at 23:38:21
        - Six shell completion assertions used a valid spaced enum as their broken schema. An isolated Bash test reproduced the extra key. The unchanged fixture predates ed934faf9, which allows spaces in enum values. Both sibling fixtures need malformed type syntax to exercise the same passive failure contract.
        - Both fixture copies now use an unclosed enum. All 85 Linux path-completion tests passed, and all six affected Windows assertions passed in the full CLI rerun.

### Completion Report

- **complete** after 1 h 34 min 8 s.
- Parent findings total 4 = 4 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed.
- Instance rows total 36 = 32 fixed + 4 clean. All four parent findings and their required consumers are verified. There are no remaining finding or instance IDs.
- R1-F1 fixes standalone YAML comment projection and BOM admission. R1-F2 retains both catalog declaration locations through validation, host admission, compose, editor refresh, and doctor. R1-F3 repairs both review-cycle root arms and guards the empty specification placeholder. R1-F4 fixes the format and role references.
- Full Linux verification passed after two stale API expectations and two shared shell-completion fixtures were corrected. The completion fixtures now use malformed type syntax because the merged enum grammar accepts values containing spaces.
- The initial Windows CLI run reproduced all six completion failures and recorded four tests at 5.007–5.856 s. The isolated Bash case passed at 0.446 s after the fixture correction. The four timing cases measured 2.503, 1.804, 1.997, and 1.752 s with one test thread. The full four-thread rerun passed all 2,687 tests below five seconds.

| Linux package area | Final commands and results | Findings |
| --- | --- | --- |
| simplified-schema | `just test` passed 2,620 and 2,196 tests. `just lint`, `just build`, and `just shipped-schemas --check` passed. | R1-F1, R1-F2 |
| darkmatter | `just test` passed 18,016 tests. `just lint`, `just build`, and `just shipped-schemas --check` passed. | R1-F1 through R1-F4 |
| claudine | `just test` passed 13,072 tests. `just lint`, `just build`, and `just shipped-schemas --check` passed. | R1-F2, R1-F4 |

- The `just check-tier-coverage simplified-schema darkmatter claudine` audit found zero stranded tests. Final full Linux and Windows runs had zero tests at or above five seconds.
- The freshly built `target/debug/md schema validate schemas/review-fix-cycle.yaml` passed for R1-F3. `just publish-skills` in Darkmatter published `use-darkmatter` and `use-dmls` for R1-F2. `git diff --check` passed.

| Native Windows package | Final command | Tests passed | Findings |
| --- | --- | ---: | --- |
| darkmatter-lsp | `just cross-check darkmatter-lsp --os windows` | 2,174 | R1-F1, R1-F2 |
| darkmatter-cli | `just cross-check darkmatter-cli --os windows --test-threads=4` | 2,687 | R1-F1, R1-F2 |
| darkmatter | `just cross-check darkmatter --os windows --test-threads=4` | 13,054 | R1-F1 through R1-F4 |
| simplified-schema | `just cross-check simplified-schema --os windows --test-threads=4` | 2,619 | R1-F1, R1-F2 |
| claudine | `just cross-check claudine --os windows --test-threads=4` | 7,250 | R1-F2, R1-F4 |
| claudine-cli | `just cross-check claudine-cli --os windows --test-threads=4` | 4,281 | R1-F4 |

- The Windows archive runs used working-tree snapshots. The secondary expected-test listing rejected the four-thread runtime flag. Nextest executed each full selected package suite and reported the counts above. These runs produced no reusable CI receipts.
- Raw runner logs are in `/tmp/darkmatter-review-1.itsXMUXi`.
-    Changed files (84)
        - `.claude/skills/claudine/architecture.md`
        - `.claude/skills/claudine/module-map.md`
        - `.claude/skills/darkmatter/schema.md`
        - `claudine/README.md`
        - `claudine/cli/tests/l1/setup_command.rs`
        - `claudine/docs/cli/doctor.md`
        - `claudine/lib/src/composition/registration/inspect.rs`
        - `claudine/lib/src/composition/registration/published.rs`
        - `claudine/lib/src/doctor/registration.rs`
        - `claudine/lib/src/doctor/registration_tests.rs`
        - `claudine/lib/tests/l1/host_publication.rs`
        - `claudine/lib/tests/l1/registration_inventory.rs`
        - `darkmatter/README.md`
        - `darkmatter/cli/schemas/output/clean.yaml`
        - `darkmatter/cli/schemas/output/code-block.yaml`
        - `darkmatter/cli/schemas/output/compose.yaml`
        - `darkmatter/cli/schemas/output/delta.yaml`
        - `darkmatter/cli/schemas/output/edit.yaml`
        - `darkmatter/cli/schemas/output/get.yaml`
        - `darkmatter/cli/schemas/output/graph.yaml`
        - `darkmatter/cli/schemas/output/hash.yaml`
        - `darkmatter/cli/schemas/output/init.yaml`
        - `darkmatter/cli/schemas/output/list-themes.yaml`
        - `darkmatter/cli/schemas/output/render.yaml`
        - `darkmatter/cli/schemas/output/rm.yaml`
        - `darkmatter/cli/schemas/output/schema-about.yaml`
        - `darkmatter/cli/schemas/output/schema-detect.yaml`
        - `darkmatter/cli/schemas/output/schema-triggers.yaml`
        - `darkmatter/cli/schemas/output/schema-validate.yaml`
        - `darkmatter/cli/schemas/output/set.yaml`
        - `darkmatter/cli/schemas/output/toc.yaml`
        - `darkmatter/cli/schemas/output/validate-refs.yaml`
        - `darkmatter/cli/src/commands/schema/validate.rs`
        - `darkmatter/cli/src/output/mod.rs`
        - `darkmatter/cli/tests/l1/compose_host_scope.rs`
        - `darkmatter/cli/tests/l1/path_completion.rs`
        - `darkmatter/cli/tests/l1/schema_validate_yaml.rs`
        - `darkmatter/dmls/docs/diagnostics.md`
        - `darkmatter/dmls/src/diagnostics/frontmatter.rs`
        - `darkmatter/dmls/src/diagnostics/yaml_document.rs`
        - `darkmatter/dmls/src/overlay/mod.rs`
        - `darkmatter/dmls/src/overlay/yaml_contract.rs`
        - `darkmatter/dmls/tests/l1/expression_call_diagnostics.rs`
        - `darkmatter/dmls/tests/l1/yaml_document_catalog_editor.rs`
        - `darkmatter/dmls/tests/l1/yaml_document_editor.rs`
        - `darkmatter/docs/cli/data-contracts.md`
        - `darkmatter/docs/topics/host-registration.md`
        - `darkmatter/docs/topics/schemas/definition.md`
        - `darkmatter/docs/use-skills/use-darkmatter/cli.md`
        - `darkmatter/docs/use-skills/use-dmls/schemas.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-1.md`
        - `darkmatter/lib/src/config/registration.rs`
        - `darkmatter/lib/src/config/registration/bundle.rs`
        - `darkmatter/lib/src/markdown/compose/collection_blocks/analysis.rs`
        - `darkmatter/lib/src/markdown/compose/context/report.rs`
        - `darkmatter/lib/src/markdown/compose/expression/catalog/checks.rs`
        - `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs`
        - `darkmatter/lib/src/markdown/compose/expression/diagnostics.rs`
        - `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs`
        - `darkmatter/lib/src/markdown/compose/expression/host/artifact.rs`
        - `darkmatter/lib/src/markdown/compose/expression/host/mod.rs`
        - `darkmatter/lib/src/markdown/compose/expression/mod.rs`
        - `darkmatter/lib/src/markdown/compose/report_diagnostics.rs`
        - `darkmatter/lib/src/markdown/schemas/yaml_documents/mod.rs`
        - `darkmatter/lib/tests/l1/diagnostic_projection.rs`
        - `darkmatter/lib/tests/l1/host_registration_companions.rs`
        - `darkmatter/lib/tests/l1/report_diagnostics.rs`
        - `darkmatter/lib/tests/l1/schema_extraction_characterization.rs`
        - `darkmatter/lib/tests/l1/shared_diagnostics.rs`
        - `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`
        - `darkmatter/lib/tests/l1/yaml_kind_registry.rs`
        - `schemas/review-fix-cycle.yaml`
        - `simplified-schema/docs/topics/language/schema-documents.md`
        - `simplified-schema/docs/topics/language/types.md`
        - `simplified-schema/lib/src/errors.rs`
        - `simplified-schema/lib/src/simplified/grammar.rs`
        - `simplified-schema/lib/src/simplified/mod.rs`
        - `simplified-schema/lib/src/simplified/signature.rs`
        - `simplified-schema/lib/src/simplified/source.rs`
        - `simplified-schema/lib/src/simplified/standalone.rs`
        - `simplified-schema/lib/src/simplified/yaml_scalar.rs`
        - `simplified-schema/lib/src/yaml_documents/semantic.rs`
        - `simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs`


## Implementation of Review Findings #2

> **started at:** 2026-11-25T00:38:46+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-2.md'
- this is iteration 2 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R2-F1 | Host registrations accept names that Markdown cannot call | fixed | One exact call-name rule serves the catalog reader, Rust registration, and facts admission. All required consumers verified in the 127-test Darkmatter slice. |
| R2-F1.1 | Shared reader, CLI and DMLS refuse uncallable keys at their declaration | fixed | `authored_and_runtime_callable_names_must_parse_as_the_exact_call_name` asserts the complete quoted key span, no related location, and refusal for space, leading digit, Boolean, empty, padded, doubled punctuation, and deep grouping shapes. |
| R2-F1.2 | Rust host builder refuses uncallable canonical names and aliases before generating facts | fixed | The same public regression binds each rejected spelling as a canonical name and as an alias. It requires the typed refusal before the configuration callback can run. |
| R2-F1.3 | Installed and verified admission refuse bad names and aliases at source; compose dispatch and DMLS completion reflect admitted names | fixed | `installed_and_verified_callable_names_are_refused_at_the_authored_key_or_alias` recomputes the digests and requires one refusal at each catalog key or second alias, with no published profile. |
| R2-F1.4 | Supported spellings, case folding, selected lookup and collision rules remain valid | clean | `accepted_callable_names_dispatch_through_the_parser_and_runtime` asserts the exact result of dashed, underscored, dotted, and case-varied calls. Existing built-in and cross-catalog collision, selected lookup, and separate-host tests passed. |
| R2-F1.5 | Mutation admission and private type helper semantics | clean | The installed mutation fixture still refuses at its kind, the helper fixture exposes exactly its callable, and the editor checks expression, mutation, and ordinary tagged schema controls. |
| R2-F1.6 | Facts alias field shape matrix, catalog types shape matrix, and quoted valid keys | clean | The installed and public facts matrices locate absent, null, numeric whole-field, mixed and all-bad element cases, and accept empty and valid aliases. The 25 Simplified Schema envelope tests verify the catalog field shapes, and the CLI/editor approve quoted valid keys. |
| R2-F1.7 | CLI validates catalog names at their keys | fixed | `callable_names_are_checked_by_schema_validate_at_the_authored_key` batches three invalid keys and a quoted valid key, asserting key spans and no sentinel effects. |
| R2-F1.8 | DMLS validates catalog names at their keys | fixed | `uncallable_names_receive_one_editor_diagnostic_at_the_authored_key` asserts the exact YAML key range, one diagnostic, and no diagnostic for the valid quoted dotted name. |
| R2-F1.9 | Verified inventory refuses the same locations as installed admission | fixed | The installation regression passes each digest-verified snapshot through `load_verified`, the path used by Claudine inventory inspection, and asserts the same member, file, line, column, and underlying reason. |
| R2-F1.10 | Compose uses only admitted names in the selected host | fixed | `compose_dispatch_does_not_see_the_callable_names_of_a_refused_installation` requires the registration and unknown-function records and no host-runtime boundary. The existing accepted-control test still reaches the runtime boundary. |
| R2-F1.11 | DMLS completion exposes only admitted names | fixed | `a_refused_callable_name_is_never_offered_by_completion` finds the refusal on the facts companion and sees neither a canonical nor a usable-alias completion for that installation. The existing selected-host alias and case test still completes. |
| R2-F2 | Type-reference errors can point to unrelated text | fixed | The shared reader follows authored type paths through every reference branch. Twelve children verified through shared, CLI, DMLS, installed, and prior projection regressions. |
| R2-F2.1 | Locate failing parameter signature despite matching comments and parameter names | fixed | The block-header and parameter-name decoys select the complete function atom, beginning at `function(`. Replacing the missing type with `Label@this` admits the catalog. |
| R2-F2.2 | Locate return type and the failing arm in block and flow sequences and folded and literal pipes | fixed | The return case and block sequence, flow sequence, folded pipe, and literal pipe regressions assert the signature range and accept the declared helper control. |
| R2-F2.3 | Locate external and callable-as-type references and preserve the related key | fixed | External and callable-as-type references with the same words in the first arm and second header report the second signature in CLI, DMLS, and installed admission. Only the callable-as-type row has the related key. |
| R2-F2.4 | Locate failing helper mapping, inline object, extension and tuple declarations | fixed | Shared-reader tests assert the import-name bytes for mapping, inline object, and scalar extension, and the tuple atom when its element is unindexed. Each edited helper control loads. |
| R2-F2.5 | Preserve syntax, overload, category, slot and collision diagnostics and their projections | clean | The 312-test recurrence slice preserves independent syntax, overload, category, slot, and folded-name marks and text/JSON compose projections. The Claudine doctor witness reports both verified catalog declarations. |
| R2-F2.6 | CLI projects the shared reference location for both catalog kinds | fixed | A batched CLI test admits six controls and checks six failures for both kinds, including exact range, one-based line and column, and related-key cardinality. |
| R2-F2.7 | DMLS publishes the shared reference location and related callable key | fixed | One DMLS diagnostic per case in both kinds, with the entire failing arm and a related key only for a callable used as a type. Passing controls have no diagnostics. |
| R2-F2.8 | Installed host warnings use the signature or helper location | fixed | An installed digest-consistent control exposes exactly `acme_label`. Three invalid references refuse the profile at the second signature, with the callable key at 5:5 only when applicable. |
| R2-F2.9 | Pattern-valued entries and inline spreads use the authored value or operand despite a same-name key or description | fixed | An exact byte-range check selects the pattern value and spread operand over an earlier key or description. Both local object controls pass. |
| R2-F2.10 | Sequence extensions use their failing operand and grouped array and tuple spreads use the owning type expression | fixed | The sequence extension selects its second operand. The tuple spread and grouped array union select the entire owning atom. Each declared helper control passes. |
| R2-F2.11 | Ordinary registered schema resolution uses structural declaration paths | clean | The `host_registration_locations` sibling-description tests locate exact expected declarations across applied payloads, unused entries, and imported library cases. Callable registration uses the shared catalog reader directly. |
| R2-F2.12 | An external type reference marks its owning editor signature and resolves no import | fixed | The isolated DMLS test checks the owning return signature and zero import checks, named-type parses, and dependencies. Passed at 0.032 s. |
| R2-F3 | No-terminal approval exceeds Windows L1 budget | fixed | Default-concurrency Windows CLI run passed the repaired test in 3.027 s. |
| R2-F3.1 | Profile setup, launch and composition and keep one no-terminal CLI assertion | fixed | Windows phase profile identified preflight as the largest phase. With baseline schema validation disabled, the same CLI assertions passed in 0.062 s on Linux compared with 0.087 s before and in 3.027 s on Windows. |
| R2-F3.2 | Other approval tests cover prompts and persistence through the in-process helper | clean | The eight sibling tests passed in 0.149–0.478 s on Windows and at most 0.090 s after the patch on Linux. The siblings already avoid CLI process launch and assert their distinct approval outcomes. |
| R2-F4 | Claudine trigger discovery exceeds Windows L1 budget | fixed | The repaired test passed in 2.519 s at default Windows package concurrency. |
| R2-F4.1 | Profile bundle setup, validation and triggers and preserve installed CLI selection | fixed | The Windows profile measured 31 ms setup, 2,154 ms validation, and 1,289 ms trigger tracing. A single validation run now proves both valid verdicts, one applied installed inline-composition layer, and zero applied installed composition layers. It passed in 0.786 s on Linux and 2.519 s on Windows. |
| R2-F4.2 | Bundle test header identifies the manifest format it installs | fixed | The module header now says format 3, matching the shared fixture manifest writer and Claudine's publisher. The test loaded the fixture installation. |
| R2-F4.3 | Sequence-document trace and validation witness | clean | This sibling asserts both trigger matches through `md schema triggers` and the separate verdict and diagnostic assertions through validation. It passed in 4.707 s in the original Windows run, 3.428 s in the profile, and 1.292 s in the focused Linux run. |
| R2-F4.4 | Seven other bundle verdict and diagnostic witnesses | clean | Each uses one validation invocation and asserts independently expected validity, problems, or positions. The original Windows run took 2.171–4.097 s per test. All seven passed again in the focused slice. |
| R2-F5 | Recursive lifecycle corpus exceeds Windows L1 budget | fixed | The entire repaired CLI corpus passed in 1.613 s at default Windows package concurrency. |
| R2-F5.1 | Profile bundle setup and validation and preserve every verdict and location | fixed | The Windows profile measured 32 ms setup, 1,693 ms validation, and 1,256 ms trigger tracing. The single CLI batch still checks all 19 verdicts, the existing pointers and positions, and exactly one installed composition layer and zero sequence-document layers for depth six. It passed in 0.704 s on Linux and 1.613 s on Windows. |
| R2-F6 | Depth-four lifecycle exceeds Windows L1 budget | fixed | The repaired test passed in 2.107 s at default Windows package concurrency. |
| R2-F6.1 | Profile trigger and validation phases and retain all depth and location controls | fixed | The Windows profile measured 68 ms setup, 1,806 ms validation, and 1,540 ms trace. The single CLI batch proves both installed trigger paths apply once, all four controls are valid, and the three edited actions fail at their existing pointers and key coordinates 16:11, 24:11, and 22:19. It passed in 0.883 s on Linux and 2.107 s on Windows. |
| V2.1 | Keep both review-cycle root shapes loadable | fixed | Reproduced the bad prose and string arms. Direct CLI validation and the 25-test corpus pass with the mapping forms. |
| V2.2 | Admit the now-populated specification schema in the corpus | fixed | Reproduced the stale placeholder assertion, then verified the authored schema through the normal standalone and root-schema resolution checks. |
| V2.3 | Style-setter parity stays within the Windows Level 1 budget | fixed | The original test took 6.096 s in the package and 1.959 s alone on Windows. Both bounded halves passed in the 43-test Linux parity slice and in 2.730 s and 2.692 s at default Windows package concurrency. |

No author rulings or blocked findings are recorded in the review.

- starting the work on 'R2-F1 Host registrations accept names that Markdown cannot call' at 00:39:13
        - The defect class is admitting names that the expression parser cannot dispatch as a call.
        - The review reproduces a catalog and host accepting `"2fa_label"` while the expression parser rejects `2fa_label('x')`. The supported control is `project_label('x')`, which parses and reaches the selected host.
        - The authored reader checks collision and signatures, the runtime builder checks collisions, and the facts reader checks JSON string shape. None asks whether the parser produces a call whose name exactly matches the declaration. Use that parser decision at all three gates and preserve the catalog key or alias index.
        - Checked the kind validation/editor routes, runtime and analysis-only constructors, verified snapshot publication, selected-host scope, compose dispatch, and editor completion. Downstream consumers use only admitted profiles and the existing case-folded index. The control for the new gate is a call to a quoted canonical key with dashed or dotted spelling, or a case-varied valid alias.
        - The expression parser treats the bare tokens `true` and `false` as literals, while a distinct spelling such as `True` lexes as a name. Admission checks the authored spelling exactly and leaves the established ASCII-folded dispatch index intact. Mutation verbs and ordinary type helpers stay under their own contracts.
        - The shared rule requires exactly the expression lexer's `Variable` token with matching spelling. The parser uses that token when followed by `(`. This avoids recursively parsing arbitrary declaration text. A 500-level parenthesized key exercises the rejection below YAML's simple-key length limit.
        - A staged bundle with correct digests now refuses bad keys in the catalog and bad strings at the JSON array element. The verified inventory returns the same underlying refusal as installed admission, which adds the surrounding register-entry context.
        - The Darkmatter L1 slice passed 127 tests with none at or above five seconds. It includes the earlier alias, case-folding, collision, mutation-kind refusal, YAML catalog, and registered-host controls. Simplified Schema passed 25 callable-envelope tests, and its recipe also passed 2,196 minimal-feature tests. `git diff --check` passed.
- work completed for 'R2-F1 Host registrations accept names that Markdown cannot call' at 00:46:45

- starting the work on 'R2-F2 Type-reference errors can point to unrelated text' at 00:47:34
        - The defect class is locating a rejected parsed reference by a spelling match that can select unrelated authored text.
        - The minimal failing edit is adding `# Missing` to the header above `function(parameters(name: Missing@this); returns(string); category(Project))`. A passing control changes the reference to the declared helper `Label@this` and keeps the header. A second-arm reference must select that arm even when the first arm mentions the same word.
        - The sibling sweep found one text search shared by external references, undeclared local helpers, and callable-as-type errors. Traversal reaches parameters, returns, union arms, object members and patterns, composition operands, and tuple elements and spreads. Registration, CLI validation, and DMLS all project the resulting range. Overload, category, slot, and name diagnostics already use structural locations. The installed ordinary-schema resolver uses structural reference paths through its own Site adapter.
        - The Linux baseline CLI probe admitted the edited control and reported the missing type at line 12, column 25 on the header comment. The source map already indexes exact names and operand spans where available, plus signature/tuple atoms. Use those atoms for nested types the map does not index. This avoids a new grammar or source-rewriting layer.
        - The parser rejects named narrowing targets in a separate typed check. It already reports the authored target rather than searching for its spelling.
        - The regression fixtures cover pattern entries, inline spreads, scalar and sequence extensions, tuple spreads, and grouped array unions found by the sibling sweep. The generic registered schema tests independently cover a description mentioning the same path and both unused and applied imports.
        - The first recurrence run found one stale DMLS assertion expecting only the file substring inside an unambiguous return type. The repaired check reports the signature that owns it. The existing work-counter and dependency assertions still prove no imported file is read.
        - Darkmatter's focused nextest slice passed eight tests. After the isolated DMLS witness passed, the recurrence slice passed 312 tests, including both new catalog reader tests, the CLI and editor probes, installed admission, and the earlier diagnostic projections. No case reached five seconds.
        - Claudine's focused verified catalog conflict doctor test passed in 1.110 s and preserved both declarations. The shared registration proof was exercised through its existing entry point.
        - The host-function guide, DMLS diagnostics reference, and DMLS caller skill now describe where a bad type reference appears.
- work completed for 'R2-F2 Type-reference errors can point to unrelated text' at 00:55:00

- Started one Windows phase profile for R2-F3–R2-F6 at default package concurrency. The defect class is a Level 1 test that crosses five seconds when subprocess and preparation work contend with the suite.
        - The failing examples are the original Windows overruns recorded in the review. Isolated passing runs of the same four tests and below-budget neighboring approval and bundle fixtures are the controls.
        - The three lifecycle cases repeat bundle preparation across validation and trigger subprocesses. The approval case has one compose subprocess and a small fixture. Temporary timers distinguish fixture setup, process launch, preflight, and the two schema commands. The profiling-only edits will be restored before repair work.
        - The profile ran all 2,690 Windows CLI tests at default package concurrency. The four timing cases passed in 2.434, 3.535, 3.026, and 3.481 s respectively. These complement the original overruns in the review. The profiler's output flag is a test-run option, so the separate expected-test listing rejected it. The standard final cross-check will verify coverage.
        - Approval setup took 7.5 ms, CLI command construction and spawning took 882 ms, and the child took 1,486 ms after spawn. Inside compose, 65 ms preceded preflight and 1,365 ms was spent in preflight. The measured refusal precedes the compose pipeline.
        - Discovery spent 31 ms in setup, 2,154 ms in validation and 1,289 ms in trigger tracing. Recursive lifecycle spent 32, 1,693, and 1,256 ms respectively. Depth four spent 68, 1,806, and 1,540 ms respectively. Both schema tasks read the same fixture installation.
        - Temporary timings were removed after the Windows run. The raw log is `/tmp/darkmatter-review-2.cWAOoQOm/windows-profile.log`.

- starting the work on 'R2-F3 The no-terminal approval test exceeds the Windows L1 budget' at 00:59:35
        - The defect class is a Level 1 test crossing the runtime budget under package contention because it performs work unrelated to its assertion.
        - The minimum witness is the existing single non-terminal compose subprocess with an unapproved interpolated shell command. It took 5.007 s in the original Windows package run. The same command refused approval, rendered no prompt, and saved no configuration in the isolated 2.503 s run. The profiled default-concurrency rerun took 2.434 s.
        - The companion approval cases use the in-process approval helper and exercise prompts, approval rules, and persistence. They finished in 0.149–0.478 s in the profiled run. The CLI witness must continue to cover the non-terminal route. The repeat-subprocess bundle siblings are R2-F4–R2-F6.
        - Windows profiling measured 7.5 ms fixture setup, 882 ms command construction and launch, and 1,486 ms child completion. Compose spent 65 ms before preflight and 1,365 ms in preflight, where the approval refusal occurs. The fixture has no dependency on baseline validation.
        - Disabled baseline schema validation in the existing subprocess. This removes unrelated preflight work while retaining the file input, shell template, no-prompt assertion, refusal assertion, and unchanged configuration assertion. Using stdin would change the input path for the witness.
        - The Linux `just test approval_prompt::` control and repair each passed nine selected tests with zero slow tests. The CLI witness measured 0.087 s before and 0.062 s after. `git diff --check` passed. The Windows package timing for this patch remains to be verified.
- work completed for 'R2-F3 The no-terminal approval test exceeds the Windows L1 budget' at 01:00:15

- starting the work on 'R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget' at 01:00:49
        - The defect class is a Level 1 test repeating installed-bundle preparation in a second process to observe a fact the first process already reports.
        - The original default-concurrency Windows run took 5.657 s. The isolated and four-thread controls took 1.804 s and 4.188 s. The new default-concurrency profile took 3.535 s, with 31 ms setup, 2,154 ms validation, and 1,289 ms trigger tracing.
        - The valid prompt document must apply the installed inline-composition trigger and exclude the installed composition trigger. The cancelled-container fixture must also keep its successful verdict.
        - The sibling sweep found duplicate validation and trace commands in the recursive and depth-four cases, tracked by R2-F5 and R2-F6. The sequence-document case took 4.707 s in the original package run and 3.428 s in the profile. It is the dedicated trigger CLI witness that checks the trace and both matched entries. The other seven module tests took 2.171–4.097 s in the original run and each makes one validation invocation.
        - The stale module header says format 2. Both the fixture manifest writer and Claudine's production publisher emit format 3.
        - Validation already reports the applied payload sources in `layers.triggers`. The repair checks the full installed paths through that output and keeps the existing CLI trace witness in the sequence-document test. A separate in-process trace query would repeat fixture preparation to retrieve the same test facts.
        - The focused `just test claudine_schema_bundle::` passed 11 tests with zero slow tests. The repaired test took 0.786 s and retained both verdicts and the applied and excluded trigger assertions. `git diff --check` passed. The normal-concurrency Windows verification remains required.
- work completed for 'R2-F4 The Claudine trigger discovery test exceeds the Windows L1 budget' at 01:02:03

- starting the work on 'R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget' at 01:02:59
        - The defect class is a Level 1 test repeating installed-bundle loading and trigger evaluation in another CLI invocation when its original validation result already contains the applied layers.
        - The original package run took 5.324 s on Windows. The isolated control took 1.752 s and preserved all verdicts. The default-concurrency phase profile recorded 32 ms setup, 1,693 ms validation, and 1,256 ms trigger tracing, with a 3.026 s total in that run.
        - The trigger check must show the exact installed composition layer applied to the valid depth-six document and the installed sequence-document layer absent. The wrong-verb depth-six document must still fail at its innermost key. The rest of the corpus keeps its independent verdicts, pointers, positions, and deferred-operand checks.
        - The sibling sweep found the repaired trigger-discovery witness in R2-F4, the matching depth-four test in R2-F6, and the separate sequence-document trace witness in R2-F4.3. The single-validation siblings are covered by R2-F4.4. The recursive test can reuse the same applied-layer assertion as R2-F4 and retain the full CLI batch.
        - The CLI validation batch now proves both trigger claims through the same applied-layer assertion used by R2-F4. It retains all 19 corpus documents and every verdict, diagnostic pointer, source position, and deferred-operand assertion. The separate sequence-document CLI trace witness continues to exercise trigger output.
        - The focused Linux `just test claudine_schema_bundle::` run passed all 11 tests with no tests at or above five seconds. Recursive lifecycle measured 0.704 s, down from 1.217 s in the preceding Linux module run. `git diff --check` passed. The Windows package timing for this patch remains to be verified.
- work completed for 'R2-F5 The recursive lifecycle corpus test exceeds the Windows L1 budget' at 01:03:27

- starting the work on 'R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget' at 01:03:52
        - The defect class is repeated bundle preparation in separate CLI processes when one validation result already proves the required trigger application.
        - The minimum failing witness is the ordinary Windows package run at 5.856 s. The same test passed in isolation at 1.997 s. A fresh normal-concurrency profile measured 68 ms setup, 1,806 ms validation, and 1,540 ms trace for a total of 3.481 s.
        - The depth-four test can use the same installed trigger path assertion as R2-F4 and R2-F5. The one-invocation siblings are covered by R2-F4.4. The distinct sequence-document trace witness in R2-F4.3 still checks the trace command itself.
        - The positive control must still apply both installed trigger layers and validate all four depth-four documents. The three edited controls must fail at their own innermost action keys and keep their distinct pointer and message.
        - The depth-four validation result now proves both installed trigger layers. The same batch keeps four passing documents and three failures at the innermost pointers, with exact line and column checks at the misspelled verb, cancelled retry, and initialize shell keys. The sequence-document test still runs the trace command.
        - The focused Linux `just test claudine_schema_bundle::` passed all 11 tests with zero slow tests. The depth-four case fell from 1.340 s in the preceding module run to 0.883 s. `git diff --check` passed. The final Windows package timing remains to be verified.
- work completed for 'R2-F6 The depth-four lifecycle test exceeds the Windows L1 budget' at 01:04:55

- The combined verification sweep found two workspace regressions in the R1-F3 corpus. The review-cycle schema again used a nested prose property and string arms. A separate edit had populated the specification schema that the test still classified as empty.
        - `just test shipped_schema_artifacts_parse_passively` reproduced the invalid nested declaration. With the prose moved to a comment, CLI validation reproduced the string-arm refusal because strings in that position are file references.
        - Restored the two mapping-form `$extends` arms and moved the blast-radius prose to a comment. Both root schemas now pass direct CLI validation.
        - The same corpus test then reproduced the stale placeholder classification. Removed that one exception so the normal shipped corpus checks the populated schema as a loadable export. The authored specification schema content was preserved.

- The first final Windows CLI run passed all assertions and verified R2-F3–R2-F6 below five seconds. It recorded one independent timing failure, `render_alias_parity::every_style_setter_family_decides_the_same_in_both_spellings`, at 6.096 s.
        - This is the same Level 1 contention class. The test inventories hundreds of style paths, then serially prepares each sampled field under both CLI spellings. The earlier instrumented Windows package measured 4.494 s. An isolated Linux reproduction measured 1.119 s and passed every assertion. The adjacent argument-parity matrix remained below budget.
        - Preserve every representative and its two independent style results by partitioning the ordered sample into two complementary bounded groups. A growing sample still enters one group, and the size assertion forces another split before either group grows too large.

- The final Windows CLI run at default package concurrency verified both style-parity halves at 2.730 s and 2.692 s. The original single test took 1.959 s alone on the rig and 6.096 s in the first final package run. The full Linux suite, lint, and tier audit were rerun after splitting it.

### Completion Report

- **complete** after 46 min 1 s.
- Parent findings total 6 = 6 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed.
- Instance rows total 31 = 23 fixed + 8 clean. Three additional verification regressions were fixed. No finding, instance, or verification ID remains open.
- R2-F1 uses the same exact lexer token rule for authored expression catalog names, Rust host binding names, and facts aliases. The CLI and DMLS keep located refusals, and invalid registrations contribute no dispatch or completion entries.
- R2-F2 follows parsed type paths to existing source ranges. Unindexed parameters, returns, and tuple elements report their owning signature or type declaration. Callable-as-type failures retain the related key.
- R2-F3 keeps the real no-terminal approval invocation and skips unrelated baseline schema work. R2-F4 through R2-F6 verify the installed trigger layers in each existing CLI validation batch. The sequence-document test still witnesses the trace command directly. All corpus verdicts and location assertions were retained, with three exact depth-four positions added.
- The repeated review-cycle schema errors and obsolete specification placeholder classification were repaired. The separate specification schema edit was included in the corpus verification.

| Verification command | Final result | Coverage |
| --- | --- | --- |
| Darkmatter `just test` on Linux | 18,029 passed, zero at or above 5 s | R2-F1–R2-F6 and V2.1–V2.3 |
| Darkmatter `just lint` on Linux | Passed, including the Zed WASM check | Rust and test edits |
| `just check-tier-coverage darkmatter` | Zero stranded tests | New and renamed L1 cases |
| Darkmatter `just shipped-schemas --check` | Passed | Built-in callable artifacts |
| Darkmatter `just publish-skills` | Published `use-darkmatter` and `use-dmls` | Caller documentation |
| `target/debug/md schema validate schemas/review-fix-cycle.yaml schemas/specification.yaml --output json` | Both valid | V2.1–V2.2 |
| `just cross-check darkmatter-cli --os windows` | 2,691 passed, zero at or above 5 s | R2-F1–R2-F6 and V2.3 |
| `just cross-check darkmatter-lsp --os windows` | 2,177 passed, zero at or above 5 s | R2-F1–R2-F2 editor consumers |
| `just cross-check darkmatter --os windows` | 13,060 passed, zero at or above 5 s | R2-F1–R2-F2 library and registration, V2.1–V2.2 |
| Claudine `just test a_verified_catalog_conflict_reports_both_declarations_in_doctor` | 1 passed in 1.110 s | R2-F2 recurrence and verified inventory projection |
| Simplified Schema `just test callable_catalog_envelopes::` | 25 focused and 2,196 minimal-feature tests passed | R2-F1 catalog input controls |
| `git diff --check` | Passed | Combined changes |

| Finding | Original Windows package | Final Windows package |
| --- | ---: | ---: |
| R2-F3 | 5.007 s | 2.878 s |
| R2-F4 | 5.657 s | 2.412 s |
| R2-F5 | 5.324 s | 2.030 s |
| R2-F6 | 5.856 s | 2.467 s |

- Native Windows gates completed with expected-test manifests at default package concurrency. Their archived runs exercised working-tree snapshots. Raw verification and profiling logs are under `/tmp/darkmatter-review-2.cWAOoQOm`. The first Linux test invocation passed 18,028 tests before its surrounding shell encountered a reserved variable name. The final Linux invocation passed on its own.
- Changed files (25)
        - `.claude/skills/darkmatter/expressions.md`
        - `darkmatter/README.md`
        - `darkmatter/cli/tests/l1/approval_prompt.rs`
        - `darkmatter/cli/tests/l1/claudine_schema_bundle.rs`
        - `darkmatter/cli/tests/l1/compose_host_scope.rs`
        - `darkmatter/cli/tests/l1/render_alias_parity.rs`
        - `darkmatter/cli/tests/l1/schema_validate_yaml.rs`
        - `darkmatter/dmls/docs/diagnostics.md`
        - `darkmatter/dmls/tests/l1/host_catalog_reload.rs`
        - `darkmatter/dmls/tests/l1/yaml_document_catalog_editor.rs`
        - `darkmatter/docs/topics/host-functions.md`
        - `darkmatter/docs/topics/host-registration.md`
        - `darkmatter/docs/use-skills/use-dmls/schemas.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-2.md`
        - `darkmatter/lib/src/markdown/compose/expression/catalog/checks.rs`
        - `darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs`
        - `darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs`
        - `darkmatter/lib/src/markdown/compose/expression/host/artifact.rs`
        - `darkmatter/lib/src/markdown/compose/expression/host/mod.rs`
        - `darkmatter/lib/src/markdown/compose/expression/mod.rs`
        - `darkmatter/lib/tests/l1/host_functions.rs`
        - `darkmatter/lib/tests/l1/host_registration_companions.rs`
        - `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`
        - `schemas/review-fix-cycle.yaml`

## Implementation of Review Findings #3

> **started at:** 2026-11-25T01:35:06+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-3.md'
- this is iteration 3 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R3-F1 | YAML anchors and aliases break overload validation in sequences | fixed | All 19 child rows are verified. The shared locator admits equivalent YAML spellings and retains the authored primary and related locations through every catalog consumer. Final gates passed. |
| R3-F1.1 | Block sequence anchors, tags and aliases preserve the copied signature contract | fixed | Public parser compares quoted, block, tagged, commented, next-line and aliased items to a written control in both kinds and checks the authored arm range. |
| R3-F1.2 | Flow sequence anchors, tags and aliases preserve valid arms and both conflict locations | fixed | Public parser compares flow verdicts and ranges. Independent CLI returned the duplicate at 4:104 and its earlier mark at 4:31. |
| R3-F1.3 | Direct duplicate overloads preserve the authored primary and related arm through standalone and installed admission | fixed | The parser checks unused anchors, earlier aliases and later aliases with exact, distinct slices. CLI, DMLS and installed admission retain the two sites. Independent CLI returned the anchored block duplicate at 7:12 and its earlier mark at 6:19. |
| R3-F1.4 | Resolved overloads and category conflicts retain both locations and missing helpers point to their declaration | fixed | CLI, DMLS and digest-verified installation assert both marks for resolved and category conflicts and exactly one primary alias mark for a missing helper. |
| R3-F1.5 | Private helper sequences and ordinary schema sequences preserve meaning and the ordinary export | fixed | Public schema validation accepts the expected string, number and nested object values and refuses wrong types. CLI admits the helper alias and the tagged schema export. |
| R3-F1.6 | Verified publisher and Claudine doctor accept valid aliases and report both conflict positions | fixed | Five final doctor and embedded-inventory cases admit written and aliased controls and preserve both marks for written, anchored and aliased duplicates. |
| R3-F1.7 | Whole-value aliases, unrelated aliases, narrowing and case-folded name conflict controls retain verdicts and locations | clean | Scalar, pipe and sequence aliases preserve their complete contracts. Both narrowing orders preserve the declaring clause and the clause-free arm. An unrelated alias leaves the duplicate marks intact, and the case-folded conflict keeps both keys. |
| R3-F1.8 | CLI and DMLS agree on admission and located errors for expression and mutation catalogs | fixed | The focused suite exercises both kinds with written and aliased controls and checks the diagnostic code, location and related count for conflicts and missing helpers. |
| R3-F1.9 | Installed and compose admission retain callable facts for valid aliases and refuse conflicted profiles with both positions | fixed | Verified installation admits one callable with complete facts. Compose binds the aliased overload and reaches the host-runtime boundary. A refused bundle contributes no callable and reports both positions. |
| R3-F1.10 | Earlier comment, conflict, and reference projection cases keep their expected verdicts and ranges | clean | The public and consumer slices passed the earlier comment carriers, grouped sequence and pipe conflicts, exact reference ranges and valid helper controls. |
| R3-F1.11 | Shared envelope and loader robustness controls retain located refusals and valid format-specific syntax | clean | Existing envelope and loader tests retained the absent, null, empty and wrong-type outcomes, duplicate and trailing document refusals, and valid comment and line-ending cases. |
| R3-F1.12 | Flow boundaries inside an anchored or tagged quoted scalar remain scalar content | fixed | Quoted signatures containing parameter commas admit as a single arm and retain their exact authored range after anchors and tags. |
| R3-F1.13 | Ordinary schema root unions and nested mappings preserve alias semantics and validate their exports | fixed | Public parser accepts the alias arm in the root union and aliased inline object arms. Data validation accepts the intended root arm and rejects incorrect nested values. |
| R3-F1.14 | Composition operands preserve the authored alias range and inherited constraints | fixed | Public source map records the alias operand and no local import-name text. CLI accepts the valid nested control and enforces the inherited minimum at `/item/text`. |
| R3-F1.15 | Aliased constraint mappings retain the constraints and the authored operand range | fixed | Public schema projects `*rules` at its use, accepts the closed root arms and refuses a document with an extra property. |
| R3-F1.16 | Standalone reference projection accepts a standard string tag and still refuses unsupported key presentations | fixed | The first full Darkmatter run found the superseded tag-refusal assertion. The isolated `just test source_aware_presentation_boundary_is_closed` passed in 0.016 s with the exact reference span and the decorated-key refusal. |
| R3-F1.17 | The strict schema spelling matrix exercises supported tags and retains unsupported-shape refusals | fixed | The isolated boundary test passed. The 82-test parser and editor slice verifies tagged mapping, indentless arm, and continued scalar parity and exact type ranges, with the prior key and directive refusals retained. |
| R3-F1.18 | Public definition failures retain the offending token beside a tagged sibling and use the authored alias range | fixed | The focused slice verifies the exact `nope` token and definition range, the later `*s` use for repeated suggestions, and failure preservation with absent ranges on a YAML version directive. |
| R3-F1.19 | The editor shows the grammar token beside a tagged sibling and still falls back when projection fails | fixed | The focused slice checks the exact `bogus` token beside a continued tagged value and the same diagnostic code with a whole-value range for the directive fixture. |
| R3-V1 | The shipped-schema corpus retains its required suggestion review schema | fixed | Restored the concurrently deleted file from the index. The isolated corpus test passed in 1.111 s and the same test passed in the final suite in 1.163 s. |

-    starting the work on 'R3-F1 YAML anchors and aliases break overload validation in sequences' at 01:35:43
        - The defect class is loss of authored item identity while projecting decoded YAML sequence values into schema declarations and diagnostics.
        - Block and flow sequence items take scalar paths that bypass node properties handled for mapping values. A written number arm and its aliased form should both admit alongside a string arm. A string arm and its aliased duplicate should instead refuse with two distinct source locations.
        - The pre-repair CLI admitted the written control, refused the equivalent alias at document start, and reported the later arm of an anchored duplicate at 7:12 without its earlier location.
        - The sibling sweep traced block sequences, compact mapping items, flow sequences and flow mappings through the shared YAML locator and both source projections. The raw flow boundary scanner also has to keep quote handling active after an anchor or tag. The grammar's direct duplicate check uses decoded token spans. Resolved duplicate and category checks consume the source map, and reference lookup should fall back to its owning alias arm when the reference text is elsewhere.
        - A scalar that decodes to text beginning with an asterisk is not necessarily a YAML alias. The shared locator must recognize an alias from its authored token before admitting its decoded value or using that token for a diagnostic. The parsed YAML remains authoritative for values, categories, signatures, and binding facts.
        - Installed host loading and verified publisher inspection call the shared catalog reader. CLI and DMLS layer kind admission over the same judgment. Compose uses the installed result. The public loader and companion field validators keep their existing matrix and are covered by their earlier controls.
        - Before the shared repair, the public catalog tests rejected an anchored quoted number arm and lost the earlier duplicate location. Standalone flow mapping and anchor-only dash-line probes also failed before the repair.
        - The locator now uses the same node-property grammar on mapping values and sequence items. It recognizes an alias as an authored token, so direct grammar diagnostics keep it and structural source maps can mark it for resolved, category and reference checks. Written signatures still use exact decoded-to-authored ranges.
        - The initial focused runs passed 28 Simplified Schema catalog tests, 2,199 minimal-feature tests, and 162 Darkmatter, CLI, and DMLS witnesses. They covered both catalog kinds, prior comment and reference cases, accepted and rejected compose dispatch, and the installed profile and verified snapshot.
        - The adjacent whole-value alias sweep found that an aliased object-constraints mapping also passed YAML parsing and then failed source projection. A root-union CLI probe with two valid closed arms, the second referring to the first arm's constraints, reproduced it at document start.
        - The final Simplified Schema regression run passed 31 selected catalog tests and all 2,202 minimal-feature tests. The controls verify whole-value scalar, pipe and sequence aliases, an unrelated helper alias, both directions of a missing narrowing clause, and ordinary root-arm, constraint-map and composition-operand source projection.
        - Narrowing diagnostics keep the grammar's existing rule. The declaring clause is primary; the overload missing that clause is related. Either location uses the authored alias token if that arm reuses a signature.
        - The 31-test Claudine publication and inventory slice passed. The 75-test doctor slice also passed before the five new fixture cases were split into separate L1 tests to keep their timings safely below the 5-second budget.
        - The five separately scheduled doctor and publisher tests passed in 1.134–1.204 s each. Claudine admitted both written and aliased number arms. The written, anchored, and aliased duplicates each kept the variant's two expected positions through both verified inspection and doctor.
        - An independent CLI batch passed eight cases for both catalog kinds, including an unused anchor, a later alias, and an earlier alias on a duplicate. Each negative case returned one catalog refusal and two independently calculated byte spans. The two copied-contract controls validated.
        - An aliased composition operand accepted a document with a valid nested value and enforced the inherited minimum length at `/item/text` on the invalid control.

        - The focused implementation and regression checks finished at 01:50:59.
        - The first full Darkmatter run stopped at the older standalone presentation-boundary test. It had asserted that `!!str` alone makes a reference unprojectable. Standard value tags now use the same locator as sequence items. The test compares the supported tagged reference with the block-scalar control and checks the exact reference span. A decorated mapping key still passes semantic parsing but fails source projection, retaining the negative control.
        - The second full run cleared the reference test and exposed the same assumption in the YAML spelling matrix. A search for tag spellings, alias projection and the projection-error text found the public definition and editor fallback siblings. Updated the tag cases to require semantic parity and exact ranges, retained source failure controls using a YAML version directive, and asserted the later alias range in the repeated-suggestion case.
        - The third full run passed the projection tests and stopped at `loadable_shipped_schemas_resolve_and_lower` because `schemas/suggestion-review.yaml` had been removed during the cycle. Restored that required schema byte for byte from the Git index. The corpus assertion and prompt reference retain their existing meaning.

- work completed for 'R3-F1 YAML anchors and aliases break overload validation in sequences' at 02:24:51

### Completion Report

- **complete** after 49 min 45 s.
- Parent findings total 1 = 1 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed.
- Instance rows total 19 = 16 fixed + 3 clean. The separate verification issue R3-V1 was fixed. There are no open finding or child rows.
- R3-F1 uses the same YAML node-property and alias rules for mapping values and block and flow sequence items. The parsed signatures determine admission and binding. The locator uses the authored alias token for the source range when the signature text is supplied at an anchor. Direct and resolved overload conflicts retain both authored locations through CLI, DMLS, and verified registration.
- The standalone parser, CLI, and editor retain unsupported-presentation controls. The first two full Darkmatter runs found older tests that treated standard value tags as unprojectable. The updated tests verify the supported spellings and use separate unsupported key or directive fixtures for fallback behavior. The third run found the required suggestion-review schema missing. The restored file matches the Git index.
- The changes exercise platform-neutral YAML parsing and public diagnostic ranges. Level 1 on Linux covers the affected behavior.

| Verification command | Final result | Evidence |
| --- | --- | --- |
| Simplified Schema `just test` | 2,626 default-feature and 2,202 minimal-feature tests passed. Zero slow tests. | R3-F1.1–.5, .7, .10–.15 |
| Darkmatter `just test` | 18,037 tests passed. Zero slow tests. | R3-F1.2–.5, .7–.11, .16–.19 and R3-V1 |
| Claudine `just test` | 13,077 tests passed. Zero slow tests. The slowest new doctor case took 1.745 s. | R3-F1.6 |
| `just lint` in Simplified Schema, Darkmatter, and Claudine | Passed in all three areas, including the Zed WASM check. | All Rust and test changes |
| `just build` and `just shipped-schemas --check` in all three areas | Passed. Checked-in artifacts match their generators. | R3-F1 and the shipped corpus |
| `just check-tier-coverage simplified-schema darkmatter claudine` and `just check-tier-coverage darkmatter` | Passed with zero stranded tests, including the Darkmatter retest after the additional boundary changes. | All added Level 1 regressions |
| Darkmatter `just publish-skills` | Published `use-darkmatter` and `use-dmls`. | Caller documentation |
| `git diff --check` | Passed. | Combined changes |

- Raw command output is under `/tmp/darkmatter-review-3.NOQXTR2f`. The final full-run transcripts include the installed two-overload assertion, exact tagged-schema ranges and the restored corpus check.
-    Changed files in iteration 3 (21)
        - `.claude/skills/darkmatter/schema.md`
        - `claudine/lib/src/doctor/registration_tests.rs`
        - `darkmatter/README.md`
        - `darkmatter/cli/tests/l1/compose_host_scope.rs`
        - `darkmatter/cli/tests/l1/schema_validate_yaml.rs`
        - `darkmatter/dmls/docs/diagnostics.md`
        - `darkmatter/dmls/tests/l1/schema_yaml_spellings.rs`
        - `darkmatter/dmls/tests/l1/yaml_document_catalog_editor.rs`
        - `darkmatter/docs/topics/host-functions.md`
        - `darkmatter/docs/use-skills/use-dmls/schemas.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-3.md`
        - `darkmatter/lib/tests/l1/host_registration_companions.rs`
        - `darkmatter/lib/tests/l1/meta_schema_phase6.rs`
        - `darkmatter/lib/tests/l1/schema_yaml_spellings.rs`
        - `darkmatter/lib/tests/l1/schemas_source_projection.rs`
        - `simplified-schema/README.md`
        - `simplified-schema/docs/topics/diagnostics.md`
        - `simplified-schema/docs/topics/language/types.md`
        - `simplified-schema/lib/src/simplified/source.rs`
        - `simplified-schema/lib/tests/l1/callable_catalog_envelopes.rs`

## Implementation of Review Findings #4

> **started at:** 2026-11-25T02:33:46+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-4.md'
- this is iteration 4 of the review-to-implement cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R4-F1 | The repository kind schema is missing from the worktree | fixed | The restored schema matches `HEAD` byte for byte. Both corpus checks require all six root artifacts and all children have focused evidence. |
| R4-F1.1 | Markdown validation admits expression and mutation catalog kinds and locates the unsupported-kind error | fixed | The public API and direct CLI accept both catalog kinds. `unsupported-catalog` produces exactly one constraint error at `/kind`, line 3, column 7. |
| R4-F1.2 | The kind guide transcludes the shared definitions and the repository prompt resolves its direct schema link | fixed | The guide composes with `--deny all` and includes the complete enum. The prompt's schema link passes independently in R4-F1.11. |
| R4-F1.3 | The shipped-schema corpus refuses to omit the shared kind schema and resolves and lowers it when present | fixed | `loadable_shipped_schemas_resolve_and_lower` failed at the new presence assertion while the schema was absent and passed after the restore. The ordinary resolution branch lowers it as a Simplified Schema. |
| R4-F1.4 | Shipped per-kind definitions and both built-in catalog inputs retain their validation behavior | clean | CLI validates the built-in expression and mutation catalogs against their respective embedded definitions. The registry test validates a callable catalog against its shipped kind definition. R4-F1.12 checks the definition files directly. |
| R4-F1.5 | Earlier review-related siblings retain artifact-presence and ordinary resolution coverage | clean | All four review-related schemas pass direct validation and remain explicitly named by both corpus checks. The focused suite passed the earlier two-arm Markdown validation test, including the required-field errors and invalid package shape. |
| R4-F1.6 | The sibling sweep checks other required root schemas, direct consumers, and corpus entry points for silent omissions | fixed | Both corpus passes now use the same required inputs. They check the four already-protected review schemas, plus the kind and evidence schemas. The classification test's two-schema count is a separate classification invariant. Its misleading description and failure message now state that contract. |
| R4-F1.7 | The corpus's test-input index selects the relevant tests when a required root artifact changes or disappears | fixed | `test_inputs.scan` returns both exact corpus tests for all six paths, plus the public Markdown regression for the kind schema. Selection succeeds for the kind candidate supplied as a deletion. |
| R4-F1.8 | The passive parser rejects a missing required root schema and still parses the restored schema without effects | fixed | `shipped_schema_artifacts_parse_passively` failed at the named missing-file assertion and passed after restoration, including its no-effects and no-write assertions. |
| R4-F1.9 | The CI evidence catalog remains available to its documented named-type consumer and both corpus passes detect its deletion | fixed | Each corpus test independently failed with `schemas/ci-evidence.yaml is part of the corpus` when the file was absent. The restored file validates the documented version 3 observation sample and rejects version 4 at `/evidence/schema_version`, line 5, column 19. |
| R4-F1.10 | Root schema classification verifies present files and the feature-review reference continues to validate through its public API | clean | All three `meta_schema_repo_schemas` tests passed. They check pure and tagged envelopes, bare-name resolution to `anyOf`, and validation of a valid review plus rejection of numeric `ready`. |
| R4-F1.11 | The direct schema link in the repository prompt resolves at its authored location | fixed | `md validate refs system-prompt.md --show-all --output plain` lists `^schemas/kind.yaml` at 30:100 and reports all five references good. |
| R4-F1.12 | Both shipped Simplified Schema definitions still validate as YAML kind definitions | clean | Direct CLI validation of both `.yaml` definitions reports that each successfully uses the shipped `yaml-schema.yaml` definition. |

- starting the work on 'R4-F1 The repository kind schema is missing from the worktree' at 02:34:08
        - A filesystem scan silently omits a required schema when the artifact disappears, even though its consumers still resolve the documented path.
        - The review reproduced all three kind-validation inputs failing for the absent file. The committed-file control admits both supported values and rejects `unsupported-catalog` at 3:7. The guide and prompt use the same checked-in path, and both corpus tests have required-artifact checks that omit it.
        - The initial sibling sweep found six root schemas. The four review-related artifacts are present and already have direct checks. The evidence catalog is also present and has documented consumers, so its presence contract will be checked with the same sweep.
        - The implementation plan requires both callable kinds in the existing catalog and asks that canonical links stay in the docs. Restoring the committed bytes preserves that contract without changing the schema format.
        - Reproduced the missing-file failure with both catalog kinds and the unsupported kind using the worktree path. A temporary copy of the committed bytes accepted both catalog kinds and gave one enum error at 3:7 for the unsupported value. Before restoration the guide failed at 5:1 and the prompt link at 30:100.
        - Both corpus passes scan files that exist, and their exact root-file lists also drive CI's test-input indexing. The same omission could affect the documented CI evidence catalog, which has a named-type consumer and no independent presence assertion. The repository-root classification test checks the envelope of each present ordinary schema. The corpus owns inventory so classification need only ensure it classified a nonempty set.
        - Restored the kind schema byte for byte from the committed version. Both corpus entry points now name all six root artifacts directly, which keeps CI's index narrowed to those tests. The classification-only sibling now describes its actual minimum count and relies on the corpus for inventory.
        - Both corpus entry points enforce the same rule for required root artifacts. Keeping the explicit paths in each test lets the planner select the two exact tests. Moving those paths into a shared helper would select the entire test binary.
        - With the kind schema absent, both corpus checks failed by naming `schemas/kind.yaml`. With just the CI evidence schema temporarily absent, both checks failed by naming `schemas/ci-evidence.yaml`. Both files are present and match their committed bytes.
        - The worktree schema accepts both callable kinds. The rejection for the unsupported kind identifies `/kind` at 3:7. The guide embeds both enum literals and all five prompt references resolve. All four review-related artifacts and both built-in catalog files and their per-kind definitions passed direct validation. The CI evidence guide passed all eight links and its sample validated. Changing only the sample environment failed at 6:16.
        - The focused Darkmatter library run passed all 30 corpus, root classification, and callable kind registry checks. None reached five seconds. The index selected both corpus tests for each root schema and the Markdown regression for the kind schema. Raw probes and the focused nextest transcript are under `/tmp/darkmatter-review-4-probes.cjf9AUrb`.
- work completed for 'R4-F1 The repository kind schema is missing from the worktree' at 02:41:26

### Completion Report

- **Cycle outcome** — complete. Elapsed duration 9 minutes, 18 seconds.
- **Parent findings** — 1 total = 1 fixed + 0 already fixed + 0 blocked + 0 unresolved + 0 disputed.
- **Required instances and sibling checks** — 12 total, with 8 fixed and 4 clean.
- **Open finding or child IDs** — none.
- Restored `schemas/kind.yaml` byte for byte from `HEAD`. The kind guide embeds both callable kinds and all five prompt references resolve. The public Markdown test and the CLI admit both kinds and reject the unsupported value at 3:7.
- Both corpus passes require all six root schemas. The missing kind and CI evidence cases each failed individually with the missing path. The restored files pass passive parsing and ordinary resolution or named-type import. The earlier review-schema cases and shipped callable definitions pass.
- Level 1 covers the changed paths through passive library and CLI calls. The source edits are portable tests over repository files, and the schema is an exact restoration. These changes carry no notable cross-OS risk.

| Final verification | Result | Finding or instance |
| --- | --- | --- |
| Darkmatter `just test` | 18,038 passed, 10 filtered by the recipe, zero at or above five seconds. The new Markdown regression took 0.020 s and both corpus checks passed. | R4-F1.1, .3–.10, .12 |
| Darkmatter `just lint` | Passed, including the Zed WASM check. | Both changed Rust test files |
| Darkmatter `just build` | Passed for the library, CLI, language server, and Zed CLI. | R4-F1.1–.4, .8 |
| Darkmatter `just shipped-schemas --check` | Passed. Checked-in shipped artifacts match the generator. | R4-F1.4, .12 |
| Root `just check-tier-coverage darkmatter` | Passed with zero stranded tests. | R4-F1.1, .3–.10 |
| Repository-built `md compose docs/kind-documents.md --deny all --output markdown` and `md validate refs system-prompt.md --show-all --output plain` | Passed. The composed enum includes both callable kinds, and the prompt link resolves at 30:100. | R4-F1.2, .11 |
| Repository-built `md schema validate --output plain` on the Markdown probes, both callable catalogs, and their two YAML kind definitions | Expected admissions and located rejection observed. Direct validation also passed all four review-related schema siblings. | R4-F1.1, .4–.5, .9, .12 |
| Focused corpus, root classification, and callable kind registry tests via `just _test darkmatter --features effects-instrumentation,work-counters` with module filters | 30 passed, zero slow tests. The two guarded corpus tests also failed individually for each of the two temporarily missing schemas. | R4-F1.1, .3–.10 |
| CI `test_inputs.scan`, committed-schema byte comparison, and `git diff --check` | Passed. The index found the exact test filters for all six root artifacts. | R4-F1.3, .6–.8 |

- Verification transcripts are under `/tmp/darkmatter-review-4.jKmze6kQ` and `/tmp/darkmatter-review-4-probes.cjf9AUrb`.
- Changed files
        - `schemas/kind.yaml` (restored to the committed bytes)
        - `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`
        - `darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-4.md`

## Implementation of Review Findings #5

> **started at:** 2026-11-25T03:42:27+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-5.md'
- this is iteration 5 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R5-F1 | The kind guide's expression catalog uses a reserved name | fixed | All five children verified; example key is `project_slug`. |
| R5-F1.1 | Repository kind guide block validates, with comment variant; a reserved key still gives the two-location error | fixed | Extracted block exit 0; comment variant exit 0; `kebab_case` variant: `dm.schema.callable_catalog` at 7:5 with related `functions.yaml:312:5`; guide composes with `--deny all`. |
| R5-F1.2 | Simplified Schema callable catalog guide example validates | clean | Review exit 0; block unchanged (`project_slug`, inline comment, two overloads). |
| R5-F1.3 | Host function guide `is_project_file` and `set_frontmatter` examples validate | clean | Review exit 0 for both; blocks unchanged. |
| R5-F1.4 | Host registration guide `./functions/project.yaml` catalog validates | clean | Review exit 0; block unchanged. Sweep found no other Markdown declaring a callable catalog. |
| R5-F1.5 | Caller skills reference guidance; both built-in catalogs validate | clean | Caller skills hold no complete catalog (grep of `^kind: (expression\|mutation)-schema`); built-ins untouched; review exit 0. |
| R5-F2 | The category guide misstates which catalogs require categories | fixed | General Categories section scopes optional/lenient rules to informal, `kind: schema`, `kind: yaml-schema`, states the callable requirement with a link; repo skill null-carrier note qualified. CLI: absent/null/`[]`/`[5]` exit 1, `[Text]` exit 0; `md validate refs` 44 ok. |
| R5-F3 | Reference commands miss schemas named by bare filename | fixed | Root lookup decoupled from triggers (`with_schema_root_lookup`); all seven children verified. The in-repository controls for compose, clean, schema validation, and path completion resolve with `--disable schema-triggers`. |
| R5-F3.1 | `md validate refs`: bare and absolute check the link; missing fails | fixed | `reference_bare_schema_roots::bare_root_schema_prepares_like_the_absolute_path`; CLI tests `reference_bare_schema::*`; `review-4.md` exit 0 (was 1); missing-name copy exit 1. |
| R5-F3.2 | `md validate refs --graph mermaid`: bare and absolute produce the graph; missing fails | fixed | `review-4.md --graph mermaid` exit 0; CLI test covers bare and missing. |
| R5-F3.3 | `md graph` with/without `--validate`: bare and absolute agree; missing fails | fixed | `md graph` and `md graph --validate` on `review-4.md` exit 0; missing copy `--validate` exit 1. |
| R5-F3.4 | Inclusion preparation (`::file`) under refs, Mermaid, and `md graph --follow --validate` | fixed | `bare_schema_in_an_included_document_prepares` (2 nodes, no issues bare/absolute; missing fails); `graph --follow --validate` exit 0. |
| R5-F3.5 | Fragment validation finds a heading generated from frontmatter; static-heading fallback preserved | fixed | `fragment_target_with_bare_schema_finds_generated_heading`: bare/absolute resolve both headings; missing schema keeps static fallback and reports the generated one as `MissingFragmentTarget`. |
| R5-F3.6 | Named-type import `Name@ci-evidence.yaml` accepted bare and absolute in standalone preparation | fixed | `bare_named_type_import_prepares_like_the_absolute_path`; by hand `validate refs` and `graph --validate` exit 0 bare, exit 1 for `@absent-evidence.yaml`. |
| R5-F3.7 | `md schema validate` and `md compose --deny all` controls preserved | fixed | Unchanged results: schema validate and compose exit 0 for `review-4.md` (with and without `--disable schema-triggers`), missing exit 1; `compose_resolves_bare_schema_with_or_without_trigger_schemas`; full suites green. |

### Progress

- starting the work on 'R5-F1 The kind guide's expression catalog uses a reserved name' at 03:42:50
        - defect class: a copyable authoring example breaks the admission rules it documents (here, a reserved built-in callable name)
        - done by the orchestrator directly; the repair is one key in a Markdown example, so a subagent added only cost
        - sweep: every Markdown file with a fenced `kind: expression-schema` or `kind: mutation-schema` block outside features/fixes is `docs/kind-documents.md`, `darkmatter/docs/topics/host-functions.md`, `darkmatter/docs/topics/host-registration.md`, `simplified-schema/docs/topics/language/schema-documents.md`; the last three were validated clean by the review (R5-F1.2–.4) and no other file declares a callable catalog
        - other `kebab_case` mentions (`darkmatter-expressions.md`, `use-darkmatter/authoring.md`) are calls of the built-in, not declarations
        - changed `kebab_case:` to `project_slug:` in the kind guide block, matching the language guide
        - extracted block: exit 0; with an inline helper comment: exit 0; renamed back to `kebab_case`: `dm.schema.callable_catalog` at 7:5 with the related built-in declaration at `functions.yaml:312:5`
        - `md compose docs/kind-documents.md --deny all --output markdown` exits 0
- work completed for 'R5-F1 The kind guide's expression catalog uses a reserved name' at 03:43:50
- starting the work on 'R5-F2 The category guide misstates which catalogs require categories' at 03:43:10
        - defect class: a general rule in shared guidance is stated for every envelope while one envelope family follows a stricter rule
        - rewrote the general Categories section of `simplified-schema/docs/topics/language/schema-documents.md`: names the optional envelopes (informal, `kind: schema`, `kind: yaml-schema`), states the callable catalogs' requirement with a link to their section, keeps the shared syntax and the rules common to all envelopes, and scopes the lenient null/number rules to the optional envelopes
        - sweep: the caller skill table (`use-dmls/schemas.md`) already marks `categories` required for both callable kinds (clean); the repo skill `.claude/skills/simplified-schema/schema-documents.md` said a null `categories` is omitted without exception, so it now notes the callable catalogs refuse it
        - CLI probes on an expression catalog: categories absent, `null`, `[]`, and `[5]` each exit 1; `[Text]` exits 0, so every new sentence matches behavior
        - `md validate refs` on the language guide: 44 references, all ok (the new anchor resolves)
- work completed for 'R5-F2 The category guide misstates which catalogs require categories' at 03:44:10
- starting the work on 'R5-F3 Reference commands miss schemas named by bare filename' at 03:44:40
        - delegated to one foreground rust-developer subagent; its report was reconciled by the orchestrator (diff read, registry-reuse claim checked, one drifted comment fixed)
        - defect class: an authored schema reference (root `$schema` or a `Name@file` import) loses its schema-root lookup on any preparation path that installs no trigger registry, because root search was tied to trigger discovery
        - confirmed cause: `DarkmatterSchemas` took bare-name roots only from an installed trigger registry (`site_schema_with_override`, `pending_templates_for`); `prepare_schemas` installs one only when `trigger_schemas` is on, which `ComposeOptions::for_document` (used by `md validate refs` and `md graph`) leaves off
        - repair: `DarkmatterSchemas::with_schema_root_lookup()` computes `SchemaRoots::for_document` for the instance's context without discovering triggers; a private `bare_name_roots()` prefers an installed registry's roots, else these; selection still goes through `select_schema_file` (no second selector)
        - `prepare_schemas` installs root lookup whenever no registry is installed for a file source inside a repository, covering the reference check, graph, inclusions, fragment targets, pre-flight collection and lifecycle, setters, pipeline, and `md_host.rs`; trigger application is unchanged where it was off
        - sibling sites:
                - `md compose --disable schema-triggers`: same defect (exit 1 before, exit 0 after) — fixed by the same line
                - `md clean` (`CleanSchemaConfig::resolve`) without triggers — fixed
                - `md schema validate --disable schema-triggers` — fixed via a `document_schema_roots` helper, CLI test added
                - path completion `effective_document_schema` — fixed through the same helper
                - `discover_document_triggers`, `md schema triggers` — clean, they always discover so a registry supplies roots
                - `data/contract.rs`, `schema_files.rs`, `yaml_documents` — clean, they call `SchemaRoots::for_document` or the kind registry's roots
                - DMLS overlays — clean: no trigger switch, always installs a registry inside a workspace; its documented rule is no roots outside a workspace
                - registry reuse across documents (subagent's open note) — clean: every `prepare_schemas` caller passes a fresh per-document `None` (`pipeline/mod.rs:428`, `setters.rs`, `effective_schema_for`, `preflight/collect.rs`, `preflight/lifecycle.rs`, `md_host.rs`), so a registry is reused only across one document's passes
        - decision: `--disable schema-triggers` was documented as disabling "schema-trigger discovery and bare-name schema-root lookup" — the same coupling as the defect; the flag now stops trigger discovery only, and `docs/cli/{policy,compose,clean}.md` and the `policy.rs` summary say so. Alternative was a separate `ComposeOptions` field for root lookup (more identity surface, keeps an option nobody needs); rejected as speculative. Ledger-only decision (easily reversed)
        - tests added: `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs` (5 tests) and `darkmatter/cli/tests/l1/reference_bare_schema.rs` (2 tests); the 4 library reference tests fail with the `prepare_schemas` change disabled and pass with it
        - docs: `darkmatter/docs/cli/{validate,graph,policy,compose,clean}.md`, `darkmatter/docs/topics/schemas/definition.md` (Schema roots); skill `.claude/skills/darkmatter/SKILL.md`
- work completed for 'R5-F3 Reference commands miss schemas named by bare filename' at 04:00:30

### Completion Report

- **outcome: complete** — elapsed 2026-11-25T03:42:27+00:00 → 04:11 UTC (about 29 minutes)
- parent findings: total 3 = fixed 3 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 12 = fixed 8 (R5-F1.1, R5-F3.1–.7) + clean 4 (R5-F1.2–.5)
- open findings: none
- verified repairs:
        - R5-F1: the kind guide's callable catalog example uses `project_slug`, validates in isolation and with a comment, and a reserved name still yields the two-location refusal
        - R5-F2: the general Categories guidance distinguishes the optional envelopes from the callable catalogs' required, strictly admitted `categories`
        - R5-F3: bare-name `$schema` and `Name@file` lookup no longer depends on trigger discovery for the checked in-repository documents, inclusions and fragment targets. A name no root holds still fails. A descendant outside Git can still inherit schema-root eligibility from a repository parent.
- decision (ledger only): `--disable schema-triggers` now stops trigger discovery only, not bare-name lookup; docs and help updated
- cross-OS: no OS-specific code was added (root computation reuses `SchemaRoots::for_document`); cross-check not run, left to CI
- changed files:
        - R5-F1: `docs/kind-documents.md`
        - R5-F2: `simplified-schema/docs/topics/language/schema-documents.md`, `.claude/skills/simplified-schema/schema-documents.md`
        - R5-F3: `darkmatter/lib/src/markdown/schemas/mod.rs`, `darkmatter/lib/src/markdown/compose/schema_validation.rs`, `darkmatter/lib/src/markdown/schemas/clean.rs`, `darkmatter/cli/src/commands/schema/validate.rs`, `darkmatter/cli/src/args/policy.rs`, `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs` (new), `darkmatter/lib/tests/l1/main.rs`, `darkmatter/cli/tests/l1/reference_bare_schema.rs` (new), `darkmatter/cli/tests/l1/main.rs`, `darkmatter/docs/cli/{validate,graph,policy,compose,clean}.md`, `darkmatter/docs/topics/schemas/definition.md`, `.claude/skills/darkmatter/SKILL.md`
        - records: this log, `review-5.md` frontmatter
- final verification (run by the orchestrator after all changes, no subagent running):
        - `darkmatter`: `just lint` exit 0; `just test` 18,045 passed, 10 skipped, none at or above 5 s (R5-F3, plus regression for all earlier findings)
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `claudine`: `just test` 13,077 passed, 16 skipped, none at or above 5 s (consumer of `prepare_schemas`)
        - `just slow-tests fresh` exit 0
        - `simplified-schema`: documentation-only change, and no test reads the edited page; suite not rerun
        - by hand: R5-F1 extraction probes; R5-F2 category probes; R5-F3 `validate refs`, `--graph mermaid`, `graph`, `graph --validate`, `graph --follow --validate`, `schema validate` on `review-4.md` all exit 0, missing-name copy exit 1; `md validate refs review-5.md` 54 ok


### Review 6 record correction (2026-11-25)

- The iteration 5 completion report generalized the in-repository result to the document's own schema roots for all inclusions and fragment targets. The source determines which folders are inspected, while eligibility in shared preparation still uses the enclosing request's repository. Using the worktree-built `md`, I put the schema in `~/schemas/` and checked the same note outside Git directly and via a repository parent. Direct validation refused the bare name. The inclusion and generated-heading link checks accepted it. Explicit relative and absolute spellings worked in both positions, and the original in-repository cases retained their recorded results. [Review 6 finding](review-6.md#r6-f1-high-included-documents-and-fragment-targets-inherit-schema-root-eligibility-from-the-parent)

## Implementation of Review Findings #6

> **started at:** 2026-11-25T04:20:51+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-6.md'
- this is iteration 6 of the review-to-implement cycle
- review frontmatter `implemented` confirmed `false` before repairs began

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R6-F1 | Included documents and fragment targets inherit schema-root eligibility from the parent | fixed | `searches_schema_roots` decides from the source context `DarkmatterSchemas` is built with; both root selectors (trigger discovery, bare lookup) share it. All children verified; 6 library + 2 CLI L1 regressions and the unit test `schema_root_search_needs_a_source_repository` fail with the old check restored. |
| R6-F1.1 | Reference report and Mermaid graph for an inclusion of an outside-Git note: explicit spellings pass for repository and outside-Git parents; bare and missing fail at the included note | fixed | Repository parent: `validate refs` and the Mermaid graph exit 1 naming `review.yaml` for bare and `absent.yaml` for missing; `./schemas/review.yaml` and absolute exit 0 (bare exited 0 before). Outside-Git parent: same results. Library test `external_included_note_refuses_a_bare_schema_under_any_parent`. |
| R6-F1.2 | `md graph`, `--validate`, `--follow --validate` over the same inclusion accept explicit and refuse bare and missing | fixed | `graph`, `graph --validate`, `graph --follow --validate`: bare and missing exit 1, explicit exit 0. CLI test `reference_commands_refuse_a_bare_schema_in_an_included_home_note`. |
| R6-F1.3 | `md validate refs --fragments`: only explicit spellings generate the heading; static-heading fallback preserved | fixed | `validate refs --fragments link.md`: bare and missing report the generated heading missing (exit 1), explicit exit 0; static heading with `absent.yaml` exit 0. Library test `external_fragment_target_generates_its_heading_only_from_an_explicit_schema`. |
| R6-F1.4 | Named-type import `entry@types.yaml` in the external note: explicit accepted, bare and missing refused on inclusion report, follow-and-validate graph, fragment check, compose with triggers disabled | fixed | `entry@types.yaml`: bare and missing fail in the inclusion report, follow-and-validate graph, fragment check, and `compose --disable schema-triggers`; relative and absolute pass; direct check unchanged. Library test `external_note_refuses_a_bare_named_type_import`. |
| R6-F1.5 | Composition of the repository parent with and without `--disable schema-triggers`: generated heading only for explicit spellings; bare and missing fail in the child | fixed | `md compose root.md` with and without `--disable schema-triggers`: bare and missing exit 1 at the child, explicit renders `# Review heading`. Library test `composing_a_repository_parent_refuses_the_external_bare_schema`. |
| R6-F1.6 | Direct checks of the external note (refs, schema validate, compose, clean, completion) and the R5-F3 in-repository controls keep their results | clean | Direct `schema validate`, `validate refs`, `compose` of `~/review.md` give identical results before and after; orchestrator probe of `md clean ~/review.md`: bare exit 1 (`$schema unresolved`), `./schemas/review.yaml` exit 0, absent exit 1. `clean.rs`, `schema validate`, and completion already use the document context; R5-F3 in-repository tests pass. Library control `external_note_checked_directly_refuses_a_bare_schema`. |
| R6-F1.7 | Default composition skips root trigger discovery for the external included note; in-repository trigger behavior preserved | fixed | Home trigger matching `topic: review` requiring `version`: `compose root.md` now exits 0 for both topics (review topic exited 1 before); refs exit 0 for both; in-repository control `inroot.md` → `./child.md` still exits 1 naming `version`. Library test `root_triggers_apply_to_repository_children_but_not_to_an_external_note`, CLI test `compose_skips_root_triggers_and_bare_names_for_an_included_home_note`. |

### Progress

- starting the work on 'R6-F1 schema-root eligibility inherited from the parent' at 04:22:10
        - defect class: the decision whether a document may search the schema roots (bare-name lookup and trigger discovery) was taken from an enclosing request's repository scope instead of the document's own source context
        - delegated to one foreground `rust-developer` subagent; its log items are grouped below
        - subagent log items:
                - `searches_schema_roots` now takes the source context that `DarkmatterSchemas` is built with (`options.source_file_resolution_context()`); both the trigger-discovery and the bare-name lookup branch of `prepare_schemas` read that one answer, so trigger application and bare lookup share one rule
                - discovery: every caller of `prepare_schemas` passes a fresh, empty trigger registry, so a parent's registry is never reused for a child; R6-F1.7 came only from the eligibility check
                - discovery: a child in a different repository searches its own captured repository's roots; no repository is discovered mid-request
                - sweep (clean): `md clean` (`schemas/clean.rs`), `md schema validate` (`cli/src/commands/schema/validate.rs`), schema-aware completion, and `validate_schema()` already decide from the document's own context; DMLS keeps its own workspace-boundary rule and does not call `prepare_schemas`
                - discovery: library compose reports a failed child as a skipped transclusion, `md compose` exits 1; library tests assert the skip count and warning text
                - tests: 6 new library tests in `lib/tests/l1/reference_bare_schema_roots.rs` (isolated home via `FileResolutionContext::from_snapshot`), 2 new CLI tests in `cli/tests/l1/reference_bare_schema.rs`, and the unit test `schema_root_search_needs_a_source_repository`; with the old check restored, 5 of the 6 new library tests and both CLI tests fail while the R5-F3 controls pass
                - docs: `docs/topics/schemas/definition.md#schema-roots`, `docs/cli/validate.md`, `docs/cli/graph.md`, `.claude/skills/darkmatter/SKILL.md`
                - CLI reproduction with an isolated `HOME`, before and after the fix, recorded for every row
        - the author committed the in-flight source change and an earlier test version mid-session as `9b9300cfb`; the later test, doc and skill edits remain uncommitted
        - orchestrator review: caller skills (`use-dmls/schemas.md`, `use-darkmatter/*`) make no claim that an external child inherits a parent's roots; no publish needed
- work completed for 'R6-F1 schema-root eligibility inherited from the parent' at 04:33:50

### Completion Report

- **outcome: complete** — elapsed 2026-11-25T04:20:51+00:00 → 04:42 UTC (about 22 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 7 = fixed 6 (R6-F1.1–.5, R6-F1.7) + clean 1 (R6-F1.6)
- open findings: none
- shared rule: bare-name lookup and trigger discovery use the repository recorded in the source context. Composition, reference graphs, and heading preparation agree with direct checks for documents in the captured repository and for the tested notes outside Git. A direct CLI open can capture another repository. Nested preparation in this iteration only derives from the current request and can miss that repository. See the dated review 7 correction below.
- earlier same-class cases rerun: the R5-F3 in-repository library and CLI tests (root, child, fragment, named import, `--disable schema-triggers`) pass alongside the new probes
- cross-OS: no OS-specific code or path comparison changed; cross-check not run, left to CI
- changed files:
        - R6-F1 (source, committed by the author mid-session as `9b9300cfb`): `darkmatter/lib/src/markdown/compose/schema_validation.rs`
        - R6-F1 (tests): `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs`, `darkmatter/cli/tests/l1/reference_bare_schema.rs`
        - R6-F1 (docs/skills): `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/docs/cli/validate.md`, `darkmatter/docs/cli/graph.md`, `.claude/skills/darkmatter/SKILL.md`
        - records: this log, `review-6.md` frontmatter
- final verification (orchestrator, after all changes, no subagent running):
        - `darkmatter`: `just lint` exit 0; `just test` 18,053 passed, 10 skipped, none at or above 5 s (R6-F1 and regression for every earlier finding)
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `claudine`: `just test` 13,077 passed, 16 skipped, none at or above 5 s (consumer of `prepare_schemas`)
        - `just slow-tests fresh` exit 0
        - by hand: isolated-`HOME` CLI reproduction of every R6-F1 row before and after the fix (subagent), plus the orchestrator's `md clean` probe

### Review 7 record correction (2026-11-25)

The iteration 6 completion report said the schema-root decisions now agree across composition, reference preparation, cleaning, and validation. That holds for the seven iteration 6 instances. It does not hold for an included document in a second checkout that the request has not captured. With two temporary Git checkouts and an isolated home, the worktree-built `md schema validate ~/notes/review.md` accepted the note's bare `$schema: review.yaml`. `md compose root.md` and `md validate refs root.md` refused that schema when the root used `::file ~/notes/review.md`. The explicit spelling passed both paths. With the explicit spelling, direct composition applied the note's matching trigger and reported its missing `version` field, while inclusion skipped that trigger. The same source-context check runs in each path, but the nested context derives from the enclosing request's captured repository. [Review 7 finding](review-7.md#r7-f1-high-documents-opened-through-another-checkout-lose-their-schema-roots)

## Implementation of Review Findings #7

> **started at:** 2026-11-25T04:50:10+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-7.md'
- this is iteration 7 of the review-to-implement cycle 
- review frontmatter `implemented` confirmed `false` before repairs began

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R7-F1 | Documents opened through another checkout lose their schema roots | fixed | Every child row is fixed or clean. Admitted sources in another Git checkout get `SourceDerivation::OwnRepository`, decided once per source at admission. A per-request `SourceRepositories` memo is shared by every consumer. 21 new tests fail with the fix reverted. |
| R7-F1.1 | Reference report and Mermaid graph over body, `prologue:`, `epilogue:` inclusion of a second-checkout note: bare, relative, absolute succeed; missing fails at the child | fixed | CLI: bare went from exit 1 to 0 for `validate refs` and the Mermaid `graph` on body, prologue and epilogue parents. Relative and absolute: 0. Missing: 1, naming `absent.yaml`. Library tests `second_checkout_{body_inclusion,prologue,epilogue}_uses_the_notes_schema_roots`. CLI tests `second_checkout_refs_of_*`, `second_checkout_mermaid_graph`. |
| R7-F1.2 | `md graph`, `--validate`, `--follow --validate` × three parents: all accept bare and explicit, refuse missing | fixed | Plain `graph`, `--validate` and `--follow --validate` on all three parents: bare went from 1 to 0, and missing still exits 1. CLI tests `second_checkout_validated_graph`, `second_checkout_followed_graph`. |
| R7-F1.3 | Fragment preparation and `::toc-linking`: bare and explicit generate the heading and let the TOC target prepare; missing fails; static fallback kept | fixed | `validate refs --fragments`: bare now generates `review-heading` and exits 0 (was 1). Missing: 1. Static heading: 0 in every case. TOC target under `validate refs`, `graph --follow --validate` and compose: bare went from 1 to 0, and missing still exits 1. Library tests `second_checkout_fragment_target_generates_its_heading_from_a_bare_schema`, `second_checkout_toc_target_prepares_with_a_bare_schema`. CLI tests `second_checkout_fragment_target`, `second_checkout_toc_target_compose`. |
| R7-F1.4 | Composition (default and disabled triggers) and `md compose --shell` discovery: accept bare and explicit, missing fails | fixed | `compose` and `compose --disable schema-triggers` on all three parents, plus `compose --shell`: bare went from 1 to 0, and missing still exits 1. CLI tests `second_checkout_compose`, `second_checkout_compose_without_triggers`, `second_checkout_command_discovery`. The library inclusion tests cover `compose_preflight`. |
| R7-F1.5 | Named-type import `entry@types.yaml` from the second checkout: bare and explicit resolve in direct and nested positions; missing fails | fixed | `entry@types.yaml` in body refs, the follow-and-validate graph, compose, `--shell`, the fragment and the TOC: bare went from 1 to 0. Explicit: 0. Missing: 1. Direct check unchanged. Library test `second_checkout_note_resolves_a_bare_named_type_import`. CLI test `second_checkout_bare_named_type_import_resolves`. |
| R7-F1.6 | Root trigger selection: default compose of a parent applies the child checkout's trigger (missing `version` for `topic: review`, accepted for `other`); no child error names `parent_only`; disabled-trigger and passive checks pass | fixed | Parent compose with `topic: review` went from exit 0 to 1, naming `version` and never `parent_only`. `topic: other`: 0. Disabled triggers and refs/graph: 0 for both topics. Library test `second_checkout_note_takes_root_triggers_from_its_own_checkout`. CLI test `second_checkout_compose_applies_the_notes_trigger`. |
| R7-F1.7 | Direct document context and shell completion keep their behavior | clean | Before/after diff shows no change for direct `schema validate`, `validate refs`, `compose`, `graph --validate` and `clean`. Completion offers the enum values for the resolvable spellings only. Controls `second_checkout_note_checked_directly_resolves_a_bare_schema` and `second_checkout_note_completion_reads_its_bare_schema` pass with and without the fix. `md`'s `document_context` now shares `derive_in_own_repository`. |
| R7-F1.8 | Same-checkout resident child and outside-Git note keep their iteration 5/6 boundary behaviors | clean | The resident child resolves bare (0). The outside-Git note refuses bare (1) and gets no root triggers. The R5-F3 and iteration 6 library and CLI tests pass with and without the fix. Unit test `admitted_sources_discover_another_repository_once_per_request` checks one discovery per repository and none for the launch checkout. |

### Progress

- starting the work on 'R7-F1 documents opened through another checkout lose their schema roots' at 04:51:00
        - defect class: a source document admitted across a file boundary is given a context derived from the enclosing request's captured repositories, so a document in a repository the request never captured loses its own repository scope (schema roots, bare lookup, triggers)
        - delegated to one foreground `rust-developer` subagent; its log items are grouped here
        - subagent log items:
                - cause confirmed: `source_file_context` rebuilt a source's context on every call from the request context, using only the `Ordinary` or `TrustedExternal` derivation. The request knew only the launch checkout, so a note in a second checkout had no repository. That turned off bare `$schema`, bare `Name@file` imports and root triggers for it, and also left `&/` references in that note without a repository (`missing-context`)
                - repair: new `SourceDerivation::OwnRepository(Arc<FileResolutionContext>)`, decided once when a source is admitted (`source_derivation_for`)
                        - every consumer gets back the same stored context: composition, pre-flight and `--shell` discovery, graph inclusion nodes, TOC and fragment targets, `prepare_schemas`, expression contexts
                        - the graph and cache identities include it
                - per-request memo: `SourceRepositories` sits behind an `Arc` on `ComposeRequest` and is shared by `derive` and `map_options`
                        - it discovers each other repository root once, and each directory outside every repository once
                        - it starts out knowing the request's own repository status
                        - it reuses `discover_repository` and `build_with_observation`
                        - a discovery or build failure keeps the old derivation
                - one rule shared with the CLI: the new public `compose::derive_in_own_repository` derives a trusted external source from its own context and keeps the launch `@` scope. `md`'s `MdRequest::document_context` now calls it
                        - the library keeps the request-derived trusted-external derivation for sources outside Git, preserving the iteration 6 behavior
                - decision: one decision at admission. Rejected alternatives:
                        - a probe in each validator, which the review ruled out
                        - replacing the request context for the child, which would have changed request-level `request_cwd` and the containment rules
                - the optional-context guard forbids `Option<FileResolutionContext>`, so the memo returns `Option<SourceDerivation>`; no allowlist entry was added
                - sweep of every site that admits a source:
                        - fixed: `::file`/`prologue:`/`epilogue:` children through `markdown_child_options`, graph `accepted_child_options`/`inclusion_child_options`, `validate_local_path`, `resolve_transclusion_target`, the transclusion resolver, root `assemble`/`local_expression_context`
                        - clean: pre-flight `as_markdown` (clones the derivation), `nested.rs` root source, `with_source_file` roots, `schemas/detect.rs`, the slot evaluator (derives from the child's context)
                        - not affected: lifecycle executor, registration and configuration loaders (root or config files)
                        - clean: DMLS (never builds a `ComposeRequest`; keeps its own workspace contexts)
                        - Claudine compiles and its nested compositions inherit the fix. Its own top-level `derive_request_context_for_source` is a separate builder and was left unchanged
                - tests:
                        - 8 library tests in `lib/tests/l1/reference_bare_schema_roots.rs`, using the `TwoCheckouts` fixture with a `.git` written directly and no `git` process
                        - 14 CLI tests in `cli/tests/l1/reference_bare_schema.rs`, generated per row with `macro_rules!`
                        - 1 unit test, `admitted_sources_discover_another_repository_once_per_request`
                        - slowest 0.32 s
                        - revert check: with the own-repository branch short-circuited, all 21 tests that prove the fix fail, and the 2 direct-open controls plus all R5-F3 and iteration 6 tests pass
                - docs and skills: `docs/topics/schemas/definition.md`, `docs/cli/validate.md`, `docs/cli/graph.md`, `docs/topics/compose-requests.md` (admission rule with a Mermaid diagram), `docs/topics/file-referencing.md` (`&` and `@` in a second-checkout document), `.claude/skills/darkmatter/{SKILL.md,requests.md,testing.md}`
                        - the skill now names source admission as the one place a request discovers another repository
                - by-hand reproduction with an isolated `HOME`, two checkouts and `md` built before and after the change, covering every review row (outputs in `/tmp/r7f1/before.txt` and `/tmp/r7f1/after.txt`)
        - orchestrator review: inspected the `request.rs`/`options.rs` diff, the memo's key and invalidation, and the shared derivation. The caller skills make no claim about this boundary, so nothing needs publishing
- work completed for 'R7-F1 documents opened through another checkout lose their schema roots' at 05:40:32

### Completion Report

- **outcome: complete** (elapsed 2026-11-25T04:50:10+00:00 to 05:41 UTC, about 51 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 8 = fixed 6 (R7-F1.1–.6) + clean 2 (R7-F1.7, R7-F1.8)
- open findings: none
- shared rule:
        - an admitted source in a sibling checkout uses that checkout's Git repository through `ComposeRequest`'s `SourceRepositories` and `SourceDerivation::OwnRepository`. A nested checkout can still be treated as part of the enclosing repository
        - a directly opened `md` document in a sibling checkout also uses `derive_in_own_repository`
        - bare lookup and trigger discovery read `source_file_resolution_context().repository_root()`. It names the source's own checkout in the sibling fixture and can name an enclosing checkout in the nested case
        - a source outside Git keeps the request-derived trusted-external derivation, so the outside-Git boundary from iteration 6 legitimately differs: it has no repository and therefore no roots
- earlier cases in the same class rerun: the R5-F3 in-repository and R6-F1 outside-Git library and CLI tests pass alongside the new probes
- cross-OS:
        - Windows passed: `just cross-check darkmatter --os windows …` (80 of 80 focused tests) and `just cross-check darkmatter-cli --os windows …` (52 of 52), filtered to `reference_bare_schema`, `context::request::tests`, `external`, `cross_repo` and `entry_point_parity`
        - macOS did not run: the attempt on `bolt` stopped before testing with `just: command not found` on the remote host. That is a rig problem, not a test failure, and CI runs macOS on pull requests
- unrelated working-tree changes not made by this cycle: `claudine/features/2026-11-18-ergonomic-sequences/{plan,spec}.md`, `prompts/fix.yaml`
- changed files (all for R7-F1):
        - source: `darkmatter/lib/src/markdown/compose/context/{options,request}.rs`, `darkmatter/lib/src/markdown/compose/{mod.rs,pipeline/mod.rs,preflight/collect.rs,transclusion/resolver.rs}`, `darkmatter/lib/src/markdown/reference/mod.rs`, `darkmatter/cli/src/request.rs`
        - tests: `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs`, `darkmatter/cli/tests/l1/reference_bare_schema.rs`, plus the unit test in `request.rs`
        - docs and skills: `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/docs/cli/{validate,graph}.md`, `darkmatter/docs/topics/{compose-requests,file-referencing}.md`, `.claude/skills/darkmatter/{SKILL,requests,testing}.md`
        - records: this log, the `review-7.md` frontmatter
- final verification (run by the orchestrator after all changes, with no subagent running):
        - `darkmatter` `just test`: 18,076 passed, 10 skipped, none at or above 5 s (R7-F1, plus regressions for every earlier finding)
        - `darkmatter` `just lint`: exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `claudine` `just test`: 13,077 passed, 16 skipped, none at or above 5 s (Claudine consumes nested composition and `derive_in_own_repository`'s module)

### Review 8 record correction (2026-11-25)

The iteration 7 completion report said every admitted source is judged against its own Git repository. All eight sibling-checkout and control rows still pass. The rule is narrower for nested Git checkouts. With the worktree-built `md`, I opened `app/vendor/review.md` from `app` and from inside `vendor`. Bare schema lookup failed from `app` and passed from `vendor`. From a separate launch checkout, including `notes/seed.md` before `notes/vendor/review.md` also made the bare lookup fail. Reversing the order restored it. With an explicit schema path the same order change switched the reported missing trigger field from `parent_only` to `version`. The completion report now limits its statement to the verified sibling checkouts. [Review 8 finding](review-8.md#r8-f1-medium-nested-git-checkouts-can-inherit-the-enclosing-checkouts-schemas)

## Implementation of Review Findings #8

> **started at:** 2026-11-25T05:49:49+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-8.md'
- this is iteration 8 of the review-to-implement cycle
- the review's `implemented` flag was already `false` before repairs started

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R8-F1 | Nested Git checkouts can inherit the enclosing checkout's schemas | fixed | All 11 listed child rows fixed or clean. The shared nearest-checkout rule replaces containment in library admission and CLI `document_context`. 36 of the new tests fail with only `lib/src` and `cli/src` reverted. |
| R8-F1.1 | Direct CLI admission and schema preparation (`schema validate`, `clean`, `compose`, `validate refs`, `graph --validate`) accept every resolvable spelling from either launch directory; missing fails | fixed | CLI `nested_checkout_{schema_validate,compose,refs,validated_graph,clean}_from_the_enclosing_checkout` plus `*_from_the_component` controls and `foreign_nested_checkout_schema_validate`: bare, relative and absolute exit 0, missing exits 1 naming `absent.yaml`. Library `nested_checkout_note_opened_directly_from_the_launch_checkout_resolves_a_bare_schema`. Orchestrator reran the bare, relative and missing cases by hand from `app` and `vendor`. |
| R8-F1.2 | Reference report and graph inclusion nodes agree for bare and explicit paths in both trees and both visit orders; missing fails | fixed | Library `nested_checkout_inclusion_tests!` (9 tests: body, prologue and epilogue in the launch tree; the foreign note alone, before and after the seed). CLI `nested_checkout_refs_of_{a_body_inclusion,a_prologue,an_epilogue}`, `nested_checkout_followed_graph`, `foreign_nested_checkout_refs_{after,before}_the_seed`. |
| R8-F1.3 | Fragment and TOC preparation generate the heading for every resolvable spelling, including after the seed; missing and static outcomes kept | fixed | Library `nested_checkout_fragment_and_toc_targets_generate_their_heading_from_a_bare_schema` (static-fragment control kept). CLI `nested_checkout_{fragment_target,toc_target_compose}`, `foreign_nested_checkout_{fragment,toc}_after_the_seed`. |
| R8-F1.4 | Composition and report-only shell discovery resolve the schema the same way regardless of visit order; disabled triggers keep bare lookup | fixed | The library macro tests run compose with triggers on and off, plus pre-flight. CLI `nested_checkout_compose_of_a_body_inclusion`, `nested_checkout_compose_without_triggers`, `nested_checkout_command_discovery`, `foreign_nested_checkout_compose_{after,before}_the_seed`, `foreign_nested_checkout_command_discovery_after_the_seed`. |
| R8-F1.5 | Named-type imports use the same component root as a standalone schema reference | fixed | Library `nested_checkout_note_resolves_a_bare_named_type_import`. CLI `nested_checkout_import_tests!` (four tests: direct, refs, compose, and after the seed). |
| R8-F1.6 | Trigger discovery and its consumers use the component's trigger (`version`), never the enclosing `parent_only`; unmatched, disabled, passive outcomes kept | fixed | Library `nested_checkout_{matching_note_takes_the,…_trigger_is_satisfied_by_version,…_other_topic_matches_no_trigger}` for the launch and foreign components (six tests, split from one for Windows speed): `topic: review` is refused for `version`, and adding `version` composes, so `parent_only` never applies. `topic: other`, disabled triggers and the passive check pass. CLI `nested_checkout_trigger_tests!` (five tests: direct compose, `schema validate`, the body inclusion, and after and before the seed) asserts `version` and never `parent_only`. |
| R8-F1.7 | Bash completion and schema detection see the component root from either launch directory | fixed | Completion (`args/path_completion.rs`) and `schema detect` (`commands/schema/detect.rs`) go through `document_context`. CLI `nested_checkout_completion_reads_the_component_schema` offers the value for the three resolvable spellings only. `nested_checkout_schema_detect_sees_the_component_root` infers `&marker.md` as `file` and `&absent.md` as `string`. |
| R8-F1.8 | Schema-aware CLI editing (`md set`) and render preparation use the same source repository as direct validation | fixed | `md set` goes through `edit_schemas` → `document_context`, covered by CLI `nested_checkout_set_from_the_{enclosing_checkout,component}`. Render: the orchestrator ran `md render … style.max-width=60` by hand from `app` and from `vendor`. Bare and relative passed and missing failed with `$schema unresolved`. `md rm` prepares no schema and is unchanged. |
| R8-F1.9 | DMLS and the iteration 7 sibling, same-checkout and outside-Git controls keep their outcomes | clean | Expected: DMLS keeps its folder-level `biscuit_file::find_git_root` lookup (`dmls/src/context.rs` `folder_key`), which the new rule wraps. The new fixtures show it recognizes a `.git` file. The R5-F3, R6-F1 and R7-F1 library and CLI tests (in-repository, outside-Git, sibling checkout) pass in the same run. |
| R8-F1.10 | Sibling found in the sweep: the reference graph and validation passes re-set the root source with `ComposeRequest::with_source_file`, which reset a root admitted into its own repository to the ordinary derivation | fixed | Re-setting the request's own source now keeps its admitted derivation; any other path stays ordinary. The existing `accepted_external_child_keeps_authoring_provenance_across_surfaces` still passes, and the nested-checkout `validate refs` and graph tests cover the repair. |
| R8-F1.11 | Memo order independence: the repository memo is keyed by exact checkout root, never by containment | fixed | Unit test `nested_checkouts_are_their_own_repository_in_either_order` (a `.git` file checkout inside the launch tree, and the seed before and after the nested note). `admitted_sources_discover_another_repository_once_per_request` now expects one discovery, because a folder outside Git or in the launch checkout needs no sniff discovery. |

### R8-F1 work

- starting the work on 'R8-F1 Nested Git checkouts can inherit the enclosing checkout's schemas' at 05:51:00
        - defect class: a source's repository identity, and so its schema roots and root triggers, was inferred from folder containment rather than Git membership; the repository memo also returned the first cached root containing a folder, so the result depended on visit order
        - sites found: library `source_derivation_for` (containment early return), `SourceRepositories::containing` (first containing root), CLI `MdRequest::document_context` (containment early return), and library `ComposeRequest::with_source_file` (dropped an admitted root's derivation, found in the sweep)
        - checked and clean: `schemas/detect.rs` and the slot evaluator take their context from the caller. `schema_validation::searches_schema_roots`, `clean.rs` and `cli/commands/schema/validate.rs` read the source's own context. Claudine's `derive_request_context_for_source` already discovers at the source's folder. DMLS `folder_key` already uses `find_git_root`. Completion, `schema detect`, `md set` and render preparation all go through `document_context`
        - intentionally unchanged: baseline loading, the slot session and configuration loading still use `launch_context`, which holds the launch scope and permission snapshot
        - shared rule: a source belongs to its nearest Git checkout, the closest folder above it with a `.git` directory or `.git` file. `compose::nearest_repository_root` wraps `biscuit_file::find_git_root`. The source keeps the request's ordinary or trusted-external derivation only when that root is the request's or launch's root (`compose::names_another_repository`, compared canonically when the spellings differ); otherwise it gets `OwnRepository` through `derive_in_own_repository`. With no root, or a failed lookup, the source keeps its previous behavior. The library memoizes the nearest root per folder and one observation per exact root; `MdRequest::document_context` applies the same rule. Outside Git there is still no repository and so no roots, which is why the iteration 6 boundary legitimately differs
        - decision: the repository is found once at the admission boundary rather than checked separately by each consumer. The alternative of probing containment in each consumer would repeat the path-exception pattern the recurrence section warns about
        - decision: `find_git_root` (gix) is used for the cheap per-folder lookup and sniff only for the expensive per-root observation
        - not covered: a `.git` file that points to no valid repository. gix skips it and walks up, so the source falls back to the enclosing root, as before
        - earlier cases in the class rerun alongside the new probes: R5-F3, R6-F1 and R7-F1 library and CLI tests pass
        - subagent scoped verification: `just test reference_bare_schema` 80/80; a wider filtered nextest run 3,327/3,327; slowest new test 1.59 s under load and at most 0.3 s alone; `just lint` exit 0
        - orchestrator hand probe with the rebuilt `md`: compose, `md set`, `schema validate` and `render … style.max-width=60` of `vendor/{bare,rel,missing}.md` from `app` and from `vendor`. Bare and relative passed everywhere; missing failed with an unresolved `$schema`. An earlier probe used a stale binary from before the rebuild and was discarded
        - Windows cross-check, attempt 1: two library failures, both from the fixtures. The unit test wrote a Windows path into Git configuration as `worktree = C:\…`, where backslashes are escapes, and the static-fragment case wrote `.display()` backslashes into a Markdown link. Fixed by dropping the `worktree` key and using the portable spellings `./vendor/review.md` and `~/notes/vendor/review.md`. CLI: 82/82
        - Windows timing: the library trigger test took 4.8 s in the parallel run and 4.0 s alone. The CLI import and trigger tests took 3.3 and 3.6 s alone with one thread. Each was split into one test per case or command with `macro_rules!`
        - Windows attempt 3 (final): `just cross-check darkmatter --os windows reference_bare_schema context::request cross_repo external entry_point_parity` 99/99, and `just cross-check darkmatter-cli --os windows reference_bare_schema document_context cross_repo external entry_point_parity` 89/89. The slowest test took 3.3 s in the parallel run
- work completed for 'R8-F1 Nested Git checkouts can inherit the enclosing checkout's schemas' at 06:45:39

### Completion Report

- **outcome: complete** (2026-11-25T05:49:49+00:00 to 06:46 UTC, about 56 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 11 = fixed 10 (R8-F1.1–.8, plus .10 and .11 found in the sweep) + clean 1 (R8-F1.9)
- open findings: none
- shared rule: CLI document preparation and library admission for the tested absolute sources use the source's nearest Git checkout. The library memoizes observations by checkout root, so visit order no longer changes the result for the tested inclusions. The outside-Git controls still have no repository or schema roots. Relative library roots, roots learned from the Markdown object, and requests reusing an enclosing repository observation can still use the wrong context. The review 9 correction below records the probes.
- earlier cases in the same class rerun alongside the new probes: R5-F3 (in-repository bare lookup), R6-F1 (outside Git) and R7-F1 (sibling checkout) library and CLI tests all pass in the full run
- unrelated failure, reported here and not hidden:
        - `darkmatter::l1 shipped_schema_corpus::review_fix_cycle_validates_markdown_with_and_without_pipeline_metadata` fails in both full runs with `the review cycle exports a root union`
        - cause: the repository-root `schemas/review-fix-cycle.yaml` was rewritten in the working tree at 06:01 UTC during this cycle, by neither the orchestrator nor its subagent. It is uncommitted and was not in the starting status. The rewrite replaces the two-arm `$schema` list (`$extends: [rfc, pipeline]` / `$extends: rfc` at `HEAD`) with a single `$extends: cycle-inputs`, while the test still asserts the two-arm union
        - this cycle left the in-flight edit in place. The corpus test still needs to follow the new inputs. The root export change also affects validation of the specification filename, as recorded in the review 9 correction below.
        - `schemas/specification.yaml` is also modified outside this cycle, as review 8 already noted
- changed files (all for R8-F1):
        - source: `darkmatter/lib/src/markdown/compose/context/{request,options}.rs`, `darkmatter/lib/src/markdown/compose/mod.rs`, `darkmatter/cli/src/request.rs`
        - tests: `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs`, `darkmatter/cli/tests/l1/reference_bare_schema.rs`, and unit tests in `request.rs`
        - docs: `darkmatter/docs/topics/{compose-requests,file-referencing,magic-paths}.md`, `darkmatter/docs/topics/schemas/definition.md`, `darkmatter/docs/cli/{graph,validate}.md`
        - skills: `.claude/skills/darkmatter/{SKILL,requests,testing}.md`
        - records: this log and the `review-8.md` frontmatter
- final verification (run by the orchestrator after the last change, with no subagent running):
        - `darkmatter` `just test --max-fail all`: 18,132 run, 18,131 passed, 1 failed (the unrelated schema test above), 10 skipped, none at or above 5 s. This covers R8-F1 and regressions for every earlier finding
        - `darkmatter` `just lint`: exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `claudine` `just test`: 13,077 passed, 16 skipped, none at or above 5 s. Claudine consumes the changed library module. It ran before the final test-only splits, which changed no source
        - Windows cross-check, attempt 3 of 3: darkmatter 99/99 and darkmatter-cli 89/89, with the filters recorded in the R8-F1 work entry
        - orchestrator hand probe with the rebuilt `md`: compose, set, `schema validate` and render with a style setter, from both the enclosing checkout and the component


### Review 9 record correction (2026-11-25)

The iteration 8 completion report said that every source repository decision used the source's nearest checkout. That holds for the CLI and for the absolute-source and inclusion cases listed in the ledger. In a public Darkmatter API probe linked against the worktree, the documented relative path `vendor/review.md` failed schema preparation while the equivalent absolute request passed. A reference request that learned the absolute path only from the loaded Markdown also failed a bare schema and used the enclosing checkout for `&marker.md` with an explicit schema. Preparing a request in the component with a captured parent observation also kept the parent root and failed to include the component marker. Preparing a fresh request in the same component rendered the marker. The ledger summary now counts all 11 enumerated cases. [Review 9 library finding](review-9.md#r9-f1-medium-library-entry-points-can-lose-the-root-documents-checkout)

The completion report treated the shared schema failure as an obsolete test. The Level 1 rerun reproduced that failure. Direct `md schema validate`, the Darkmatter API and DMLS also showed that the new export accepts an existing `plan.md` in the `spec` field. Both the committed schema and a temporary one-arm version of the new schema rejected the plan. [Review 9 shared-schema finding](review-9.md#r9-f2-high-the-shared-review-schema-accepts-a-plan-as-its-specification)

## Implementation of Review Findings #9

> **started at:** 2026-11-25T06:59:30+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-9.md'
- this is iteration 9 of the review-to-implement cycle
- the review's `implemented` flag was already `false` before repairs started

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R9-F2 | The shared review schema accepts a plan as its specification | fixed | All six child rows fixed or verified. The one-arm root union export restores enforcement, and both new tests fail with the export reverted to a mapping. |
| R9-F2.1 | Public Markdown validator and `md schema validate`: `spec.md` and `./spec.md` pass, an existing `./plan.md` fails at `/spec`, a missing file fails | fixed | L1 `review_fix_cycle_inputs_require_an_existing_specification_file`: `spec.md` and `./spec.md` are valid; `./plan.md` and a nonexistent file fail at `/spec`; a missing `spec` fails. `md schema validate`: exit 0 for the spec, exit 1 for the plan (arm diagnostic at 3:7). |
| R9-F2.2 | `md compose`, `md validate refs`, `md graph --validate` refuse the plan and accept the spec; `md clean` emits the frontmatter diagnostic | fixed | Hand probes with the rebuilt `md`. For the plan, `compose`, `validate refs` and `graph --validate` exit 1 and `clean` exits 0 with `schema.shape-repair … (at /spec)`. For the spec, all exit 0. Each command keeps its own handling of validation errors. |
| R9-F2.3 | DMLS flags the plan with `dm.schema.constraint`, keeps the spec clean | fixed | New DMLS L1 `review_cycle_schema.rs` (a real LSP session): `dm.schema.constraint` on the plan's `spec:` line and none for the spec. Passes on Linux and Windows. |
| R9-F2.4 | Corpus regression test asserts current public behavior (minimal input, optional paths, missing spec, wrong file) | fixed | The stale test is replaced and asserts the current public behavior. Optional `plan` and `implementation-log` accept absence, null, an empty string and a path, and reject a list. |
| R9-F2.5 | Schema-language controls (inline/exported mapping vs unions) keep their documented behavior | clean | No change to the language. Union enforcement is covered by `schema_cache_validity.rs` `match_on_a_direct_file_property_of_a_root_union_arm_is_enforced`. Completion-only behavior in a mapping is covered by the `convert.rs` lowering unit test `bare_file_emits_reference_format_and_drops_match_extension`. |
| R9-F2.6 | Specification schema imports (verdict, blast radius, pipeline) keep validating with located diagnostics | fixed | New L1 `specification_schema_validates_its_imported_cycle_fields`: the valid inputs pass, and a bad verdict, a mapping in place of `packages`, and a pipeline missing `name` each give a located problem. |
| R9-F1 | Library entry points can lose the root document's checkout | fixed | All nine child rows fixed or preserved (.8 and .9 found in the sweep). Relative and Markdown-supplied roots go through the one admission, and observations are reused only for the same checkout. 26 of the 34 new tests fail with the source reverted. |
| R9-F1.1 | A relative request source is resolved from the captured directory before deriving its context; relative and absolute both pass; missing names fail | fixed | `admit_source_in` makes a relative path absolute against the request directory. `source_form_tests!` relative rows pass for bare, relative and absolute schemas and imports; the missing schema fails naming `absent.yaml`. |
| R9-F1.2 | Compose, compose-with-warnings, frontmatter-hook variants and command preflight use the same document checkout for all four source forms; component trigger requires `version` | fixed | `compose_with`, `compose_with_warnings`, `compose_with_frontmatter_hook` and preflight agree across all four forms (`extended_for`, pipeline root). The `*_takes_the_component_trigger` tests refuse `topic: review` for `version`, never `parent_only`, and adding `version` composes. |
| R9-F1.3 | Transclusion graph, full graph, flattened references, validation with and without a prebuilt graph admit a root learned from Markdown; `&marker.md` selects the component marker | fixed | `build_graph_inner`, `validate_graph_contents` and `verify_graph_compatibility` admit the Markdown root through `with_root_source`. `*_graphs_reach_the_component_marker` and `*_validates_*` (fresh and with a prebuilt graph) reach `vendor/marker.md`. Reloading by a relative path keeps the admitted context. |
| R9-F1.4 | `Markdown::transclusions_with_options` reports the component marker for every source form | fixed | `transclusions_in` admits the document's source; `*_transclusions_query_the_component_marker` passes for every form. |
| R9-F1.5 | Previously admitted roots, descendants, CLI and DMLS controls keep their outcomes | fixed | Preserved. The R5-F3, R6-F1, R7-F1 and R8-F1 library and CLI tests pass (`reference_bare_schema` 128/128), as does the full darkmatter run of 18,170 tests. One unit assertion changed (decision below). |
| R9-F1.6 | `local_expression_context` / `local_expression_resolution_context` yield the component directory and checkout for relative and absolute sources | fixed | `local_expression_context_uses_the_component_for_either_source_spelling` checks both APIs. The loaded-document form does not apply because these APIs take no Markdown. |
| R9-F1.7 | Request construction reuses a captured repository observation only for the same checkout | fixed | `request_repository_for` requires the observation's root to be the request directory's nearest checkout. `a_captured_parent_observation_is_not_reused_for_a_component_request` covers fresh, component-capture and parent-capture options; all three render the component marker and keep the caller-owned anchor. |
| R9-F1.8 | Found in the sweep: `effective_schema_for`, used by the host lifecycle check, judges the schema of the document's checkout for a Markdown-only or relative source | fixed | It now roots the request and document the way composition does; `*_effective_schema_is_the_component_schema` (four tests). |
| R9-F1.9 | Found in the sweep: a schema slot session builds its repository from the request's checkout, never a caller-captured parent observation | fixed | `package()` and `package_area()` repaired (`slot_repository_describes`, `for_request_in_checkout`); `a_schema_slot_reads_the_package_layout_of_the_request_checkout` fails with the source reverted. The read grant was already correct, because it comes from the request's own configuration; `a_schema_slot_read_grant_is_the_request_checkout` locks that in, and the drifted doc comment was corrected. |
- starting the work on 'R9-F2 The shared review schema accepts a plan as its specification' at 06:59:53
        - orchestration note: the R9-F2 subagent was launched in the background by mistake, so it overlapped with the R9-F1 subagent instead of running before it; their files were disjoint (schema, corpus test and DMLS test versus library request, admission and reference code) and each verified only its own slice
        - defect class: a shared schema's root `$schema` export changed from a root union to a plain mapping, so a `file(match(...))` rule stopped rejecting files and only guided completion
        - repair: `schemas/review-fix-cycle.yaml` now exports a one-arm root union `- $extends: cycle-inputs`, with a comment explaining why; the author's revised fields and types are unchanged. The language's `match` semantics are unchanged (documented in `simplified-schema/docs/topics/language/types.md`)
        - sweep of every `.yaml` containing `match(`:
                - `schemas/review-fix-cycle.yaml`: fixed (a union at `HEAD`, a mapping in the worktree)
                - `schemas/specification.yaml`: clean; it has no `match(` and imports only named types
                - `prompts/complete.yaml`, `prompts/_fix/fix-review.yaml`, `prompts/fix.yaml`, `prompts/_implement/implement.yaml`: clean for this class; they have always exported a mapping, so nothing regressed, and `prompts/` is outside the tested surface
                - the frozen Claudine prompt fixture: clean, frozen by design
                - the Markdown inline `$schema` uses: none changed in the worktree
        - the stale corpus test is replaced by `review_fix_cycle_inputs_require_an_existing_specification_file`, and `specification_schema_validates_its_imported_cycle_fields` is new
        - new DMLS L1 test `review_cycle_schema.rs` (declared in `dmls/tests/l1/main.rs`) runs a real LSP session
        - mutation check: with the export reverted to a plain mapping, both the corpus test and the DMLS test fail
        - scoped verification: `just test shipped_schema_corpus` 29/29 (slowest 1.16 s); `just test review_cycle_schema` 1/1; `just lint` exit 0. Neither schema is part of a shipped artifact, so `just shipped-schemas --check` does not apply
- work completed for 'R9-F2 The shared review schema accepts a plan as its specification' at 07:05:00
- starting the work on 'R9-F1 Library entry points can lose the root document's checkout' at 07:01:00
        - defect class: a document's checkout (its schema roots, triggers, `&` root and local expression context) was decided by how the caller spelled or supplied the source (relative or absolute, on the request or learned later from the loaded Markdown, or a reused enclosing observation chosen by containment) rather than by the document's resolved location
        - divergences found:
                - `admit_source_in` passed a relative path to `nearest_repository_root`, which returned nothing
                - `DarkmatterSchemas` resolved against the literal relative directory of the Markdown source
                - compose, preflight and `effective_schema_for` used only the request's source, never the Markdown's
                - the graph and validators reset the root through `with_source_file` and took the ordinary (launch) derivation
                - `transclusions_in` applied the request's derivation to a different path
                - `request_repository_containing` reused an observation by containment
        - shared rule: a root source, from the request or from the Markdown, is made absolute against the request directory and goes through the one existing admission (`admit_source_in` and the `SourceRepositories` memo). A captured observation is reused only when its root is the request directory's nearest checkout (`request_repository_for`). New request helpers `with_root_source`, `rooted_at` and `root_document` feed these consumers: the pipeline root, `extended_for` (preflight and the frontmatter shell walk), `effective_schema_for`, `build_graph_inner`, `transclusions_in`, `validate_graph_contents`, `verify_graph_compatibility` and `collect_composed_heading_slugs`
        - sweep, clean sites with reasons:
                - lifecycle `shell.rs`, Claudine `composition/mod.rs`, `prepare.rs` and `sequence/mod.rs`: go through `with_context` or `local_expression_context`, so the central admission covers them
                - CLI `compose.rs`: passes an absolute path and uses the iteration 8 `document_context`
                - `schemas/detect.rs`: its contract is that the caller passes the document's own context
                - `slots/evaluator.rs`: derives from the source context it is given
                - Claudine `invocation_context.rs`: guarded by `contains_nested_repository_boundary`
                - DMLS: uses none of these APIs
        - decision: each consumer keeps its existing precedence. Compose, preflight and the effective schema prefer the request's source; the graph, the validators and the transclusion query prefer the document's. The alternative was one global precedence, which would change how existing callers' sources are selected. Either way, both inputs now go through the same admission
        - decision: the existing unit test `accepted_external_child_keeps_authoring_provenance_across_surfaces` expected the transclusion query to fail for a sibling queried as its own root. That relied on the inconsistent ordinary derivation, so it now asserts the same admission `ComposeRequest::prepare` gives that source
        - tests: 34 new tests in `reference_bare_schema_roots.rs` (`source_form_tests!`) cover the absolute, relative, absolute-document-with-relative-request and loaded-document-only forms against compose (three variants), preflight, the effective schema, the graphs, validation with and without a prebuilt graph, the transclusion query, the component trigger and the local expression context, plus the captured-observation test. 26 fail with the source reverted; the 8 that pass are the absolute controls
        - new row R9-F1.8: `effective_schema_for` ignored a source carried only by the Markdown, and a relative source
        - new row R9-F1.9: the schema slot session took its repository observation from a caller-owned parent capture, which a second subagent repaired:
                - `package()` and `package_area()` in a slot returned the parent's layout. Fixed: `SlotSession::for_request` now uses the request checkout's observation when the captured one describes another checkout (`slot_repository_describes`, `SessionObservations::for_request_in_checkout`)
                - the read grant was already correct, because it comes from the request's own configuration snapshot. The stale doc comment on `slot_repository_root` claimed otherwise and was corrected (comment drift; the code was right)
                - new tests `a_schema_slot_reads_the_package_layout_of_the_request_checkout` and `a_schema_slot_read_grant_is_the_request_checkout`. With the source reverted, the package test fails; the grant test locks in existing behavior
                - residual, intentionally unchanged: a caller-extended context with no `Repo` capture and no observation still takes `Repo` from its own context authority, as before
        - earlier cases in the class (R5-F3, R6-F1, R7-F1, R8-F1) rerun alongside the new probes: `just test reference_bare_schema` 128/128
        - scoped verification: `reference` 1424, `preflight` 221, `request` 236, `compose` 4520, `transclusion` 320, `schema` 3155, `slot` 513, `external` 38, `entry_point_parity` 79 all pass; Claudine `cargo check` passes and `just test composition` passes 2912/2912; slowest new test 0.66 s; `just lint` exit 0
        - the first subagent twice ran `--no-fail-fast` locally while iterating, against the brief; its final verification used the recipes
        - docs: `darkmatter/docs/topics/{compose-requests,permissions}.md`, `darkmatter/docs/topics/schemas/schema-slots.md`; skills: `.claude/skills/darkmatter/{requests,schema}.md`
- work completed for 'R9-F1 Library entry points can lose the root document's checkout' at 07:33:00

### Completion Report

- **outcome: complete** (2026-11-25T06:59:30+00:00 to 07:45 UTC, about 46 minutes)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: 15 = fixed 14 (R9-F2.1–.4 and .6; R9-F1.1–.9, where .8 and .9 were found in the sweep and .5 is preserved behavior verified by the earlier tests) + clean 1 (R9-F2.5)
- open findings: none
- shared rules:
        - R9-F2: a shared schema whose file rule must reject exports a root union (here, one arm), so `match(...)` enforces it. The language's documented mapping and union behavior is unchanged
        - R9-F1: source admission resolves a relative path against the request directory and uses the repository memo. The four verified forms all put the path on the Markdown, either alone or alongside a request source. The request-only form can still lose the admitted source during schema preparation and reference handling. A captured repository observation is reused only for the same nearest checkout, including by the schema slot session. Outside Git there is still no repository, so the iteration 6 boundary legitimately differs
- earlier cases in the same class rerun alongside the new probes: R5-F3, R6-F1, R7-F1 and R8-F1 library and CLI tests all pass in the full run
- decisions (recorded in the ledger only; none needs an author ruling): per-consumer source precedence kept; the transclusion query for a root outside the request tree now gets the same admission `ComposeRequest::prepare` gives it, and its old unit assertion was updated; slot `Repo` is captured lazily from the request's checkout when the caller's observation describes another checkout
- changed files:
        - R9-F2: `schemas/review-fix-cycle.yaml`, `darkmatter/lib/tests/l1/shipped_schema_corpus.rs`, `darkmatter/dmls/tests/l1/{review_cycle_schema.rs (new),main.rs}`
        - R9-F1 source: `darkmatter/lib/src/markdown/compose/context/{options,request}.rs`, `compose/context/capture/observations.rs`, `compose/pipeline/mod.rs`, `compose/schema_validation.rs`, `reference/{graph,mod,validate}.rs`, `schemas/slots/{observations,session}.rs`
        - R9-F1 tests: `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs`, plus the unit test in `reference/graph.rs`
        - R9-F1 docs and skills: `darkmatter/docs/topics/{compose-requests,permissions}.md`, `darkmatter/docs/topics/schemas/schema-slots.md`, `.claude/skills/darkmatter/{requests,schema}.md`
        - records: this log and the `review-9.md` frontmatter
        - `schemas/specification.yaml` and the field rewrite in `schemas/review-fix-cycle.yaml` are the author's in-flight edits; this cycle changed only the export shape and its comment
- final verification (run by the orchestrator after the last change, with no subagent running):
        - `darkmatter` `just test --max-fail all`: 18,170 run, 18,170 passed, 10 skipped; `L1 run recorded: 18170 test(s), none at or above 5.0s` (R9-F1, R9-F2 and regressions for every earlier finding)
        - `darkmatter` `just lint`: exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `claudine` `just test --max-fail all`: 13,077 passed, 16 skipped, none at or above 5 s (Claudine consumes the changed request and admission code)
        - Windows cross-check, attempt 1, passed: `just cross-check darkmatter --os windows reference_bare_schema slot context::request external entry_point_parity shipped_schema_corpus` 548/548, slowest 3.57 s; `just cross-check darkmatter-lsp --os windows review_cycle_schema` 1/1
        - R9-F2 hand probes with the rebuilt `md` are recorded in the ledger

### Review 10 record correction (2026-11-25)

The iteration 9 completion report generalized source admission to the behavior of files supplied solely by the request. Its four tested forms all put the path on the Markdown. A program linked to the current Darkmatter library confirmed those ledger cases, then supplied the same component path only through `ComposeOptions::with_source_file`. Composition and preflight rejected a valid bare schema and skipped a matching trigger. The reference graph labeled the root `<stdin>`, and reference validation reported `MissingSourceContext` for an existing relative link. Attaching the same path to the Markdown restored the expected results. Separate direct calls showed that relative same-checkout prologue and epilogue targets still fail in the local query, outside the ledger's body-target probes. [Review 10 finding](review-10.md#r10-f1-medium-library-consumers-can-ignore-the-admitted-document-source)

## Implementation of Review Findings #10

> **started at:** 2026-11-25T16:14:39+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-10.md'
- this is iteration 10 of the review-to-implement cycle
- review `implemented` was already `false`; no human-in-the-loop items and no blocked findings

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R10-F2 | Reference validation drops malformed transclusions | partially fixed | The original malformed shapes are refused. Review 11 reproduced generic codes for some recognized directive failures and a suppressed TOC fallback reported as missing. Full Darkmatter L1 18,207 passed. |
| R10-F2.1 | Body `::file`/`::code`/`::url` without target, both graph modes: propagate line-specific parse error; valid shapes preserved | fixed | `parsed_inclusions` forwards `TransclusionError::ParseDirective` (`dm.directive.parse_failure`, line-specific) from both graph modes. `reference_malformed_inclusions::body_directive_without_target_is_refused_by_every_reader` checks both graphs, local query, flattening, validation and compose, and that quoted and unquoted targets give the same graph. |
| R10-F2.2 | `::toc-linking level=h9`, `::file-links --depth two`, both graph modes: propagate invalid-level/depth error | fixed | `MarkdownError::TocLinking`/`FileLinks` with the document named, now coded `dm.toc_linking.invalid_level` / `dm.file_links.invalid_depth`. `invalid_toc_level_and_file_links_depth_are_refused`. |
| R10-F2.3 | `prologue`/`epilogue` invalid shapes (blank, null, number, object, mixed, all-invalid list) error; omitted and `[]` stay no-ops | fixed | `frontmatter_inclusion_shapes`: absent and `[]` are no-ops; scalar/list controls give child edges; blank key, null, number, object, mixed and all-invalid lists refused by every reader and compose, at the key's authored line. Decision: null is an error because composition rejects it. |
| R10-F2.4 | `transclusions_with_options` reports parser errors for TOC, file-list, prologue, epilogue | fixed | `transclusions_with_options` returns TOC, file-list and frontmatter parse errors (frontmatter as `ReferenceError::ParseDirective`, now `dm.directive.parse_failure`). |
| R10-F2.5 | Flattened references, fresh and prebuilt validation, `md validate refs`, `md graph --validate` fail on malformed input | fixed | Flattening and fresh/prebuilt validation fail through the graph. CLI `validate_refs::reference_commands_refuse_a_malformed_inclusion` and `compose_and_reference_commands_report_inclusion_failures_alike`: `md validate refs` and `md graph --validate` exit 1 with the code, file and line; control exits 0. |
| R10-F2.6 | Composition, command discovery, editor scanner keep existing boundaries | clean | Composition unchanged via crate-private `defer_inclusion_parse_failures` set only by `for_reference_check` (`reference_check_ahead_of_composition_defers_inclusion_parse_failures`; the two CLI compose tests that regressed without it pass). Command discovery already uses `?` (`preflight/collect.rs`). The DMLS scanner does not call the directive parsers. |
| R10-F2.7 | `has_transclusions` documents successful-parse requirement | fixed | `has_transclusions` doc states the successful-parse requirement; `has_transclusions_reports_only_parsed_inclusions`. |
| R10-F2.8 | Sibling: child that exists but cannot be loaded is dropped (`load_markdown(..).ok()`) | fixed | `load_inclusion` at all four child sites fails with the typed error naming the child; deferred under `for_reference_check`; `an_included_file_that_cannot_be_loaded_is_refused` |
| R10-F2.9 | Sibling: `path#fragment` into an unloadable file passes fragment validation | fixed | New `UnreadableTarget` error issue; `a_fragment_into_a_file_that_cannot_be_loaded_is_reported` |
| R10-F2.10 | Sibling: failing `when=` read as false | fixed | Graph raises composition's `ConditionEval`; deferred under the reference check; unset values stay falsy; `a_failing_when_condition_is_refused_unless_composition_owns_it` |
| R10-F2.11 | Sibling: TOC/file-list failures under catch-all `dm.compose.failure` | partially fixed | The typed error projection has specific codes. Tolerated invalid filters/globs and wrong-directory failures still surface as `dm.compose.warning` in real composition. The graph command also loses typed reference failures at its FileTree boundary. |
| R10-F2.12 | Sibling: frontmatter inclusion errors located at line 0 | fixed | Key line from `Markdown::frontmatter_error_context()`; `an_invalid_frontmatter_inclusion_is_located_at_its_key` |
| R10-F2.13 | Sibling: depth refusal dropped by `enter(..).is_ok()`, and refused node leaked on the tracker stack | fixed | `DependencyTracker::enter` refuses before pushing; `enter_inclusion` fails the graph with `MaxDepthExceeded` (deferred under the reference check); `a_chain_past_the_depth_limit_is_refused_but_a_cycle_is_a_graph`; `reference_integration::depth_limit_respected` updated from silent truncation to refusal |
| R10-F2.14 | Sibling: transclusion cycle | clean | Every document on a cycle is a graph node and is inspected, so no content goes unchecked; the graph keeps drawing it (pinned by `cycle_detection`, `cycle_composed_references_are_finite` and the new cycle control); documented in `docs/cli/graph.md` and `validate.md` |
| R10-F2.15 | Swept sites: `call_precheck.rs` `if let Ok`, prefetch `unwrap_or_default`, `resolve_toc_linking_target(..).ok()`, `css.rs`/`html.rs`/`validate.rs` fallbacks | partially fixed | Passive precheck and prefetch have an authoritative parser downstream. Graph TOC target selection collapses a suppressed fallback with an unresolved target, emitting a missing reference in both cases. The CSS/HTML tokenizers operate on link text. |
| R10-F1 | Library consumers can ignore the admitted document source | fixed | The seven review rows and two sweep rows use the admitted source or retain their existing source contract. With the fallback disabled, all 20 request-only and local-query tests fail. Full darkmatter L1 and Claudine L1 pass. |
| R10-F1.1 | Schema prep in all three compose APIs uses admitted request-only source for bare lookup and trigger eligibility | fixed | Three compose APIs start from `admitted_document`. `request_only_{relative,absolute}_source_composes_in_the_component` (bare/explicit/missing schema and import) and `..._takes_the_component_trigger` (missing `version` refused, supplied accepted, unmatched topic accepted). |
| R10-F1.2 | `compose_preflight`, approval entry and both command collectors apply the same schema/trigger checks; frontmatter-only scanner unchanged | fixed | `collect_effects`, `compose_preflight_approvals_with` and `approve_slot_reads` admit the root; the preflight helper asserts `compose_preflight`, the approval entry and both collectors agree with compose. Frontmatter-only scanner unchanged. |
| R10-F1.3 | `effective_schema_for` returns component schema/trigger for request-only source; unresolved names still none | fixed | `effective_schema_for` uses `admitted_document`; `..._effective_schema_is_the_component_schema` and trigger rows (component `version` declared only for a matching note). |
| R10-F1.4 | Both graph builders root at admitted path; flattened origins carry it; component inclusion kept | fixed | `build_graph_inner` admits first; `*_roots_the_graph_and_its_links_at_the_note` (5 forms) assert root id and flattened origins name the admitted note, component inclusion kept. |
| R10-F1.5 | Fresh and prebuilt reference validation resolve links from admitted source; only missing heading reported | fixed | `validate` and `validate_with_graph` admit; the same tests assert the only issue is `MissingFragmentTarget` for `./side.md#gone`, fresh and prebuilt, bare and explicit schema. |
| R10-F1.6 | Local query body `::file`/`::code`, `prologue`, `epilogue` resolve from admitted path (request-only and relative same-checkout); missing stays no-match | fixed | Local query takes source, resolution base, origins and error documents from the admitted document for all four forms; two tests cover `&marker.md`, `./side.md` and a missing target across five source forms in `app/vendor` and same-checkout `app/docs`. |
| R10-F1.7 | File-backed CLI/DMLS and local expression contexts stay clean | clean | CLI opens files with a source attached; DMLS calls none of the changed entry points; `local_expression_context_uses_the_component_for_either_source_spelling` and the CLI `reference_bare_schema` runs pass. |
| R10-F1.8 | Sibling: `approve_slot_reads` prepared schemas from the raw document and an unrooted request | fixed | Admits and roots before `prepare_schemas`; covered by the preflight agreement rows |
| R10-F1.9 | Swept sites: `as_markdown` pre-flight content, internal `run_root_pipeline_collecting`, Claudine `document_for_mode`, `link_resolve`, `normalize_links`, `unknown_identifiers`, `call_precheck`, capture, `frontmatter_shell_commands`, slot `for_request`, `detect_from_document`, render-tree imports | clean | Either not the file's text (`as_markdown`, context-identity text), already reading the request's admitted `options.source` or rooted request, or taking no request |

### Progress

- starting the work on 'R10-F2 Reference validation drops malformed transclusions' at 16:15:00
        - defect class: a reference reader treats a parser error from an authored inclusion as absence of that inclusion
        - pass 1 (subagent): both graph modes (`parsed_inclusions` helper in `reference/graph.rs`) and `transclusions_with_options` now forward the shared parsers' errors (body directive, `::toc-linking`, `::file-links`, `prologue`/`epilogue`) with composition's typed errors; `md validate refs` and `md graph [--validate]` exit 1 where they used to report clean
        - added crate-private `ComposeOptions::defer_inclusion_parse_failures`, set only by `for_reference_check`, so `md compose`'s own reference check leaves these failures to composition (which rejects a root's and tolerates a transcluded child's); without it two existing CLI tests regressed
        - decision: blank/`null` `prologue`/`epilogue` is an error because composition already rejects it; omitted and `[]` stay no-ops (alternative: no-op for null, rejected because it contradicts composition)
        - `has_transclusions` documented as counting only inclusions that parse
        - tests: `lib/tests/l1/reference_malformed_inclusions.rs` (6), `cli/tests/l1/validate_refs.rs::reference_commands_refuse_a_malformed_inclusion`; a temporary always-defer mutation made 5 of 6 fail
        - pass 1 sweep left four sibling sites open; sent to a second pass rather than deferred: child document that exists but fails to load (`load_markdown(..).ok()`), `when=` failure read as false, TOC/file-list failures reported under catch-all `dm.compose.failure`, frontmatter inclusion errors located at "line 0"
        - pass 2 (subagent) closed the four open siblings:
                - a child that exists but cannot be loaded (I/O or frontmatter parse) fails both graph modes and validation with the typed error naming the child (`load_inclusion` replaces `load_markdown(..).ok()` at all four child sites); a file that vanished after resolving stays a `MissingLocalTarget` issue. `md compose` preflight refuses a `::file` child with invalid YAML. A TOC target uses raw headings in composition, so that command can succeed while the graph refuses the invalid YAML.
                - a `path#fragment` link into a file that cannot be loaded is a new `UnreadableTarget` error issue (`dm.reference.failure`), never deferred, because composition never reads link targets
                - a failing `when=` now raises composition's `TransclusionError::ConditionEval` through the same `ConditionSite` and remote runtime; deferred under `for_reference_check`; unset/missing values stay falsy
                - TOC and file-list failures no longer use catch-all `dm.compose.failure`: `dm.toc_linking.invalid_level|invalid_cleanup|invalid_filter`, `dm.file_links.invalid_depth|missing_source|not_a_directory|invalid_glob` (new typed `FileLinksError::InvalidDepth`), plus `dm.directive.parse_failure`, `dm.reference.failure`, `dm.input.read_failure` for their syntax, missing-target and I/O variants; the local query's `ReferenceError::ParseDirective` moved from `dm.reference.failure` to `dm.directive.parse_failure`; codes listed in `docs/cli/data-contracts.md`, handling example in `docs/errors/composition-problems.md`
                - an invalid `prologue`/`epilogue` reports the authored key line (`Markdown::frontmatter_error_context()`); an unknown line is never shown as 0
                - decision: per-option codes rather than one family code with detail (a detail payload would change all 17 output schemas)
        - orchestrator: the sweep also found the transclusion depth refusal dropped by `enter(..).is_ok()` at the four graph child sites, and `DependencyTracker::enter` pushed the refused node before returning `MaxDepthExceeded`, so a recovering caller leaked a stack entry; `enter` now refuses before pushing, and the graph's `enter_inclusion` fails with the depth error (deferred under `for_reference_check`); `reference_integration::depth_limit_respected` pinned the silent truncation and now asserts the refusal; new test `a_chain_past_the_depth_limit_is_refused_but_a_cycle_is_a_graph`
        - decision: a transclusion cycle stays a finite graph (every document on it is a node and is inspected), unlike composition, which refuses it; documented in `docs/cli/graph.md` and `docs/cli/validate.md`
        - focused run: `cargo nextest run -p darkmatter --lib --test l1 -E 'test(/reference|graph|transclusion|depth|cycle|toc_linking|file_links/)'` 2159 passed; CLI focused run (subagent) 1039 passed, slowest 1.085 s
- work completed for 'R10-F2 Reference validation drops malformed transclusions' at 16:51:46
- starting the work on 'R10-F1 Library consumers can ignore the admitted document source' at 16:51:46
        - defect class: a consumer uses the raw or absent Markdown path to decide file identity after the request has already admitted the document's file
        - shared rule: new `ComposeRequest::admitted_document` (the document's own source, made absolute by the existing `root_document`; otherwise the request's admitted file path attached without re-reading the buffer; text carrying a `with_context_identity` excluded), called once at every public root entry point: `compose_with`, `compose_with_warnings`, `compose_with_frontmatter_hook`, `collect_effects` (preflight and both command collectors), the approval and slot-approval entries, `effective_schema_for`, both graph builders, fresh and prebuilt validation, and the local query
        - the local query's `prologue`/`epilogue` loops now resolve from the same admitted source as the body loop; origins and error documents name the admitted file
        - sweep found one more site: `approve_slot_reads` prepared schemas from the raw document and an unrooted request; now admits and roots first
        - not applied inside the pipeline: `as_markdown` content composed against the root source is not the file, and composition gives it no source
        - decision: `directive_error_document::toc_linking_and_file_links_failures_name_a_context_identity` expected a source-less buffer under a request file to name no document; that expectation encoded the defect and now expects the request's file (generated text with an identity is unchanged)
        - decision: local query origins use the absolute admitted path however the path was supplied (alternative: keep the caller's relative spelling, rejected because it makes identity depend on the spelling)
        - tests: 16 matrix rows plus 5 root/link and 2 local-query tests in `reference_bare_schema_roots.rs` (request-only relative and absolute, `app/vendor` and same-checkout `app/docs`); with the fallback disabled all 20 request-only and local-query tests fail and the 3 Markdown-source controls pass
        - docs: `docs/topics/compose-requests.md` (request-only form and example), repo skill `.claude/skills/darkmatter/requests.md`
        - focused runs (subagent): `reference_bare_schema_roots` 96/96; wider darkmatter filter 8230/8230; CLI `reference_bare_schema|validate_refs|graph|entry_point_parity|stdin|compose` 859/859; claudine `composition|sequence|lifecycle|prepare` 2857/2857
- work completed for 'R10-F1 Library consumers can ignore the admitted document source' at 17:06:32

### Completion Report

- implementation run completed at 17:22:30 UTC (started at 16:14:39 UTC, about 67 minutes)
- repair status at review 11 verification: R10-F1's ledger instances verified and R10-F2 partially fixed
        - R10-F2 (high): the original shapes are refused; the code projection in .11 and the TOC resolution in .15 have remaining defects
        - R10-F1 (medium): fixed; children .1–.6 fixed, .7 clean; sweep row .8 fixed, .9 clean
- shared rules after this cycle:
        - inclusion failures: both graph modes, flattening and validation propagate inclusion parser errors. They also inspect descendants and evaluate conditions. The local query checks authored syntax and resolves supported paths without opening or following descendants or evaluating conditions. `ComposeOptions::for_reference_check` defers parser, load, condition and depth failures to composition.
        - file identity: `ComposeRequest::admitted_document` supplies a common file identity for the ledger paths. FileTree builds its graph with that identity, then can lose it during root heading validation.
- recurrence: R10-F1 is the sibling of R9-F1 named in the review; both sites named there (request-only source at schema eligibility, graph attribution, validation, local query; the local query's frontmatter loops) are fixed, and the iteration 9 four-form suite still passes alongside the new request-only rows
- no human-review items: every decision is recorded in the ledger and progress notes and follows existing composition behavior
- open findings and newly found siblings are recorded in [review 11](review-11.md)
- caller skill `darkmatter/docs/use-skills/use-darkmatter/library.md` changed at its source; `just publish-skills` installs outside the worktree and was not run
- changed files:
        - library: `darkmatter/lib/src/markdown/` `reference/{graph,mod,validate,errors}.rs`, `reference/file_tree/model.rs`, `compose/{mod,schema_validation,report_diagnostics}.rs`, `compose/context/{options,request}.rs`, `compose/transclusion/{mod,parser,types}.rs`, `compose/file_links/{parser,types}.rs`, `compose/pipeline/phases.rs`, `compose/preflight/{collect,lifecycle}.rs`, `mod.rs` (R10-F1, R10-F2)
        - tests: new `lib/tests/l1/reference_malformed_inclusions.rs` (declared in `main.rs`); `reference_bare_schema_roots.rs`, `directive_error_document.rs`, `diagnostic_source_provenance.rs`, `reference_report.rs`, `reference_integration.rs`; `cli/tests/l1/validate_refs.rs`
        - docs and skills: `darkmatter/docs/cli/{validate,graph,data-contracts}.md`, `docs/errors/composition-problems.md`, `docs/inline/link-validation.md`, `docs/structs/Markdown.md`, `docs/topics/compose-requests.md`, `docs/use-skills/use-darkmatter/library.md`, `.claude/skills/darkmatter/{SKILL,requests}.md`
- final verification (orchestrator, after the last change, no subagent running):
        - `darkmatter` `just test --max-fail all`: 18,207 passed, 10 skipped; `L1 run recorded: 18207 test(s), none at or above 5.0s` (R10-F1, R10-F2, and earlier findings' regressions)
        - `darkmatter` `just lint`: exit 0
        - `just check-tier-coverage darkmatter`: 0 stranded
        - `darkmatter` `just shipped-schemas --check`: exit 0
        - `claudine` `just test --max-fail all`: 13,077 passed, 16 skipped, none at or above 5 s (Claudine consumes the changed composition and request code)
        - Windows cross-check attempt 1 passed: `just cross-check darkmatter --os windows reference_bare_schema reference_malformed_inclusions validate_refs directive_error_document reference_integration context::request entry_point_parity` 213/213; `just cross-check darkmatter-cli --os windows validate_refs reference_bare_schema graph` 120/120; none at or above 5 s


### Review 11 record corrections (2026-11-25)

- The completion report treated the local transclusion query as if it loaded descendants and evaluated their conditions. A public Rust API probe showed that it returns one raw inclusion for a syntactically valid directive with a failing condition. The graph and validators evaluate that condition and refuse it. The completion report now states the boundary between the two operations.
- The child-load note generalized preflight's behavior from a `::file` child to all included targets. With the same invalid-YAML file, the built `md compose --output json` returned `dm.frontmatter.yaml_parse` for `::file` and exited 0 for `::toc-linking`. The graph and reference validators refused both. The note now distinguishes the two cases.
- The ledger claimed all TOC and file-list failures kept specific codes and the TOC target selector preserved required reference outcomes. The built CLI emitted `dm.compose.warning` for a malformed TOC filter and file-list glob and `dm.compose.failure` for a typed invalid reference in `md graph`. A TOC chain ending in `false` with no existing target incorrectly failed reference validation. The affected ledger rows now record the verified limits.
- The file-identity summary said every root entry point used the admitted document. In a public Rust API probe a FileTree with fragment validation enabled reported `#ready` missing with a request-only file path and a bare schema, while fresh and prebuilt reference validation accepted the same note. The summary now identifies the remaining FileTree path.
- The body parse checks used a fixture without a YAML header and asserted the line printed in the message. With a four-line header, `md graph` and `md validate refs` both report line 3 for the directive on line 7. A false page block before the directive produces a different body-relative line. The frontmatter key-location checks still pass.

## Implementation of Review Findings #11

> **started at:** 2026-11-25T17:37:04+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-11.md'
- this is iteration 11 of the review-to-implement cycle
- review `implemented` was already `false`; the review has no human-in-the-loop items and no blocked findings

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R10-F2 | Reference validation drops malformed transclusions (typed-code projection remainder) | fixed | Every child below is fixed and verified. Also fixed: siblings for transclusion `Io` errors, normalization errors, heading overflow, a remote child's frontmatter, and shell-block preflight (rows R11-S*). Full Darkmatter L1 18,246/18,246 passed. |
| R10-F2.11 | Keep typed projection; add a real command assertion for each variant that could be bypassed | fixed | Projection unchanged. The CLI test `validate_refs::commands_keep_specific_codes_for_tolerated_failures_conditions_and_references` asserts each variant that could be bypassed, in compose, `validate refs`, `graph` and `graph --validate`. |
| R10-F2.16 | TOC filter / file-list glob / directory / missing-source runtime warnings keep specific codes, same severity and recovery; reader example explains tolerated failures | fixed | Tolerated TOC/file-list failures keep `dm.toc_linking.*`, `dm.file_links.*`, `dm.reference.failure` and `dm.input.read_failure` at severity warning, exit 0. The codes are registered in `COMPOSE_DIAGNOSTIC_CODES` (`--deny <code>` tested). Tests: `typed_diagnostic_codes::tolerated_toc_linking_and_file_links_failures_keep_their_codes`, `a_tolerated_file_links_glob_without_a_source_keeps_its_code`. The reader example in `composition-problems.md` is updated. An unreadable directory was checked by a chmod probe only. |
| R10-F2.17 | Malformed `when=` condition uses the expression parse code in compose, both graph modes, flattening; evaluation code and local-query contract kept | fixed | `ConditionParse` gives `dm.expression.parse_failure` at the authored line in compose, both graphs, flattening and validation. `1 / 0` still gives `dm.expression.evaluation_failure` and the local query is unchanged. Test: `typed_diagnostic_codes::a_malformed_when_condition_is_an_expression_parse_failure_from_every_reader`. |
| R10-F2.18 | `md graph` (with and without `--validate`) forwards typed reference cause (`dm.reference.failure`), not `dm.compose.failure` | fixed | `error_diagnostics` now recognizes a `ReferenceError` in the error chain, and `md graph` routes `FileTreeError` through it. For `::file @` and `prologue: "@"`, `graph`, `graph --validate` and `validate refs` all report `dm.reference.failure`. Test: `typed_diagnostic_codes::a_file_tree_reports_a_typed_reference_failure_under_its_code`. |
| R11-F1 | Reference checks misclassify inline and suppressed inclusions | fixed | Every child below is fixed and verified. Prologue/epilogue now share one classifier (`frontmatter_value_is_inline`), and the TOC chain keeps its three outcomes (`toc_suppressed`). The new module `reference_inline_and_suppressed_inclusions` fails with the change reverted. |
| R11-F1.1 | Prologue/epilogue use the composition frontmatter classifier in both graph modes and local query; inline values are not resolved as files | fixed | `inline_frontmatter_text_is_not_a_file_reference` covers both keys × prose, link, block scalar and empty, each as a scalar and as a list. Both graph modes record an `Inline` target and the local query gives `resolved_target: None`. Control: `a_frontmatter_path_is_still_a_file_reference`. |
| R11-F1.2 | Flattening, fresh/prebuilt validation, FileTree, CLI validators and compose reference check: inline links validated as links, placeholders give no missing-file verdict | fixed | Flattening, fresh, prebuilt, the compose reference check and FileTree raise no issue for valid inline text. A missing link gives `(MissingLocalTarget, Hyperlink)` in all of them. Inline text is now read with the body extractor (`extract_all_references`), so images are checked too (`an_image_inside_inline_frontmatter_text_is_checked_as_an_image`). CLI: `reference_commands_accept_inline_frontmatter_and_suppressed_toc_chains`. |
| R11-F1.3 | TOC fallback chain: suppression distinct from failure through graph and validation; required-missing and selected fallback preserved; local query lists target | fixed | `toc_linking_chain_outcomes_match_composition`: an existing first target and a fallback after a missing path are selected. All missing plus `false` is suppressed: no issue, compose not blocked. All missing without `false` is a required missing target. The local query still lists the target. |
| R11-F1.4 | Controls: `::file`/`::code` required/missing, `when="false"`, URL, file-list query, required missing TOC, preflight selector, editor scanner | clean | `neighboring_inclusion_selectors_keep_their_decisions` covers present and missing `::file`/`::code`, `when="false"`, a quoted URL (never fetched), glob and `--dir` file-lists. The existing DMLS suppression control is in `dmls/src/graph/substrate.rs`. Sibling DMLS `frontmatter_prologue_epilogue` was fixed and is covered by `inline_prologue_text_is_not_a_document_link`. |
| R11-F2 | Inclusion diagnostics point to body-relative lines | fixed | Every child below is fixed and verified. `DirectiveSite` maps a body line to the authored opener through the body map conditions use, and errors carry `authored`. `directive_authored_lines` (14 tests) fails when placement is made a no-op. |
| R11-F2.1 | Body `::file`/`::code`/`::url` parse errors name the authored line (with YAML header) in graph modes, local query, flattening, validation, structured location | fixed | After a 2-key header, `::file`/`::code`/`::url` with no target report line 7, column 1 in the message and `source.line` from every reader. CLI JSON for compose, graph and validate refs: `validate_refs::directive_parse_failures_report_their_authored_line`. |
| R11-F2.2 | TOC parser errors name the authored line across compose, command discovery, graphs, local query, validators | fixed | TOC `level=h9`, `levels=x` and `cleanup=bogus` give the authored line in every reader, including command discovery. A tolerated bad `filter=` warns at the authored line. |
| R11-F2.3 | File-list parser errors name the authored line across compose, graphs, local query, validators; discovery boundary kept | fixed | `--depth two` gives the authored line. Command discovery still does not parse file-list options. |
| R11-F2.4 | Prepared-body (false page block removed) vs raw-body paths: authored opener used, or file only when unknown | fixed | With a false page block before the directive, every reader reports the authored line (`a_removed_page_block_does_not_move_the_reported_line`). The raw local query uses the loaded body map. When the position is unproven, the error names the file with no line (`an_unproven_directive_names_its_file_without_a_line`), and status blocks drop the `Line:` row. |
| R11-F2.5 | Controls: frontmatter key line, condition eval/parse locations, editor scanner | clean | `prologue: null` keeps key line 3. `when="1 / 0"`/`"1 +"` keep the authored line with and without a block, and valid spellings report nothing (`valid_spellings_report_nothing`). The DMLS raw scanner is untouched. |
| R11-F3 | FileTree loses the admitted source when checking root headings | fixed | Every child below is fixed and verified. `validate_graph_contents` admits the root source before root-heading preparation, which covers FileTree and every other engine caller. |
| R11-F3.1 | FileTree with request-only (relative/absolute) sources and bare schema accepts `#ready`, reports `#gone`, in all four forms | fixed | `reference_root_heading_admission`: 4 source forms, with both the bare and the explicit schema. FileTree accepts `#ready` and gives one `MissingFragmentTarget` for `#gone`. It failed before the fix at the request-only relative form. |
| R11-F3.2 | Direct and prebuilt validation verdicts and origins unchanged | clean | The same matrix passes for fresh and prebuilt validation. A second admission returns the document borrowed, so the existing request-only, origin and bare-schema tests are unchanged. |
| R11-F3.3 | File-backed CLI and neighboring library consumers stay consistent | clean | CLI `validate_refs`/`graph` 74/74 passed. Descendant headings still come from the graph's prepared records. |
| R11-S1 | Sibling (R10-F2 class): transclusion `Io` errors map to the catch-all `dm.transclusion.failure` | fixed | No match now gives `dm.reference.missing_transclusion` and other I/O gives `dm.input.read_failure`. Test: `typed_diagnostic_codes::a_transclusion_target_that_matches_no_file_is_a_missing_transclusion`. |
| R11-S2 | Sibling (R10-F2 class): `NormalizationError` wrapped as `Transform`; heading-overflow warning uncoded | fixed | `MarkdownError::Normalization`, `dm.normalization.level_overflow`/`validation_failed`, `dm.transclusion.heading_overflow`, all registered and documented. Tests in `typed_diagnostic_codes` and `phases.rs`; the oracle test `finding_35_2` was updated for the code. |
| R11-S3 | Sibling (R10-F2 class): fetched remote child's frontmatter error wrapped as `Transform` | fixed | `Markdown::try_from_named_content` keeps `dm.frontmatter.yaml_parse` and names the URL. Test: `network_permission::a_remote_child_with_malformed_frontmatter_keeps_its_parse_code`. |
| R11-S4 | Sibling (R10-F2 class): preflight shell-block tokenize error and remote-child fetch wrapped as `Transform` | fixed | The typed `ShellBlockError` now gives `dm.shell.failure` at the file line, with no "line 0". A remote fetch gives `TransclusionError::RemoteFetchFailed`. Tests: `directive_authored_lines::preflight_refuses_a_malformed_shell_block_with_its_code`, `preflight::collect::tests::a_failed_remote_child_fetch_keeps_the_transclusion_code`. |
| R11-S5 | Sibling (R11-F2 class): `::shell` parse/run failures and `::shell-block` parse/option/command lines are body-relative | fixed | `ShellCommandOrigin::Authored`, `ShellBlockError::at_authored_lines`, `PageBlockError::at_authored_lines`, note lines moved to file lines. Tests in `directive_authored_lines` and CLI `validate_refs::shell_failures_report_their_authored_line`. |
| R11-S6 | Sibling (R11-F2 class): an unclosed block or unmatched end (page block) gives a body line and a null structured line | fixed | `authored` on `ParseDirective`/`UnmatchedEnd`/`UnterminatedBlock`. `diagnostic_source_provenance::{an_unmatched_end_block_names_its_file, an_unterminated_block_names_its_file}` now expect line 3. |
| R11-S7 | Checked: `link_resolve.rs:123` and `frontmatter_interpolation.rs:1071` raise untyped `Transform` errors | clean (not in these classes) | These errors have no typed cause to lose. Each starts as a plain message, outside the inclusion, reference and directive-location code these findings cover, so neither is an instance of R10-F2 (a typed cause lost on projection) or R11-F2. |
| R11-S8 | Checked required TOC target validation | partial coverage | The validator refuses bare `@`. It accepts a required HTTP URL or a path with a fragment when it checks the spelling as an ordinary link. See the dated verification note below. |

### Progress

- starting the work on 'R10-F2 typed diagnostic codes' at 17:37:25
        - defect class: a typed error's specific code is lost while it is projected to a diagnostic (a tolerated runtime warning, a `when=` parse failure, the `md graph` FileTree error boundary)
        - R10-F2.16 fixed: a tolerated `::toc-linking`/`::file-links` failure keeps its error's code as a warning (`report_diagnostics::directive_failure_code`, used by `pipeline/phases.rs::tolerated_failure_warning`; the engine's `ignore_invalid` file-links skip uses `file_links_code`); severity and recovery unchanged. Built CLI: `filter="["` → `dm.toc_linking.invalid_filter`, `./files/[.md` → `dm.file_links.invalid_glob`, absent `--dir` → `dm.reference.failure`, Markdown file as `--dir` → `dm.file_links.not_a_directory`, unreadable dir (chmod 000 probe) → `dm.input.read_failure`, stdin glob → `dm.file_links.missing_source`; all exit 0 with a `warning`
                - registered those warning codes (7 `dm.toc_linking.*`/`dm.file_links.*` plus `dm.reference.failure`, `dm.input.read_failure`) in `COMPOSE_DIAGNOSTIC_CODES`, so `md compose --deny <code>` accepts and blocks them
                - docs: tolerated-failure example in `docs/errors/composition-problems.md` ("Telling directive failures apart"), sentence in `docs/cli/data-contracts.md`, paragraph in `docs/cli/policy.md`; repo skill `compose.md`
        - R10-F2.17 fixed: `TransclusionError::ConditionParse` projects to `dm.expression.parse_failure` (location unchanged); `1 / 0` stays `dm.expression.evaluation_failure`; local query still returns the raw directive. `diagnostic_source_provenance::a_transclusion_condition_parse_keeps_its_position` encoded the old code and now expects the parse code
        - R10-F2.18 fixed: `error_diagnostics` classifies a bare `ReferenceError` in a source chain (with its `reference_details`); `md graph` passes every `FileTreeError` to it, so `::file @` and `prologue: "@"` report `dm.reference.failure`/`invalid-reference` from `md graph`, `md graph --validate`, and `md validate refs`
        - sweep (sites checked):
                - `report_diagnostics::leaf_diagnostic` Transclusion arm: `ConditionParse` fixed; `InvalidReference`/`FileReference` mapped to `dm.reference.failure` (sibling, fixed; documented meaning of that code); `Io` (including the no-match `TargetNotFound`), `UnsupportedFileType`, cycle, depth, URL, JSON stay `dm.transclusion.failure` (no narrower registered code proven for the class; `Io` noted in the report as an open question)
                - `leaf_diagnostic` fallback `COMPOSE_FAILURE_CODE`: no inclusion/TOC/file-links/condition variant reaches it (TocLinking, FileLinks, PageBlock, Transclusion, Reference all have arms)
                - `error_diagnostics` chain walk: bare `ReferenceError` fixed; `FileTreeError::PathNotFound`/`NotAFile` unreachable from `md graph` (it builds from a loaded `Markdown`)
                - `engine.rs::skipped_transclusion_warning` (`ignore_invalid`/missing): a no-match keeps `dm.reference.missing_transclusion`; other causes are shadowed in `md compose` by the reference check (`dm.reference.failure`) — clean
                - `engine.rs` URL-skip and empty file-links result warnings: built from no typed error — clean (outside the class)
                - CLI `commands/graph.rs` boundary fixed; `commands/validate.rs` already projected through `MarkdownError` — clean
        - tests: new `lib/tests/l1/typed_diagnostic_codes.rs` (4 tests); `cli/tests/l1/validate_refs.rs::commands_keep_specific_codes_for_tolerated_failures_conditions_and_references`
        - focused runs: darkmatter `--lib --test l1` filter `typed_diagnostic_codes|reference_malformed|directive_error|diagnostic_source|report_diagnostics|runtime_blocking|policy|toc_linking|file_links|transclusion|file_tree|condition|graph` 1108/1108; darkmatter-cli `--lib --bins --test l1` filter `validate_refs|graph|compose_report_diagnostics|policy|output_commands|toc|file_links|stdin|transclusion|deny` 418/418; `just lint` exit 0
- work completed for 'R10-F2 typed diagnostic codes'
- orchestrator closed the one sibling the R10-F2 subagent left open
        - decision: a transclusion `Io` error whose resolution failure is `NoMatch` now projects to `dm.reference.missing_transclusion` (the code the tolerated skip warning already uses), and any other `Io` error to `dm.input.read_failure`; alternative was keeping the catch-all `dm.transclusion.failure`, rejected by the specific-code rule
        - `report_diagnostics.rs` projection, `docs/cli/data-contracts.md` code table, new test `typed_diagnostic_codes::a_transclusion_target_that_matches_no_file_is_a_missing_transclusion`; 90/90 scoped tests passed
- work completed for 'R10-F2 typed diagnostic codes' at 17:47:24
- starting the work on 'R11-F1 inline and suppressed inclusions' at 17:47:24
        - defect class: a valid non-file inclusion outcome (inline `prologue`/`epilogue` text, the multiline inline placeholder, an empty scalar, a `::toc-linking` chain suppressed by a trailing `false`) was collapsed into a required file reference by graph construction and validation
        - discovered: the graph used its own `is_literal_content` (newline or leading `---`) instead of composition's `classify_frontmatter_reference`, so single-line prose and links were resolved as paths and `""` failed the build; the inline placeholder was `classify_target("")` (a `LocalPath`), hence `EmptyTarget`; `resolve_toc_linking_target` collapsed `resolve_target_chain`'s `Ok(None)` (suppressed) and `Err` (required) with `.ok().flatten()`
        - R11-F1.1 fixed: one shared predicate `transclusion::frontmatter_value_is_inline` (public; composition's classifier's inline decision) used by both graph modes (`reference/graph.rs`) and the local query (`reference/mod.rs::transclusions_in`, which lists inline values with `resolved_target: None` and attempts no resolution); a parse error of a non-inline value still goes through the old resolver, so R10-F2.18 codes are unchanged
        - R11-F1.2 fixed: the inline placeholder record's target is `ReferenceTarget::Inline` (still `fm_literal`), which validation counts valid; the links inside inline text were already extracted as `Hyperlink` records and are now the only thing checked
        - R11-F1.3 fixed: the graph matches `toc_linking::resolve_target_chain` (the preflight selector) directly; a suppressed chain's record carries `toc_suppressed: true` and `validate.rs` skips its path-existence check; required failure keeps its first-alternative record and `MissingLocalTarget`
        - R11-F1.4 clean: present/missing `::file`/`::code`, `when="false"`, quoted URL, glob and `--dir` file-links pinned in a new control test; DMLS substrate suppression control is the existing `dmls/src/graph/substrate.rs` test over `"&missing.md | ./guide.md | false"`
        - sweep (sites):
                - `compose/transclusion/engine.rs` (composition) and `compose/preflight/collect.rs` (command discovery) already call the classifier; `preflight/lifecycle.rs` reads no prologue/epilogue or TOC chain: clean
                - `reference/graph.rs` frontmatter (both modes) and TOC: fixed; `reference/mod.rs` local query: fixed (TOC entry already lists the authored chain unresolved: clean)
                - `reference/validate.rs` (fresh and prebuilt share `validate_graph_contents`), `compose`'s reference check, FileTree (`file_tree/model.rs` reads `fm_literal`, `Inline` has no raw target): fixed by the graph record change, no extra code
                - DMLS `providers/dsl.rs::frontmatter_prologue_epilogue` (document links): fixed, now skips inline values (`prologue: README` beside a `README` file linked it); DMLS TOC chains already go through `target_chain`: clean
        - tests: new `lib/tests/l1/reference_inline_and_suppressed_inclusions.rs` (5 tests; 3 fail with the graph/validate/local-query changes reverted, the 2 controls pass both ways); `cli/tests/l1/validate_refs.rs::reference_commands_accept_inline_frontmatter_and_suppressed_toc_chains`; `dmls/tests/l1/repository_contexts.rs::inline_prologue_text_is_not_a_document_link` (fails with the DMLS filter reverted)
        - docs: `docs/transclusion/fm-transclusion.md` ("Text or File?"), `docs/cli/graph.md` (stale "newlines or `---`" rule replaced; TOC suppression), `docs/cli/validate.md`, `docs/structs/Markdown.md`
        - focused runs: darkmatter l1 filter `reference_|typed_diagnostic_codes|transclusion|frontmatter_inclusion|prologue|epilogue|toc_linking|empty_target` 761/761; darkmatter `--lib` `reference::|transclusion::resolver|toc_linking|file_tree` 279/279; darkmatter-cli l1 `validate_refs|graph|compose_report` 97/97; darkmatter-lsp l1 `repository_contexts|entry_point_parity|document_link` 27/27 and `--lib` `dsl` 28/28; `just lint` exit 0
- work completed for 'R11-F1 inline and suppressed inclusions' at 17:58:01
- orchestrator review of the R11-F1 subagent result
        - sibling found: inline `prologue`/`epilogue` text was scanned for Markdown links only, though composition inserts it as body Markdown; both sites in `reference/graph.rs` now use `extract_all_references` (the body's extractor), so images and HTML/CSS references in inline text are checked too
        - new test `reference_inline_and_suppressed_inclusions::an_image_inside_inline_frontmatter_text_is_checked_as_an_image`; 683/683 scoped reference tests passed
        - checked: a `::toc-linking` chain whose first target is an invalid reference (not just missing) is still reported as a required missing first target; it remains an error verdict (nothing is silently accepted), so it is recorded as clean for this defect class
- work completed for 'R11-F1 inline and suppressed inclusions' at 17:59:37
- starting the work on 'R11-F2 authored directive lines' at 17:59:37
        - defect class: a directive parser's (or resolver's) prepared-body or raw-body line was published as a file line, in the message and (for TOC/file-links) with a null structured line; condition failures already used authored provenance
        - discovered: every directive parser counts lines in the text it scans (prepared body for compose and both graph modes, raw body for the local query, prepared body plus a fixed header offset for command discovery); the header and a removed `::block` both shift it; `report_diagnostics::parse_failure_source` published only frontmatter-key lines, and TOC/file-links projected file-only
        - fix: new `site_scope::DirectiveSite` (`Body` for a document with no file, `Authored(ctx, opener)` via `authored_opener`, `Unproven` → line `0`, no line in the message, file-only source); `authored: Option<AuthoredSpan>` added to `TransclusionError::ParseDirective`, `ReferenceError::ParseDirective`, and every line-carrying `TocLinkingError`/`FileLinksError` variant; TOC/file-links messages use `at_line`/`line_note` so line `0` names no line; projection publishes `authored` as `source.line`/`column`/`span` (`parse_failure_source`, new `directive_source`, `reference_error_source`)
        - swept sites (all fixed):
                - `pipeline/phases.rs`: body `::file`/`::code`/`::url`, `::toc-linking`, `::file-links` parse; resolve closure places this document's own TOC/file-links resolution and render failures (`Unresolved`, `InvalidGlob`, `TargetNotFound`, `TargetNotDirectory`, `GlobReference`, `Unreadable`, `MissingSourceContext`) by the directive span; `tolerated_failure_warning` carries the authored opener
                - `transclusion/engine.rs`: tolerated file-links failure warning and empty-result warning message, and the FileSystem transform message, use the authored line (`TransclusionEngine::with_origin`)
                - `reference/graph.rs`: the three parser calls (both graph modes share them)
                - `reference/mod.rs` local query: the three parser calls, with `BodyOrigin::capture` of the raw body (header offset proven, no fixed offset)
                - `preflight/collect.rs` (command discovery): transclusion parse (was a fixed header offset), TOC parse and chain resolution, and `::shell` line parse
                - `inline/shell_expansion.rs`: `::shell` parse failure in the compose stage; shell message line only (structured line stays null: a body shell origin has no authored marker)
                - clean: `has_transclusions`, remote-URL prefetch parses, `call_precheck` (errors swallowed); frontmatter `prologue`/`epilogue` (key line, unchanged); `when=` conditions (already authored); DMLS raw-token scanner (no composition parse)
        - tests: new `lib/tests/l1/directive_authored_lines.rs` (7 tests; 4 fail with `DirectiveSite::place` made a no-op); `cli/tests/l1/validate_refs.rs::directive_parse_failures_report_their_authored_line`; updated with notes: `diagnostic_source_provenance::an_omitted_transclusion_target_names_its_line` (was file-only) and `cli compose_report_diagnostics::a_toc_or_file_links_failure_names_the_document_holding_it` (was a null line)
        - docs: `docs/errors/composition-problems.md` (table rows, two JSON examples, authored-line example), `docs/cli/data-contracts.md` code row; repo skill `compose.md`
        - focused runs: darkmatter l1 filter (diagnostic_source, reference_, directive_error, typed_diagnostic_codes, toc, file_links, directive_authored_lines, error_snapshots, shell, condition, compose) 1550/1550; darkmatter `--lib` (toc_linking, file_links, transclusion, reference, shell, preflight, pipeline) 1494/1494; darkmatter-cli l1 (validate_refs, graph, compose_report, shell, toc, file_links, output_commands) 371/371; claudine `--lib` (toc, directive, generated) 199/199; `just lint` exit 0
- starting the work on 'R11-F2 shell and status-block siblings' (orchestrator-assigned follow-up of the R11-F2 subagent)
        - defect class (same as R11-F2): a `::shell`/`::shell-block` failure published a prepared-body line (body count plus a fixed header offset) as a file line, and its structured `source.line` stayed null even where the authored opener was proven
        - decision: the proof rides in the origin, not in a new error field: new public variant `ShellCommandOrigin::Authored { site: Box<origin>, at: Box<AuthoredSpan> }` (boxed for `result_large_err`), `Display`/`line_number` delegate to `site`, and `ShellCommandOrigin::is_body()`; alternatives rejected: an `authored` field on every `ShellExpansionError` variant (~400 construction sites across darkmatter and claudine) or on `Body`/`ShellBlock` (132 sites, and it would enter origin equality), and a wrapper error variant (breaks the CLI's and Claudine's direct `MarkdownError::ShellExpansion`/variant matches). Claudine's prompt-origin rewrite replaces the whole origin, so a placed generated-body origin cannot leak a stale span
        - item 1 fixed: `::shell` parse failure now publishes `source.line`/`column`/`span` (`ShellExpansionError::at_site` places a `Body` origin; `report_diagnostics::shell_source` projects `Authored`). Built CLI: `::shell "unterminated` after a header and a false block → `dm.directive.parse_failure` line 9 column 1 (was null)
        - item 2 fixed (was wrong): with frontmatter and an earlier false `::block`, composition's command failures named the prepared line (`Command failed (exit 1): 'false' at line 7` for authored line 11; `Command not found` the same; shell-block `command at line 8` for authored 12). Now `inline/shell_expansion.rs` places the site, prepare, and execute failures, and `ShellBlockError::at_authored_lines` (called from `pipeline/phases.rs::run_shell_blocks_operation`) places a block command's lines, excerpt (now of the file), and inner origin (`at_block_sites`); built CLI reports line 11 / 12 in message and `source`
                - pre-flight (`md compose`'s approval walk) named the prepared line after interpolation inserted lines (`Approval required ... at line 10` for authored line 7); `preflight/collect.rs` now moves the `::shell` and shell-block entry lines to the authored ones, keeping `Body`/`ShellBlock` (not `Authored`): its error context is the root's even for a child's entry, so a structured line would name the child's line in the root file; its `source` stays file-only, documented
                - membership is by normalized command, never by origin, so placing changes no approval
                - unproven (body no longer described by the map): message keeps its line, `source` file-only (pinned by a test)
        - item 3 fixed: `site_scope::{line_row, status_rows}`; every line-carrying `TocLinkingError`/`FileLinksError` status block omits the `Line:` row for line `0`; other blocks touched this cycle already handle `0` (`TransclusionError`/`ReferenceError::ParseDirective` drop the excerpt; `at_line`/`line_note` messages): clean
        - `DirectiveSite::of_line` now starts past block-quote markers, so a block command inside `> ::map` is placed at its command column (the iteration location test caught column 1)
        - unresolved, recorded: `ShellBlockError::Parse`/`Unterminated` lines are counted inconsistently (scan errors body-relative, tokenize errors body-relative, data-shaped rejections with the header offset), so they are not placed; and pre-flight stringifies a shell-block tokenization failure as `MarkdownError::Transform` (`dm.compose.failure`, body-relative line, inner `line 0`), a code-loss sibling of R10-F2
        - tests: `directive_authored_lines` 4 new (`a_shell_parse_failure_places_its_source_at_the_file_line`, `a_failing_shell_command_names_its_file_line` (not found and denied), `a_failing_shell_block_command_names_its_file_lines`, `an_unproven_shell_failure_names_its_file_without_a_line`) and `a_malformed_shell_line_names_its_file_line` now asserts the structured line; the 4 placement tests fail with `at_site`/`at_block_sites` made no-ops; `error_snapshots::toc_linking::an_unknown_line_shows_no_line_row`; CLI `validate_refs::shell_failures_report_their_authored_line`; updated with notes: `diagnostic_source_provenance::{a_body_command_past_a_rewrite_publishes_no_prepared_line, a_shell_block_command_publishes_no_prepared_line}` (were file-only, now the authored line), `scope_site_conformance::origin_line` (new variant), claudine `prepare::tests::inline_shell_execution_failure_in_a_child_keeps_the_child_origin` (child origin now placed)
        - claudine: `prepare.rs` prompt-origin rewrites use `ShellCommandOrigin::is_body()` so a placed origin is still moved to `prompt`
        - docs: `docs/errors/composition-problems.md` (stale "file alone" paragraph rewritten with both cases, table rows); repo skill `compose.md`
        - focused runs: darkmatter l1 (directive_authored_lines, diagnostic_source, shell, error_snapshots, typed_diagnostic_codes, collection_blocks) 794/794; darkmatter `--lib` (shell, toc_linking, file_links, preflight, pipeline) 996/996; darkmatter-cli l1 (shell, compose_report, validate_refs, approval) 166/166; claudine `--lib` (shell, compose) 450/450, plus (prompt, preflight, origin) 1013/1013, claudine l1 (shell, diagnostic, origin, inline) 83/83, claudine-cli (shell, error_walker, origin) 173/173; `cargo check -p claudine-cli --tests` clean; `just lint` (darkmatter) exit 0
- work completed for 'R11-F2 shell and status-block siblings' at 18:49:11
- starting the work on 'R11-F2 shell-block parse and pre-flight residuals' (the two sites the previous entry left unresolved)
        - item 1 fixed: `ShellBlockError::Parse`/`Unterminated` gain `authored: Option<Box<AuthoredSpan>>` (boxed for `result_large_err`); every shell-block failure now leaves the stage counted past the header (scan errors, `parse_shell_block_region`, and `split_logical_commands` were body-relative, the data-shaped rejection was offset) through `shell_blocks::split_block_commands` and `ShellBlockError::shifted`; `ShellBlockError::at_authored_lines` now also places a parse failure (authored line, `authored`, file excerpt; unproven: line `0`, no excerpt, no line in the message or status block; no file: unchanged); the tokenizer's own `line 0` no longer enters the message (`body.rs::reason`); `report_diagnostics::shell_block_source` publishes `authored`
                - discovered: an unclosed `::shell-block` never reaches `ShellBlockError::Unterminated` (no constructor); the page-block scan refuses it first as `PageBlockError::UnterminatedBlock`, with the body line in the message and a file-only `source`. Fixed in the same way: `authored` on `ParseDirective`/`UnmatchedEnd`/`UnterminatedBlock`, `PageBlockError::at_authored_lines` at both page-block scans (`inline/page_blocks.rs`, `inline/collection_blocks.rs`), projected through `condition_source`
                - pairing notes ("opened at line N", "file ends at line N") counted the body in the collection, page-block, and shell-block stages: `BlockPairError::note_with`, `site_scope::{scan_note, DirectiveSite::file_line}`, `CollectionError::with_file_note` (compose stage and call precheck) move the note's line to the file or drop it when unproven
                - built CLI, `---` header + false `::block` + `::shell-block`/`echo "unterminated`: `dm.shell.failure` line 13 column 1 (was `dm.compose.failure`, body line 9, `line 0`); `timeout="x"`: line 10; unclosed block: `dm.directive.parse_failure` line 10 (was body line 6, null); stdin document keeps its document-text line, `source` null
        - item 2 fixed: `preflight/collect.rs` no longer stringifies a block command's tokenize failure; both split sites return the typed `ShellBlockError` placed against the walked document (`at_authored_lines` over the prepared body, or `BodyOrigin::capture` for the authored-bytes retry). Code is `dm.shell.failure`, what composition reports for the same block
                - decision: unlike a refused command (file-only, its error context is the root's), the block error's `source_file` is the walked document's own file, so its authored `source.line` is published; documented in `docs/errors/composition-problems.md`
                - decision: kept `dm.shell.failure` for a shell-block parse failure (the existing composition code) rather than moving it to `dm.directive.parse_failure`; `docs/cli/data-contracts.md` now states which code each gets
        - item 3 sweep:
                - `preflight/collect.rs::fetch_remote_child_body`: a failed remote child fetch was `Transform` (`dm.compose.failure`); now composition's `TransclusionError::RemoteFetchFailed` (`dm.transclusion.failure`): fixed
                - `preflight/collect.rs` `::shell` line parse and transclusion/TOC parse (placed last cycle), `preflight/lifecycle.rs`, `preflight/mod.rs`, claudine `prepare.rs`/`lifecycle_preflight.rs`: no `Transform` wrapping of a typed error: clean
                - `pipeline/mod.rs`/`phases.rs` internal-error `Transform`s: not typed errors: clean
                - not fixed, outside the pre-flight scope: `pipeline/phases.rs` link normalization wraps `NormalizationError` in `Transform` (`dm.compose.failure`); a specific code needs registering
        - tests: `directive_authored_lines` 3 new (`a_malformed_shell_block_names_its_file_line`, `an_unterminated_shell_block_names_its_opener_line`, `preflight_refuses_a_malformed_shell_block_with_its_code_and_file_line` with a valid-block control; all 3 fail with the placement made a no-op); `preflight::collect::tests::a_failed_remote_child_fetch_keeps_the_transclusion_code`; CLI `validate_refs::shell_failures_report_their_authored_line` extended with the pre-flight block record; updated with notes: `diagnostic_source_provenance::{an_unmatched_end_block_names_its_file, an_unterminated_block_names_its_file}` (were file-only, now line 3) and `shell_block_integration::pairing_errors_keep_their_secondary_lines_for_an_ordinary_file` (notes now file lines)
        - docs: `docs/errors/composition-problems.md` (two table rows, pre-flight block example), `docs/cli/data-contracts.md` (`dm.shell.failure`, `dm.directive.parse_failure` rows); repo skill `compose.md`
        - focused runs: darkmatter l1 (directive_authored_lines, diagnostic_source, shell, preflight, approval, error_snapshots, typed_diagnostic_codes, block, collection) 1150/1150; darkmatter `--lib` (shell, preflight, page_block, block_pairs, collection, call_precheck) 992/992; darkmatter-cli l1 (shell, approval, compose_report, validate_refs) 166/166; claudine `--lib` (shell, compose, preflight) 705/705, claudine l1 (shell, diagnostic, origin, inline, block) 93/93, claudine-cli (shell, preflight, block, error_walker) 267/267; `cargo check -p claudine-cli --tests` clean; `just lint` (darkmatter) exit 0
- work completed for 'R11-F2 shell-block parse and pre-flight residuals' at 19:19:40
- orchestrator reconciliation of R11-F2 follow-ups
        - two follow-up subagents closed the `::shell`/`::shell-block`/page-block sibling sites (see their bullets above); a remaining R10-F2-class sibling was found in heading normalization (`NormalizationError` wrapped as `Transform` → `dm.compose.failure`), dispatched with R11-F3 below
        - decision kept: a shell-block option or command that does not parse stays `dm.shell.failure` (the code composition already reported); `docs/cli/data-contracts.md` now states it explicitly; alternative `dm.directive.parse_failure` rejected to avoid moving an existing code
- work completed for 'R11-F2 authored directive lines' at 19:20:23
- starting the work on 'R11-F3 FileTree admitted source' (and the normalization-code sibling in parallel) at 19:20:23
- R11-F3 subagent (parallel group A) reported:
        - defect class: an admitted document identity is lost when it passes into a second preparation step
        - fix: the shared validation engine `validate_graph_contents` (`reference/validate.rs`) admits the root's source (`ComposeRequest::admitted_document`) before `rooted()` and the root-heading lookup, so FileTree's `validate_fresh_graph` call and any future caller get the admitted document; a second admission returns it borrowed, so fresh and prebuilt validation are unchanged
        - new module `reference_root_heading_admission`: 4 source forms × {`#ready` → 0, `#gone` → 1 `MissingFragmentTarget`} × {FileTree, fresh, prebuilt}, bare `labels.yaml` and `./schemas/labels.yaml`; failed before the change at `labels.yaml #ready RequestOnlyRelative file tree`
        - swept: FileTree `ensure_built`, `validate`, `validate_with_graph`, `validate_fresh_graph`, heading-slug collectors, graph builders, public reference APIs, CLI graph/validate/compose reference check, `report.rs`, DMLS (no calls) — all fixed through the engine or clean
        - scoped runs: darkmatter l1 691/691, `--lib` 434/434, darkmatter-cli l1 74/74
- normalization-code subagent (parallel group B, R10-F2 class sibling) reported:
        - `MarkdownError::Normalization(NormalizationError)` plus registered codes `dm.normalization.level_overflow` and `dm.normalization.validation_failed`; the compose stage returns the typed error instead of `Transform`; `error_diagnostics` classifies a bare `NormalizationError` from `Markdown::normalize`/`normalize_mut`/`relevel`
        - the tolerated overflow warning carries `dm.normalization.level_overflow`; sibling: the transclusion heading-overflow-to-bold warning now carries `dm.transclusion.heading_overflow` (was `dm.compose.warning`); both in `COMPOSE_DIAGNOSTIC_CODES`
        - docs: `data-contracts.md`, `policy.md`, new "Headings pushed past H6" section in `composition-problems.md`; drift fixed in `transclusion-design.md` (it described an overflow error that the code never raised)
        - scoped runs: darkmatter l1 141/141, `--lib` 210/210, darkmatter-cli l1 259/259
        - swept: internal invariant `Transform`s in `phases.rs`/`pipeline/mod.rs` (not user-facing, clean); `link_resolve.rs:123` and `frontmatter_interpolation.rs:1071` produce untyped author-facing errors outside the inclusion/reference code this cycle's classes cover (recorded below)
- orchestrator: fixed the remote-child sibling group B found in the inclusion path
        - `transclusion/engine.rs` wrapped a fetched child's frontmatter parse error as `Transform` (`dm.compose.failure`); it now parses through the new crate-private `Markdown::try_from_named_content`, so the error keeps `dm.frontmatter.yaml_parse` and names the URL
        - test `network_permission::a_remote_child_with_malformed_frontmatter_keeps_its_parse_code` (fail-fast; valid child control); 25/25 `network_permission` passed
- work completed for 'R11-F3 FileTree admitted source' at 19:31:02
- starting final verification gates at 19:31:02
- work completed for 'final verification gates' at 19:49:08

### Completion Report

- **outcome: complete.** Elapsed time was about 2 h 12 min (17:37 to 19:49 UTC).
- parent findings: total 4 = fixed 4 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - R10-F2 (high): fixed. Each typed cause now keeps its registered code: tolerated TOC/file-list warnings, `when=` parse failures, `md graph`'s reference failures, plus sibling rows R11-S1 to S4
        - R11-F1 (high): fixed for inline prologue/epilogue and the local-path TOC fixtures. The graph records suppression separately, but validation only honors it for local-path targets. See the dated verification note below.
        - R11-F2 (high): fixed for the listed parser and placement cases. Directive grammar, shell, shell-block and page-block failures report the authored line, or the file when a line cannot be proven (siblings R11-S5, S6). File-target failures can still lose their sites. See the dated verification note below.
        - R11-F3 (medium): fixed. The shared validation engine admits the root source, so FileTree and every other caller prepare root headings from the admitted document
- shared rules: one frontmatter inline classifier (composition, graph, local query, DMLS); one TOC chain selector with three outcomes; one authored-opener mapping (`DirectiveSite`, the body map that conditions use); one admission at the validation engine's root
- decisions (ledger only, nothing for human review): a shell-block parse failure stays `dm.shell.failure`, and both code rows say so; a tolerated child failure other than TOC/file-links keeps its existing warning code, so `--deny` doesn't widen; a missing transclusion target maps to `dm.reference.missing_transclusion`
- open finding or child IDs: none
- cross-OS: no `just cross-check` was run. The changes are line and code projection plus source admission, with no new path, process or `#[cfg]` code, so cross-OS risk is low and CI's nightly covers Windows/WSL2
- final verification
        - `darkmatter: just test --max-fail all`: 18,246 passed, 10 skipped, none at or above 5.0 s (all findings). An earlier run stopped on 2 `finding_35_2` oracle tests that expected the uncoded heading-overflow warning; the oracle now sets the code (R11-S2)
        - `darkmatter: just lint`: exit 0, including DMLS and the zed-dmls wasm check
        - `claudine: just test --max-fail all`: 13,077 passed, 16 skipped, none at or above 5.0 s (R11-S5 changes in `claudine/lib/src/composition/prepare.rs`)
        - `claudine: just lint`: exit 0
        - `just check-tier-coverage darkmatter` and `claudine`: 0 stranded
- changed files (working tree, uncommitted)
        - `.claude/skills/darkmatter/SKILL.md`
        - `.claude/skills/darkmatter/compose.md`
        - `.claude/skills/darkmatter/requests.md`
        - `claudine/lib/src/composition/prepare.rs`
        - `claudine/lib/src/composition/prepare/tests.rs`
        - `darkmatter/cli/src/commands/graph.rs`
        - `darkmatter/cli/tests/l1/compose_report_diagnostics.rs`
        - `darkmatter/cli/tests/l1/validate_refs.rs`
        - `darkmatter/dmls/src/providers/dsl.rs`
        - `darkmatter/dmls/tests/l1/repository_contexts.rs`
        - `darkmatter/docs/cli/data-contracts.md`
        - `darkmatter/docs/cli/graph.md`
        - `darkmatter/docs/cli/policy.md`
        - `darkmatter/docs/cli/validate.md`
        - `darkmatter/docs/errors/composition-problems.md`
        - `darkmatter/docs/inline/link-validation.md`
        - `darkmatter/docs/structs/Markdown.md`
        - `darkmatter/docs/topics/compose-requests.md`
        - `darkmatter/docs/transclusion/fm-transclusion.md`
        - `darkmatter/docs/transclusion/transclusion-design.md`
        - `darkmatter/docs/use-skills/use-darkmatter/library.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/log.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-10.md`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/spec.md`
        - `darkmatter/lib/src/markdown/compose/block_pairs.rs`
        - `darkmatter/lib/src/markdown/compose/call_precheck.rs`
        - `darkmatter/lib/src/markdown/compose/collection_blocks/error.rs`
        - `darkmatter/lib/src/markdown/compose/context/options.rs`
        - `darkmatter/lib/src/markdown/compose/context/request.rs`
        - `darkmatter/lib/src/markdown/compose/file_links/discovery.rs`
        - `darkmatter/lib/src/markdown/compose/file_links/parser.rs`
        - `darkmatter/lib/src/markdown/compose/file_links/types.rs`
        - `darkmatter/lib/src/markdown/compose/inline/collection_blocks.rs`
        - `darkmatter/lib/src/markdown/compose/inline/page_blocks.rs`
        - `darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs`
        - `darkmatter/lib/src/markdown/compose/mod.rs`
        - `darkmatter/lib/src/markdown/compose/page_blocks/parser.rs`
        - `darkmatter/lib/src/markdown/compose/page_blocks/types.rs`
        - `darkmatter/lib/src/markdown/compose/parse_utils.rs`
        - `darkmatter/lib/src/markdown/compose/pipeline/phases.rs`
        - `darkmatter/lib/src/markdown/compose/policy.rs`
        - `darkmatter/lib/src/markdown/compose/preflight/collect.rs`
        - `darkmatter/lib/src/markdown/compose/preflight/lifecycle.rs`
        - `darkmatter/lib/src/markdown/compose/report_diagnostics.rs`
        - `darkmatter/lib/src/markdown/compose/schema_validation.rs`
        - `darkmatter/lib/src/markdown/compose/shell_blocks/body.rs`
        - `darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs`
        - `darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs`
        - `darkmatter/lib/src/markdown/compose/shell_blocks/types.rs`
        - `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`
        - `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
        - `darkmatter/lib/src/markdown/compose/site_scope.rs`
        - `darkmatter/lib/src/markdown/compose/toc_linking/filter.rs`
        - `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs`
        - `darkmatter/lib/src/markdown/compose/toc_linking/parser.rs`
        - `darkmatter/lib/src/markdown/compose/toc_linking/types.rs`
        - `darkmatter/lib/src/markdown/compose/transclusion/engine.rs`
        - `darkmatter/lib/src/markdown/compose/transclusion/mod.rs`
        - `darkmatter/lib/src/markdown/compose/transclusion/parser.rs`
        - `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs`
        - `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
        - `darkmatter/lib/src/markdown/errors/mod.rs`
        - `darkmatter/lib/src/markdown/mod.rs`
        - `darkmatter/lib/src/markdown/reference/errors.rs`
        - `darkmatter/lib/src/markdown/reference/file_tree/model.rs`
        - `darkmatter/lib/src/markdown/reference/graph.rs`
        - `darkmatter/lib/src/markdown/reference/mod.rs`
        - `darkmatter/lib/src/markdown/reference/validate.rs`
        - `darkmatter/lib/src/markdown/types.rs`
        - `darkmatter/lib/tests/l1/diagnostic_source_provenance.rs`
        - `darkmatter/lib/tests/l1/directive_error_document.rs`
        - `darkmatter/lib/tests/l1/error_snapshots/page_block.rs`
        - `darkmatter/lib/tests/l1/error_snapshots/reference.rs`
        - `darkmatter/lib/tests/l1/error_snapshots/toc_linking.rs`
        - `darkmatter/lib/tests/l1/error_snapshots/transclusion.rs`
        - `darkmatter/lib/tests/l1/main.rs`
        - `darkmatter/lib/tests/l1/network_permission.rs`
        - `darkmatter/lib/tests/l1/reference_bare_schema_roots.rs`
        - `darkmatter/lib/tests/l1/reference_integration.rs`
        - `darkmatter/lib/tests/l1/reference_report.rs`
        - `darkmatter/lib/tests/l1/scope_site_conformance.rs`
        - `darkmatter/lib/tests/l1/shell_block_integration.rs`
        - `darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-11.md`
        - `darkmatter/lib/tests/l1/directive_authored_lines.rs`
        - `darkmatter/lib/tests/l1/reference_inline_and_suppressed_inclusions.rs`
        - `darkmatter/lib/tests/l1/reference_malformed_inclusions.rs`
        - `darkmatter/lib/tests/l1/reference_root_heading_admission.rs`
        - `darkmatter/lib/tests/l1/typed_diagnostic_codes.rs`


### Verification correction, 2026-11-25

Review 12 reran the installed-form commands from the checkout’s built `md` with temporary files. The required-TOC ledger row overgeneralized the bare-`@` refusal. `md validate refs --fragments` accepts `::toc-linking ./side.md#side` when `side.md` has that heading. It also accepts a required HTTP URL. `md compose` rejects both targets. A chain ending in `false` is suppressed by composition, but a leading `#absent` still produces a missing fragment in reference validation and a leading loopback URL still receives a HEAD request under `--remote`. The existing local-path suppression cases still pass.

The completion report also overstated location coverage for target resolution. With a YAML header, both `::file @` and `prologue: "@"` produce `dm.reference.failure` with a null structured line in the built validating commands. A public graph or preflight probe with the same directive in an included child produces an error without the child source. Directive grammar and condition errors still carry the authored position for the original repair fixtures. [Review 12](review-12.md)

## Implementation of Review Findings #12

> **started at:** 2026-11-25T19:57:40+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-12.md'
- this is iteration 12 of the review-to-implement cycle 
- review `implemented` was already `false`; the review has no human-in-the-loop items and no blocked findings

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R12-F1 | Reference validation applies hyperlink rules to inclusion targets | fixed | The listed hash-suffix and TOC cases are fixed or checked. Inclusion records keep the full local filename and the TOC outcome, which validation reads before the target-kind branches and remote batch. The additional Markdown type and URL admission cases are in R13-F1. Siblings R12-S1–S3 below. Full Darkmatter L1 18,273/18,273. |
| R12-F1.1 | Body `::file`/`::code` `./side.md#side` (quoted too): validate the complete filename in both graphs, local query, flattening, validators | fixed | `reference_inclusion_targets::a_hash_suffix_on_a_file_inclusion_is_part_of_its_filename`: quoted and unquoted `::file`/`::code` `./side.md#side` give `(MissingLocalTarget, Transclusion)` in fresh, prebuilt, compose-check and FileTree validation in every mode (it was `[]`). The local query resolves nothing and the plain `./side.md` control passes. CLI: `validate_refs_fragments_judges_a_file_inclusion_by_its_whole_filename`. |
| R12-F1.2 | Prologue/epilogue `"./side.md#side"`: same filename rule; inline text and its links keep working | fixed | Same test covers `prologue`/`epilogue` `"./side.md#side"`. The R11 inline-text module still passes 6/6. |
| R12-F1.3 | Required TOC with `./side.md#side`, `#here`, HTTP URL, `mailto:`, `data:`: required failure preserved in every validation mode; no HEAD request decides it | fixed | `a_required_toc_target_the_selector_rejects_is_a_required_failure`: `./side.md#side`, `#here`, loopback URL, `mailto:`, `data:` and `./gone.md` are each a required `MissingLocalTarget` in every mode, with 0 server requests; `./side.md` passes. CLI: `validate_refs_remote_never_requests_a_toc_target`, `graph_validate_reports_a_fragment_shaped_toc_target_as_missing`. |
| R12-F1.4 | Suppressed TOC (`… \| false`, `FALSE`, `false` alone) for path, fragment and URL: suppression applies before target-kind checks and remote batch; removing `false` restores required failure | fixed | `a_suppressed_toc_chain_is_never_validated_or_requested`: `./gone.md`, `#absent` and URL with `\| false` / `\| FALSE`, and `false` alone give no issue, no warning, 0 requests in all modes. Removing `false` restores the required failure with 0 requests. |
| R12-F1.5 | Controls: URL then `./side.md` then `false` selects local file in graphs, validators, preflight; query lists raw alternatives; editor resolver unchanged | clean | `a_toc_chain_falls_back_from_a_url_to_a_local_file`: both graphs select `./side.md`, no issues, 0 requests. The local query lists the raw chain unfollowed and `compose_preflight` selects `side.md`. The DMLS passive resolver does not read the reference graph and is unchanged; darkmatter-lsp reference/link tests 398/398. |
| R12-F1.6 | Controls: plain links `#here`, `./side.md#side`, URL; `::url`; file-list; inline frontmatter links; false `when=`; generated TOC links | clean | `ordinary_links_and_remote_inclusions_keep_their_rules`: `[a](#here)`, `[b](./side.md#side)`, `[c](URL)`, `::url` and a generated TOC pass, with exactly 6 requests (link and `::url` × 3 remote validators). Broken `#nowhere` gives `MissingFragmentTarget`. The R11 file-list, `when=false` and inline-link tests pass. |
| R12-F2 | Inclusion target errors lose the authored file and line | fixed | Every child below fixed or checked. `InclusionSite` plus `TransclusionError::Located` attach the authored site where the inclusion is known, and one projection publishes it from every reader. Siblings R12-S4–S12 below. Full Darkmatter L1 18,273/18,273 and Claudine 13,077/13,077. |
| R12-F2.1 | Body `::file`/`::code`/`::url` `@` (and quoted under a false page block): authored opener in compose, preflight, both graphs, validators, local query; listing boundary kept | fixed | `inclusion_target_sites::an_invalid_body_target_names_its_directive_line`: `::file`/`::code`/`::url` with `@` and `'@'` at line 8, and at line 11 after a false block, across 7 readers give `dm.reference.failure`, `doc.md` and the authored line. The listing boundary holds and the valid control passes. CLI: `validate_refs::an_invalid_body_target_reports_its_directive_line`. |
| R12-F2.2 | Frontmatter `prologue`/`epilogue` `"@"` on line 3: key location in graph, raw query, preflight, compose, validation; shape-error control | fixed | `an_invalid_frontmatter_target_names_its_key_line`: `prologue`/`epilogue` `"@"` at 3:11 and as a list entry at line 5. The shape control `3` keeps its key line and `dm.directive.parse_failure`; `"./side.md"` passes. CLI: `an_invalid_frontmatter_target_reports_its_key_line`. |
| R12-F2.3 | Missing targets `./gone.md` in direct compose and preflight (`::file`, `::code`, prologue, epilogue) carry the site and keep `dm.reference.missing_transclusion` | fixed | `a_missing_target_names_its_inclusion_site`: `./gone.md` in `::file`, `::code`, prologue and epilogue keeps `dm.reference.missing_transclusion` plus the site in direct compose and preflight. |
| R12-F2.4 | Nested `nested.md` with `::file @` at line 7: graph builds, preflight, validating CLI name `nested.md` line 7 | fixed | `a_nested_invalid_target_names_the_child`: graph builds, preflight and validation name `nested.md` line 7; the tolerated compose warning names it too, with `dm.reference.failure`. CLI: `a_nested_invalid_target_reports_the_child` (compose, `validate refs`, `graph --validate`). |
| R12-F2.5 | Required TOC bare `@` through graph validation keeps the located selector failure | fixed | Via R12-F1 `toc_unresolved`. `a_required_toc_target_the_selector_rejects_names_its_directive_line`: the preflight error, the validation issue source and the compose warning are at line 8, and at 11 after a removed block. The existing-file control is clean. |
| R12-F2.6 | Controls: directive grammar, TOC level, file-list depth/glob, `when=`, shell/page block, child YAML, missing inline link keep locations and codes | clean | `directive_authored_lines`, `typed_diagnostic_codes`, `diagnostic_source_provenance`, `reference_malformed_inclusions`, `reference_inline_and_suppressed_inclusions` and `reference_inclusion_targets` all pass in the full L1 run. No error snapshot changed; one new snapshot was added. |
| R12-S1 | Sibling (F1 class): a URL-shaped `::file-links` glob reached the `RemoteUrl` branch | fixed | Skipped before the kind branch and the remote batch. `a_file_list_glob_is_never_a_remote_reference` failed with only this part reverted. |
| R12-S2 | Sibling (F1 class): `::file "#side"` / `::code "#side"` were checked against the root's headings | fixed | Now missing files: `a_fragment_shaped_file_inclusion_names_a_local_file`. |
| R12-S3 | Sibling (F1 class): preflight, local query `transclusions_in`, DMLS `substrate.rs`/`document_links` | clean | Each already reads the whole filename; `entry_point_parity` hash and chain tests and darkmatter-lsp 398/398 pass. |
| R12-S4 | Sibling (F2 class): a tolerated child failure warning was uncoded and unlocated | fixed | `phases.rs::tolerated_failure_warning` carries the site and a registered code; covered by the F2.4 test. |
| R12-S5 | Sibling (F2 class): `::code` read failures (non-UTF-8) unlocated | fixed | `PreparedTransclusion::{Markdown, Code, RemoteFile, RemoteCode}` carry the site and apply it through `target_failure_at`. |
| R12-S6 | Sibling (F2 class): an existing but unreadable included child gave `dm.input.read_failure` with no line | fixed | Ordinary child loads are located at the directive in compose, preflight, both graphs and validation. The TOC heading read takes a separate path and still loses the location (R13-F2). |
| R12-S7 | Sibling (F2 class): a refused remote fetch (`::url`, remote `::code`) was unlocated | fixed | `fetched_body`/`fetch_remote_child_body` attach the site; `transclusion_failure_is_fatal` looks through the wrapper, so a refused fetch is still fatal. |
| R12-S8 | Sibling (F2 class): local query file/code/prologue/epilogue resolution and `validate_local_path` for inclusions | fixed | Both go through `reference::at_inclusion`; covered in `inclusion_target_sites`. |
| R12-S9 | Consumer: Claudine `is_missing_include` | fixed | Matches through the wrapper, with a unit case for a located error; `sequence_initialize_include_preflight` and Claudine L1 13,077/13,077 pass. |
| R12-S10 | Sibling (F2 class): a hyperlink/image `[x](@)` was a hard `dm.reference.failure` with a null line | fixed | Decision: a located error-severity `ReferenceIssue` (`InvalidReference`, `failure: invalid-reference`) instead of a second error wrapper. Default validation still exits 1 and the location is added (6:1, 8:1, inline prologue 2:11). A matching missing-target allowance can change composition’s verdict to exit 0 (R13-F3). Tests: `an_invalid_link_target_is_a_located_issue_in_every_validator`, CLI `an_invalid_link_target_is_reported_at_its_position`. |
| R12-S11 | Sibling (F2 class): the human status block of a located error had no location row | fixed | `File:`/`Line:` rows via the shared `site_scope::line_row`, using the new public `biscuit-terminal` `StatusBlock::prepend_body_line` (unit tested). New snapshot `located_file_reference`, plus `located_composition_failure_names_the_file_and_line`. |
| R12-S12 | Found by S10: a refused `md compose` reference check dropped warnings found while preparing it | fixed | `ReferenceValidationReport::earlier_warnings` plus `deliver_refused_check_warnings` (plain and lifecycle compose). Tests: `an_unresolvable_link_refuses_the_run_and_keeps_the_earlier_warning`, `a_refused_check_keeps_the_earlier_warning`. |
| R12-S13 | Coverage: CLI document-fallback wiring lost its fixture after F2 | fixed | An 18-deep chain gives a file-less depth error; `compose_report_diagnostics::reference_commands_name_their_document_on_a_failure_with_no_file` asserts `n0.md` with null line/column for `validate refs` and `graph`. |
| R12-S14 | Checked: inclusion depth-limit and cycle errors name only a file | clean (not in this class) | They judge the whole chain, not one target's resolution, so no target site was lost; the depth error is also the CLI fallback fixture above. |

### Progress

- starting the work on 'R12-F1 inclusion targets validated as hyperlinks' at 19:58:07
        - defect class: reference validation discards the inclusion selector's decision (a file inclusion's whole filename, or a TOC chain's selected / required-failure / suppressed outcome) and re-judges the raw input as a hyperlink, stripping `#fragment`, checking headings and sending URLs to the remote HEAD batch
        - shared rule: graph record construction decides each inclusion's target once (`graph.rs::opened_target` applies composition's `FileReference` rule: `http(s)` → `RemoteUrl`, anything else one whole-filename `LocalPath`, `#` included; a TOC record is always the selected alternative, or the first alternative marked `toc_suppressed` or the new `toc_unresolved`); validation only reads it
        - `validate.rs::names_no_target` (file-list listings and suppressed TOC chains) runs before any target-kind branch and filters the remote batch; `toc_unresolved` goes straight to a located `MissingLocalTarget` issue, never re-resolved; a `Transclusion` `LocalPath` never splits a fragment
        - decision: a required TOC failure takes the selector's verdict instead of resolving again; re-resolving `mailto:` produced a hard `ReferenceError` where composition only tolerates a warning. Side effect: a required TOC bare `@` is now a located issue at the directive (feeds R12-F2.5)
        - sibling R12-S1 fixed: a URL-shaped `::file-links` glob reached the `RemoteUrl` branch (unverified-URL warning or HEAD request); now skipped before the kind branch
        - sibling R12-S2 fixed: `::file "#side"` / `::code "#side"` were checked against the root's heading; now missing files
        - sibling R12-S3 clean: preflight (`entry_point_parity` hash and chain tests), local query `transclusions_in`, DMLS `substrate.rs` / `document_links` already read the whole filename
        - tests: new `lib/tests/l1/reference_inclusion_targets.rs` (4 of its 6 tests failed before the fix); CLI `validate_refs::{validate_refs_fragments_judges_a_file_inclusion_by_its_whole_filename, validate_refs_remote_never_requests_a_toc_target, graph_validate_reports_a_fragment_shaped_toc_target_as_missing}`
        - docs: `docs/cli/validate.md`, `docs/inline/link-validation.md`, `docs/transclusion/fm-transclusion.md` now say `#` belongs to an inclusion's filename and TOC alternatives are always files
        - focused runs: darkmatter l1 reference/toc/transclusion/preflight/file_tree/graph/parity filter 994/994; diagnostic/snapshots/network/remote filter 526/526; lib unit 709/709; CLI validate_refs/graph/compose_report 102/102; `just lint` exit 0
- work completed for 'R12-F1 inclusion targets validated as hyperlinks' at 20:12:43
- starting the work on 'R12-F2 inclusion target errors lose authored file and line' at 20:12:43
        - defect class: an inclusion's typed target failure (invalid `@`, missing, unreadable, refused fetch, unsupported type) keeps its cause but loses the directive or frontmatter site before it reaches the caller, giving a null line or naming the input document instead of the child
        - shared mechanism: public `InclusionSite` (file plus optional authored span) and `TransclusionError::Located { site, source }`; `at_inclusion()` never replaces an existing site, so a nested child's site survives its parents; sites come from `DirectiveSite::of_span` (body) and `InclusionSite::of_frontmatter_value` over a shared `frontmatter_value_range` (key, list index, value)
        - graph readers convert a target `ReferenceError::{FileReference, Io}` into composition's located error via `reference::at_inclusion`, so compose, preflight, both graph modes, local query and validation report one code and one place; `report_diagnostics` takes the code from the inner error (`transclusion_code`) and publishes `source.line`/`column`/`span`; `with_document_fallback` only fires when no file is known
        - decision: one wrapper variant instead of a site field on each variant (`Io`/`FileReference` are tuple variants); graph and validation now return `ReferenceError::Compose(Transclusion(Located))` where they returned `ReferenceError::FileReference`; messages gain a `file:line:column:` prefix
        - siblings fixed: R12-S4 tolerated child failure warning uncoded and unlocated; R12-S5 `::code` read failures (non-UTF-8); R12-S6 existing-but-unreadable included child; R12-S7 refused remote fetch (`transclusion_failure_is_fatal` looks through the wrapper); R12-S8 local query and `validate_local_path` transclusion records; R12-S9 Claudine `is_missing_include` matches through the wrapper
        - sibling R12-S10 found open: a hyperlink `[x](@)` is a hard `dm.reference.failure` with a null line; R12-S11 found open: the human status block of a located error has no location row
        - tests: new `lib/tests/l1/inclusion_target_sites.rs` (with `at_inclusion` made a no-op 6 of 8 fail; the other 2 are controls); CLI `validate_refs::{an_invalid_body_target_reports_its_directive_line, an_invalid_frontmatter_target_reports_its_key_line, a_nested_invalid_target_reports_the_child}`; tests that encoded unlocated behavior updated (`compose_report_diagnostics` fallback pair now expects line 3 col 11, `file_tree_roots`, `reference_integration`, compose unit transclusion test, Claudine sequence tests read the cause through `without_site()`)
        - docs: `docs/errors/composition-problems.md` (row and nested-child JSON example), `docs/cli/data-contracts.md`, `docs/cli/validate.md`, repo skill `compose.md`
        - focused runs: darkmatter l1 2359/2359, lib 3848/3848, CLI l1 1016/1016, CLI bins+lib 239/239, claudine-cli 1057/1057, claudine 1346/1346, `cargo check -p claudine -p claudine-cli --tests` clean, `just lint` exit 0
- work completed for 'R12-F2 inclusion target errors lose authored file and line' at 20:50:30
- starting the work on 'R12-S10/S11 sweep remainders' at 20:50:30
        - decision R12-S10: make an invalid hyperlink/image target a located `ReferenceIssue` (its record already holds the authored position) rather than adding a second site wrapper to `ReferenceError`; the default check fails with status 1 and the report gains the authored location. A matching missing-target allowance changes the result in composition (R13-F3)
        - R12-S10 fixed: a link or image target whose file reference fails to parse is a located error-severity `ReferenceIssue` (new `ReferenceIssueCode::InvalidReference`, reason "invalid file reference"; an I/O failure while probing gives `UnreadableTarget`); the failure class is kept in the new `ReferenceIssue::failure` (`"failure": "invalid-reference"`); the code stays `dm.reference.failure`; `md validate refs`, `md graph --validate` and the default `md compose` exit 1 with `source.line`/`column` (`[x](@)` 6:1, `![y](@)` 8:1, inline `prologue` link 2:11); fresh, prebuilt, FileTree and compose-check validators agree. Tests: `reference_inclusion_targets::an_invalid_link_target_is_a_located_issue_in_every_validator`, CLI `validate_refs::an_invalid_link_target_is_reported_at_its_position`
                - sweep: `validate_local_path` was the only per-record hard `ReferenceError` for a link target; `graph.rs`/`local.rs` resolve only inclusion targets (located by R12-F2); an inclusion's invalid target still stops the run as composition does
        - R12-S12 fixed (found by R12-S10): when `md compose`'s reference check refused a document, warnings found while preparing it (e.g. `dm.expression.uncertain_call`) were dropped (already true for a plain missing link); `ReferenceValidationReport::earlier_warnings` plus `deliver_refused_check_warnings` in `compose.rs` and `compose/lifecycle.rs`. Tests: `an_unresolvable_link_refuses_the_run_and_keeps_the_earlier_warning` (rewritten from `a_reference_check_that_cannot_run_fails_the_run`), `a_refused_check_keeps_the_earlier_warning`
        - R12-S11 fixed: a `TransclusionError::Located` status block now starts with `File:` and, when proven, `Line:` rows (shared `site_scope::line_row`); needed a new public `biscuit-terminal` method `StatusBlock::prepend_body_line` (unit tested). Tests: new snapshot `located_file_reference`, `located_composition_failure_names_the_file_and_line`; no existing snapshot changed
        - fallback coverage restored: an 18-deep inclusion chain (limit 16) raises a depth error with no file; `compose_report_diagnostics::reference_commands_name_their_document_on_a_failure_with_no_file` asserts `md validate refs` and `md graph` name `n0.md` with null line/column
        - checked, outside the class: the depth-limit and cycle errors name only a file; they judge the whole chain rather than one target, so they are not a target failure that lost its site
        - focused runs: darkmatter l1 1213/1213, lib 470/470, CLI l1 275/275 and compose/lifecycle 853/853, darkmatter-lsp 398/398, biscuit-terminal status_block 45/45, claudine check clean, `just lint` in darkmatter and biscuit-terminal exit 0
- work completed for 'R12-S10/S11 sweep remainders' at 21:12:58
- starting phase gates at 21:12:58
- phase gates completed at 21:30:16
        - darkmatter `just test --max-fail all`: 18,273 passed, 10 skipped, none at or above 5 s; `just lint` exit 0
        - claudine `just test --max-fail all`: 13,077 passed, 16 skipped, none at or above 5 s; `just lint` exit 0
        - biscuit-terminal `just test --max-fail all`: 4,022 passed, 55 skipped, none at or above 5 s; `just lint` exit 0
        - `just check-tier-coverage` for darkmatter, claudine, biscuit-terminal: 0 stranded
        - cross-OS check not run: the changes move error sites and target classification and add no path comparison, `#[cfg]` code or OS-specific API; a filename containing `#` is legal on Windows, macOS and Linux, and the tests that use one only assert that it is missing

### Completion Report

- outcome: **complete**; elapsed 1 h 33 min (19:57:40 → 21:30)
- parent findings: total 2 = fixed 2 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: R12-F1.1–.4 fixed, R12-F1.5–.6 clean; R12-F2.1–.5 fixed, R12-F2.6 clean
- sweep rows: R12-S1, S2, S4–S13 fixed; R12-S3 and R12-S14 clean
- verified repairs:
        - R12-F1: graph construction keeps the whole local filename and the TOC selection outcome. Validation and the remote batch respect those decisions in the ledger’s tested cases. Markdown file-type and URL admission still differ from composition (R13-F1)
        - R12-F2: target failures carry an `InclusionSite` attached where the inclusion is known (body opener or frontmatter key/entry); the shared projection publishes file, line and column for the located target failures in the ledger. The ordinary nested child control retains its source. The TOC heading-read path still loses the source (R13-F2)
- open findings from the iteration 12 repair report: none for the original cases. Review 13 documents newly found siblings and a recovery regression.
- decisions recorded in the ledger (none meet the worth-the-author's-time bar, so `human-review` is unchanged): a required TOC failure takes the selector's verdict and is not resolved again; the site travels in one `TransclusionError::Located` wrapper; an invalid link target is a located issue rather than a hard error
- public API changes: `TransclusionError::Located`, `InclusionSite`, `ReferenceIssueCode::InvalidReference`, `ReferenceIssue::failure`, `StatusBlock::prepend_body_line` (biscuit-terminal); graph and validation target failures now return `ReferenceError::Compose(Transclusion(Located))` where they returned `ReferenceError::FileReference` (Claudine updated)
- changed files (this iteration):
        - biscuit-terminal: `lib/src/components/status_block.rs`
        - darkmatter library: `lib/src/markdown/reference/{graph.rs, mod.rs, validate.rs, types.rs, file_tree/model.rs}`, `lib/src/markdown/compose/{mod.rs, report_diagnostics.rs, policy.rs, pipeline/phases.rs, preflight/collect.rs}`, `lib/src/markdown/compose/transclusion/{types.rs, engine.rs, mod.rs}`, `lib/src/markdown/types.rs`
        - darkmatter CLI: `cli/src/commands/compose.rs`, `cli/src/commands/compose/lifecycle.rs`
        - claudine: `cli/src/commands/wrap/sequence/{mod.rs, tests.rs}`, `lib/src/composition/sequence/tests.rs`
        - tests: new `lib/tests/l1/{reference_inclusion_targets.rs, inclusion_target_sites.rs}`, new snapshot `l1__error_snapshots__transclusion__located_file_reference.snap`; updated `lib/tests/l1/{main.rs, file_tree_roots.rs, reference_integration.rs, reference_report.rs, error_snapshots/transclusion.rs}`, `lib/src/markdown/compose/tests/transclusion.rs`, `cli/tests/l1/{validate_refs.rs, compose_report_diagnostics.rs}`
        - docs and skills: `docs/cli/{validate.md, data-contracts.md}`, `docs/inline/link-validation.md`, `docs/transclusion/fm-transclusion.md`, `docs/errors/composition-problems.md`, `docs/structs/Markdown.md`, `.claude/skills/darkmatter/{SKILL.md, compose.md}`
- final verification: the gates listed above exercised the original repair cases. Focused runs for each repair are recorded under its progress entry. The review 13 probes cover additional cases that those runs did not exercise.

### Review 13 record corrections (2026-11-25)

- The target classification claim above was too broad. The rebuilt CLI and public API accepted an existing `./side.txt` as a Markdown child for `::file`, `prologue`, and `epilogue`, and validated `::url ./side.md` as a local path. Composition and preflight refused those targets. The original hash and TOC selection checks still pass.
- The location claim for unreadable children omitted the TOC heading-read path. The public transclusion-only graph located that error at the directive. The full graph and validation lost the source, and compose lost the line. The CLI reproduced this with a UTF-16 Markdown target named on line 11 of an included child and fell back to the input guide in the validating commands.
- The invalid-link verdict stayed unchanged under default options only. The rebuilt CLI exited 0 with a document for `[Guide](@)` under `--allow missing-hyperlinks` and for `![Guide](@)` under `--allow missing-image-refs`, including in lifecycle composition where the composed event ran. Without the allowance, both commands exited 1 with the located issue. The same result held for a reference whose path probe hit a permission error. Before the repair, the validator propagated the resolution error before the allowance could apply.

[Review 13](review-13.md)

## Implementation of Review Findings #13

> **started at:** 2026-11-25T21:38:40+00:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-22-expression-and-mutation-schemas/review-13.md'
- this is iteration 13 of the review-to-implement cycle 
- review `implemented` was already `false`; the review has no human-in-the-loop items and no blocked findings

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R13-F1 | Reference checks accept targets that the inclusion directive rejects | fixed | Graph and validation now call composition's own admission functions (`transclusion::admit_local_target`, `admit_url_target`) before opening a child, branching on target kind, or requesting. Children F1.1–F1.4 fixed, F1.5 clean, sibling R13-S1 fixed. `reference_inclusion_admission` 8/8; CLI 2 new tests. |
| R13-F1.1 | Body `::file ./side.txt` (bare and quoted): both graphs, flattening, fresh/prebuilt/FileTree validation and both validating CLI commands apply the Markdown-file admission rule; `./side.md` control passes; navigation lookup stays distinct | fixed | `a_file_inclusion_of_a_text_file_is_refused_at_its_directive`: compose, preflight, both graphs, flattening, fresh, prebuilt and FileTree (with and without following) each give one `dm.transclusion.failure` at `doc.md` line 7 (line 10 after a removed block), bare and quoted; before the fix the graph built with a child and validators passed. `./side.md` is clean everywhere. The local query still resolves `side.txt` (lookup). CLI `validating_commands_refuse_a_text_file_inclusion_at_its_directive`: compose, `validate refs`, `graph --validate` exit 1 with that record; human report shows `Line: 7`. |
| R13-F1.2 | Frontmatter `prologue`/`epilogue` `"./side.txt"`: same Markdown-file rule; inline Markdown text unchanged | fixed | `a_frontmatter_inclusion_of_a_text_file_is_refused_at_its_key`: each key, scalar at line 3 and list entry at line 5, gives `dm.transclusion.failure` at that line in all 9 readers (was: graph child, validators clean). `"./side.md"` and inline `"Some *inline* text"` are clean. |
| R13-F1.3 | Body `::url` with `./side.md`, `"./side.md"`, `"#here"`, scheme-relative and `mailto:` targets: URL syntax validated before hyperlink branches and remote batch; absolute URL passes; bare `@` keeps its located failure; raw query and nonvalidating graph still list authored targets | fixed | `a_url_inclusion_of_a_non_url_is_refused_before_any_request`: the four parse failures give `dm.transclusion.failure`, `mailto:` gives `dm.reference.failure` (composition's codes), at line 7/10 in fresh, prebuilt and both FileTree views, local and remote mode, with 0 loopback requests (was: pass, missing-file issue, or a HEAD request for the scheme-relative URL); graphs and flattening list the record as `RemoteUrl`. Absolute loopback URL: no issue, exactly 2 requests (fresh + prebuilt). `a_url_inclusion_of_an_incomplete_reference_keeps_its_failure`: `::url @` still `dm.reference.failure` at line 7; local query lists `./side.md`. CLI `validating_commands_refuse_a_relative_url_inclusion_without_a_request`: 0 requests for relative and scheme-relative, 1 for the absolute URL. |
| R13-F1.4 | `::code ./side.txt` passes; `::code ./dir` and `::file ./dir` keep the directory rejection; presence check needs no payload decode | fixed | `a_code_inclusion_admits_a_text_file_and_no_inclusion_admits_a_directory`: `::code ./side.txt` clean in all 9 readers; `::code ./dir` and `::file ./dir` refused by compose and `[MissingLocalTarget]` in fresh, prebuilt and both FileTree views (passed before and after: behavior preserved). The admission functions read no file; the code check runs on the resolved path only. |
| R13-F1.5 | Controls: TOC selection/suppression, URL-like file-list glob, ordinary fragment link, image reference, editor navigation keep their own rules | clean | `links_and_images_keep_their_own_target_rules` (`[Side](./side.md#side)`, `![Text](./side.txt)`, `[Plain](./side.txt)` clean with fragments on); the R12 TOC, file-list and link tests in `reference_inclusion_targets` pass; DMLS resolves paths in `dmls/src/context.rs` and reads no graph verdict, darkmatter-lsp `reference\|link` 220/220. |
| R13-S1 | Sibling (F1 class): a request with `with_allow_local_code(false)` refuses every local `::code` in composition; validators only checked existence. `with_allow_local_markdown(false)` for `::file` | fixed | `validate.rs::admit_code_inclusion` applies `admit_local_target(Code)`. `a_request_that_disallows_local_sources_is_refused_by_its_validators`: fresh and FileTree give composition's `dm.transclusion.failure` at line 7 for both options (the `::code` case failed before: validation passed). |
| R13-S2 | Sibling (F1 class): `md compose`'s reference check ahead of composition | fixed (decision) | Defers a refused target to composition, which refuses or tolerates it: `the_compose_reference_check_leaves_a_refused_target_to_composition` (no issue, no child, 0 requests; failed before the batch filter with a `RemoteUnreachable` builder error for `::url ./side.md`). |
| R13-S3 | Checked: preflight, TOC selection, `::file-links`, local query, FileTree model, CLI graph rendering, DMLS, Claudine | clean | Preflight already admits through `resolve_target`; TOC uses composition's selector; file-list skipped by `names_no_target`; local query is a lookup (rustdoc says so); FileTree follows only followable syntaxes; DMLS and Claudine call no graph/validation API. |
| R13-F2 | TOC target read errors lose the directive location | fixed | Both heading reads now return `transclusion::read_failure_at` (new; also the body of `target_failure_at`'s read branch) at the directive's `InclusionSite`: composition (`engine.rs` TOC branch, built from `directive_site(&span)`) and the full graph (`graph.rs::generate_toc_link_references` takes the site the child load already used, now built once before both). Children F2.1–F2.2 fixed, F2.3–F2.4 clean, siblings R13-S4–S6 clean. New `lib/tests/l1/toc_target_read_sites.rs` 3/3 (2 failed before the fix) and CLI `validate_refs::commands_report_an_unreadable_toc_target_at_its_directive` (failed before). |
| R13-F2.1 | Compose TOC heading read (root and nested, quoted and bare, before/after removed block): keep warning and `dm.input.read_failure`, add selected filename and directive opener | fixed | `composition_warns_at_the_toc_directive_that_selected_an_unreadable_file`: bare, quoted, and `"./absent.md \| ./utf16.md"`, each at line 8 and at line 11 after a removed block, root (`doc.md`) and nested (`guide.md` → `nested.md`): compose succeeds, keeps the `_Could not link headings_` notice, and gives one `dm.input.read_failure` warning at the file and line, column 1, whose message names `utf16.md` (before: `TOC linking error: stream did not contain valid UTF-8`, no line). CLI `md compose --output json` exits 0 with that warning at `nested.md` line 11. |
| R13-F2.2 | Full graph TOC heading extraction, flattening, fresh validation, FileTree, CLI graph with/without validation: read error attached to the directive, preserving hard failure and code | fixed | `graph_readers_fail_at_the_toc_directive_that_selected_an_unreadable_file`: reference graph, flattened, fresh validation, FileTree with and without following each fail with one `dm.input.read_failure` at the directive's file and line, column 1, naming `utf16.md`, for the same 6 shapes direct and nested (before: `Failed to load file: cannot read …` with no source line). Prebuilt validation cannot get a graph, so its error is the graph's. CLI `md validate refs`, `md graph --validate`, and `md graph` exit 1 with that record at `nested.md` line 11; the human report names `nested.md` and `utf16.md`. |
| R13-F2.3 | Controls: transclusion-only graph and ordinary `::file`/frontmatter reads keep their correct sites | clean | The transclusion-only graph is in the same reader list and passes (its located child load now shares the one site value); `readable_and_suppressed_toc_targets_add_no_read_diagnostic` checks `::file ./utf16.md` at line 11 in every graph reader (passed before and after); R12 `inclusion_target_sites` (`::file ./bad.md`, frontmatter keys) 8/8. |
| R13-F2.4 | Controls: preflight, authored query, target selection, required/suppressed TOC chains and valid UTF-8 target unchanged | clean | Pre-flight and the local query succeed for every unreadable shape (`collect.rs` resolves TOC targets and never reads them); `./side.md` and `"./absent.md \| false"` give no warning in compose and no error in any graph reader, and CLI control exits 0 with no diagnostics in all four commands; required `::toc-linking @` keeps its site (`a_required_toc_target_the_selector_rejects_names_its_directive_line` passes). |
| R13-S4 | Sibling (F2 class): `::code` source read (`render_code_transclusion`'s `fs::read`) and remote `::file`/`::code` fetches | clean | Already returned through `target_failure_at(…, inclusion)` / `at_inclusion(site)` at the directive (`engine.rs` code branch and remote fetch); `an_unreadable_target_names_its_directive_line` and `a_refused_remote_target_names_its_directive_line` pass. |
| R13-S5 | Sibling (F2 class): `::file-links` directory reads | clean | `FileLinksError::Unreadable` carries `line`/`authored` and `phases.rs::at_directive_site` places it at the opener; the graph never reads a listing (`names_no_target`). No other path raises `FileLinksError::Io`. |
| R13-S6 | Sibling (F2 class): fragment validation's read of a linked file, DMLS, Claudine | clean | `validate_cross_doc_fragment` reports `UnreadableTarget` at the link's own `origin`; DMLS reads workspace files through its own index and calls no TOC heading or graph read; Claudine matches `TocLinkingError` only for an identity document (`prepare.rs`), which the I/O variant never named. |
| R13-F3 | Missing-target allowances let invalid links and images pass composition | fixed | One predicate, `ReferenceIssue::is_missing_target` (failure class `NoMatch`), now gates `compose.rs::recovers_missing` before the kind's allowance; `check_references` is the one decision both the direct and the lifecycle route call. Children F3.1–F3.3 fixed, F3.4–F3.5 clean, siblings R13-S7 fixed, R13-S8–S9 clean. New CLI `compose_missing_target_recovery` 17/17 (9 fail with the predicate bypassed) and lib `reference_missing_target_class` 10/10. |
| R13-F3.1 | Direct compose: `[Guide](@)`, `[Guide](<@>)`, `![Guide](@)`, inline frontmatter link/image refuse before emitting even with matching allowance; location kept | fixed | `compose_missing_target_recovery::{an_invalid_link_target…, an_angle_bracketed_invalid_link_target…, an_invalid_image_target…, an_invalid_html_anchor_target…, an_invalid_prologue_link…, an_invalid_epilogue_image…}_refuses_under_its_allowance`: with no allowance, the matching one, and `--allow all`, JSON and Markdown runs exit 1, `document` null, stdout empty, one error `dm.reference.failure` with `failure: invalid-reference` at `doc.md` line 4 (body) or line 3 (`prologue`/`epilogue` key). Each failed before the fix (exit 0 with the document). |
| R13-F3.2 | I/O failure (untraversable directory) for link and image refuses composition under the allowance | fixed | `#[cfg(unix)]` `an_unreadable_{link,image}_target_refuses_under_its_allowance` (`locked/` at mode 000, skipped only when a probe `metadata` read still succeeds): exit 1, no document, `failure: io`, line kept, both routes; both failed before the fix. Lib `an_unreadable_link_target_is_not_missing`: `UnreadableTarget`, error, `Io`, not missing (ran for real, uid 1000). |
| R13-F3.3 | Lifecycle compose refuses at the shared check before emitting the document or dispatching `composed` | fixed | Every refusal and unreadable case above also runs with `composed: [{stdout: done}]`: exit 1, `document` null, `lifecycle_output` `[]` (no `done`), line shifted by the event's frontmatter lines; inline `prologue`/`epilogue` cases included. Controls in the same route record `done`. Before the fix the lifecycle run returned the document and `done`. |
| R13-F3.4 | Controls: validating CLI and public report consumers keep error verdict and failure class | clean | No consumer outside `check_references` maps an allowance to an issue (`md validate refs`/`md graph --validate` take no `--allow`; `validate.rs`, `report.rs`, FileTree read severity only). `validate_refs::an_invalid_link_target_is_reported_at_its_position` (validate refs, graph --validate, compose: exit 1, `invalid-reference` at line 3/5) passes; lib class tests pin `InvalidReference`/`UnreadableTarget`/`InvalidUrl` with their classes. |
| R13-F3.5 | Controls: missing link/image recover, empty link warns, malformed inclusion targets fail under `missing-transclusions`, missing `::file` recovers, suppressed TOC clean | clean | `an_absent_{link,image,inclusion}_target_recovers_under_its_allowance` (exit 0, document kept, `dm.reference.missing_*`, lifecycle `done`), `an_empty_link_target_warns` (exit 0, no allowance), `a_malformed_{inclusion,code,url}_target_refuses_under_its_allowance` (`::file @`/`::code @`/`::url @` exit 1 under `missing-transclusions` and `all`), `a_suppressed_toc_chain_stays_clean` (no diagnostics); all passed before and after. `policy_verbs` and `compose_report_diagnostics` pass. |
| R13-S7 | Sibling (F3 class): a malformed hyperlink URL (`[Guide](http://[bad)`, `<a href="http://[bad">`), an `InvalidUrl` error issue with no failure class, recovered under `missing-hyperlinks` | fixed | Same predicate: `InvalidUrl` has no `NoMatch` class. `a_malformed_link_url_refuses_under_its_allowance` exit 1 in both routes (failed before: exit 0 with the document); lib `a_malformed_link_url_is_not_missing`. |
| R13-S8 | Sibling (F3 class): library recovery by `ComposeOptions::with_allow_missing_transclusions` (transclusion engine and preflight) | clean | Both already gate on `err.resolution_failure() == Some(NoMatch)` (`removes_missing` in `transclusion/engine.rs` and `preflight/collect.rs`); the CLI rule now uses the same class test. The deny path (`check_references`' blocking code) and `report_diagnostics::reference_issue_diagnostic` only project codes, never grant recovery. |
| R13-S9 | Sibling (F3 class): Claudine compose/sequence, DMLS | clean | No Claudine or DMLS source calls reference validation or sets a missing-target allowance (`grep` of `claudine/` and `dmls/src` for `validate_references`, `allow_missing`, `missing-hyperlinks`, `missing_reference_code`: none); Claudine sets only `with_allow_ctx_override`/`with_allow_remote_transclusion`. |

### Progress

- starting the work on 'R13-F1 inclusion target admission' at 21:39:48
        - defect class: an inclusion target is treated as usable before its directive's admission rule is applied, so a reference check passes a target composition refuses
        - shared rule: the resolver's passive admission checks are now two crate functions that composition itself calls, `transclusion::admit_local_target` (`::file`/`prologue`/`epilogue`: Markdown extension only and `allow_local_markdown`; `::code`: `allow_local_code`) and `transclusion::admit_url_target` (`::url`: `url::Url::parse`, an absolute URL); neither reads a file, decodes a code payload, or touches the network
        - graph (both modes, so flattening, fresh/prebuilt validation and FileTree too): `graph.rs::admits_markdown_child` applies `admit_local_target(File)` to the canonical path after local resolution and before the cycle/depth check and the load, for body `::file`, `prologue` and `epilogue`; a refusal is composition's `UnsupportedFileType` wrapped by `TransclusionError::at_inclusion` at the directive or key
        - `::url` record target is now always `RemoteUrl` (it was `classify_target`, so `./side.md` was a `LocalPath` and `#here` a `Fragment`); `validate.rs::admit_url_inclusion` (file-reference parse, then `admit_url_target`) runs before every target-kind branch, and the remote batch filters on it, so a refused URL is never resolved as a file or requested
        - decision: a refused target is a hard error with composition's code and site (as `::file @` already is), not a `ReferenceIssue`; alternative (an issue such as `InvalidUrl`) would give a different code from composition's
        - decision: `md compose`'s reference check (`defer_inclusion_parse_failures`) leaves a refused target to composition, which rejects it or tolerates it with a notice, like an unloadable child: no child is opened, the `::url` record counts valid, nothing is requested
        - decision: the local query `transclusions_in` is the navigation-only lookup the review asks to keep distinct; it still reports where `./side.txt` resolves (rustdoc of `transclusions_with_options` now says so). Raw query and the non-validating graphs keep listing authored `::url` targets
        - sibling R13-S1 fixed: a request with `ComposeOptions::with_allow_local_code(false)` refuses every local `::code` source in composition, while validation only checked existence; `validate.rs::admit_code_inclusion` applies `admit_local_target(Code)` to the resolved path (no read). `with_allow_local_markdown(false)` for `::file` is covered by the graph rule
        - sweep clean: preflight (`collect.rs` calls `resolve_target`, which already admits); TOC chain selection (graph calls composition's `resolve_target_chain`); `::file-links` (`names_no_target` before the URL check); FileTree model (`::url` is not followable, groups only hyperlinks by target kind); DMLS (`dmls/src/context.rs` resolves paths itself and reads no graph record or verdict); CLI `md graph` renders graph records only; Claudine calls none of the graph/validation APIs (`claudine/lib/src/render/prompt/tokens.rs` only names FileTree in a comment), so no Claudine check was needed
        - tests: new `lib/tests/l1/reference_inclusion_admission.rs` (8 tests); before the fix 3 of the first 6 failed (`a_file_inclusion_of_a_text_file_is_refused_at_its_directive`, `a_frontmatter_inclusion_of_a_text_file_is_refused_at_its_key`, `a_url_inclusion_of_a_non_url_is_refused_before_any_request`), the sibling test failed before `admit_code_inclusion`, and with only `admits_markdown_child` neutralized 4 of 8 fail; CLI `validate_refs::{validating_commands_refuse_a_text_file_inclusion_at_its_directive, validating_commands_refuse_a_relative_url_inclusion_without_a_request}`
        - docs: `docs/cli/validate.md` (admission table and example), `docs/inline/link-validation.md`, `docs/transclusion/fm-transclusion.md`, `docs/errors/composition-problems.md`; repo skill `.claude/skills/darkmatter/SKILL.md`
        - focused runs: darkmatter l1 `reference|inclusion|toc|transclusion|preflight|file_tree|graph|parity|diagnostic|remote` 1392/1392; lib unit `reference|transclusion|graph|resolver` 580/580; CLI l1 `validate_refs|graph|compose_report` 110/110; darkmatter-lsp `reference|link` 220/220; `just lint` exit 0; no test at or above 5 s
- work completed for 'R13-F1 inclusion target admission' at 21:54:30
- starting the work on 'R13-F2 TOC target read locations' at 21:55:34
        - defect class: the authored directive site is lost when a file a directive selected fails to read on a path separate from the located child load
        - shared rule: every read failure of a selected target is `transclusion::read_failure_at(io, path, site)`, a `TransclusionError::Io` naming the file (code `dm.input.read_failure`) wrapped by `at_inclusion` at the directive's `InclusionSite`; `target_failure_at`'s read branch now calls it, so the child load, composition's TOC heading read, and the graph's TOC heading read use one construction
        - composition: the TOC branch of `resolve_prepared_transclusion` builds the site with `InclusionSite::of_directive(self.markdown, &self.directive_site(&span))` (the same body origin `phases.rs::at_directive_site` uses for TOC errors) only when the read fails; the tolerated warning keeps `dm.input.read_failure` through `tolerated_failure_warning`'s located path, now with opener line/column and the selected path in the message
        - graph: the TOC branch builds the directive's site once, before the heading read, and passes it to `generate_toc_link_references`; both it and `load_inclusion` go through the new `graph.rs::read_failure` (site → `target_failure_at`, none → the old file-named `FileLoad`), so a document with no file keeps its previous error
        - decision: report the read as the transclusion I/O failure (as the `::file`/`::code` child load does) rather than extend `TocLinkingError::Io` with a site; alternative would add a site field to a variant that no other path locates and give the TOC read a second location rule
        - decision: the no-site case (a document with no file) in composition is now an unlocated `TransclusionError::Io` naming the file; the code is unchanged, and no Claudine match reads the old variant
        - sweep: `::code` read and remote fetches (R13-S4), `::file-links` directory reads (R13-S5), fragment validation, DMLS, Claudine (R13-S6) clean; pre-flight never reads TOC headings (`collect.rs`)
        - tests: new `lib/tests/l1/toc_target_read_sites.rs` (3 tests; with the two `map_err` sites reverted, the composition and graph tests fail and the control passes); CLI `validate_refs::commands_report_an_unreadable_toc_target_at_its_directive` (fails with the composition site reverted); UTF-16 fixture bytes written in the temp dir
        - docs: `docs/cli/validate.md` (unreadable `::toc-linking` target), `docs/errors/composition-problems.md` (location table row); repo skill `.claude/skills/darkmatter/SKILL.md`
        - focused runs: darkmatter l1 `reference|inclusion|toc|transclusion|preflight|file_tree|graph|diagnostic|snapshot|provenance|directive_authored_lines|typed_diagnostic_codes` 1477/1477; lib unit `toc|transclusion|reference|graph` 702/702; CLI l1 `validate_refs|graph|compose_report|toc` 136/136; `just lint` exit 0; new tests each under 0.5 s
- work completed for 'R13-F2 TOC target read locations' at 22:05:40
- starting the work on 'R13-F3 recovery by failure class' at 22:08:42
        - defect class: recovery is granted by reference kind without checking that the failure class is a missing target, so any error-severity link, image, or inclusion issue passed under its kind's allowance
        - shared rule: new `ReferenceIssue::is_missing_target()` in `lib/src/markdown/reference/validate.rs`, true only when `resolution_failure()` is `NoMatch` (a well-formed target naming no file); `cli/src/commands/compose.rs::recovers_missing` now takes the issue and returns false unless it holds, then applies the kind's allowance; `check_references` is the one decision for the direct route and `compose/lifecycle.rs::try_prepare`, so both refuse before writing a document or dispatching `composed`
        - sweep: library `allow_missing_transclusions` (engine and preflight) already gated on `NoMatch` (R13-S8); the deny path and diagnostic code projection grant nothing; `md validate refs`/`md graph --validate` take no allowance (F3.4); Claudine and DMLS apply no reference allowance (R13-S9); sibling found and fixed: a malformed hyperlink URL (`InvalidUrl`, no class) was recovered by `missing-hyperlinks` (R13-S7)
        - decision: the predicate reads the failure class, not `ReferenceIssueCode`, matching the library's transclusion recovery test; alternative (a code match on `MissingLocalTarget`) is equivalent today but would admit any future code that reuses the kind
        - decision: `MissingFragmentTarget` is not a missing target for recovery (no class); composition's check does not validate fragments, so no behavior changes
        - decision: an empty target stays a warning judged by the existing deny path; it is not routed through the predicate
        - tests: new CLI `cli/tests/l1/compose_missing_target_recovery.rs` (17 tests, one per case, each running direct and lifecycle; 9 refusal cases fail with the predicate bypassed, the 8 controls pass either way) and lib `lib/tests/l1/reference_missing_target_class.rs` (10 tests over the predicate); Unix permission cases skip only when a probe read shows permissions are not enforced
        - docs: `docs/cli/policy.md` (absent-only scope table and example), `docs/cli/compose.md` (allowance list and `--allow` bullet), caller skill source `docs/use-skills/use-darkmatter/cli.md` (not republished); repo skill `.claude/skills/darkmatter/SKILL.md`; `docs/cli/validate.md` and `docs/inline/link-validation.md` describe no allowance (unchanged)
        - focused runs: CLI l1 `compose|validate_refs|graph|allow|deny|lifecycle|policy` 870/870; CLI lib+bins `compose` 35/35; darkmatter l1 `reference|policy|inclusion|toc_target_read_sites|diagnostic` 1105/1105; darkmatter lib `reference|policy` 464/464; `just lint` exit 0; new tests each under 1 s
- work completed for 'R13-F3 recovery by failure class' at 22:14:37
