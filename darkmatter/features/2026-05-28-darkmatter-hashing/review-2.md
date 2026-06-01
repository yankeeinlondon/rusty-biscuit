---
ready: false
agent: codex
model: ""
---

# Review 2

## Findings

### High: structured and detailed heading fingerprints are not using literal heading text

- Location: `darkmatter/lib/src/markdown/hash/compute.rs:202`
- Requirement: `structured` `body_structure` hashes all headings including heading level, and `detailed.sections[*].heading` stores the literal heading text with only the leading `#` characters and surrounding whitespace removed.
- Current behavior: both `hash_body_structure` and `compute_detailed` use `MarkdownTocNode::title`. The TOC title is assembled from pulldown events using only `Event::Text` and `Event::Code` (`darkmatter/lib/src/markdown/toc/mod.rs:130`), so inline Markdown syntax is discarded. For example, `# Install *Now*` and `# Install Now` produce the same structural heading text even though the literal heading source differs.
- Why this matters: `--kind structured --diff` can report "same structural layout" for a heading-source change that the spec defines as structural, and saved `detailed` hashes do not persist the specified literal heading string. The body hash still changes, so the CLI exits with a difference, but the explanation resolution is wrong.
- Verification level: missing Level 1 coverage. Add unit tests showing that `# A *B*` and `# A B` have different `body_structure` values and that detailed tuples persist the literal heading text expected by the spec.
- Suggested fix: derive the heading text for hashing/storing from the heading source span instead of `MarkdownTocNode::title`, stripping only ATX/setext markers and surrounding whitespace while preserving inline Markdown syntax.

### Medium: detailed stored hashes accept impossible section levels

- Location: `darkmatter/lib/src/markdown/hash/stored.rs:274`
- Requirement: detailed `sections` tuples store `[level-num, heading, content-hash]`, where `level-num` is a heading level `1-6`.
- Current behavior: `validate_detailed` validates hash fields but never validates `section.level`. A stored detailed hash with `[0, "Bad", "..."]` or `[9, "Bad", "..."]` parses successfully instead of going through the `MalformedStoredHash` operational-error path.
- Why this matters: corrupted baselines can produce nonsensical reports such as `H9` promotions/demotions and bypass the spec's malformed stored hash contract.
- Verification level: missing Level 1 coverage. Add parser tests for `0` and `7` section levels expecting `MarkdownError::MalformedStoredHash`, plus a CLI `--diff` fixture expecting exit code `1`.
- Suggested fix: extend `validate_detailed` to reject section levels outside `1..=6`.

## Test Rigor

This feature is library/CLI file, stdout, stderr, and exit-code behavior. Level 1 tests are the appropriate minimum; no Level 2 or Level 3 terminal verification is required because there are no terminal rendering, keyboard input, glyph width, paste, mouse, or emulator encoder requirements.

Existing Level 1 coverage now covers the prior review's main gaps: directory hashing uses the new ignore policy, detailed parent hashes include child subtrees, malformed flat hashes exit through the operational error path, and strict frontmatter key ordering is tested. The remaining gaps above are also Level 1 gaps.

## Production Readiness

Not ready for production. The implementation is close, but `structured` and `detailed` do not yet persist or compare the literal heading text required by the feature contract, which directly affects user-facing diff explanations.

## Verification Notes

I attempted targeted Level 1 test runs:

- `cargo test -p darkmatter hash:: --color=never`
- `cargo test -p darkmatter-cli test_hash_ --test cli --color=never`

Both commands were still blocked on Cargo locks / compiling after roughly 60 seconds, so I terminated the Cargo processes per the non-interactive session guidance. No passing test result is claimed from those runs.
