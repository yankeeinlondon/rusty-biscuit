---
kind: implementation-log
---

# Documentation audit: 2026-10-14-union-type-constraints

Updated **3 existing canonical documents**; created **0 documents**. The
completed specification now lists four canonical documents with implicit
repository-relative references. The fourth page was already current and had
the requested kind, so it needed no edit in this step.

| Canonical document | Requirements covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/definition.md` | Accepted and rejected reference modifiers; item versus list constraints; presence and timing; default inheritance; JSON Schema lowering; representation and serialization; Content Policy example; validation, conversion, rewriting, cleanup, completion, and error APIs | Corrected stale anonymous-union limitation, distinguished scalar defaults from programmatic array defaults, and clarified explicit null arms versus automatic optional-property wrappers. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Typed schema errors and definition/use-site related locations; named types; completion, hover, and passive editor behavior | Reviewed unchanged; already current and canonical. |
| `darkmatter/dmls/docs/autocomplete.md` | Keys and scalar values for union-list items, per-item literal-tag narrowing, authored-key filtering | Added canonical kind and made the union-item example self-contained by declaring its two alternatives and list type. |
| `darkmatter/dmls/docs/hover.md` | Whole-union descriptions, uniquely matching item alternatives, ambiguous and missing item display, nested-field hover | Added canonical kind. |

The schema definition page remains the clear entry point. Its named-union
section describes the authoring forms and examples, links to the broader
union grammar, and separates consumer safety rules with a diagram and table.
No new page is necessary.

## Drift corrected

The same reference page both documented grouped `(A | B)[]` arrays and claimed
that spelling was unsupported. The latter statement was left over from the
earlier snapshot and now points to the implemented grouped syntax. The
original spec body remains unchanged.

The default rejection table previously said any arm default under `[]` was
rejected. The resolver rejects scalar defaults but accepts actual JSON arrays
supplied through the Rust model, subject to equal-value merging. The docs now
state that distinction without claiming the authoring grammar supports an
array-valued `default(...)` argument.

The lowering explanation previously said list items get no null arm. They
get no *automatic* null wrapper, but an explicit `null` or `unknown` arm still
admits null. The required-union explanation now also preserves the existing
null-admitting presence exception.

## Validation

Compared the rules with `resolve.rs::apply_union_postfix`, including
`ScalarDefault`, `ArrayArm`, and constraint classification; the current
conversion rules; and the existing editor completion and hover documentation.
All four references exist under a package area's `docs/` directory and have
`kind: canonical-documentation`. A comparison against HEAD verifies the
specification body is byte-for-byte unchanged. `git diff --check` passed.
No feature/fix references were added to canonical docs. No test suite was run
by this documentation agent; the concurrent test agent owns verification.

Related stale wording in the Darkmatter skill was reported to the test agent,
which owns code/workflow updates for this step.
