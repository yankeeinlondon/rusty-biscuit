---
spec: "/home/build/wt/rusty-biscuit/fix-biscuit-terminal/darkmatter/features/2026-11-20-schema-typed-load-data/spec.md"
plan: "darkmatter/features/2026-11-20-schema-typed-load-data/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
    - "simplified-schema/lib/src/expression/ast.rs"
    - "simplified-schema/lib/src/expression/lexer.rs"
    - "simplified-schema/lib/src/expression/mod.rs"
    - "simplified-schema/lib/src/expression/parser.rs"
    - "simplified-schema/lib/src/simplified/grammar.rs"
    - "simplified-schema/lib/tests/l1/expression_syntax.rs"
    - "darkmatter/lib/src/markdown/compose/cache/remote_cache.rs"
    - "darkmatter/lib/src/markdown/compose/remote_fetch.rs"
    - "darkmatter/lib/src/markdown/compose/expression/absence.rs"
    - "darkmatter/lib/src/markdown/compose/expression/call_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/lint.rs"
    - "darkmatter/lib/src/markdown/compose/expression/member_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/prepared.rs"
    - "darkmatter/lib/src/markdown/compose/preflight/collect.rs"
    - "darkmatter/lib/src/markdown/compose/remote.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/admission.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/evaluator.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs"
    - "darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs"
    - "darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs"
    - "darkmatter/lib/tests/l1/semantic_results_never_persist.rs"
    - "darkmatter/dmls/src/overlay/expressions.rs"
    - "claudine/lib/src/composition/lifecycle/prewarm/classify.rs"
    - "claudine/lib/src/dispatch/matcher.rs"
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - ".claude/skills/darkmatter/remote-cache.md"
packages:
    - "simplified-schema"
    - "darkmatter"
    - "darkmatter-lsp"
    - "claudine"
    - "darkmatter-cli"
    - "claudine-cli"
    - "biscuit-terminal"
decisions_for_review:
    - |-
        Phase 1 changes the shared expression tree, which breaks every Darkmatter, DMLS, and Claudine reader of object literals; the plan schedules their spread semantics for Phase 2 but Phase 1 must still compile and pass. I adapted every reader now: pure walkers visit spread operands (what Phase 2 asks for), the evaluator already merges entries in order with a null spread adding nothing, and passive typing treats any literal containing a spread as "some object" (no member or option findings). A non-object spread operand fails at evaluation with the generic catch-all error ("...(): object spread requires an object or null, found number") instead of the planned typed error and code. Alternatives: leave the build broken until Phase 2 (fails this phase's gates), or reject spread at evaluation until Phase 2 (simpler, but a parsed spread would fail every document). Phase 2 replaces the placeholder error with `InvalidSpreadOperand` / `dm.expression.invalid_spread_operand` and the "unknown" typing with ordered spread inference; nothing else needs redoing.
    - |-
        A cache-only remote runtime registers nothing when asked to fetch: `register_and_fetch` returns without creating a slot, so `get_content` reports the URL as never registered, and editor reads must go through `cached_text(url, now)`. The plan only said this path must never register a fetch task. The alternative was to fill the slot synchronously from the cache (hit becomes a ready body, miss a failure), which would let existing `get_content` callers work unchanged but would turn "pending" into a read failure. If the reviewer prefers that, only `register_and_fetch`'s cache-only branch and its tests change.
    - |-
        A spread operand that turns out, when evaluated, to be neither an object nor null now stops composition everywhere, including in the body, where most evaluation errors are only warnings and leave the `{{ }}` text in place. The spec said only that the error is "checked when evaluated" and reported at the operand. I treated it like the passive rejection of the same mistake, and like a call that returns the wrong type: no partial object should stand in for one that could not be built. The alternative is to treat it like an arithmetic type error, which becomes a warning in lenient body interpolation. That would make the code `dm.expression.evaluation_failure` on that path unless the warning path were also taught the new code and operand location. To switch, change one line in `ExpressionError::is_authoring_fatal`, its row in the error tests, and the docs sentence in `docs/inline/call-check.md`.
    - |-
        The passive "invalid spread operand" finding travels as an ordinary call-check record whose function name is `...`, with no arguments and the new `CallProblem::InvalidSpreadOperand` problem; the plan named the problem variant but not the carrier. Reusing the call-check record means composition's precheck, its error location, the lifecycle checks, and DMLS diagnostics all report it with no new plumbing. The cost is a pseudo-call in a list that otherwise holds only calls: any consumer that looks up the function by name must ignore `...`, and a lifecycle warning would read "`...(…)` in lifecycle …". The alternative is a separate spread-check type with its own path through each of those consumers, which is more code but keeps the call list pure.
    - |-
        The passive check rejects a spread only when every possible value of the operand fails: its type excludes null and no alternative can be an object. An operand that may be null (`maybe: number`), may be an object (`object | number`), or has an unknown type is left to evaluation. The message names JSON types ("found number", "found number | string"), worded like the runtime error rather than as the declared type text (`literal(1.0) | literal(2.0)`). The alternative of also warning on "may fail" operands, in the way the uncertain-call category warns, was not specified and would need a new severity rule.
    - |-
        Provenance tracking (which `load_data` call, if any, produced a property's final value) uses a per-thread "origin of the last evaluated value" slot, switched on only while a whole-value frontmatter expression is evaluated. Each evaluated expression node clears it, except parentheses and the ternary operator (whose last evaluated child is the selected branch), and a `load_data` call sets it after a successful read. The plan described a record keyed by each executing node of the expression tree. Both give the same answer, and neither compares values or evaluates a branch twice; the slot needs no node identities and costs one thread-local check per node. If the reviewer wants the keyed record (for example because Phase 4 needs per-call facts for callbacks), only `expression/trace.rs` and the wrapper in `evaluate_expr` change; the stored `LoadOrigin`, the provenance record, and the reporting stay as they are.
    - |-
        `try(...)` still recovers malformed data, including the new header-name failures (an empty camelCase key, two headers that collide) and an explicit name list of the wrong width. The plan's test matrix said malformed content "remains non-recoverable through try", but the existing code and the Loading Data guide ("try covers a file that is present but bad") recover it, and the spec says malformed data keeps its current failure behavior. I kept the documented behavior and pinned it (`load_data_uncontracted::malformed_names_are_recovered_like_other_malformed_data`). Row-shape conflicts and invalid options stay non-recoverable. Making malformed data non-recoverable would mean changing `ExpressionError::is_recoverable_read_failure`, the guide, and the existing recovery tests.
    - |-
        A wrapped scalar (a data file whose root is `5`, loaded as `[5]`) is reported against the data file, not the wrapped array. A failure at element 0 points at the file's root (pointer ""), and so does a failure at a later element the file cannot hold (for example a two-element tuple contract). The original contract-mismatch records used the loaded value's pointer (`/0`, `/1`). The alternative was to keep those pointers while still giving the root's line and column. Reverting means editing `report_diagnostics::source_path` and one pointer line in `contract_records`.
    - |-
        A destination-schema failure on an unexpected member is placed at that member's value in the data file (for example column 28 of `"extra": 1`), which is how the loader already places contract mismatches. The member's key would be another reasonable place. Changing it means having `ExpressionProjector::project` ask the locator for the key position.
    - |-
        The status block (the human-readable error) adds one line per located problem: `data: <file>:<line>:<column> (<pointer>)`, with the file as a link, and the problem's ` at L:C` now names the expression's position instead of the frontmatter key's. The plan asked to "show both locations" but gave no layout. The text is assembled in `errors/blocks.rs::schema_validation_failed_block`.
    - |-
        The editor-facing provisional type of a contract-free `load_data` call narrows to rows only when the file's format and the header option are both literal. `header: "false"` written as a string (which the argument binder turns into `false`) is not narrowed and keeps the general `open object | unknown[]` type. Narrowing it would require binding the options inside the type function, as the passive options check already does.
    - |-
        The initial implementation admitted the whole property expression before evaluation, including unused branches. Review 1 rejected that behavior; its repair now admits calls and resolves bindings only when evaluation reaches them. An executed unsupported operation leaves the property pending before it runs; an unused operation does not prevent observing the selected result.

        Correction verified 2026-11-21 during Review 2: the former present-tense statement that admission happens once over the whole expression no longer describes the implementation. Verified `admit_editor_call` in expression/mod.rs, `PassLookup` in slots/observe.rs, and the public-observation and DMLS unused-branch regression tests in the 721-test focused run. Schema-slot admission remains a separate, whole-expression contract.
    - |-
        In the editor, `fm.X` of a property whose authored value is still a template is pending, while a bare `X` evaluates that other property first and uses its result. This follows the plan's wording that `fm` sees the authored seed. If compose actually exposes interpolated values through `fm` at that point, `fm.X` should evaluate the dependency too; only the binding step in `observe.rs` changes.
    - |-
        A property expression that does not parse is reported as pending in the editor, not as a failure, because the passive call check already reports the syntax problem at the same place. Reporting it as a failure would need deduplication against that passive diagnostic.
    - |-
        Deciding which properties are file-dependent (and so get no editor verdict) searches the property's compiled JSON Schema declaration, following references, for the eager file format or the `match()` keyword. An unresolvable reference or nesting deeper than 32 levels also counts as file-dependent. This can over-defer but never probes a file. The alternative, reading the SimplifiedSchema declaration, is unavailable for raw JSON Schema layers.
    - |-
        When the schema root links properties together (a root union, or `anyOf`/`oneOf`/`allOf`/`if`/`not`/`dependent*` at the root), every observed property is deferred if any other property is still a template, and also if any file-dependent property is present. A root-level problem with no property of its own on a fully known document is reported on every observed property. Narrowing either needs an analysis of which arms touch which property.
    - |-
        Observed values feed completion and hover only. Diagnostics such as undeclared-member warnings keep using declared types, because an observed row that happens to lack an optional field would otherwise produce false warnings. A missing required member inside an observed value is always an error, even when DMLS is not in strict mode, since compose would fail on it.
    - |-
        Per-call observed values (used to type a callback's parameter, as in `map(load_data('data.yaml'), i => i.name)`) are matched by the call as written, not by its position, and the values of the same call text in several properties are merged (up to eight distinct values, then the call stays provisional). Position matching would need every editor parse to use the same coordinates, which the completion prefix parse does not.
    - |-
        Every slot job, not only a property observation, is now dropped and re-run when something it read changed between dispatch and install (including any grant or configuration change). This makes freshness uniform at the cost of occasional repeated work after two quick invalidations.
    - |-
        DMLS tests seed the transport cache through a new library feature, `cache-test-hooks` (`compose::cache_test_hooks::seed_remote_entry`), enabled only by DMLS's dev-dependency. The alternative is making the cache module's types public.
    - |-
        Editor diagnostics for a missing or failed load use the source name `darkmatter.compose`, and a destination failure's related data-file location is a zero-width range at the value's start. Hover still shows the raw observed preview for a headerless, uncontracted CSV read, which gets no verdict and no shape.
    - |-
        To clear the Darkmatter corpus gate (`shipped_markdown_named_type_references_resolve`), I edited a research document outside this feature, `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`. Its `$schema` used `apps: app[]` with `app` declared in a top-level `types:` block, a form only `kind: schema` files support, so the type never resolved. I wrote the object type inline (`"{ name: string(required), urls: url(required)[], support: enum(true,false,unknown; required) }[]"`) and dropped the three field descriptions that were attached with `->`. The alternatives were to leave the gate red, as Phases 3 and 4 did, or to move the type into a separate `kind: schema` file and import it with `app@file`, which keeps the descriptions but adds a file. If the author prefers the import, only that document changes.
    - |-
        The generated `load_data` page said `header: false` "needs a list-row `as`" and that a name list "needs an object-row `as`". That reads as if a contract were required, but without `as` these modes return `unknown[][]` and verbatim-keyed objects. I reworded the catalog description in `darkmatter/schemas/partials/functions.yaml` ("an `as` must then have list rows" / "object rows") and regenerated both shipped artifacts and the page. The alternative was to leave the catalog text alone and document the nuance only in the guide. Reverting means restoring one sentence and running `just shipped-schemas` and `just regen-expr-doc`.
    - |-
        The caller skill (`use-darkmatter/authoring.md`) summarizes `load_data` and links to the canonical Loading Data guide instead of transcluding a passage of it. The repository convention prefers transclusion, but the guide's prose has inline `{{ … }}` examples that would be evaluated (or fail under `--deny all`) at publication unless included with `interpolate=false`. Switching to transclusion means choosing a passage, adding `::file … interpolate=false`, and checking the published text.
source_files_during_phase_2:
    - "darkmatter/lib/src/markdown/compose/expression/spread.rs"
    - "darkmatter/lib/src/markdown/compose/expression/error.rs"
    - "darkmatter/lib/src/markdown/compose/expression/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/call_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/projection.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/policy.rs"
    - "darkmatter/lib/src/markdown/compose/preflight/collect.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/admission.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs"
    - "darkmatter/lib/tests/l1/object_spread.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs"
    - "darkmatter/dmls/src/diagnostics/codes.rs"
    - "darkmatter/dmls/src/diagnostics/call_check.rs"
    - "darkmatter/dmls/tests/l1/object_spread_editor.rs"
    - "darkmatter/dmls/tests/l1/main.rs"
    - "claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs"
    - "claudine/lib/src/dispatch/matcher/tests.rs"
docs_updated_during_phase_2:
    - "darkmatter/docs/inline/call-check.md"
    - "darkmatter/dmls/docs/diagnostics.md"
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - ".claude/skills/darkmatter/expressions.md"
source_files_during_phase_3:
    - "claudine/lib/generated/shipped.json"
    - "claudine/lib/src/composition/schema/translate.rs"
    - "darkmatter/cli/schemas/output/clean.yaml"
    - "darkmatter/cli/schemas/output/code-block.yaml"
    - "darkmatter/cli/schemas/output/compose.yaml"
    - "darkmatter/cli/schemas/output/delta.yaml"
    - "darkmatter/cli/schemas/output/edit.yaml"
    - "darkmatter/cli/schemas/output/get.yaml"
    - "darkmatter/cli/schemas/output/graph.yaml"
    - "darkmatter/cli/schemas/output/hash.yaml"
    - "darkmatter/cli/schemas/output/init.yaml"
    - "darkmatter/cli/schemas/output/list-themes.yaml"
    - "darkmatter/cli/schemas/output/render.yaml"
    - "darkmatter/cli/schemas/output/rm.yaml"
    - "darkmatter/cli/schemas/output/schema-about.yaml"
    - "darkmatter/cli/schemas/output/schema-detect.yaml"
    - "darkmatter/cli/schemas/output/schema-triggers.yaml"
    - "darkmatter/cli/schemas/output/schema-validate.yaml"
    - "darkmatter/cli/schemas/output/set.yaml"
    - "darkmatter/cli/schemas/output/toc.yaml"
    - "darkmatter/cli/schemas/output/validate-refs.yaml"
    - "darkmatter/cli/src/commands/frontmatter.rs"
    - "darkmatter/cli/tests/l1/load_data.rs"
    - "darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json"
    - "darkmatter/dmls/tests/l1/option_arguments.rs"
    - "darkmatter/features/2026-11-20-schema-typed-load-data/implementation-log.md"
    - "darkmatter/lib/generated/shipped.json"
    - "darkmatter/lib/src/markdown/compose/expression/call_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/contract.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/error.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/load.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/locate.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/options.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/parse.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/runtime.rs"
    - "darkmatter/lib/src/markdown/compose/expression/error.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/strings.rs"
    - "darkmatter/lib/src/markdown/compose/expression/mod.rs"
    - "darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs"
    - "darkmatter/lib/src/markdown/compose/interpolation/mod.rs"
    - "darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs"
    - "darkmatter/lib/src/markdown/compose/pipeline/mod.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/schema_validation.rs"
    - "darkmatter/lib/src/markdown/compose/value_origin.rs"
    - "darkmatter/lib/src/markdown/errors/blocks.rs"
    - "darkmatter/lib/src/markdown/mod.rs"
    - "darkmatter/lib/src/markdown/types.rs"
    - "darkmatter/lib/tests/l1/diagnostic_source_provenance.rs"
    - "darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs"
    - "darkmatter/lib/tests/l1/load_data_execution.rs"
    - "darkmatter/lib/tests/l1/load_data_options.rs"
    - "darkmatter/lib/tests/l1/load_data_options_passive.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/schemas/partials/functions.yaml"
    - "darkmatter/schemas/partials/lifecycle.yaml"
    - "darkmatter/lib/src/markdown/compose/expression/trace.rs"
    - "darkmatter/lib/tests/l1/load_data_destination.rs"
    - "darkmatter/lib/tests/l1/load_data_uncontracted.rs"
docs_updated_during_phase_3:
    - "darkmatter/docs/functions/load-data.md"
    - "darkmatter/docs/topics/darkmatter-expressions.md"
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - ".claude/skills/darkmatter/expressions.md"
source_files_during_phase_4:
    - "darkmatter/dmls/Cargo.toml"
    - "darkmatter/dmls/src/clock.rs"
    - "darkmatter/dmls/src/diagnostics/frontmatter.rs"
    - "darkmatter/dmls/src/diagnostics/mod.rs"
    - "darkmatter/dmls/src/diagnostics/observed.rs"
    - "darkmatter/dmls/src/overlay/mod.rs"
    - "darkmatter/dmls/src/overlay/properties.rs"
    - "darkmatter/dmls/src/providers/dsl.rs"
    - "darkmatter/dmls/src/providers/frontmatter.rs"
    - "darkmatter/dmls/src/router.rs"
    - "darkmatter/dmls/src/slot_jobs.rs"
    - "darkmatter/dmls/tests/common/mod.rs"
    - "darkmatter/dmls/tests/l1/main.rs"
    - "darkmatter/dmls/tests/l1/typed_load_data_editor.rs"
    - "darkmatter/lib/Cargo.toml"
    - "darkmatter/lib/src/config/permissions.rs"
    - "darkmatter/lib/src/markdown/compose/cache/remote_cache.rs"
    - "darkmatter/lib/src/markdown/compose/cache/store.rs"
    - "darkmatter/lib/src/markdown/compose/cache_test_hooks.rs"
    - "darkmatter/lib/src/markdown/compose/expression/call_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/load.rs"
    - "darkmatter/lib/src/markdown/compose/expression/editor_reads.rs"
    - "darkmatter/lib/src/markdown/compose/expression/error.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/fs.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/read_gate.rs"
    - "darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/projection.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/tests.rs"
    - "darkmatter/lib/src/markdown/compose/mod.rs"
    - "darkmatter/lib/src/markdown/compose/remote_fetch.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/schemas/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/observed.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/admission.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/evaluator.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/observe.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/observe_tests.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/session.rs"
    - "darkmatter/lib/tests/l1/editor_observation.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/lib/tests/l1/observed_validation.rs"
    - "darkmatter/lib/tests/l1/semantic_results_never_persist.rs"
    - "simplified-schema/lib/src/file_match.rs"
    - "simplified-schema/lib/src/format.rs"
    - "simplified-schema/lib/src/validate.rs"
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - ".claude/skills/darkmatter/dmls.md"
source_files_during_phase_5:
    - "darkmatter/schemas/partials/functions.yaml"
    - "darkmatter/lib/generated/shipped.json"
    - "claudine/lib/generated/shipped.json"
    - "claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap"
docs_updated_during_phase_5:
    - "darkmatter/README.md"
    - "darkmatter/docs/topics/loading-data.md"
    - "darkmatter/docs/topics/darkmatter-expressions.md"
    - "darkmatter/docs/topics/caching.md"
    - "darkmatter/docs/topics/schemas/schema-slots.md"
    - "darkmatter/docs/inline/interpolation.md"
    - "darkmatter/docs/lsp/features.md"
    - "darkmatter/docs/cli/data-contracts.md"
    - "darkmatter/docs/errors/composition-problems.md"
    - "darkmatter/docs/functions/load-data.md"
    - "darkmatter/dmls/docs/autocomplete.md"
    - "darkmatter/dmls/docs/hover.md"
    - "darkmatter/dmls/docs/diagnostics.md"
    - "darkmatter/docs/use-skills/use-darkmatter/authoring.md"
    - "simplified-schema/docs/topics/expression-syntax.md"
    - "biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md"
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - ".claude/skills/darkmatter/expressions.md"
    - ".claude/skills/darkmatter/requests.md"
    - ".claude/skills/darkmatter/dmls.md"
    - ".claude/skills/simplified-schema/SKILL.md"
source_code:
    - "simplified-schema/lib/src/expression/ast.rs"
    - "simplified-schema/lib/src/expression/lexer.rs"
    - "simplified-schema/lib/src/expression/mod.rs"
    - "simplified-schema/lib/src/expression/parser.rs"
    - "simplified-schema/lib/src/simplified/grammar.rs"
    - "simplified-schema/lib/tests/l1/expression_syntax.rs"
    - "darkmatter/lib/src/markdown/compose/cache/remote_cache.rs"
    - "darkmatter/lib/src/markdown/compose/remote_fetch.rs"
    - "darkmatter/lib/src/markdown/compose/expression/absence.rs"
    - "darkmatter/lib/src/markdown/compose/expression/call_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/lint.rs"
    - "darkmatter/lib/src/markdown/compose/expression/member_check.rs"
    - "darkmatter/lib/src/markdown/compose/expression/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/prepared.rs"
    - "darkmatter/lib/src/markdown/compose/preflight/collect.rs"
    - "darkmatter/lib/src/markdown/compose/remote.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/admission.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/evaluator.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/targets.rs"
    - "darkmatter/lib/tests/l1/more_is_more_literals_and_indexes.rs"
    - "darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs"
    - "darkmatter/lib/tests/l1/semantic_results_never_persist.rs"
    - "darkmatter/dmls/src/overlay/expressions.rs"
    - "claudine/lib/src/composition/lifecycle/prewarm/classify.rs"
    - "claudine/lib/src/dispatch/matcher.rs"
    - "darkmatter/lib/src/markdown/compose/expression/spread.rs"
    - "darkmatter/lib/src/markdown/compose/expression/error.rs"
    - "darkmatter/lib/src/markdown/compose/expression/diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/projection.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs"
    - "darkmatter/lib/src/markdown/compose/report_diagnostics.rs"
    - "darkmatter/lib/src/markdown/compose/policy.rs"
    - "darkmatter/lib/tests/l1/object_spread.rs"
    - "darkmatter/lib/tests/l1/main.rs"
    - "darkmatter/dmls/src/diagnostics/codes.rs"
    - "darkmatter/dmls/src/diagnostics/call_check.rs"
    - "darkmatter/dmls/tests/l1/object_spread_editor.rs"
    - "darkmatter/dmls/tests/l1/main.rs"
    - "claudine/lib/src/composition/lifecycle/prewarm/classify/tests.rs"
    - "claudine/lib/src/dispatch/matcher/tests.rs"
    - "claudine/lib/generated/shipped.json"
    - "claudine/lib/src/composition/schema/translate.rs"
    - "darkmatter/cli/schemas/output/clean.yaml"
    - "darkmatter/cli/schemas/output/code-block.yaml"
    - "darkmatter/cli/schemas/output/compose.yaml"
    - "darkmatter/cli/schemas/output/delta.yaml"
    - "darkmatter/cli/schemas/output/edit.yaml"
    - "darkmatter/cli/schemas/output/get.yaml"
    - "darkmatter/cli/schemas/output/graph.yaml"
    - "darkmatter/cli/schemas/output/hash.yaml"
    - "darkmatter/cli/schemas/output/init.yaml"
    - "darkmatter/cli/schemas/output/list-themes.yaml"
    - "darkmatter/cli/schemas/output/render.yaml"
    - "darkmatter/cli/schemas/output/rm.yaml"
    - "darkmatter/cli/schemas/output/schema-about.yaml"
    - "darkmatter/cli/schemas/output/schema-detect.yaml"
    - "darkmatter/cli/schemas/output/schema-triggers.yaml"
    - "darkmatter/cli/schemas/output/schema-validate.yaml"
    - "darkmatter/cli/schemas/output/set.yaml"
    - "darkmatter/cli/schemas/output/toc.yaml"
    - "darkmatter/cli/schemas/output/validate-refs.yaml"
    - "darkmatter/cli/src/commands/frontmatter.rs"
    - "darkmatter/cli/tests/l1/load_data.rs"
    - "darkmatter/dmls/tests/fixtures/response_contracts/a3_completions.json"
    - "darkmatter/dmls/tests/l1/option_arguments.rs"
    - "darkmatter/lib/generated/shipped.json"
    - "darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/contract.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/error.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/load.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/locate.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/options.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/parse.rs"
    - "darkmatter/lib/src/markdown/compose/expression/data/runtime.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/mod.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/strings.rs"
    - "darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs"
    - "darkmatter/lib/src/markdown/compose/interpolation/mod.rs"
    - "darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs"
    - "darkmatter/lib/src/markdown/compose/pipeline/mod.rs"
    - "darkmatter/lib/src/markdown/compose/schema_validation.rs"
    - "darkmatter/lib/src/markdown/compose/value_origin.rs"
    - "darkmatter/lib/src/markdown/errors/blocks.rs"
    - "darkmatter/lib/src/markdown/mod.rs"
    - "darkmatter/lib/src/markdown/types.rs"
    - "darkmatter/lib/tests/l1/diagnostic_source_provenance.rs"
    - "darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs"
    - "darkmatter/lib/tests/l1/load_data_execution.rs"
    - "darkmatter/lib/tests/l1/load_data_options.rs"
    - "darkmatter/lib/tests/l1/load_data_options_passive.rs"
    - "darkmatter/schemas/partials/functions.yaml"
    - "darkmatter/schemas/partials/lifecycle.yaml"
    - "darkmatter/lib/src/markdown/compose/expression/trace.rs"
    - "darkmatter/lib/tests/l1/load_data_destination.rs"
    - "darkmatter/lib/tests/l1/load_data_uncontracted.rs"
    - "darkmatter/dmls/Cargo.toml"
    - "darkmatter/dmls/src/clock.rs"
    - "darkmatter/dmls/src/diagnostics/frontmatter.rs"
    - "darkmatter/dmls/src/diagnostics/mod.rs"
    - "darkmatter/dmls/src/diagnostics/observed.rs"
    - "darkmatter/dmls/src/overlay/mod.rs"
    - "darkmatter/dmls/src/overlay/properties.rs"
    - "darkmatter/dmls/src/providers/dsl.rs"
    - "darkmatter/dmls/src/providers/frontmatter.rs"
    - "darkmatter/dmls/src/router.rs"
    - "darkmatter/dmls/src/slot_jobs.rs"
    - "darkmatter/dmls/tests/common/mod.rs"
    - "darkmatter/dmls/tests/l1/typed_load_data_editor.rs"
    - "darkmatter/lib/Cargo.toml"
    - "darkmatter/lib/src/config/permissions.rs"
    - "darkmatter/lib/src/markdown/compose/cache/store.rs"
    - "darkmatter/lib/src/markdown/compose/cache_test_hooks.rs"
    - "darkmatter/lib/src/markdown/compose/expression/editor_reads.rs"
    - "darkmatter/lib/src/markdown/compose/expression/functions/fs.rs"
    - "darkmatter/lib/src/markdown/compose/expression/read_gate.rs"
    - "darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs"
    - "darkmatter/lib/src/markdown/compose/expression/scope/tests.rs"
    - "darkmatter/lib/src/markdown/compose/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/observed.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/grants/gate.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/mod.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/observe.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/observe_tests.rs"
    - "darkmatter/lib/src/markdown/schemas/slots/session.rs"
    - "darkmatter/lib/tests/l1/editor_observation.rs"
    - "darkmatter/lib/tests/l1/observed_validation.rs"
    - "simplified-schema/lib/src/file_match.rs"
    - "simplified-schema/lib/src/format.rs"
    - "simplified-schema/lib/src/validate.rs"
    - "claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap"
documentation:
    - ".claude/skills/darkmatter/remote-cache.md"
    - "darkmatter/docs/inline/call-check.md"
    - "darkmatter/dmls/docs/diagnostics.md"
    - ".claude/skills/darkmatter/expressions.md"
    - "darkmatter/docs/functions/load-data.md"
    - "darkmatter/docs/topics/darkmatter-expressions.md"
    - ".claude/skills/darkmatter/dmls.md"
    - "darkmatter/README.md"
    - "darkmatter/docs/topics/loading-data.md"
    - "darkmatter/docs/topics/caching.md"
    - "darkmatter/docs/topics/schemas/schema-slots.md"
    - "darkmatter/docs/inline/interpolation.md"
    - "darkmatter/docs/lsp/features.md"
    - "darkmatter/docs/cli/data-contracts.md"
    - "darkmatter/docs/errors/composition-problems.md"
    - "darkmatter/dmls/docs/autocomplete.md"
    - "darkmatter/dmls/docs/hover.md"
    - "darkmatter/docs/use-skills/use-darkmatter/authoring.md"
    - "simplified-schema/docs/topics/expression-syntax.md"
    - "biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md"
    - ".claude/skills/darkmatter/requests.md"
    - ".claude/skills/simplified-schema/SKILL.md"
completed_phase: 5
implemented: true
---

# Implementation Log for 2026-11-20-schema-typed-load-data (5 phases)

## Phase 1

### Add spread syntax (Simplified Schema)

- `expression/lexer.rs`: new `Token::Ellipsis` for three adjacent dots (one
  token, three-byte span). A lone `.` is still `Token::Dot`.
- `expression/ast.rs`: `Expr::ObjectLiteral(Vec<ObjectEntry>)` and
  `SpannedExprKind::ObjectLiteral(Vec<SpannedObjectEntry>)`, each enum with
  `Field { key, value }` / `Spread { operand }` and a borrowed `value()`
  accessor; `SpannedObjectEntry::erase`, and `Display` prints `...operand`.
  Both enums are re-exported from `simplified_schema::expression` and from
  `darkmatter::markdown::compose::expression` (its public `Expr` names them).
- `expression/parser.rs`: `parse_object_literal` accepts `... expression` only
  at the start of an entry. The explicit-key `HashSet` survives across spreads,
  so `{ a, ...rest, a }` is still `duplicate object key "a"` at the second key.
  `[...x]` is rejected with "array spread is not supported"; `...` anywhere
  else is the ordinary "expected expression" error. Grammar doc updated.
- `simplified/grammar.rs`: the literal-example extractor rejects a spread at
  any depth ("`...base` is not a literal; a spread is not allowed in a literal
  example").
- `expression::scan` is unchanged and still asks the parser; a test pins
  `{{ {...base, nested: {k: 1}}}}` closing at the right `}}`.

### Interim consumer adaptation (needed for Phase 1 to compile)

The AST change breaks every tuple-destructuring consumer, and Phase 2 owns
their real spread semantics. To keep Darkmatter, DMLS, and Claudine compiling
and green in this phase:

- pure walkers (absence, call-node visits, prepared, member_check pending
  lists, preflight `visit_calls_in_scope`, `remote.rs`, slot admission /
  evaluator / grant targets, DMLS `overlay/expressions.rs` x3, Claudine
  `prewarm/classify.rs` x2 and `dispatch/matcher.rs`) now visit
  `entry.value()`, i.e. spread operands too — the behavior Phase 2 asks for;
- `lint.rs` flags only written keys and substitutes inside spread operands;
- `call_check.rs`: `expression_type` returns `Unknown { non_null: true }` for a
  literal containing a spread, `partial_object` and `literal_value` return
  `None` for one (no passive option inspection, never a literal);
- the evaluator (`expression/mod.rs`) merges entries in written order, last
  key wins, `null` spread contributes nothing; any other operand fails with
  `ExpressionError::Other { function: "...", message: "object spread requires
  an object or null, found <type>" }` as a placeholder for Phase 2's typed
  `InvalidSpreadOperand` / `dm.expression.invalid_spread_operand`.

### Add cache lookup (Darkmatter)

- `cache/remote_cache.rs`: `read_cached` is split into a side-effect-free
  `decode_cached` (Absent / NoStore / Unusable / Usable) shared by both paths;
  the fetching path still purges a `no-store` entry exactly as before.
  New `lookup_fresh(store, url, now) -> Result<FreshCachedBody, CacheOnlyMiss>`
  serves only a current-version, decodable, non-`no-store`, non-`no-cache`
  entry whose `expires_at` is after `now`, and returns body plus expiry.
- `remote_fetch.rs`: `RemoteFetchRuntime::cache_only(policy, network, store)`
  builds a runtime whose `register_and_fetch` / `register_nested` register
  nothing (no task, no slot); `cached_text(url, now)` checks scheme
  (HTTP(S)), then the `network` configuration error, then the host policy
  (`check_allowed`), and only then opens the store, mapping every miss to
  `CacheOnlyPending::{NotHttp, Refused, NoStore, Miss(..)}`.
  `with_other_policy` keeps a cache-only runtime cache-only. The store root is
  whatever the caller passes; Phase 4 opens
  `FileStore::resolve_cache_root(context.base_dir(), None)`.
- The new items carry `#[cfg_attr(not(test), expect(dead_code, …))]` until
  Phase 4 consumes them; the `expect` will fail the build once they are used,
  so it must be removed then.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| spread order, shorthand, operand is a full expression | `parser::tests::object_spread::{entries_keep_written_order, spread_operand_is_a_whole_expression}`; L1 `expression_syntax::object_spread_keeps_entry_order_and_operand_spans` |
| explicit duplicates across a spread (incl. shorthand / quoted) rejected at the second key | `object_spread::written_duplicates_separated_by_a_spread_are_rejected`; L1 `object_spread_rejects_written_duplicates_and_array_spread` |
| a spread never conflicts with a written key at parse time | `object_spread::spread_may_repeat_a_written_key` |
| nesting | `object_spread::nested_spreads_parse_inside_operands_and_values` |
| interpolation closer with adjacent `}}}}` | L1 `a_spread_with_nested_objects_closes_at_the_interpolation_closer` |
| `...` rejected outside an object entry, array spread, malformed entries | `object_spread::{spread_is_rejected_outside_an_object_entry, malformed_spread_entries_are_rejected}` |
| spans (operand, key, value) and erasure parity | `object_spread::spread_spans_cover_the_operand_and_keys_keep_theirs`; `spanned::erasure_matches_parse` corpus (+2 spread rows) |
| catalog example literal rejects spread at any depth | L1 `a_catalog_example_result_rejects_spread_at_any_depth` (control + 4 shapes) |
| interim evaluator merge / null / wrong type | darkmatter L1 `more_is_more_literals_and_indexes::object_spread_merges_in_written_order_and_rejects_a_non_object` |
| cache-only: fresh hit with expiry | `remote_fetch::cache_only_tests::a_fresh_entry_returns_its_body_and_expiry` |
| stale (== now, < now, no lifetime) | `…::an_entry_expiring_now_or_earlier_or_without_a_lifetime_is_expired` |
| miss / no store | `…::a_missing_entry_or_store_is_pending` (also: no store directory created) |
| denied host before cache | `…::a_denied_host_is_refused_before_the_cache_is_read` |
| invalid permission block outranks an allowed host | `…::an_invalid_network_permission_outranks_an_allowed_host` |
| no-store (current + legacy version), no-cache | `…::no_store_and_no_cache_entries_are_never_served_or_purged` |
| corrupt body / missing blob / other version / bad manifest JSON | `…::corrupt_or_foreign_entries_are_unusable` |
| non-HTTP URL | `…::a_non_http_url_never_reaches_the_cache` |
| slot-derived runtime stays cache-only | `…::a_derived_slot_runtime_stays_cache_only` |
| zero requests, zero writes, no registration | every cache-only test goes through `observe`: wiremock `.expect(0)` + `verify`, byte+mtime snapshot of the store before/after, `registered_urls()` empty, `get_content` = `Ok(None)` |
| normal path still purges `no-store` | existing `remote_cache` / `persistent_cache_tests` (all pass) |

The deterministic clock is an injected `now` argument (`UNIX_EPOCH + 1e6 s`).
The input-robustness matrix does not apply to this phase: no file-format or
configuration reader changed (the cache manifest decode was refactored, not
changed, and its shapes are covered above).

### Other changes found by the gates

- `darkmatter/lib/tests/l1/semantic_results_never_persist.rs`: its exact
  `FileStore` allowlist counts rose with the new transport-cache code
  (`remote_cache.rs` 6 → 8: shared decode + `lookup_fresh`; `remote_fetch.rs`
  5 → 6: `cache_only`). Every new mention is transport-cache code, so no
  semantic result gains a path to disk. The comments now name the new uses.
- clippy `large_enum_variant`: `StoredEntry::Usable` holds a `Box<CachedEntry>`.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  → `packages: [claudine, darkmatter, darkmatter-lsp, simplified-schema, test-toolkit, darkmatter-cli]`,
  `change_class: package`, so the gates ran.
- `simplified-schema`: `just test` 2017/2017 passed; `just lint` clean;
  `just check-tier-coverage simplified-schema` 0 stranded.
- `darkmatter`: `just test` 17274/17274 passed, none at or above 5 s;
  `just lint` clean (including the zed-dmls wasm check);
  `just check-tier-coverage darkmatter` 0 stranded. The opt-in `prompts`
  binary (`--features prompt-tests`) compiles.
- `claudine` (edited directly): `just test-library` 6932/6932 passed, none
  slow; `just lint` clean.
- No other workspace crate names `ObjectLiteral` (repo-wide search), and the
  cache API is `pub(crate)`, so no further dependent can be affected.

### Problems that existed before this phase

- `horizontal_rule_integration::tests::test_custom_weight_thick_differs_from_thin`
  fails when the test process runs with `LANG=C`, as this agent shell did
  (`TERM_PROGRAM=rex`). The terminal rule falls back to ASCII, and thin and
  thick then render the same `-`. It passes alone and in the full suite under
  `LANG=C.UTF-8`, which is how the recorded runs above were made. It is not
  caused by this phase: the test reads the ambient locale, so it depends on the
  host it runs on.
- `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
  showed as modified during this session. This phase did not edit it, and it
  is left alone.

## Phase 2

### Integrate spread evaluation (Darkmatter library)

- `expression/error.rs`: new `ExpressionError::InvalidSpreadOperand { found, site }`
  ("object spread requires an object or null, found number"). It is
  authoring-fatal and not a recoverable read (`try` does not absorb it).
  `ExpressionError::placed_in(root)` fills in the failing operand's site,
  looking through callback wrappers.
- `expression/spread.rs` (new): `SpreadSite` records the evaluated operand
  node, and `evaluate`/`evaluate_observed` turn it into the node's ordinal in
  `member_check::expr_nodes`, the existing erased-tree node list that
  `spanned_nodes` mirrors. `spread_operand_span(spanned, ordinal)` maps it back
  to the authored operand. A callback body is one node, so every iteration's
  failure names the same authored operand.
- `expression/mod.rs`: the evaluator raises the typed error; both entry points
  place its site. The public `evaluate` signature is unchanged.
- `report_diagnostics.rs`: `InvalidSpreadOperand` reports under
  `dm.expression.invalid_spread_operand`. An interpolation failure anchored to
  its `{{ … }}` is narrowed to the operand's line, column, and range by
  reparsing the recorded expression text with spans. If the expression text is
  not found verbatim in the construct, as in an escaped YAML scalar, the
  record keeps the construct's position.
- `expression/call_check.rs`:
  - `CallProblem::InvalidSpreadOperand { span, found }`, carried by a
    `CallCheck` named `...` (`SPREAD_CHECK_NAME`) and spanning the operand.
    The existing pipeline (compose call precheck, `rejected_call`,
    `call_diagnostic`, DMLS call diagnostics, lifecycle checks) therefore
    rejects and locates it with no new plumbing.
  - `check_expression_calls` now walks every node (`visit_nodes`, with
    `visit_call_nodes` as a filter over it). It checks each spread operand in
    its own `branch_scope`, so unselected branches and callback bodies are
    covered. The operand is rejected only when its type excludes `null` and
    no alternative can be an object.
  - `expression_type` folds object literals through
    `Shape::object_of_entries`, in the order written (see below).
  - `literal_value` folds literal-object and `null` spreads.
  - `partial_object` folds entries in order. An opaque spread clears the
    literal members read before it and sets a new
    `ArgumentType::PartialObject::opaque` flag. `OptionsView::Partial` reads
    that flag, so an opaque spread keeps the format open, while a literal
    written after it (for example `delimiter: ';;'`) is still judged.
  - `INVALID_SPREAD_OPERAND_CODE` is exported and listed in
    `COMPOSE_DIAGNOSTIC_CODES`. `diagnostics.rs` maps the problem to it.
- `scope/projection.rs`: `spread_fields`/`spread_always_fails` give, for each
  alternative of the operand, its declared keys (with required-ness) and
  whether it is open, or for a non-object the JSON type it has instead.
  Recursive references open through the same alias budget as `member_shape`.
- `scope/iterable.rs`: `Shape::object_of_entries` implements Rule 2's ordered
  inference.
  - A written field sets its key's type.
  - A spread replaces a key only when every alternative always writes it.
    Otherwise the key widens, from `null` when it was not written yet.
  - An unknown or open operand widens earlier keys to `unknown` and adds
    `<string>: unknown`, so later reads are never reported as missing members.
- Comment drift: `expression_type`'s table now mentions spreads, and
  `literal_value` and `partial_object` describe their folding.
- Pre-existing limitation, not changed: `serialize_property_atom` omits an
  inline object's pattern keys, so an open spread result displays like a
  closed one (`{a: unknown}`). The tests check openness through member
  findings instead.

### Adapt editor traversal (DMLS, done by a subagent and reviewed here)

- `overlay/expressions.rs` needed no change. Its three `ObjectLiteral`
  consumers already walk spread operands, and a temporary mutation (skipping
  spreads) made both new hover tests fail, which confirms the tests can catch
  a regression.
- `diagnostics/codes.rs`: `EXPRESSION_INVALID_SPREAD_OPERAND` aliases
  Darkmatter's `INVALID_SPREAD_OPERAND_CODE`, with an `ERROR` row in the
  module table. `diagnostics/call_check.rs`: module doc only. The finding
  already flows through `expression_call_diagnostics` → `CallCheck::diagnostics`.
- `dmls/docs/diagnostics.md`: new `dm.expression.invalid_spread_operand` row.
- New `dmls/tests/l1/object_spread_editor.rs`, declared in `tests/l1/main.rs`.
  Its tests run through an in-process LSP session in a temp workspace:
  hovering an operand, hovering a callback binding inside an operand,
  completion inside an operand (the same labels as a field-value position), a
  call in an overwritten operand that is still call-checked, and the
  invalid-operand error ranged exactly on `7` (frontmatter), `5`, and
  `[1, 2]`, under both `strict` settings, with no finding for `...null` or an
  undeclared operand.

### Adapt Claudine traversal (subagent, reviewed here)

- `classify.rs` and `matcher.rs` are unchanged. Five tests were added in
  `prewarm/classify/tests.rs` and `dispatch/matcher/tests.rs`:
  - a call, a sequence-injected read, `fm`, and a late root (`err`) inside an
    overwritten operand classify like the direct form;
  - a condition inside an overwritten operand qualifies a matcher as an
    expression;
  - a variable used only as an operand is read when the matcher runs.
- A mutation that skipped spreads failed three of the five tests. The other
  two document behavior that does not depend on the walker.

### Docs and skills

- `darkmatter/docs/inline/call-check.md`: a new "Object spread operands"
  subsection with what is rejected, what is not, and the runtime failure.
- `darkmatter/dmls/docs/diagnostics.md`: a new code row (from the DMLS
  subagent).
- `.claude/skills/darkmatter/expressions.md`: the `...` call-check record, the
  ordered typing, and the `SpreadSite` node-ordinal mirror.
- The plan schedules the spread syntax guide itself
  (`simplified-schema/docs/topics/expression-syntax.md`) for Phase 5.

### Requirement-to-test mapping

| Requirement | Test(s) |
|---|---|
| written order, last key wins, multiple spreads, null (literal and computed), nested copy | L1 `object_spread::evaluation::object_spread_entries_merge_in_written_order`; `more_is_more_literals_and_indexes::object_spread_merges_in_written_order_and_rejects_a_non_object` |
| each operand evaluated once, left to right, even when overwritten | L1 `object_spread::evaluation::object_spread_operands_are_evaluated_once_each_in_order` (counting test function) |
| dynamic wrong type: typed error, fatal, not `try`-recoverable, located at the operand (including in a callback) | L1 `object_spread::evaluation::object_spread_of_a_non_object_is_a_typed_fatal_error_at_its_operand`; unit `expression::spread::tests::*`; `error::tests::recovery` row |
| runtime error code and authored line/column in the body, a frontmatter value, and a callback (normal compose path) | L1 `object_spread::composition::object_spread_runtime_failure_is_located_at_its_operand_in_body_and_frontmatter` |
| passive rejection of a proven non-object (literal, typed, union, unselected branch, callback, nested), code, span, message | L1 `object_spread::passive::object_spread_of_a_proven_non_object_is_rejected_at_its_operand` |
| no passive finding for null, maybe-null, object, unknown, or mixed operands; calls in operands still checked | L1 `object_spread::passive::object_spread_operands_that_may_be_objects_or_null_are_left_to_evaluation` |
| compose rejects before anything runs, located at the operand | L1 `object_spread::composition::object_spread_proven_wrong_rejects_the_document_before_anything_runs` |
| ordered inference: replace, widen, unknown spread, later explicit write | L1 `object_spread::typing::object_spread_types_follow_written_order`, `…member_types_reach_callbacks_and_member_checks` |
| open result: no undeclared-member finding; closed result keeps one | `…member_types_reach_callbacks_and_member_checks` |
| literal option folding, opaque spread, a known option after `...opts` | L1 `object_spread::typing::object_spread_literal_options_fold_in_written_order`; unit `call_check::tests::a_partial_object_keeps_its_literal_members_and_classifies_as_its_shape` |
| reads in overwritten operands: remote discovery, preflight `as_markdown` walk, slot read targets, slot admission | L1 `object_spread::scans::object_spread_operand_urls_are_discovered_even_when_overwritten`; unit `preflight::collect::tests::object_spread_operands_are_scanned_even_when_overwritten`, `slots::grants::targets::tests::object_spread_operand_reads_are_collected_even_when_overwritten`, `slots::admission::tests::object_spread_operands_are_admitted_even_when_overwritten` |
| DMLS hover, completion, call checks inside operands; diagnostic code and range | DMLS L1 `object_spread_editor::*` (5 tests) |
| Claudine classification and matcher discovery | `prewarm::classify::tests::object_spread_operand_*` (2), `dispatch::matcher::tests::object_spread_operand_*` (3) |

The Input Robustness Matrix does not apply: no file-format or configuration
reader changed.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [claudine, darkmatter, darkmatter-lsp, test-toolkit, darkmatter-cli]`
  and `change_class: package`, so the gates ran.
- `simplified-schema` `just test`: 2017/2017 passed (its code is unchanged in
  this phase; the plan asks for the run).
- The plan's Darkmatter filter, `just test -E '… | …'`, cannot run through the
  recipe: it passes its arguments unquoted, so the parentheses break bash. I
  ran the same filter with nextest directly and the recipe's features:
  darkmatter 92/92, darkmatter-lsp 5/5, darkmatter-cli 0 matching.
- `darkmatter` `just test`: 12093 passed and 1 failed before fail-fast stopped
  the run, with no slow tests. The failure,
  `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`,
  reads `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`.
  That file has uncommitted edits made outside this phase (it was already
  modified during Phase 1). The edit adds `apps: app[]` to an inline
  `$schema` without a `types` definition the schema can see, and the test's
  message is "unknown named type `app` in the inline `$schema`". I left the
  file alone. I then ran the whole area (darkmatter, darkmatter-cli,
  darkmatter-lsp, zed-dmls-cli, with the recipe's features) through nextest
  excluding only that test: 17294/17294 passed, none slow.
- `darkmatter` `just lint`: clean, including the zed-dmls wasm check. A first
  run failed because `tests/l1/object_spread.rs` imported the
  instrumentation-only test hooks unconditionally; that one test is now
  gated on `effects-instrumentation`, the feature `just test` and CI enable.
- `just check-tier-coverage darkmatter`: 0 stranded.
- `claudine` `just test-library`: 6937/6937 passed, none slow.
  `cargo check -p claudine-cli --tests` is clean. Claudine is the only
  dependent that names `ExpressionError`, `CallProblem`, or `PartialObject`.
  Its `just lint` was run clean by the subagent.

## Phase 3

### What changed

- **Optional contract** (`functions.yaml`, `data/load.rs`): the catalog now
  declares `as?: type-definition(container) | null` and
  `header: boolean | string[]`. An omitted and a `null` `as` are the same
  call: no contract is prepared, the bind memo key's contract part is empty,
  and the value is returned unconverted (CSV cells stay text). A bound
  `null` file still returns `null` first. `plan_read`, `judge_absent`, and
  `bind_source` take `Option<&PreparedContract>`.
- **Root normalization** (`load.rs::bind_source`): arrays and objects kept,
  a `null` root (empty YAML, JSON `null`) becomes `[]`, any other scalar a
  one-element array, before a contract judges it. `LoadedData` gained
  `wrapped` and `table`; `DataError::ContractMismatch` gained `wrapped`, and
  its records (and destination records) place a wrapped scalar's paths at the
  document root with a root pointer.
- **Header modes** (`data/options.rs::HeaderMode::{FirstRecord, None,
  Names}`; `header_names` validates names after binding, never trims). The
  bind memo keys on the effective mode, names included. `load.rs::select_row_mode`
  picks rows from the contract (checked by the shared
  `contract::require_header_rows`, new `RowShapeConflict::NamedListRows`) or,
  without one, from the mode. Header-record keys are
  `functions::strings::to_camel_case` (extracted from `camel_case`);
  `header_keys` reports an empty key or a collision as `dm.data.malformed`
  naming the original headers at the first offending field, even with no data
  rows. `tabular_source` checks an explicit list against the first record's
  width (`dm.data.malformed` at the record, both counts).
  `TabularSource::keys` carries each column's key; `locate.rs::table_offset`
  finds cells by it.
- **Passive check** (`call_check.rs`): one-argument and `null`-contract
  calls; a literal `header` list goes through `header_names`
  (`invalid_options` at argument 2) and the row rule (`row_shape_conflict` at
  argument 1). Uncontracted calls get the provisional type
  (`uncontracted_type`): open object or `unknown[]`, narrowed for a provable
  CSV/TSV read to open-object rows or `unknown[][]` for `header: false`.
- **Compose provenance** (Rules 6–7, 15): new `expression/trace.rs`
  (`traced`, `record_load`, `derive`, `LoadOrigin`). `evaluate_expr` now wraps
  `evaluate_node`; with a trace open it clears the origin after every node
  except a parenthesized expression, a ternary, and a `load_data` call (whose
  handler records the origin after a successful bind and clears it on entry).
  `interpolation/rewrite.rs::interpolate_value_traced` opens the trace for a
  whole-value `{{ … }}`; `frontmatter_interpolation.rs` records a top-level
  key's `ExpressionOrigin` on `FrontmatterProvenance`
  (`set_expression_origin`, cleared by `set_authored_source`). Both
  `run_with_registry` calls and `validate_typed_shell_values` take the
  origins; an instance failure builds
  `MarkdownError::SchemaValidationFailed::expression_origins`
  (`ExpressionMetadata { span, load, value }`; empty for preparation
  failures). `report_diagnostics::ExpressionProjector` (one `SourceLocator`
  per property) feeds both `failure_diagnostics` (primary source and `span`
  at the expression, `data_location` for a direct load) and
  `errors/blocks.rs::schema_validation_failed_block` (" at L:C" of the
  expression and a linked `data: file:L:C (pointer)` line). No extra read or
  shape derivation is added to compose.
- **Callers**: Claudine's `schema/translate.rs` and `md set`'s
  `commands/frontmatter.rs` construct the error with empty origins. The 19
  CLI output schemas describe the wider `data_location` applicability.
  `just shipped-schemas` (twice: after `functions.yaml`, and after
  `just regen-lifecycle-schema` updated `schemas/partials/lifecycle.yaml`),
  and `just regen-expr-doc` regenerated `docs/functions/load-data.md` and
  the function table. The DMLS `a3_completions.json` golden was refreshed
  (`DMLS_UPDATE_GOLDEN=1`); its diff is only the new signature.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| Omitted/`null`/computed-null `as` share one read, parse, bind; cells stay text | `load_data_uncontracted::omitted_and_null_contracts_share_one_read_parse_and_bind` |
| Literal and computed `null` file: `null`, no reads, no imports | `load_data_uncontracted::a_null_file_returns_null_without_reads_or_imports` |
| Scalar wrapping (number, boolean, empty string), array contract accepts, tuple fails at the original root | `load_data_uncontracted::a_scalar_root_is_wrapped_before_the_contract` |
| Empty YAML, `~`, JSON `null`, empty JSONL → `[]`; empty JSON malformed | `load_data_uncontracted::a_null_root_is_an_empty_list`, `load_data_execution::empty_yaml_is_an_empty_list` |
| Three header modes without a contract (spec example) | `load_data_uncontracted::each_header_mode_chooses_the_row_shape_without_a_contract` |
| Header modes with compatible contracts, positional skip of unusable names | `load_data_uncontracted::each_header_mode_with_a_compatible_contract` |
| Explicit names verbatim, binder conversions, two lists → two binds over one parse | `load_data_uncontracted::explicit_names_are_verbatim_and_bind_separately`, `runtime::tests::a_header_choice_is_its_own_binding_over_one_parse` |
| Empty normalized names and collisions (zero data rows too) | `load_data_uncontracted::unusable_header_names_are_malformed_at_the_header`, `load_data_execution::an_object_header_must_not_repeat_a_name` |
| Explicit-name width mismatch with counts and coordinates; later ragged record | `load_data_uncontracted::explicit_names_must_match_the_first_record_width` |
| No-record input judged against a contract | `load_data_uncontracted::no_records_is_an_empty_list_judged_by_the_contract` |
| Computed header refusals read nothing and are not recovered by `try`; `header: null` on YAML | `load_data_uncontracted::computed_header_refusals_read_nothing_and_are_not_recovered` |
| Header option shapes (absent/null/booleans/list/wrong type/mixed/all-invalid/empty/empty name/duplicate) | `load_data_options::option_matrix_invalid_cells_read_nothing` (new `Refuser::Header` cells), `load_data_options_passive::binder::unbindable_option_field_is_a_binding_error_at_argument_2`, `catalog::tests::load_data_options_bind_through_the_shared_binder` |
| Cell coordinates through normalized and explicit keys (contract) | `load_data_uncontracted::contract_failures_locate_cells_by_their_keys` |
| Passive provisional types | `call_check::load_data_options_tests::an_uncontracted_call_has_a_provisional_type` |
| Worked example passes; each property in isolation fails with the right location | `load_data_destination::{the_worked_example_passes_for_valid_data, a_direct_load_failure_names_the_expression_and_the_data_position, a_derived_result_failure_names_only_the_expression, a_selected_ternary_arm_keeps_its_data_position}` |
| Paren forwards; `try`, `||`, index, callback derive | `load_data_destination::only_parentheses_and_a_selected_arm_forward_the_load` |
| Contract failure stays `dm.data.*` | `load_data_destination::a_contract_failure_stays_a_data_error` |
| Enum, numeric string conversion, default, closed object, imported type, trigger | `load_data_destination::{an_enum_destination_judges_each_loaded_value, conversion_and_defaults_agree_with_the_destination, an_unexpected_member_fails_at_the_member, an_imported_destination_type_is_applied, a_trigger_destination_declaration_is_applied}` |
| Normalized/explicit keys and wrapped scalar in destination records | `load_data_destination::{tabular_keys_locate_the_original_cell, a_wrapped_scalar_is_located_at_the_root}` |
| Status block shows both locations | `load_data_destination::the_status_block_shows_both_locations` |
| Pass-two property; caller-owned post-shell verdict | `load_data_destination::{a_property_completed_in_pass_two_keeps_its_provenance, a_caller_owned_post_shell_verdict_is_the_callers}` |
| CLI wiring (explicit-name width, destination failure JSON and Markdown report) | `cli l1 load_data::{an_explicit_name_list_of_the_wrong_width_is_located, a_destination_schema_failure_names_the_expression_and_the_data, a_destination_schema_failure_report_shows_both_positions}` |
| DMLS: nullable contract, header-list diagnostics and completion, provisional members | `dmls l1 option_arguments::{an_omitted_or_null_contract_is_no_diagnostic, options_after_a_null_contract_are_ranged_on_the_third_argument, header_list_problems_are_ranged_like_composition, a_header_list_value_offers_no_boolean_items, an_uncontracted_load_is_provisional_for_members, hover_on_load_data_shows_the_options_parameter_and_its_defaults}` |
| Catalog examples (one-argument YAML, explicit-name CSV with text port) | catalog example verification tests and `load_data_function_page_matches_catalog` |

Tree-loader document shapes (YAML duplicate key, second document, trailing
syntax, JSON/JSON5 last-wins duplicates, trailing content, TOML duplicates,
invalid UTF-8) were already covered by `load_data_execution.rs`'s tables;
they are unchanged by this phase and were run as controls.

Code-smell audit of the changed loader and binder lines (`serde(default)`,
`unwrap_or_default()`, `.ok()`, `filter_map`): the only hit is a
re-indented existing `.ok()?` in `load_data_type` (an unparsable literal
contract types as the catalog's return, as before).

### Findings while testing

- My first provisional type parsed `unknown[][]`, which the SimplifiedSchema
  grammar does not accept, behind an `expect`: a panic on any provable
  headerless uncontracted read. Now built from shapes with no panic path,
  pinned by `an_uncontracted_call_has_a_provisional_type`.
- `{header: [1]}` now binds (`["1"]`), so the tests that used it as an
  unbindable element use an object element and pin the conversion
  separately.
- Two more problems surfaced in the gates. The new `expression_origins` map pushed `MarkdownError` past clippy's
  `large-error-threshold`, so it is boxed. A test object key `"any"` tripped
  `trigger_spelling_guard` (a retired trigger spelling), so it was renamed.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  (excluding the unrelated `sniff/fixes/2026-11-20-commit-link-walks/plan.md`
  from another session) gave `packages: [claudine, darkmatter, darkmatter-cli,
  darkmatter-lsp, test-toolkit, claudine-cli, simplified-schema]`,
  `change_class: package`, so the gates ran.
- The plan's filter `just test -E 'test(load_data) | …'` cannot run through the recipe because of the
  parentheses (see Phase 1), so I ran it with nextest directly:
  darkmatter 681/681 (with `call_check` and `expression_recovery_data`),
  darkmatter-lsp `option_arguments` 19/19, darkmatter-cli `load_data::` 32/32.
- `LANG=C.UTF-8 just test` (darkmatter area): 12114 passed and 1 failed before fail-fast stopped the run, with none at or above 5 s. The failure is
  `shipped_schema_corpus::shipped_markdown_named_type_references_resolve` on
  `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
  (`apps: app[]`, committed in e21f32677, outside this feature; same failure
  as Phases 1 and 2). I then ran each package with nextest, no fail-fast, and the recipe's
  features: darkmatter 12666/12667 (one failure, the trigger-spelling guard
  hit described above, fixed and re-run green); darkmatter-cli 2642/2642;
  darkmatter-lsp 1996/1996; zed-dmls-cli 27/27. After the boxing change:
  darkmatter `load_data|schema|error_snapshots|diagnostic_source` 2698/2699
  (only the OSC-9 test fails). No slow tests.
- `just lint` (darkmatter): clean after the two fixes above.
- `just check-tier-coverage darkmatter`: 0 stranded.
- Dependent package, Claudine (its library has more than 200 tests, so the run was scoped to tests whose names include
  `schema|load_data|shipped|translate|expression|validation|compose`):
  1365/1365 passed. `cargo check -p claudine --tests` is clean.
- No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`,
  `BUILD_MACOS` unset); cross-OS evidence is left to CI.

## Phase 4

Wave 3 ran as two parallel subagents on disjoint files; their reports were
reviewed here and `just lint` (darkmatter) was rerun clean on the merged tree.

### Secure editor reads (Wave 3, task A)

- New public module `darkmatter::markdown::schemas::slots::observe`
  (`observe.rs`): `PropertySite`, `ObserveRequest`, `observes_property`,
  `SlotSession::observe_properties`, `PropertyObservations` (with `get`),
  `PropertyObservation`, `PropertyOutcome::{Observed, Pending, Missing,
  Failed}`, and `PendingReason` (defined in the new
  `compose/expression/editor_reads.rs`, re-exported from `observe`).
- Editor mode is a field on `ResolutionContext` (`editor`), set only by
  `observe_properties`; ordinary compose is unchanged. In editor mode the
  local `load_data`/`file_exists` resolution goes through
  `resolve_detailed_with_presence`: a lexical precheck
  (`ReadGate::admits_lexically`, `PermissionPolicy::may_allow_file_read_lexically`)
  and then the full non-prompting decision run before each candidate is
  probed; open buffers count as present. A refused candidate is the new
  nonrecoverable `ExpressionError::ObservationPending { function, reason }`
  (`try`/`||` never recover it); an admitted clean miss is the ordinary
  NotFound error (`try` recovers it; unrecovered it is `Missing`); I/O,
  missing context, or a vanished file is `Pending(ReadUnavailable)`.
- Remote reads in editor mode check the gate, then read
  `RemoteFetchRuntime::cache_only` through `cached_text_bounded` (never
  register, fetch, refresh, or write); a hit records its expiry and charges
  its bytes. The unbounded `lookup_fresh` was replaced by a bounded store
  read (`cache/store.rs`), and the Phase 1 `dead_code` expectations are gone.
- Admission (`admission::property_reads`): pure functions plus `load_data`
  and `file_exists` run; any other reader or effectful function is
  `Unsupported`; captured/volatile functions, `ctx`, and `current`/`local`
  outside a callback are `RuntimeOnly`. A bare name naming another
  whole-value site is evaluated first (memoized, `Cycle` on a loop); `fm.X`
  or a bare name whose authored value still holds `{{`/`$(` is `Dependency`.
- One `DataRuntime` (the session's) and one `IterationBudget` cover the
  pass; local bytes are charged to the 4 MiB allowance with a one-byte
  overflow check before parsing; cancellation is checked before each site
  and each read.
- Guard updates: `semantic_results_never_persist` allowlist counts
  (`remote_cache.rs` 9, `remote_fetch.rs` 8; the store is opened only in
  `remote_fetch.rs::cache_only_under`), and the editor reader check moved to
  `functions::is_editor_guarded_reader` to satisfy `type_scope_ownership_guard`.

### Validate observed values (Wave 3, task B)

- New `darkmatter/lib/src/markdown/schemas/observed.rs`:
  `EffectiveSchema::validate_observed_frontmatter(seed, replacements) ->
  ObservedValidation { properties: BTreeMap<String, ObservedVerdict> }`,
  `ObservedVerdict::{Judged(problems), Deferred(DeferReason)}`,
  `DeferReason::{Undeclared, UnresolvedDeclaration, RootUnionUnresolved,
  FileDependent}`, re-exported from `markdown::schemas`.
- The seed is copied with the replaced keys removed before the template
  scan, so a replacement's own `{{ }}` text is data; it then reuses
  `converted_candidate` and `validate_instance` on the full candidate.
- Simplified Schema gained a thread-local, nesting, panic-safe scope
  `validate::{with_unverified_file_checks, file_checks_unverified}`:
  `format.rs`'s eager `darkmatter-file` check judges syntax only,
  `file_match.rs`'s keyword admits every value, and
  `darkmatter_file_reference_outcome` does no I/O inside it. `coerce.rs`
  needed no edit (its validators run on the calling thread inside the
  scope). Validator cache keys are unchanged.

### Install editor results (after Wave 3, subagent C, reviewed here)

- Demand: `overlay/properties.rs` (`located_sites`, `sites_to_observe`)
  finds whole-value top-level sites through the shared scanner and
  `observes_property`; a document with an eligible site now gets a slot job
  even when its schema is not a template. `SlotSnapshots::demand_with`
  replaces the old demand path and keeps a separate slot-demand decision: a
  property-only job assembles over the installed pins (`SlotUse::Pins`) or
  with no template, never resolves slots, and carries the installed
  session, error, and slot reads forward. `resolve_slot_templates` now
  returns the `SchemaBundle` and takes `registered_hosts` and a `SlotUse`.
- Job: `session.observe_properties` then
  `bundle.effective.validate_observed_frontmatter`; the result carries
  observations, verdicts, observed reads, and the minimum remote expiry.
- Install/expiry: an input generation plus a log of changed paths drops a
  result dispatched before a change to a path it read
  (`InstallOutcome::Outdated`) and requeues it; every read path is
  re-stat'ed at install; a result already expired installs no observation;
  the remote deadline is the job's paired monotonic sample plus
  (expiry − paired wall time), served by a `crossbeam_channel::at` arm in the
  router's `select!` and also checked on access. A stale or refreshing entry
  withholds its `PropertyView`. `clock.rs` gained `Clock::pair`/`wall` and
  `ManualClock::starting_at`.
- Diagnostics: new `diagnostics/observed.rs::property_diagnostics`
  publishes destination verdicts at the authored expression with the
  existing schema codes, `relatedInformation` at the data file (UTF-16
  column from the captured source text), and `data.data_location`;
  `Missing` is a `dm.expression.evaluation_failure` warning with the
  file-reference details; `Failed` keeps its own code and is skipped when a
  passive diagnostic with that code already sits in the site's range;
  pending sites publish nothing, and placeholder findings at eligible sites
  are filtered. Library: `report_diagnostics::data_locations` (public, over
  `ExpressionProjector`).
- Completion/hover: `Shape::observed` (bounded depth and alternatives) and
  `DeclaredTypes::with_observed` feed completion and hover; hover appends a
  bounded "Observed value" preview (`overlay/properties.rs::preview`: 8
  entries, depth 3, 120 characters, omitted counts) after the declared type.
- Test seam: `LspFixture::start_with_slot_options(workspace, snapshot,
  gated, cache_root, clock)`, `StartupHooks.slot_cache_root`,
  `SlotSnapshots::{set_clock, set_cache_root}`.
- Host note: the build hit "Disk quota exceeded" (filesystem 100% full);
  the subagent deleted this worktree's rebuildable
  `target/debug/incremental` (55 GB), leaving the disk at 77%.

### Call facts and editor parity (subagent D, reviewed here)

- Per-call observed facts: `editor_reads.rs` collects what each evaluated
  `load_data` node returns (up to eight distinct values, then `overflow`);
  `observe.rs` exposes them as `PropertyObservation.calls: Vec<ObservedCall
  { span, call, values, overflow }>`; `DeclaredTypes::with_observed_calls`
  and `call_check.rs::expression_type` type a matching call from them, so a
  callback parameter completes with observed members while the view is
  current.
- Defects found and fixed in the install work: (1) the router waited on
  `crossbeam_channel::at(deadline)` with a deadline from the injected clock,
  so a `ManualClock` deadline never fired; it now waits
  `after(deadline − clock.now())`; (2) an uncontracted `header: false` read
  published cell verdicts and an observed shape, which the spec forbids; such
  properties and calls now get neither.
- Test seam: library feature `cache-test-hooks`
  (`compose::cache_test_hooks::seed_remote_entry`), enabled only by DMLS's
  dev-dependency.

### Requirement-to-test mapping

| Requirement | Tests |
|---|---|
| Worked example observed; one read and one parse | `editor_observation::{the_worked_example_is_observed_with_an_approval, properties_share_one_parse}`, `typed_load_data_editor::{the_worked_example_is_valid_in_the_editor, properties_loading_one_file_share_its_parse}` |
| Bad age on line 4: direct, parenthesized, conditional carry the data location; spread row expression-only | `typed_load_data_editor::{a_bad_age_fails_every_property_with_its_location, a_parenthesized_load_keeps_its_data_position}` |
| Editor/compose parity per property | `typed_load_data_editor::the_editor_and_compose_agree_on_each_property` |
| Refused read probes nothing; lexical and physical (symlink) exclusions; missing vs unreadable vs unauthorized | `editor_observation::{an_unapproved_read_is_pending_and_probes_nothing, lexical_and_physical_exclusions_refuse_before_reading, missing_unreadable_and_unauthorized_reads_differ}` |
| Approved missing path is `Missing` and watched; `try` recovers only a proven miss | `editor_observation::{an_approved_missing_file_is_missing_and_its_candidate_is_watched, recovery_turns_only_a_proven_miss_into_the_fallback}` |
| Guarded/recovered miss no warning; unrecovered warning cleared by creating the file (watch and stat fallback) | `typed_load_data_editor::{a_guarded_or_recovered_missing_input_is_no_warning, an_unrecovered_missing_load_warns_until_the_file_is_created, without_watchers_a_save_finds_the_created_file}` |
| Unsaved dependent buffer | `typed_load_data_editor::an_unsaved_data_buffer_decides_the_verdict` |
| Approval/config/schema invalidation; pending gives no verdict or invented member error | `typed_load_data_editor::{a_refused_read_is_pending_until_the_configuration_approves_it, a_once_grant_refreshes_a_pending_observation, a_schema_change_rejudges_the_observed_value}` |
| Superseded jobs, older generation, expired at install, stale withholds, read changed before install | `typed_load_data_editor::a_superseded_job_never_installs`, `slot_jobs::tests::{a_result_from_an_older_input_generation_is_discarded, a_result_already_expired_at_install_installs_no_observation, a_stale_view_withholds_its_observation, a_read_that_changed_before_install_marks_the_entry_stale}` |
| Property-only job resolves no slots; pins carried forward | `slot_jobs::tests::{a_property_only_job_observes_and_judges_without_resolving_slots, a_job_over_served_pins_keeps_the_installed_session}` |
| Remote: fresh entry observed with expiry, denied URL never consults cache, expired/missing pending, no fetch, expiry deadline | `observe_tests::{an_allowed_url_is_read_from_a_fresh_cache_entry, a_denied_expired_or_missing_entry_leaves_the_read_pending, a_cached_body_past_the_allowance_stops_the_pass}`, `typed_load_data_editor::{a_cached_remote_response_is_judged_until_it_expires, a_denied_url_is_pending_with_a_fresh_entry}`, `slot_jobs::tests::a_remote_expiry_becomes_a_monotonic_deadline_from_the_paired_clock`, `clock::a_manual_clock_advances_wall_and_monotonic_time_together` |
| Rejected effectful function, runtime-only binding, cycle; dependency-first properties | `editor_observation::{unsupported_operations_and_runtime_bindings_are_pending, properties_read_literals_and_other_properties_dependency_first}`, `typed_load_data_editor::unsupported_runtime_only_and_cyclic_properties_are_pending` |
| Budget and byte bounds, prompt-free | `editor_observation::{an_exhausted_allowance_stops_the_pass, a_source_past_the_byte_allowance_stops_the_pass}`, `typed_load_data_editor::an_exhausted_allowance_is_pending_without_a_prompt` |
| Bounded hover with declared type | `overlay::properties::the_preview_is_bounded_by_entries_depth_and_string_length`, `typed_load_data_editor::hover_shows_a_bounded_preview_after_the_declared_type` |
| Call facts and callback completion | `editor_observation::each_load_occurrence_keeps_what_it_returned`, `typed_load_data_editor::a_callback_parameter_completes_with_the_observed_members`, `scope::tests::an_observed_value_has_its_exact_bounded_shape` |
| Headerless uncontracted: `unknown[][]`, no editor cell verdict, compose fails | `typed_load_data_editor::a_headerless_uncontracted_read_has_no_editor_cell_verdict` |
| Eligibility by contract argument | `editor_observation::observes_property_follows_the_contract_argument`, `overlay::properties::only_a_top_level_uncontracted_load_is_eligible` |
| Observed validation: enum, numeric string, default, closed object, import, trigger, root union pass/fail/unresolved, file-dependent deferral without probe, template-like strings as data, undeclared deferred, inputs unchanged | `observed_validation::*` (13 tests), `simplified_schema validate::unverified_file_checks_judge_syntax_only_and_end_with_their_scope` |
| UTF-16 related range past multibyte text | `diagnostics::observed::a_character_position_is_found_past_multibyte_text` |

An editor destination diagnostic (worked example, Grace's age `"old"`):
`code: dm.schema.type_mismatch`, range at the `direct` expression,
`relatedInformation` at `data.yaml` line 4 (LSP line 3, character 7),
`data: { instance_path: "/direct/1/age", data_location: { file, pointer:
"/1/age", line: 4, column: 8 } }`.

The loader's input matrix is unchanged this phase (no new parsed field); the
editor read path reuses the same parsers, so the Phase 3 tables ran as
controls inside the package runs below.

### Gates

- Scope: `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  (excluding another session's `sniff/fixes/2026-11-20-commit-link-walks/plan.md`)
  gave `packages: [claudine, darkmatter, darkmatter-cli, darkmatter-lsp,
  simplified-schema, test-toolkit, claudine-cli, zed-dmls-cli]`,
  `change_class: package`; the gates ran. (Phase 3's changes are still
  uncommitted, so this covers both phases.)
- `LANG=C.UTF-8 just test` (darkmatter): 12148 passed, 1 failed before
  fail-fast stopped it, none at or above 5 s. The failure is the known
  `shipped_schema_corpus::shipped_markdown_named_type_references_resolve` on
  `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
  (outside this feature). Then each package with nextest, `--no-fail-fast`,
  and the recipe's features: darkmatter 12697/12698 (only that failure),
  darkmatter-cli 2642/2642, darkmatter-lsp 2027/2027, zed-dmls-cli 27/27. No
  slow tests.
- `just lint` (darkmatter): clean. `just check-tier-coverage darkmatter`
  (from the root): 0 stranded.
- Simplified Schema (changed directly): `just test` 2385 + 2017 passed, none
  slow; `just lint` clean.
- Dependent Claudine (more than 200 tests; scoped to
  `schema|load_data|shipped|translate|expression|validation|compose|fresh|slot`,
  which includes its shipped-artifact freshness over the edited Simplified
  Schema files): 1397/1397.
- No local OS test rigs (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`,
  `BUILD_MACOS` unset); cross-OS evidence is left to CI. The symlink case is
  `cfg(unix)`; the chmod-000 case skips itself when the process can still
  read the file (as root).

## Phase 5

Documentation, regeneration, and acceptance. No library behavior changed.
The only non-documentation edits were a catalog description, the
regenerated artifacts, a stale Claudine CLI snapshot, and a broken research
document that failed the Darkmatter corpus gate.

### Documentation (three writers on separate files, reviewed here)

- **Loading and composition:** `docs/topics/loading-data.md` is now the
  canonical guide. It covers the optional `as` and when conversion happens:
  in the call, or later when frontmatter is validated (a CSV `"1000"` stays
  text until then). It also covers wrapping a scalar root and turning `null`
  into `[]`, the three header modes, and the `First Name` → `firstName`
  migration in a contract (`p['First Name']` becomes `p.firstName`) and in
  member access. It includes `header: ['label', 'years']`, the spec's
  direct/additive/conditional example, and which forms keep
  `data_location`. Shorter updates went to `darkmatter-expressions.md`,
  `inline/interpolation.md`, the README (overview only),
  `cli/data-contracts.md` ("Destination schema failures"),
  `errors/composition-problems.md`, and the caller skill
  `use-skills/use-darkmatter/authoring.md`. The caller skill links to the
  guide instead of transcluding it, because the guide's prose has inline
  `{{ … }}` examples that would be evaluated at publication. The writer
  fixed drift it found: the empty-YAML advice (`| null` → it is now `[]`),
  `{header: [1]}` (it now binds as `["1"]`), and the duplicate-header rule
  (now the camelCase collision rule).
- **Editor:** `topics/schemas/schema-slots.md` "In the editor" has a new
  "Observed properties" subsection. It has the Mermaid flow (demand →
  admission/authorization → evaluation → validation → install →
  invalidation) and covers authorization, cached-remote setup, the
  provisional state, the missing-file warning versus the compose error,
  deferrals, and refresh triggers. Also updated: `topics/caching.md` ("Editor
  reads": read-only, fresh entries only, never fetches), `lsp/features.md`
  §4.12, and `dmls/docs/{autocomplete,hover,diagnostics}.md` (members of a
  loaded value, bounded preview, a `relatedInformation` + `data.data_location`
  JSON example, `unknown[][]` with no cell verdict). Drift fixed: the
  `load_data` signature in `hover.md` and `diagnostics.md` still showed `as`
  as required, and diagnostics.md said a `{{ }}` value is never flagged.
  Schema failures on an observed value use the source `darkmatter.schema`.
  Only missing-file and evaluation failures use `darkmatter.compose`.
- **Spread:** `simplified-schema/docs/topics/expression-syntax.md` "Object
  literals and spread" covers entries and spans, duplicate written keys,
  array spread, the literal-example grammar (spread refused), and
  Darkmatter's evaluation (in order, last key wins, shallow, `null` adds
  nothing, `dm.expression.invalid_spread_operand`, and inference with
  unknown keys).
- **Repo skills:** `.claude/skills/darkmatter/{expressions,requests,dmls}.md`
  and `.claude/skills/simplified-schema/SKILL.md` (the spread AST node; walkers
  must visit spread operands).
- Checked: the data-contracts JSON example matches real `md compose
  --output json` output for the worked example with `age: "old"`.
  It gives `dm.schema.type_mismatch` at `doc.md` 4:10, span 78..106, and
  `data_location` `{file: data.yaml, pointer: /1/age, line: 4, column: 8}`.

### Regeneration

- `just regen-expr-doc`: the first run changed no function page. Phase 3
  had already produced the optional signature
  `load_data(file: file | null, [as: type-definition(container) | null], [options: …])`.
  The page's description said `header: false` "needs a list-row `as`" and a
  name list "needs an object-row `as`". That is wrong without a contract.
  I reworded it in `schemas/partials/functions.yaml` ("an `as` must then have
  list rows" / "… object rows"). Then I ran `just shipped-schemas`, which
  regenerated `darkmatter/lib/generated/shipped.json` and
  `claudine/lib/generated/shipped.json`, and ran `just regen-expr-doc` again
  (`docs/functions/load-data.md` updated).
- `just publish-skills` (darkmatter): published `use-darkmatter` and
  `use-dmls`. I inspected `~/.claude/skills/use-darkmatter/authoring.md`. It
  has the optional-`as` examples, the explicit-name example, and the
  `First Name` → `firstName` note, with the fenced `{{ … }}` examples left
  verbatim.

### Fixes outside the feature, made to clear the gates

- `biscuit-terminal/docs/research/terminal-standards/osc/osc-9-conemu.md`
  declared `apps: app[]` with the `app` type in a sibling top-level `types:`
  block. `types:` is a `kind: schema` construct, so a Markdown document's
  inline `$schema` cannot see it. That failed
  `shipped_schema_corpus::shipped_markdown_named_type_references_resolve`,
  the known failure that Phases 3 and 4 left. I inlined the object type and
  dropped the three field descriptions (see decisions_for_review).
- `claudine/cli/tests/l1/snapshots/l1__context_command__context_expressions_function_order_snapshot.snap`
  still showed `load_data(file, as, [options])`. Phase 3 made `as` optional,
  so I updated the line to `load_data(file, [as], [options])`. Earlier phases
  never ran Claudine's CLI tests.

### Requirement-to-test mapping

This phase has no new behavior, so it adds no new tests. What it ran:

- Optional signature in the shipped catalog and generated docs: Darkmatter's
  `shipped_artifact_is_fresh` and expression-doc freshness tests (inside
  `just test`), and Claudine's `context_expressions_function_order_snapshot`.
- Shipped Markdown corpus with the passive named-type check:
  `shipped_schema_corpus` and `shipped_corpus_invocation` with
  `BISCUIT_L1_INCLUDE_SLOW=1`.
- Shipped-artifact consumers: `test-toolkit`'s
  `shipped_schemas_coordinator_contracts` (5 tests; these are the
  test-toolkit tests that read the regenerated artifacts).
- Loader, CLI, and editor entry points (Phases 3–4): `load_data*`,
  `typed_load_data_editor`, `schema_slots_editor`, `editor_observation`, and
  `observed_validation`, all inside `just test`.

### Gates

- Scope:
  `git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`
  gave `packages: [claudine, darkmatter, claudine-cli, darkmatter-cli,
  darkmatter-lsp, simplified-schema, test-toolkit]` and `change_class:
  package`, so the gates ran.
- Darkmatter: `LANG=C.UTF-8 just test`: 17394 passed, 10 skipped, 0 at or
  above 5 s. The corpus failure is gone. `just lint`: clean, including the
  zed-dmls wasm check. `just check-tier-coverage darkmatter`: 0 stranded.
- Darkmatter slow corpus:
  `LANG=C.UTF-8 BISCUIT_L1_INCLUDE_SLOW=1 cargo nextest run -p darkmatter --features effects-instrumentation,work-counters -p darkmatter-cli --features darkmatter-cli/test-fixtures -E 'test(shipped_schema_corpus) | test(shipped_corpus_invocation) | test(shipped_artifact_is_fresh) | test(load_data) | test(typed_load_data_editor) | test(schema_slots_editor)'`:
  549/549. The recipe's `-E '(...)'` form still breaks on parentheses, so
  this ran nextest directly.
- Simplified Schema: `just test`: 2017 (plus the 2385-test unit binary)
  passed, none slow. `BISCUIT_L1_INCLUDE_SLOW=1 just test shipped_corpus`:
  passed (the `-E` form breaks the same way). `just lint`: clean.
- Claudine: `just test-library` under the host's `LANG=C` failed 4
  `group_framing` tests, which use ASCII fallback glyphs under `LANG=C`.
  They are the same locale issue Phase 1 recorded for Darkmatter. With
  `LANG=C.UTF-8`, they passed alone (12/12) and the full run passed
  6937/6937, none slow. `just lint`: clean.
- claudine-cli (dependent): `LANG=C.UTF-8 just test-cli`: 5326/5326, none
  slow, after the snapshot update. A first plain `cargo nextest` run failed
  15 tests in about 0.01 s each, because it skipped the recipe's `md`
  provisioning (`_ensure-md`). That was a harness problem, not a code
  problem.
- test-toolkit (dependent, reads the regenerated artifacts):
  `shipped_schemas_coordinator_contracts` 5/5. Two other tests fail and read
  no file changed here:
  `ci_workflow_contracts::the_lint_step_measures_a_sub_second_command_instead_of_recording_zero`
  fails the same way alone (`duration_s=0.0`, from `time.monotonic`
  readings in separate interpreters), and `real_kache_config_selector_bypasses_the_managed_pin`
  is a `real_`-tier test that L1 excludes. Both are outside this feature.
- `git diff --check`: clean. `just slow-tests fresh`: fresh. After the
  Darkmatter run, only the Claudine snapshot file changed, and no Darkmatter
  test reads it.
- No local OS test rigs: `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, and
  `BUILD_MACOS` are unset, so cross-OS evidence is left to CI.
