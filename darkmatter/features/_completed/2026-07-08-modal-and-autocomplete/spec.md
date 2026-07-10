---
status: ready for planning and implementation
reviewed: true
review_iterations: 4
depends_on:
  - ../_completed/2026-07-08-single-sourcing-schema/spec.md
inputs:
  - ../../dmls/src/providers/dsl.rs
  - ../../dmls/src/providers/frontmatter.rs
  - ../../dmls/src/overlay/expressions.rs
related:
  - ../_completed/2026-07-08-single-sourcing-schema/spec.md
---

# DMLS Interpolation Assistance (Hover + Completion)

**Status:** Reviewed and ready for planning and implementation. This is the
**editor-facing** consumer of the library work in
[single-sourcing-schema](../_completed/2026-07-08-single-sourcing-schema/spec.md).
It owns no library, schema, expression-evaluator, or context-capture change. It
consumes the single-sourced context-variable and expression-function catalogs to
improve DMLS hover and completion inside `{{ ... }}` interpolation.

> **Reader note — terminology:** the draft called hover information a “modal”
> and completion “autocomplete.” This spec uses the LSP capability names
> **hover** and **completion**. It does not add a modal UI owned by DMLS or the
> distinct LSP `textDocument/signatureHelp` capability.

## Dependency

This spec **must not land before** single-sourcing-schema. It relies on:

- `context_variable_descriptors()` returning YAML-sourced, corrected type and
  description data (for example, `ctx.now` is `datetime`, not `string`);
- `ContextVariableDescriptor::display_type` rendering the projected
  SimplifiedSchema type, including the `[]` suffix for arrays;
- `ExpressionFunctionDescriptor::typed_signature()` rendering parameter and
  return types, including `| error` for fallible functions;
- the six list-formatting functions (`as_csv`, `as_tsv`,
  `as_space_separated`, `as_line_separated`, `as_unordered_list`, and
  `as_ordered_list`) being registered in the expression-function catalog; and
- list-typed `ctx.*` variables being real arrays (for example,
  `ctx.packages: string[]`, with the `*_list` twins removed).

If this work starts first, it would re-encode the same semantic data DMLS is
trying to stop duplicating. Ordering is therefore a hard constraint, not a
preference.

## Motivating defect

Today the `{{ ctx.today }}` hover from `dmls::providers::dsl` shows only:

```text
Expression
ctx.today
The `ctx` variable is evaluated at _compose_ time (rather than now).
```

It omits the type and description already shown by the frontmatter provider's
`ctx_hover`. Interpolation completion also puts the description in `detail`,
has no `documentation`, and discards the variable type. Function completion
uses the catalog's untyped display signature even though the dependency adds a
typed signature authority.

The two `ctx.*` hover surfaces should present the same catalog facts, and
completion should populate the LSP fields intended for type and documentation
metadata.

## Design decisions

### D1 — Keep one semantic adapter in DMLS

`dmls::overlay::expressions` remains the passive adapter over the Darkmatter
catalogs, but it must stop projecting descriptors into `(name, description)`
and `(signature, description)` tuples. Those tuple projections lose the type
information this feature needs.

The adapter exposes borrowed `ContextVariableDescriptor` and
`ExpressionFunctionDescriptor` values, or lookup functions returning those
descriptors. Both `providers::dsl` and `providers::frontmatter` consume that
adapter for `ctx.*` assistance. DMLS must not introduce a parallel type,
description, signature, fallibility, or list-function table.

The shared adapter also owns the small Markdown formatter for the catalog-backed
portion of a `ctx.*` hover. This makes the qualified name, type, ownership note,
and description identical between frontmatter and interpolation hover. Each
provider may append surface-specific text after that shared portion.

### D2 — Enrich interpolation `ctx.*` hover

When the parsed interpolation expression is a `ctx.<name>` variable, or has
that variable as its root through member/index access, hover shows:

```text
Expression
ctx.today (date) — read-only, Darkmatter-owned

Local date in ISO-8601 format.

The `ctx` variable is evaluated at _compose_ time (rather than now).
```

The type and description come from `context_variable_descriptors()` through
the D1 adapter. The final compose-time note remains DMLS-owned because it
describes passive editor behavior rather than the variable itself. DMLS must
not capture host context or evaluate the expression to build hover content.

Classification requires an explicit `ctx.` prefix. A bare interpolation such
as `{{ today }}` is a frontmatter variable, even when `today` is also a known
context-variable tail. Unknown `ctx.<name>` values retain the generic
expression hover and must not borrow metadata from a similarly named bare key.

The hover range may remain the complete `{{ ... }}` expression, matching the
existing provider contract; this feature does not require changing the public
span-bearing expression AST.

### D3 — Put completion metadata in the correct LSP fields

For each matching `ctx.<name>` completion item:

- `label` and inserted text are the fully qualified `ctx.<name>`;
- `kind` remains `VARIABLE`;
- `detail` is the descriptor's rendered `display_type` (for example,
  `string[]`);
- `documentation` is eager Markdown documentation containing the descriptor's
  description; and
- `textEdit` eagerly replaces the current interpolation token, preserving the
  existing Zed-safe no-snippet behavior.

Documentation must be present on the initial completion response. DMLS does
not advertise `completionItem/resolve`, so deferring it would make the feature
client-dependent.

Completion matching remains prefix-based and case-sensitive. `{{ ctx.pa }}`
offers matching `ctx.*` variables; it does not offer removed `*_list` aliases.
Existing top-level-frontmatter and expression-function candidates remain
available in their current contexts.

To make completion after `ctx.` automatic rather than manual-invocation-only,
the server advertises `.` in `CompletionOptions::trigger_characters`, alongside
the existing triggers. The completion provider still verifies that the cursor
is inside an open interpolation, so a period in ordinary prose produces no DSL
items.

### D4 — Use typed function descriptors for completion

All expression-function completion items use the shared function descriptor:

- the existing untyped signature remains the label (for example,
  `as_csv(list)`);
- insertion remains the bare function name, with no snippet or synthesized
  parentheses;
- `detail` is `ExpressionFunctionDescriptor::typed_signature()`; and
- `documentation` is the descriptor description as eager Markdown.

Consequently, the six formatting functions appear automatically when the
dependency registers them. For example, `as_csv` has detail equivalent to:

```text
as_csv(list: any[]) -> string | error
```

The spec deliberately uses `typed_signature()` rather than reconstructing
parameter or return types in DMLS. This preserves catalog fallibility and future
signature changes without another editor-side migration.

### D5 — Add catalog-backed function-call hover in v1

Function-call hover is required, not optional. When the cursor is on a known
function identifier inside `{{ ... }}`, DMLS shows the descriptor's typed
signature and description. This applies consistently to every catalog function,
not only the six new formatting functions; a catalog-driven implementation has
no sound reason to maintain an editor-only allowlist.

This remains ordinary `textDocument/hover`. Adding LSP signature help, active
parameter tracking, snippet insertion, or evaluation is out of scope. Unknown
functions retain the generic parsed-expression hover.

### D6 — Do not add an array-rendering hint in v1

Array completion documentation stays equal to the catalog description. DMLS
does not append a second statement about line-separated default rendering or a
hard-coded list of formatting functions. Such a note would duplicate evaluator
semantics in the editor layer and could drift independently. The typed
`string[]` detail and the catalog-backed function candidates provide discovery
without creating a new semantic authority.

If the library later adds catalog metadata for preferred conversions or default
rendering, DMLS may surface that metadata without changing this decision.

## Concurrent DMLS changes to preserve

These changes landed on this feature's shared files during the review cycle and
**must not be regressed** by the implementation. Only the first (the compose-time
note wording) is normative to this feature; the rest are recorded for
coordination because they touch the same frontmatter/DSL surface.

- **Compose-time note wording (normative, D2).** The passive note wording was
  updated to *"The `ctx` variable is evaluated at _compose_ time (rather than
  now)."* — see the D2 hover block above and the source string in
  `providers::dsl`. The retired form was *"Resolved from ctx.* at compose time
  (not evaluated here)."*; `providers::dsl` unit tests assert the new wording, so
  implementations must match it exactly (note the italic `_compose_`).
- **Frontmatter schema-property hover styling** (`providers::frontmatter::schema_hover_body`
  — same file, out of this feature's scope): the property name renders as an
  inline-code box, the type as **bold** (not inline code), and enum/default
  values as _italic_. The rule and its rationale (LSP-Markdown cannot express
  color or dim) are in `dmls/docs/hover.md`. The D1 shared-formatter refactor
  must not revert this schema-hover styling.
- **Frontmatter diagnostics de-noising** (`diagnostics::frontmatter` — a separate
  file, unrelated to hover/completion): the `dm.schema.pending_shell_value`
  diagnostic was removed (deferred `{{ }}` / `$(...)` values are never diagnosed
  at edit time), and `dm.schema.missing_required` is suppressed outside strict
  mode (`required` is a compose-time contract). Noted so a broad frontmatter
  refactor does not reintroduce the removed edit-time noise.

## Out of scope

- Changes to the context-variable catalog, schema, capture logic, expression
  evaluator, or formatting functions; those belong to single-sourcing-schema.
- Frontmatter completion or schema hover beyond routing existing `ctx.*` hover
  through the shared D1 adapter.
- Runtime context capture, expression evaluation, filesystem access, shell
  execution, or network access while answering hover/completion.
- LSP `textDocument/signatureHelp`, completion resolve, snippets, active
  parameter tracking, and automatic insertion of parentheses or arguments.
- Compatibility completions for the removed `ctx.*_list` variables.

## Testing requirements

### Unit/provider tests

- The descriptor adapter returns catalog descriptors and preserves rendered
  array types and typed function signatures.
- Interpolation and frontmatter `ctx.*` hover share the same catalog-backed
  name/type/ownership/description block.
- Interpolation hover appends the passive compose-time note and never evaluates
  `ctx.*`.
- A bare key whose name matches a `ctx.*` tail is treated as frontmatter, not as
  generated context.
- `ctx.*` completion sets `detail`, eager Markdown `documentation`, and the
  eager token-replacing `textEdit` from the catalog descriptor.
- Function completion sets the untyped label, typed-signature `detail`, eager
  documentation, and bare-name insertion from one descriptor.
- All six formatting functions are present; at least one fallible typed
  signature asserts the `| error` suffix.
- Function hover covers a formatting function, a pre-existing function, and an
  unknown function.
- Completion after a period outside interpolation returns no DSL candidates.

### Capability and L2 tests

- The initialize response advertises `.` as a completion trigger without
  dropping `/`, `(`, or `#`.
- An in-memory LSP session verifies hover and completion response shapes,
  including `detail`, `documentation`, ranges, and edits. Include an astral
  Unicode character before the interpolation so the negotiated UTF-16 path is
  exercised.
- The existing no-side-effects test continues to prove that editor assistance
  does not execute directives, expressions, or commands.

Use the package area's canonical `just test`, `just test-l2`, and `just lint`
recipes. The implementation must remain portable across macOS, Windows, and
Linux and must not add platform-specific path or terminal behavior.

## Acceptance criteria

1. `{{ ctx.<name> }}` hover shows the catalog-derived type and description and
   the passive compose-time note; its catalog-backed block matches frontmatter
   `ctx_hover` for the same variable.
2. Only explicitly qualified `ctx.*` expressions receive context-variable
   metadata; same-named bare frontmatter variables do not.
3. `ctx.*` completion exposes catalog-derived type in `detail`, description in
   eager Markdown `documentation`, and a correct eager `textEdit`.
4. `.` is an advertised completion trigger, and triggering it outside an open
   interpolation produces no DSL completion items.
5. Every expression-function completion derives its label, typed detail,
   documentation, and insertion from one library descriptor. The six formatting
   functions are present with their catalog-defined fallibility.
6. Hovering a known function identifier shows its typed signature and
   description; unknown functions keep the generic expression hover.
7. No DMLS-side re-declaration of context-variable or function semantic data is
   introduced, and no editor request evaluates or executes document content.
8. DMLS L1, L2, lint, and no-side-effects suites pass, and the change remains
   compatible with macOS, Windows, and Linux.

## Open questions

None. Function hover is included in v1 (D5), and editor-authored array-rendering
hints are excluded until the library catalog can supply that metadata (D6).
