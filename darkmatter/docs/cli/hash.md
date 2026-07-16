## Overview

The `hash` command computes deterministic, Markdown-aware xxHash64 values for a
document's frontmatter and body content.

A hash is split into structural *kinds* of increasing resolution. Beyond the
default report, the command can write the hash back into the document
(`--save`) or report how the document has drifted from a previously stored hash
(`--diff`). Single-file/stdin and aggregate directory hashing are both
supported.

## Reporting

### Usage

```bash
# Hash frontmatter + body (default `simple` kind)
md hash doc.md
md hash -

# Hash only body or only frontmatter
md hash doc.md --body          # shorthand for `--kind body`
md hash doc.md --frontmatter   # shorthand for `--kind fm` (alias: --fm)

# Force a specific kind
md hash doc.md --kind structured
md hash doc.md --kind detailed

# Strict mode (no whitespace normalization / key reordering)
md hash doc.md --strict

# Write the computed hash back into the document frontmatter
md hash doc.md --save

# Report how the document differs from its stored hash (exits 2 on difference)
md hash doc.md --diff

# Directory aggregate hashing (bare-hash only)
md hash docs/
md hash docs/ --body
```

### Arguments

- `[INPUT]`: file path, directory path, or `-` for stdin. If omitted, reads
  stdin when piped; otherwise errors.

### Options

- `--kind <fm|body|simple|structured|detailed>`: structural kind to compute.
  Conflicts with `--body` and `--frontmatter`.
- `--body`: shorthand for `--kind body` (body/prose hash only). Conflicts with
  `--frontmatter`.
- `--frontmatter` (alias `--fm`): shorthand for `--kind fm` (frontmatter hash
  only). Conflicts with `--body`.
- `--save`: write the computed hash back into the document frontmatter.
  Conflicts with `--diff`.
- `--diff`: report how the document differs from its stored hash. Conflicts
  with `--save`.
- `--strict`: disable normalization and hash raw serialized content.

### Hash Kinds

| Kind | Shape |
|------|-------|
| `fm` | single frontmatter hash |
| `body` | single body/prose hash |
| `simple` | `{fm}-{body}` (default) |
| `structured` | `{fm}-{fm_keys}-{body}-{body_structure}` |
| `detailed` | nested YAML object: `frontmatter`, nullable `preamble`, per-section tuples |

All flat hashes are 16-char lowercase hex values. `detailed` prints a nested
YAML object whose `sections` are `[level, "heading", content_hash]` tuples.

### Kind Selection

When no kind is forced (`--kind`/`--body`/`--frontmatter` omitted), the kind is
selected as:

1. forced kind, if a kind flag was passed;
2. otherwise the kind of the document's stored hash;
3. otherwise `simple`.

`simple < structured < detailed` form a resolution ordering; `fm` and `body`
are lower than `simple` and incomparable to each other. This ordering drives
`--save` upgrade/downgrade behavior.

### Environment Variables

- `HASH_PROPERTY`: name of the frontmatter property that stores the hash.
  Defaults to `hash`. Whitespace is trimmed; an empty value falls back to the
  default.
- `HASH_IGNORE_PROPERTIES`: CSV list of *extra* frontmatter properties to
  ignore when hashing. Entries are trimmed and empties dropped. The list is
  additive — it can never un-ignore the active hash property or `last_updated`,
  which are always ignored.

The active hash property and `last_updated` are always excluded from hash
computation, with exact key matching. Only the extra ignored properties are
recorded in the stored hash's `ignored` list (sorted; omitted entirely when
empty).

### Stored Hash Shapes

The stored `hash` property uses a shorthand string when — and only when — the
kind is `simple` with no extra ignored properties:

```yaml
hash: 2f1c9a0b4d3e5f60-8a7b6c5d4e3f2a10
```

Otherwise it is an object:

```yaml
hash:
  kind: structured
  value: "<fm>-<fm_keys>-<body>-<body_structure>"
  ignored: [draft, reviewer]   # extras only, sorted; omitted when empty
```

`detailed` stores `value` as a nested object with `frontmatter`, nullable
`preamble`, and section tuples.

### `--save` Behavior

`--save` decides what to write and, when a write is needed, the CLI persists the
canonical frontmatter (via the same serializer as `md clean --save`) followed by
the original body bytes, unchanged.

- No stored hash → writes the first baseline and exits `0`.
- No content change → leaves the file untouched and exits `0`.
- Content change → updates the stored hash and sets `last_updated` to the
  current local date (`YYYY-MM-DD`).
- Ignore-policy-only change → rewrites `hash.ignored` and recomputes `value`
  under the new ignore-set **without** bumping `last_updated`.
- Higher- or lower-resolution `--kind` → compares at the shared resolution
  before upgrading/downgrading the stored value; an incomparable kind switch
  rewrites at the forced kind without treating the switch as a content change.

`--save` is not supported for stdin input or for directories. It prints an
explanation of what changed rather than the raw hash.

### `--diff` Behavior

`--diff` reports how the document differs from its stored hash, at the
resolution the stored kind affords, and never writes. It exits `2` when any
difference is found (or when there is no stored hash, printing
`No stored hash to compare against`), and `0` when the document is unchanged.

The report shape follows the stored kind:

- `simple` / `fm` / `body`: a single summary line.
- `structured`: two bullet lines distinguishing frontmatter key vs value
  changes and body structural vs semantic-content changes.
- `detailed`: the frontmatter line plus a nested per-section body list
  (content changed, renamed, renamed-and-edited, promoted, demoted, reordered,
  moved, added, removed), in document order.

When the current ignore policy differs from the stored one, an advisory line is
appended; it is never counted as a content change.

### Output Format

**Single file/stdin (bare hash)**

- `simple`: `<frontmatter_hash>-<body_hash>`
- `fm` / `body`: `<hash>`
- `structured`: `<fm>-<fm_keys>-<body>-<body_structure>`
- `detailed`: nested YAML object

**Directory input**

- Recursively collects `.md` and `.dm` files.
- Skips hidden (dot-prefixed) directories. All other directories — including
  `node_modules`, `target`, and `vendor` — are traversed, so their Markdown
  contributes to the aggregate fingerprint.
- Ignores non-markdown files.
- Sorts paths before aggregation for deterministic output.

Directory mode is **bare-hash only**: `--save`, `--diff`, and the `structured`
and `detailed` kinds are rejected with a usage error. Directory output forms:

- Default: aggregate `<frontmatter_hash>-<body_hash>`
- `--body` or `--frontmatter`: single aggregate hash

### Exit Codes

- `0`: bare hash printed, or `--save` completed (whether or not it wrote).
- `1`: operational error (unreadable file, malformed stored hash, etc.).
- `2`: `--diff` found differences, or there was no stored hash to compare.

### Normalization Behavior

**Non-strict mode**

- Frontmatter: canonicalized by sorted keys and JSON-serialized values.
- Frontmatter keys (`structured`/`detailed` `fm_keys`): sorted before hashing.
- Body: leading/trailing whitespace and blank-line variants are ignored, so
  whitespace-only edits do not change the semantic hash.

**Strict mode**

- Frontmatter: hashes YAML serialization without canonical key sorting.
- Frontmatter keys (`structured`/`detailed` `fm_keys`): preserved in document
  order — strict performs no key reordering.
- Body: hashes raw body bytes.
- Body structure (`structured` `body_structure`): a verbatim fingerprint of the
  heading skeleton (level + literal heading text, in document order). The heading
  text is the literal source with only ATX/setext markers and surrounding
  whitespace removed, so inline Markdown is preserved — `# Install *Now*` and
  `# Install Now` are distinct structural headings. It applies no whitespace
  normalization in either mode, so strict does not change it; whitespace-only
  differences in heading *source* surface through the verbatim `body` component
  instead.

## Lessons Learned

- Non-strict mode is best for change detection where formatting-only edits
  should collapse.
- Strict mode is best when exact serialized content differences matter.
- The library reads no environment; the CLI owns all flag and env parsing and
  passes an explicit options bundle to deterministic library APIs.
- Directory mode is deterministic and optimized for larger doc trees.

## Issues

- There is no structured (`--json`) output mode; bare hash output is plain text
  (or YAML for `detailed`).
