---
kind: implementation-log
---

# Documentation Audit: Schema Enhancements

Updated **6 existing canonical documents**; created **0 documents**. The
completed specification now lists ten canonical documents with implicit
repository-relative paths. Its snapshot body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/definition.md` | Tuples, function signatures, union syntax, categories, shared conversion rules, overloads, containers, raw schema conversion, and errors | Reviewed; already complete and canonical. |
| `darkmatter/docs/topics/schemas/authoring-schemas.md` | Authoring schema envelopes, unions, and category metadata | Reviewed; already current and canonical. |
| `darkmatter/docs/topics/numbers.md` | Exact whole-number admission, fractional rounding, underflow, finite values, ingress, arithmetic, and presentation | Added canonical kind; explained `f64` as a 64-bit floating-point representation. |
| `darkmatter/docs/topics/darkmatter-expressions.md` | Expression semantics and the catalog-generated function reference, including categories and examples | Reviewed; already current and canonical. The generated table was preserved. |
| `darkmatter/docs/inline/call-check.md` | Five argument categories, passive rejection, dead branches, declared-type projection, effects boundary, and runtime limits | Added canonical kind and a descriptive title; corrected stale function-result/fallback typing and qualified default rejection behavior with blocking policy. |
| `darkmatter/docs/darkmatter-compose-pipeline.md` | Placement of passive checking before evaluation and authored-text boundaries | Added canonical kind. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Schema authoring diagnostics, completion, and hover | Reviewed; already current and canonical. |
| `darkmatter/dmls/docs/diagnostics.md` | Call codes, severity categories, strict warnings, remedies, and source locations | Removed a stale claim that a function result is unknown in strict mode; the outer call uses the catalog's declared return type and inner calls remain independently checked. |
| `darkmatter/docs/lsp/configuration.md` | File/editor configuration, `expressions.strict`, precedence, invalid settings, and reload | Added canonical kind. |
| `claudine/docs/topics/state-management/expression-engine.md` | Claudine's use of the shared binder/checker/catalog and expression-reference generation | Added canonical kind. |

No new page was needed. The schema definition, numeric guide, and call-check
guide already provide clear entry points with examples and diagrams; the
remaining pages explain their editor and CLI integration. Documentation follows
the final integrated implementation, including later type projection and
blocking policy, rather than restoring earlier limitations.

## Validation

Compared the grammar/conversion and documentation requirements with the final
implementation log and its acceptance-criterion inventory. Checked call-result
and fallback projection against `call_check::expression_type` and
`call_returns`; the latter reads declared return types by argument count and
does not evaluate a call. Reviewed the category/remedy and strict-setting
documentation together to remove their contradictory result-type statements.

All ten references exist beneath package-area `docs/` folders and declare
`kind: canonical-documentation`. New relative policy and diagnostic links
resolve. The completed specification body matches HEAD byte for byte. No
feature/fix links or names were added to canonical documents. `git diff --check`
passes. No tests or generated function-reference regeneration were needed for
these prose/frontmatter changes; the concurrent test agent owns the functional
verification.
