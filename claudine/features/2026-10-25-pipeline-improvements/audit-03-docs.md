---
kind: implementation-log
---

# Documentation audit: 2026-10-13-named-type-presence-and-local-names

Updated **5 existing canonical documents**; created **0 documents**. The
completed spec now lists seven canonical documents with implicit
repository-relative paths. Two referenced pages were already current and
already had the requested kind from earlier audit work; they needed no edit
in this step.

| Canonical document | Requirements covered | Change in this step |
| --- | --- | --- |
| `darkmatter/docs/topics/schemas/local-type-references.md` | Local namespaces, built-in collisions, forward references, defining-file lookup, inherited presence and constraints, serialization | Added canonical kind, a complete description-precedence example, and a concise guide to named-type editor assistance. |
| `darkmatter/docs/topics/schemas/authoring-schemas.md` | Schema envelopes, reusable types, optional outer objects versus required fields, generated validation phases | Set canonical kind; corrected the runtime-presence paragraph to distinguish launch from completion. |
| `darkmatter/docs/topics/schemas/definition.md` | Full constraint merge rules, target-directed bounds, separate item/container constraints, origins, conjunctive patterns, grouped file matches, public APIs | Reviewed; already current and canonical. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Bare-name completion, hover, definition navigation, missing-name/conflict diagnostics and related locations, dependency refresh | Reviewed; already current and canonical. |
| `darkmatter/docs/inline/schema-validation.md` | Normal validation and root-union matching through every inherited match group | Added canonical kind. |
| `claudine/docs/topics/composition.md` | Grouped file-completion metadata and alternatives across root-union arms | Added canonical kind. |
| `claudine/docs/topics/completions/shell-completions.md` | Candidate intersection, group-local exclusions, and empty intersections | Added canonical kind; removed an archived-spec directory reference and replaced a dated spec example with a generic path. |

No new canonical page was needed: the local-reference guide is the obvious
entry point, and it links into the authoring, reference, and editor pages.
Nullable and recursive behavior introduced later in the pipeline remains
documented as implemented rather than being reverted to the earlier
specification's planned state.

## Validation

Checked the current resolver's `apply_import_postfix` description precedence,
constraint inheritance, and item/container separation; the serializer's local
reference spelling; and `phase.rs::project_atom`'s launch/completion presence
rules. All seven canonical references exist under a package area's `docs/`
folder and declare `kind: canonical-documentation`. The specification's body
is byte-for-byte unchanged from HEAD. New cross-reference anchors exist, and
`git diff --check` passed. Test execution belongs to the concurrent test agent;
this documentation step ran no test suites.
