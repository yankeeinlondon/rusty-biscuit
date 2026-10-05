---
$schema: feature-review.yaml
ready: false
findings:
    - title: Link destinations change across prose parsing and expression composition
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: 2026-10-04T07:46:10-07:00
spec: 2026-10-02-inline-prose/spec.md
implemented: false
description: 'A **feature** review of `2026-10-02-inline-prose/spec.md`'
feature: 2026-10-02-inline-prose/review-14.md
previous: 2026-10-02-inline-prose/review-13.md
---

# Inline Prose — Review 14

The feature is **not production ready**. Review 13's generated-comment finding is repaired at all of its named consumer sites. A further sweep found that link destinations are still interpreted as prose syntax or serialized without sufficient escaping. This changes the file or URL a link opens, even though its label can look correct.

This review evaluates the existing working tree. Permanent changes are limited to this document and the requested review/spec metadata. Temporary probes were appended to declared Level 1 integration and unit-test modules, run through nextest, and removed by restoring those files byte for byte. No implementation repair, formatter, commit, or lifecycle move was made.

## Previous review follow-through

Review 13 had one unblocked finding and no blocked findings.

| Previous requirement | Result |
|---|---|
| Generated separators must be invisible through darkmatter's public terminal and HTML entry points | Implemented. The retained integration matrix passes through both public entry points across sixteen writer contexts, eleven wrapper routes, thirteen boundaries, six value pairs, and both dialects. |
| Shared terminal rendering must accept comment-only nodes in every strictness mode | Implemented. Comment-only nodes contribute no output and no diagnostic; other raw HTML retains its prior policy. |
| Browser tree and streaming renderers must handle the separator under default Escape, Reject, and Allow policies | Implemented. Escape/Reject emit nothing without diagnostics; Allow retains the actual comment. The policy matrix passes. |
| Both public prose components must read their emitted separator without visible text | Implemented. Both components preserve the neighboring code values; literal code, escaped comment text, and recognized quoted attributes remain intact. |
| Keep distinct code values and safe fences through an independent Markdown reader | Passing. The shared adjacency matrix and the new consumer matrix both pass. |
| Keep comments during Markdown reserialization and document the consumer policy | Implemented. The fold retains structural comment nodes, and the writer keeps the separator without a portability diagnostic. The relevant docs describe the exception. |

I compared findings and blocked sections across reviews 1–13. None contains a blocked code finding that became unblocked before this implementation. Reviews 2–3's human-review requests concerned repair completeness, not an unresolved design choice or a human-only test. No new human decision blocks the repair below.

## Recurrence

This repeats the opacity class in reviews 1–3's **“Opaque code bodies and quoted attributes are altered by preprocessing”** and review 4's **“Wrapper scanning still interprets quoted and escaped tag text.”** Those repairs should have swept Markdown link destinations alongside authored quoted attributes: destinations become generated `href` attributes after the initial lifting pass. The new comment reader follows the same unprotected route as lifted code spans and explicit code bodies.

It also repeats the literal-value serialization class in reviews 1–2's **“Markdown serialization changes literal backslashes and break meaning”** and review 4's **“Markdown serialization reinterprets literal text and entity spellings.”** The shared tree writer's destination escaping was repaired, but the expression destination formatter and darkmatter's cleanup reserialization were not swept with the same values. Review 13 checked generated comments as standalone markup and opaque code/attribute controls; comment-shaped text in a Markdown destination was another sibling that needed checking.

The full affected and clean destination paths are listed below as one finding. Previously repaired code-content and generated-separator cases remain clean.

## Unblocked Findings

### High: Link destinations change across prose parsing and expression composition

**Defect class:** a link destination is interpreted as inline syntax or emitted without the escapes its next reader requires, so parsing or composition changes the destination rather than preserving its literal value.

In **biscuit-terminal**, [the initial opaque-content scan](../../lib/src/components/prose/markdown.rs:228) lifts code spans, comments, and explicit code bodies before [Markdown links are recognized](../../lib/src/components/prose/markdown.rs:573). If that syntax occurs in a destination, its placeholder becomes part of the generated `href`. [Restoring destinations](../../lib/src/components/prose/markdown.rs:908) restores that already-modified string, and [the attribute reader](../../lib/src/components/prose/tokens.rs:27) leaves the generated placeholder in the public link node. The same destination reader stops at the first unescaped closing parenthesis and retains angle-bracket destination delimiters. The intermediate anchor always uses double quotes without protecting a destination's own double quotes.

In **darkmatter**, [`code_link_fn`](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2302), which creates template links with code-styled labels, shares [destination formatting](../../../darkmatter/lib/src/markdown/compose/expression/functions/mod.rs:2186) with `link_fn`. That formatter protects angle brackets when wrapping but does not protect literal backslashes or entity spellings. A standard Markdown reader therefore changes those values immediately. Separately, [cleanup's event-to-Markdown serialization](../../../darkmatter/lib/src/markdown/cleanup/mod.rs:356) can remove necessary destination escapes from otherwise faithful input. This happens to both links and images and is observable in composed template output.

For a valid backtick-containing destination, the new function produces:

```markdown
[`label`](https://e.io/a`x`b)
```

Darkmatter's public document reader preserves `https://e.io/a` followed by the literal backtick-delimited `x` and final `b`. Both prose components instead store `https://e.io/a\u{0002}1\u{0002}b`: the notation here represents actual control characters, not the printed escape spelling. Their Markdown, HTML `href`, and terminal hyperlink carry that corrupted value. The label still renders as code, which can hide the defect from a label-only assertion.

The shipped CLI also reproduces the new comment instance:

```sh
cargo run --quiet --color=never -p biscuit-terminal-cli -- \
  prose '[x](https://x.io/a<!-- -->b)' --html
```

The anchor's destination becomes `https://x.io/a\u{0002}0\u{0002}b`. This input follows Prose's existing literal-destination handling; the backtick example above additionally demonstrates a standard Markdown link and an actual `code_link()` output.

#### Complete site sweep

I copied the retained destination and `code_link()` fixture builders, changed only destination values, and exercised public outputs. The direct prose probe checked thirteen payloads through two link syntaxes and both components: **52 cases**, with **16 failures**, all on the Markdown-link route. The extended sweep recorded **331 cases**: thirty-one destination payloads through explicit anchors, all three expression names, both shared-writer dialects, link cleanup, and image writing/cleanup, plus seven actual file names through each expression name's one-argument form. Each recorded case checks the relevant public destination rather than merely the parser's success status.

| Site | Shape tested | Observed result | Expected result |
|---|---|---|---|
| `Prose` Markdown-link reader | Matched backticks; six completed comment spellings/pairs; explicit code body | Generated placeholders enter the tree, HTML, Markdown, and OSC 8 destination | Keep literal destination contents; recognize code/comments in the label only. |
| `InlineProse` Markdown-link reader | Same destination edits | Same failures | Same destination semantics as block prose. |
| Both components' destination closer scan | Balanced/nested parentheses from composed links | Ends at the first `)`; leaves the suffix as visible text | Consume the whole destination. |
| Both components' angle-bracket destination reader | A space-containing URL or file name; escaped comment destination from the shared writer | `<` and `>` become part of the destination | Remove syntax delimiters and preserve the value they delimit. |
| Both components' generated-anchor attribute reader | Destination containing double quotes | Destination truncates at the first quote | Preserve quotes without changing the generated attribute boundary. |
| Both components reading the shared writer's escaped destination | Literal `&amp;`, `&copy;`, and `&#10;` spellings | Keeps the protective backslash in the destination | Decode the Markdown escape exactly once and preserve the literal entity spelling. |
| Explicit anchor using public `Prose::quoted_attr` | All thirty-one payloads, both components | Clean: all authored destination strings retained | Keep this opaque-attribute route working. |
| Darkmatter `code_link(target, desc)` and alias `codelink` | Literal backslash before punctuation, doubled backslash, and three entity spellings | The raw emitted link already decodes to a changed destination | Serialize the resolved value faithfully. |
| Darkmatter `link(target, desc)` | Same payloads and fixture | Same destination defects; ordinary label instead of code label | Share the repaired destination policy. This existing function is a sibling of the new function. |
| All three expression names, `function(file)` | Actual files: ordinary, space, parentheses, matched backticks, literal `&copy;`, literal `&amp;`, and non-entity ampersand | Ordinary/non-entity controls clean; prose misreads spaces/parentheses/backticks; composed entity names point at a differently spelled file | Preserve the resolved file identity and the code label's existing behavior. |
| Renderable shared link writer | All thirty-one payloads, both dialects, read immediately by darkmatter | Clean: all destination values retained | Preserve the existing correct escaping. |
| Renderable shared image writer | Same matrix, both dialects | Clean: all image destination values retained | Same contract; it shares the destination helper. |
| Darkmatter cleanup of the shared writer's link output | Same matrix, both dialects | Seven payloads lose destination meaning, including the escaped comment, entity spellings, and escaped backslashes | A cleanup must not change the link target. |
| Darkmatter cleanup of the shared writer's image output | Same matrix, both dialects | The same seven payloads fail | Preserve image targets through the same serialization path. |
| Darkmatter public document reader | Faithfully escaped raw expression output and direct shared-writer output | Clean for correctly escaped destinations; faithfully reveals malformed cleanup output | Retain the independent-reader control; do not hide serializer losses in the reader. |
| Shared browser/terminal projections | Inspected direct-node handling and exercised prose-generated nodes | Preserve the destination they receive; prose's bad value reaches final output | Repair the producer, preserving renderer policies. |
| Darkmatter pull-request link formatter | Copied retained provider record; changed only its web URL through the same thirty-one payload families; rendered the actual formatter output through public document and prose APIs | Literal `&amp;`, `&copy;`, and `&#10;` spellings decode to different targets in darkmatter; prose retains them. Other tested shapes follow the provider's existing URL normalization policy | Protect entity spellings after normalization. This preexisting sibling is outside the feature's changed code. |
| Darkmatter CI-job link formatter | Same matrix on the retained job fixture's web URL | Same three entity failures; other tested shapes clean under its normalization policy | Same contract. This also predates the feature. |
| Code labels, ordinary code bodies, quoted attributes, and generated separators | Retained prose opacity/grammar tests and both adjacency matrices | Clean | Do not repair destinations by undoing code opacity or deleting comments globally. |

The payload table completes the shape sweep, including clean controls. Each payload replaces `ZQZ` in the positive-control destination `https://e.io/aZQZb`. File cases use corresponding actual fixture names.

| Payload family | Prose reading generated Markdown | Expression formatter → standard reader | Correct shared writer → cleanup → standard reader |
|---|---|---|---|
| Matched single/double backtick runs | Placeholder leak | Clean | Clean |
| Unmatched backtick | Clean | Clean | Clean |
| Completed ordinary comment | Placeholder leak on direct input; delimiter leak on escaped/wrapped input | Raw formatter clean; cleanup removes angle escapes and the link disappears | Link/image disappears |
| Short completed comment | Placeholder leak | Clean | Clean |
| Explicit `<code-block>` body | Placeholder leak | Clean | Clean |
| Recognized ordinary style tag; unknown `<x>` spelling | Clean | Clean | Clean |
| Balanced and nested parentheses | Composed expression input truncates; shared writer's escaped parentheses are clean | Clean for literal parentheses | Clean for literal parentheses |
| Backslashes before parentheses | Shared writer input clean; composed expression input already changed and then truncates | Literal backslashes consumed | Literal backslashes consumed |
| Embedded spaces | Angle delimiters retained | Clean | Clean |
| Double quotes | Generated attribute truncates | Clean | Clean |
| Single quotes | Clean | Clean | Clean |
| Literal `&amp;`, `&copy;`, `&#10;` | Expression input already changed; shared-writer input keeps an unwanted protective backslash | Entity decoded; numeric line-feed entity can make the composed link disappear | Entity decoded; numeric line-feed entity can make the cleaned link disappear |
| Non-entity ampersand | Clean | Clean | Clean |
| Backslash before non-punctuation `x` | Clean | Clean | Clean |
| Backslash before `*`; doubled backslash | Shared writer input clean; expression input already changed | Backslash removed/halved | Backslash removed/halved |
| Brackets, `**x**`, `_x_` | Clean | Clean | Clean |
| Percent-encoded backticks/angles, Unicode, pipe, fragment, query | Clean outside table-specific transformations | Clean | Clean |

The initial direct-input sweep additionally covers `<!--->`, a nonempty comment, two comments separated by text, and an unclosed comment. All completed comment forms leak placeholders on the Markdown-link route; the unclosed comment stays literal. Every explicit-anchor counterpart is clean. Existing retained table-cell destination matrices cover pipe protection; this review does not reinterpret their table-specific policy as a destination failure.

The final formatter search found an independently implemented [provider destination helper](../../../darkmatter/lib/src/markdown/compose/expression/functions/escape.rs:98), used by [pull-request formatting](../../../darkmatter/lib/src/markdown/compose/expression/functions/pull_requests.rs:136) and [CI-job formatting](../../../darkmatter/lib/src/markdown/compose/expression/functions/cicd.rs:181). An additional **62-case** fixture sweep checks those sibling sites through final public rendering, with no provider network call. Their three entity-spelling failures are included above for completeness, explicitly as preexisting code outside this feature. They are not needed to establish the feature's readiness failure. The helper's deliberate URL normalization and percent encoding are distinct from the expression family's preserve-the-resolved-string contract; retain that provider policy while repairing its Markdown escaping.

Repair destination preservation across the affected paths in the site table. Protect destinations before prose's code/comment lifting, preserve destination boundaries and escapes while recognizing Markdown links, and avoid inserting unescaped values into generated attribute markup. Use a common faithful destination spelling for the expression family and preserve it through cleanup. Do not globally remove comments or generated-looking controls, percent-encode every input indiscriminately, or relax raw-HTML policies to conceal the problem.

Retain one destination fixture matrix spanning these routes and shapes. Assert the decoded public tree target, browser target, Markdown-reader target, and terminal hyperlink, as applicable; also assert the exact label and trailing text so truncation cannot pass. Include one- and two-argument expressions, the alias, real file names, both dialects, links and image cleanup, and the clean explicit-anchor controls. Review the affected function comments and current documentation: the code's claim that URL contents are protected from Markdown phases omits the earlier lifting pass, and the docs' destination-preservation promise is currently untrue for these cases.

This is a Level 1 correctness defect: the destination is already wrong before a browser or terminal emulator receives it. It requires no keyboard injection or human design choice.

## Blocked Findings

None.

## Input Robustness Matrix

There is no new manifest, lockfile, or configuration reader in the latest repair. The feature adds one load-bearing serialized field, JSON `browser.block_element`. Its retained production-serialized fixture matrix passed in the fresh full renderable suite. The input side is JSON deserialization; the output side is the public browser rendering result.

| Shape | JSON `browser.block_element`: expected and observed |
|---|---|
| Positive control | All seven lowercase tags render correctly; other attributes retained. |
| Absent | Legacy paragraph renders `<p>`. |
| Explicit null | Rejected. |
| Wrong whole-field type | Number rejected. |
| Wrong type, one element | Mixed array rejected; this field is scalar. |
| Wrong type, every element | All-invalid array rejected. |
| Empty | Empty string, array, and object rejected. |
| Duplicate key | Rejected. |
| Trailing or invalid content | Rejected. |
| Invalid spelling | Unknown and uppercase names rejected. |

## Verification

| Check | Fresh result |
|---|---|
| Renderable `just test` | 684 passed, zero skipped, including serialized-field, literal-value, HTML, table, line-ending, adjacency, and comment-policy matrices. |
| Biscuit-terminal focused destination/grammar/opacity/separator/comment run | 85 passed. |
| Biscuit-terminal `just test prose`, after removing probes | 455 passed; 3,156 excluded by scope/tier. |
| Darkmatter `just test serialized_code_neighbors code_link` | 25 passed, including the retained full serialized-neighbor consumer sweep and DMLS completion. |
| Temporary declared Level 1 destination probes | Direct 52-case reproduction, extended 331-case public-output sweep, and 62-case provider-formatter sweep completed. Removed afterward. The first probe deliberately fails with the sixteen destination mismatches; the recording probes complete successfully and record affected and clean results for comparison, rather than claiming correctness. |
| Shipped `bt prose --html` | Reproduced a generated placeholder inside the link destination. |
| `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_prose_inline_code_and_breaks_in_tmux` | One actual CLI tmux scenario passed; backend proof records `run=1`, `skip=0`, `panic=0`. Zero library tests selected. Background/detached harnesses requested no focus. |
| Root `just check-tier-coverage` for biscuit-terminal, renderable, darkmatter | All passed; zero stranded tests. |
| `just lint` in renderable, biscuit-terminal, and darkmatter | All passed; library/CLI checks and darkmatter's DMLS/Zed checks included. Temporary provider probes were added after these lint checks and restored byte for byte. |

Full consumer suites, changed examples/benchmarks, and cross-OS runs were not repeated. Their prior implementation-log results are not substituted for fresh evidence. Cross-OS evidence and human sign-off do not determine this readiness verdict. No permanent test was added or renamed; the relevant consolidated roots declare the retained prose and darkmatter modules, and terminal test targets require features enabled by CI and live recipes.

## Requirement-to-verification mapping

Criterion numbers below refer to the specification's Acceptance Criteria list.

| User-facing requirement | Strongest relevant verification present | Assessment |
|---|---|---|
| Inline/block shape, empty inputs, neutral inline projection, tags and streaming parity (criteria 1, 8–9, 11, 25, 31) | Level 1 public tree/HTML and serialized-fixture tests | Appropriate level; inspected retained coverage and fresh grammar/renderable tests pass. |
| Paragraphs, soft/hard breaks, whitespace, line endings and opaque code (criteria 2–7, 18, 26–28) | Level 1 grammar/opacity tests and Level 2 terminal row captures | Appropriate levels; fresh tmux scenario verifies displayed soft/hard breaks and paragraph separation. |
| Layout exactly once, wrapping and container shape (criteria 10, 12, 27–28, 31) | Level 1 container/layout assertions and retained Level 2 container captures | Appropriate levels present; complete container tier not rerun. |
| Code appearance, enclosing-style restoration, hyperlink extent and unstyled fallback (criteria 1, 14–16, 21, 29) | Level 1 capability tests and Level 2 captured styles/links | Fresh tmux scenario passes; destination-value cases fail at Level 1 as reported above. |
| Safe fences, literal code, hard-break serialization and table structure (criteria 14–19, 21, 25, 28, 30) | Level 1 independent-reader and public-consumer matrices | Fresh retained matrices pass, including review 13's separator integration. |
| Diagnostic and newline migrations (criteria 13, 17, 20) | Retained Level 1 output snapshots/regressions and representative Level 2 geometry | Coverage present; full downstream suites not rerun. |
| `code_link()` destinations, labels, templates, listing and completion (criteria 22–23) | Level 1 expression/template/CLI/LSP tests | Appropriate level, but destination coverage missed the reproduced shapes. High finding above. |
| Affected-area tests/lint and example/benchmark compilation (criterion 24) | Prior implementation log and explicitly scoped fresh checks | No claim that every affected package was reverified in this review. |
| Keyboard/input encoding | No input interaction added | No Level 3 requirement. |

No separate performance or ergonomic defect was reproduced. The preexisting messenger Discord/Slack fence-adjacency issue recorded in earlier logs is outside this feature's implementation and does not determine readiness here.
