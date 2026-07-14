# Expression-Function Catalog Drift Log

## Phase 3 transcription

The authored schema stores `description` once per function, but three existing
multi-overload registrations carry overload-specific descriptions:

- `frontmatter(file, prop)` describes reading a single property, while the
  function-level description describes reading the complete frontmatter object.
- `validate_schema(file, obj)` describes its forward-compatibility purpose,
  while the function-level description describes schema validation.
- `link(target, desc)` describes its explicit-description form, while the
  function-level description describes the single-argument form.

Phase 3 preserves each function's first-overload description, matching the
Phase 1 inventory and the schema shape mandated by the specification. No Rust
descriptor was corrected during transcription. Phase 4 descriptor projection
will necessarily give every overload the function-level description unless the
catalog AST is separately revised to support overload-specific descriptions.

`has_command` was the only registration without an example. The authored
catalog adds the specification-required display-only example with the reason
`result is host-dependent`; the Rust registration remains unchanged in Phase 3.

The existing public `catalog_order` range begins at `0`, despite the plan's
parser language calling `order` positive. Faithful transcription and the stated
ordering-compatibility requirement therefore require non-negative integers.
The parser now accepts `0`, rejects negative values, and the authored catalog
preserves all baseline order values exactly.
