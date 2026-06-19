# Integrating with Darkmatter's Shell Expansion API

This document records how Claudine's harness shell validator was updated to track the [Better Shell Command Parsing](../../darkmatter/features/2026-05-07-better-shell-cmd-parsing/spec.md) work in Darkmatter, and the architectural reasoning behind the chosen integration boundary.

## Background

Darkmatter's `shell_expansion` module previously rejected shell metacharacters (`>`, `<`, `|`, `;`, `&&`, `||`) at tokenization time. The "Better Shell Command Parsing" feature lifts that restriction for a curated set of operators so users can write practical commands like `npm install && npm run build` and `cmd > /dev/null` inside `::shell` directives and `$(...)` frontmatter expansions.

The new design replaces the old `tokenize() -> Vec<String>` API with a structured token stream and a pipeline parser:

| Old API                                  | New API                                         |
| ---------------------------------------- | ----------------------------------------------- |
| `tokenize(&str) -> Vec<String>`          | `tokenize(&str) -> Vec<ShellToken>`             |
| _(no equivalent — chains were rejected)_ | `parse_pipeline(&[ShellToken]) -> ShellPipeline` |
| _(implicit)_                             | `tokenize_simple(&str) -> Vec<String>`          |

Where `ShellToken` is:

```rust
pub enum ShellToken {
    Word(String),
    And,                       // &&
    Or,                        // ||
    RedirectStdoutNull,        // > /dev/null
    RedirectStderrNull,        // 2> /dev/null
    RedirectStderrToStdout,    // 2>&1
    RedirectStdoutToStderr,    // >&2
}
```

Two related shape changes also landed in Darkmatter:

- `Expr` (the compose expression AST) gained `BoolLiteral(bool)` and `Paren(Box<Expr>)` variants.
- `ShellApprovalRequest` gained a `chain_executables: Vec<String>` field that lists the unique executables in a chain so an approval handler can show the user the full chain context. For non-chained requests this list is empty.

## What Broke in Claudine

Compiling Claudine against the new Darkmatter surface produced seven errors across four files:

1. **`composition/preflight.rs:103`** — `tokenize(normalized)` now returns `Vec<ShellToken>`, but the call site assigned it to `Vec<String>`.
2. **`harness/parse/overlays.rs:80–81`** — Frontmatter `shell_command` parsing destructured `tokens[0]` as the executable string and `tokens[1..]` as args; both now have type `ShellToken`, not `String`.
3. **`harness/shell.rs:73`** — `validate_and_approve_command` passed its tokens to `validate_and_approve_command_parts(&tokens, …)`, which expects `&[String]`.
4. **`harness/shell.rs:204`** — `ShellApprovalRequest` constructor was missing the new `chain_executables` field.
5. **`dispatch/matcher.rs:97`** — `expression_uses_known_features` matched on `Expr` exhaustively; the new `BoolLiteral` and `Paren` variants were uncovered.

## Integration Boundary: Who Owns Chain Handling?

Before deciding how to fix these, the key question was: should Claudine's harness validator gain its own pipeline-aware code path, or trust Darkmatter to handle chains and keep Claudine's harness in single-command mode?

The spec describes chain semantics in §2.3:

> Every command in the chain must be checked against the policy engine. If any command in the chain requires user intervention, the user must be presented with the entire chain of commands for upfront approval.

That requirement is **already implemented inside Darkmatter**, not Claudine. Specifically:

- `darkmatter/lib/src/markdown/compose/shell_expansion/mod.rs` builds a single `ShellApprovalRequest` representing the whole chain (raw command, normalized form joined with ` && `, and `chain_executables` populated with the unique executables in order).
- The approval handler is invoked **once** for the entire chain. On approval, every command in the chain is persisted individually.
- `discovery::collect_shell_commands` emits one `ShellCommandEntry` per `CommandAction` in the parsed pipeline. By the time entries reach Claudine's harness validator, chains have already been split into per-command records.

Claudine's harness validator only sees three classes of input, all of which are single-command by contract:

| Site                                  | Source of input                                                            | Why it's single-command                                                            |
| ------------------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| `composition/preflight.rs:103`        | Iterates `unique` from `collect_shell_commands` + `collect_auditable_commands` | Discovery already split per `CommandAction`                                        |
| `harness/audit.rs:116`                | Iterates already-collected auditable commands                              | Same — caller has a per-command record                                             |
| `harness/parse/overlays.rs`           | Frontmatter `shell_command` field                                          | Decoded into `ApprovedRuntimeCommand { executable: String, args: Vec<String> }` — chains have no representation |
| `harness/shell.rs::validate_and_approve_command` (raw entry) | `&str`                                                          | Returns a single `ApprovedRuntimeCommand`; only used internally and in tests       |

Adding pipeline-aware machinery in Claudine would create a parallel implementation of work Darkmatter already owns, complicate `ApprovedRuntimeCommand`'s shape, cascade through the executor and `fs` validation paths, and split chain-approval policy across two layers. The cleaner integration is for Claudine to **preserve a strict single-command contract at its harness boundary** and explicitly reject chains/redirections that arrive there — which would only happen via misuse or a discovery bug.

## What We Actually Changed

### 1. New helper: `tokenize_words_strict`

Added to `claudine/lib/src/harness/shell.rs`:

```rust
pub(crate) fn tokenize_words_strict(raw: &str) -> Result<Vec<String>, ShellExpansionError> {
    let tokens = tokenize(raw)?;
    let mut words = Vec::with_capacity(tokens.len());
    for tok in tokens {
        match tok {
            ShellToken::Word(w) => words.push(w),
            _ => {
                return Err(ShellExpansionError::ParseDirective {
                    origin: ShellCommandOrigin::Body { line: 0 },
                    message: format!(
                        "chain operators and redirections are not allowed here: {raw}"
                    ),
                });
            }
        }
    }
    Ok(words)
}
```

This deliberately differs from Darkmatter's `tokenize_simple`. `tokenize_simple` _silently filter-maps_ non-`Word` tokens out, which would let a chain like `git status && git push` flatten to `["git", "status", "git", "push"]` — a single nonsense command. `tokenize_words_strict` _hard-errors_ instead, restoring the safety property the old `tokenize` provided implicitly when it rejected metacharacters at the source.

### 2. Routed all single-command call sites through the helper

- **`harness/shell.rs::validate_and_approve_command`** — replaced `tokenize_simple` with `tokenize_words_strict`. Doc comment now states the single-command contract explicitly.
- **`composition/preflight.rs`** — same swap. Previously read `tokenize_simple` from Darkmatter; now imports and uses `tokenize_words_strict` from `harness::shell`.
- **`harness/parse/overlays.rs::tokenize_to_approved_command`** — same swap.

### 3. `ShellApprovalRequest::chain_executables: Vec::new()`

The single approval-request construction site in Claudine (`harness/shell.rs:204`) now populates `chain_executables` with an empty vector. This is correct because requests built at this layer represent single commands — chain-aware requests are constructed inside Darkmatter, never inside Claudine.

### 4. Exhaustive `Expr` match in `dispatch/matcher.rs::expression_uses_known_features`

This function decides whether a parsed `Expr` matcher should run as a Darkmatter condition expression or fall back to regex. The two new variants:

- `BoolLiteral(_)` → returns `true`. A bare `true`/`false` is unambiguously condition-grade and has no useful regex interpretation.
- `Paren(inner)` → recurses into `inner`. Parenthesization itself adds no semantic features; the inner expression decides. This preserves the existing rule that bare `Variable("Bash")` falls back to regex even when wrapped in parens, while `(some_var > 0)` is correctly recognized as a condition.

### 5. Regression test for chain rejection

Added `chain_operators_rejected_at_single_command_slot` in `harness/shell.rs` which asserts that all four operator/redirection forms are denied at the single-command entry point:

```rust
for raw in [
    "git status && git push",
    "make build || echo failed",
    "ls > /dev/null",
    "cmd 2>&1",
] {
    let result = validate_and_approve_command(raw, &options);
    assert!(matches!(result, Err(HarnessError::ShellCommandDenied { .. })));
}
```

The pre-existing `shell_metacharacters_rejected` test (covering `|`) still passes — pipes remain unsupported in the new tokenizer per spec §2.4.

## Why This Alignment Makes Sense

### Single owner for chain semantics

Chain-aware approval is non-trivial: it requires building a normalized representation, listing all executables in dependency order, calling the handler once with the whole chain, and persisting every command on approval. Doing this in two layers would mean two normalization rules, two handler-invocation policies, and two persistence paths to keep in sync. Centralizing it in Darkmatter — where pipelines live as first-class types (`ShellPipeline`, `PipelineAction`, `ChainOperator`) — keeps the invariants in one place.

### Claudine's harness boundary stays simple

`ApprovedRuntimeCommand { raw, executable, args }` is the unit Claudine's executor and `fs` validator already work with. Keeping this shape avoids ripple changes through `execute_approved_command`, `harness/validate/fs.rs`, audit collection, and the harness plan model. The cost of preserving this shape is exactly one strict tokenizer helper; the alternative would be a multi-file restructure.

### `tokenize_words_strict` ≠ `tokenize_simple`

The lenient `tokenize_simple` API is correct for callers that want word tokens and don't care about operators (e.g. extracting executable names for display). It's incorrect for a security boundary, where silently dropping operators turns a chain into a single mangled command and bypasses validation. Strict rejection is the correct posture for Claudine's harness validator.

### Chain inputs at Claudine's boundary indicate a bug

Because Darkmatter's discovery layer always splits chains before emitting `ShellCommandEntry`, a chain string reaching Claudine's harness validator means either:

1. A new caller bypassed discovery (e.g. constructed a raw harness command directly with chain syntax), or
2. A regression in Darkmatter's discovery splitting.

Either way, a hard error surfaces the misuse immediately. Silent flattening would mask both.

### `chain_executables: Vec::new()` is a contract, not a workaround

The empty vector encodes the boundary explicitly: "every approval request Claudine constructs is single-command." If someone in the future adds a Claudine-side path that legitimately needs chain handling, they should not paper over `chain_executables` — they should refactor that path to delegate to Darkmatter's chain-aware approval, or move the chain handling into Darkmatter where the rest of the pipeline machinery lives.

## Files Changed

```
claudine/lib/src/composition/preflight.rs    # tokenize_simple -> tokenize_words_strict
claudine/lib/src/dispatch/matcher.rs         # added BoolLiteral / Paren arms
claudine/lib/src/harness/parse/overlays.rs   # tokenize_simple -> tokenize_words_strict
claudine/lib/src/harness/shell.rs            # new helper, swapped call sites,
                                             # chain_executables: Vec::new(),
                                             # regression test
```

No changes to `ApprovedRuntimeCommand`, `ShellApprovalOptions`, the executor, or any caller outside the four files above. Public API of `claudine::harness::shell` gains exactly one item: `pub(crate) fn tokenize_words_strict`.

## Future Work

Items deliberately out of scope for this integration:

- **Surfacing chain support in Claudine's harness model.** If a future feature wants Claudine harness commands (not Darkmatter `::shell` directives) to support `&&`/`||`, the right path is to thread Darkmatter's `ShellPipeline` through the harness plan model, not to re-implement chain handling locally. The strict rejection in `tokenize_words_strict` will be the natural place that future feature opts out of.
- **Richer error context.** `tokenize_words_strict` returns `ShellExpansionError::ParseDirective` with `line: 0` because the helper has no way to recover the source line from a raw string. Callers that have provenance (e.g. `validate_and_approve_command_parts` with `source_line: Some(_)`) could wrap their own errors with that context — left unchanged here to match existing behavior.
