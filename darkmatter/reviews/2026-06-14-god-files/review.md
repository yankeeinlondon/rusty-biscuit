# Darkmatter God-File Refactor Review

Date: 2026-06-14
Scope: high-risk god-file candidates in the `darkmatter` package area

This review consolidates six subagent evaluations. The common recommendation is to keep existing large files as compatibility facades first, extract private modules by responsibility, preserve public import paths with re-exports, and move tests with the behavior they protect.

## `darkmatter/lib/src/markdown/compose/mod.rs`

Current responsibilities: public `Markdown::compose*` API, root pipeline orchestration, effective-state setup, operation dispatch, transclusion parse/prepare/resolve/apply, cache/runtime wiring, remote prefetch discovery, source-map application, and broad integration tests.

Proposed split:
- Keep `mod.rs` as the public facade and phase coordinator: module declarations, re-exports, `Markdown::compose`, `compose_with`, `compose_mut`, and a short `run_compose_pipeline`.
- Extract `pipeline.rs` for `run_compose_pipeline_internal`, phase loop, frontmatter prelude, effective state build, and warning conversion.
- Extract `stages.rs` for inline pre/post/finalization dispatch helpers and small stage runners.
- Extract `transclusion_phase/mod.rs` plus `prepare.rs`, `resolve.rs`, `apply.rs`, and `source_map.rs` for prepared/resolved transclusion state and application.
- Extract `reference_ranges.rs` for `find_target_range` and syntax attribute helpers.
- Extract `path_display.rs` or `diagnostics.rs` for path abbreviation, git-root discovery, and small shared helpers.

Migration order: move reference/path helpers first, then inline stage dispatch, then transclusion data/apply/source-map helpers, and prepare/resolve last. Move tests by behavior into compose pipeline, replacement, interpolation, and transclusion groups.

Risks/tests: preserve compose public API and operation order. The risky boundary is cache/runtime borrowing around concurrent transclusion resolution; run focused compose, remote, and caching transclusion tests after each extraction.

## `darkmatter/cli/tests/cli.rs`

Current responsibilities: one integration test file covers render, compose, remote fetch/cache, state/set overlays, hash, frontmatter get/set/rm, shell approval, refs/graph, completions, layout flags, style frontmatter, and code block flags.

Proposed split:
- Convert shared helpers into `tests/common/mod.rs`.
- Extract `tests/render.rs`, `tests/compose.rs`, `tests/compose_remote.rs`, `tests/hash.rs`, `tests/frontmatter.rs`, `tests/refs_graph.rs`, `tests/layout_cli.rs`, `tests/style_cli.rs`, `tests/completions.rs`, and `tests/code_block_flags.rs`.
- Keep `cli.rs` only for tiny command smoke tests, or remove it once command domains are split.

Migration order: move shared helpers first (`md_cmd`, `md_file`, mock HTTP server, layout helpers), then split self-contained command groups like completions/hash/frontmatter, then remote tests, and layout/style last.

Risks/tests: hidden ordering and helper coupling are the main risks. Keep files under `darkmatter/cli/tests`, use the common module consistently, and run the CLI test target after each chunk.

## `darkmatter/lib/src/layout/page.rs`

Current responsibilities: defines `ComponentPolicy` and `DarkmatterPage`, stores page/layout/style state, implements builder API, derives terminal/browser render options, renders terminal/browser/MarkdownPlus, applies row decoration, emits browser wrapper HTML, implements `TerminalRenderable`, and contains layout/render regression tests.

Proposed split:
- Keep `page.rs` as the public type surface: `ComponentPolicy`, `DarkmatterPage`, constructor, accessors, and builders.
- Extract `page/render_terminal.rs`, `page/render_browser.rs`, `page/frame.rs`, `page/browser_wrapper.rs`, and `page/terminal_renderable.rs`.
- Optionally extract `page/policy.rs` for `ComponentPolicy`, color lookup helpers, and painted-color scanning.

Migration order: pure frame helpers first, browser wrapper next, then `TerminalRenderable`, browser rendering, and terminal rendering last.

Risks/tests: high risk around byte-for-byte terminal output, terminal color mode/depth, and wrapper/no-wrapper decisions. Preserve builder signatures and move tests by behavior: builders, terminal rendering, browser rendering, component policy, and frame decoration.

## `darkmatter/lib/src/markdown/compose/expression/functions.rs`

Current responsibilities: all built-in expression functions, argument validation, date/string/case helpers, filesystem/path functions, skill-root discovery, Markdown-reading functions, schema validation, registration tables, dispatch helpers, and tests.

Proposed split:
- Keep `functions.rs` as registration and dispatch facade: `PureFunction`, `FsFunction`, `PURE_FUNCTIONS`, `FS_FUNCTIONS`, dispatch functions, and re-exports.
- Extract `functions/args.rs`, `types.rs`, `math.rs`, `collections.rs`, `strings.rs`, `dates.rs`, `paths.rs`, `skills.rs`, `markdown_docs.rs`, and `terminal.rs`.

Migration order: pure helpers first, then dates, path helpers, skill and Markdown document functions, then registration table imports. Move tests into matching modules and keep descriptor/dispatch catalog tests in the facade.

Risks/tests: dispatch tables must remain authoritative. Preserve alias dispatch, remote URL behavior in read-side functions, local-only frontmatter failures, and date tests with injectable dates.

## `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`

Current responsibilities: parses `$(...)` frontmatter values, detects ternaries, classifies value-vs-command bodies, validates executable interpolation and pipeline shape, scans frontmatter, prepares branch pipelines, evaluates ternary branches, executes directives concurrently, rewrites frontmatter, exposes reachable pipelines, and contains parser/execution tests.

Proposed split:
- Keep `frontmatter_shell_expansion.rs` as facade with `execute_frontmatter_shell_expansion`, `scan_frontmatter`, `parse_shell_value`, `directive_reachable_pipelines`, and re-exports.
- Extract `types.rs`, `parser.rs`, `ternary.rs`, `classify.rs`, `validation.rs`, `execution.rs`, `eval.rs`, and `diagnostics.rs`.

Migration order: types first, diagnostics and lexical helpers next, then classification/ternary parsing, validation, and finally execution/evaluation.

Risks/tests: preserve security invariants: no interpolated executable, no interpolation-created chain actions, both ternary command branches pre-approved before selection, local-only frontmatter resolution, and deterministic rewrite order after parallel execution.

## `darkmatter/cli/tests/level2_layout.rs`

Current responsibilities: WezTerm Level 2 harness setup, command wrapping, sentinel polling, temp fixtures, `md` binary shim, raw terminal/SGR helpers, and layout smoke/regression tests for page, code, tables, block quotes, lists, images, HRs, hyperlinks, and disclosures.

Proposed split:
- Extract shared harness helpers to `darkmatter/cli/tests/support/level2_terminal.rs`.
- Extract raw/SGR inspection helpers to `support/terminal_capture.rs`.
- Split scenarios into `level2_page.rs`, `level2_code_blocks.rs`, `level2_components.rs`, `level2_style_frontmatter.rs`, `level2_hr.rs`, `level2_links_images.rs`, and `level2_disclosure.rs`.

Migration order: helpers first, then page/code-block tests, style/component tests, and disclosure tests last.

Risks/tests: preserve `#[serial(level2_terminal)]`, tempdir lifetimes, and exact Level 2 command behavior.

## `darkmatter/lib/src/markdown/compose/types.rs`

Current responsibilities: compose operation model, public `ComposeOptions`, `ComposeSource`, `ComposeContext`, reports/warnings/source maps, shell spans, redaction, perf reporting, and tests.

Proposed split:
- Extract `operation.rs`, `options.rs`, `source.rs`, `context_types.rs` or extend `compose/context/`, `report.rs`, and `perf_types.rs`.
- Keep `types.rs` as a compatibility facade with `pub use` re-exports until imports are migrated.

Migration order: operation metadata first, then perf/report types, `ComposeSource`, `ComposeOptions`, and finally `ComposeContext`.

Risks/tests: public API risk is high. Preserve imports from `darkmatter::markdown::compose::{...}` and keep exact descriptor ordering tests.

## `darkmatter/lib/src/markdown/cleanup.rs`

Current responsibilities: cleanup entrypoints, list/emphasis options, parser option selection, emphasis marker preservation, event-stream cleanup orchestration, table alignment, list marker/indent/spacing normalization, blockquote/bracket/code-block cleanup, trailing-newline handling, and tests.

Proposed split:
- Keep `cleanup.rs` as public facade and orchestration layer.
- Extract `cleanup/emphasis.rs`, `tables.rs`, `lists.rs`, `code_blocks.rs`, `blockquotes.rs`, and `escaping.rs`.

Migration order: pure leaf helpers first, table alignment next, list handling next, and emphasis handling last.

Risks/tests: behavior is fragile around list spacing, emphasis placeholders, and code-block protection. Move tests by cluster and keep end-to-end cleanup tests in the facade.

## `darkmatter/lib/src/markdown/render_tree/fold.rs`

Current responsibilities: parser options, stack-frame model, `Fold` state machine, CommonMark/GFM event dispatch, disclosure lowering, inline extension envelope dispatch, embedded marker handling, container construction, text/slug helpers, span/provenance resolution, and tests.

Proposed split:
- Keep `fold.rs` as public fold orchestration and main `Fold` state machine.
- Extract `parser_options.rs`, `containers.rs`, `text_extract.rs`, `inline_envelope.rs`, `disclosure_fold.rs`, `embedded.rs`, and `spans.rs`.

Migration order: pure helpers first, inline envelope helpers next, disclosure sub-folding after that, embedded handling last.

Risks/tests: source spans and diagnostics are the highest risk. Preserve provenance tests and move embedded/envelope/disclosure/slug tests with their modules.

## `darkmatter/lib/src/render/image_ref.rs`

Current responsibilities: `ImageRef` model and builders, browser image attrs, rich errors/status blocks, Markdown/HTML/terminal rendering, Markdown/HTML parsing, structured image-title directives, metadata package encoding/decoding, string utilities, and tests.

Proposed split:
- Keep `image_ref.rs` as public facade with `ImageRef`, builders/accessors, and re-exports.
- Extract `image_ref/error.rs`, `attrs.rs`, `render.rs`, `parse.rs`, `metadata.rs`, `structured_title.rs`, and `util.rs`.

Migration order: public enums first, error handling next, utilities/metadata, parsers, then render methods.

Risks/tests: preserve public names from `darkmatter::render`. Pin metadata encoding, `IMAGE_REF_METADATA`, structured title parsing, `srcset` fallback, and terminal/HTML/Markdown round trips.

## `darkmatter/lib/src/markdown/transform/mod.rs`

Current responsibilities: public `Markdown` transform entry points, recursive pipeline orchestration, state/default/override merging, stage runners, block transclusion, frontmatter prologue/epilogue handling, wrappers, child state construction, and tests.

Proposed split:
- Keep `mod.rs` as public facade for `Markdown::{transform, transform_with, transform_mut}`.
- Extract `pipeline.rs`, `state_merge.rs`, `stages.rs`, `transclusion_runner.rs`, `wrappers.rs`, and `env.rs` or `options_resolve.rs`.

Migration order: pure helpers first, simple stage runners next, transclusion rendering next, recursive pipeline runner last.

Risks/tests: preserve stage order and recursive transclusion runtime sharing. Keep public transform tests in the facade and move transclusion/state/shell tests with their modules.

## `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs`

Current responsibilities: shell expansion facade and engine, directive preparation, alias resolution, approval/allowlist/blacklist policy, approval persistence, execution, error handling, display commands, pipeline actions, replacements, and tests.

Proposed split:
- Keep `mod.rs` as facade.
- Extract `prepare.rs`, `approval.rs`, `execute.rs`, `error_handling.rs`, `effective.rs`, and `replace.rs`.

Migration order: replacements first, command-shape helpers next, alias/effective resolution, execution/preparation split, approval handling last.

Risks/tests: approval semantics are the main risk: allow-once reservation, partial chain approval, concurrent pending behavior, and persisted exact/prefix rules. Preserve current public re-exports.

## `darkmatter/lib/src/markdown/compose/context/capture.rs`

Current responsibilities: context demand scanning, group mapping, raw capture from `sniff`, parallel capture orchestration, derived repo/package/doc/language/file-change/OS/hardware/agent values, formatting glue, test fixtures, and tests.

Proposed split:
- Keep `capture.rs` as orchestration facade exposing `ContextGroup`, `scan_needed_groups`, and `capture_runtime_context*`.
- Extract `groups.rs`, `snapshot.rs`, `populate/datetime.rs`, `populate/repo.rs`, `populate/file_changes.rs`, `populate/languages.rs`, `populate/docs.rs`, `populate/system.rs`, `populate/agent.rs`, and test support.

Migration order: group scanning first, pure populate functions next, repo/file-change/language/docs together or in adjacent changes, and `ContextCapture::new` last.

Risks/tests: preserve demand-driven capture so unknown or unrelated `ctx.*` references do not trigger expensive probes.

## `darkmatter/lib/src/style/apply.rs`

Current responsibilities: style override structs, apply errors, page style lowering, component/list/color/HR/disclosure lowering, HR enum mapping, length conversion helpers, percent/max-width resolution, and tests.

Proposed split:
- Keep `apply.rs` as public facade and sequencing surface.
- Extract `apply/overrides.rs`, `error.rs`, `length.rs`, `page.rs`, `components.rs`, `lists.rs`, `hr.rs`, and `disclosure.rs`.

Migration order: override structs and error enum first, length helpers next, HR/disclosure, list/component lowering, page style last.

Risks/tests: public API risk is high for `style::apply::*`; preserve re-exports. CLI precedence and field-by-field override suppression need regression tests.

## `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs`

Current responsibilities: public parse entry point, token definitions, lexer, parser, inline object grammar, primitive type parsing, constraints, argument coercion, error formatting, and parser tests.

Proposed split:
- Keep `grammar.rs` as facade with `MAX_INLINE_OBJECT_DEPTH` and `parse_type_expr`.
- Extract `grammar/token.rs`, `lexer.rs`, `parser.rs`, `inline_object.rs`, `constraints.rs`, `args.rs`, and grouped tests.

Migration order: token and argument helpers first, lexer next, constraint parsing, inline object parsing, main parser last.

Risks/tests: preserve `SchemaError::Grammar` spans and token consumption behavior. Split tests into primitive types, constraints, enum behavior, descriptions, inline objects, and error spans.

## `darkmatter/lib/src/markdown/compose/cache/runtime.rs`

Current responsibilities: run-local Markdown/TOC/compose/operation cache, single-flight coordination, persistent cache read/write, document snapshots, manifest validation, dependency closure hashes, remote URL revalidation hooks, stats, and tests.

Proposed split:
- Keep `runtime.rs` as facade defining `RunLocalCache`, constructors, `load_markdown`, `load_toc_headings`, `stats`, and public dependency-ref helpers.
- Extract `single_flight.rs`, `persistent_compose.rs`, `persistent_operation.rs`, `snapshot.rs`, and `validation.rs`.
- Extract `stats.rs` only if stats mutation grows.

Migration order: group tests first, snapshot logic, persistent operation, persistent compose/validation, single-flight last.

Risks/tests: cache freshness, stale fallback, and single-flight wake/error paths are highest risk. One subagent noticed possible duplicated/drifted lines; verify with compile/tests before refactoring and treat code as authoritative where comments drift.

## `darkmatter/lib/src/render/link.rs`

Current responsibilities: link diagnostics/status blocks, `LinkType`/`LinkTarget`/`Link`, builders/accessors, OSC8/HTML/Markdown/popover rendering, Markdown metadata policy, metadata encoding/decoding, HTML/Markdown parsing, structured props, escaping/unescaping, ANSI stripping, CSS serialization, base64 helpers, and tests.

Proposed split:
- Keep `link.rs` as facade with core public types and re-exports.
- Extract `error.rs`, `target.rs`, `render.rs`, `markdown.rs`, `html.rs`, `metadata.rs`, and `util.rs`.

Migration order: group tests first, then target/error, HTML/Markdown parsers, metadata, and rendering last.

Risks/tests: preserve `darkmatter::render::link::Link` imports, `LINK_METADATA`, lossless metadata, ANSI stripping, and escaping behavior. Potential duplicate assignment/iterator issues should be handled separately from mechanical splitting.

## `darkmatter/lib/tests/level2_render_tree_terminal.rs`

Current responsibilities: WezTerm Level 2 harness, render-tree fixture rendering, ANSI/SGR helpers, render-probe re-exec path, and real-terminal tests for code blocks, headings, inline styles, tables, mark/dim, HRs, page geometry, images, public entrypoints, layout policy parity, percentage frames, and `::file-links`.

Proposed split:
- Keep a thin integration entry file.
- Extract `support/harness.rs`, `support/render.rs`, `support/ansi.rs`, and `support/image.rs`.
- Split tests into `level2_render_tree_basic.rs`, `level2_render_tree_spans.rs`, `level2_page_geometry.rs`, `level2_images.rs`, `level2_public_entrypoints.rs`, and `level2_file_links.rs`.

Migration order: support helpers first, render-probe code carefully next, then feature-family test files.

Risks/tests: preserve serial groups, env vars, current test executable names, sentinel timing, and `BISCUIT_TEST_LEVEL_REQUIRED` semantics.

## `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`

Current responsibilities: public `EvalResult` and `EvalValue`, thin `Evaluator` wrapper over expression evaluation and scalar conversion, direct variable logging, and a large test suite.

Proposed split:
- Keep `evaluator.rs` small with `Evaluator` and `eval`/`eval_value`.
- Extract `value.rs` for `EvalValue` conversions/truthiness/number coercion.
- Extract `result.rs` for `EvalResult`.
- Move most tests to separate evaluator test modules because production logic is already small.

Migration order: move tests first, extract `EvalResult`/`EvalValue`, re-export from `interpolation::mod.rs`, leave `Evaluator` in place.

Risks/tests: preserve public import paths. Tests should verify behavior through public `parse` plus `Evaluator`, since evaluation delegates to `expression`.

## `darkmatter/cli/src/args.rs`

Current responsibilities: all clap command/subcommand definitions, value enums, top-level `Cli`, global render/layout flags, shell completion helpers, parsers for indent/theme/fill/length/bools/max width/color/Tailwind names, and tests.

Proposed split:
- Keep `args.rs` as facade exposing `Cli`, `Command`, and re-exported submodules.
- Extract `formats.rs`, `commands.rs` or `command.rs`, `layout.rs`, `color.rs`, and `completion.rs`.

Migration order: parser-only code first, completion helpers next, format/target enums, `Command` last.

Risks/tests: preserve clap behavior exactly: aliases, globals, conflicts, completers, and defaults. Use representative `Cli::try_parse_from` tests after each extraction.

## `darkmatter/lib/src/markdown/compose/expression/parser.rs`

Current responsibilities: `ParseError`, `Parser`, public `parse`/`parse_condition`, recursive descent grammar, condition-mode behavior, and parser tests.

Proposed split:
- Keep `parser.rs` as public facade for `Parser`, `ParseError`, `parse`, and `parse_condition`.
- Extract `error.rs`, `cursor.rs` or `state.rs`, `precedence.rs`, `primary.rs`, and focused test modules.

Migration order: `ParseError` first, cursor helpers, primary/postfix parsing, precedence parsing last.

Risks/tests: condition mode has subtle `||` semantics: fallback in interpolation mode, logical OR in condition mode. Run parser-focused tests, then expression integration tests.

## `darkmatter/lib/src/style/bespoke.rs`

Current responsibilities: bespoke style data types, sub-spec #7 application to `DarkmatterPage`, local/remote stylesheet resolution, page metadata parsing, link/image width validation, local link/image classification, common style merging, terminal inline layout, and tests.

Proposed split:
- Keep `bespoke.rs` as public facade with `apply_bespoke_style` and re-exports.
- Extract `stylesheet.rs`, `meta.rs`, `locality.rs`, `inline_layout.rs`, and `apply_bespoke.rs`.

Migration order: pure helpers first, stylesheet IO next, public data types only with re-exports, and shrink `apply_bespoke_style` last.

Risks/tests: preserve `style/mod.rs` re-exported names. Keep renderer integration tests for hyperlink/image color and terminal width near the facade. One subagent observed comment/test fixture noise that should be cleaned only when touching nearby code.

## `darkmatter/lib/src/markdown/output/terminal.rs`

Current responsibilities: terminal rendering options and mode enums, image alt-width parsing, `ImageRenderer` capability/path/file-size/fallback behavior, code-block header helpers, test-only code-block re-exports, and mixed tests.

Proposed split:
- Keep `terminal.rs` as facade for stable imports.
- Extract `options.rs`, `image.rs`, and `code_header.rs`.
- Keep syntax-highlighted code-block rendering in existing `output/code_block.rs`.

Migration order: code header first, image handling next, options last.

Risks/tests: public API risk is high around `TerminalOptions`; preserve field names/defaults. Cover image path traversal, absolute paths, remote URLs, missing files, and force/never mode.

## `darkmatter/lib/src/markdown/render_tree/entrypoints.rs`

Current responsibilities: adapter boundary from `Markdown`/`DarkmatterPage` to render-tree rendering, document/context building, top-level HR defaults, HTML/terminal/page-terminal/Markdown/MarkdownPlus rendering, browser directive validation, option mapping, source descriptors, and tests.

Proposed split:
- Keep `entrypoints.rs` as public function facade.
- Extract `document.rs`, `hr_defaults.rs`, `browser.rs`, `terminal.rs`, `markdown.rs`, and `validation.rs`.

Migration order: pure adapters first, HR defaults, browser mapping/validation, terminal mapping/page-terminal rendering, facade last.

Risks/tests: preserve render-tree cutover semantics, diagnostic separation in `PipelineResult`, browser fatal vs terminal degraded malformed code directives, terminal capability mapping, and page content width.

## `darkmatter/lib/src/markdown/compose/expression/lexer.rs`

Current responsibilities: interpolation expression finding in Markdown, code-region skipping rules, token model, lexer errors, lexical scanner, condition-mode operator behavior, and tests.

Proposed split:
- Keep `lexer.rs` as facade re-exporting token, error, finder, and scanner types.
- Extract `finder.rs`, `token.rs`, `error.rs`, and `scanner.rs`.
- Defer `literals.rs` unless scanner growth justifies it.

Migration order: token enums/display, error, expression finder, scanner last.

Risks/tests: preserve public re-exports from `expression/mod.rs`. Finder behavior is subtle: inline code spans are scanned, fenced/indented blocks are skipped. Run parser tests after lexer movement.

## `darkmatter/lib/src/markdown/compose/expression/mod.rs`

Current responsibilities: expression facade, public re-exports, `EvaluationLookup`, scalar coercion/truthiness helpers, evaluator dispatch, binary/member/index semantics, function dispatch glue, and evaluator tests.

Proposed split:
- Keep `mod.rs` as facade with module docs, `pub mod`, `pub use`, `EvaluationLookup`, and `UNKNOWN_FUNCTION_PREFIX`.
- Extract `eval.rs` for `evaluate`, `evaluate_binary`, `evaluate_index`, `evaluate_member`, and `evaluate_function`.
- Extract `value.rs` for `is_truthy`, number coercion, `scalar_string`, and numeric helpers.

Migration order: `value.rs` first, then `eval.rs`, then move tests by behavior group.

Risks/tests: preserve imports from `darkmatter::markdown::compose::expression::{...}`. Run expression, interpolation, page-block, and frontmatter shell tests.

## `darkmatter/cli/src/output.rs`

Current responsibilities: terminal rendering, CLI layout/style override mapping, style frontmatter application, artifact generation/opening, env parsing for terminal images, TOC printing, and delta reporting.

Proposed split:
- Keep `output.rs` as facade for CLI command imports.
- Extract `output/render.rs`, `style.rs`, `artifact.rs`, `env.rs`, `toc.rs`, and `delta.rs`.

Migration order: artifact/env first, style override logic next, TOC and delta last.

Risks/tests: preserve `crate::output` import paths for CLI commands. Visible output formatting for TOC/delta needs snapshot-style coverage if available.

## `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`

Current responsibilities: shell command origin/directive/pipeline structs, redirection rendering, command entries, error-handling rules, shell options/approval traits, `ShellExpansionError` status rendering, policy rule types, shell approval runtime state, blacklist rules, and broader `PipelineRuntime`.

Proposed split:
- Keep `types.rs` as compatibility facade, or re-export smaller modules from `shell_expansion/mod.rs`.
- Extract `origin.rs`, `directive.rs`, `handling.rs`, `options.rs`, `error.rs`, `policy_types.rs`, `runtime.rs`, and `pipeline_runtime.rs`.

Migration order: pure data groups first, error after imports stabilize, runtime and pipeline runtime last.

Risks/tests: API risk is high inside compose. Keep old paths with re-exports until call sites migrate and add focused tests for reservation behavior if missing.

## `darkmatter/lib/src/markdown/reference/graph.rs`

Current responsibilities: reference graph construction, runtime/cache setup, recursive node traversal, transclusion and `toc-linking` follow behavior, prologue/epilogue traversal, local reference extraction, InlinePre preparation, path/source/node-id helpers, graph flattening, and integration tests.

Proposed split:
- Keep `graph.rs` as public crate-local entry facade: `build_reference_graph`, `build_transclusion_graph`, `flatten_graph`, and `prepare_content_for_validation`.
- Extract `runtime.rs`, `flatten.rs`, `node.rs`, `directives.rs`, `extract.rs`, `prepare.rs`, and `path.rs`.

Migration order: flatten/extract/path first, prepare next, directive handling out of `build_node`, then move node orchestration.

Risks/tests: preserve composed ordering, cycle detection, `when=` evaluation, and `toc-linking` synthesized references. Keep recursive traversal, prologue/epilogue, cycle, and `when=` tests at facade/integration level.

## `darkmatter/lib/src/markdown/schemas/coerce.rs`

Current responsibilities: schema shape recognition, coercion target model, boolish/numberlike recognizers, root-union arm selection with pending shell keys, object/property-union coercion, scalar/array/inline-object coercion, and matrix tests.

Proposed split:
- Keep `coerce.rs` as facade exposing `CoercionTarget`, `CoercionOutcome`, `coercion_target`, `coerce_frontmatter`, and `coerce_frontmatter_with_pending`.
- Extract `target.rs`, `engine.rs`, and `value.rs`; optionally extract `tests.rs` if test volume remains high.

Migration order: target recognition first, value coercion next, engine/root-union behavior last.

Risks/tests: main behavior risk is broadening recognized schema shapes. Preserve exact recognizer tests and root-union pending-key tests.

