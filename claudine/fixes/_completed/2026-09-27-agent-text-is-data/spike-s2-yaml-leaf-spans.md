---
created: 2026-09-28
spike: S2
---

# Spike S2: nested YAML leaf spans

**Question.** Which agent-owned string leaves in an inline document's
frontmatter can Phase 5 locate and replace with a double-quoted literal token
by editing exact source bytes, without re-serializing the document and without
adding a YAML parser?

**Answer.** Darkmatter already has what is needed:
`schemas::simplified::source::locate_frontmatter_value` (block/flow structure
with byte spans) plus `schemas::simplified::decode_scalar_node` (exact scalar
extent through quoting, escapes, folding, chomping, and CRLF). This is the
pair `FrontmatterExpressionLocation::project` uses today to anchor
interpolation errors. Across 60+ shapes, every located leaf replaced cleanly:
the reparse matched the original tree with only that leaf changed, and CRLF
was kept. There are two limits. The locator is all-or-nothing over the text
it is given, so it must run on **one top-level node at a time**. It also does
not model several common shapes (see the table).

## Candidates evaluated

| Candidate | Verdict |
|---|---|
| `parse_text_frontmatter` / `locate_all_nodes` (`darkmatter/lib/src/markdown/hash/write.rs`) | Top level only. It gives the line range of each top-level `key: …` node, but no scalar spans and nothing nested. Useful as the **first stage**: it isolates the owned top-level node. |
| `restore_properties_text` → `FrontmatterDelta` | Semantic and top level only. Entries carry whole JSON values, so a nested diff must recurse `previous_value` vs `value` itself. There are no spans. |
| `biscuit-terminal` `SourceContext::focused_yaml_regions` (`4f3f601cf`) | Unsuitable. It is a display heuristic that returns 1-based **line** regions for dotted key paths. It splits with `.lines()`, has no sequence indexes, no scalar extents, and returns `None` on any `&`, `*`, or `<<`. |
| `locate_frontmatter_value` + `decode_scalar_node` (`darkmatter/lib/src/markdown/schemas/simplified/{source,yaml_scalar}.rs`) | **Use this.** The locator is `pub(crate)`; `decode_scalar_node` is already `pub`. |

**YAML crates in the workspace.** Darkmatter parses with `serde_yaml_ng`
0.10 (libyaml via `unsafe-libyaml`); `serde_yaml` 0.8/0.9 and `yaml-rust`
0.4 appear only as transitive dependencies (`tree-hugger` uses `serde_yaml`
0.9). None of them exposes scalar markers through its `Value` API, so the
hand-written locator is the only span source. No dependency is needed.

## Supported subset for N8

"Located" means the locator walks the path and `decode_scalar_node` returns
an extent whose decoded text equals the parsed value. "Replaceable" means
splicing `"{{!data:v1:…}}"` over `[node.span.start, end)` reparses to the
original tree with only that leaf changed and no line terminators altered.

| Shape (the owned leaf) | Located | Replaceable | Notes |
|---|---|---|---|
| Top-level plain | yes | yes | |
| Top-level single-quoted (`''` escape) | yes | yes | Span includes the quotes. |
| Top-level double-quoted (escapes, `\u00e9`, `""`) | yes | yes | Span includes the quotes. |
| Plain or quoted followed by `# comment` | yes | yes | The span stops before the comment, so the comment is kept. |
| Quoted key (`"my key": v`) | yes | yes | Keys are never encoded. |
| Value on the next line (`a:\n  text`) | yes | yes | |
| Block map, 2, 3, and 4 levels deep | yes | yes | |
| Block sequence of scalars (indented `  - x`) | yes | yes | |
| Block sequence of maps (every key, every item) | yes | yes | This includes `-` alone on its line with the map below it, and a sequence nested inside a sequence-item map. |
| Multi-line plain, single-quoted, or double-quoted | yes | yes | The span covers the continuation lines. |
| `\|`, `\|-`, `\|+`, `>`, `>-`, `\|2` (explicit indent) | yes | yes | The span runs from the header to the last content byte and excludes the final line break. Trailing blank lines of `\|+` stay behind, which is harmless. |
| Block scalar with a header comment (`\| # note`) | yes | yes | The comment is inside the span and is **dropped** by the replacement. |
| Block scalar nested in a map or as a sequence item | yes | yes | |
| CRLF: top level, nested, sequence of maps, `\|`, `>-` | yes | yes | Only bytes inside the span change, so every `\r\n` survives. |
| Quoted item or value inside a flow collection (`[a, "b"]`, `{b: 'y'}`) | yes | yes | Single-line flow only. |
| Plain item inside a flow collection (`[a, b, c]`) | **no** | no | The decoder is block-context and reads `b, c]`. The decoded-equals-parsed guard catches it. |
| `\|+` block scalar that ends the frontmatter | **no** | (yes) | Guard mismatch. `serde_yaml_ng` on the full block, on the trimmed block, and `Frontmatter` disagree on the trailing newlines. Reject it. |
| Anchored leaf (`&a v`) | located | **no** | The span includes `&a`. Replacing it drops the anchor, and `*a` then fails to parse. Reject it. |
| Alias leaf (`*a`) | located | (yes) | The span is the alias token and the decoded text is the anchor's. Reject it, per N8. |
| Tagged leaf (`!!str 12`) | located | (yes) | The span includes the tag, and the replacement drops it. Reject it for simplicity. |
| Key reached only through a merge (`<<: *b`) | **no** | no | The value has no authored location in the child. A sibling key next to `<<` locates normally. |
| **Indentless sequence** (`items:\n- a`) | **no** | no | This is `serde_yaml_ng`'s emitted style. The whole locator returns `None`. |
| Empty or null value (`a:`) in the same node | **no** | no | The whole locator returns `None`. |
| Sequence of sequences (`- - x`) | **no** | no | |
| Sequence item with a wide marker (`-   name:`) | **no** | no | The locator assumes the map starts at `-` + 2, so the key is not found. It fails safely. |
| Multi-line flow collection | **no** | no | |
| Unquoted `{{ … }}` template value (`review: {{ x }}`) | **no** | no | libyaml sees a flow mapping. The whole locator returns `None`. |

Every "no" fails closed: the locator returns `None`, the key is missing, or
the decode guard rejects the leaf. No experiment produced a wrong span that
the guard accepted.

## Recommended API

Add one function next to `restore_properties_text` in
`darkmatter/lib/src/markdown/hash/write.rs`. It reuses that module's
top-level node splitting and the `MarkdownError::FrontmatterTextEdit` error
family, and it avoids widening the schema-source module's public surface.

```rust
pub enum FrontmatterPathSegment { Key(String), Index(usize) }

pub struct LeafSpan {
    pub range: Range<usize>, // absolute byte range in the document, quotes included
    pub decoded: String,     // the text Darkmatter reads there
}

/// Locates every requested string leaf and returns the spans, or the first
/// unsupported path.
pub fn locate_frontmatter_leaves(
    document: &str,
    paths: &[Vec<FrontmatterPathSegment>],
) -> MarkdownResult<Vec<LeafSpan>>;
```

Algorithm, as proven in the probe:

1. `extract_frontmatter_block`, then `locate_all_nodes` to get the owned
   top-level node's range.
2. Run `locate_frontmatter_value(node_text, yaml_offset + node.range.start)`
   on **that node only**. A whole-frontmatter call fails on any unmodeled
   construct anywhere in the document: in this repository, 68 of 2,785
   Markdown files with valid frontmatter return `None`, including prompts
   with unquoted `{{ … }}` values. Per node, those failures are confined to
   the node that contains them.
3. Walk the path, tracking `parent_indent`: the column of the mapping key, or
   of the sequence's `-`. `project()` already does this.
4. Reject a node whose source starts with `&`, `*`, or `!`, or whose path
   crosses a `<<` key.
5. `decode_scalar_node(yaml_trimmed, node.span.start, parent_indent)`, where
   `yaml_trimmed` is the frontmatter text without its final line terminator,
   exactly as `project()` passes it. The guard is
   `decoded == serde_yaml_ng(yaml_trimmed)[path]`. Otherwise, return an error
   naming the path.

The Claudine encoder (`encode_agent_values`) then does three things:

1. It builds `"{{!data:v1:" + base64url(leaf.decoded) + "}}"`. The payload is
   **the decoded text**, not the `FrontmatterDelta` value. See amendment 3.
2. It splices the replacements in descending `range.start` order.
3. It re-parses the whole frontmatter and asserts that the tree equals the
   restored tree with the owned leaves replaced.

## Evidence

The probe was a throwaway crate under `/tmp/s2ws/`. It used a copy of
`darkmatter/lib` in which `locate_frontmatter_value` was widened to `pub`, so
no repository file changed. It built against the worktree's `Cargo.lock`.

For each fixture, the probe:

1. extracted the frontmatter;
2. located the path;
3. compared the decoded text with `serde_yaml_ng` on both the full and the
   trimmed YAML;
4. spliced in `serde_json::to_string("{{!data:v1:SGk}}")`;
5. re-parsed and compared the result with the expected tree;
6. counted `\n` against `\r\n`.

The probe ran two passes, one on whole-frontmatter location and one on
per-top-level-node location (`CHUNKED=1`). It also ran a corpus pass over the
4,736 tracked `*.md` files outside test fixtures.

Excerpts from the output:

```text
| block |- strip | span=13..43 src="|-\n  line one {{x}}\n  line two" | decoded==parser: true | replace->reparse equal: true
| CRLF block >- last | span=23..45 src=">-\r\n  one {{x}}\r\n  two" | ... | eol ok: true
| anchor def | span=7..24 src="&anc shared {{x}}" | ... | replace->reparse equal: false   (dangling *anc)
| flow seq plain item | span=14..19 src="b, c]" | decoded==parser: false
| block | last key in fm | decoded==parser: full=false trimmed=true
valid-frontmatter docs: 2785, locator Some: 2717, None: 68
CHUNKED: | template elsewhere | span=58..67 src="\"v {{x}}\"" | ... | replace->reparse equal: true
```

Most of the whole-document failures in the corpus come from indentless
sequences (`blast_radius:\n- …`, `sequence:\n- name: …`) and bare `key:`
lines. The rest are unquoted `{{ }}` values and multi-line flow collections.

## Amendments to N8

1. **Location is per top-level node.** Replace "nested leaves use the subset
   S2 proves" with: nested leaves are located inside their own top-level node
   using the table above. A construct Darkmatter's locator does not model, in
   a node the agent did not change, never blocks encoding.
2. **Indentless sequences and empty values should be supported before Phase 5
   ships.** Both are the default output of `serde_yaml_ng` and common in agent
   edits, and today they make the whole owned node unlocatable. Extend
   `BlockLocator::value` in the frontmatter mode only (`multi_line_scalars`),
   leaving the schema v1 grammar untouched:
   - an empty value followed by a `- ` line at the **same** indent is a
     sequence;
   - an empty value with no deeper line is a null scalar with an empty span;
   - use the item content's column rather than `-` + 2 for sequence-item maps.

   This is a small change in Darkmatter with its own unit tests. Without it,
   N8's "fails with an agent-attributed diagnostic" fires on routine edits.
3. **The token payload is the decoded scalar.** `FrontmatterDelta` and
   `parse_text_frontmatter` parse the full YAML block, which includes the final
   terminator. Compose (`Frontmatter`) and `decode_scalar_node` read it without
   that terminator. For a clip block scalar that ends the frontmatter they
   differ by one trailing `\n`. Ownership may still be decided by comparing
   trees from the same parser, but the encoded bytes must be what Darkmatter
   would have read.
4. **Explicitly unsupported (agent-attributed failure, then rollback):**
   - an owned leaf carrying an anchor, alias, or tag, or reached through `<<`;
   - a plain flow-collection item, or any multi-line flow collection;
   - `- - x` nested sequences;
   - a `|+` block scalar that ends the frontmatter.

   Quoted items in single-line flow collections are **supported**, a small
   widening of N8's blanket "flow collections fail".
5. **The header comment of a replaced block scalar is lost** (`| # note` is
   inside the span). N8 already replaces a changed block scalar wholesale, so
   say so explicitly.
6. **Keys are never encoded.** Only scalar values are leaves. An agent-written
   key containing `{{` is outside R3 unless Darkmatter scans keys. S3 should
   confirm.
