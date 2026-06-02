---
agent: codex
model: ""
ready: false
---

# Review: Ternary-Conditional Commands in Frontmatter Shell Expansion

## Findings

### High: resolved-condition punctuation can become executable branch content

The parser decides that a ternary exists from the original string, but then re-splits the fully interpolated command text and trusts the resolved then/else slices:

- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:202)
- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:215)

The comment says the resolved inner must preserve structure, but the code only checks that `split_top_level_ternary(inner_command)` returns `Some`; it does not prove that the `?` / `:` pair is the same syntactic pair from the original expression. That means interpolation in the condition can introduce top-level `?` or `:` and shift the resolved branch boundaries.

Example shape:

```yaml
cond: "true ? date : false"
out: "$({{cond}} ? basename README.md : '')"
```

After frontmatter interpolation, the resolved inner can be split as:

```text
condition = "true"
then      = "date"
else      = "false ? basename README.md : ''"
```

`date` came from the interpolated condition text, not from either original branch. The executable-interpolation check still validates only `basename README.md` from the original then-branch, so the core invariant from the spec is broken: every command that could execute is no longer statically determinable from the branch AST captured at parse time.

Suggested fix: parse the ternary AST from the original source and preserve those branch boundaries. Interpolate/evaluate the condition separately, and interpolate branch command text per original branch slice before tokenizing it. Do not split the already-interpolated whole `$(...)` body to recover branch boundaries.

Verification level present: Level 1 parser/execution tests only. Add Level 1 compose or parser tests where the condition interpolation contains top-level `?` / `:` and assert that no command from condition text can become a branch executable.

### High: computed false conditions can select the then-branch

Ternary execution evaluates `condition_source` against a seed state built from the current frontmatter after the first interpolation pass:

- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:661)
- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:740)
- [expression/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/expression/mod.rs:203)

Frontmatter interpolation rewrites templated values as `Value::String`, so a computed boolean such as `has_spec: "{{ false }}"` becomes the string `"false"`. `is_truthy(Value::String("false"))` is true because only empty strings are falsy. For the motivating form:

```yaml
has_spec: "{{ false }}"
spec_file: "$({{has_spec}} ? basename '{{spec}}' : '')"
```

the condition source is `{{has_spec}}`, lookup returns `"false"`, and the then-branch is selected.

Suggested fix: evaluate the resolved condition expression itself when it is a literal `true` / `false`, or preserve typed expression results for condition evaluation. At minimum, add an explicit test for `has_spec: "{{ false }}"` selecting the empty branch.

Verification level present: Level 1 in-process tests cover literal `false` and a brace-wrapped true condition, but not a brace-wrapped false condition produced by frontmatter interpolation. Level 1 is the correct level for this non-terminal behavior, but the requirement is not covered.

### Medium: nested or multiple top-level ternaries are accepted instead of rejected

The spec explicitly lists nested ternaries as a v1 non-goal and the requested tests include rejecting multiple top-level `?`. The splitter takes the first top-level `?` and first later top-level `:`:

- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:307)

It does not reject an additional top-level `?` or `:` in either branch. Inputs like `$(a ? echo one : b ? echo two : echo three)` can be parsed as a ternary whose else-branch is just a pipeline with punctuation tokens, instead of producing a clear parse error.

Suggested fix: after splitting, reject any top-level `?` or unmatched ternary separators in `then` or `else` branch text before tokenization.

Verification level present: Level 1 tests cover missing `:`, but not multiple top-level `?` / nested ternaries. Add Level 1 parser tests for nested then-branch and else-branch ternaries.

### Medium: no end-to-end compose or CLI coverage for the motivating workflow

Most new tests call `parse_shell_value` or `execute_frontmatter_shell_expansion` directly, often with no `pre_interpolation_snapshot`:

- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1683)
- [frontmatter_shell_expansion.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:1868)

That misses the important integration path: first frontmatter interpolation, original snapshot capture, frontmatter shell expansion, and second interpolation. This feature is specifically about the interaction between those stages.

Suggested fix: add Level 1 compose-level tests, and preferably one CLI smoke test, for the exact motivating YAML shape with both true and false outcomes. No Level 2 or Level 3 terminal tests are required because this feature has no terminal rendering or keyboard-input behavior.

## Production Readiness

Not ready for production. The implementation currently violates the static-command invariant when condition interpolation introduces ternary punctuation, and it can select the wrong branch for computed false conditions.
