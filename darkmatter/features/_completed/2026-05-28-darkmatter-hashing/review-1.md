---
ready: false
agent: codex
model: ""
---

# Review 1

## Findings

### High: directory hashing bypasses the new ignore policy and hashes managed keys

- Location: `darkmatter/cli/src/commands.rs:1407`
- Requirement: `hash` / `last_updated` are always ignored, `HASH_PROPERTY` selects the active hash key, and `HASH_IGNORE_PROPERTIES` is additive for hash computation.
- Current behavior: directory mode still calls the legacy `md.hash(body_only, frontmatter_only, strict)` path for every file. That path hashes the raw frontmatter and does not apply `MdHashOptions::ignore_set()`, so a directory aggregate changes when a document only gains `hash` / `last_updated`, and `HASH_IGNORE_PROPERTIES` has no effect in directory mode.
- Why this matters: after `md hash --save file.md`, `md hash some-directory/` can report a different aggregate solely because the managed baseline fields were written. That violates the core feature invariant that hashes do not hash themselves.
- Verification level: missing Level 1 CLI coverage. Add a directory CLI test with a file whose only difference is `hash` / `last_updated`, plus a `HASH_IGNORE_PROPERTIES=draft` aggregate test.
- Suggested fix: either compute per-file values through `compute_hash(MdHashKind::Simple|Fm|Body, &options)` in directory mode, or explicitly reject env ignore/custom property options for directory mode. The spec points to the former.

### High: detailed section content hashes exclude child sections, contrary to the persisted shape definition

- Location: `darkmatter/lib/src/markdown/hash/compute.rs:224`
- Requirement: detailed `sections` entries hash "all content after the heading and up to the next heading which is at the same level or a parent level."
- Current behavior: `compute_detailed` hashes only `node.prelude_content()`, documented in the implementation as "content before its first child heading." That excludes nested child sections from the parent section hash.
- Why this matters: the saved detailed value is not the shape described by the spec. For a document with `# Intro` and nested `## Setup`, the `Intro` tuple should cover its subtree until the next H1; the implementation only covers text before `Setup`. Downstream downgrade/comparison logic relying on detailed values will not be comparing the specified component.
- Verification level: missing Level 1 unit coverage. Add a detailed hash test where editing a child section changes both the child tuple and the parent tuple required by the spec.
- Suggested fix: compute each section content slice from the heading end through the next same-or-parent heading boundary, then hash that slice under the body whitespace policy.

### Medium: malformed flat hash strings are accepted as valid stored hashes

- Location: `darkmatter/lib/src/markdown/hash/stored.rs:60`, `darkmatter/lib/src/markdown/hash/compare.rs:180`
- Requirement: malformed stored hashes are operational errors surfaced through `MalformedStoredHash` / CLI exit code 1.
- Current behavior: shorthand strings and flat `value` strings are accepted without validating 16-character lowercase hex components. `split_flat` only checks component count, so `hash: not-a-real-hash-but-two-parts` is treated as a valid `simple` baseline and reported as content differences instead of a malformed stored hash.
- Why this matters: corrupted baselines become false content-change reports, and `--diff` exits 2 instead of the specified operational error path.
- Verification level: missing Level 1 unit and CLI coverage. Add parser/comparison tests for invalid hex, wrong component length, and a CLI `--diff` fixture expecting exit 1.
- Suggested fix: validate every flat component by kind during `StoredHash::parse` or immediately in `split_flat`, and validate detailed nested hash fields as well.

### Medium: strict structured hashes still normalize structural subcomponents

- Location: `darkmatter/lib/src/markdown/hash/compute.rs:163`
- Requirement: `--strict` means no whitespace normalization or key reordering.
- Current behavior: structured hashes pass `strict` to the frontmatter and body value hashes, but `fm_keys` always sorts keys and `body_structure` is computed from parsed heading titles without any strict-mode distinction.
- Why this matters: `md hash --kind structured --strict` can still collapse key order and heading-source spelling differences in the two structural components, which weakens the promised strict semantics.
- Verification level: missing Level 1 unit/CLI coverage for `--kind structured --strict`.
- Suggested fix: define strict behavior for `fm_keys` and `body_structure` explicitly. If strict is meant to affect only value components, document that exception in the spec/design and CLI docs; otherwise thread `strict` into those computations and test it.

## Test Rigor

All requirements here are pure library/CLI file and stdout behavior. Level 1 tests are the appropriate minimum; no Level 2 or Level 3 terminal verification is required because there are no terminal-rendering, key-input, PTY, glyph-width, mouse, paste, or emulator-encoder requirements in this feature.

Current coverage has useful Level 1 unit and CLI tests for kind selection, save/diff basics, ignore-policy comparison, write-back body preservation, and explanation rendering. The gaps above are also Level 1 gaps: directory-mode policy, detailed parent section content semantics, malformed stored hash validation, and strict structured behavior.

## Production Readiness

Not ready for production. The implementation covers much of the planned surface, but the directory-mode managed-key bug breaks the central "hash does not hash itself" invariant, and the detailed persisted shape does not match the spec for nested sections.

## Verification Notes

I attempted targeted Level 1 test runs:

- `cargo test -p darkmatter hash:: --color=never`
- `cargo test -p darkmatter-cli test_hash_ --test cli --color=never`

Both were still compiling / waiting on Cargo locks after roughly a minute, so I stopped them per the non-interactive session guidance. No test result is claimed from those runs.
