---
ready: true
agent: codex
model: ""
---

# Review 3

## Findings

### High: ternary conditions do not use the condition-mode expression grammar

Spec lines 62 and 68 say the ternary condition is evaluated by the existing expression engine, including `&&`, `||`, `!`, comparisons, and `?:`. The implementation interpolates the condition and then calls the normal parser:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:948`

The normal `parse(...)` entrypoint does not enable condition-mode infix logical operators; `parse_condition(...)` is the entrypoint used by the existing conditional content path. As a result, a valid spec-level condition such as:

```yaml
out: "$(has_spec && enabled ? basename {{spec}} : '')"
```

will fail to parse instead of selecting a branch. This is a designed user-facing behavior that is not implemented.

Suggested fix: switch ternary condition parsing to `parse_condition(&expression_text)` or otherwise route through the same condition-mode parser used by page/transclusion conditions. Add Level 1 compose tests for `&&`, `||`, `!`, comparisons, and an expression ternary in the condition.

Verification level present: Level 1 tests cover simple truthy/falsy values and stringified booleans, but there is no Level 1 coverage for condition-mode logical operators. Level 1 is sufficient for this parser/evaluator requirement.

### High: shell-command discovery does not flatten ternary branch commands

`parse_shell_value` intentionally returns ternary directives with `executable = ""`, `args = []`, and `pipeline = None`:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:229`

The shell discovery path still converts every frontmatter candidate back into a plain `ShellDirective` and then calls `directive_action_iter`, which only understands `ShellDirective.pipeline`:

- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:456`
- `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:472`

For a frontmatter ternary like `out: "$(flag ? echo yes : basename {{file}})"`, discovery will not emit the reachable `echo` and `basename` commands. It will instead operate on the empty legacy fields from the ternary placeholder. That breaks the preflight approval/discovery surface for the new feature and makes the allowlist workflow incomplete even though execution later prepares both branches.

Suggested fix: expose a helper from frontmatter shell expansion that returns every reachable branch pipeline/action after the same safe interpolation/shape validation used at execution time, or teach discovery to match `FrontmatterShellAst::Ternary` directly. Add Level 1 discovery tests proving both branch commands are emitted, empty branches emit nothing, branch chains emit one entry per action, and interpolated branch arguments are resolved without allowing executable interpolation.

Verification level present: existing Level 1 discovery tests cover plain frontmatter `$()` and chained pipelines, but none cover ternary frontmatter. Level 1 is sufficient for this command-discovery behavior.

### Medium: ternary separator detection is stricter than the spec

The spec says ternaries are detected by scanning for an unquoted, top-level `?` followed by an unquoted, top-level `:`:

- `darkmatter/features/2026-05-26-ternary-conditional-commands/spec.md:69`
- `darkmatter/features/2026-05-26-ternary-conditional-commands/spec.md:129`

The implementation only treats `?` and `:` as separators when they are padded by spaces or tabs:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:337`
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:365`

That avoids false positives for URLs and glob-like arguments, but it also silently narrows the accepted ternary syntax. For example, `$(flag? echo yes : '')` contains a top-level ternary by the spec but is parsed as a plain pipeline with executable `flag?`.

Suggested fix: either implement the spec's literal top-level separator rule with a clearer branch parser, or update the spec to require whitespace-padded ternary separators and add parser tests documenting that contract.

Verification level present: Level 1 tests cover whitespace-padded separators and punctuation inside quoted strings/URLs. They do not cover unpadded top-level separators or lock in the narrower whitespace contract. Level 1 is sufficient here.

## Test Rigor

This feature is frontmatter parsing, expression evaluation, shell allowlist validation, shell command discovery, and command execution. There are no terminal rendering, keypress, paste, IME, mouse, or real-terminal encoder/decoder requirements in the spec, so Level 2 and Level 3 tests are not required for production readiness.

The current strongest relevant tests are Level 1. That is the right level for the feature, but coverage is incomplete for condition-mode operators and ternary shell-command discovery.

## Verification

I attempted:

```bash
cargo test -p darkmatter ternary --lib --color=never
```

It was still compiling after roughly 60 seconds, so I stopped it per the non-interactive session guidance. No completed test result is available from this review pass.

## Production Readiness

Not ready for production. The implementation covers the main execution path, but it misses a specified condition grammar requirement and leaves the preflight shell-command discovery/approval workflow unaware of ternary branch commands.
