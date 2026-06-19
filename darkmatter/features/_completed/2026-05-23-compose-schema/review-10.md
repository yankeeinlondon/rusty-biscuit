---
ready: false
agent: codex
model: ""
---

# Review 10

## Findings

### High: `darkmatter` does not compile

- Location: `darkmatter/lib/src/markdown/schemas/coerce.rs:21` and `darkmatter/lib/src/markdown/schemas/simplified/convert.rs:38`
- Requirement: the feature cannot be production-ready unless the affected package builds and its compose/schema tests can run.
- Current behavior: `cargo test -p darkmatter compose::cache::hashing::tests::options_hash_sensitive_to_baseline_schema --color=never` fails during compilation with `E0603: constant BOOLISH_VALUES is private`. `coerce.rs` imports `super::simplified::convert::BOOLISH_VALUES`, but `convert.rs` declares it as `pub(super)`.
- Impact: no darkmatter library tests or downstream CLI tests can execute in this worktree, including the tests intended to verify compose schema validation.
- Suggested fix: make the constant visible to the `schemas::coerce` module, or move the shared boolish spellings to a module intentionally shared by `convert` and `coerce`.

### Medium: compose pipeline diagram documents the new stage as the wrong operation

- Location: `darkmatter/docs/darkmatter-compose-pipeline.md:15`
- Requirement: the spec adds Schema Validation immediately after Frontmatter Interpolation and calls out docs updates for the compose pipeline.
- Current behavior: the diagram node is named `schemaValidation`, but its visible label is `2. Frontmatter Shell Expansion` and it links to `./topics/schema-validation.md`. The actual new file is `darkmatter/docs/inline/schema-validation.md`, so the link is broken and the diagram shows two step-2 shell expansion nodes.
- Impact: the implementation path is correct, but the public compose pipeline documentation misrepresents the feature and sends readers to a missing page.
- Suggested fix: change the node to link to `./inline/schema-validation.md`, label it `2. Schema Validation`, and renumber the following inline-pre stages so Frontmatter Shell Expansion becomes step 3.

### Medium: darkmatter skill hash was not regenerated after the skill content changed

- Location: `.claude/skills/darkmatter/SKILL.md:4`
- Requirement: the spec explicitly requires updating `.claude/skills/darkmatter/SKILL.md` and regenerating the `hash:` frontmatter via `md hash`.
- Current behavior: the file still declares `hash: 38ce971c4dbb3efe-c7fdc5b66e6654df`, but `md hash .claude/skills/darkmatter/SKILL.md` reports `35d0453b98d01a42-c7fdc5b66e6654df`.
- Impact: the skill catalog metadata is stale. That can make downstream skill verification think the darkmatter workflow instructions are out of sync with their content.
- Suggested fix: update the `hash:` frontmatter to the value emitted by `md hash`.

## Verification Level Review

- Schema validation fail-fast before frontmatter shell expansion: covered at Level 1 by in-process compose tests and CLI integration tests in `darkmatter/cli/tests/compose_schema.rs`. This is appropriate because the requirement is process behavior, not terminal input encoding.
- Override and baseline interactions: covered at Level 1 by `schema_validation.rs`, compose integration tests, and option-hash/cache tests. This is appropriate for API and cache-key behavior.
- Recursive child validation with parent `set=` overlays: covered at Level 1 by compose integration tests. This is appropriate because the behavior is data-flow and error propagation.
- Styled `SchemaValidationFailed` block rendering: covered at Level 1 by block-rendering tests and at Level 2 by `darkmatter/cli/tests/level2_errors.rs`, which drives `md compose` in a real terminal and checks OSC8/styling. This matches the user-observable terminal-rendering requirement.
- Level 3 is not required for this feature. The spec does not define keyboard, paste, mouse, IME, modifier-key, or terminal input-encoder behavior.

## Notes

Apart from the package-level compile blocker, I did not find a functional gap in the Rust compose-stage placement, baseline API, cache-key handling, or schema-validation failure path. The remaining compose-schema-specific blockers are documentation/metadata drift against explicit feature requirements.

## Local Verification

- Failed: `cargo test -p darkmatter compose::cache::hashing::tests::options_hash_sensitive_to_baseline_schema --color=never`
- Result: compile failed before test execution with `E0603` because `BOOLISH_VALUES` is private.
- Not completed: the broader `cargo test -p darkmatter schema_validation --color=never` command was started concurrently by mistake, then stopped while it was still waiting for Cargo's package-cache lock.
