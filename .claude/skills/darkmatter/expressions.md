# Expression Evaluation Rules

Detail behind the "Testing and verification" section of [SKILL.md](SKILL.md): the expression grammar gate, inserted-text rules, failure policy, unknown identifiers, and warning identity.

The expression-grammar corpus gate is
`lib/tests/l1/dasherized_identifier_corpus.rs`. It walks root `prompts/`,
`.claude/commands/`, `darkmatter/prompts/`, and `claudine/prompts/` through
the library's own extractors: `ExpressionFinder`,
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
only for `compose_subtree(..., Lenient)` and preflight discovery (see
[compose.md](compose.md#error-handling)). The error carries the authored
span (`SourceRef::OnDiskSpan`) whenever it is provable, including after an
earlier stage rewrote the body and inside block (`|`, `>`), multi-line,
tagged, anchored, or aliased frontmatter scalars. `interpolation_block` renders the file and
authored line and column for every cause, and
`interpolation/fatality_characterization.rs` is the drift guard.

A well-formed identifier that resolves to nothing warns as
`dm.expression.unknown_identifier` (once per root per document) unless the
author handled its absence (`x || d`, `x ? …`, `is_null(x)`) or the root is
known to the final state, a caller input, or the effective schema. Read
[compose.md](compose.md#unknown-identifiers-dmexpressionunknown_identifier)
before changing a runtime evaluation surface: new surfaces must observe
through the shared `AbsenceScope`, not a new walk. DMLS reports the same code
at `WARNING` through the static twin, `expression::static_variable_reads`
(see [dmls.md](dmls.md#unknown-identifiers)).

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
`dm.expression.unknown_identifier`, adds a `WarningSubject` constructor
rather than a message-based check.
