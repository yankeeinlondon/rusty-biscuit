---
kind: implementation-log
---

# Documentation audit: 2026-10-26-task-inputs-and-providers

Updated **4 existing canonical documents**; created **0 documents**. The
completed specification now lists eight canonical documents using implicit
repository-relative paths. All eight declare
`kind: canonical-documentation`; the specification body is unchanged.

| Canonical document | Functionality covered | Change in this step |
| --- | --- | --- |
| `claudine/docs/cli/sequence.md` | Per-task prediction, precedence, missing versus late hints, installed-provider failures, review rows, actual launch reporting, models and captured environments | Made provider-specific model environment precedence over generic `MODEL` explicit; distinguished compatible inherited/review-selected models and per-document environment capture. Existing flow diagram and runnable mixed-provider examples are current. |
| `claudine/docs/topics/flow-control/sequences.md` | Task `with`, freed `params` data, input precedence, supported task kinds, mapping errors, lexical containers and static preflight | Added an approachable atomic-evaluation failure example, data-versus-template explanation, and a direct link to the complete file-provenance contract. |
| `claudine/docs/topics/composition.md` | Caller and `with` provenance, eager/lazy/remote files, union-arm selection, raw-data retention and fresh-read rematerialization | Corrected stale sequence-model retention wording to match the tested fresh-read repair: a removed document-owned model leaves prompt, argv and child environment together; explicit/captured choices and genuine compatible fallbacks retain their ownership. |
| `claudine/docs/topics/flow-control/flow-control-reference.md` | Shared task/directive `with` shapes, error priority, typed values, source-relative files, atomic evaluation, overlay precedence and lifetime | Reviewed current; already canonical. |
| `claudine/docs/topics/schemas.md` | Sequence trigger selection, embedded and disk bundles, application-data boundary, parser-only rules, recursively typed containers and event restrictions | Reviewed current; already canonical. Later recursive coverage supersedes the snapshot's interim bounded expansion. |
| `claudine/docs/topics/frontmatter-properties.md` | Provider/model catalog, sequence trigger ownership and generated runtime roots | Reviewed current; already canonical. |
| `darkmatter/docs/topics/schemas/dmls-schema-support.md` | Editor projection of schema data, trigger discovery, completion/hover/diagnostics and compiled-host versus disk-editor delivery | Reviewed current; already canonical. |
| `biscuit-tui/docs/components/input_table.md` | Read-only `CellValue::StaticText` in an otherwise editable column, skipped focus, and typed extraction | Added canonical kind. The existing API example demonstrates editable and read-only provider rows and already describes their behavior. |

No additional canonical page was needed. The sequence command page is the
obvious provider-selection entry point; the sequence guide owns task inputs,
and both lead to the common provenance and schema references. Existing docs
correctly preserve integrated null/empty-hint distinctions, recursive schemas,
and parser-only validation boundaries instead of reinstating the earlier
snapshot's interim behavior. No historical feature/fix references were added.

## Validation

Compared the text with provider prediction and selection, the shared model
resolver, launch rebuilding, task input layering, and InputTable's static-cell
state. Checked all eight repository-relative canonical references and kinds,
the new relative documentation link, and the specification body against HEAD.
`git diff --check` passed. This documentation-only slice ran no tests; the
concurrent test agent owns functional verification. No skill change was needed:
the Claudine skill already describes per-task selection and shared provenance.
The test agent confirmed 130 focused tests passed after the fresh-read model
repair, including removal before retry and before the first task launch, with
controls for task `with`, captured `MODEL`, and genuinely inherited fallback.
The refreshed model prose was checked with that agent before finalization.
