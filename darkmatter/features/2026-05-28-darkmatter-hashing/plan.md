---
phases: 8
created: 2026-05-28
start_phase: 1
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1:
  - darkmatter/features/2026-05-28-darkmatter-hashing/design.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/hash/mod.rs
  - darkmatter/lib/src/markdown/hash/kind.rs
  - darkmatter/lib/src/markdown/hash/options.rs
  - darkmatter/lib/src/markdown/hash/compute.rs
  - darkmatter/lib/src/markdown/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/hash/stored.rs
  - darkmatter/lib/src/markdown/hash/mod.rs
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/hash/kind.rs
  - darkmatter/lib/src/markdown/hash/compare.rs
  - darkmatter/lib/src/markdown/hash/save.rs
  - darkmatter/lib/src/markdown/hash/stored.rs
  - darkmatter/lib/src/markdown/hash/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/hash/write.rs
  - darkmatter/lib/src/markdown/hash/mod.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/hash/explain.rs
  - darkmatter/lib/src/markdown/hash/mod.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
source_files_during_phase_7:
  - darkmatter/cli/Cargo.toml
  - darkmatter/cli/src/args.rs
  - darkmatter/cli/src/commands.rs
  - darkmatter/cli/tests/cli.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - darkmatter/docs/cli/hash.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/darkmatter/SKILL.md
packages:
  - darkmatter
---

# Darkmatter Hashing Execution Plan

## Success Criteria

- [ ] `md hash <file>` preserves current default behavior by printing a `simple` `{frontmatter-hash}-{body-hash}` value and exiting `0`.
- [ ] The library exposes deterministic hashing APIs driven only by explicit options; the CLI owns environment and flag parsing.
- [ ] Hashing supports `fm`, `body`, `simple`, `structured`, and `detailed` kinds using `biscuit-hash` xxHash.
- [ ] Stored `hash` and `last_updated` fields are always ignored during hash computation, with exact key matching.
- [ ] `--save` writes canonical frontmatter plus byte-for-byte unchanged body content and follows the `last_updated` rules in the spec.
- [ ] `--diff` reports differences without writing and exits `2` when differences are found.
- [ ] Existing `--body`, `--frontmatter`, `--strict`, and directory aggregate behavior remains compatible unless explicitly superseded by the new kind API.

## Phase 1: Baseline Discovery And API Design

- [ ] Inspect `darkmatter/lib/src/markdown/hash.rs`, `darkmatter/lib/src/markdown/frontmatter.rs`, `darkmatter/lib/src/markdown/output/string.rs`, `darkmatter/cli/src/args.rs`, and `darkmatter/cli/src/commands.rs` to confirm the current public hash behavior, frontmatter mutation APIs, and save/write patterns.
- [ ] [parallelizable] Identify whether existing delta or normalize code can safely provide heading extraction, section splitting, or section alignment; document any reuse decision in the implementation notes or code comments only where it explains hidden coupling.
- [ ] Define the new library API surface before coding: `MdHashKind`, hash options, stored-hash parsing type, computed-hash type, comparison result type, save result type, and rendered explanation type.
- [ ] Decide whether the expanded implementation stays in `markdown/hash.rs` or moves to `markdown/hash/` submodules; prefer submodules if the detailed comparison logic would make a single file hard to review.
- [ ] Confirm how current `--body` and `--frontmatter` map onto `MdHashKind::Body` and `MdHashKind::Fm`, including conflicts or precedence with the new `--kind` flag.
- [ ] Validation checkpoint: write down the intended library/CLI boundary in code-level terms before implementation begins: library receives explicit property names, ignored extras, forced kind, and mode; CLI reads `HASH_PROPERTY`, `HASH_IGNORE_PROPERTIES`, `--kind`, `--save`, and `--diff`.

## Phase 2: Core Hash Model

- [ ] Add `MdHashKind` with clap/value parsing support at the CLI layer and serde/string parsing support at the library layer for `fm`, `body`, `simple`, `structured`, and `detailed`.
- [ ] Add hash options that include the hash property name, extra ignored properties, forced kind, strictness, and mode-independent defaults.
- [ ] Implement exact-key ignore-set construction with always-ignored managed keys: the active hash property and `last_updated`.
- [ ] Update frontmatter hashing so it can compute against a filtered frontmatter map without mutating the source document.
- [ ] [parallelizable] Implement `fm` and `body` computed values as single 16-digit lowercase hex strings.
- [ ] Preserve current `simple` value shape as `{fm-hash}-{body-hash}`.
- [ ] Implement `structured` value shape as `{fm-hash}-{fm-keys-hash}-{body-hash}-{body-structure-hash}`.
- [ ] [parallelizable] Implement frontmatter key-structure hashing by sorting or otherwise stabilizing keys consistently with existing non-strict frontmatter hashing, while ignoring values.
- [ ] [parallelizable] Implement body structure hashing by extracting headings in document order and hashing the concatenated heading lines including the markdown heading level prefix.
- [ ] [parallelizable after body section extraction exists] Implement `detailed` computed values with `frontmatter`, nullable `preamble`, and ordered `sections` tuples of `[level-num, heading, content-hash]`.
- [ ] Ensure absent or empty frontmatter hashes exactly as empty frontmatter, including `md hash --kind fm` on documents without a frontmatter block.
- [ ] Validation checkpoint: add focused library unit tests for every computed kind using stable expected shapes and at least one whitespace-only body change that must not alter non-strict semantic hashes.

## Phase 3: Stored Hash Parsing And Persistence Shapes

- [ ] [parallelizable] Parse shorthand string stored hashes as `simple` with no extra ignored properties.
- [ ] [parallelizable] Parse longhand stored hashes with `kind`, `value`, and optional `ignored`.
- [ ] [parallelizable after detailed computed shape exists] Parse `detailed.value` as a nested YAML object with `frontmatter`, nullable `preamble`, and section tuples.
- [ ] Reject malformed stored hash objects through the existing error path with messages actionable enough for CLI users.
- [ ] Implement serialization for stored hashes, preserving the shorthand invariant: use a string if and only if kind is `simple` and there are zero extra ignored properties.
- [ ] Implement longhand serialization for `simple` with ignored extras, and for all `fm`, `body`, `structured`, and `detailed` values.
- [ ] Sort `ignored` extras before serialization and omit `ignored` entirely when the extra list is empty.
- [ ] Ensure the stored `ignored` list contains only extra ignored properties, never the active hash property or `last_updated`.
- [ ] Validation checkpoint: add round-trip tests for shorthand `simple`, longhand `simple` with `ignored`, `structured`, `fm`, `body`, and nested-object `detailed`.

## Phase 4: Kind Selection, Comparison, And Save Semantics

- [ ] Implement kind selection: forced `--kind` wins; otherwise match the document's stored hash kind; otherwise default to `simple`.
- [ ] Implement the kind resolution ordering: `simple < structured < detailed`, with `fm` and `body` lower than `simple` but incomparable to each other.
- [ ] Compare same-kind hashes using the stored ignore-set, not the current environment ignore-set.
- [ ] Implement higher-resolution save behavior by comparing at the old lower resolution before upgrading the stored value.
- [ ] Implement lower-resolution save behavior by recomputing the forced lower kind and comparing it against the corresponding components of the stored higher-resolution value.
- [ ] Implement incomparable kind-switch save behavior by rewriting the hash at the forced kind without treating the switch itself as a content change.
- [ ] Implement no-stored-hash behavior: `--diff` reports `No stored hash to compare against` and exits `2`; `--save` writes the first baseline and exits `0`.
- [ ] [parallelizable] Implement ignore-policy advisory detection when the stored ignored extras differ from the current extras.
- [ ] Ensure ignore-policy-only changes rewrite `hash.ignored` and recompute `value` under the new ignore-set without bumping `last_updated`.
- [ ] Ensure content changes update the stored hash and set `last_updated` to the current local date in `YYYY-MM-DD` format.
- [ ] Ensure no-change comparisons leave the file untouched in `--save`.
- [ ] Validation checkpoint: add library tests for same-kind no-op, same-kind content change, higher-resolution upgrade with and without lower-resolution change, lower-resolution downgrade with and without lower-resolution change, incomparable kind switch, and ignore-policy-only rewrite.

## Phase 5: Write-Back Path

- [ ] Add a library method that mutates the frontmatter model for save operations and returns serialized Markdown for the CLI to write.
- [ ] Reuse `Markdown::as_string` or the same serialization primitive as `md clean --save` for canonical frontmatter output.
- [ ] Preserve body content byte-for-byte from the parsed `Markdown::content()` during hash save; do not run cleanup or body normalization before writing.
- [ ] Verify that adding the first hash to a document without frontmatter creates a valid frontmatter block and leaves body bytes unchanged after the closing delimiter.
- [ ] Verify that updating an existing hash preserves frontmatter key order as much as the current `IndexMap` model allows.
- [ ] Keep CLI file writing in `darkmatter/cli/src/commands.rs`, matching the existing `run_clean` responsibility split.
- [ ] Validation checkpoint: add tests that compare the saved body segment to the original body segment exactly, including irregular spacing, code fences, and trailing newlines.

## Phase 6: Difference Explanation Engine

- [ ] [parallelizable] Implement simple explanations for unchanged, frontmatter-only, body-only, and both-changed states.
- [ ] [parallelizable] Implement `fm` explanations with only the frontmatter concern and `body` explanations with only the body concern.
- [ ] [parallelizable after structured comparison data exists] Implement structured explanations that distinguish frontmatter key changes from value-only changes and body structural changes from semantic content changes.
- [ ] [parallelizable] Implement detailed frontmatter explanations at the same resolution as structured.
- [ ] [parallelizable after detailed computed shape exists] Implement detailed body comparison for preamble added, removed, changed, and unchanged cases.
- [ ] Implement detailed section alignment in the specified order: heading match, content-hash match, same-level positional match, then leftover added/removed classification.
- [ ] Implement detailed section classifications for unchanged, content-changed, renamed, renamed-and-edited, promoted, demoted, reordered, moved, added, and removed.
- [ ] Render detailed body reports as nested lists with changed sections in document order and removed sections named from stored heading text.
- [ ] Keep the new explanation implementation separate from the legacy `Markdown` delta descriptions unless a reusable internal alignment helper is clearly sound.
- [ ] Validation checkpoint: add snapshot or exact-string tests for representative simple, `fm`, `body`, structured, and detailed messages from the spec.

## Phase 7: CLI Integration

- [ ] Add `--kind <fm|body|simple|structured|detailed>` to `md hash`.
- [ ] Add `--save` to `md hash`, conflicting with `--diff`.
- [ ] Add `--diff` to `md hash`, conflicting with `--save`.
- [ ] [parallelizable] Wire `HASH_PROPERTY` parsing in the CLI with default `hash`.
- [ ] [parallelizable] Wire `HASH_IGNORE_PROPERTIES` parsing in the CLI as CSV extras, trimming whitespace and dropping empty entries.
- [ ] Ensure `HASH_IGNORE_PROPERTIES` is additive and cannot un-ignore the active hash property or `last_updated`.
- [ ] Enforce flag interactions with clap, including `--save` conflicting with `--diff` and any decided conflict between `--kind` and legacy `--body` / `--frontmatter`.
- [ ] Update `run_hash` to call the new library API and keep business logic out of the CLI.
- [ ] Preserve bare `md hash` output and exit code `0`.
- [ ] Make `--save` print explanatory output instead of raw hash output and exit `0` on successful operation whether or not it wrote the file.
- [ ] Make `--diff` print explanatory output and exit `2` when differences are detected.
- [ ] Keep operational errors on the existing eyre path with exit code `1`.
- [ ] Decide whether directory mode supports only bare hashing initially or all new modes; if restricted, fail invalid combinations early with a usage error and document the behavior in tests.
- [ ] Validation checkpoint: add CLI tests or command-level tests for bare hash, forced kind, environment property override, extra ignored properties, save, diff, mutually exclusive flags, and exit codes `0`, `1`, and `2`.

## Phase 8: Documentation, Compatibility, And Final Verification

- [ ] [parallelizable after CLI surface is stable] Update user-facing CLI docs or README sections that describe `md hash`, including `--kind`, `--save`, `--diff`, `HASH_PROPERTY`, and `HASH_IGNORE_PROPERTIES`.
- [ ] [parallelizable after library API is stable] Update Rustdoc for changed public library APIs, following the repo convention: summary first, then `Examples`, `Returns`, `Errors`, `Panics`, `Safety`, and `Notes` only when useful.
- [ ] Review all edited comments and docs for drift, especially where existing hash docs still imply only two hash modes.
- [ ] Confirm whether `darkmatter/docs/dependencies.md` or repo dependency docs need updates; only edit them if crates are added or removed.
- [ ] Run focused library tests for hashing modules.
- [ ] Run focused CLI tests for hash command behavior and exit codes.
- [ ] Run a package-level validation command for `darkmatter` that matches the repo's testing skill guidance, without running `cargo fmt` unless explicitly requested.
- [ ] Manually exercise `md hash`, `md hash --kind structured`, `md hash --diff`, and `md hash --save` on temporary Markdown fixtures.
- [ ] Validation checkpoint: verify `git diff` contains only intended hashing, CLI, test, and documentation changes, and that no unrelated formatting or comment-only cleanup was mixed into behavior changes.
