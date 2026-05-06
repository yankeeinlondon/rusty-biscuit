---
blast_radius:
  - darkmatter/lib/src/markdown/normalize/mod.rs
  - darkmatter/lib/src/markdown/normalize/types.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
---

# Structural Normalization

During the **Structural Normalization** stage the post-transclusion document is normalized by validating and adjusting _heading_ levels. The first heading encountered establishes the document's **root level**; all subsequent headings are evaluated against that baseline.

## Rules

A well-formed document satisfies all of the following:

- **No hierarchy violations** — every heading after the first must be at the same level as, or deeper than, the root level. A heading that appears shallower than the root (e.g. an H2 in a document whose root is H3) is a `HierarchyViolation`.
- **No skipped levels** — heading depth must increase by at most one level at a time. An H2 followed directly by H4 (skipping H3) is a `SkippedLevel`.
- **At most one H1** — a document with more than one H1 heading is flagged as `MultipleH1`.
- **No level overflow** — after re-leveling, no heading may exceed H6. Attempting to re-level a deeply nested document to a shallow target that would push headings past H6 is a `LevelOverflow` error.

## What the Stage Does

When the compose pipeline invokes normalization (`normalize::normalize`), it passes `None` as the target level. This means:

1. The root level of the document is detected from the first heading.
2. The target level defaults to the existing root level — no uniform shift is applied.
3. If `level_adjustment` is zero, the content is returned unchanged.

The primary use of heading re-leveling occurs during **transclusion**, where a child document's headings are shifted so they nest correctly under the parent's preceding heading depth.

## Validation

Separate from the compose pipeline, `Markdown::validate_structure()` checks all four rules and returns a `StructureValidation` containing:

| Field | Description |
|---|---|
| `root_level` | Level of the first heading |
| `min_level` / `max_level` | Shallowest and deepest heading levels found |
| `heading_count` | Total headings in the document |
| `issues` | `Vec<StructureIssue>` — one entry per broken rule |

Each `StructureIssue` carries a `StructureIssueKind` (`HierarchyViolation`, `SkippedLevel`, `MultipleH1`, or `LevelOverflow`), the offending heading's title and line number, a human-readable description, and an optional suggested fix.

## Re-leveling API

Two methods are available for explicit level adjustment:

- **`normalize(target)`** — shifts all headings uniformly so the root matches the target level. Returns a `NormalizationReport` listing every `HeadingAdjustment` and any `ViolationCorrection`s. Fails with `NormalizationError::LevelOverflow` if any heading would exceed H6.
- **`relevel(target)`** — a convenience wrapper that returns the adjusted content and the numeric `level_adjustment`.

---

[**Return to Compose Pipeline**](../darkmatter-compose-pipeline.md)
