---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-schema-narrowing-and-extraction/darkmatter/features/2026-10-28-extract-simplified-schema/spec.md"
plan: "darkmatter/features/2026-10-28-extract-simplified-schema/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
decisions_for_review:
    - |-
        Glob pattern parsing is part of biscuit-file's new minimal `file-reference-syntax` feature, so the minimal build keeps the `globset` crate. The plan said the minimal build must exclude gix, walkdir, and globset.
        Why: the schema grammar checks `match(...)` glob patterns while it parses (via GlobReference::new, which compiles a globset matcher). Dropping globset from the minimal build would mean skipping that check there, so some schemas that are rejected today would be accepted, and the spec forbids changing which schemas parse.
        Alternative: leave globset out and report the glob check as an "unavailable required check" in the minimal build. That is a slightly smaller dependency tree, but the minimal build would then behave differently from the default build.
        If the reviewer prefers the alternative: move `glob/parse.rs`, `glob/error.rs`, and `GlobReference::new` back under `file-reference`, and have Phase 4 make the grammar's `match(...)` check conditional on the `file` feature. The Phase 2 and Phase 8 dependency-tree checks should then also list globset.
    - |-
        The new `biscuit_file::frontmatter` module needs the `yaml` feature. The plan called it "unfeatured".
        Why: recognizing a near-miss `----` fence means checking whether the enclosed text is a YAML mapping, and that needs the YAML parser, which is optional in biscuit-file. `yaml` is on by default, and simplified-schema needs it anyway.
        Alternative: make the YAML parser a mandatory dependency of biscuit-file. That makes every no-default-features user pay for YAML.
        If reversed: drop the `#[cfg(feature = "yaml")]` on `pub mod frontmatter` and make serde_yaml_ng non-optional.
    - |-
        In the biscuit-file feature split, only the path-identity and strategy types of the "portable path" family are in `file-reference-syntax`. `PortablePath`, `PortableReference`, their diagnostics, and `PORTABLE_ENV_VARIABLES` stay under the full `file-reference` feature. The plan listed "portable path types" under the syntax feature.
        Why: PortablePath checks each spelling by running the resolver, which needs the resolution stack.
        Alternative: move the resolver's lexical parts into the syntax feature as well. That is a much larger refactor, with no current consumer that needs it.
        If reversed: the resolution code that PortablePath calls would have to be split out of `resolve.rs`.
    - |-
        Biscuit-file's `FileReferenceError::Git` variant exists only when `file-reference` is enabled, because its payload is a gix type. The error enum is not `#[non_exhaustive]`, so its set of variants depends on the enabled features.
        Alternative: change the payload to a boxed `dyn Error` so the variant always exists. That changes the variant's public type for existing users.
        If reversed: only error.rs and the one match on that variant change.
    - |-
        Plan Task 1.1 placed the inventory under `darkmatter/features/2026-10-25-extract-simplified-schema/`. That directory does not exist, so the inventory is at `darkmatter/features/2026-10-28-extract-simplified-schema/inventory.md`, beside the spec.
    - |-
        The inventory recommends, without implementing yet, two homes that the spec may not have anticipated, for review before Phase 4.
        (a) SchemaError's terminal presentation (its `BlockError` implementation): Rust does not let Darkmatter implement a trait from another crate for a type from another crate, so once SchemaError moves this code must move with it, into an opt-in, non-default `terminal` feature of simplified-schema that only Darkmatter enables. The alternative is a wrapper type in Darkmatter.
        (b) The pure `{{!data:v1:...}}` pending-token codec, plus the plain `{{ }}` scanner it relies on, moves into simplified-schema in Phase 2.4, because the format checks decode pending tokens during validation.
    - |-
        The pending-token codec's helper scanner (the plain `{{ }}` scan) moved as free functions `scan(content, excluded_ranges)`, `scan_plain`, and `find_all_plain` in a new `simplified_schema::expression::scan` module. The plan and inventory said only "move the scan core". Darkmatter keeps its `ExpressionFinder` type as a thin wrapper that finds Markdown code blocks and passes them to `scan`, so every existing caller and the 2,400-line lexer test module are unchanged.
        Alternative: move `ExpressionFinder` itself with a constructor that takes the code regions. That would add a second constructor and make Darkmatter's `new` (which needs the Markdown scanner) impossible on the moved type, so it would still need a wrapper.
        If reversed: only `scan.rs` and `ExpressionFinder`'s three delegating methods change.
    - |-
        `newline_offset_table` and `line_at_offset`, two crate-private helpers in the old `span.rs`, stayed in Darkmatter, which keeps a small `span.rs` that re-exports the library's span items alongside them. The plan said to move only the span helpers the moved code uses, and only Darkmatter's table-of-contents and remote-content code (which stay) use these two.
        If a later phase needs them in the library: move both and make them `pub`.
    - |-
        The `sniff` dependency is declared now, optional and behind the default `file` feature, although nothing calls it until the path-based factories arrive. The plan's Task 2.1 asked for this.
        Alternative: add it in the phase that writes the capture module. That keeps the dependency list strictly to what is used, but it means CI registration and the feature table change twice.
        If reversed: delete the `sniff` line and the `dep:sniff` entry from `file`.
    - |-
        Re-exports left in Darkmatter: `markdown::numeric` is marked `// migration: removed in Phase 7`, because it exists only for unmoved callers. `markdown::literal_token` and `markdown::span` are not marked, because the inventory keeps the literal-token path (Claudine uses it widely) and the span items are part of Darkmatter's public root API (`darkmatter::Spanned`, pinned by the `span_compat` test). `pending::first_token` is `pub` in the library (a Darkmatter caller needs it) but re-exported only `pub(crate)` in Darkmatter, so Darkmatter's public API does not grow.
        If the reviewer wants every re-export marked for removal, Phase 7 must also repoint Darkmatter's internal and Claudine callers of `literal_token`.
    - |-
        Two tests inside the moved literal-token module loaded YAML through Darkmatter's Markdown frontmatter parser, which the library cannot depend on. In the library they now load through the same admitting YAML loader and JSON conversion that the frontmatter parser uses. Darkmatter keeps two copies that still go through the real frontmatter parser, so both facts stay pinned.
        Alternative: move those two tests to Darkmatter only. That loses the library-level round-trip check.
    - |-
        The library counts its own work. Moving the parser meant its `ExpressionParse` work counter could no longer call Darkmatter's counters, so simplified-schema gained a test-only `work-counters` feature with its own `work_counters` module (just `ExpressionParse` for now). Darkmatter's `work-counters` feature turns it on, and Darkmatter's `snapshot()` adds the library's count into its existing `Work::ExpressionParse`, so every test and bench that reads that counter is unchanged. This follows the inventory's plan for work counters; the plan itself did not mention them for this phase.
        Alternative: have Darkmatter pass a counting callback into the parser. That puts an instrumentation parameter on the public parse functions.
        If reversed: delete `simplified-schema/lib/src/work_counters.rs` and the macro in its `lib.rs`, and the fold in `darkmatter/lib/src/work_counters.rs`.
    - |-
        Two parser tests that moved with the parser read Claudine test fixtures (`claudine/cli/tests/fixtures/nested_span_regression/*.md`) with `include_str!`, so a simplified-schema test now reads files from another package. Only the relative path changed, as the move rules require.
        Alternative: copy the two fixtures into `simplified-schema/lib/tests/fixtures/`. That removes the cross-package read, but the copies can drift from the incident files they pin.
        If reversed: copy the files and change the two `include_str!` paths in `simplified-schema/lib/src/expression/parser.rs`.
    - |-
        The plan asked for a library test per schema check (expression values, expression defaults, function examples) proving `unknown_fn(1)` is accepted. Those checks are still in Darkmatter until Phase 4 moves the schema code, so the library tests cover the parser function each check calls, and new Darkmatter tests (`schema_expression_syntax_checks.rs`) cover the three checks themselves through the public schema API. The three checks now import the parser from `simplified_schema::expression` directly, so Phase 4 only has to change those imports to `crate::expression`.
        If the reviewer wants the check-level tests in the library: move them when Phase 4 moves `format.rs`, `convert.rs`, and `grammar.rs`.
    - |-
        `PRECEDENCE_TABLE` (the parser's operator-precedence list) was crate-private. It is now public but hidden from the docs, because a Darkmatter test checks that Darkmatter's operator catalog matches it, and that test now has to reach it from another crate.
        Alternative: move that comparison test into the library. The test reads Darkmatter's catalog, so it cannot move.
    - |-
        The plan's decision 5 expected `error.rs` to be split into a syntax part and a host part. As the inventory found, it has no syntax part: the syntax errors are `LexerError` and `ParseError`, which moved with the lexer and parser. `error.rs` stays whole in Darkmatter, and `ExpressionError` converts the library's `ParseError` as before.
    - |-
        Phase 4 placed the moved modules flat at the library root, mirroring Darkmatter's `schemas/` folder (`simplified_schema::{simplified, resolve, validate, format, coerce, coercion, ...}`), with root re-exports equal to what `darkmatter::markdown::schemas` re-exported. The plan named no module layout; the inventory proposed curated paths such as `simplified_schema::{types, parse, json_schema, query, validation}`.
        Why: a one-to-one layout made every internal path rewrite mechanical and kept the move reviewable as a move. Curating the public paths is the facade's job (Phase 6) and the consumer migration's (Phase 7).
        Alternative: reorganize into the proposed paths now. That mixes a rename of every module with the move and makes the diff much harder to check.
        If reversed: Phase 6 or 7 renames modules in the library and updates Darkmatter's `pub use` lines; no behavior depends on the layout.
    - |-
        Darkmatter already had a public `schemas::SchemaRoot` (one of its five schema *search* roots), which collided with the plan's new name for the `SimplifiedSchema` enum. The search-root struct is now `SchemaSearchRoot` (3 files); `SchemaRoots`, `SchemaRootKind`, and `SchemaRootState` kept their names. The plan did not foresee the collision.
        Alternative: keep the search root's name and expose the AST enum in Darkmatter only as `schemas::simplified::SchemaRoot`. That avoids touching Darkmatter's roots API but leaves two public types with the same name in one module family, and every import of the enum would need a longer path.
        If reversed: rename `SchemaSearchRoot` back and change the `pub use simplified::{… SchemaRoot …}` line in `schemas/mod.rs` to a module path.
    - |-
        The resolver now takes Darkmatter's shipped documents and its `darkmatter:` namespace as an explicit `ShippedSources` argument on every public resolve function, as plan task 4.4 asked. Darkmatter keeps its old signatures through same-named wrapper functions in `schemas/resolve.rs` that pass `SHIPPED_SOURCES`, so no Darkmatter, DMLS, CLI, or Claudine caller changed.
        Alternative: a process-wide table the host installs once. Fewer signature changes, but it is hidden global state: the library's behavior would depend on whether Darkmatter happened to be loaded in the process, and tests could not exercise both cases.
        If reversed: replace the parameter with a registration function and delete the wrappers.
    - |-
        `SchemaOrigin::{trigger, for_trigger, trigger_source}` read Darkmatter's `TriggerSource`, and Rust does not let Darkmatter add methods to a library type. They are now a Darkmatter extension trait, `schemas::TriggerOrigin`; call sites keep their spelling but must import the trait (five files did). The inventory suggested plain functions.
        Alternative: free functions (`trigger_source(&origin)`). No import needed for the trait, but every call site would change shape.
        If reversed: replace the trait with functions and rewrite the five callers.
    - |-
        `SchemaError`'s terminal rendering (its `biscuit-terminal` `BlockError` implementation) is in the library behind a new opt-in `terminal` feature, which Darkmatter enables. The inventory flagged this as needing a ruling because the spec puts presentation in rendering packages.
        Why: Rust forbids Darkmatter from implementing a third-party trait for a library type, and Darkmatter's error registry must hand out a borrowed `&dyn BlockError`.
        Alternative: a transparent wrapper type in Darkmatter plus an unsafe pointer cast (or a `ref-cast` dependency) to borrow it. That keeps the library free of terminal code at the price of unsafe code or a new crate.
        If reversed: move the `impl` block (errors.rs, under `#[cfg(feature = "terminal")]`) into a Darkmatter wrapper and drop the feature.
    - |-
        Moving the resolver and the file formats into the library meant its `--no-default-features` build (no file resolution) had to decide what a file check does. It now fails with a new typed error, `biscuit_file::FileReferenceError::ResolutionUnavailable { reference }` (classified as missing context), and file globs accept nothing. Claudine's deliberately exhaustive error-to-slug mapping (`harness/error.rs` and its mirror test) now maps the variant to `missing_context`, matching biscuit-file's own class. That follows the plan's rule that a check with no implementation must fail, never pass silently. The new error variant is a small change to biscuit-file's public enum; the one exhaustive match in the workspace (Claudine's failure-slug vocabulary and its mirror test) maps it to `missing_context`.
        Alternative: a library-only error type. That would need a new `SchemaError` variant and different error plumbing in every resolution path.
        If reversed: add the library error and map it at the four places that now build `ResolutionUnavailable`.
    - |-
        83 moved inline tests that judge real files on disk are marked `#[cfg(feature = "file")]` (two whole test modules are gated the same way), so they run in the default suite but not in the minimal build, where those checks are unavailable by design. A new L1 test pins the minimal build's typed failure.
        Alternative: run them in both builds and expect failures in the minimal one. That doubles the assertions for no new information.
    - |-
        The plan asked for a new golden test of the shipped catalogs' lowered output. The existing `recursive_schema_golden` corpus already pins every shipped catalog (the embedded baseline, every partial, Content Policy, and the catalog body) byte for byte and was generated before this phase, so it is the golden; no duplicate was added.
    - |-
        Only the parts of `schemas/mod.rs` that moved code needs were moved (validation problem types and codes, schema origins, the shared validator cache). The memo helpers `phase_json_schema`, `merged_trigger_schema`, `CompiledValidators`, and `wrapped_root_arms` stay in Darkmatter for now: only `EffectiveSchema` uses them and `merged_trigger_schema` takes Darkmatter's trigger layers. The inventory listed them as moving.
        If Phase 5/6 needs them for prepared state, they move then.
    - |-
        `DARKMATTER_SCHEMA_CACHE_SIZE` (the validator cache size) is now read by the library's `validate.rs`. Darkmatter's context-construction guard only scans Darkmatter's own source, so its allowlist entry for that read was removed and the library has no equivalent guard yet. The trigger-spelling guard was extended to scan `simplified-schema`.
        Follow-up: Phase 8 (guards) should run the ambient-state guard over the library's source too.
    - |-
        Phase 5 put the strict/soft methods on a new library type, `Prepared` (in `simplified_schema::prepared`), built from an already lowered JSON Schema plus the directory and file-resolution context file values resolve in: `Prepared::new(json_schema, base_dir, context)`. The plan said Task 5.2 exposes the prepared state the Phase 6 `SimplifiedSchema` object owns, with reuse keyed on definition revision and context identity.
        Why: the facade does not exist yet, so the revision-keyed reuse is left to Phase 6, which will hold a `Prepared` and rebuild it when its definition changes. `Prepared` itself is built once and judges any number of values.
        Alternative: build the facade's lazy cache now. That would pull Phase 6's layering and revision design into this phase.
        If reversed: nothing here changes; the facade wraps `Prepared` either way.
    - |-
        `ValidationFailure` is a `#[non_exhaustive]` enum with two variants, `InvalidData { problems }` and `UnavailableCheck { problems }`, and each `ValidationProblem` gained `is_unavailable_check()`. When a failure has both a real violation and a check that could not run, it is `InvalidData` (the value is certainly invalid), and the unavailable problems are still marked one by one. The spec only said the two kinds must be told apart.
        Alternative: let "could not complete" win whenever any check was unavailable. A caller would then not learn that the value is certainly wrong.
        Schema compile errors are not a `ValidationFailure` variant: `Prepared::new` returns `SchemaError`, so validating can never hit one. If Phase 6 compiles lazily inside `validate_*`, it adds a variant (the enum is non-exhaustive, so that is not a breaking change).
    - |-
        A problem counts as an unavailable check when the crate is built without the `file` feature and the problem is a file value's resolution failure or a `match(...)` glob rejection. Without that feature both checks always fail that way, so the rule is exact, and with the feature it never applies. Malformed file-reference syntax is still a real violation in every build.
        Alternative: add a new `FileReferenceDiagnostic::Unavailable` variant carrying the exact cause. That is more explicit, but it changes a public enum that Claudine matches exhaustively and Darkmatter constructs in tests, for a case only the minimal build can reach.
        If reversed: map `FileReferenceError::ResolutionUnavailable` to the new variant in `validate.rs`'s `file_reference_diagnostic`, have `is_unavailable_check` test for it, and update Claudine's match.
    - |-
        `ValidationSuccess` reports the schema's advisories (non-fatal resolution findings, set with `Prepared::with_advisories`) as its "diagnostics". Successful validation produces no other non-fatal finding today, so nothing else had to go there.
        If a reviewer wants room for more: add a field; the struct's fields are public, so a new one is additive only for code that does not build the struct by hand.
    - |-
        Darkmatter's `CompiledValidators` (with a new `accepts` and `collect_problems`), `wrapped_root_arms` and its memo, and the problem-description pass moved into the library's `prepared` module. Darkmatter's `EffectiveSchema` now calls them, so strict/soft and Darkmatter's own validation run the same collection code. The memo is cleared by the library's `reset_process_caches`. `phase_json_schema` and `merged_trigger_schema` stay in Darkmatter: they implement phase and trigger policy, which the library does not need.
        Darkmatter's entry points (`validate`, `validate_raw`, the phase and stored variants) are unchanged and do not route through `validate_strict`/`validate_soft`, as the spec requires; they add templates, lifecycle-verb operands, and positions that only a host has.
    - |-
        The schema object lives in a new module, `simplified_schema::schema`, and is re-exported at the crate root with its helpers: `SimplifiedSchema`, `SimplifiedSchemaError` (one error type for building, layering, and export), `SchemaProvenance`, `TypeDefinition`, `FunctionView`, `PathSyntaxError`, and, with the `file` feature, `SchemaSource` and `repository_scope_catalog`. The plan named the views `TypeDefinition` and `FunctionType`; `FunctionType` is already the language's name for a parsed function signature, so the borrowed view is `FunctionView`.
        Alternative: separate error types per operation (loading, layering, export). More precise signatures, but three enums that share most variants.
        If reversed: split `SimplifiedSchemaError`; only `schema/error.rs` and the `Result` types of the facade's methods change.
    - |-
        The file factories take one argument, `impl Into<SchemaSource>`. `SchemaSource` converts from `&Path`, `PathBuf`, `&str`/`String` (parsed as a file reference at entry) and `FileReference`. A plain source is raw (the factory captures a new request context once, first thing); `.bound(context)` makes it context-bound (no capture); `.with_text(buffer)` supplies the file's text so it is not read (for an editor's unsaved buffer). A raw relative path is joined to the captured directory; a reference resolves through the context's normal candidate order. The plan left the input type's name and shape open.
        Alternative: two input types (a raw one and a bound one) or a trait the caller implements. Same behavior with more types to learn.
        If reversed: only `schema/source.rs` changes.
    - |-
        A schema built from text (`TryFrom<&str>`) that names another file (a `Name@./file` import, a whole-file `$schema` reference, a file arm of a root union, or `example(./file)`) is built successfully and keeps the reference as parsed, as the spec asks. It can be inspected and exported, but validation fails with a new `ValidationFailure::Unprepared { error }` variant carrying `SchemaError::Unresolved` with the cause `ResolutionUnavailable`. There is no method to attach a file context to such a schema afterwards; loading it through a file factory is the way to follow its references.
        Alternative: add a `bind(context)` method that resolves the kept references later. That is a second way to get a file-backed schema, with its own capture rules to audit.
        If reversed: add the method on `SimplifiedSchema`; it would rebuild the layer through the same `resolve_authored` path the factories use.
    - |-
        `ValidationFailure` gained a third variant, `Unprepared { error: Arc<SchemaError> }`, for a schema that cannot be prepared: a function-typed property (no JSON Schema form), a lowered schema `jsonschema` rejects, or text that names a file. The Phase 5 agent suggested exactly this. The error is shared, so every validation of such a schema returns the same one without retrying. `problems()` is empty for it.
        Alternative: make construction fail for any schema that does not lower. Function catalogs (`kind: schema` with only function types) could then never be loaded for inspection, which the spec requires.
    - |-
        A schema built from text has no file context, so its `file(eager)` and `match(...)` checks cannot run. Such a check now fails as `ValidationFailure::UnavailableCheck` (not `InvalidData`) in every build, the same way the minimal build reports them. Malformed file-reference syntax is still invalid data. `ValidationProblem::is_unavailable_check()` still answers only for the build (no `file` feature); the content case is decided when the failure is classified, from a crate-private flag on `Prepared` (`without_file_checks`).
        Alternative: report them as invalid data, which is what resolving against an empty context would produce. That would reject every valid path.
        If reversed: drop `Prepared::without_file_checks` and the flag on `ValidationFailure::from_problems`.
    - |-
        `TryFrom<&str>` and `try_from_yaml` also accept a raw JSON Schema written in YAML or JSON when its root `$schema` is a `json-schema.org` dialect URI; a mapping with no recognizable schema structure is `MissingSchema`. The spec asked for classification by `$schema` structure and for the raw JSON Schema surface to be preserved; it did not say whether content can be raw JSON Schema. `try_from_json_schema` uses the resolver's existing strict JSON loader unchanged, which accepts any JSON object.
        Alternative: accept raw JSON Schema only through `try_from_json_schema`. Simpler classification, but `TryFrom<&str>` would reject a valid JSON Schema document.
        If reversed: delete the dialect branch in `Layer::from_yaml_document`.
    - |-
        `SimplifiedSchema::select(name)` returns an object named type as a schema of its own (`kind: schema` with `types: { Person: … }` → a `Person` schema), because the spec's example says "a consumer can select `Person` and validate a mapping against it". Its export writes the selected type as `$schema` with the whole authored `types` table beside it, so references between named types still resolve. The plan did not list this method.
        Alternative: have callers query the type and build a schema from it themselves. Every caller would then have to repeat the lowering.
    - |-
        Layering keeps the typed layers as a list (highest precedence first) plus the merged lowered JSON Schema, rather than merging the typed declarations into one. A query answers from the highest layer that declares the top-level key, which is exactly the property-level replacement the JSON merge performs; a raw JSON Schema layer that declares the key hides typed layers below it. The file context that judges `file(...)` values and the reported provenance stay the receiver's, also under `layer_over`. A call whose merge leaves the receiver's JSON unchanged (every incoming property shadowed) changes nothing, so the revision and prepared validators are kept, as the spec asks for unchanged definitions. Darkmatter's `EffectiveSchema` keeps its typed layers the same way (document shape plus trigger shapes).
        Alternative: merge the typed declarations into one `SchemaShape`. That needs a typed merge of named-type tables, which the resolver does not have; inventing one is the new merge semantics the spec rules out.
        If reversed: `Definition` in `schema/mod.rs` and the query in `schema/view.rs` change.
    - |-
        The YAML export writes nested objects as block mappings and property-level unions as YAML lists, because the existing inline serializer drops pattern keys and `$constraints` of inline objects, and keeps only the last arm's description of a `a | b` union. Where the serializer would still have to drop something (an inline object with pattern keys inside a type string, such as `'{<string>: number}[]'`), the export returns `ExportUnsupported` instead. A layered schema exports when every layer is a SimplifiedSchema object schema and the properties copied from lower layers do not refer to named types (which resolve in the layer that declared them); otherwise `ExportUnsupported`. The spec forbids silently dropping metadata.
        Alternative: extend the inline serializer to write pattern keys and object constraints. That changes an existing serializer the extraction must preserve.
    - |-
        `coords[1]` on a tuple returns the type at that position (past the last element, the spread's type). The existing path walk the facade uses (`completion::def_at_path`, as the inventory recommended) treats any index as "an element of the list" and returns the list's own declaration, so the facade resolves the tuple position itself when the last step is an index. A key below a tuple position (`coords[1].x`) still returns `None`, as the walk does. A property that root-union arms declare is answered with the union of the arms' declarations, owned, even when only one arm declares it, as the existing walk does.
        Alternative: return the tuple's declaration with an "element" marker and leave the position to the caller. The spec names tuple indices as a query feature.
        If reversed: drop `tuple_position` in `schema/view.rs`.
    - |-
        The facade resolves with `ShippedSources::NONE`, so `Name@darkmatter:<stem>` is an ordinary file reference through it. The spec keeps that namespace host policy, and Darkmatter's own entry points still pass its shipped sources. If Phase 7 migrates a Darkmatter or DMLS path onto the facade, it needs a way to pass the host's `ShippedSources` (for example a `SchemaSource::with_shipped_sources`).
    - |-
        `repository_scope_catalog` (the projection from a `sniff` repository observation to biscuit-file's scope catalog) moved from Darkmatter into the library's `capture` module, and Darkmatter re-exports it, as the inventory's row F7 decided, so the library's raw-path capture and Darkmatter's request capture use one projection. The capture itself mirrors Darkmatter's binaries: the process directory, home and environment through biscuit-file, the repository root through `sniff::filesystem::git::repo_root`, the packages through `detect_repo_structure`. A discovery error is an error (`SimplifiedSchemaError::Discovery`), no repository is not, as in Darkmatter.
    - |-
        `SimplifiedSchema::resolve(uri)` is an `async fn` that does all its work before its first (and only) poll, so it needs no runtime dependency; it selects the loader by extension the way the resolver does for referenced files (`.md`/`.markdown` Markdown, `.json` raw JSON Schema, anything else YAML). The plan only said it supports local references and supplied sources and keeps HTTP(S) as `RemoteUnsupported`.
    - |-
        The capture and read counters are thread-local `cfg(test)` statics, so the context-boundary tests (criterion 15) are unit tests inside the crate (`src/schema/tests.rs`) rather than L1 integration tests, which cannot see `cfg(test)` code. The tests that move the process directory or home are `#[serial]` and restore what they changed.
        Alternative: a test-only Cargo feature exposing the counters to L1 tests. That is a public surface the plan said not to add.
    - |-
        Darkmatter's effective schema does not hold a `SimplifiedSchema` object. The plan said `DarkmatterSchemas`/`EffectiveSchema` would hold the library object and build the effective schema with `effective.layer_under([&baseline, ...triggers])`.
        What was done instead: Darkmatter keeps merging the already lowered baseline, trigger, and document layers itself, with the library's `merge_baseline`, the same merge `layer_under` runs, in the same low-to-high order with the document winning. A new test file (`darkmatter/lib/tests/l1/effective_schema_layering.rs`) builds the same layers as library objects and asserts the merged schema and the verdicts are equal, for a custom baseline, the shipped baseline, a matched trigger, and no document schema.
        Why: the Darkmatter merge is memoized by pointer identity of the shared baseline and trigger payloads, and the shipped baseline and a lone trigger are shared rather than copied. That keeps the validator cache warm and is what the spec's "no per-page reparsing or cloning" asks for. Building library objects per page would add a schema object per layer per page, and the library's validation uses the schema file's folder for file checks while Darkmatter's uses the document's folder, so validation would still have to go through Darkmatter's own path.
        Alternative: give the library a constructor from already resolved parts (typed schema, definitions, lowered JSON) and hold the object inside `EffectiveSchema`, keeping Darkmatter's memo and validators beside it. That exposes the typed queries on an effective schema but adds a second representation of the same data.
        If the reviewer prefers the alternative: add that constructor to `simplified-schema/lib/src/schema/layer.rs`, store the object in `EffectiveSchema` (`darkmatter/lib/src/markdown/schemas/mod.rs`, `effective_for_with_override`), and keep the parity tests as they are.
    - |-
        Phase 7 removed every Darkmatter re-export of the schema language, including the untagged ones. The plan's task 7.6 named only the re-exports marked `migration: removed in Phase 7`.
        The untagged lists at `darkmatter::markdown::schemas` (`SchemaRoot`, `SchemaError`, `PropertyDef`, `ValidatorCache`, `parse_yaml_schema`, `to_json_schema`, and about a hundred more) were the same kind of re-export, and the marked module re-exports could not be removed while they stayed, since they re-exported through those modules. Removing them gives every type one import path. As a result `darkmatter-cli`, `dmls`, `claudine`, `claudine-cli`, `claudine-gen`, and `messenger` (under its `research` feature) now depend on `simplified-schema` directly.
        Kept, as earlier phases decided: `markdown::span`, `markdown::literal_token`, and the expression items `compose::expression` names in its own API (`Expr`, `Lexer`, `Token`, `parse`, …). Darkmatter's `schemas::coerce` module is now crate-private because nothing public is left in it.
        Alternative: keep the untagged root lists, pointed at `simplified_schema` directly. Consumers would then need no new dependency, but each type would have two import paths again.
        If the reviewer prefers the alternative: re-add `pub use simplified_schema::{…}` lists to `darkmatter/lib/src/markdown/schemas/mod.rs`; the consumer import changes can stay as they are.
    - |-
        The library's content constructor (`SimplifiedSchema::try_from(&str)`) was left unchanged, though it cannot read JSON text with an object key longer than 1024 characters. It reads JSON through the YAML parser, which limits a key to that length. Darkmatter's shipped baseline has a longer `patternProperties` key, so its JSON text is rejected as invalid YAML, while the same schema written as YAML text loads.
        Why it was left: Darkmatter loads YAML schema files and frontmatter with the same loader, so changing only the library's constructor would make the two disagree on the same text. The spec asks for parity.
        Alternative: when the content starts with `{`, parse it with the JSON parser first. That accepts this text but changes how the constructor classifies content, including how duplicate keys are reported.
        If the reviewer prefers the alternative: change `Layer::from_content` in `simplified-schema/lib/src/schema/layer.rs` and add its JSON cases to the content tests; the Darkmatter parity test can then pass JSON text.
source_files_during_phase_1:
    - biscuit-file/lib/Cargo.toml
    - biscuit-file/justfile
    - biscuit-file/lib/src/lib.rs
    - biscuit-file/lib/src/frontmatter.rs
    - biscuit-file/lib/src/path_text.rs
    - biscuit-file/lib/src/yaml/types.rs
    - biscuit-file/lib/src/file_reference/mod.rs
    - biscuit-file/lib/src/file_reference/error.rs
    - biscuit-file/lib/src/file_reference/context.rs
    - biscuit-file/lib/src/file_reference/resolve.rs
    - biscuit-file/lib/src/file_reference/lexical.rs
    - biscuit-file/lib/src/file_reference/glob/mod.rs
    - biscuit-file/lib/src/file_reference/glob/parse.rs
    - biscuit-file/lib/src/file_reference/glob/error.rs
    - biscuit-file/lib/src/file_reference/portable/mod.rs
    - biscuit-file/lib/src/file_reference/portable/path_identity.rs
    - biscuit-file/lib/tests/l1/main.rs
    - biscuit-file/lib/tests/l1/reference_syntax_only.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
docs_updated_during_phase_1:
    - biscuit-file/README.md
    - biscuit-file/docs/dependencies.md
    - biscuit-file/docs/topics/file-references.md
docs_created_during_phase_1:
    - darkmatter/features/2026-10-28-extract-simplified-schema/inventory.md
skills_files_updated_during_phase_1:
    - .claude/skills/biscuit-file/SKILL.md
    - .claude/skills/biscuit-file/references/api.md
    - .claude/skills/biscuit-file/references/architecture.md
packages:
    - biscuit-file
    - claudine
    - claudine-cli
    - claudine-gen
    - darkmatter
    - darkmatter-cli
    - dmls
    - simplified-schema
    - messenger
source_files_during_phase_2:
    - simplified-schema/lib/Cargo.toml
    - simplified-schema/justfile
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/numeric.rs
    - simplified-schema/lib/src/numeric/ingress.rs
    - simplified-schema/lib/src/span.rs
    - simplified-schema/lib/src/pending.rs
    - simplified-schema/lib/src/expression/mod.rs
    - simplified-schema/lib/src/expression/scan.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/test_layout.rs
    - simplified-schema/lib/tests/l1/foundations.rs
    - Cargo.toml
    - Cargo.lock
    - justfile
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/numeric/ingress.rs
    - darkmatter/lib/src/markdown/span.rs
    - darkmatter/lib/src/markdown/literal_token.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/simplified_schema_reexports.rs
docs_updated_during_phase_2:
    - darkmatter/docs/dependencies.md
    - darkmatter/docs/inline/interpolation.md
    - docs/dependencies.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_2:
    - simplified-schema/README.md
    - simplified-schema/docs/dependencies.md
    - simplified-schema/docs/topics/features.md
    - simplified-schema/docs/topics/foundations.md
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
    - simplified-schema/lib/Cargo.toml
    - simplified-schema/justfile
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/work_counters.rs
    - simplified-schema/lib/src/expression/mod.rs
    - simplified-schema/lib/src/expression/lexer.rs
    - simplified-schema/lib/src/expression/parser.rs
    - simplified-schema/lib/src/expression/ast.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/expression_syntax.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/parser.rs
    - darkmatter/lib/src/markdown/compose/expression/ast.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
    - darkmatter/lib/src/markdown/schemas/simplified/grammar.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/simplified_schema_reexports.rs
    - darkmatter/lib/tests/l1/schema_expression_syntax_checks.rs
docs_updated_during_phase_3:
    - simplified-schema/README.md
    - simplified-schema/docs/topics/foundations.md
    - simplified-schema/docs/topics/features.md
    - darkmatter/docs/topics/schemas/parsing/index.md
    - darkmatter/docs/topics/schemas/parsing/scanning.md
    - darkmatter/docs/topics/schemas/parsing/lexing.md
    - darkmatter/docs/topics/schemas/parsing/grammar.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_3:
    - simplified-schema/docs/topics/expression-syntax.md
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
    - claudine/lib/src/harness/error.rs
    - claudine/lib/src/harness/error/tests.rs
    - claudine/lib/tests/l1/boundary_lint.rs
    - darkmatter/lib/tests/l1/context_construction_guard.rs
    - darkmatter/lib/tests/l1/trigger_spelling_guard.rs
    - Cargo.lock
    - biscuit-file/lib/src/file_reference/error.rs
    - claudine/cli/src/completion/schema_completion/keys.rs
    - claudine/gen/src/inputs.rs
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/src/composition/completion/tests.rs
    - claudine/lib/src/composition/inline_prompt.rs
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/system_prompt/prepare.rs
    - claudine/lib/src/system_prompt/prepare/tests.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/detect.rs
    - darkmatter/cli/src/commands/schema/triggers.rs
    - darkmatter/cli/tests/l1/entry_point_parity.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/tests/l1/collection_editor.rs
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/cleanup/diagnostic.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/repository_scope.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/resolve_ctx.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/errors/mod.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/clean.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/completion.rs
    - darkmatter/lib/src/markdown/schemas/detect.rs
    - darkmatter/lib/src/markdown/schemas/embedded.rs
    - darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/positions.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/roots.rs
    - darkmatter/lib/src/markdown/schemas/tests/baseline_grammar.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/tests/host_file_formats.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/mod.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/coercion_core.rs
    - darkmatter/lib/tests/l1/coercion_engine_parity.rs
    - darkmatter/lib/tests/l1/collection_analysis.rs
    - darkmatter/lib/tests/l1/content_policy_embedded_baseline.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/file_predicates.rs
    - darkmatter/lib/tests/l1/flow_sensitive_narrowing.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/recent_commits_shared_type.rs
    - darkmatter/lib/tests/l1/recursive_schema_completion.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - darkmatter/lib/tests/l1/recursive_schema_typed_consumers.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/tests/l1/schemas_convert_snapshots.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_union_completion.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs
    - darkmatter/lib/tests/l1/shared_acyclic_consumers.rs
    - darkmatter/lib/tests/l1/shell_template_execution.rs
    - darkmatter/lib/tests/l1/shell_template_guards.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - simplified-schema/justfile
    - simplified-schema/lib/Cargo.toml
    - simplified-schema/lib/src/advisory.rs
    - simplified-schema/lib/src/coerce.rs
    - simplified-schema/lib/src/coercion/engine.rs
    - simplified-schema/lib/src/coercion/error.rs
    - simplified-schema/lib/src/coercion/mod.rs
    - simplified-schema/lib/src/coercion/shape.rs
    - simplified-schema/lib/src/coercion/union.rs
    - simplified-schema/lib/src/completion.rs
    - simplified-schema/lib/src/discriminant.rs
    - simplified-schema/lib/src/embedded.rs
    - simplified-schema/lib/src/errors.rs
    - simplified-schema/lib/src/example.rs
    - simplified-schema/lib/src/file_context.rs
    - simplified-schema/lib/src/file_match.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/frontmatter_shape.rs
    - simplified-schema/lib/src/json_shape.rs
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/mapping_orders.rs
    - simplified-schema/lib/src/origin.rs
    - simplified-schema/lib/src/phase.rs
    - simplified-schema/lib/src/positions.rs
    - simplified-schema/lib/src/reference.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/resolve/graph.rs
    - simplified-schema/lib/src/resolve/recursive.rs
    - simplified-schema/lib/src/resolve/searched_tests.rs
    - simplified-schema/lib/src/scope.rs
    - simplified-schema/lib/src/simplified/alphanumeric.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/cursor.rs
    - simplified-schema/lib/src/simplified/definitions.rs
    - simplified-schema/lib/src/simplified/grammar.rs
    - simplified-schema/lib/src/simplified/lint.rs
    - simplified-schema/lib/src/simplified/mod.rs
    - simplified-schema/lib/src/simplified/narrowing.rs
    - simplified-schema/lib/src/simplified/query.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/signature.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/src/simplified/subtype.rs
    - simplified-schema/lib/src/simplified/types.rs
    - simplified-schema/lib/src/simplified/yaml_scalar.rs
    - simplified-schema/lib/src/timings.rs
    - simplified-schema/lib/src/validate.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/src/work_counters.rs
    - simplified-schema/lib/tests/l1/coercion_core.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/resolution_unavailable.rs
docs_updated_during_phase_4:
    - darkmatter/docs/topics/schemas/definition.md
    - docs/dependencies.md
    - docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md
    - simplified-schema/README.md
    - simplified-schema/docs/dependencies.md
    - simplified-schema/docs/topics/features.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_4:
    - simplified-schema/docs/topics/language-core.md
skills_files_updated_during_phase_4:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/biscuit-file/references/file-references.md
source_files_during_phase_5:
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/coerce.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/outcome.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/validation_contract.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/validation_contract_parity.rs
docs_updated_during_phase_5:
    - simplified-schema/README.md
    - simplified-schema/docs/topics/language-core.md
    - simplified-schema/docs/topics/features.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_5:
    - simplified-schema/docs/topics/validation.md
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
    - simplified-schema/lib/src/lib.rs
    - simplified-schema/lib/src/outcome.rs
    - simplified-schema/lib/src/prepared.rs
    - simplified-schema/lib/src/validation.rs
    - simplified-schema/lib/src/resolve.rs
    - simplified-schema/lib/src/simplified/standalone.rs
    - simplified-schema/lib/src/schema/mod.rs
    - simplified-schema/lib/src/schema/error.rs
    - simplified-schema/lib/src/schema/layer.rs
    - simplified-schema/lib/src/schema/layering.rs
    - simplified-schema/lib/src/schema/view.rs
    - simplified-schema/lib/src/schema/export.rs
    - simplified-schema/lib/src/schema/capture.rs
    - simplified-schema/lib/src/schema/source.rs
    - simplified-schema/lib/src/schema/tests.rs
    - simplified-schema/lib/examples/validate_person.rs
    - simplified-schema/lib/tests/l1/main.rs
    - simplified-schema/lib/tests/l1/schema_object.rs
    - simplified-schema/lib/tests/l1/schema_files.rs
    - darkmatter/lib/src/markdown/compose/context/repository_scope.rs
docs_updated_during_phase_6:
    - simplified-schema/README.md
    - simplified-schema/docs/topics/validation.md
    - simplified-schema/docs/topics/features.md
    - simplified-schema/docs/topics/language-core.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_6:
    - simplified-schema/docs/topics/schema-object.md
skills_files_updated_during_phase_6:
    - .claude/skills/darkmatter/SKILL.md
message_to_agent: |-
    Phase 7 is done. Darkmatter no longer re-exports the schema language: every `migration: removed in Phase 7` re-export and the untagged `darkmatter::markdown::schemas::{SchemaRoot, SchemaError, ...}` lists are gone, `markdown::numeric` is deleted, and `darkmatter-cli`, `dmls`, `claudine`, `claudine-cli`, `claudine-gen`, and `messenger` (feature `research`) depend on `simplified-schema` directly. Still re-exported on purpose: `markdown::span`, `markdown::literal_token`, `compose::repository_scope_catalog`, and the expression items `compose::expression` names (`Expr`, `Lexer`, `Token`, `parse`, ...). Darkmatter's `schemas::resolve` keeps only its shipped-source wrappers (`resolve_schema`, `expand_named_types_with_locals`, `prepare_embedded_*`, ...); import anything else from `simplified_schema::resolve`.
    Points for Phase 8:
    (1) `EffectiveSchema` does not hold a `SimplifiedSchema` (decision recorded); the effective merge is `merge_baseline` over lowered layers, and `darkmatter/lib/tests/l1/effective_schema_layering.rs` pins it equal to the object's `layer_under` (criterion 4 evidence, with `dmls/tests/l1/core_schema_types.rs::an_unsaved_document_schema_judges_the_buffer`).
    (2) The library's content constructor cannot read JSON text with a key over 1024 characters (YAML parser limit); Darkmatter's shipped baseline has one. A corpus check that feeds the shipped baseline as JSON content will fail for that reason; feed YAML text or a file through `try_from_json_schema` (decision recorded, not changed).
    (3) The library's doctests now all pass (`cargo test -p simplified-schema --doc`, 29); nextest does not run them, so a Phase 8 guard could.
    (4) Carried over from Phase 6, still open: the default build's graph holds `reqwest`/`tokio` through `sniff -> schematic-schema -> schematic-runtime` (criterion 14 needs a ruling or a narrower `sniff` dependency); the minimal graph holds `globset` (Phase 1 decision); the facade resolves with `ShippedSources::NONE` and no schema roots.
    (5) Pre-existing, not from this extraction: four `claudine-gen` `cli_switches` tests fail because `claudine/docs/research/model-config/_schema.yaml` declares `notes: string[]` (commit f0c34b7b0) while seven model-config research documents (codex, claude, gemini, goose, kilo, opencode, pi) still write `notes` as a string. Fix the data or the sidecar before treating the claudine area as green; Phase 7 left it alone as a contract decision.
source_files_during_phase_7:
    - Cargo.lock
    - claudine/cli/Cargo.toml
    - claudine/cli/src/commands/context/format.rs
    - claudine/cli/src/completion/schema_completion/candidates.rs
    - claudine/cli/src/completion/schema_completion/keys.rs
    - claudine/cli/src/completion/schema_completion/tests.rs
    - claudine/cli/tests/prompts/shipped_prompt_contract.rs
    - claudine/features/_completed/2026-10-28-schema-enhancements/spikes/single-engine/src/from_grammar.rs
    - claudine/features/_completed/2026-10-28-schema-enhancements/spikes/single-engine/src/from_json_schema.rs
    - claudine/features/_completed/2026-10-28-schema-enhancements/spikes/single-engine/src/mod.rs
    - claudine/features/_completed/2026-10-28-schema-enhancements/spikes/single-engine/src/numberlike_format.rs
    - claudine/features/_completed/2026-10-28-schema-enhancements/spikes/single-engine/src/tests.rs
    - claudine/features/_completed/2026-10-30-claudine-recursive-schemas/spike/deferred_operand_spike.rs
    - claudine/gen/Cargo.toml
    - claudine/gen/src/inputs.rs
    - claudine/gen/src/schema_compat.rs
    - claudine/lib/Cargo.toml
    - claudine/lib/src/composition/completion.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/frontmatter_excerpt.rs
    - claudine/lib/src/composition/inline_prompt.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/schema/bundle.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/schema/translate.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/system_prompt/prepare.rs
    - claudine/lib/tests/l1/claudine_schemas_embedded.rs
    - claudine/lib/tests/l1/claudine_schemas.rs
    - claudine/lib/tests/l1/container_schema_recursion.rs
    - claudine/lib/tests/l1/host_catalog.rs
    - darkmatter/cli/Cargo.toml
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/src/commands/schema/assignment.rs
    - darkmatter/cli/src/commands/schema/detect.rs
    - darkmatter/cli/src/commands/schema/validate.rs
    - darkmatter/cli/tests/l1/entry_point_parity.rs
    - darkmatter/cli/tests/l1/schema_validate_phase.rs
    - darkmatter/dmls/Cargo.toml
    - darkmatter/dmls/src/diagnostics/codes.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/position_parity_tests.rs
    - darkmatter/dmls/src/diagnostics/named_types.rs
    - darkmatter/dmls/src/overlay/analysis.rs
    - darkmatter/dmls/src/overlay/expressions.rs
    - darkmatter/dmls/src/overlay/mod.rs
    - darkmatter/dmls/src/overlay/named_types.rs
    - darkmatter/dmls/src/overlay/registry.rs
    - darkmatter/dmls/src/overlay/schema.rs
    - darkmatter/dmls/src/overlay/suggestions.rs
    - darkmatter/dmls/src/providers/code_actions.rs
    - darkmatter/dmls/src/providers/dsl.rs
    - darkmatter/dmls/src/providers/frontmatter.rs
    - darkmatter/dmls/src/providers/frontmatter/named_references.rs
    - darkmatter/dmls/src/providers/frontmatter/sequence_tests.rs
    - darkmatter/dmls/src/router.rs
    - darkmatter/dmls/src/validation_timing.rs
    - darkmatter/dmls/src/work_counters.rs
    - darkmatter/dmls/tests/l1/collection_editor.rs
    - darkmatter/dmls/tests/l1/content_policy_baseline.rs
    - darkmatter/dmls/tests/l1/core_schema_types.rs
    - darkmatter/dmls/tests/l1/expression_call_diagnostics.rs
    - darkmatter/dmls/tests/l1/expression_scopes.rs
    - darkmatter/lib/benches/effective_schema_ownership.rs
    - darkmatter/lib/benches/pipeline_work_support/mod.rs
    - darkmatter/lib/benches/schema_validation.rs
    - darkmatter/lib/src/lifecycle/operands.rs
    - darkmatter/lib/src/lifecycle/schema_projection.rs
    - darkmatter/lib/src/markdown/cleanup/frontmatter.rs
    - darkmatter/lib/src/markdown/cleanup/rules.rs
    - darkmatter/lib/src/markdown/compose/cache/hashing.rs
    - darkmatter/lib/src/markdown/compose/collection_blocks/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/collection_blocks/parser.rs
    - darkmatter/lib/src/markdown/compose/context/catalog.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/expression/absence.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/bind.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/error.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/registration.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/signature.rs
    - darkmatter/lib/src/markdown/compose/expression/call_binding/translate.rs
    - darkmatter/lib/src/markdown/compose/expression/call_check.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/catalog/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/deferred.rs
    - darkmatter/lib/src/markdown/compose/expression/diagnostics.rs
    - darkmatter/lib/src/markdown/compose/expression/fm_namespace.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/collections.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
    - darkmatter/lib/src/markdown/compose/expression/functions/strings.rs
    - darkmatter/lib/src/markdown/compose/expression/host/artifact.rs
    - darkmatter/lib/src/markdown/compose/expression/host/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/prepared.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/iterable.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/lexical.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/projection.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/raw.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/shape.rs
    - darkmatter/lib/src/markdown/compose/expression/scope/tests.rs
    - darkmatter/lib/src/markdown/compose/expression/semantics.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
    - darkmatter/lib/src/markdown/compose/policy.rs
    - darkmatter/lib/src/markdown/compose/report_diagnostics.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/canonical.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/matching.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/tests.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/template/typing.rs
    - darkmatter/lib/src/markdown/compose/tests/invalid_triggers.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/errors/mod.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/numeric.rs
    - darkmatter/lib/src/markdown/schemas/about.rs
    - darkmatter/lib/src/markdown/schemas/bound_operands.rs
    - darkmatter/lib/src/markdown/schemas/catalog.rs
    - darkmatter/lib/src/markdown/schemas/coerce.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/resolve.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/roots.rs
    - darkmatter/lib/src/markdown/schemas/tests/clean_quoting.rs
    - darkmatter/lib/src/markdown/schemas/tests/embedded_sources.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
    - darkmatter/lib/src/markdown/schemas/triggers/discovery.rs
    - darkmatter/lib/src/markdown/schemas/triggers/envelope.rs
    - darkmatter/lib/src/markdown/schemas/triggers/finding.rs
    - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
    - darkmatter/lib/src/markdown/schemas/triggers/lint.rs
    - darkmatter/lib/src/markdown/schemas/triggers/matcher.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/src/style/parse.rs
    - darkmatter/lib/src/work_counters.rs
    - darkmatter/lib/tests/l1/base_schema_end_to_end.rs
    - darkmatter/lib/tests/l1/coercion_engine_parity.rs
    - darkmatter/lib/tests/l1/coercion_translator_conformance.rs
    - darkmatter/lib/tests/l1/collection_analysis.rs
    - darkmatter/lib/tests/l1/compose_call_precheck.rs
    - darkmatter/lib/tests/l1/content_policy_disk_baseline.rs
    - darkmatter/lib/tests/l1/content_policy_editor_schema.rs
    - darkmatter/lib/tests/l1/content_policy_embedded_baseline.rs
    - darkmatter/lib/tests/l1/design_time_call_check.rs
    - darkmatter/lib/tests/l1/diagnostic_positions.rs
    - darkmatter/lib/tests/l1/directive_target_analysis.rs
    - darkmatter/lib/tests/l1/effective_schema_layering.rs
    - darkmatter/lib/tests/l1/error_snapshots/markdown_error.rs
    - darkmatter/lib/tests/l1/expression_type_scope.rs
    - darkmatter/lib/tests/l1/file_predicates.rs
    - darkmatter/lib/tests/l1/flow_sensitive_narrowing.rs
    - darkmatter/lib/tests/l1/frontmatter_coercion_engine.rs
    - darkmatter/lib/tests/l1/function_binding_conversions.rs
    - darkmatter/lib/tests/l1/function_call_binding.rs
    - darkmatter/lib/tests/l1/git_context_integration.rs
    - darkmatter/lib/tests/l1/glob_consumers.rs
    - darkmatter/lib/tests/l1/global_catalog.rs
    - darkmatter/lib/tests/l1/host_functions.rs
    - darkmatter/lib/tests/l1/lifecycle_generated_operands.rs
    - darkmatter/lib/tests/l1/lifecycle_operand_validation.rs
    - darkmatter/lib/tests/l1/lifecycle_schema_generation.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/match_keyword_input_matrix.rs
    - darkmatter/lib/tests/l1/meta_schema_phase1.rs
    - darkmatter/lib/tests/l1/meta_schema_phase3.rs
    - darkmatter/lib/tests/l1/meta_schema_phase4.rs
    - darkmatter/lib/tests/l1/meta_schema_phase5.rs
    - darkmatter/lib/tests/l1/meta_schema_phase6.rs
    - darkmatter/lib/tests/l1/meta_schema_reference_graph.rs
    - darkmatter/lib/tests/l1/meta_schema_repo_schemas.rs
    - darkmatter/lib/tests/l1/numeric_admission.rs
    - darkmatter/lib/tests/l1/raw_schema_conversion.rs
    - darkmatter/lib/tests/l1/recent_commits_shared_type.rs
    - darkmatter/lib/tests/l1/recursive_schema_completion.rs
    - darkmatter/lib/tests/l1/recursive_schema_conversion.rs
    - darkmatter/lib/tests/l1/recursive_schema_golden.rs
    - darkmatter/lib/tests/l1/recursive_schema_merge.rs
    - darkmatter/lib/tests/l1/recursive_schema_reporting.rs
    - darkmatter/lib/tests/l1/recursive_schema_typed_consumers.rs
    - darkmatter/lib/tests/l1/recursive_schema_types.rs
    - darkmatter/lib/tests/l1/report_diagnostics.rs
    - darkmatter/lib/tests/l1/required_context.rs
    - darkmatter/lib/tests/l1/schema_cache_validity.rs
    - darkmatter/lib/tests/l1/schema_embedded_trigger_bundles.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/envelope.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/mod.rs
    - darkmatter/lib/tests/l1/schema_enhancements_matrix/raw_schema.rs
    - darkmatter/lib/tests/l1/schema_expression_syntax_checks.rs
    - darkmatter/lib/tests/l1/schema_extraction_characterization.rs
    - darkmatter/lib/tests/l1/schema_grammar_descriptors.rs
    - darkmatter/lib/tests/l1/schema_json_key_order.rs
    - darkmatter/lib/tests/l1/schema_nested_template_deferral.rs
    - darkmatter/lib/tests/l1/schema_phase_validation.rs
    - darkmatter/lib/tests/l1/schema_roots.rs
    - darkmatter/lib/tests/l1/schema_validation_timings.rs
    - darkmatter/lib/tests/l1/schema_yaml_spellings.rs
    - darkmatter/lib/tests/l1/schemas_alphanumeric_check_cache.rs
    - darkmatter/lib/tests/l1/schemas_conjunctive_lowering.rs
    - darkmatter/lib/tests/l1/schemas_constraints_block_matrix.rs
    - darkmatter/lib/tests/l1/schemas_convert_snapshots.rs
    - darkmatter/lib/tests/l1/schemas_core_type_grammar.rs
    - darkmatter/lib/tests/l1/schemas_core_type_values.rs
    - darkmatter/lib/tests/l1/schemas_effective_presence.rs
    - darkmatter/lib/tests/l1/schemas_grammar_proptest.rs
    - darkmatter/lib/tests/l1/schemas_literal_expression.rs
    - darkmatter/lib/tests/l1/schemas_named_import_errors.rs
    - darkmatter/lib/tests/l1/schemas_named_type_memo.rs
    - darkmatter/lib/tests/l1/schemas_named_type_merge.rs
    - darkmatter/lib/tests/l1/schemas_required_count_matrix.rs
    - darkmatter/lib/tests/l1/schemas_runtime_parity.rs
    - darkmatter/lib/tests/l1/schemas_source_projection.rs
    - darkmatter/lib/tests/l1/schemas_union_coerce.rs
    - darkmatter/lib/tests/l1/schemas_union_completion.rs
    - darkmatter/lib/tests/l1/schemas_union_postfix.rs
    - darkmatter/lib/tests/l1/schemas_union_schema_matrix.rs
    - darkmatter/lib/tests/l1/schemas_union_typed_consumers.rs
    - darkmatter/lib/tests/l1/schemas_validation_summary.rs
    - darkmatter/lib/tests/l1/shared_acyclic_consumers.rs
    - darkmatter/lib/tests/l1/shared_acyclic_definitions.rs
    - darkmatter/lib/tests/l1/shared_acyclic_selection.rs
    - darkmatter/lib/tests/l1/shell_template_execution.rs
    - darkmatter/lib/tests/l1/shell_template_guards.rs
    - darkmatter/lib/tests/l1/shell_template_policy.rs
    - darkmatter/lib/tests/l1/shipped_schema_corpus.rs
    - darkmatter/lib/tests/l1/simplified_schema_reexports.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase1.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase2.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase3.rs
    - darkmatter/lib/tests/l1/suggest_constraint_phase4.rs
    - darkmatter/lib/tests/l1/trigger_discovery_recovery.rs
    - darkmatter/lib/tests/l1/trigger_envelope_claim.rs
    - darkmatter/lib/tests/l1/trigger_input_matrix.rs
    - darkmatter/lib/tests/l1/trigger_match_grammar.rs
    - darkmatter/lib/tests/l1/trigger_payload_load_errors.rs
    - darkmatter/lib/tests/l1/unknown_identifier_warning.rs
    - darkmatter/lib/tests/l1/variable_type_projection.rs
    - darkmatter/lib/tests/prompts/dasherized_identifier_corpus.rs
    - messenger/lib/Cargo.toml
    - messenger/lib/src/research/load.rs
    - simplified-schema/lib/src/coercion/mod.rs
    - simplified-schema/lib/src/format.rs
    - simplified-schema/lib/src/json_shape.rs
    - simplified-schema/lib/src/mapping_orders.rs
    - simplified-schema/lib/src/simplified/alphanumeric.rs
    - simplified-schema/lib/src/simplified/convert.rs
    - simplified-schema/lib/src/simplified/cursor.rs
    - simplified-schema/lib/src/simplified/serialize.rs
    - simplified-schema/lib/src/simplified/source.rs
    - simplified-schema/lib/src/simplified/subtype.rs
    - simplified-schema/lib/src/timings.rs
docs_updated_during_phase_7:
    - darkmatter/docs/dependencies.md
    - claudine/docs/dependencies.md
    - docs/dependencies.md
    - darkmatter/docs/topics/schemas/definition.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/plan.md
    - darkmatter/features/2026-10-28-extract-simplified-schema/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/schema.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/expressions.md
    - .claude/skills/darkmatter/frontmatter.md
---

# Implementation Log for 2026-10-28-extract-simplified-schema (9 phases)

## Phase 1

### 1.1 Boundary inventory

Written to `inventory.md` in this feature directory (the plan names a
`2026-10-25-…` directory that does not exist; the inventory sits beside the
spec). Headline findings that later phases must act on:

- **Pending codec must move (decision 4 → "required").** `format.rs` calls
  `literal_token::decode_leaf` (expression and shell formats) and
  `holds_pending_syntax` while JSON Schema format checks run, so Darkmatter
  cannot pre-classify. Phase 2.4 moves the pure `{{!data:v1:…}}` codec plus the
  plain `{{ }}` scan core of `ExpressionFinder` (`scan_plain`); the
  Markdown-aware finder and `is_deferred_template` stay.
- **`sniff` is needed by the `file` feature (decision 6).** Darkmatter captures
  repository roots through `sniff` (`GitRepo::discover`,
  `detect_repo_structure` in `context/request.rs`).
- **Owner-table corrections:** `file_match.rs`, `timings.rs`,
  `mapping_orders.rs`, `resolve_ctx::{document_file_context,
  resolve_document_file_ref}` move; `frontmatter_shape.rs`, `completion.rs`,
  `coerce.rs`, and `embedded.rs` split; `compose/expression/error.rs` stays
  whole (syntax errors are already `LexerError`/`ParseError`).
- **Nine "needs a fact" rows (F1–F9)** each have a decided home in the
  inventory; none introduces a callback trait.
- **Query surface:** no current helper parses a string path; the facade's
  string-path parser is new and maps onto `completion::def_at_path`
  (`PathStep::{Key, Index}`); empty path → `None`; dotted segment is always a
  literal key (no precedence rule exists to preserve).
- Consumers: strict only in Claudine's completion-phase validation; soft in
  Messenger `research/load.rs`, claudine-gen `inputs.rs`, claudine-cli
  `signals.rs`, DMLS diagnostics/overlay, `md schema validate`; everything
  else inspection-only.

### 1.2 Baseline

- host: macOS (Darwin 27.2.0), this worktree; toolchain `rustc 1.98.1
  (48a229cea 2026-09-01)`; profile `bench` (release), Criterion defaults from
  the bench files.
- commands, each run three times, sequentially:
  `cargo bench -p darkmatter --bench schema_validation` and
  `cargo bench -p darkmatter --bench effective_schema_ownership`.
- values are the median of the three runs' Criterion point estimates. Load
  average during the runs was 4–14 (other host activity plus the read-only
  inventory agent); treat differences under ~10% as noise.
- caveat: the darkmatter release compile for run 1 started within a minute of
  the 1.5 frontmatter edit (behavior-identical delegation to `biscuit-file`;
  not on any benchmarked path).

| Benchmark | Median |
| --- | --- |
| `effective_schema_ownership/darkmatter_baseline/baseline_only` | 5.4 µs |
| `effective_schema_ownership/darkmatter_baseline/baseline_plus_document` | 11.5 µs |
| `effective_schema_ownership/darkmatter_baseline/clone_effective` | 4.7 µs |
| `effective_schema_ownership/synthetic_512_properties/baseline_only` | 24.3 µs |
| `effective_schema_ownership/synthetic_512_properties/baseline_plus_document` | 30.9 µs |
| `effective_schema_ownership/synthetic_512_properties/clone_effective` | 11.8 µs |
| `effective_schema_ownership/control/document_only` | 10.6 µs |
| `effective_schema_initialization/darkmatter_baseline/owned_accessor` | 191.1 µs |
| `effective_schema_initialization/darkmatter_baseline/configure_default_baseline` | 334.1 µs |
| `effective_schema_initialization/darkmatter_baseline/configure_and_resolve_default_baseline` | 338.4 µs |
| `effective_schema_initialization/darkmatter_baseline/merge_empty_document` | 756.4 µs |
| `effective_schema_initialization/darkmatter_baseline/merge_tiny_document` | 750.4 µs |
| `schema_validation/corpus_1000/validate_1000_docs` | 18.14 ms |
| `schema_validation/corpus_1000/validate_1000_docs_warm_cache` | 18.40 ms |
| `schema_validation/claudine_cold/no_trigger_match` | 22.06 ms |
| `schema_validation/claudine_cold/document_events` | 22.68 ms |
| `schema_validation/claudine_cold/nested_sequence_group` | 24.28 ms |
| `schema_validation/claudine_warm/no_trigger_match` | 1.250 ms |
| `schema_validation/claudine_warm/document_events` | 1.361 ms |
| `schema_validation/claudine_warm/nested_sequence_group` | 1.338 ms |
| `resolve_frozen/unrolled_depth5` | 8.18 ms |
| `resolve_frozen/recursive` | 2.43 ms |
| `resolve_frozen_io/unrolled_depth5` | 25.3 µs |
| `resolve_frozen_io/recursive` | 23.0 µs |
| `resolve_frozen_compile/unrolled_depth5` | 5.18 ms |
| `resolve_frozen_compile/recursive` | 1.76 ms |

### 1.3 Characterization tests

`darkmatter/lib/tests/l1/schema_extraction_characterization.rs` (declared in
`tests/l1/main.rs`), 90 L1 tests, all green on unmodified Darkmatter code:
`yaml_loader_matrix` 46 (one `#[test]` per cell via `macro_rules!`),
`json_schema_loader_matrix` 21, `frontmatter_schema_field` 8,
`query_edge_cases` 6, `r6_table` 4, `function_schema` 4, `immutability` 1.

Entry points pinned: strict = `coercion::check_exact`/`accepts_exactly`
(value level) and `EffectiveSchema::validate_raw` /
`validate_for_phase(.., Completion)` (frontmatter level); soft =
`coercion::bind` (`Bound::Kept` = borrowed, `Bound::Converted` = owned) and
`EffectiveSchema::validate` / `validate_for_phase(.., Launch)`; YAML loader =
`parse_standalone_schema_document` (+ `parse_property_definition`,
`parse_yaml_schema`); raw JSON Schema loader = `resolve::resolve_schema` on a
`./schema.json` in a tempdir.

Pinned outcomes that look wrong (asserted as-is; parity authority, not fixed):

1. YAML `kind: schema` without `types` → `SchemaDocument` "found unsupported
   keys: " with an empty list (the "missing `types`" message is unreachable).
2. Leading BOM: YAML loader → "more than one document"; JSON loader →
   "invalid JSON: expected value".
3. Frontmatter `$schema:` and `$schema: null` → `FrontmatterShape` (not absent).
4. Unknown type word (`numbr`) parses as a local named reference, not a
   grammar error.
5. Null property type parses as `null`; `{}` parses as an empty inline object.
6. JSON loader: `$schema` null/`5`/`""`/`{}`, a draft-07 URI, `"kind":
   "schema"`, and a remote `$ref` all load; only a remote `$schema` reference
   is `RemoteUnsupported`.
7. Raw JSON load errors say "invalid standalone schema document".
8. A whole-file `$schema` reference with a sibling `types` map is accepted.
9. `serialize_property_def` drops spaces (`number(min(0);integer)`).
10. Two `suggest` constraints across one union are rejected by the grammar.

R6 row 3 (`number(integer)` + `"2.5"`): exact → `ConversionNotAllowed`; bind →
`ConstraintNotMet`; input unchanged.

### 1.4 biscuit-file feature split

- `file-reference-syntax = ["dep:globset", "url"]`;
  `file-reference = ["file-reference-syntax", "dep:gix", "dep:walkdir"]`.
- `cargo tree -p biscuit-file --no-default-features --features
  file-reference-syntax -e normal`: `globset` present, no `gix`, no `walkdir`.
- Syntax feature: `FileReference` (`new`/`raw`/`class`/`payload`/builders),
  kind/class/error types, `ResolutionFailure`, `ResolutionCandidate`,
  `RootProvenance`, `PathPosition`, `MagicPathTier`, `GlobReference`
  parsing, `GlobReferenceError`, `FileResolutionContext` and its data types,
  `PathIdentity`, `RelativeRoute`, `PortabilityPreference`, `IntentForms`,
  `home_dir`, `user_cache_dir`, `capture_env`.
- `file-reference` only: every resolving method, detailed resolution and
  completion types, `Resolved`, glob listing/matching
  (`matches_without_context` included), `PortablePath` and its diagnostics,
  `find_git_root`, `FileReferenceError::Git`.
- Two lexical path helpers moved from `resolve.rs` to a new
  `file_reference/lexical.rs`; `FileReferenceError::resolution_failure` moved
  to `error.rs`. Paths and type identities unchanged.
- Incidental fixes needed for `--all-targets` in reduced builds: the `parsers`
  bench declares `required-features = ["toml", "yaml", "json5"]`;
  `yaml/types.rs` `test_max_depth_exceeded_toml` gated on `toml`; `--features
  url` alone now compiles (`Resolved` export moved under `file-reference`).
- New L1 module `tests/l1/reference_syntax_only.rs` (6 tests, ungated);
  resolution modules in `tests/l1/main.rs` gated on `file-reference`.
  `just lint` adds a clippy pass for `--no-default-features --features
  file-reference-syntax,yaml`; `just test` (`test-minimal`) adds a nextest pass
  over that module in that configuration.
- Verified by the subagent: `just test` 1187/1187 + 9/9 no-default + 6/6
  syntax-only, doctests 28/28, `just lint` clean, Windows-gnu clippy clean in
  both configurations; `cargo check` clean for darkmatter, claudine,
  messenger, worktree, tree-hugger, biscuit-terminal, content-policy.

### 1.5 Passive frontmatter helper

- New `biscuit_file::frontmatter` (`extract_frontmatter_block`,
  `FrontmatterExtraction`, `FrontmatterEnvelopeError::FenceMismatch`,
  `line_spans`/`LineSpan`, `detect_near_miss_fence`), gated on `yaml` only
  (see decisions). 13 unit tests moved/added there: CRLF, lone CR, BOM, BOM
  does not weaken delimiters, `----` near-miss typed error, dash runs that are
  not frontmatter, missing closer, empty block, body-less, junk after the
  closer stays body, line spans for every terminator.
- `darkmatter::markdown::extract_frontmatter_block` keeps its signature and
  maps `FenceMismatch` to `MarkdownError::FrontmatterFenceMismatch`;
  `FrontmatterExtraction` is re-exported from `biscuit-file` (same path in
  Darkmatter, one type). `parse_frontmatter` uses the shared `line_spans` and
  `detect_near_miss_fence`. Darkmatter keeps the near-miss mapping test and the
  presence-agreement test.
- `cargo check -p biscuit-file --no-default-features --features yaml` clean.

### Phase gates

- `biscuit-file`: `just lint` clean (including the new syntax-only clippy
  pass); `just test` 1187/1187 L1 + 13 new `frontmatter::tests` within them,
  9/9 no-default `path_text`, 6/6 syntax-only `reference_syntax_only`.
- `darkmatter`: `just lint` clean (including the zed-dmls wasm32 check);
  `just test` 13849 passed, 10 skipped, 0 failed (includes the 90
  characterization tests, dmls and darkmatter-cli).
- Dependents of `biscuit-file`: `cargo check` clean for darkmatter, claudine,
  messenger, worktree, tree-hugger, biscuit-terminal, content-policy (the
  default build's public surface is unchanged, so no dependent test run was
  needed beyond darkmatter's full suite).
- Cross-OS: `just cross-check biscuit-file --os windows` passed (1114/1114 on
  build-win-native; no observation was published because the remote tree was
  this uncommitted working tree). Windows-gnu clippy was clean in both feature
  configurations.
- The CI scope calculator was not used to skip anything: source files changed,
  so both gates ran.

## Phase 2

Done inline by the orchestrator (no subagents): the moves were small and
sequentially dependent, so splitting them would only have added coordination.

### 2.1 Package area

- New area `simplified-schema/` (library only): `lib/Cargo.toml` (package
  `simplified-schema`, lib `simplified_schema`, edition 2024, `AGPL-3.0-only`,
  `autotests = false`, one `[[test]] l1` root), `README.md` (overview),
  `docs/topics/foundations.md`, `docs/topics/features.md`,
  `docs/dependencies.md`, and a `justfile` modeled on `biscuit-contract`.
- Features: `default = ["file"]`, `file = ["biscuit-file/file-reference",
  "dep:sniff"]`. Normal dependencies are only what moved code uses:
  `tracing`, `serde`, `serde_json` (`preserve_order`, `raw_value`,
  `float_roundtrip`), `thiserror`, `base64`, `biscuit-file`
  (`default-features = false`, `yaml`, `json5`, `file-reference-syntax`), and
  optional `sniff`. `indexmap` and `jsonschema` are not added yet: nothing that
  moved in this phase uses them (they arrive with the modules that do).
  `json5` is needed by `reject_non_finite_json5`.
- `sniff` is declared now (decision 6, confirmed by the inventory) but unused
  until the capture module lands; see decisions_for_review.
- `just test` runs the default suite and then `test-minimal`
  (`cargo nextest run -p simplified-schema --no-default-features`, skipped in
  archive mode like biscuit-file's). `just lint` adds a
  `--no-default-features --all-targets` clippy pass. `bench` is a
  not-applicable stub until benches exist.
- Registered in the root `Cargo.toml` members and the root `justfile`
  `areas :=`. `sniff repo package-areas` lists `simplified-schema`;
  `just check-canonical` passes (13 canonical recipes);
  `just check-tier-coverage simplified-schema` reports nothing stranded.
  No CI manifest enumerates areas by hand: `scripts/ci/affected_scope.py`'s
  `("claudine", "darkmatter", "sniff", "playa")` tuple is specialized-job
  flags and `scripts/ci/consolidation.py`'s list is the historical
  test-consolidation tool; neither needs the new area.
- No `[package.metadata.ci.tests] source-inputs` yet: no test of this crate
  reads another package's file (Phase 8 revisits).

### 2.2 Numeric model

- `git mv darkmatter/lib/src/markdown/numeric.rs` and `numeric/ingress.rs` to
  `simplified-schema/lib/src/numeric.rs` / `numeric/ingress.rs`. Inline tests
  moved with them unchanged (8 + 6).
- Widened to `pub` in the library (Darkmatter's moved schema code and
  frontmatter loader need them across the crate line): `load_admitted_yaml`,
  `is_yaml_admission_error`, `admit_json_text_numbers`, `JsonNumberError`
  (field `reason` gained a doc line).
- `darkmatter/lib/src/markdown/numeric.rs` is now a re-export file
  (`// migration: removed in Phase 7`): the public items keep their public
  paths, the three loader helpers stay `pub(crate)` in Darkmatter.
- Doc drift fixed: module header no longer says "Darkmatter's" model; doc
  examples use `simplified_schema::numeric`. The internal
  `NON_FINITE_MARKER` string (`"darkmatter: non-finite number"`) is unchanged
  on purpose: it never surfaces (it is matched and converted to
  `NotFinite`), and the extraction is behavior-preserving.
- Named boundary cases: `numberlike`/integer boundary — `admit_integer(2^53)`
  is exact and `2^53 + 1` is `Inexact`; `9007199254740993.0` in any spelling
  is `Inexact`; `1e400` is `NotFinite`; `pear` is `NotANumber` (new L1
  `foundations::integer_admission_boundary_is_two_to_the_fifty_third`). The
  existing `numeric::tests::whole_numbers_are_exact_in_every_spelling`,
  `decimals_round_and_extremes_are_rejected`,
  `json_numbers_and_supplied_floats` (i128/f64 cases) moved unchanged.
  Darkmatter's `tests/l1/numeric_admission.rs` (compose, frontmatter, and raw
  schema entry points) stays in Darkmatter as the cross-crate parity test.

### 2.3 Span vocabulary

- `git mv darkmatter/lib/src/markdown/span.rs simplified-schema/lib/src/span.rs`:
  `Spanned<T>`, `line_of_offset`, `line_col_of_offset`, and the
  `SourceSpan` re-export (still `biscuit_file::SourceSpan`). Its 9 inline
  tests moved unchanged.
- `newline_offset_table` and `line_at_offset` (`pub(crate)`, used only by
  Darkmatter's `toc` and `compose/remote.rs`, which stay) were kept in a new,
  smaller `darkmatter/lib/src/markdown/span.rs` that re-exports the library's
  items. Not migration-tagged: `darkmatter::markdown::span::*` and the
  crate-root `darkmatter::{SourceSpan, Spanned, line_of_offset,
  line_col_of_offset}` are Darkmatter's public API (pinned by
  `span_compat`).
- Criterion 8 test: `foundations::spans_are_byte_offsets_into_crlf_text_with_windows_paths`
  (a `C:\…` path and a `\\?\C:\…` verbatim path on CRLF lines; CRLF counts as
  one break; byte columns; an offset between `\r` and `\n`).

### 2.4 Pending codec (required, per the inventory)

- `git mv darkmatter/lib/src/markdown/literal_token.rs
  simplified-schema/lib/src/pending.rs`, whole file (all 11 inline tests).
- The region-free scan core of `ExpressionFinder` moved to
  `simplified_schema::expression::scan`: `ExpressionLocation`,
  `InterpolationLiteral`, `LiteralTokenLocation`, `ExpressionScanResult`, and
  free functions `scan(content, excluded)`, `scan_plain`, `find_all_plain`
  (code-region ranges are an argument: inventory fact F8). The scanning
  logic is byte-for-byte the old `ExpressionFinder::scan` /
  `scan_legacy_expression` / `is_escaped_opener`, with `self.` fields turned
  into parameters. `expression` is the module Phase 3 extends with the lexer
  and parser.
- Darkmatter's `ExpressionFinder` keeps `new` (Markdown code-region
  detection), `with_code_blocks`, `find_all`, `code_regions`; `scan`,
  `scan_plain`, and `find_all_plain` delegate to the library, and the four
  result types are re-exported from `compose::expression::lexer` (same
  paths). Its 2,400-line lexer test module stays in Darkmatter and still
  drives the scan through `ExpressionFinder`.
- `pending::first_token` widened to `pub` (Darkmatter's
  `compose/frontmatter_interpolation.rs` calls it).
- `darkmatter/lib/src/markdown/literal_token.rs` is now a re-export of
  `simplified_schema::pending` (not migration-tagged: the inventory keeps
  `darkmatter::markdown::literal_token` as a lasting path because Claudine
  and Darkmatter compose use it widely); `first_token` is re-exported
  `pub(crate)` so Darkmatter's public API does not grow.
- Minimal test edits (listed per the Move Protocol): in `pending.rs` tests,
  `yaml_value` parsed through `Markdown::from` frontmatter; it now parses
  through `numeric::load_admitted_yaml` and `serde_yaml_ng::from_value` into a
  JSON map, which is the exact conversion Darkmatter's frontmatter loader
  performs (`parse_yaml_map_rejecting_duplicates`). To keep the Darkmatter
  frontmatter fact pinned too, Darkmatter's `literal_token.rs` keeps two
  tests that load through `Markdown::from`:
  `the_yaml_scalar_loads_back_from_frontmatter_as_the_bare_token` and the
  proptest `the_yaml_scalar_round_trips_through_frontmatter`. Scan calls
  changed from `ExpressionFinder::scan_plain` to `scan_plain`.

### Tests added (requirement → test)

| Requirement | Test |
| --- | --- |
| Scan core is Markdown-free and honors caller exclusions (F8) | `expression::scan::tests::excluded_ranges_hide_only_the_openers_inside_them` |
| Scan semantics preserved (offsets, nesting, unclosed, literals, tokens, escapes) | `expression::scan::tests::*` (10) |
| Scan offsets are bytes of the supplied CRLF text | `expression::scan::tests::offsets_are_bytes_of_the_supplied_text_including_crlf` |
| Spans over CRLF + Windows paths (criterion 8) | `foundations::spans_are_byte_offsets_into_crlf_text_with_windows_paths` |
| Numeric boundaries through the public API | `foundations::integer_admission_boundary_is_two_to_the_fifty_third` |
| Loader admission errors keep their location (YAML path; JSON pointer/line/column on CRLF) | `foundations::loaders_reject_inadmissible_numbers_with_a_location` |
| Codec and scan agree (whole leaf = data; embedded = pending; escaped = text) | `foundations::a_whole_leaf_token_is_data_and_never_pending` |
| Darkmatter paths are the library types; finder passes Markdown regions to the scan | DM `tests/l1/simplified_schema_reexports.rs` (2) |
| Token round trip through Darkmatter's frontmatter loader | DM `markdown::literal_token::tests` (2) |
| Every test file is compiled by a declared target | `test_layout::every_test_source_is_compiled_by_a_declared_target` |

Boundary and fidelity: the crate line (Darkmatter → simplified-schema) is
the boundary; F1 in-process tests at each crate's public API, L1. No CLI,
filesystem, or persistence boundary changed. The Input Robustness Matrix
loaders did not change in this phase (number admission moved unchanged; its
existing cells moved with it).

### Checkpoint

- `cargo tree -p simplified-schema --no-default-features -e normal`: no
  `darkmatter`, `claudine`, `content-policy`, `gix`, `walkdir`, `sniff`;
  `globset` present by the Phase 1 decision (the checkpoint's globset clause
  is waived, as the Phase 1 handoff said). The default build holds `gix`,
  `walkdir`, `sniff` through `file`, and no Darkmatter-family crate.

### Phase gates

- `simplified-schema`: `just test` 49/49 (default) + 49/49
  (`--no-default-features`); doctests 6/6; `just lint` clean in both
  configurations; `just check-tier-coverage simplified-schema` nothing
  stranded; `just check-canonical` 28/28 areas.
- `darkmatter`: `just lint` clean (including dmls, darkmatter-cli, and the
  zed-dmls wasm32 check); `just test` 13819 passed, 10 skipped, 0 failed
  (Phase 1: 13849; minus the 34 inline tests that moved to the library, plus
  4 new Darkmatter tests). After narrowing the `first_token` re-export to
  `pub(crate)`, clippy (`--all-targets`, all CI features) was rerun clean and
  the `literal_token` / `frontmatter_interpolation` unit tests (86) pass.
- Dependents: `cargo check --all-targets` clean for claudine, claudine-cli,
  claudine-gen, messenger. Targeted runs over the code that calls the moved
  symbols: claudine (`composition::closure`, `file_detail`, `schema`,
  `dispatch::template`, `lifecycle::tests::nested_span`, `looping`,
  `claudine_schemas_embedded`) 854/854; claudine-cli (`agent_text_is_data`,
  `override_boundary_guard`) 37/37. Messenger calls none of the moved
  symbols, so its compile check stands in for a test run.
- Cross-OS: `just cross-check simplified-schema --os windows` 49/49 on
  build-win-native.
- No test failed or flaked during the phase.
- The gates were not skipped: source files changed, so the CI scope
  calculator was not consulted for a skip.
- `git mv` staged the renames; the index was reset afterwards (nothing is
  staged), so the moves show as deletions plus untracked files.

## Phase 3

Done inline by the orchestrator (no subagents): the three tasks were
sequential (split, then move, then repoint) and touched the same files.

### 3.1 Split `lexer.rs`

- `git mv compose/expression/lexer.rs simplified-schema/lib/src/expression/lexer.rs`
  (the token lexer is the larger half, so history follows it), then
  Darkmatter's `lexer.rs` was rewritten with only the Markdown half:
  `ExpressionFinder` (`new`, `with_code_blocks`, `find_all`, `scan`,
  `code_regions`, `find_all_plain`, `scan_plain`, `find_code_regions`) and
  its three test modules (`expression_finder`, `interpolation_literal`,
  `backslash_escape`), unchanged.
- Moved: `Token`, `ParseMode`, `ComparisonOp`, `LexerError`, `Lexer`,
  `lex_spanned`, `identifier_prefix_start`, `is_identifier_start/char`, and
  the eleven lexer test modules (`lexer_tokens` … `condition_mode_tokens`),
  unchanged. The lexer's number admission now calls
  `crate::numeric::admit_number_spelling` / `NumberTextError` in the library.
- `is_deferred_template`, `defer_templates`, `is_malformed_template`, and
  `is_deferred_value` were never in `lexer.rs` (they are in `deferred.rs`) and
  stay in Darkmatter.

### 3.2 Move tokens, AST, parser

- `git mv` of `ast.rs` and `parser.rs` into `simplified_schema::expression`.
  Import edits only: `crate::markdown::span` → `crate::span`,
  `crate::markdown::numeric::format_number` → `crate::numeric::format_number`,
  `super::{LexerError, ParseMode, Token, ComparisonOp}` → `super::lexer::…`,
  two test-local `use crate::markdown::compose::expression::…` →
  `crate::expression::…`. Doc examples now name `simplified_schema::expression`.
- Minimal test edit (Move Protocol): `parser::tests::raw_literal_block_indentation`
  `include_str!`s two Claudine fixtures
  (`claudine/cli/tests/fixtures/nested_span_regression/{review-spec-inline,commit}.md`);
  the relative path lost two `../` segments. See decisions_for_review.
- `PRECEDENCE_TABLE` widened from `pub(crate)` to `#[doc(hidden)] pub`
  (Darkmatter's `semantics` test compares its operator catalog with it); its
  `#[allow(dead_code)]` was dropped and its doc now says the mirror is in
  Darkmatter.
- New library module docs: `expression/mod.rs` states the module is *syntax
  only* (decision 5's "documented as syntax only") and re-exports the same
  item set Darkmatter's `compose::expression` re-exported, so
  `simplified_schema::expression::{parse, Expr, …}` works.
- Darkmatter: `compose/expression/mod.rs` replaces `pub mod ast;` / `pub mod
  parser;` with `pub use simplified_schema::expression::{ast, parser};`
  (module re-exports keep every `expression::ast::…` / `expression::parser::…`
  path, including `parser::INVALID_CALLBACK_CODE` and `PRECEDENCE_TABLE`), and
  `lexer.rs` re-exports the token items. These three are tagged
  `// migration: removed in Phase 7`; the existing `pub use ast::{…}`,
  `pub use lexer::{…}`, `pub use parser::{…}` lines in `mod.rs` are unchanged
  and resolve through them.
- `error.rs` stays whole (inventory finding 11): the only syntax errors are
  `LexerError` and `ParseError`, which moved with their modules.
  `ExpressionError`'s `From<ParseError>` (in Darkmatter's `error.rs`) now
  converts the library type; no inherent or foreign-trait impl on a moved
  type exists outside the moved files (checked with `rg`).
- **Work counter (inventory F5).** `Parser::with_mode` calls
  `note_work!(ExpressionParse)`. The library gained a test-only
  `work-counters` feature and a `simplified_schema::work_counters` module
  (`Work::ExpressionParse`, `add`, `snapshot`, `WorkSnapshot::{get, since}`)
  plus a crate-private `note_work!`. Darkmatter's `work-counters` feature now
  enables `simplified-schema/work-counters`, and
  `darkmatter::work_counters::snapshot()` adds the library's count into
  Darkmatter's own `Work::ExpressionParse` slot, so every reader
  (`prepared::tests::counted`, Claudine's `preflight::tests::scaling`, the
  `pipeline_work_alloc` bench) is unchanged. The library's
  `[package.metadata.ci.tests] features = ["work-counters"]`, and its `just
  test` / `just lint` now include the feature.

### 3.3 Syntax-only schema checks

- The three checks stay in Darkmatter until Phase 4 moves `schemas/`; they
  now import the library parser directly rather than through Darkmatter's
  re-export: `schemas/format.rs` (`darkmatter-expression` format,
  `parse_condition`), `schemas/simplified/convert.rs` (`expression` default,
  `parse_condition`), `schemas/simplified/grammar.rs` (inline
  `example(call => result)`, `parse_spanned` + `literal_to_json`).
- Verified passive by reading: each calls only a parse function and (for the
  example's expected side) folds literal nodes into JSON; none evaluates,
  runs an example, or looks up a function or variable.

### Tests added (requirement → test)

| Requirement | Test |
| --- | --- |
| Expression-typed value / default parse accepts an unresolvable call (`unknown_fn(1)`) | lib `expression_syntax::an_unresolvable_call_is_an_expression_typed_value_and_default` |
| Function-example call parse accepts `unknown_fn(1)`; spans and `erase` agree | lib `expression_syntax::an_unresolvable_call_is_a_function_example_call` |
| Example expected literal parses to literal nodes | lib `expression_syntax::an_example_expected_literal_parses_to_literal_nodes` |
| Malformed text is rejected with a byte position (parser and lexer errors) | lib `expression_syntax::malformed_text_is_rejected_with_a_byte_position` |
| The dialects differ only in `\|\|` (parser and lexer) | lib `expression_syntax::the_two_dialects_differ_only_in_how_or_parses` |
| Criterion 8: spans over CRLF text with a Windows path | lib `expression_syntax::spans_are_byte_offsets_into_crlf_text_with_windows_paths` |
| Library counts each parse once (success or failure) | lib `expression_syntax::each_parse_is_counted_once_whatever_the_outcome` (`work-counters`) |
| Darkmatter folds the library count into `Work::ExpressionParse` | existing DM `compose::expression::prepared::tests::counted::*` (would read 0 parses if the fold broke) |
| Value check accepts `unknown_fn(1)`, rejects `unknown_fn(` with the format named | DM `schema_expression_syntax_checks::an_expression_typed_value_calling_an_unknown_function_validates` |
| Default check accepts `unknown_fn(1)`, rejects `unknown_fn(` as `SchemaError::Convert` | DM `schema_expression_syntax_checks::an_expression_default_calling_an_unknown_function_lowers` |
| Example check accepts `unknown_fn(1) => {…}`, rejects a malformed call and a non-literal result | DM `schema_expression_syntax_checks::a_function_example_calling_an_unknown_function_parses` |
| Darkmatter's expression syntax paths are the library's types and functions | DM `simplified_schema_reexports::the_darkmatter_expression_syntax_paths_are_the_library_parser` |

Boundary and fidelity: the crate line (Darkmatter → simplified-schema) is the
boundary; F1 in-process tests at each crate's public API, L1. The DM tests
anchor validation in a temp dir (`context_at`), not the cwd. No CLI,
filesystem, or persistence behavior changed. The Input Robustness Matrix does
not apply: the expression grammar is a text grammar, and no loader changed.

### Checkpoint

- `rg "fn parse_spanned"` finds one grammar: the `Parser::parse_spanned`
  method and the free `parse_spanned`, both in
  `simplified-schema/lib/src/expression/parser.rs`. No `Token` enum,
  `lex_spanned`, or `parse_condition_spanned` remains in Darkmatter.
- Darkmatter's expression tests are unchanged: the finder tests stayed, the
  lexer/parser/ast inline tests moved with their modules (no assertion
  edited), and the integration tests that use `compose::expression::…` paths
  compile against the re-exports.

### Docs

- New `simplified-schema/docs/topics/expression-syntax.md`; README,
  `foundations.md` (diagram), and `features.md` (`work-counters`) updated.
- `darkmatter/docs/topics/schemas/parsing/{index,scanning,lexing,grammar}.md`
  pointed at the new source locations. Their old links (`../../../lib/…`)
  were one directory level short and never resolved; the new links were
  checked to exist.
- `.claude/skills/darkmatter/SKILL.md` boundary note extended to the
  expression syntax and the folded counter.

### Phase gates

- `simplified-schema`: `just test` 317/317 (default + `work-counters`) and
  316/316 (`--no-default-features`; the counter test compiles out);
  `just doctest` 12/12; `just lint` clean (default, `work-counters`,
  no-default-features); `just check-tier-coverage simplified-schema` nothing
  stranded.
- `darkmatter`: `just lint` clean (lib, cli, dmls, zed-dmls wasm32), plus
  `cargo clippy -p darkmatter --all-targets --features
  effects-instrumentation,work-counters,terminal-tests,browser-tests -D
  warnings` clean; `just test` 13562 passed, 10 skipped, 0 failed (Phase 2:
  13819; the about 268 moved lexer/parser/ast inline tests now run in the
  library, and 4 Darkmatter tests were added).
- Dependents: `cargo check --all-targets` clean for claudine, claudine-cli,
  claudine-gen, messenger. Claudine targeted run with `work-counters`
  (`preflight::tests::scaling`, which reads `Work::ExpressionParse` through
  the fold, plus `composition::closure`, `dispatch::template`,
  `lifecycle::tests::nested_span`, `looping`): 309/309.
- Cross-OS: `just cross-check simplified-schema --os windows` 317/317 on
  build-win-native.
- No test failed or flaked. The gates were not skipped (source files
  changed). The `git mv` renames were unstaged; nothing is staged.

## Phase 4

Done inline by the orchestrator (no subagents): every task moved files that
import each other, so the library had to compile as one unit before any
Darkmatter edit could be checked. Note: the author committed part of this
phase's working tree mid-run (`2f7c2ad14`, "extract schema language
library"); the rest of the phase is uncommitted on top of it.

### Layout

The moved modules sit **flat at the library root**, mirroring Darkmatter's
`schemas/` tree (`simplified_schema::{simplified, resolve, validate, format,
file_match, json_shape, coerce, coercion, completion, frontmatter_shape,
positions, phase, scope, example, discriminant, reference, advisory, timings,
errors, mapping_orders}`) plus three new files carved out of Darkmatter:
`validation.rs` (`ValidationProblem` and its kind/code, the six
`SCHEMA_*_CODE` constants except `SCHEMA_INVALID_TRIGGER_CODE`, `JsonPointer`,
`FileReferenceDiagnostic`, `CallerFileReferenceProvenance`,
`authored_node_at`), `origin.rs` (`SchemaOrigin`, `SchemaOriginKind`,
`SchemaOriginMap`), `embedded.rs` (`EmbeddedDocument`, `find`,
`resolve_path`, `resolve_bundle_path`, `is_plain_segment`, new
`ShippedSources`), and `file_context.rs` (`document_file_context`,
`resolve_document_file_ref` from `compose/expression/resolve_ctx.rs`).
`lib.rs` re-exports what `schemas/mod.rs` re-exported, so moved code's
`super::X` and `crate::markdown::schemas::X` paths became `crate::X`
mechanically. Darkmatter re-exports the modules at their old paths
(`pub use simplified_schema::{discriminant, errors, example, file_match,
format, simplified, validate}`, `markdown::coercion`, `MappingOrders`),
tagged `// migration: removed in Phase 7`.

### 4.1 AST, grammar, and the `SchemaRoot` rename

- `git mv` of `schemas/simplified/` (all 16 files), `markdown/mapping_orders.rs`.
- The `SimplifiedSchema` enum is `SchemaRoot` everywhere (126 Rust files in
  darkmatter, dmls, cli, claudine, claudine-gen, messenger). Only code tokens
  and type references in docs were renamed; prose naming the *language*
  ("a SimplifiedSchema property", error strings) is unchanged, so no message
  changed.
- Darkmatter already exported `schemas::SchemaRoot` (one of the five schema
  search roots, `roots.rs`). It is now `SchemaSearchRoot` (3 files);
  `SchemaRoots`, `SchemaRootKind`, `SchemaRootState` keep their names. See
  decisions_for_review.
- `simplified/` submodules that were private are `pub` in the library
  (`cursor`, `definitions`, `lint`, `narrowing`, `serialize`, `signature`,
  `source`, `standalone`, `subtype`, `alphanumeric`), and
  `locate_frontmatter_value` is `pub` (Darkmatter's validation needs it).
- Grammar and convert already called `simplified_schema::expression`; now
  `crate::expression`.
- Three tests in `simplified/mod.rs` read the shipped `fm.yaml` and
  `darkmatter_base_json_schema()`; they moved to Darkmatter as
  `schemas/tests/baseline_grammar.rs` (bodies unchanged).

### 4.2 Conversion engine

- `git mv` of `markdown/coercion/` (5 files) and `schemas/coerce.rs`.
- Signatures unchanged. Darkmatter's argument binding still calls the engine
  directly, with no schema document or JSON Schema round trip per argument:
  `compose/expression/call_binding/bind.rs` calls `bind_with` /
  `bind_with_marked` on `Shape`s it translates in `call_binding/translate.rs`,
  through the `markdown::coercion` re-export (the same functions).
- Pending classification stays in Darkmatter (inventory F1):
  `NestedTemplates::find` became `schemas::coerce::nested_templates` (a
  Darkmatter free function; an inherent method cannot be added to a foreign
  type) which walks the instance with `ConversionPlan::shape` and
  `bind_with_marked`, classifying with `is_deferred_template` and
  `holds_pending_syntax`, and hands the paths to the new
  `NestedTemplates::from_template_paths`. `holds_template_syntax` moved with
  it. `bound_operands::marks_operands` (a pure keyword scan) moved into the
  library's `coerce.rs`.
- Tests: `tests/l1/coercion_core.rs` (the engine's public API) moved to
  `simplified-schema/lib/tests/l1/` with its imports repointed;
  `coercion_engine_parity.rs` stays in Darkmatter as the thin integration case
  (it compares frontmatter conversion with expression-call binding, both
  Darkmatter surfaces); its source-reading guard now `include_str!`s the
  library's `coerce.rs`.

### 4.3 Subtype, serialize, query, cursor, source, lint, standalone

- Moved with `simplified/`. `source.rs` needed **no split**: it holds no
  Markdown-position code (it compiled in the library unchanged); the
  document-relative projection was already in Darkmatter's callers.
- `completion.rs` split: the pure union-aware part (`PathStep`,
  `AlternativeSite/Key/Value/Completion`, `for_path_in`, `def_at_path`,
  `alternative_accepts(_in)`, `AlternativeMatcher`,
  `alternative_fingerprint`, `alternatives_for_def(_in)`,
  `root_property_def` now `pub`) moved; `impl DefinitionLookup for
  EffectiveSchema`, `CompletionSuggestion`, `CompletionKind`,
  `for_property`, `completable_properties`, `for_list_item`,
  `for_property_value`, `for_path`, and the tests (they build
  `EffectiveSchema`s) stay in Darkmatter, which `pub use`s the library part.
- Literal-key precedence and union-query helpers are untouched (moved
  byte-for-byte apart from paths).

### 4.4 Resolution and recursion

- `git mv` of `resolve.rs` and `resolve/{graph,recursive,searched_tests}.rs`.
- **Supplied shipped sources.** The resolver no longer reads Darkmatter's
  compiled-in table. `embedded::ShippedSources` (reserved namespace prefix,
  partials directory, documents; `ShippedSources::NONE` for none) is a new
  last parameter of every public resolve entry point (`resolve_schema`,
  `resolve_schema_with_roots`, `resolve_yaml_schema`,
  `resolve_yaml_schema_with_roots`, `resolve_yaml_schema_owned_by`,
  `resolve_catalog_declaration`, `named_type_reference_path`,
  `expand_named_types(_with_locals)`, `check_standalone_named_types`,
  `prepare_embedded_*`) and is held by the `ImportEngine`. The shipped
  named-type memo is keyed by path and text address so two hosts' tables
  cannot share an entry.
- Darkmatter's `schemas/resolve.rs` is now a wrapper module: `pub use
  simplified_schema::resolve::*` plus same-named functions with the old
  signatures that pass `embedded::SHIPPED_SOURCES` (`darkmatter:` over
  `SHIPPED`). Every Darkmatter, DMLS, CLI, and Claudine caller is unchanged.
- The `reserved_namespace` test module (which names `darkmatter:` partials)
  moved into Darkmatter's `resolve.rs` and runs through the wrappers. The
  library gained `resolve::reserved_namespace` tests over a fake `host:`
  namespace: a supplied partial resolves (including an import inside it)
  and records no file; with `NONE` the same spelling is an ordinary file
  reference; an unknown stem is `NoMatch`.
- `SchemaOrigin::{trigger, for_trigger, trigger_source}` read Darkmatter's
  `TriggerSource`; they are the Darkmatter extension trait
  `schemas::TriggerOrigin` (callers import it: 2 Darkmatter L1 tests, 2
  Claudine L1 tests, `claudine/lib/src/composition/schema/bundle.rs`). The
  library gained `SchemaOrigin::trigger_layer(SchemaSourceId)`.
- `HTTP(S)` references still return `SchemaError::RemoteUnsupported` (moved
  tests `remote_https_is_rejected`, `remote_http_is_rejected`).
- `schema_plus_phase1` tests read fixtures under
  `darkmatter/features/_completed/2026-07-08-schema-plus/`; the
  `feature_dir()` path gained `../darkmatter` (minimal edit).

### 4.5 Lowering and validation core

- `git mv` of `validate.rs`, `format.rs`, `json_shape.rs`, `phase.rs`,
  `example.rs`, `advisory.rs`, `discriminant.rs`, `scope.rs`, `reference.rs`,
  `errors.rs`, `timings.rs`, `positions.rs`, `frontmatter_shape.rs`.
- `positions.rs` split: `ProblemPositions` (its `Deferred` variant borrows a
  `Markdown`) stays in Darkmatter's new `schemas/positions.rs`; the library
  gained `PositionMap::locate_all`, which `validate.rs` and
  `ProblemPositions` call.
- `frontmatter_shape.rs` split: `effective_property_shape` /
  `layered_property_shape` (they read the baseline) stay in Darkmatter; the
  library's `layered_property_shape(base, …)` takes the base shape as an
  argument. The module's doctest and three tests use an empty base.
- `SHARED_VALIDATOR_CACHE` moved to `validate.rs` (conversion consults it);
  the library has `reset_process_caches()`, which Darkmatter's
  `reset_process_caches_for_bench` calls before clearing its own memos.
- `format.rs`'s `file_format_package_reference_prefers_package_area_over_repository`
  needs Darkmatter's request capture (`build_resolution_context`); it moved to
  Darkmatter's `schemas/tests/host_file_formats.rs`.
- `markdown::errors::{with_glob_hint, with_hint_line}` moved into the
  library's `errors.rs`; Darkmatter re-exports them.
- `SchemaError`'s `BlockError` impl (and the 19 tests that render it) is
  behind the new opt-in `terminal` feature (inventory F6); Darkmatter enables
  it. See decisions_for_review.
- The mod.rs memo helpers (`phase_json_schema`, `merged_trigger_schema`,
  `CompiledValidators`, `wrapped_root_arms`) stay in Darkmatter for now:
  only `EffectiveSchema` uses them, and `merged_trigger_schema` takes
  Darkmatter's trigger layers. Phase 5/6 moves what the facade needs.
- **Golden.** The existing `tests/l1/recursive_schema_golden.rs` corpus
  already pins every shipped catalog's lowered JSON Schema byte for byte
  (`embedded-darkmatter-base`, every `darkmatter/schemas/partials/*`,
  `content-policy`, the catalog body), generated before this phase and never
  re-blessed. It passes unchanged, so no second golden was added.
- Work counters (inventory F5): the library's `Work` gained
  `ValidatorCompile`, `AlphanumericCheckCompile`, `NamedTypeTableParse`,
  `CoercionFailureBuild`, `AlternativeMatcherPrepare`,
  `RecursiveTargetTranslate`, `PositionKeyComparison`; Darkmatter's
  `library_kind` folds each into its own variant of the same name.
  `PositionMapBuild` and `EffectivePreparation` are recorded in Darkmatter
  code that stayed.

### Minimal (`--no-default-features`) build

Moving the resolver and formats made the library call `biscuit-file`
resolution (`resolve_in_context`, `resolve_detailed`, glob `matches`,
`prepare`), which the syntax-only build lacks. Without the `file` feature:
`file_context::resolve_document_file_ref`, the bare-name and `$schema`
reference probes in `resolve.rs` return the new
`biscuit_file::FileReferenceError::ResolutionUnavailable { reference }`
(classified `MissingContext`); `FileMatchGlobs::matches` and the context-free
match accept nothing; `prepare`/`PreparedFileMatch` exist only with `file`.
Nothing that needs a file passes silently. 83 inline tests that judge files
on disk carry `#[cfg(feature = "file")]` (they run in the default suite); the
`file_match` and `searched_tests` modules are gated whole. New L1
`tests/l1/resolution_unavailable.rs` pins both outcomes through the public
API (control with `file`, typed failure without).

### Tests added or moved (requirement → test)

| Requirement | Test |
| --- | --- |
| Moved code keeps its behavior | 819 more library tests than Phase 3 (317 → 1136 with `work-counters,terminal`): every moved inline test plus `coercion_core` (L1); none edited beyond paths, the `SchemaRoot` rename, and the `ShippedSources::NONE` argument |
| A host's reserved namespace resolves among supplied documents, records no file; `NONE` treats it as a file reference | lib `resolve::reserved_namespace::*` (2) |
| Darkmatter still resolves `Name@darkmatter:<stem>` | DM `schemas::resolve::reserved_namespace::*` (6, moved) |
| Shipped catalogs lower byte-for-byte as before | DM `recursive_schema_golden` (existing corpus) |
| Minimal build fails resolution checks with a typed cause | lib L1 `resolution_unavailable::*` (2, both builds) |
| Baseline grammar and ctx semantics | DM `schemas::tests::baseline_grammar::*` (3, moved) |
| Package-area `^` reference in the eager file format | DM `schemas::tests::host_file_formats` (1, moved) |
| Resolver files never discover git themselves | DM `repository_scope::tests::resolver_inventory_…` now also reads the library's `format.rs`, `file_context.rs`, `file_match.rs`, `resolve.rs` |
| Frontmatter conversion and call binding agree | DM `coercion_engine_parity` (stays) |

Boundary and fidelity: the crate line is the boundary; F1 in-process tests
at each crate's public API, L1. The Input Robustness Matrix: no loader's
behavior changed (the YAML schema, raw JSON Schema, and frontmatter loaders
moved unchanged and their Phase 1 characterization tests pass unchanged), so
no matrix cell changed; the one new outcome (`ResolutionUnavailable`, minimal
build only) is pinned above.

### Checkpoint

- `rg "crate::markdown|crate::lifecycle" simplified-schema/lib/src`: none.
- One implementation each: `fn parse_spanned`, `fn to_json_schema`,
  `fn bind_with` are only in `simplified-schema`.

### Darkmatter and Claudine follow-on edits

- `compose/context/repository_scope.rs` guard
  (`resolver_inventory_has_one_projection_adapter_and_no_discovery_fallbacks`)
  now also reads the library's `format.rs`, `file_context.rs`,
  `file_match.rs`, `resolve.rs` for `find_git_root_from`.
- `tests/l1/context_construction_guard.rs`: the allowlist entry for the
  `DARKMATTER_SCHEMA_CACHE_SIZE` read in `markdown/schemas/validate.rs` is
  gone (the read moved to the library; see decisions_for_review).
- `tests/l1/trigger_spelling_guard.rs`: the allowlist names
  `simplified-schema/lib/src/errors.rs`, and the scan covers the library's
  `src`, `tests`, and `docs`.
- `claudine/lib/tests/l1/boundary_lint.rs`: the `FileReference`-routing check
  reads the library's `resolve.rs`.
- `claudine/lib/src/harness/error.rs` (+ its test): `ResolutionUnavailable`
  → `missing_context`.

### Phase gates

- `simplified-schema`: `just test` 1136/1136 (`work-counters,terminal`) and
  1014/1014 (`--no-default-features`); `just lint` clean (default,
  `work-counters,terminal`, no-default-features); `just
  check-tier-coverage simplified-schema` nothing stranded. The `test` and
  `lint` recipes now enable `terminal` (19 rendering tests).
- `darkmatter` (lib, cli, dmls): `just lint` clean (incl. zed-dmls wasm32);
  `just test` 12748 passed, 10 skipped, 0 failed. The first two runs failed
  on three guard tests whose allowlists or read lists named moved files
  (fixed above); no behavior test failed.
- `biscuit-file`: `just test` 1187/1187 (+6 syntax-only); `just lint` clean.
- Dependents: `cargo check --all-targets` clean for dmls, darkmatter-cli,
  claudine, claudine-cli, claudine-gen, messenger, messenger-cli, research.
  `messenger` `just test` 782/782. `claudine` `just lint` clean; targeted run
  (schema, completion, bundle, prepare, trigger, preflight, file-reference
  filters) 2678/2679, the failure was the boundary lint fixed above (re-run
  7/7).
- `claudine` full `just test`: 11680 passed, **4 failed** (fail-fast stopped
  the last 120 claudine-gen tests): `claudine-gen::l1 cli_switches::{control_row_types_dash_c_as_a_string,
  a_spelling_may_repeat_at_disjoint_command_paths,
  a_revision_one_document_generates_an_explicit_gap,
  every_compiled_provider_has_switches_or_a_gap}`. **Pre-existing, not caused
  by this phase:** the same test fails identically on the Phase 3 tree
  (`f5b53f574`, exported with `git archive` and run in `/tmp`). The
  model-config research documents (`claudine/docs/research/model-config/*.md`)
  write `notes` as a single string while their contract
  (`_schema.yaml`, changed in `f0c34b7b0` on 2026-10-29) types it
  `string[]`, and conversion never wraps a scalar in a list. Fixing it means
  either rewriting about 120 `notes` values as lists or loosening the
  contract, which is the author's choice; it was left as is.
- Cross-OS: `just cross-check simplified-schema --os windows` 1136/1136 on
  build-win-native.
- The gates were not skipped (source files changed).

## Phase 5

Done inline by the orchestrator (no subagents): the result types, the
prepared state, and the Darkmatter switch-over touch one call path, and
the tests run in well under a second.

### Mapping decision (5.1)

- `validate_strict` = the existing **exact** operation, what
  `EffectiveSchema::validate_raw` does: compiled validators judge the value as
  given; problems are collected (root unions per arm, discriminated unions
  narrowed), annotated with the conversion core's reasons, and given their
  declared descriptions.
- `validate_soft` = the existing **coercing** operation, what
  `EffectiveSchema::validate` / `validate_with_positions` does: a value the
  validators accept as written is kept; otherwise the working copy
  (`coerce::validation_copy_with_plan`) is judged by the same collection.
  The accepted value equals `coerce_frontmatter_with_pending`'s.
- Existing callers keep their operation. No Darkmatter, DMLS, or Claudine
  entry point routes through the new methods; Darkmatter's
  `validate_instance` now calls the same moved collection code (see below).

### 5.1 Result types (`simplified-schema/lib/src/outcome.rs`)

- `ValidationSuccess<'input> { value: Cow<'input, Value>, advisories }` with
  `is_converted()`.
- `ValidationFailure` (`#[non_exhaustive]`): `InvalidData { problems }` or
  `UnavailableCheck { problems }`, with `problems()` / `into_problems()`. No
  variant carries a value. A failure with any decided violation is
  `InvalidData`; one whose every problem is an unavailable check is
  `UnavailableCheck`.
- `ValidationProblem::is_unavailable_check()` (`validation.rs`): without the
  `file` feature, a file value's resolution failure
  (`FileReferenceDiagnostic::ResolutionFailed`, which in that build is always
  `ResolutionUnavailable`) or a `match(...)` rejection; always `false` with
  `file`.

### 5.2 `Prepared`, `validate_strict`, `validate_soft` (`prepared.rs`)

- `Prepared::new(Arc<Value>, Option<PathBuf>, FileResolutionContext)`
  compiles through `SHARED_VALIDATOR_CACHE` and finds the conversion plan
  once; `with_advisories`; `json_schema()`.
- Both methods take `&Value`. Strict and unconverted soft success return
  `Cow::Borrowed(input)`; soft after conversion returns `Cow::Owned`. The
  input is never cloned to validate: the new crate-private
  `coerce::validation_working_copy` returns a `Cow` that borrows the
  instance whenever nothing converted (`validation_copy_with_plan` is now
  `validation_working_copy(..).into_owned()`, behavior unchanged).
- Moved from Darkmatter's `schemas/mod.rs` into `prepared.rs` (one
  implementation): `CompiledValidators` (+ new `accepts`,
  `collect_problems`), `wrapped_root_arms` and its `WRAPPED` memo (now cleared
  by the library's `reset_process_caches`), and the description pass
  (`describe_problems`). Darkmatter's `accepts_as_is` and `validate_instance`
  call them; `rewrite.rs` uses `wrapped_root_arms` through the import.
  `phase_json_schema` and `merged_trigger_schema` stay (host policy).
- The revision/context-keyed reuse is the Phase 6 facade's: it owns a
  `Prepared` (see decisions_for_review).

### Tests added (requirement → test)

| Requirement | Test |
| --- | --- |
| R6 row 1: `number`/`4` borrowed by both (`std::ptr::eq`) | lib L1 `validation_contract::r6_table::number_with_number_4_is_borrowed_by_both` |
| R6 row 2: `number`/`"4"` strict rejects (TypeMismatch at `/v`), soft owns `4`, input unchanged, equals `coerce_frontmatter` | `…::number_with_string_4_is_rejected_strictly_and_owned_as_4_softly` |
| R6 row 3: `number(integer)`/`"2.5"` rejected by both, never repaired | `…::integer_with_string_2_5_is_rejected_by_both_and_never_repaired` |
| R6 row 4: `string \| number`/`"4"` borrowed as the string by both | `…::string_or_number_with_string_4_is_borrowed_as_the_string_by_both` |
| Nested conversion in mappings and lists | `conversion_rules::nested_values_convert_inside_mappings_and_lists` |
| Union: as-written arm kept, otherwise converted as the engine does | `conversion_rules::a_union_keeps_the_arm_that_accepts_as_written_before_converting` |
| Numeric precision failure with its reason | `conversion_rules::a_number_the_model_cannot_hold_exactly_fails_with_the_reason` |
| No partial value on failure | `conversion_rules::one_failing_value_fails_the_whole_soft_validation_without_a_value` |
| Structured diagnostics kept (codes, instance/schema paths, missing and unknown keys) | `diagnostics::every_problem_is_kept_with_its_paths_and_code` |
| Declared description rides on its problem | `diagnostics::a_declared_description_rides_on_its_problem` |
| Success exposes diagnostics (advisories) | `diagnostics::success_reports_the_schema_advisories` |
| Input and schema immutability in both methods; one schema, many values | `immutability::both_methods_leave_input_and_schema_unchanged` |
| Root union through both methods | `root_union::a_root_union_borrows_an_arm_match_and_converts_into_an_arm` |
| Unavailable check vs control (both builds) | `unavailable_checks::an_eager_file_value_is_accepted_only_where_it_can_be_resolved`, `…::a_missing_file_is_invalid_data_with_resolution_and_unavailable_without` |
| Mixed failure is invalid data; unavailable problem still marked | `unavailable_checks::a_decided_violation_beside_an_unavailable_check_is_invalid_data` |
| Malformed syntax is decided in every build | `unavailable_checks::malformed_file_syntax_is_invalid_data_in_every_build` |
| Parity with Darkmatter's exact/coercing operations (verdict, problem paths, accepted value, converted flag) on the characterization cells | DM L1 `validation_contract_parity::*` (8 cells) |

Boundary and fidelity: the crate boundary; F1 in-process tests at each
crate's public API, L1. The lib file runs in both builds (`just test` runs
the default-plus-features and `--no-default-features` suites), so the
`UnavailableCheck` branch is exercised in the minimal build. Input
Robustness Matrix: not applicable, no loader or configuration reader changed.

Note found while writing tests: `""` is accepted by SimplifiedSchema's
`file(eager)` (an existing lowering outcome, not changed here); the malformed
syntax case uses `~alice/notes.md`.

### Docs

- New `simplified-schema/docs/topics/validation.md` (strict/soft, result
  types, failure kinds, host mapping), linked from the README,
  `language-core.md` (module table and diagram), and `features.md`.

### Phase gates

- `simplified-schema`: `just test` 1153/1153 (`work-counters,terminal`, +17)
  and 1031/1031 (`--no-default-features`, +17); `just lint` clean (after one
  `clippy::cloned_ref_to_slice_refs` fix in the new test); `just
  check-tier-coverage simplified-schema`: nothing stranded.
- `darkmatter` (lib, cli, dmls): `just lint` clean; `just test` 12756
  passed, 10 skipped, 0 failed (+8 parity tests).
- Dependents: `cargo check --all-targets` clean for claudine, claudine-cli,
  claudine-gen, messenger. Claudine targeted run (`test(/schema|validat/)`)
  1178/1178. Darkmatter's change is internal (private validation helpers now
  called from the library), with no public API change, so no wider
  dependent run was needed.
- Cross-OS: not run. No path, process, or `cfg(windows)` code changed; the
  new tests resolve files only through tempdirs and `./` references, which
  the Phase 4 Windows run already exercises in the same shape.
- The gates were not skipped (source files changed).

## Phase 6

Done inline by the orchestrator (no subagents): the facade's modules share
one internal representation, so splitting them across agents would have
meant agreeing that representation up front anyway.

### Shape of the facade (`simplified-schema/lib/src/schema/`)

| File | What it holds |
| --- | --- |
| `mod.rs` | `SimplifiedSchema` (definition `Arc`, `revision`, `Mutex<Option<Arc<PreparedState>>>`), `TryFrom<&str>`, `validate_strict`/`validate_soft`, `select`, `revision`, `provenance` |
| `layer.rs` | `Layer` (authored form, expanded typed form, lowered JSON Schema or its error, file binding, `SchemaProvenance`), content/Markdown/YAML/JSON construction, resolution of a parsed declaration |
| `layering.rs` | `layer_under`, `layer_over`, `as_base`, `override_with` |
| `view.rs` | `query`, `query_steps`, `property`, `properties`, `named_type`, `TypeDefinition`, `FunctionView`, `PathSyntaxError` |
| `export.rs` | `to_json_schema`, `to_simplified_yaml` |
| `source.rs` | `SchemaSource`, `try_from_yaml`/`try_from_markdown`/`try_from_json_schema`, async `resolve` (feature `file`) |
| `capture.rs` | private `request_context()` (the one raw-path capture), public `repository_scope_catalog` (feature `file`) |
| `error.rs` | `SimplifiedSchemaError` |
| `tests.rs` | private-seam unit tests (prepared identity, capture and read counters) |

- **One read, one parse.** A file factory captures or reuses the context,
  resolves the target, reads the text once (`fs::read_to_string`, or the
  supplied text), and builds the layer from that buffer. The parsed
  declaration goes straight to the resolver's crate-private entry points
  (`resolve_reference_in_context`, `resolve_root_union`,
  `resolve_standalone_root_union`, `resolve_document_examples`, newly
  `pub(crate)`), and named types expand through a new
  `expand_named_types_searched` (the public `expand_named_types_with_locals`
  now delegates to it with a fresh probe set, so its behavior is unchanged).
  Inline Markdown `$schema` resolves with the document namespace
  (`InlineRoot`/`RootUnionArm`); a standalone file with its file namespace,
  exactly as `resolve_inline_mapping` / `resolve_standalone_schema` do.
- **Lowering is kept, not fatal.** A layer stores `Result<Arc<Value>,
  Arc<SchemaError>>`; a function-typed property (no JSON Schema form) builds
  an inspectable, exportable schema whose validation is
  `ValidationFailure::Unprepared`. Expansion errors (unknown named type,
  cycles) are malformed definitions and fail construction.
- **Prepared state.** Built on first validation through `Prepared::new`
  (shared validator cache), stored as an `Arc` in the object, and dropped by
  a committing layering call. A schema that cannot prepare stores the error,
  so every later validation returns the same `Arc<SchemaError>` without
  retrying.
- **Content has no file context.** Validation uses a placeholder
  `FileResolutionContext::from_snapshot("", None, {})` (no process reads,
  fails validation before any probe) and `Prepared::without_file_checks`, so
  failures made only of file checks are `UnavailableCheck` in every build.
  `ValidationFailure::from_problems` gained that flag; the public
  `is_unavailable_check` is unchanged (it delegates to a new crate-private
  `is_file_check`).

### Layering staging measurement (6.5)

Release build, this host (macOS, `rustc 1.98.1`), three runs, 300 iterations
each; receiver 20 properties, incoming baseline 400 and trigger 50 (each
property an inline object of three fields):

| Operation | Run 1 | Run 2 | Run 3 |
| --- | --- | --- | --- |
| Direct host merge (`merge_baseline` twice, cloning each higher layer once) | 1.349 ms | 1.314 ms | 1.318 ms |
| `SimplifiedSchema::layer_under(&[&base, &trigger])` (same merges + stage + unchanged check + commit) | 1.297 ms | 1.307 ms | 1.326 ms |
| Deep copy of the merged JSON (the copy-for-rollback the spec rules out) | 0.502 ms | 0.537 ms | 0.513 ms |
| `SimplifiedSchema::clone` (shares the definition) | 6 ns | 10 ns | 8 ns |

Staging is not material: the facade's only owned working data is the clone a
merge already consumes (the receiver's JSON as the higher side), and its
`merged == receiver` no-op check is within noise. No action taken. The probe
was a throwaway example, not committed.

### Dependency graph (criterion 1, recorded for Phase 8)

`cargo tree -p simplified-schema -e normal`: no Darkmatter, Claudine, DMLS,
or `biscuit-terminal` in either configuration. The minimal build
(`--no-default-features`, 116 crates) holds `globset` (decision recorded in
Phase 1) and no `gix`, `walkdir`, or `sniff`. The default build (363 crates)
holds `reqwest` and `tokio` through `sniff -> schematic-schema ->
schematic-runtime`: present since Phase 2 declared `sniff`, now exercised by
`capture.rs`. Phase 8 must decide whether that counts as an effect stack
under criterion 14 (see `message_to_agent`).

### Tests added (requirement → test)

L1 (`simplified-schema/lib/tests/l1/`), unit tests (`src/schema/tests.rs`),
and the example; all F1, in-process, at the crate's public API except the
private-seam unit tests, which observe the capture/read counters and prepared
identity no public API exposes (the plan made the counter a `cfg(test)` seam).

| Requirement | Test |
| --- | --- |
| C1: schema-only example, AST inspected before validation, many values through one object | `examples/validate_person.rs` (ran: strict/soft output per value); `schema_object::the_example_schema_text_is_the_spec_person`; `validation::one_object_validates_many_values_in_both_modes` |
| C10: spec precedence table | `schema_object::layering::the_spec_precedence_table` |
| C10: whole-property replacement, no intersection | `layering::a_declaration_is_replaced_whole_never_intersected` |
| C10: returned reference aliases the receiver (`std::ptr::eq`); sources unchanged | `layering::the_returned_reference_is_the_receiver_and_sources_are_unchanged` |
| C10: valid first layer, unsupported later layer: receiver unchanged, validation uses prior state | `layering::a_failure_mid_list_leaves_the_receiver_as_it_was`; unit `prepared_state::a_failed_layering_keeps_the_receivers_definition_and_prepared_state` (definition `Arc` and prepared identity) |
| C10: chain keeps earlier successes | `layering::a_chain_keeps_its_earlier_successes_when_a_later_call_fails` |
| C10: next validation observes a change; unchanged definitions keep prepared state | `layering::a_successful_change_is_observed_by_the_next_validation`, `layering::layering_with_nothing_new_keeps_the_revision`; unit `prepared_state::a_successful_change_drops_the_prepared_state_and_an_unchanged_one_keeps_it`, `prepared_state::validation_prepares_once_and_reuses_it` |
| C11: typed errors for missing file, absent structure, malformed definitions | `schema_files::factory_errors_are_typed`, `content::content_without_schema_structure_is_a_typed_missing_schema_error`, `content::malformed_input_is_a_typed_error` |
| C11: classification by structure; raw JSON Schema surface preserved | `content::standalone_envelopes_are_recognized`, `content::raw_json_schema_content_is_recognized_by_its_dialect`, `schema_files::json_schema_files_go_through_the_resolvers_json_loader` |
| C11: unsaved buffer, no reread, no loader setup | `schema_files::a_bound_source_with_supplied_text_reads_no_file`; unit `context_boundary::supplied_text_is_never_read_from_disk` |
| C12: file-looking string is content, never opened | `content::a_file_looking_string_is_content_and_never_opened` (a real file at that path exists) |
| C12: YAML ≡ Markdown, body ignored | `content::yaml_and_markdown_with_the_same_payload_give_the_same_schema`; files: `schema_files::yaml_and_markdown_files_with_the_same_payload_are_the_same_schema` |
| C12: external references retained, no I/O | `content::external_references_are_kept_as_parsed_with_no_io` (import, whole-file reference, file arm, `example(...)`) |
| C13: dotted keys, list and tuple indices, quoted dotted keys, segments | `queries::dotted_keys_indices_and_quoted_keys` |
| C13: empty path, missing match, unsupported step | `queries::empty_missing_and_unsupported_paths_are_none`, `queries::malformed_paths_are_typed_errors` |
| C13: union-query behavior preserved | `queries::a_root_union_answers_with_the_union_of_its_arms` |
| C13: recursion traversed lazily, no expansion; borrowed views | `queries::a_recursive_type_is_queried_lazily_without_expansion` |
| C13: function view keeps names, order, types, optional/rest, return, `fallible` default, description | `queries::a_function_view_keeps_the_whole_signature` |
| C15: raw path captures once; imports, validation, queries, export, layering capture nothing; input read once | unit `context_boundary::a_raw_path_captures_once_and_nothing_downstream_captures_again` |
| C15: bound input and content capture zero times | unit `context_boundary::a_bound_source_and_content_capture_nothing` |
| C15: CWD and home changes after capture do not retarget | unit `context_boundary::process_changes_after_capture_do_not_retarget` (`~/` and `./` file values after moving both) |
| C15: source-relative imports and file values | `schema_files::imports_and_file_values_resolve_from_the_schema_files_directory` |
| C15/C8: CRLF, Windows paths | `content::crlf_content_parses_like_lf`, `schema_files::crlf_files_parse_like_lf`; unit `context_boundary::a_windows_path_with_a_drive_resolves` (`cfg(windows)`); every file test uses temp-dir paths, which are `C:\…` on Windows |
| C15: raw relative path and raw reference resolve from the captured directory | `schema_files::a_raw_relative_path_resolves_from_the_process_directory` |
| C16: parse → export → parse, metadata, union arms, functions, finite recursive graphs | `export::object_and_metadata` (descriptions, `$constraints`, pattern keys, per-arm union descriptions), `export::root_union_arms`, `export::function_definitions` (with categories), `export::finite_recursive_named_definitions` |
| C16: raw JSON Schema export unsupported; function lowering keeps its located error; no silent drop | `content::raw_json_schema_content_is_recognized_by_its_dialect`, `validation::a_function_typed_property_cannot_be_prepared_and_keeps_its_located_error`, `export::an_inline_object_the_serializer_cannot_write_is_refused`, `layering::a_layered_schema_exports_its_merged_properties` |
| R4: HTTP(S) stays `RemoteUnsupported` | `content::remote_references_are_rejected_at_parse`, `schema_files::remote_references_are_unsupported` (factory and async `resolve`) |
| Content file checks are unavailable, not violated | `validation::file_checks_of_content_are_unavailable_not_violated` |
| Named-type selection (spec `Person`) | `validation::a_selected_named_type_validates_its_own_values` |
| `resolve` by extension | `schema_files::resolve_selects_the_loader_by_extension` |

Input Robustness Matrix, for the new content loader (`TryFrom<&str>`) and
the factories, which reuse the existing loaders: the default outcome is a
typed error and no schema. Exceptions pinned: `$schema` absent is
`MissingSchema`; explicit null (`$schema:` and `$schema: null`) is *not*
absence and reaches the declaration parser (`Grammar`); a number is
`Grammar`; `{}` is an empty object schema (accepted); `[]` is an error.
Document shapes: the YAML loader's duplicate key and malformed text
(`schema_files::factory_errors_are_typed`, `content::malformed_input_is_a_typed_error`),
the JSON loader's trailing content, duplicate key, and non-object
(`json_schema_files_go_through_the_resolvers_json_loader`), the frontmatter
extractor's near-miss fence and empty block, and CRLF. Smell grep over the
new code (`src/schema/`): no `#[serde(default)]`, no `unwrap_or_default()` or
`.ok()` on a load-bearing parse; the two `.ok()` in `capture.rs` drop an
invalid scope catalog exactly as Darkmatter's capture does (the context then
carries no catalog), and `view.rs`'s `unwrap_or_default` lists no
properties for a schema that does not lower.

Test-file notes: the original Phase 5 contract tests are unchanged. One test
expectation was corrected while writing (an earlier draft of the test, not
the code): the existing root-union property lookup returns an owned union
even for a key only one arm declares (`root_property_def`), so the view is
not borrowed there; pinned as is.

### Darkmatter edit

`darkmatter/lib/src/markdown/compose/context/repository_scope.rs`:
`repository_scope_catalog` is now `pub use
simplified_schema::repository_scope_catalog` (the inventory's F7: one
projection for Darkmatter and the library's capture). Its body and the
`package_area_root` helper moved; its unit tests stay and pass
unchanged against the re-export.

### Docs

- New `simplified-schema/docs/topics/schema-object.md` (building, the
  raw/bound/supplied-text flow as a Mermaid diagram, validation, layering
  with the precedence table, queries, export).
- `README.md` (facade example first, link), `validation.md`
  (`Unprepared`, content file checks), `features.md` (file factories),
  `language-core.md` (module table and diagram).
- `.claude/skills/darkmatter/SKILL.md`: the facade and the shared
  projection.

### Checkpoint

- Criteria 1, 10, 11, 12, 13, 15, 16: see the requirement → test table; the
  example ran and printed per-value strict/soft verdicts.
- Criterion 14, part 1: both builds compile, lint, and pass; the minimal
  build runs every test that does not need a file (39 of the 53 new tests;
  the file-factory and capture tests are `cfg(feature = "file")`).

### Phase gates

- `simplified-schema`: `just test` 1206/1206 (`work-counters,terminal`,
  +53) and 1070/1070 (`--no-default-features`, +39); `just lint` clean in
  both configurations; `just check-tier-coverage simplified-schema`: nothing
  stranded; the new `select` doctest passes (`cargo test --doc`).
- `darkmatter` (directly changed: the re-export; lib, cli, dmls): `just lint`
  clean; `just test` 12756 passed, 10 skipped, 0 failed.
- Dependents: `cargo check --all-targets` clean for claudine, claudine-cli,
  claudine-gen, messenger, dmls, darkmatter-cli. Claudine uses
  `compose::repository_scope_catalog` (`invocation_context.rs`); targeted run
  `test(/invocation_context|repository_scope|scope_catalog|schema|validat/)`
  1235/1235. DMLS's use of `expand_named_types_with_locals` is covered by the
  Darkmatter area run.
- Cross-OS: `just cross-check simplified-schema --os windows` (native
  Windows): 1207/1207 passed, including the `cfg(windows)` drive-path test.
  Run because the facade joins, anchors, and captures paths. The rig tested
  the working tree (no observation published for this head, as the recipe
  reports).
- Scope calculator over the working tree (`git status --porcelain
  --untracked-files=all | awk '{print $NF}' | xargs python3
  scripts/ci/affected_scope.py`): `packages` = darkmatter,
  simplified-schema, test-toolkit, darkmatter-cli; `change_class` =
  package. `test-toolkit` is selected by every Rust source file (its
  repository-wide source checks); `cargo nextest run -p test-toolkit`
  479 passed, 2 skipped.
- The gates were not skipped (source files changed).
- No load-sensitive or pre-existing failures were seen.

## Phase 7

Done inline by the orchestrator (no subagents): the work was one
compiler-driven import migration across six packages plus a handful of
targeted tests, and splitting it would have meant several agents editing
the same `use` lines.

### 7.6 first: the re-exports are gone

Every `// migration: removed in Phase 7` re-export was deleted, and with it
the untagged lists in `darkmatter::markdown::schemas` that re-exported the
library's types (`SchemaRoot`, `SchemaError`, `PropertyDef`,
`ValidatorCache`, `parse_yaml_schema`, `to_json_schema`, …, and
`alias_search_work`). See `decisions_for_review` for why the untagged lists
went too.

| Removed | Now imported from |
| --- | --- |
| `markdown::numeric` (file deleted), `markdown::coercion` | `simplified_schema::{numeric, coercion}` |
| `schemas::{discriminant, errors, example, file_match, format, simplified, validate}`, crate-private `phase`, `scope`, `timings` | `simplified_schema::…` (the schemas module keeps private `use`s for its own code) |
| `schemas::{SchemaRoot, SchemaError, …}` (both root lists) | `simplified_schema::…` |
| `schemas::coerce::*` glob | `simplified_schema::coerce`; Darkmatter's `schemas::coerce` keeps only `nested_templates` and is now `pub(crate)` |
| `schemas::resolve::*` glob | `simplified_schema::resolve`; Darkmatter's `schemas::resolve` keeps its wrappers that pass the shipped sources (`resolve_schema`, `resolve_schema_with_roots`, `expand_named_types_with_locals`, `prepare_embedded_*`, …) |
| `compose::expression::{ast, parser}` modules, the `lexer` token re-exports | `simplified_schema::expression::{ast, parser, lexer}`; `compose::expression` still re-exports the items its composition API names (`Expr`, `SpannedExpr`, `Lexer`, `Token`, `ParseMode`, `parse`, `parse_spanned`, …), as Phase 3 decided |

Kept, as earlier phases decided: `markdown::span`, `markdown::literal_token`,
`compose::repository_scope_catalog`.

How: two throwaway scripts (kept outside the repo) split every
`use darkmatter::…::{…}` group, nested groups included, by each leaf's
full path. A leaf that names a library item moved to a
`use simplified_schema::…;` line; a leaf that names a Darkmatter wrapper
(the `resolve` wrappers, `coerce::nested_templates`) stayed. Inline paths
were rewritten the same way. The compiler did the rest (relative
`super::ast` paths in `compose/expression`, `resolve::merge_baseline` in
`schemas/`, and the in-crate test modules, which now import
`Constraint`, `SchemaOriginKind`, `to_json_schema`, and `parse_yaml_schema`
from the library). Long generated `use` lines were wrapped; nothing else was
reformatted.

New direct `simplified-schema` path dependencies: `darkmatter-cli`, `dmls`,
`claudine`, `claudine-cli`, `claudine-gen`, and `messenger` (optional, under
its `research` feature). Each already had it in its graph through
Darkmatter, so `Cargo.lock` gains edges and no package.

`rg 'darkmatter::markdown::schemas::simplified' --type rust` and
`rg 'migration: removed in Phase 7' --type rust` both return nothing. The
stale doc-comment examples in `simplified-schema` that still named
`darkmatter::markdown::{coercion, schemas::simplified, schemas::format}`
now name the library's paths.

Test edits forced by the removal (minimal, listed per the conventions):
`darkmatter/lib/tests/l1/simplified_schema_reexports.rs` dropped its numeric
assertion (`markdown::numeric` no longer exists, so it would compare the
library with itself) and spells the expression items through
`compose::expression`'s remaining re-exports instead of the removed `ast`,
`parser`, and `lexer` module paths. Every other test change is an import
line.

### 7.1 Darkmatter adapters

`DarkmatterSchemas`/`EffectiveSchema` do not hold a `SimplifiedSchema`
object (decision recorded). The effective schema is still merged by
`merged_trigger_schema` over the already lowered baseline, trigger, and
document layers with `simplified_schema::resolve::merge_baseline`, the
same merge the object's `layer_under` runs, in the same low-to-high order
with the document as the receiver. Its doc comment now says so and why
(memo by `Arc` identity; no per-page schema objects). Origin tracking,
pending classification, literal tokens, lifecycle operands, default and
eager-file handling are untouched.

New `darkmatter/lib/tests/l1/effective_schema_layering.rs` (F1, in-process,
public API of both crates) pins the parity and the reuse:

| Requirement (criterion 4) | Test |
| --- | --- |
| Document over custom baseline and a matched trigger equals `doc.layer_under([&baseline, &trigger])`; document wins `title`, each layer's own key survives; verdicts | `the_document_schema_is_layered_over_the_baseline_and_a_matched_trigger` |
| No document schema: trigger over baseline, as the library layers them | `a_trigger_wins_over_the_baseline_when_the_document_has_no_schema` |
| Absent document schema with an enabled baseline: shared `Arc` across pages; disabled baseline and no layers: `None` and a vacuous report (`has_schema == false`), while the library constructor's absence is the typed `MissingSchema` | `an_absent_document_schema_keeps_the_shared_baseline_and_no_layers_is_no_schema` |
| A lone matched trigger is shared as resolved; an unmatched page has no schema | `a_lone_matched_trigger_is_shared_as_resolved` |
| Shipped baseline: the process baseline itself (`ptr::eq`), and layered like the library | `the_shipped_baseline_layers_like_the_library_and_is_never_cloned_per_page` |
| Compiled-validator reuse: two pages, same effective schema, one memoized merge and one validator (`ptr::eq`) | `compiled_validators_are_reused_across_pages_with_the_same_effective_schema` |

Existing reuse tests that still hold: `darkmatter_base_json_schema_is_cached`,
`darkmatter_base_schema_is_prepared_once_with_its_json`,
`darkmatter_baseline_builder_shares_cached_json_schema` (unit), and
`content_policy_embedded_baseline` (L1, `ptr::eq` with the process baseline).

Finding while writing the shipped-baseline case: the library's content
constructor (`SimplifiedSchema::try_from(&str)`) reads JSON text through
the YAML parser, which caps an implicit key at 1024 characters, and the
shipped baseline has a longer `patternProperties` key; the test passes the
baseline as YAML text. Not changed (decision recorded): it is the same
loader Darkmatter uses for YAML schema files and frontmatter.

### 7.2 Darkmatter CLI

Only imports changed (6 files). `md schema` behavior evidence is the
existing CLI suite in the area run below.

### 7.3 DMLS

Imports only (24 files); DMLS already builds every effective schema from
the open buffer's `Markdown` through `DarkmatterSchemas`, with no path-only
factory and no reread. New test
`dmls/tests/l1/core_schema_types.rs::an_unsaved_document_schema_judges_the_buffer`:
the saved page declares `count: string` (valid), the open buffer declares
`count: number` with `count: many`; DMLS reports one
`dm.schema.type_mismatch` on `many`, the saved file is unchanged, and the
library accepts the saved page. Searched-but-absent candidate invalidation
is the existing `searched_candidates.rs` and `overlay/registry.rs` tests
(`a_nearer_candidate_created_in_an_absent_folder_invalidates_a_ready_registry`
and siblings), unchanged and in the area run.

### 7.4 Claudine

`claudine-gen`'s `schema_compat.rs` and `inputs.rs` (`coerce`) and every
schema-only use in `claudine` and `claudine-cli` now import
`simplified_schema`; Markdown, triggers, and composition stay on Darkmatter.
No `source-inputs` entry named a moved or deleted file.

### 7.5 Messenger

`research/load.rs` imports `ValidationProblemCode` from the library. It
still validates through `EffectiveSchema::validate` and deserializes the
authored frontmatter (`loaded.frontmatter`), never the converted value;
`research_validation::a_quoted_schema_version_is_unsupported_even_though_the_schema_coerces_it`
and `strict_scalars_reject_authored_type_mismatches_at_their_pointer` are
the evidence.

### Docs and skills

- `darkmatter/lib/src/markdown/schemas/mod.rs` module docs rewritten (the
  module layout listed moved modules as Darkmatter's), and the old
  `[SchemaRoot]`-style link block removed; `compose/expression/{mod,lexer}.rs`
  module docs no longer promise the old paths.
- `darkmatter/docs/dependencies.md` (the SimplifiedSchema entry),
  `claudine/docs/dependencies.md` (`claudine-gen`), `docs/dependencies.md`
  (the new direct edges), `darkmatter/docs/topics/schemas/definition.md`
  (two code examples).
- Skills: `.claude/skills/darkmatter/{SKILL,schema,compose,expressions,frontmatter}.md`
  name `simplified_schema::{numeric, coercion, SchemaRoot, …}` and describe
  what Darkmatter still re-exports and how the effective merge relates to
  `layer_under`.

### Library doctests (found and fixed)

`cargo test -p simplified-schema --doc` failed 11 of 29 before this phase:
the doc examples moved in Phase 4 still said `use darkmatter::…`, which the
library cannot see, and nextest (the `just test` runner) never runs
doctests, so no gate caught it. Their imports now name the library's paths
(`mapping_orders.rs`, `timings.rs`, `json_shape.rs`,
`simplified/{alphanumeric,cursor,source,subtype}.rs`; `subtype.rs`'s
`resolve_yaml_schema` example also passes `ShippedSources::NONE`, the
argument the library's resolver takes). Result: 29 passed, 2 ignored.
Comment-only edits in `coercion/mod.rs`, `format.rs`,
`simplified/{convert,serialize}.rs` likewise name the library's paths.
Darkmatter's doctests (whose examples the import migration rewrote): 209
passed, 8 ignored.

### Phase gates

- `darkmatter` area (lib, cli, dmls): `just lint` clean; `just test` 12763
  passed, 10 skipped, 0 failed (12756 before, plus the 6 parity tests and
  the DMLS unsaved-buffer test). `just check-tier-coverage darkmatter`:
  nothing stranded. Doctests: 209 passed.
- `simplified-schema`: `just lint` clean (both configurations); `just test`
  passed in both configurations (1070 in the minimal one); doctests 29
  passed (11 failed before; see above).
- `claudine` area: `just lint` clean; `just test` 11680 passed, 4 failed,
  15 skipped. **The 4 failures are pre-existing and not caused by this
  phase**: `claudine-gen::l1 cli_switches::{control_row_types_dash_c_as_a_string,
  a_spelling_may_repeat_at_disjoint_command_paths,
  a_revision_one_document_generates_an_explicit_gap,
  every_compiled_provider_has_switches_or_a_gap}` fail deterministically
  (rerun alone: same) with `ResearchInvalid` for the copied
  `claudine/docs/research/model-config/codex.md`: its `notes` values are
  strings, and the sidecar `_schema.yaml` declares `notes: string[]` since
  commit `f0c34b7b0` (2026-10-29, "support local schema types and ordered
  output", before Phase 1 of this plan; it changed `notes: string` to
  `string[]` without updating the documents). `md schema validate
  claudine/docs/research/model-config/codex.md` reports the same problems.
  `claude.md`, `gemini.md`, `goose.md`, `kilo.md`, `opencode.md`, and
  `pi.md` hold scalar `notes` too. Not fixed here: choosing between
  reverting the sidecar to `string` and rewriting seven research documents
  is a contract decision outside this extraction (recorded in
  `message_to_agent`). The import change in `claudine-gen/src/inputs.rs` is
  the same function under its owning path.
- `messenger`: `just lint` clean; `just test` 782 passed, 2 skipped
  (includes the 30 `research_validation` cases).
- Other packages the scope calculator selected: `test-toolkit` 479 passed;
  `zed-dmls-cli` 27 passed; `sniff`, filtered to its workspace, lockfile,
  package, and git readers (`test(/lockfile|integration|git_parity|package|workspace|repo/)`),
  911 passed.
- Scope calculator (`git status --porcelain --untracked-files=all | awk
  '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`): `packages` =
  claudine, claudine-cli, claudine-gen, darkmatter, darkmatter-cli, dmls,
  messenger, simplified-schema, test-toolkit, sniff, zed-dmls-cli;
  `change_class` = package. Gates were not skipped.
- Cross-OS (native Windows, `just cross-check <package> --os windows
  <name filter>`): run because the import rewrite was checked by the
  compiler only for code built on macOS, and a `cfg(windows)` block naming
  a removed path would compile nowhere else. A name filter still builds
  every test target. `darkmatter`: all targets built, 6 passed.
  `darkmatter-cli` (`schema_validate`) 155 passed; `dmls`
  (`core_schema_types`) 11 passed; `claudine` (`invocation_context`) 49
  passed; `claudine-cli` (`schema_completion`) 52 passed; `claudine-gen` and
  `messenger` built every target (their filters matched no test). No
  Windows failures, so `os_failing` is not set.
- No load-sensitive failures were seen.
