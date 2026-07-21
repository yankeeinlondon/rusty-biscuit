---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T19:06:33-07:00
spec: 2026-07-13-fixed-width-lists/spec.md
log: darkmatter/fixes/2026-07-13-fixed-width-lists/log.md
implemented: true
implemented_by: codex/default
description: "A **fix** review of `2026-07-13-fixed-width-lists/spec.md`"
fix: 2026-07-13-fixed-width-lists/review-4.md
previous: 2026-07-13-fixed-width-lists/review-3.md
---

# Review 4 — Fixed-Width Lists

## Verdict

This fix is **not ready for production**. Review 3's three reported structural reproducers are
fixed for their exact unquoted/tight-list fixtures, `--indent 8` is accepted again, and the full
Level-1 and Level-2 gates are green. However, the parser-derived repair does not cover the same
content inside blockquotes or raw HTML. Valid blockquoted additional paragraphs escape their list
items, blockquoted marker-looking indented code becomes a real list, and raw HTML can steal the
authored marker belonging to a later list item. The restored `--indent 8` option also does not
actually emit an eight-column step for ordinary narrow markers, and the mandatory Criterion timing
budgets remain unmeasured.

## Findings

### High — Blockquoted indented code is converted into a list

Review 3's marker-looking-code fix is explicitly limited to unquoted code.
`extract_unquoted_indented_code_marker_ordinals` ignores indented code while a blockquote is
active, while `fix_list_indentation` still recognizes a serialized quoted `- ` or ordered marker
by its physical line. A later real item at the same blockquote depth supplies the pending
`ListItemContext`, so the code line consumes that context and is rewritten as a list item.

Fresh CLI reproduction:

```markdown
> - Parent.
>
>       - literal code
>
> - Later sibling.
```

`md clean -` emits:

```markdown
> - Parent.
>
> - literal code
>
>
> - Later sibling.
```

The input contains an indented code block owned by the first item; the output contains another
quoted list item. The nearby function documentation says blockquoted code is handled by existing
container-specific paths, but this reproduction proves that comment is stale. This violates AC 5,
9, 10, 12, and 13.

**Suggested resolution:** replace the unquoted-only ordinal side channel with parser-derived block
records that retain blockquote depth and block kind through serialization. Add exact-output,
structural-fingerprint, and second-pass Level-1 fixtures for unordered and ordered code-looking
lines at blockquote depths one and two, through default/configured/fixed-width library cleanup,
spawned CLI, compose, and DMLS.

### High — Additional paragraphs inside blockquoted items escape the list

The additional-paragraph repair is also unquoted-only. `extract_unquoted_additional_paragraph_indents`
collects no context while `blockquote_depth > 0`, and a serialized blank quote line is not
recognized by `fix_list_indentation` as the blank that precedes a subsequent item paragraph. Valid
input such as:

```markdown
> - Parent first.
>
>   Second paragraph.
>
>   - Child.
```

is cleaned to:

```markdown
> - Parent first.
>
> Second paragraph.
>
>     - Child.
```

`Second paragraph.` has moved out of the list item and become ordinary blockquote prose. With
`--fixed-width 24`, that paragraph is wrapped with only `> ` and the child can remain longer than
the requested width because the semantic list ownership has already been lost. The current tests
cover an unquoted additional paragraph plus child and a tight blockquoted parent plus child, but
never their required combination. This violates AC 1, 5, 8, 9, 10, 11, 12, and 13.

**Suggested resolution:** represent subsequent paragraphs as parser-derived container records,
including quote depth, item depth, source body column, and serialized continuation prefix. Add a
matrix combining loose paragraphs, nested children, ordered/unordered/task parents, one/two quote
levels, list-spacing modes, configured indentation, fixed width, and a second cleanup pass.

### High — Full cleanup rewrites list-looking content inside raw HTML

The specification protects HTML blocks, but the full cleanup pipeline's marker restoration remains
line-oriented. `restore_list_markers` treats any serialized `* ` body outside a fence as an actual
unordered item; it does not consume parser-derived HTML ownership. For example:

```markdown
<div>
* literal html
</div>

- Actual item.
```

`md clean -` emits:

```markdown
<div>
- literal html
</div>

* Actual item.
```

The literal HTML payload is modified and consumes the authored `-` marker intended for the actual
list item. Existing HTML coverage exercises `strip_incidental_newlines` and `reflow_to_width`, not
the complete `cleanup_content` sequence where the corruption occurs. This violates AC 5, 9, 10,
12, and 14.

**Suggested resolution:** restore unordered markers only at parser-confirmed item starts rather
than at arbitrary marker-looking physical lines. Carry the authored marker on each item context or
use protected serialization placeholders; do not add a second default-path parse. Add full-cleanup
exact-output and fingerprint fixtures for list-looking lines inside HTML, fenced/indented code,
tables, and protected directive bodies, followed by real list items with different markers.

### High — `--indent 8` is accepted but its specified behavior is still not implemented

The CLI schema regression from Review 3 was reversed, but the contract was not reconciled.
`parse_indent_size` accepts 8 while `fix_list_indentation` clamps a child to at most three columns
beyond its parent's content column. Consequently:

```markdown
- Parent
  - Child
```

cleaned with `md clean --indent 8 -` places the child at column 5, not column 8. Eight columns are
emitted only when a sufficiently wide parent marker already makes that column CommonMark-valid.
The new test locks in this marker-dependent clamp and calls it structural preservation; it does not
satisfy the specification's configured eight-space nesting requirement. The CLI help still calls
the value "spaces per level," and the cleanup guide simultaneously says the value is only preferred
and that it is enforced at every nested level.

Review 3 required either a ratified compatibility/specification change or actual support. Accepting
the flag while silently producing a different width does neither, so AC 7 and 14 remain failed.

**Suggested resolution:** make one explicit public-contract decision. Either redefine `--indent`
as a preferred step with a documented per-marker clamp and amend the specification/acceptance
tests, or remove the unrepresentable value. If exact eight-column nesting remains normative, choose
a serialization strategy that can represent it without changing the CommonMark tree.

### High — Required performance timing evidence remains deferred

The deterministic parse-count half of AC 15 passes: default cleanup performs one cleanup-path
parse, and full cleanup plus fixed-width reflow performs two. The Criterion timing half still has
no baseline/candidate medians or B1/B2/B3 verdicts. At review time the 16-core host had eight users
and load averages of `15.90 17.66 20.15`, above the deferred artifact's admissibility ceiling of
2.0, so no meaningful timing sample was taken.

**Suggested resolution:** run the documented baseline → candidate → baseline bracket on an
admissible quiet host, enforce the 3% baseline-drift guard, and record every per-fixture median and
the explicit 10%, 15%, and 2x verdicts. AC 15 remains incomplete until every case passes.

### Medium — Ten-digit numeric prose still participates in list-spacing normalization

The reflow and strip paths correctly cap ordered markers at nine digits, but
`is_list_item_start` has no equivalent cap. Full default cleanup therefore removes the blank line
between ordinary ten-digit prose and a following list:

```markdown
1234567890. prose

- actual
```

becomes:

```markdown
1234567890. prose
- actual
```

This contradicts the specification's explicit marker boundary and leaves AC 11 and 14 only
partially verified. Current ten-digit tests cover strip/reflow, not spacing normalization.

**Suggested resolution:** share one ordered-marker recognizer across reflow and list normalization,
then add nine/ten-digit boundary fixtures for normal, compact, and loose cleanup adjacent to real
ordered and unordered lists.

## Requirement-to-Verification Assessment

The cleanup behavior is deterministic source-to-source transformation, so Level 1 is the correct
primary tier. The specification separately mandates the package-area Level-2 gate; it is now green.
No requirement depends on a terminal input encoder or OS keyboard event, so Level 3 is not
applicable.

| Requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| AC 1 — list prose cleanup at nesting/quote depths | Level 1 library and spawned CLI | **Fail:** a subsequent paragraph inside a quoted item escapes the item. |
| AC 2 — collapsed continuation prefix leaves zero/one separator | Level 1 exact output | Pass for represented prose-continuation fixtures. |
| AC 3 — default strip emits no hanging indentation | Level 1 exact output | Pass for represented fixtures. |
| AC 4 — preserve mode | Level 1 library/CLI and second pass | Pass for represented authored soft breaks. |
| AC 5 — complete logical-block unwrap before wrapping | Level 1 exact output and matrix | **Fail:** quoted loose paragraphs lose item ownership before fixed-width wrapping, and protected quoted/HTML blocks are corrupted. |
| AC 6 — full hanging prefix and display-column alignment | Level 1 exact output/width assertions | Pass for represented logical list paragraphs. |
| AC 7 — digit/task/configured nesting/quote prefixes | Level 1 library and CLI | **Fail:** accepted indent 8 is marker-dependent and usually emits five columns. |
| AC 8 — total display width | Level 1 width assertions | **Fail:** after quoted paragraph ownership is lost, fixed-width output can leave the child line over width. |
| AC 9 — structural boundaries and protected blocks | Level 1 fingerprints/exact output | **Fail:** quoted code and HTML change meaning/content. |
| AC 10 — non-vacuous structural fingerprints | Level 1 parser fingerprint | **Fail:** the required combination matrix omits all three structural reproducers. |
| AC 11 — spacing modes | Level 1 exact output | **Fail:** ten-digit prose is treated as an item by spacing normalization. |
| AC 12 — library/compose/CLI/save/DMLS parity | Level 1 cross-surface tests | **Fail:** byte parity on selected fixtures does not cover the shared incorrect quoted/protected-block cases. |
| AC 13 — idempotence | Level 1 second-pass tests | **Fail:** a stable second pass cannot repair structure already changed on the first pass. |
| AC 14 — public/CLI/marker compatibility | Level 1 parser/completion/output tests | **Fail:** indent 8's effective behavior conflicts with the specified spaces-per-level contract; ten-digit spacing uses the wrong marker rule. |
| AC 15 — parse and timing budgets | Level 1 parse counters; Criterion timing absent | **Fail:** parse counts pass, but all timing verdicts are missing. |
| AC 16 — scoped build/L1/L2/lint/change detection | Build, Level 1, Level 2, lint, GitNexus | Pass: all scoped gates are green and the changed execution flows are cleanup/compose related. |

## Verification Performed

- `just build`: passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- `just test --partition hash:{1,2,3,4}/4`: passed all Level-1 tests — 5,893/5,893
  library, 619/619 CLI, and 569/569 DMLS.
- `just lint`: passed for all three packages.
- Broker-owned Level-2 recipes passed all 91 tests: 19/19 library, 69/69 CLI (two exhaustive
  hash partitions), and 3/3 DMLS.
- Fresh `md clean` and `md clean --fixed-width` stdin reproductions confirmed the quoted-code,
  quoted-additional-paragraph, raw-HTML, indent-8, and ten-digit-spacing findings above.
- `sniff` identified the affected package area as `darkmatter`, containing `darkmatter`,
  `darkmatter-cli`, and `dmls`; the public library also has downstream workspace consumers.
- GitNexus rates `fix_list_indentation` **critical** (145 upstream symbols, two direct callers,
  one compose execution-flow family) and `cleanup_content_with_indent` **high** (17 upstream
  symbols, 15 direct callers). Review-cycle change detection reported the expected cleanup, CLI,
  test, documentation, and compose-cleanup flow surfaces.
- Criterion timing was not run because the host failed the documented quiet-host precondition.

## Production Readiness

`ready: false` is required until quoted list containers and protected blocks retain parser-derived
ownership through full cleanup, the indent-8 contract is explicitly reconciled, the ten-digit
marker rule is shared by spacing normalization, and every mandatory performance budget has a
recorded passing verdict.
