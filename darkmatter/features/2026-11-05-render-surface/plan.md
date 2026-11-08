---
total_phases: 7
created: 2026-11-06
phase: 6
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
- darkmatter
- darkmatter-cli
- darkmatter-lsp
source_files_during_phase_2:
- darkmatter/lib/src/style/apply.rs
- darkmatter/lib/src/style/bespoke.rs
- darkmatter/lib/src/style/coverage_tests.rs
- darkmatter/lib/src/style/descriptor.rs
- darkmatter/lib/src/style/error.rs
- darkmatter/lib/src/style/layer.rs
- darkmatter/lib/src/style/length.rs
- darkmatter/lib/src/style/mod.rs
- darkmatter/lib/src/style/normalize.rs
- darkmatter/lib/src/style/parse.rs
- darkmatter/lib/src/style/presentation.rs
- darkmatter/lib/src/style/schema/common.rs
- darkmatter/lib/src/style/schema/components.rs
- darkmatter/lib/src/style/schema/hr.rs
- darkmatter/lib/src/style/schema/mod.rs
- darkmatter/lib/src/style/schema/page.rs
- darkmatter/lib/src/style/walker.rs
- darkmatter/lib/src/style/warning.rs
- darkmatter/lib/src/layout/page.rs
- darkmatter/lib/src/markdown/render_tree/structural_gate.rs
- darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
- darkmatter/lib/benches/render_pipeline_steps.rs
- darkmatter/schemas/partials/style.yaml
- darkmatter/cli/src/args/cli.rs
- darkmatter/cli/src/artifact.rs
- darkmatter/cli/src/commands/clean.rs
- darkmatter/cli/src/commands/frontmatter.rs
- darkmatter/cli/src/commands/mod.rs
- darkmatter/cli/src/commands/render.rs
- darkmatter/cli/src/io/mod.rs
- darkmatter/cli/src/render.rs
- darkmatter/dmls/src/diagnostics/frontmatter.rs
- darkmatter/lib/tests/l1/base_schema_end_to_end.rs
- darkmatter/lib/tests/l1/context_construction_guard.rs
- darkmatter/lib/tests/l1/cutover_reference.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/meta_schema_phase6.rs
- darkmatter/lib/tests/l1/style_effective_lowering.rs
- darkmatter/lib/tests/l1/style_frontmatter_parity.rs
- darkmatter/lib/tests/l1/style_frontmatter.rs
- darkmatter/lib/tests/l1/style_layers.rs
- darkmatter/lib/tests/l1/tree_features_characterization.rs
- darkmatter/lib/tests/level2/level2_render_tree_terminal/layout_policy.rs
- darkmatter/lib/tests/level2/level2_render_tree_terminal/support/mod.rs
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
- darkmatter/cli/tests/l1/feature_flag.rs
- darkmatter/cli/tests/l1/layout_fill.rs
- darkmatter/cli/tests/l1/layout_style_frontmatter.rs
- darkmatter/cli/tests/l1/mermaid_features.rs
- darkmatter/cli/tests/level2/level2_frontmatter_tables.rs
- darkmatter/example-docs/rendering/code-blocks.md
- darkmatter/example-docs/rendering/style-prop.md
docs_updated_during_phase_2:
- darkmatter/docs/topics/layouts-and-style.md
- darkmatter/docs/rendering/style.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/rendering.md
source_files_during_phase_3:
- darkmatter/cli/src/args/render.rs
- darkmatter/cli/src/args/render/tests.rs
- darkmatter/cli/src/args/render_prepare.rs
- darkmatter/cli/src/args/mod.rs
- darkmatter/cli/src/args/cli.rs
- darkmatter/cli/src/args/target.rs
- darkmatter/cli/src/args/path_completion.rs
- darkmatter/cli/src/commands/mod.rs
- darkmatter/cli/src/commands/render.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/src/render.rs
- darkmatter/cli/src/render_style.rs
- darkmatter/cli/src/main.rs
- darkmatter/cli/src/lib.rs
- darkmatter/cli/src/output/report.rs
- darkmatter/lib/src/markdown/compose/expression/data/parse.rs
- darkmatter/lib/src/markdown/compose/expression/data/mod.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/render_style_files.rs
- darkmatter/cli/tests/l1/feature_flag.rs
- darkmatter/cli/tests/l1/help.rs
- darkmatter/cli/tests/l1/mermaid_features.rs
- darkmatter/cli/tests/l1/output_envelope.rs
- darkmatter/cli/tests/l1/render_basic.rs
- darkmatter/cli/tests/level2/level2_code_block_styling.rs
docs_updated_during_phase_3:
- darkmatter/docs/cli/render.md
- darkmatter/docs/cli/code-block.md
- darkmatter/docs/rendering/code-highlighting.md
- darkmatter/docs/rendering/style.md
- darkmatter/docs/topics/layouts-and-style.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/cli.md
source_files_during_phase_4:
- darkmatter/cli/src/args/cli.rs
- darkmatter/cli/src/args/command.rs
- darkmatter/cli/src/args/completion.rs
- darkmatter/cli/src/args/mod.rs
- darkmatter/cli/src/args/output.rs
- darkmatter/cli/src/args/path_completion.rs
- darkmatter/cli/src/args/render.rs
- darkmatter/cli/src/args/render_prepare.rs
- darkmatter/cli/src/artifact.rs
- darkmatter/cli/src/commands/clean.rs
- darkmatter/cli/src/commands/frontmatter.rs
- darkmatter/cli/src/commands/list_themes.rs
- darkmatter/cli/src/commands/mod.rs
- darkmatter/cli/src/commands/render.rs
- darkmatter/cli/src/commands/schema/validate.rs
- darkmatter/cli/src/io/mod.rs
- darkmatter/cli/src/main.rs
- darkmatter/cli/src/output/command.rs
- darkmatter/cli/src/output/mod.rs
- darkmatter/cli/src/output/report.rs
- darkmatter/cli/src/render.rs
- darkmatter/cli/src/render_style.rs
- darkmatter/cli/src/style_claims.rs
- darkmatter/cli/schemas/output/list-themes.yaml
- darkmatter/lib/src/markdown/schemas/completion.rs
- darkmatter/cli/tests/l1/clean.rs
- darkmatter/cli/tests/l1/clean_fidelity.rs
- darkmatter/cli/tests/l1/clean_schema.rs
- darkmatter/cli/tests/l1/compose_report_diagnostics.rs
- darkmatter/cli/tests/l1/feature_flag.rs
- darkmatter/cli/tests/l1/help.rs
- darkmatter/cli/tests/l1/layout_fill.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/mermaid_features.rs
- darkmatter/cli/tests/l1/output_commands.rs
- darkmatter/cli/tests/l1/path_completion.rs
- darkmatter/cli/tests/l1/render_dispatch.rs
- darkmatter/cli/tests/level2/level2_path_completion.rs
docs_updated_during_phase_4:
- darkmatter/cli/README.md
- darkmatter/docs/cli/clean.md
- darkmatter/docs/cli/delta.md
- darkmatter/docs/cli/index.md
- darkmatter/docs/cli/output-and-width.md
- darkmatter/docs/cli/render.md
- darkmatter/docs/rendering/mermaid.md
- darkmatter/docs/rendering/style.md
- darkmatter/docs/topics/layouts-and-style.md
docs_created_during_phase_4:
- darkmatter/docs/cli/list-themes.md
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/cleanup.md
- .claude/skills/darkmatter/cli.md
source_files_during_phase_5:
- darkmatter/cli/src/args/cli.rs
- darkmatter/cli/src/args/completion.rs
- darkmatter/cli/src/args/mod.rs
- darkmatter/cli/src/args/parsers.rs
- darkmatter/cli/src/args/render.rs
- darkmatter/cli/src/args/render/tests.rs
- darkmatter/cli/src/args/render_prepare.rs
- darkmatter/cli/src/args/wrappers.rs
- darkmatter/cli/src/commands/render.rs
- darkmatter/cli/src/lib.rs
- darkmatter/cli/src/render.rs
- darkmatter/cli/src/style_claims.rs
- darkmatter/cli/tests/common/mod.rs
- darkmatter/cli/tests/common/style.rs
- darkmatter/cli/tests/l1/code_block.rs
- darkmatter/cli/tests/l1/feature_flag.rs
- darkmatter/cli/tests/l1/help.rs
- darkmatter/cli/tests/l1/layout_alignment.rs
- darkmatter/cli/tests/l1/layout_fill.rs
- darkmatter/cli/tests/l1/layout_flags.rs
- darkmatter/cli/tests/l1/layout_style_frontmatter.rs
- darkmatter/cli/tests/l1/main.rs
- darkmatter/cli/tests/l1/mermaid_features.rs
- darkmatter/cli/tests/l1/output_envelope.rs
- darkmatter/cli/tests/l1/render_alias_parity.rs
- darkmatter/cli/tests/l1/render_basic.rs
- darkmatter/cli/tests/l1/render_style_matrix.rs
- darkmatter/cli/tests/l1/render_style_wiring.rs
- darkmatter/cli/tests/level2/level2_code_block_styling.rs
- darkmatter/cli/tests/level2/level2_disclosure_blocks.rs
- darkmatter/cli/tests/level2/level2_frontmatter_images.rs
- darkmatter/cli/tests/level2/level2_frontmatter_tables.rs
- darkmatter/cli/tests/level2/level2_horizontal_rules.rs
- darkmatter/cli/tests/level2/level2_layout_dimensions.rs
- darkmatter/cli/tests/level2/level2_ordered_lists.rs
- darkmatter/cli/tests/level2/level2_render_no_input.rs
- darkmatter/cli/tests/level2/main.rs
- darkmatter/lib/benches/render_pipeline_steps.rs
- darkmatter/lib/src/layout/error.rs
- darkmatter/lib/src/layout/page/tests.rs
- darkmatter/lib/src/markdown/render_tree/structural_gate.rs
- darkmatter/lib/src/markdown/render_tree/style_tree_parity_tests.rs
- darkmatter/lib/src/style/apply.rs
- darkmatter/lib/src/style/bespoke.rs
- darkmatter/lib/src/style/cli_claims.rs
- darkmatter/lib/src/style/coverage_tests.rs
- darkmatter/lib/src/style/descriptor.rs
- darkmatter/lib/src/style/layer.rs
- darkmatter/lib/src/style/mod.rs
- darkmatter/lib/tests/browser/browser_render.rs
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-inline.json
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-root-union.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-baseline-schema-file.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-inline-compose.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/effective-trigger-with-document-schema.digest
- darkmatter/lib/tests/fixtures/recursive_schema_golden/golden/embedded-darkmatter-base.json
- darkmatter/lib/tests/l1/cutover_reference.rs
- darkmatter/lib/tests/l1/disclosure_render_targets.rs
- darkmatter/lib/tests/l1/horizontal_rule_integration.rs
- darkmatter/lib/tests/l1/layout_snapshots.rs
- darkmatter/lib/tests/l1/main.rs
- darkmatter/lib/tests/l1/style_frontmatter.rs
- darkmatter/lib/tests/l1/style_frontmatter_parity.rs
- darkmatter/lib/tests/l1/style_layer_matrix.rs
- darkmatter/lib/tests/l1/tree_features_characterization.rs
- darkmatter/lib/tests/level2/level2_render_tree_terminal/layout_policy.rs
- darkmatter/schemas/partials/style.yaml
docs_updated_during_phase_5:
- darkmatter/docs/cli/developing.md
- darkmatter/docs/cli/output-and-width.md
- darkmatter/docs/cli/render.md
- darkmatter/docs/rendering/code-highlighting.md
- darkmatter/docs/rendering/mermaid.md
- darkmatter/docs/rendering/style.md
- darkmatter/docs/topics/layouts-and-style.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/SKILL.md
- .claude/skills/darkmatter/cli.md
- .claude/skills/darkmatter/library-surfaces.md
- .claude/skills/darkmatter/rendering.md
---

# Render Surface Implementation Plan

## Summary and Definition of Done

Make implicit rendering and `md render` share one argument definition, one preparation path, and one handler. Replace render presentation flags with schema-driven `style.*=VALUE` setters and repeatable local `--style FILE` inputs. Normalize sparse style layers in the library, preserve each winning file-valued field's origin, and lower the merged typed style through the existing render tree and page frame. Add `md list-themes`, retain command-local options, and migrate tests and documentation.

Completion means both render spellings produce equal prepared values, diagnostics, status, and bytes under a captured environment; invalid styles fail before output or artifact opening on every target; source output never contains CLI styles; document styles retain their warning policy; and every retained layout behavior has coverage through the new surface. All removed flags and aliases fail with ordinary clap unknown-argument errors. The old claim structures and their override arguments are gone, including their test and benchmark callers. Darkmatter L1, affected L2, headless browser coverage, lint, and example/benchmark compilation pass. Required dependency work completes first, without a human gate. No performance experiment, new CI matrix, commit, branch, worktree, or lifecycle-directory move is part of this plan.

Phases run in order. Wave numbers are unique across this plan. Tasks within a wave marked parallel may use separate agents with the listed file ownership; every agent receives the non-interactive constraint. A wave completes and integrates before its successor starts. Record implementation evidence and spec departures in `implementation-log.md` in this directory; human review happens after Phase 7 and never interrupts execution.

## Phase 1 — Establish Shared Contracts

### Necessary Rules

1. **Complete the dependencies in their existing order.** `2026-11-04-composition-errors` is already reflected in the current output implementation. Complete the remaining tasks of `2026-11-06-dotted-paths`, then `2026-11-04-loading-data`, using their existing plans before starting Phase 2. Both specs currently say `implemented: false`; `simplified-schema/lib/src/property_path.rs` and `setter_value.rs` do not exist, `compose.rs::parse_shorthand_value` is still private primitive parsing, and format detection is not yet the promised public surface. These are implementation dependencies, not spikes or author rulings. Keep their feature directories active; dependency plan instructions to update progress do not authorize moving directories or committing.

2. **Use one clap `Args` type and preserve the terminator structurally.** Keep `RenderOptions` in `cli/src/args/render.rs`, flattened in `Cli` and held by `Command::Render`. Replace `input` with `args: Vec<OsString>` and `literal_args: Vec<OsString>`; the latter is a separate `last = true` positional populated only after `--`. Both use `num_args = 0..`; retain root `subcommand_precedence_over_arg = true`. A shared `classify_render_args` produces `RenderInput { input: Option<PathBuf>, setters: Vec<RawStyleSetter> }`; classify only `args`, and always treat `literal_args` as input. A recognized setter has Unicode text before its first `=` equal to `style` or beginning `style.`. Preserve native tokens otherwise, split values only once, reject multiple input tokens with exit 2, and suggest `style.*` for an unresolved input containing both `.` and `=`. Evidence: the spec's setter classification and terminator requirements; `args/cli.rs` already gives clap precedence over positionals. Do not scan argv for command words or insert `render` into argv.

3. **Do not silently reinterpret non-Unicode paths.** Keep file arguments as `OsString`/`PathBuf` through clap and classification. At `io::parse_argument`, use checked text conversion and report a typed invalid-reference failure for an unrepresentable file reference rather than opening its lossy replacement. This is a run error inside the report boundary. Evidence: `FileReference::new` takes `&str`, and current `io::parse_argument` uses `to_string_lossy`; the new positional classifier must not broaden that loss. No new native-path bypass of `FileReference` is introduced.

4. **Reject every explicitly supplied root render field before a subcommand.** Remove all defaults from render option fields: use `Option` for scalar options, empty vectors for repeated options, and `false` for boolean switches; resolve actual defaults after dispatch. Add `RenderOptions::has_explicit_arguments` covering both positional vectors, `--output`, `--show`, `--indent`, `--deny`, `--feature`, `--mermaid`, and `--style`. `validate_subcommand_usage` rejects any such root value with exit 2. `command_or_render` merely selects the explicit command or clones the shared render options; remove feature splicing. Evidence: current `commands/mod.rs::validate_subcommand_usage` omits features, and `Cli::command_or_render` merges them. Generated parity and root-conflict tests enumerate the clap definition so a later option cannot escape this rule.

5. **Shared controls retain placement.** Keep width, verbosity, and debug shared; magic roots remain repeatable root-only launch configuration; completion generation remains a root action. Render gets only `--style`, setters, `--mermaid`, and the retained behavior/output fields. Remove the entire root presentation inventory, root `save`, and `list_themes`. Do not add their spellings to `main.rs::REMOVED_FLAGS`. Move `--code-block` onto `SchemaTarget::About` with default `inverse`; `CodeBlock --theme`, `Clean --save`, and `Set --save` retain their definitions. Evidence: the spec's placement and removal boundaries and `commands/mod.rs`'s `SchemaTarget::About` dispatch.

6. **Resolve output once and isolate the handler by type.** Introduce `PreparedRenderOptions` in `cli/src/commands/render.rs`, holding classified input/setters, ordered file references, resolved policy, `show`, `indent`, `mermaid`, and features. Dispatch resolves `ResolvedOutput` once using the captured `OutputEnvironment`; `run_render(&PreparedRenderOptions, &ResolvedOutput, &MdRequest) -> Result<()>` takes no `Cli`. Keep `source_markdown_width` in that preparation. Page builders receive prepared style, behavior, output options, and request-derived origins, never `Cli`. Construct terminal state through `Terminal::from_output_options`; no downstream `Terminal::default` or theme host detection. Evidence: `commands/render.rs::{run_render,Render::write,source_markdown_width}`, `render.rs::ResolvedTheme::from_cli`, and `artifact.rs::html_page` currently recapture these inputs.

7. **Own merge semantics in `darkmatter::style`.** Add `style/layer.rs` with public `StyleLayer`, `StyleOrigin`, `PreparedStyle`, and `prepare_style_layers`; add `style/normalize.rs` for normalization. `StyleLayer` carries a raw object, source kind (`Document`, `File`, `Setters`), and per-path origins. `StyleOrigin` carries the written argument or document identity, canonical property path, optional source span, and a request-derived `FileResolutionContext`. `PreparedStyle` carries the schema-admitted merged value, the built-in rendering projection as `StyleFrontmatter`, winning origins, and diagnostics. Pass the already-selected `EffectiveSchema` and explicit target-validation policy to preparation; the library does not read CLI arguments, process state, or style files. Evidence: `style/schema/mod.rs::StyleFrontmatter` lacks cleanup fields, while `ComponentBorder::deserialize` materializes defaults. Raw layers must survive until after the merge.

8. **Normalize each source before cross-source merging.** Page keys at the top level alias `page.*`. For margin/padding, expand all-sides, axis pairs, then individual sides in that order. Add page shorthands both at the top level and under `page`. Component `padding` expands before `padding-left`/`padding-right`. Normalize alignment into page alignment and all ten `PageComponent::ALL` buckets; a same-layer component alignment wins. Preserve the page's own alignment as well. For HR broadcast values use `left`, `center`, and `right`, retaining direct HR `full`. Reject different explicit spellings of one specific field, including nested-object versus dotted equivalents; broad-plus-specific is valid. Merge defaults < document < each file in occurrence order < setters. Evidence: spec precedence examples and `layout/types.rs::PageComponent::ALL`; `style/cli_claims.rs::apply_cli_claims` currently broadcasts to those ten categories.

9. **Setters are one layer with their own overlap rules.** Reuse dependency `PropertyPath`, `SetterValue`, and declared-type binding. Collapse repeated exact decoded paths to their last occurrence before conversion; repeating `style` replaces that entire assignment. Apply surviving ancestors before descendants, then shorthand specificity. Distinct paths normalizing to the same explicit field fail. Descendants can select a compound style union's object arm after a scalar ancestor, including a border; keep this style-only behavior out of the shared property editor and compose setter traversal. `page.meta` and other open objects accept whole-object values, without arbitrary descendant setters. Deprecated key aliases have no setter or completion. Evidence: the spec's compound/repeated setter rules differ intentionally from the dotted-path dependency's scalar-parent rejection.

10. **Keep sparse values sparse through validation.** CLI roots must be objects; null leaves disappear before merge, while zero and false remain explicit. Objects merge recursively, arrays/scalars replace, and a scalar/object union shape change replaces that union value. Reject `width` and `max-width` together in an individual layer; setting either in a later layer removes the inherited other. Validate every file independently even when a later file replaces it. Validate surviving setters independently before the combined effective check. Deserialize the built-in projection once after merge; do not let border defaults overwrite prior color or weight. Evidence: spec sparse-border and cross-field sections, `schema/common.rs::ComponentBorder`, and width conflicts in `apply.rs` and `bespoke.rs`.

11. **Separate schema admission from runtime support.** Select the original document's effective schema once using baseline/environment selection, document-context trigger discovery, and `DarkmatterSchemas::effective_for`; never compose or overlay `$schema`. Extract the reusable passive selection helper used by schema validation and dependency path completion rather than copying `schema/validate.rs::load_api`. Validate raw layer fields against that schema, suppressing only required-field errors for sparse individual layers. Check required style fields on the merged value, without validating unrelated required frontmatter. Use canonical-path lookup to project merged values back into the schema's admitted spelling for required-field checks; this preserves required shorthand/custom declarations while runtime normalization uses canonical paths. Never replace the document's effective style definition with the built-in one: a document declaring only `style.foo` rejects `style.margin`. Filter the rendering projection through runtime descriptors, retaining admitted custom/cleanup values separately with no unknown-key notices. Widening a built-in field still cannot bypass runtime validation. Evidence: `EffectiveSchema::{property_shape,validate}`, `simplified_schema::completion::def_at_path`, `style/descriptor.rs`, and the spec's custom-schema examples.

12. **Warnings keep their source and target policy.** Capture document diagnostics before cleanup or overrides. Unknown authored style keys remain warnings in render and errors in compose, except schema-admitted keys produce neither. Keep deprecated and informational notices, inline-HR warnings, original positions, and `--deny style`/individual-code behavior. Unknown file keys are excluded with warnings; unknown setter keys fail with `dm.schema.unknown_key`; bad setter types use `dm.schema.type_mismatch`. Runtime failures carry the canonical path and supplied argument/file rather than a fabricated document position. Schema admission is checked before the built-in walker so admitted custom keys never reach its unknown-key path. Evidence: `render.rs::{style_diagnostics,stop_if_style_code_denied,denied_style_issues}`, `style/parse.rs::from_frontmatter`, and `style/walker.rs::walk` currently consult only built-in descriptors.

13. **Keep source-target rendering behavior.** Explicit CLI layers are prepared for every target after `ReportSink` exists, before printing or `--show`. Source Markdown and data outputs never load a stylesheet or apply CLI style to authored frontmatter. Keep existing authored-style validation differences: plain Markdown does not demand a page; data output reports authored style warnings without making malformed authored style a page-build failure. Explicit CLI style values always receive schema and runtime checks, including width constraints, without CSS I/O. CLI `style.markdown` is validated but cannot enter cleanup overrides; existing guarded cleanup and `--indent` remain. Evidence: `commands/render.rs::Render::run` currently cleans before selecting source versus page output, and data output uses `style_diagnostics`.

14. **Use a dedicated local-file adapter.** Add `cli/src/render_style.rs`. First passively classify the complete `--style` argument with `FileReference`; explicit references retain all `@` characters. Only an implicit relative reference with a nonleading `@` and no separator before it splits into offset and reference; parse the offset with `PropertyPath`, without fallback. Open through `io::open_argument`, derive its context from `OpenedArgument::document_context`, then select the mapping at the offset. Reject stdin, HTTP(S) without a request, unsupported extensions, missing/nonmapping offsets, invalid UTF-8, and non-object roots as run failures. Use public `detect_from_extension` and biscuit-file's JSON, JSON5, YAML, and TOML readers; `.yml` shares the YAML branch. Do not use the expression `load_data()` executor or recursively import styles. Evidence: spec reference table; `io/mod.rs::{OpenedArgument,open_argument}` preserves opening-reference provenance.

15. **Retain reader-specific duplicate behavior.** JSON and JSON5 files use last-key-wins; YAML and TOML reject duplicate keys. Inline container setters reject duplicates through the shared setter parser. All loaders must consume the entire document; extra YAML documents fail. Style canonical-alias collisions fail separately even after successful file parsing. Reuse dependency numeric admission/location helpers without changing the readers globally. Evidence: `biscuit-file/lib/src/json5/types.rs` uses `json_five` into a JSON value, `toml_impl/types.rs` parses native TOML, `yaml/types.rs` uses `serde_yaml_ng`, and the loading-data plan retains last-occurrence JSON/JSON5 locations.

16. **A style file must supply a non-null admitted leaf.** After unknown-key exclusion, `{}`, empty nested containers, and null-only values fail per file. Cleanup-only, admitted custom, and target-inactive non-null leaves count; inline `style={}` remains a no-op. Empty strings remain values and receive their field validator, rather than being classified as absent. Evidence: spec empty-file examples; the safeguard applies to files and not setters.

17. **Resolve assets in the winning field's context.** Document stylesheet fields retain the original opening-reference context; inline fields use launch context; file fields use their own opened file context. Later unrelated files cannot change that origin. Replace `bespoke.rs::resolve_stylesheet`'s parent-join/CWD fallback with `FileReference` resolution against the supplied context. Local CSS is read only for HTML/MarkdownPlus; terminal/plain and source/data targets perform no CSS read. HTTP(S) CSS remains an emitted browser link, never a fetched style file; retain existing `file://` refusal. Images keep document context. Carry typed file errors into the existing diagnostic projection. Evidence: `bespoke.rs::apply_bespoke_style` currently takes one `source_path`, and its resolver calls `current_dir`; `OpenedArgument::document_context` already supplies the correct replacement context.

18. **Add only real presentation fields and use existing theme identity.** Rename Rust page spacing members to `margin_left`/`padding_left` etc., with canonical CSS-order serde names; remove old names and their snake aliases, and provide explicit new-name suggestions for old authored spellings. Add `PageStyle.theme: Option<ThemePair>`, `CodeStyle.mode: Option<CodeBlockMode>`, typed code theme, `CodeBlockStyle.line_numbers: Option<bool>`, and horizontal padding lengths to `CommonStyle` and `HrStyle`. Theme validation uses `ThemePair::try_from`, including its existing case/alias acceptance; completion offers `ThemePair::all()` canonical names. Resolve prose fallback from the captured output profile only after merge, then derive code fallback from that prose theme; mode defaults to inverse. Per-fence/direct-component settings keep higher precedence. Remove unsupported HR margin/border/emphasis/word-wrap from schema and descriptors. Evidence: `schema/page.rs`, `schema/components.rs`, `highlighting/themes.rs::ThemePair::try_from`, `layout/page.rs::{with_line_numbers,with_page_code_theme,with_code_block_mode}`, and the spec's measured HR behavior.

19. **Retire both claims and claim-only masks.** Delete `cli/src/style_claims.rs`, `lib/src/style/cli_claims.rs`, `FillClaim`, `apply_cli_claims`, all `*_overrides_from_claims`, `PageStyleOverrides`, `ComponentStyleOverrides`, `ListStyleOverrides`, `HrStyleOverrides`, `DisclosureStyleOverrides`, and `BespokeStyleOverrides`. Change `apply_*_style` functions to apply the effective style without override parameters. No external package uses these symbols in the current repository search. Update `lib/benches/render_pipeline_steps.rs`, render-tree structural/parity tests, browser tests, L1 style/cutover tests, and L2 layout policy tests. Evidence: repository-wide symbol search confines callers to Darkmatter; `apply.rs` masks exist solely to protect the old claim ordering. Removal follows migrated behavioral tests in Phase 5.

20. **Migrate all page-building consumers.** Change `render.rs::terminal_page`, `artifact.rs::html_page`, and `commands/render.rs::document_value` to receive prepared presentation/output/context values. `commands/clean.rs` and `commands/frontmatter.rs::set_value` use the document-only style path and the output already owned by `CommandRun`; they get no render overrides. Preserve exact edited Markdown output. Evidence: these are the additional production callers of `document_value`/page builders; compose no longer renders HTML.

21. **Handle no input in one render path.** With no input and terminal stdin, formatted outputs write root help to stderr, leave stdout empty, and return reported exit 1. Data outputs emit one failure envelope, empty stderr, exit 1. Check stdin's terminal state without reading it; redirected stdin, including empty input, uses the same `load_document` path in both spellings. Inject stdin state into the in-process entry test; use a controlled PTY only for one wiring test. Evidence: `main.rs::run` currently prints help and succeeds only for implicit rendering; `io::load_document` already distinguishes TTY from redirected stdin.

22. **List themes through the command-output contract.** Add `Command::ListThemes { output }`, `commands/list_themes.rs`, a `COMMAND_LEAVES` row, `cli/schemas/output/list-themes.yaml`, and conformance coverage. Use ordered `ThemePair::all()` entries with dark-mode descriptions as today; data is a bare array of `{ name, description }`, using `DataShape::Bare` for failure envelopes. Terminal presentation uses `TerminalRenderable` components, with migration guidance naming the new style setters. Exit 0 on success. Remove root listing dispatch and include the command in `Cli::requests_data_output`. Evidence: `main.rs::list_themes`, `output/command.rs::{CommandRun,DataShape}`, and the spec's list contents/order contract.

23. **Reuse completion, do not build another catalog.** Extend dependency `args/path_completion.rs` and neutral `schemas::completion::path_candidates` with a render command mode rooted at `style`. Reuse recovered shell words, original-document schema selection, and its advisory fallbacks. Never open a `--style` file, evaluate a value, compose a document, read stdin, or resolve a stylesheet during completion. Add a file completer restricted to JSON/JSON5/YAML/YML/TOML. Root alias and explicit render use the same adapter. Evidence: dotted-path plan Rules 19–21 specify the shared completer and shell recovery; the spec requires only render wiring, not a second shell matrix.

24. **Preserve the measured migration boundaries without further spikes.** Support all ten broadcast targets through explicit objects; list-group broadcasts name `ul`, `ol`, and `li`. Fill `pad`/`indent` replacements reset padding to zero before horizontal sides. The only accepted loss is automatic indent-side selection; callers name sides for left/right/center. Hyperlink fill and HR full had no observable effect in the spec's 2026-11-06 measurements; do not invent replacement behavior or benchmark them again. HR padding is implemented, not dropped. No retained claim machinery or additional behavior removal is planned. Evidence: the finalized spec's fill/HR measurements and decisions already answer the proposed equivalence questions.

### Wave 1 — Prerequisites (serial)

- [x] **Complete dependencies**
  - Finish the remaining dotted-path plan phases, followed by the loading-data plan phases, in this session's worktree. Preserve their public path/value/completion and reader contracts and their own validation evidence.
  - Record their completed APIs and evidence in this feature's implementation log. Do not reproduce their shell/format matrices in this feature.
  - Checkpoint: shared `PropertyPath`, `SetterValue`, schema path candidates, shell-word recovery, public format detection, and reader location helpers compile and their prerequisite suites pass. Phase 2 starts directly afterward.

## Phase 2 — Build the Effective Style Library

### Wave 2 — Models and Merge (parallel)

- [x] **Extend style models**
  - Own `style/schema/*`, `descriptor.rs`, `parse.rs`, `length.rs`, and `schemas/partials/style.yaml`. Implement Rule 18's names, shorthands, theme/mode/gutter fields, component sides, HR surface, and typo suggestions together.
  - Apply vertical `u16` limits to all-sides/vertical page spacing, preserve horizontal length grammar, and require a positive resolved page cap. Preserve unrelated compatibility aliases for authored frontmatter only.
  - Add parser/schema/typed-value agreement cases for new fields, compound unions, metadata, and cleanup rules. Correct comments on changed behavior, including currently stale “inactive” page-field descriptions.

- [x] **Implement sparse layers**
  - Own new `style/layer.rs`, `style/normalize.rs`, and their new L1 test modules. Implement Rules 7–12 and 16 against the existing style types; export integration follows Wave 2.
  - Build public preparation tests for normalized shorthand precedence, all broadcast components, exact-path replacement, distinct-spelling collisions, scalar/object union switching, null absence, sparse borders, width replacement, and required fields supplied across layers.
  - Validate schema-admitted custom values without routing them to the built-in unknown-key walker; test schema replacement and widened known fields explicitly.

### Wave 3 — Lowering (serial)

- [x] **Wire effective styles**
  - Export the new library API from `style/mod.rs`. Extend `apply.rs`, `bespoke.rs`, `layout/context.rs`, `layout/page.rs`, and render-tree build context to consume the new fields. Keep existing claim APIs temporarily so all intermediate callers compile.
  - Lower component side padding into `ComponentPolicy.layout.padding` for terminal and browser; update hyperlink/image `CommonStyle::to_css_overlay` and local-style merges as well as block/list/disclosure/HR policy paths. Preserve inherited vertical padding for ordinary side-only overrides.
  - Split passive style/runtime validation from asset execution; page construction takes `PreparedStyle` and a target-aware asset resolver. Implement winning stylesheet origins with request contexts, no `current_dir`, local CSS reads only for browser-backed targets, and retained remote CSS links.
  - Resolve theme/mode/line-number defaults after merging and propagate through both browser and terminal contexts. Capture `THEME` and `CODE_THEME` from `MdRequest`'s snapshot into prepared presentation defaults: preserve current `ThemePair::from_str_or_default` behavior for present environment values, use `OneHalf` for absent prose defaults, and derive an absent code default from effective prose through `get_code_theme_for_prose`. Color mode/capability degradation comes from resolved output options. Do not call environment-reading `detect_prose_theme`/`detect_code_theme` downstream. Add deterministic output/style assertions with explicit widths and pinned terminal capabilities.
  - Checkpoint: focused library nextest coverage passes for new models, public layer preparation, effective layout policy, themes, and CSS origins; existing paths still compile.

## Phase 3 — Implement CLI Preparation

### Wave 4 — Files and Arguments (parallel)

- [x] **Load style files**
  - Own `cli/src/render_style.rs`, its library export, and new style-file L1 tests. Implement passive reference/offset parsing, local opening, extension dispatch, source locations, empty-file checking, and per-field origins under Rules 14–17.
  - Extract the reusable passive schema-selection helper beside the dependency's shared selection code and route execution through it. Preserve baseline environment controls and trigger findings; use the Markdown document's context, not the style file's context, for schema selection.
  - Return typed diagnostics and prepared layers to the caller; print nothing here. Reuse data-reader location helpers and file-reference failure fields. Wrap schema/value failures with `output/report.rs::ClassifiedFailure::report` so `failure_diagnostics` preserves `dm.schema.*` codes; retain `DocumentArgumentError` in file-failure chains. Store explicit CLI locations with `ReportSink::diagnostics`, avoiding its authored-document fallback.

- [x] **Refactor render arguments**
  - Own `args/render.rs`, root `args/cli.rs`, `args/command.rs`, `args/target.rs`, `args/mod.rs`, and focused parser tests. Implement shared positional classification, root-option presence checks, style file references, behavior switches, removals, and local `schema about --code-block`.
  - Add `PreparedRenderOptions` construction in a new `cli/src/args/render_prepare.rs`; keep semantic style preparation outside clap so its failures can use the run sink.
  - Tailor root help with clap's root-only help template and argument help headings: commands, shared controls, `--style`, and the exact alias sentence; keep full retained render details on `render --help`. Do not hide shared `RenderOptions` fields globally merely to shorten root help.
  - Prove the real parser accepts command-name files after `--`, setter-looking files after `--`/`./`, values containing `=`, native argument tokens, and setters on either side of the input; root render arguments before commands fail with exit 2.

## Phase 4 — Integrate Dispatch and Outputs

### Wave 5 — Render Execution (serial)

- [x] **Unify render dispatch**
  - Own `commands/mod.rs`, `commands/render.rs`, `main.rs`, `render.rs`, and `artifact.rs`. Resolve output once, install the sink before file/value preparation, prepare schema and CLI layers before output, and call the isolated handler signature in Rule 6.
  - Capture original document diagnostics and context before cleanup; run existing guarded cleanup with `--indent` only; apply effective presentation separately. Preserve source/data behavior and complete all validation before artifact opening.
  - Implement common no-input behavior; remove implicit clean and root theme-list branches. Keep root help/version/completion early actions and existing removed-policy-flag diagnostics.
  - Update `document_value`, `commands/clean.rs`, and `commands/frontmatter.rs` to document-only prepared styles and captured output/request values. Update `io::parse_argument`'s non-Unicode refusal without introducing a native-path resolver.
  - Checkpoint: implicit/explicit smoke cases match in terminal/plain, HTML, MarkdownPlus, Markdown, and each data target; style failures produce exit 1 and no artifact; data failures produce exactly one stdout value and empty stderr.

### Wave 6 — Secondary Surfaces (parallel)

- [x] **Add theme command**
  - Own `commands/list_themes.rs`, output data model/schema, `COMMAND_LEAVES`, and theme conformance tests. Wire the completed handler into dispatch and data-output detection after the wave integrates.
  - Preserve theme ordering/descriptions and test all data formats plus one formatted control. Keep valid local `--theme`, `--save`, and `--code-block` command tests.

- [x] **Connect style completion**
  - Own render additions to `args/path_completion.rs`, `args/completion.rs`, and render completion L1 tests. Wire both spellings to effective-schema candidates; add file filtering and style-root scoping.
  - Test `style.` → `style.page.`, `style.ul.` → `padding-left=`, enum values after `=`, unknown/open/custom schemas, no per-key flags after `--`, and no removed flag suggestions. Reuse dependency shell adapters and their recorded shell constraints.
  - Use unreadable/side-effect sentinel style files and expressions to demonstrate that completion does not prepare styles or execute content.

## Phase 5 — Prove Equivalence and Retire Claims

### Wave 7 — Behavioral Coverage (parallel)

- [x] **Cover alias parity**
  - Own new `cli/tests/l1/render_alias_parity.rs`, `help.rs`, and render output-contract tests. Generate the render option inventory from clap and setter candidates from effective schema/type information, with one explicit representative-value override map for conflict-sensitive combinations. Fail on an argument without a representative; never silently skip it.
  - Compare classified/prepared values and effective style as well as parser success. Enumerate each nonshared render option in root-before-subcommand rejection tests. Keep help/version/completion exceptions separate.
  - Prove byte parity with a styled MarkdownPlus fixture and deterministic terminal/plain/HTML fixtures. Cover no-input TTY, redirected empty/nonempty stdin, explicit `-`, shared-controls-only invocations, and command-name inputs without waiting on stdin.

- [x] **Cover input failures**
  - Own new public style-preparation robustness tests and CLI style-file/setter wiring tests. Implement the matrix below, including invalid overridden files, repeated setter replacement, empty-file versus inline-empty behavior, custom schemas, and diagnostics with paths/origins.
  - Verify a malformed or denied style cannot cause output, CSS access, network access, or artifact opening. A local style-file read is permitted preparation; expressions and shell commands never run. Use public preparation results for the matrix and sparse subprocess wiring cases per loader.

- [x] **Migrate layout coverage**
  - Own existing `cli/tests/l1/layout_flags.rs`, `layout_alignment.rs`, `layout_fill.rs`, `layout_style_frontmatter.rs`, other style-flag callers, affected L2 files, and headless browser assertions. Replace flags with semantically equivalent setters/objects while retaining expected output for retained behavior.
  - Cover every broadcast target and each fill mode; name sides explicitly for indent, reset whole padding for pad/indent, and retain HR placement assertions. Pin output width and terminal depth rather than ambient host facts.
  - Set `MD_DRY_RUN` on artifact-opening tests. Use the toolkit's focus-preserving terminal harness and browser-protocol input only; do not activate host windows.

### Input Robustness Matrix

The file loader and sparse-layer validators are new file/configuration readers, so this matrix applies to them. It does not apply to Markdown parsing or render-tree fields. Default outcome: an invalid supplied shape returns a located public preparation error, produces no rendered output, opens no artifact, and executes no expression or shell operation. CLI wiring preserves exit 1 and the data envelope. A location is an authored file/span or a command-line argument plus canonical property path; do not invent line numbers for setters.

Test each validator family once using a valid, load-bearing fixture and one edit per case. Each other field using that validator receives a positive case and one invalid case. Exercise absent, explicit null, wrong whole type, and empty values for every scalar family; additionally exercise one-invalid-element and all-invalid-elements for collection validators exposed by the selected schema. Use a custom admitted style array to exercise array validation without adding a rendering feature.

| Validator or loader | Control and exceptions to the default |
|---|---|
| CLI style root | A mapping with `margin: 2` changes effective margins. Scalar/array/null roots fail. Inline `{}` succeeds as a no-op; file `{}` fails. No supplied layer means document/default style. |
| Page rows and shorthands | `margin: 2` sets four sides; absent preserves prior values; null is absent in CLI layers. Zero is explicit. Negative, fractional, over-65535, unit-bearing vertical, and empty-string values fail. `mx: '10%'` succeeds; all-sides percentage fails. |
| Horizontal lengths and cap | `ml: '2ch'` changes the left margin; null preserves it; zero margin is explicit. Invalid units/ranges/empty strings fail. Zero or zero-resolving page content cap fails; available output width still bounds a valid cap. |
| Enum, theme, and bool | Valid theme, mode, alignment, and `line-numbers: false` reach the typed result; absent/null preserve lower layers. Theme aliases accepted by `ThemePair::try_from` succeed. Wrong type, unknown enum/theme, and empty string fail. |
| Compound border/object | A partial object preserves inherited weight/color. Scalar-to-object replaces the union arm; unknown members follow document/file/setter admission policy. CLI null leaves disappear. Empty nested objects supply no file leaf. Test valid scalar and object union controls and all invalid field shapes through public preparation. |
| Admitted array/open metadata | A custom schema-admitted array is retained without rendering effect; absent/null are no override, `[]` explicitly replaces an inherited array but alone supplies no non-null file leaf. One/all wrong elements fail. A whole valid `page.meta` object succeeds; nested arbitrary metadata setters fail. |
| Required properties | Two layers together satisfy required style members; omissions in a single sparse layer are valid. Combined missing required style fails. Unrelated required document fields do not become render blockers. Required null behavior follows effective schema admission before CLI null stripping and final required checking. |
| Unknown properties | Setter unknowns fail with schema codes. File unknowns warn and are removed; authored unknowns retain target-dependent warnings. Schema-admitted custom keys succeed silently; `--deny style` blocks relevant warnings. Check one invalid sibling never disappears silently. |
| JSON loader | Real root/offset fixture succeeds. Duplicate exact keys keep the last value. Trailing garbage fails. A distinct alias collision still fails after parsing. |
| JSON5 loader | Comments, identifier keys, single quotes, and trailing commas succeed. Duplicate exact keys keep the last value. Trailing garbage and inadmissible numeric values fail. Inline setter duplicate keys fail through the shared value parser. |
| YAML loader (`yaml`/`yml`) | Flow/block mappings and permitted aliases follow the shared reader. Duplicate keys, second documents, invalid trailing syntax, and inadmissible values fail. `page.color:` is explicit null and cannot alone make a file nonempty. |
| TOML loader | A real table/offset fixture succeeds. Duplicate keys, malformed/trailing content, and non-finite/inexact admitted numbers fail. Absent members stay absent; TOML has no null literal. |
| Reference/offset adapter | Plain and explicit references, escaped dots, existing array indexes, magic/repository/home forms, and explicit paths containing `@` select the correct mapping/context. Missing/nonmapping/out-of-range selections, unsupported formats, `-`, and remote style-file references fail. Remote refusal records zero requests. |

For each file loader spawn the CLI only for its positive control and one malformed plus one semantic failure, checking file/key location, empty output/artifact state, and data-stream behavior. Keep the full field-shape matrix in-process at `prepare_style_layers` or the exported CLI preparation boundary. Test multiple files with CSS selected from an earlier file and an unrelated later file, stdin Markdown with local styles, and document versus launch versus file-relative CSS origins. Add target controls proving source/data and terminal/plain never read CSS while HTML/MarkdownPlus read local CSS or emit remote links.

Before completing this wave, search changed validators for `#[serde(default)]`, `Option<T>`, `filter_map` with `as_str`, `unwrap_or_default()`, and swallowed `.ok()` parse results. Preserve defaults only behind explicit presence validation; intentional CLI null-as-absence is covered by Rule 10. Record findings and their resolved tests in the implementation log. Any review defect in a matrix cell triggers an audit of all shapes of that validator and all loaders feeding the same shape, with the status of those cells recorded together.

### Wave 8 — Remove Compatibility Machinery (serial)

- [x] **Retire style claims**
  - Delete Rule 19's symbols/modules and simplify all application signatures. Update remaining callers in library tests, browser/terminal suites, render-tree structural gates, and `lib/benches/render_pipeline_steps.rs` to effective styles with explicit origins.
  - Search the entire repository for claim types, override-mask types, old page key spellings, old render presentation flags, and obsolete root `save`/theme-list branches. Retain old spellings only in removal/migration tests, migration documentation, and historical specs/logs.
  - Checkpoint: new equivalence cases and migrated historical expectations pass; repository code has no claim references and production page builders read no `Cli`. No behavior removal beyond the spec's explicit boundaries is introduced.

## Phase 6 — Publish the Current Contract

### Wave 9 — Documentation (parallel)

- [ ] **Document CLI migration**
  - Own `docs/cli/render.md`, `index.md`, `output-and-width.md`, `data-contracts.md`, `clean.md`, `policy.md`, and `cli/README.md`. Publish the alias/no-input rules, exact placement of controls, setters/file grammar, precedence/errors, theme-list data shape, and retained local flags.
  - Include a migration row for every removed flag and visible alias in the spec inventory; explicitly expand list/all-component broadcasts and value-dependent fill modes, including hyperlinks, HR, and disclosure. State indent-side selection's removal and no-op measured replacements accurately.
  - Explain available output width versus content cap and why `md compose ... | md render - ...` uses launch context for relative assets. Remove root `--save` examples, preserve editing command `--save`, and replace listing guidance with `md list-themes`.

- [ ] **Document style architecture**
  - Own `docs/topics/layouts-and-style.md`, `docs/rendering/{style,mermaid,code-highlighting}.md`, affected example documents/golden fixtures, `.claude/skills/darkmatter/{cli,rendering,terminal,cleanup}.md`, and relevant CLI skill/reference text.
  - Explain sparse merge, shorthand specificity versus source precedence, repeat/conflict behavior, null absence, reader-specific duplicates, file/CSS origins, custom admission without rendering support, and validated cleanup values having no CLI render effect. Give compact examples and a Mermaid preparation/merge flow.
  - Replace old key names and flags in current docs, examples, and skill guidance. State shared-control placement and cite the parity test in `.claude/skills/cli/` and `docs/cli-best-practices.md`. Current docs never name feature/fix specs.
  - Checkpoint: all documented inline style examples parse through the shared value parser and new help examples match generated command help; no dependency change is needed by this feature beyond its prerequisite plans.

## Phase 7 — Validate the Integrated Result

### Wave 10 — Final Checks (serial)

- [ ] **Run integrated checks**
  - Run `just test`, `just lint`, and `just test-l2` from the absolute session-worktree `darkmatter/` directory. Run the area's `just test-browser` for changed browser style/geometry assertions, headlessly. Use nextest and the existing toolkit; never run `cargo test` or `cargo fmt`.
  - Compile examples and benchmarks with `cargo --color=never check -p darkmatter -p darkmatter-cli --examples --benches`; this covers the direct benchmark caller that L1 does not compile. Dependency areas retain their prerequisite validation evidence; run their focused checks again only for later edits to them.
  - Check every acceptance criterion against named tests/evidence in `implementation-log.md`: alias equality, root isolation, flags/help/completion, schema/typed agreement, sparse precedence, file formats/offsets/origins, output/stream/error behavior, all broadcasts, and theme listing.
  - Keep portable paths and tests compatible with macOS, Linux, native Windows, and WSL2. Native Windows lexical references receive platform-neutral parser tests plus target-gated real-path tests; do not claim a cross-compile or macOS result proves Windows runtime behavior. This plan's execution runs the local macOS checks above; the existing scheduled CI provides other environment evidence. Record unexecuted platform coverage explicitly. Do not invoke `just cross-check`: its documented transport creates a throwaway commit, outside this plan's no-commit scope. Do not add CI scope or a performance gate.
  - Review changed behavior's docs/comments and the matrix smell audit, report exact commands/results and any environment-limited evidence, update plan checkboxes, and leave implementation complete and ready for review in the active directory. No intermediate human decision remains.