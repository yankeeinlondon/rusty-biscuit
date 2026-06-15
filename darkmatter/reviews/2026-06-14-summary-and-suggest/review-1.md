---
agent: codex
model: ""
ready: false
---

# Review: Summary and Suggest Follow-up

## Findings

### High: `md code-block --output markdown --title ...` without a language does not round-trip metadata

The new Markdown serialization helper emits metadata immediately after the opening fence even when no language is present:

- `darkmatter/cli/src/commands/code_block.rs:171`
- `darkmatter/cli/src/commands/code_block.rs:175`
- `darkmatter/cli/src/commands/code_block.rs:204`
- `darkmatter/cli/src/commands/code_block.rs:276`

That produces output like:

```markdown
```title="Untitled"
x
```
```

But the library parser treats the first whitespace-delimited token as the language and only parses key/value metadata from the remainder:

- `darkmatter/lib/src/markdown/dsl/parser.rs:69`
- `darkmatter/lib/src/markdown/dsl/parser.rs:71`
- `darkmatter/lib/src/markdown/dsl/parser.rs:79`

So re-reading that fence makes `title="Untitled"` the language token, not the title metadata. This violates the fix requirement for an "empty language with metadata" case and the helper's own "safe round-trip" contract.

Verification level: Level 1 CLI/module tests exist, but they assert the broken output instead of verifying parse/render round-trip semantics. Raw Markdown output does not need Level 2/3 terminal verification.

SOLUTION: the `Language` struct should _fallback_ to `txt`/`text` as the format when there is not match available through the lookup functions provided by 

### Medium: `ComposeOperation::default_order()` is still a second source of truth

The descriptor table was added, but `default_order()` remains a separate hand-written array:

- `darkmatter/lib/src/markdown/compose/types.rs:147`
- `darkmatter/lib/src/markdown/compose/types.rs:154`
- `darkmatter/lib/src/markdown/compose/types.rs:389`
- `darkmatter/lib/src/markdown/compose/types.rs:390`

The invariant test at `darkmatter/lib/src/markdown/compose/types.rs:3146` catches drift, but the spec asked for operation metadata and default order to be centralized so they "cannot silently diverge." This implementation still requires updating both the descriptor table and a manual list when adding or reordering an operation. The API should derive the default order from `COMPOSE_OPERATION_DESCRIPTORS` filtered by `default_enabled`, or otherwise expose a descriptor-derived static order.

Verification level: Level 1 invariant tests are present and pass, but they verify duplication rather than remove it.

### Low: Compose phase docs still say "three phases" in the skill reference

The main skill file and module docs describe the four-phase pipeline, but `.claude/skills/darkmatter/compose.md` still opens with "three phases" while listing Finalization later:

- `.claude/skills/darkmatter/compose.md:3`
- `.claude/skills/darkmatter/compose.md:34`

The structure reference also still summarizes `compose/` as "Inline Pre + Transclusion + Inline Post":

- `.claude/skills/darkmatter/structure.md:19`

This misses the spec's documentation acceptance criterion for consistently describing four phases and root-only finalization.

## Test Notes

Focused checks run:

- `cargo test -p darkmatter --lib markdown::dsl --color=never` passed.
- `cargo test -p darkmatter --lib compose_operation --color=never` passed.
- `cargo test -p darkmatter-cli code_block_markdown --color=never` passed.

I did not run the full `just test` package-area suite.

## Production Readiness

Not ready. The maintenance cleanup is mostly in place, but the no-language metadata Markdown output is user-observable and currently not semantically safe to re-read, and the compose descriptor work has not fully removed the duplicate default-order source.
