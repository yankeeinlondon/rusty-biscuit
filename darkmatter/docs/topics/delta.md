# The **delta** functionality in Darkmatter

## Overview

Delta compares two parsed `Markdown` documents and produces a `MarkdownDelta` — a structured analysis of every change between them. Rather than line-level diffs, it operates at the semantic level: frontmatter keys, preamble text, heading-delimited sections, fenced code blocks, and internal anchor links are all first-class comparison units.

The public entry point is `Markdown::delta(&self, other: &Markdown) -> MarkdownDelta`, which delegates to `delta::compute_delta()`.

## Core Types

### `MarkdownDelta`

The top-level result struct containing the full analysis:

| Field                         | Type                     | Description                                          |
|-------------------------------|--------------------------|------------------------------------------------------|
| `classification`              | `DocumentChange`         | High-level change category (see Classification)      |
| `statistics`                  | `DeltaStatistics`        | Quantitative metrics                                 |
| `frontmatter_changed`         | `bool`                   | Whether any frontmatter key differs                  |
| `frontmatter_formatting_only` | `bool`                   | Changed YAML formatting but not parsed values        |
| `frontmatter_changes`         | `Vec<FrontmatterChange>` | Per-key change records                               |
| `preamble_changed`            | `bool`                   | Whether content before the first heading differs     |
| `preamble_whitespace_only`    | `bool`                   | Preamble changed but only whitespace                 |
| `added`                       | `Vec<ContentChange>`     | Sections in updated but not original                 |
| `removed`                     | `Vec<ContentChange>`     | Sections in original but not updated                 |
| `modified`                    | `Vec<ContentChange>`     | Sections in both but with different content           |
| `moved`                       | `Vec<MovedSection>`      | Same content, different position in document          |
| `code_block_changes`          | `Vec<CodeBlockChange>`   | Fenced code block differences                        |
| `broken_links`                | `Vec<BrokenLink>`        | Internal links in updated doc that target missing slugs |

### `SectionId`

Uniquely identifies a section in the document hierarchy:

- `path: SectionPath` — heading titles from root to the node (e.g., `["Getting Started", "Prerequisites"]`)
- `content_hash: u64` — xxHash of the section's own content (for disambiguation)
- `occurrence: usize` — index when multiple sections share an identical path

### `ContentChange`

Describes an addition, removal, or modification:

- `path: Vec<String>` — section heading path
- `line_number: usize` — where the change appears
- `description: String` — human-readable summary (e.g., "+5 content lines", "-3 chars")
- `action: ChangeAction` — one of `ContentModified`, `WhitespaceOnly`, `Added`, `Removed`

### `MovedSection`

Records a section that exists in both documents with identical content but at a different location:

- `original_path` / `new_path` — heading paths before and after
- `level_delta: i32` — heading level change (negative = promoted, positive = demoted, zero = reordered)
- `original_sibling_index` / `new_sibling_index` — position among siblings

### `FrontmatterChange`

A single frontmatter key difference:

- `action` — `PropertyAdded`, `PropertyRemoved`, or `PropertyUpdated`
- `key: String`
- `original_value` / `new_value` — `Option<serde_json::Value>`
- `description: String`

### `CodeBlockChange`

A fenced code block difference:

- `language: String` — info string language tag
- `section_path: Vec<String>` — parent section heading path
- `line_number: usize`
- `description: String` — e.g., "Language: rust → python", or "Modified bash code block"
- `action` — `ContentModified` or `Renamed` (language/metadata only)
- `bytes_changed: usize` — Levenshtein edit distance

### `BrokenLink`

An internal link in the updated document whose target slug no longer exists:

- `target_slug: String`
- `link_text: String`
- `line_number: usize`
- `suggested_replacement: Option<String>` — fuzzy-matched alternative (>50% Levenshtein similarity)
- `suggestion_confidence: Option<f32>` — 0.0–1.0

## Classification System

`DocumentChange` categorizes the overall magnitude of change. Classification is determined from `content_change_ratio` with checks evaluated in priority order:

| Priority | Variant                    | Condition                                                        |
|----------|----------------------------|------------------------------------------------------------------|
| 1        | `WhitespaceOnly`           | Changes detected but all are whitespace-only                     |
| 2        | `NoChange`                 | No differences at all (all hashes match)                         |
| 3        | `FrontmatterOnly`          | Only frontmatter changed, body untouched                         |
| 4        | `FrontmatterAndWhitespace` | Frontmatter + whitespace only, no substantive body changes       |
| 5        | `StructuralOnly`           | Sections moved/reordered, content unchanged                      |
| 6        | `ContentMinor`             | `content_change_ratio` < 0.10                                    |
| 7        | `ContentModerate`          | `content_change_ratio` 0.10–0.40                                 |
| 8        | `ContentMajor`             | `content_change_ratio` 0.40–0.80                                 |
| 9        | `Rewritten`                | `content_change_ratio` > 0.80                                    |

The ratio is calculated as:

```
content_change_ratio = (bytes_added + bytes_removed + bytes_modified) / max(original_bytes, new_bytes)
```

Capped at 1.0.

## Comparison Pipeline

`compute_delta()` follows this sequence:

```
1. Extract TOC from both documents
2. Compare frontmatter
3. Compare preamble
4. Compare sections  (4-pass algorithm)
5. Compare code blocks
6. Detect broken links
7. Calculate statistics
8. Classify overall change
```

### Frontmatter Comparison

Uses set operations on the key sets of both documents:

- Keys in updated but not original → `PropertyAdded`
- Keys in original but not updated → `PropertyRemoved`
- Keys in both with different values → `PropertyUpdated`

A **formatting-only** flag is set when the normalized frontmatter hashes match — meaning the parsed values are identical but YAML formatting (indentation, quoting style) differs.

### Preamble Comparison

The preamble (content before the first heading) is compared using two hash layers:

1. `preamble_hash` — detects any change
2. `preamble_hash_trimmed` — detects content changes only

If the raw hash differs but the trimmed hash matches, the change is whitespace-only.

### Section Comparison (4-Pass Algorithm)

This is the core of the delta engine. Sections from both documents are matched across four passes, each consuming unmatched sections from a working set:

**Pass 1 — Exact matches.** Sections with identical path AND identical content hash (`prelude_hash()`) are marked unchanged and removed from both working sets.

**Pass 2 — Content matches at different paths (moves).** Remaining sections are organized by content hash into a `HashMap<u64, Vec<...>>`. When the same content hash appears in both documents at different paths, it's recorded as a `MovedSection` with:

- `level_delta` calculated as `new_level - original_level`
- Negative = promoted (e.g., H3 → H2), positive = demoted, zero = reordered at same depth

**Pass 3 — Path matches with different content (modifications).** Remaining sections sharing the same heading path but differing content are compared in detail:

- **Semantic hash variants** (`LeadingWhitespace`, `TrailingWhitespace`, `BlankLine`) are tested to determine if the change is whitespace-only
- **Levenshtein distance** is computed between the original and updated content for byte-level change metrics
- A description is generated: "+5 content lines", "-3 chars", or "text edited" depending on what changed

**Pass 4 & 5 — Unmatched leftovers.** Remaining original sections → `removed`. Remaining updated sections → `added`.

### Code Block Comparison

Code blocks are matched within their parent section by position:

1. **Group** blocks by `parent_section_path`
2. **Match** blocks in order within each group
3. **Compare** each pair on two axes:
   - **Info string** (language + metadata) — a change here with no content change produces a `Renamed` action
   - **Content hash** (trimmed) — a change here produces `ContentModified`
4. **Parse** info strings to report specific property changes (e.g., "Language: rust → python")
5. **Track** unmatched blocks as added or removed

Code fence lines (`` ``` ``) are stripped before content comparison to avoid double-counting language tag changes in the content diff.

### Broken Link Detection

All internal links in the updated document are validated against the updated TOC's `slug_index`:

1. For each internal link, check if `target_slug` exists in the slug index
2. For broken links, compute Levenshtein similarity against all valid slugs
3. Suggestions with >50% similarity are attached with a confidence score

## Hashing Strategy (biscuit-hash Integration)

Delta's ability to cheaply distinguish "unchanged", "whitespace-only", and "content-modified" without full-text comparison on every pair depends on a multi-tier hashing strategy powered by `biscuit-hash`.

### biscuit-hash xxHash API

The `biscuit-hash` crate exposes three xxHash functions:

| Function            | Signature                                            | Behavior                                       |
|---------------------|------------------------------------------------------|-------------------------------------------------|
| `xx_hash`           | `(data: &str) -> u64`                                | Raw XXH64, no preprocessing                    |
| `xx_hash_bytes`     | `(data: &[u8]) -> u64`                               | Raw XXH64 on bytes                             |
| `xx_hash_variant`   | `(data: &str, variants: Vec<HashVariant>) -> u64`    | XXH64 after applying normalization transforms  |

### `HashVariant` Enum

Each variant strips or normalizes a specific aspect of the input before hashing. Variants are always applied in a **fixed canonical order** regardless of the order passed in the vector:

| Order | Variant               | Effect                                              |
|-------|-----------------------|-----------------------------------------------------|
| 1     | `BlockTrimming`       | `trim()` the entire block (leading + trailing)      |
| 2     | `BlankLine`           | Filter out empty lines                              |
| 3     | `LeadingWhitespace`   | `trim_start()` each line (strips indentation)       |
| 4     | `TrailingWhitespace`  | `trim_end()` each line                              |
| 5     | `InteriorWhitespace`  | Collapse runs of whitespace within each line        |
| 6     | `ReplacementMap(…)`   | Apply find/replace pairs                            |
| 7     | `DropChars(…)`        | Remove specific characters                          |

Variants are idempotent and composable. The fixed application order means `vec![BlankLine, LeadingWhitespace]` and `vec![LeadingWhitespace, BlankLine]` produce identical results.

### Three-Tier Hash Strategy

Every section prelude (`PreludeNode`) computes three hashes at TOC extraction time, before delta ever runs:

| Tier | Field                    | Variants Used                                                    | Detects                        |
|------|--------------------------|------------------------------------------------------------------|--------------------------------|
| 1    | `content_hash`           | `xx_hash` (none)                                                 | Any change at all              |
| 2    | `content_hash_trimmed`   | `BlockTrimming`                                                  | Changes after trimming edges   |
| 3    | `content_hash_normalized`| `LeadingWhitespace` + `TrailingWhitespace` + `BlankLine`         | Only substantive content changes |

The decision tree during comparison:

```
Tier 1 matches → Unchanged (nothing changed)
Tier 1 differs, Tier 3 matches → WhitespaceOnly (indentation, blank lines, trailing spaces)
Tier 1 differs, Tier 3 differs → ContentModified (real content change, compute Levenshtein)
```

### Hashes Computed Per Content Type

| Content Type         | Computed In                    | Raw Hash              | Trimmed Hash                | Normalized Hash                                          |
|----------------------|--------------------------------|-----------------------|-----------------------------|----------------------------------------------------------|
| Full page            | `MarkdownToc::new()`           | `page_hash`           | `page_hash_trimmed`         | —                                                        |
| Frontmatter          | `MarkdownToc::new()`           | `frontmatter_hash` (raw YAML) | —                  | `frontmatter_hash_normalized` (canonical JSON)           |
| Preamble             | `MarkdownToc::new()`           | `preamble_hash`       | `preamble_hash_trimmed`     | —                                                        |
| Section title        | `MarkdownTocNode::new()`       | `title_hash`          | `title_hash_trimmed`        | —                                                        |
| Section prelude      | `PreludeNode::new()`           | `content_hash`        | `content_hash_trimmed`      | `content_hash_normalized`                                |
| Code block           | `CodeBlockInfo::new()`         | `content_hash`        | `content_hash_trimmed`      | —                                                        |
| Subtree              | `compute_subtree_hash()`       | `subtree_hash`        | `subtree_hash_trimmed`      | —                                                        |

### How Delta Uses Each Hash

**Frontmatter** — raw YAML is hashed with `xx_hash` to detect any formatting change; a separate canonical JSON serialization is hashed to detect value changes. If the raw hashes differ but canonical hashes match, the change is formatting-only (key reordering, quoting style).

**Preamble** — `preamble_hash` (raw) detects any change. If it differs but `preamble_hash_trimmed` (BlockTrimming) matches, the change is whitespace-only at the block edges.

**Section matching (Pass 1 & 2)** — uses `prelude_hash()` (the raw tier-1 hash) for exact matching and move detection. An additional `!= 0` guard filters empty sections to prevent false move matches.

**Section modification detection (Pass 3)** — when two sections share a path but have different raw hashes, delta computes semantic hashes on-the-fly:

```rust
let semantic_variants = vec![
    HashVariant::LeadingWhitespace,
    HashVariant::TrailingWhitespace,
    HashVariant::BlankLine,
];
let orig_hash = xx_hash_variant(&strip_code_fences(orig_content), semantic_variants.clone());
let upd_hash = xx_hash_variant(&strip_code_fences(upd_content), semantic_variants);
let is_whitespace_only = orig_hash == upd_hash;
```

Code fences are stripped before hashing so that language tag changes (compared separately in code block comparison) don't inflate the semantic diff.

**Code blocks** — uses `content_hash_trimmed` (BlockTrimming) to compare content. This makes comparison insensitive to trailing newlines in code blocks while still catching any content change.

### Why This Matters

Without the multi-tier strategy, delta would need to either:

1. **Full-text compare every pair** — O(n*m) Levenshtein on every section pair, prohibitively expensive for large documents
2. **Single hash** — fast but unable to distinguish whitespace-only from substantive changes

The three-tier approach gives O(1) classification for the common cases (unchanged, whitespace-only) and reserves expensive Levenshtein computation only for sections confirmed to have real content changes. This is what enables delta to report "blank lines removed (no visual effect)" separately from "+5 content lines" without performance degradation.

## Statistics

`DeltaStatistics` provides quantitative metrics across several dimensions:

### Content Metrics

| Field                  | Description                                                             |
|------------------------|-------------------------------------------------------------------------|
| `original_bytes`       | Total bytes in original (excluding frontmatter)                         |
| `new_bytes`            | Total bytes in updated (excluding frontmatter)                          |
| `bytes_changed`        | Absolute difference between `original_bytes` and `new_bytes`            |
| `bytes_added`          | Bytes in newly added sections                                           |
| `bytes_removed`        | Bytes in removed sections                                               |
| `bytes_modified`       | Levenshtein edit distance within modified sections                      |
| `content_change_ratio` | `(added + removed + modified) / max(original, new)`, capped at 1.0     |

### Section Metrics

| Field                     | Description                                          |
|---------------------------|------------------------------------------------------|
| `original_section_count`  | Total headings in original                           |
| `new_section_count`       | Total headings in updated                            |
| `sections_added`          | Count of new sections                                |
| `sections_removed`        | Count of removed sections                            |
| `sections_modified`       | Count of content-modified sections                   |
| `sections_moved`          | Count of moved sections                              |
| `sections_unchanged`      | Sections with identical path and content              |
| `structural_changes`      | `sections_moved + moved.len()`                       |
| `content_only_changes`    | Count of `ContentModified` actions                   |
| `whitespace_only_changes` | Count of `WhitespaceOnly` actions                    |

### Code Block Metrics

| Field                          | Description                                     |
|--------------------------------|-------------------------------------------------|
| `code_blocks_added`            | New code blocks                                 |
| `code_blocks_removed`          | Removed code blocks                             |
| `code_blocks_modified`         | Content-changed code blocks                     |
| `code_blocks_language_changed` | Info string changed but content unchanged        |

### Link Metrics

| Field                | Description                        |
|----------------------|------------------------------------|
| `broken_links_count` | Internal links targeting missing slugs |

## Text Output Rendering

The CLI text report (`print_delta`) renders sections in this order:

| Order | Section              | Symbol       | Details                                                     |
|-------|----------------------|--------------|-------------------------------------------------------------|
| 1     | Classification       | varies       | Symbol + name + percentage (e.g., `△ Minor changes (8.2%)`) |
| 2     | Frontmatter changes  | `+` `-` `~`  | Per-key; notes "(formatting changes only)" when applicable  |
| 3     | Preamble changes     | —            | "whitespace changes only" or "modified"                     |
| 4     | Added sections       | `+`          | Path joined with ` > `                                      |
| 5     | Removed sections     | `-`          | Original path                                               |
| 6     | Modified sections    | —            | Content changes first, with description                     |
| 7     | Moved sections       | `↷`          | "from → to" with level delta (e.g., "(promoted by 1)")     |
| 8     | Code block changes   | —            | Language in inverse video, section path in bold              |
| 9     | Broken links         | `⚠`          | Target slug + line; "did you mean #slug?" when suggested    |
| 10    | Whitespace changes   | —            | Grouped last, italicized, with explanatory footer           |

### Verbose Mode

Adds after the main report:

- **Byte statistics** — `original → new (changed)` byte counts
- **Section statistics** — `original → new (unchanged)` section counts
- **Visual diffs** for changed frontmatter (YAML) and content body:
  - **Unified format** when terminal width ≤ 110 columns
  - **Side-by-side format** when terminal width > 110 columns
  - 3 context lines, ANSI colored (green = added, red = removed)

## JSON Output

All types derive `Serialize`. Passing `--json` emits the full `MarkdownDelta` as pretty-printed JSON with the same structure described above. Key top-level fields:

- `classification` — string variant of `DocumentChange`
- `statistics` — all metrics as a flat object
- `frontmatter_changes` — array with `action`, `key`, `original_value`, `new_value`
- `added`, `removed`, `modified` — arrays of `ContentChange`
- `moved` — array of `MovedSection`
- `code_block_changes` — array of `CodeBlockChange`
- `broken_links` — array of `BrokenLink`

## Key Design Decisions

- **Hash-based matching** — sections are identified by xxHash of their content, enabling accurate move detection without expensive full-text comparison.
- **Multi-hash layers** — each section carries raw, trimmed, and normalized hash variants to distinguish whitespace-only from substantive changes without full text comparison on every pair.
- **Levenshtein for metrics** — byte-level edit distance provides accurate change quantification and powers fuzzy slug matching for broken link suggestions.
- **Section-oriented over line-oriented** — grouping changes by heading-delimited sections produces more actionable reports for docs maintenance than raw line diffs.
- **Priority-ordered classification** — whitespace-only is checked before no-change to correctly classify documents that differ only in insignificant whitespace.
- **Whitespace changes last** — listed at the end of text output since they have no visual effect when rendered.
