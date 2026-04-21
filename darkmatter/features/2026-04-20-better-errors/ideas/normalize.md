# NormalizationError — Better Error Ideas

Source: `darkmatter/lib/src/markdown/normalize/types.rs:399`

Two variants: `LevelOverflow` and `ValidationFailed`.

Both errors originate in the **Inline Post** normalization stage, which adjusts heading levels after the compose pipeline has run. The errors are triggered by `normalize()` and `relevel()` in `darkmatter/lib/src/markdown/normalize/mod.rs`.

---

## `LevelOverflow`

### Current Message

```
Cannot re-level to {target}: would push {affected_count} heading(s) beyond H6
(deepest heading at "{deepest_title}" would become H{would_become})
```

The variant carries rich structured data — `target`, `affected_count`, `deepest_title`, `would_become` — but the flat `thiserror` format loses visual hierarchy and makes it harder to scan the actual constraint violation.

### Idea 1: Visual heading-depth diagram in the block body

Use a `StatusBlock` with `StatusState::Error` to render a heading-depth visualization that shows exactly which headings overflow and by how much.

**Title line** — `Status::from_prose` with bold red error name + bold title:

```
<b>NormalizationError:</b> <b>Heading level overflow when re-leveling to H5</b>
```

**Block body** — `StatusBlock::body` with a `Compose` containing prose that shows the heading map:

```
┃ Re-leveling from H4 → H5 would require pushing headings past the
┃ maximum depth (H6). The document's heading depth is 2 levels,
┃ leaving no room at the target root.
┃
┃   #### Level 4  (root)     →  H5  ✓
┃   ##### Level 5            →  H6  ✓
┃   ###### Level 6           →  H7  ✗ exceeds H6
┃
┃ 3 heading(s) affected
```

**Hint** — `StatusBlock::hint` with actionable advice:

```
<yellow>Hint:</yellow> Reduce the document's heading depth or choose a shallower target.
The deepest heading <b>"Level 6"</b> would need to become <red>H7</red>.
```

**Why this is better**: The visual diagram makes the overflow immediately obvious. Users can see exactly which headings are affected rather than parsing a sentence with interpolated numbers.

### Idea 2: Show the arithmetic in context with a code snippet

Instead of (or in addition to) the diagram, include a code-block style example showing the original heading range and what the target would require.

**Title line**:

```
<b>NormalizationError:</b> <b>Cannot fit 3 heading levels under H5</b>
```

**Block body** — `Prose` inside the block:

```
┃ The document spans H4–H6 (depth of 3 levels).
┃ Re-leveling the root to H5 requires depth 2 or less.
┃
┃ <dim>Available slots at H5 root:</dim>
┃   H5 → H6 → <red>H7 ✗</red>
┃        ↑     ↑ out of bounds
┃
┃ <dim>Maximum safe target:</dim> <green>H4</green> (root + 2 levels fits within H6)
```

**Hint**:

```
<yellow>Hint:</yellow> Use <b>--clean</b> without a target level to auto-detect,
or pass <b>--target H4</b> to stay within bounds.
```

**Why this is better**: The "available slots" visualization directly shows the constraint (`target + depth ≤ 6`) as a visual, and the hint offers an immediate fix (the max safe target level). The `StructureValidation::can_relevel_to()` method already computes this — the error variant could carry a `max_safe_target: HeadingLevel` field.

---

## `ValidationFailed`

### Current Message

```
Validation failed: {0}
```

A bare string wrapper. The variant currently has **no usages in the codebase** (it's defined but never constructed), so this is an opportunity to design its block-style output before any call sites exist.

### Idea 1: Structured validation issue listing

When a custom validation rule is violated, `ValidationFailed(String)` should carry enough information to render individual issues. Change the inner type from `String` to a structured list of `StructureIssue` items (already defined in `types.rs`), or at minimum format each issue on its own line in the block.

**Title line**:

```
<b>NormalizationError:</b> <b>Document structure validation failed</b>
```

**Block body** — render each `StructureIssue` as its own line with kind, location, and description:

```
┃ <red>hierarchy violation</red> at line 42
┃   Heading "API Reference" (H2) appears above the document root (H3)
┃   <dim>Suggestion: demote to H4 or promote earlier headings</dim>
┃
┃ <red>skipped level</red> at line 78
┃   H2 followed by H4 (skipping H3)
┃   <dim>Suggestion: add an intermediate H3 heading</dim>
┃
┃ <red>multiple H1 headings</red> at line 15
┃   Found 3 H1 headings; expected at most 1
┃   <dim>Suggestion: use H2+ for subsections</dim>
```

**Hint**:

```
<yellow>Hint:</yellow> Run <b>md --clean</b> to auto-fix structural issues, or
manually adjust heading levels to follow a consistent hierarchy.
```

**Why this is better**: The current `String` variant would force the caller to flatten all context into one message. By rendering each `StructureIssue` individually — using data already available from `StructureValidation` — the user sees exactly where each problem is and gets per-issue suggestions (the `StructureIssue::suggestion` field already exists).

### Idea 2: Severity-aware per-issue blocks

For documents with many issues, group them by `StructureIssueKind` and render the most severe first using a `Compose` of multiple `StatusBlock` components.

**Compose layout** (multiple blocks stacked):

```
<b>NormalizationError:</b> <b>3 structural issues found in doc.md</b>

┃ <red>hierarchy violation</red> — 1 issue
┃ Line 42: "API Reference" (H2) is shallower than root H3

┃ <orange-500>skipped level</orange-500> — 1 issue
┃ Line 78: H2 → H4 (missing H3)

┃ <orange-500>multiple H1</orange-500> — 1 issue
┃ Lines 1, 15, 92: 3 H1 headings found
```

**Hint** (shared):

```
<yellow>Hint:</yellow> Use <b>md delta --structure</b> to see a detailed
structural comparison, or <b>md --clean</b> to auto-normalize.
```

**Why this is better**: Grouping by kind lets the user assess the nature of problems at a glance. Using different colors per severity (red for hierarchy violations, orange for skipped levels / multiple H1) creates a visual priority order. The `Compose` component from biscuit-terminal makes it straightforward to stack multiple `StatusBlock` instances into a single error output.

---

## Summary

| Variant | Idea | Key biscuit-terminal components | New data to carry |
|---------|------|--------------------------------|-------------------|
| `LevelOverflow` | Heading-depth diagram | `StatusBlock` + `Compose` + `Prose` | (existing fields sufficient) |
| `LevelOverflow` | Slot arithmetic + max safe target | `StatusBlock` + `Prose` | `max_safe_target: HeadingLevel` |
| `ValidationFailed` | Per-issue structured listing | `StatusBlock` + `Prose` | `Vec<StructureIssue>` instead of `String` |
| `ValidationFailed` | Grouped severity blocks | `Compose` of multiple `StatusBlock` | `Vec<StructureIssue>` instead of `String` |
