---
$schema: "@.claudine/schemas/review.yaml"
ready: false
agent: codex/default
created: 2026-07-11T12:35:50
implemented: true
---

# Review: Authored Expression-Function Schemas

## Verdict

Not ready for production.

The authored catalog is embedded, projected into the existing descriptor API, and consumed by the documentation and DMLS paths. The focused parser tests pass. However, two core requirements are not implemented: the parser does not validate catalogs through their SimplifiedSchema declaration, and runtime dispatch does not enforce catalog-derived overload eligibility. The latter is masked by a circular parity test.

## Findings

### High: runtime dispatch ignores catalog-derived overload eligibility

Requirement 7 and the runtime-binding design require the registry to decide whether an authored overload is eligible before invoking its handler. Both dispatch paths calculate an eligible overload and discard it:

- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2461`
- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2465`
- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2495`
- `darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2499`

Consequently, changing the YAML parameter shapes does not define dispatch arity. A matching canonical name still invokes the Rust handler at any arity, leaving the handler's handwritten checks as the actual authority. This is the duplicate arity authority the specification intended to remove.

The purported overload parity test cannot detect this. `dispatchable_signatures()` is built directly from the same catalog-derived descriptors it is compared against (`functions/mod.rs:2438-2441`), so descriptor/runtime set equality is true by construction. The end-to-end test exercises only one arity per signature, counts optional parameters as present, and uses two arguments for every variadic signature (`catalog/mod.rs:414-429`, `catalog/mod.rs:544-558`). It does not verify each overload's minimum and maximum arity or rejection immediately outside those bounds as required by the test plan.

The registry should gate handler invocation on `accepts_arity`. Preserve established diagnostics by returning a registry-produced arity error or by calling a handler-independent diagnostic helper, rather than invoking an ineligible handler. Add Level 1 tests covering minimum and maximum arities, one below and above bounded ranges, and representative zero/multiple variadic arities through both pure and context dispatch paths.

### High: catalog loading never validates the authored SimplifiedSchema

Requirement 3 says the catalog must be structurally validated using SimplifiedSchema before dedicated function-domain validation. `RawCatalog` instead accepts `$schema` as an optional, opaque YAML value (`catalog/parser.rs:62-68`), and `parse_expression_function_catalog` immediately deserializes into the Serde shapes. It never calls `parse_yaml_schema`, `to_json_schema`, or a validator.

The two schema tests validate only the fixed test documents independently of the production parser. A catalog with no `$schema`, or with a malformed/incomplete `$schema`, can therefore be accepted and promoted by the embedded accessor. This also means the authored schema is not the structural authority in the loading path.

Make `$schema` required, parse it through the existing SimplifiedSchema API, validate the complete YAML instance before semantic projection, and return a structured catalog error for schema declaration or instance failures. Add fixtures proving the production parser rejects a missing, malformed, and structurally incomplete declaration.

### Medium: the fallible fixture projection API leaks valid fixture data

The lifetime decision permits leaking only the one bounded, successfully validated embedded catalog; fixture parsing must remain owned. `try_parse_catalog` accepts arbitrary YAML and calls `project_descriptors`, which leaks strings and parameter slices (`catalog/mod.rs:271-324`, `catalog/mod.rs:351-355`). Repeated valid fixture parsing therefore creates unbounded process-lifetime leaks. The existing test covers only malformed input, which fails before projection.

Keep the fallible fixture parser returning the owned `ExpressionFunctionCatalog`, or introduce an owned descriptor projection for tests. Restrict the leaking projection to the embedded `LazyLock`, and add a repeatable valid-fixture test that does not require static promotion.

## Test Rigor

The user-facing changes are schema metadata, evaluator dispatch, generated Markdown, CLI report content, and DMLS completion/hover. Level 1 is appropriate for parser, evaluator, generated-document, and DMLS semantic assertions. The CLI report's terminal presentation has Level 2 tmux coverage, which is sufficient because there is no terminal input-encoding requirement. No Level 3 coverage is applicable.

The strongest relevant coverage is Level 1 for the two blocking requirements, but it is incomplete and partly circular as described above. The reported package-wide Level 1 and Level 2 runs do not close those requirement-level gaps.

Focused command run:

```sh
cargo test -p darkmatter catalog::parser::tests --color=never
```

Result: 9 parser tests passed. This confirms the current tests, not the missing production-path guarantees.
