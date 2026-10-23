# Expression Evaluation Rules

Detail behind the "Testing and verification" section of [SKILL.md](SKILL.md): the expression grammar gate, inserted-text rules, failure policy, undeclared properties, host bindings, and warning identity.

The expression-grammar corpus gate is
`lib/tests/prompts/dasherized_identifier_corpus.rs`, opt-in through the
`prompt-tests` feature and run by `just test-prompts`, never by CI. It walks
root `prompts/`, `.claude/commands/`, `darkmatter/prompts/`, and
`claudine/prompts/` through the library's own extractors: `ExpressionFinder`,
`scan_darkmatter_directives`, `parse_frontmatter_shell_value_spanned`, and
`frontmatter_expression_values`, which finds Expression-typed properties of
any name under each document's passively resolved effective schema, the
classification DMLS uses. Claudine's `when`/`while`/`until` keys stay as a
separate check because Claudine's schema types them as `string` but
evaluates them as lifecycle conditions. The gate requires every extracted
expression to parse. Run it after any lexer or parser
change. A red result names a shipped prompt, and that prompt usually already
fails to compose. Check with `md compose` before blaming the grammar.

Identifiers may contain `-`: `read_variable` joins a `-` only mid-identifier
and only when the next character continues an identifier, so `spec-name` is
one name while `a - b`, `a -b`, `4-2`, and `foo--bar` stay subtraction. The
rule lives in the lexer, never in `is_identifier_char`. Any cursor-side scan
(DMLS completion partials) must call `expression::identifier_prefix_start`
rather than add `-` to a character class, which would merge `foo--bar`.

A `{{ … }}` that cannot be parsed or evaluated fails full-document
composition regardless of `fail_fast`.

**Design-time call check** (`expression/call_check.rs`). One classifier gives
every call argument one of five `ArgumentCategory` values, and composition
and DMLS both read it.
- Composition (`compose/call_precheck.rs`) rejects a document whose surface
  holds a category 5 call before anything evaluates, dead branches included.
  The error is `ExpressionError::CallCheck`; find it with
  `MarkdownError::rejected_call()`.
- DMLS maps categories to `dm.expression.incompatible_call` (5) and
  `dm.expression.uncertain_call` (3; also 2 and 4 under `[expressions] strict`).
- Build the site over `DeclaredTypes::new(schema)` without static frontmatter:
  a set value is runtime data.
- An all-literal call is classified by the real `bind_call`, which runs no
  handler.
- An `as_sent` source (`numberlike`, `boolish`) is two representations, the
  native scalar and retained text (`Family::NumberText`/`BooleanText`,
  `source_kinds` adds `Text`); never collapse it to one primitive family, or
  `min(boolish, 5)` becomes a false category 5 and `numberlike → json` a
  false category 1.
- New authored-expression surfaces must be added to both consumers.
  `dmls/tests/l1/call_check_agreement.rs` pins their agreement on one corpus.
- Projection limits are intended, not gaps to patch locally: paths below a
  top-level property, raw JSON Schema, and root unions with a raw arm or
  disagreeing arms are `unknown`; `replace` insertions, `::shell` `{{ }}`
  with body interpolation off, and unparsable expressions are not prechecked.
  User docs: `darkmatter/docs/inline/call-check.md`.

**Function binding** (`expression/call_binding/`). Every call binds through
the shared coercion engine before its handler runs; handlers are
`PureFn = fn(&BoundCall) -> Result<Value, String>` and
`ContextFn = fn(&BoundCall, &ResolutionContext)`, and read only `BoundArgs`
accessors and `overload()`. Never re-check types, parse, or convert inside a
handler: the catalog signature in `schemas/partials/functions.yaml` is the
whole contract, and a handler that needs the caller's original form declares
`numberlike`/`boolish`/`any` instead. A binding failure is
`ExpressionError::Call(CallBindingError)`; only `ImplementationContract` (a
handler returning a value outside its declared `returns`, checked by
`check_return` without conversion) is authoring-fatal. Null reaches a
parameter only through an explicit `| null`. `and`/`or` are lazy
(`LazyCall`): the precheck visits every argument, the runtime binds only the
ones it evaluates. User docs: "Function Calls: Bind, Then Compute" in
`darkmatter/docs/topics/schemas/definition.md`.

**Inserted text is data.** Every authored span is scanned once. Produced text
is never scanned again. This covers expression results, file reads, shell
output, `{{{ }}}` results, data overrides, decoded tokens, and values a parent
passes to a child. There is intentionally no fixed point:
`note: "fixed {{{ area }}}"` renders `{{ note }}` as `fixed {{ area }}`. A
template that relied on a rescan is rewritten at the source. Rules to keep:

- Origin travels beside values. Frontmatter uses `compose/value_origin.rs`,
  and the body uses `DataRanges`. A directive scanner reads
  `parse_utils::structural_view`.
- Frontmatter `$( … )` runs only when its *authored source* is a whole value.
- Guards judge authored syntax, never flattened text.
- `--set` (`with_set_overrides`) stays an authored template, and its failures
  name the override (`SourceRef::Supplied`). Callers pass produced values
  with `with_data_overrides` or `with_override_layers`.
- A stored `"{{!data:v1:<base64url>}}"` (`markdown::literal_token`) is a data
  string. Loaders keep it encoded, and readers call `decode_literal_tokens`.
- A malformed token is a located, authoring-fatal `MalformedLiteralToken`.
- Data is never schema-pending (`holds_pending_syntax`).
- `hash::locate_frontmatter_leaves` finds a leaf's bytes for an in-place
  write.

Details are in [compose.md](compose.md#inserted-text-is-data) and
[frontmatter.md](frontmatter.md#literal-tokens).

`interpolate_text`/`interpolate_value` take an explicit
`ExpressionFailurePolicy`. Pass `Strict` from document stages. Use `Lenient`
only for preflight discovery (see
[compose.md](compose.md#error-handling)). The error carries the authored
span (`SourceRef::OnDiskSpan`) whenever it is provable, including after an
earlier stage rewrote the body and inside block (`|`, `>`), multi-line,
tagged, anchored, or aliased frontmatter scalars. `interpolation_block` renders the file and
authored line and column for every cause, and
`interpolation/fatality_characterization.rs` is the drift guard.

A well-formed identifier that resolves to nothing is an absent document
property (`null`); a bare name never falls back to `ctx`. Compose reports it as
the advisory `dm.expression.undeclared_property` (once per root per document)
unless the author handled its absence (`x || d`, `x ? …`, `is_null(x)`) or the
root is declared by the final state, a caller input, or the effective schema.
Read
[compose.md](compose.md#undeclared-properties-dmexpressionundeclared_property)
before changing a runtime evaluation surface: new surfaces must observe
through the shared `AbsenceScope`, not a new walk. DMLS reports the same
`dm.expression.undeclared_property` at `WARNING` through the static twin,
`expression::static_variable_reads`, classified by `BindingView::baseline()`,
and an unknown function as the `ERROR` `dm.expression.unknown_function` through
`expression::validate_expression` (see
[dmls.md](dmls.md#undeclared-properties-and-unknown-functions)).

**Type scope** (`expression/scope/`). `TypeScope` is the one source of
static presence and type at a site: `DeclaredTypes` (built once per owner
from `EffectiveSchema.simplified`, static frontmatter, and host roots or a
`BindingView`) refined by guards. `TypeScope::assume(&Expr, Polarity)` is the
only fact producer. It handles `!`, parentheses, `and`/`or`/`||` (lowered to
`FunctionCall`/`Fallback`, matched through the registry), ternaries,
`==`/`!=` by strict typed equality (`Shape::equal_to`/`differs_from`, and
`finite_values` for subject-to-subject), and the predicate
facts in `scope/facts.rs::PREDICATE_FACTS`, which a unit test keeps
registry-backed. `invalidate(&WriteSet)` (`WriteSet::destination` for a
`set` path) and `TypeScope::join` keep facts from leaking past writes or
regions. There is no `Default`; an unguarded site calls
`TypeScope::unguarded`. Only `ScopedType::guard_presence` (not a
declaration) may excuse a discovery-time null, and `ScopedType::admits`
checks a live value against the refinement. `template::project_hole(expr,
&TypeScope)` reads the declaration from the scope's `DeclaredTypes` and, when
a guard refined the subject, approves the narrowed type and records it as
`HoleType::refinement`. Its consumers: shell candidates, body `::block`
guards (`page_blocks::BodyGuards`, the one body-region helper; DMLS reuses
it), transclusion targets (`directive_targets::classify_target_nullability`
and `analyze_directive_targets`' `narrowed_paths` read the scope), and
`AbsenceScope`'s branch facts. `expression::branch_scope(expr, span, site)`
gives the scope at one node inside an expression (ternary/`&&`/`||` sides,
same routing as `walk_static`); DMLS hover uses it. Never add another guard walker.
`lib/tests/l1/type_scope_ownership_guard.rs` (and the DMLS twin) fails on an
unlisted `TypeScope::unguarded` site, a stale allowlist entry, or a retired
classifier; the site × guard-case matrix is `lib/tests/l1/scope_site_conformance.rs`
over `lib/tests/common/type_scope_cases.rs`. Equality follows the evaluator:
both use `expression/equality.rs::values_equal` (same runtime type and value,
exact numeric value across `i64`/`u64`/`f64`, recursive containers, a missing
key differs from a `null` entry). A missing operand is `null`, which equals
only `null`, so `x != null` proves presence and `x != ''` does not. Never
compare `Value`s with `==` or through rendered text for an expression
`==`/`!=`; ordering (`<`, `>=`, …) still coerces to numbers. User docs:
[What a Guard Proves](../../../darkmatter/docs/topics/darkmatter-expressions.md#what-a-guard-proves).

Numbers enter Darkmatter only through `markdown::numeric` (`admit_number_spelling`
for loader/lexer text, `admit_number_text` for the one lenient text parser,
`admit_json_number`/`admit_f64` for values without a spelling). A whole number
must be exact in every spelling, decimals round, underflow and non-finite are
errors. Expression literals, frontmatter YAML, raw schemas, and SimplifiedSchema
metadata are admitted at load; operators compute in `f64` with no integer
casts, and a non-finite result (overflow, `/ 0`, `% 0`) is an error. Store an
admitted number with `numeric::json_number` and render it with
`numeric::format_number`; never `as i64`. User docs:
[Numbers](../../../darkmatter/docs/topics/numbers.md).

Hosts add globals (Claudine's `err`, `timing`, `group`) through the binding
model in `expression/binding.rs`, never through a lookup override. A root
resolves as reserved namespace → registered global (available, possibly
`null`, or unavailable with a namespaced reason) → document property. Declare
globals in an immutable `BindingView`, pair them with runtime entries through
`EvaluationSession::associate` (rejects reserved names, omissions, and
contradictions before any provider runs), check authored text passively with
`prepare_value` + `validate_prepared` (every branch, no provider), and evaluate
with `evaluate_prepared` or `SubtreeCompose::with_binding_view`. A whole
composition takes globals through `ComposeOptions::with_host_globals`
(`HostGlobals::associate`): the frontmatter seed state and the body's
`ResolvingLookup` consult them after the reserved roots, a transcluded child
inherits them, a bare read of one is exempt from the deferred-key (DM1a) check,
and an unavailable read is a fatal transclusion failure
(`MarkdownError::unavailable_binding`). Claudine declares `loop` and the retired
`_loop_*` names this way. There is no
strict mode and no root-membership hook. See
[Host Bindings](../../../darkmatter/docs/topics/darkmatter-expressions.md#host-bindings).

Each issue is reported once. A coded `ComposeWarning` family declares its
identity in its constructor (`from_schema_advisory`,
`unknown_context_variable`, `expression_failure`). `add_warning` and
`merge` collapse warnings whose `(source, code, subject)` match, and never
compare messages. Schema advisories key on the referenced path. Root and
expression subjects are per source document:
`run_compose_pipeline_node` calls `attribute_to_document` before a child
report merges upward. Push new warnings through `add_warning`/`add_warnings`,
not `report.warnings.push`. Membership is O(1) through a private
`WarningIndex` cache: it indexes direct pushes lazily and rebuilds when the
vector shrinks, but an element replaced in place is not seen.
`interpolate_text` scans once, so a failed span
is reported once. A new coded family, such as
`dm.expression.undeclared_property`, adds a `WarningSubject` constructor
rather than a message-based check.
