---
created: 2026-05-26
status: draft
area: darkmatter
component: frontmatter-shell-expansion
---

# Ternary-Conditional Commands in Frontmatter Shell Expansion

## Problem

Darkmatter's frontmatter shell expansion (`$(cmd)` in frontmatter values) refuses any directive where template interpolation `{{var}}` appears in the *executable position* of any pipeline segment. The check is enforced by `validate_no_executable_interpolation()` in `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:139`. It is necessary — the shell allowlist (`.darkmatter-shell-whitelist`) can only protect callers if the command name is statically knowable.

The check is currently implemented as a textual rule: *"if any executable-position token contains `{{ }}`, reject."* That rule is **too coarse** for a real and common authoring case: a value that should run *one of a small, fixed set of commands* depending on a boolean (often "does this optional variable have a value?").

The motivating example, from `prompts/implement.md`:

```yaml
spec_file: "$({{has_spec}} ? basename '{{spec}}' : '')"
```

Here:

- `{{has_spec}}` is a template-resolvable boolean — it never reaches the shell.
- The *only* command that can run is `basename`. The else-branch is a literal empty string and invokes nothing.
- Every reachable executable is statically known and allowlistable.

The current parser sees `{{has_spec}}` sitting where it expects an executable name and aborts with:

```
Shell directive parse error at frontmatter.spec_file:
  Frontmatter shell executable may not come from interpolation
```

Authors then fall back to brittle shell ternaries (`$(test -n '{{x}}' && basename '{{x}}' || echo '')`) which are harder to read, harder to audit, and trade one allowlist entry (`basename`) for three (`test`, `basename`, `echo`).

## Invariant Being Protected

The existing rule is a proxy for the real invariant. The real invariant is:

> **Every command that the directive could execute must be statically determinable at parse time, and every such command must be allowlisted.**

A bare `$({{cmd}} arg)` violates this — the resolved value of `{{cmd}}` is not knowable at parse time. A ternary `$({{cond}} ? basename arg : '')` does **not** violate this — the reachable command set is `{basename}` regardless of what `{{cond}}` resolves to.

This spec re-states the parser's job in terms of that invariant.

## Goals & Non-Goals

**Goals**

- Add a ternary form `COND ? THEN_CMD : ELSE_CMD` inside `$(...)` frontmatter values.
- Allow each branch to be either a command pipeline (subject to the existing executable-literal rule) **or** a string literal (`''` / `""`) that contributes no command.
- Allow the condition position to contain template interpolation (`{{var}}`). The condition is evaluated by the existing template expression engine before shell execution; it never reaches the shell.
- Validation walks the ternary AST and rejects a directive only if a *reachable* leaf has an unresolved-interpolation executable or an off-allowlist command name.
- Preserve every existing rejection. Today's failing inputs (e.g. `$({{cmd}} arg)`) remain failures.

**Non-Goals (v1)**

- **Nested ternaries** (`COND1 ? A : COND2 ? B : C`). v1 supports a single top-level ternary inside `$(...)`. Authors needing chained conditions write multiple frontmatter keys.
- **Ternary inside a command argument.** Only the top-level shape `$(EXPR ? CMD : CMD)` is supported. `$(echo {{x ? "a" : "b"}})` already works through template interpolation and is out of scope.
- **`$(...)` nested inside a branch.** Branches are bare command pipelines (no outer `$(`); the surrounding `$(` is the single shell-entry boundary.
- **Boolean operators inside the condition.** The condition is a single boolean expression evaluated by the existing template engine, which already supports `&&`, `||`, `!`, comparisons, and `?:`. No new condition syntax is added here.
- **Replacing the `&&` / `||` chain operators.** Pipelines inside each branch continue to support `&&`/`||` exactly as today.

## Foundational Decisions

- **Decision #1** — The shell-directive parser gains an AST. The current shape becomes one of two variants: `Pipeline(ShellPipeline)` or `Ternary { condition: String, then: Branch, else: Branch }` where `Branch ∈ { Empty, Pipeline(ShellPipeline) }`.
- **Decision #2** — The condition string is the **original** (pre-interpolation) text of the condition expression, captured before the frontmatter-interpolation pass runs. The shell-expansion stage evaluates it through `darkmatter::compose::expression::evaluate` using the same lookup the interpolation pass uses, then applies `is_truthy()`.
- **Decision #3** — The ternary is detected by scanning the inner text of `$(...)` for an unquoted, top-level `?` followed somewhere later by an unquoted, top-level `:`. "Top-level" means: not inside single quotes, double quotes, or a parenthesized sub-expression; not preceded by a backslash. If no such pair is found, the directive parses as a plain pipeline (today's path).
- **Decision #4** — Each branch is parsed independently. The empty-string literal (`''` or `""`, with optional surrounding whitespace) is the only non-pipeline branch form and produces `Branch::Empty`. Anything else is fed to `parse_pipeline` as today.
- **Decision #5** — The executable-interpolation check (`validate_no_executable_interpolation`) is moved from "string-level over the full inner text" to "AST-level over each branch's pipeline." The condition position is exempt from the check; all other rules are unchanged.
- **Decision #6** — Allowlist resolution runs per branch. The set of reachable command executables = `then.commands() ∪ else.commands()` (with `Branch::Empty` contributing none). Every member of that set must satisfy the existing allowlist policy. If any does not, the directive fails before any command runs.
- **Decision #7** — At execution time, the condition is evaluated first. The selected branch's pipeline is then prepared and executed through the existing `prepare_directive` / `execute_prepared_directive` path. A `Branch::Empty` selection short-circuits to `stdout = ""` without entering the shell runtime and consumes zero approvals.
- **Decision #8** — Execution-time errors retain their existing origins. The `ShellCommandOrigin::Frontmatter { key }` is unchanged. Parse errors gain enough message detail to identify which branch failed (e.g. "in then-branch of ternary").

## Worked Example

Input frontmatter:

```yaml
spec: "features/2026-05-15-schemas/spec.md"
has_spec: "spec ? true : false"
spec_file: "$({{has_spec}} ? basename '{{spec}}' : '')"
```

1. **Interpolation pass 1** resolves `has_spec` to `"true"` and `spec` to its literal path. `spec_file` is deferred (it contains `$(...)`).
2. **Shell-expansion stage** scans `spec_file`. The parser sees `$(...)`, splits on top-level `?`/`:`, and builds:
   - `condition = "{{has_spec}}"` (original text)
   - `then = Pipeline(basename '{{spec}}')`
   - `else = Empty`
3. **Validation** confirms `basename` is literal in the then-branch (the only branch with a command). Allowlist check runs on `{basename}`.
4. **Condition evaluation** uses the same interpolation lookup; `{{has_spec}}` evaluates to `true`. `is_truthy(true) == true`, so the then-branch is selected.
5. **Execution** prepares and runs `basename features/2026-05-15-schemas/spec.md`. Output `"spec.md"` is trimmed and written back to `spec_file`.

If `spec` were empty (so `has_spec` evaluated to `false`), the else-branch would be selected, `spec_file` would be set to `""`, and `basename` would never run.

## Pipeline Placement

Unchanged. Ternary parsing happens inside the existing `parse_shell_value` call, which is invoked by `scan_frontmatter` during the Frontmatter Shell Expansion stage (after the deferred-interpolation pass has resolved `{{has_spec}}` and `{{spec}}`). The two-pass interpolation flow described in `2026-04-08-shell-expansion-in-fm/spec.md` continues to apply.

## Surface Changes

### New AST

```rust
pub(crate) enum FrontmatterShellAst {
    Pipeline(ShellPipeline),
    Ternary {
        condition_source: String,    // original, with {{...}} unresolved
        then_branch: Branch,
        else_branch: Branch,
    },
}

pub(crate) enum Branch {
    Empty,
    Pipeline(ShellPipeline),
}

impl Branch {
    fn commands(&self) -> impl Iterator<Item = &str>;  // executables only
}
```

`FrontmatterShellDirective` gains an `ast: FrontmatterShellAst` field. The existing `executable` / `args` / `pipeline` fields are retained for the `Pipeline` case only (and become Option-wrapped or are computed on demand).

### Parser

`parse_shell_value` gains a small helper `split_top_level_ternary(inner: &str) -> Option<(&str, &str, &str)>` that returns `(condition, then, else)` slices, respecting quotes and parentheses, or `None` if no top-level `?`/`:` pair exists.

`validate_no_executable_interpolation` continues to enforce its rule but is called per branch's slice rather than once over the whole inner text. The condition slice is **not** checked.

### Execution

`execute_frontmatter_shell_expansion` adds a branch:

- For `Pipeline` AST: unchanged path.
- For `Ternary` AST: evaluate `condition_source` via `compose::expression::evaluate` against the same `FrontmatterSeedState` lookup used by the interpolation pass; select the branch; recurse on the selected branch (which is either Empty → `""` or Pipeline → existing path).

### Errors

Reuses `ShellExpansionError::ParseDirective` with refined messages:

- *(unchanged)* `"Frontmatter shell executable may not come from interpolation"` — now scoped to a single branch.
- *(new)* `"Ternary branch must be a command pipeline or an empty string literal"` — for branches that are neither.
- *(new)* `"Ternary condition must be a boolean expression"` — only emitted when condition evaluation produces a non-coercible value (current `is_truthy` is total over `serde_json::Value`, so practically only emitted if the expression engine itself returns an error).

### CLI / Public API

No CLI changes. No new `ComposeOptions` fields. The shell allowlist file format is unchanged.

## Backwards Compatibility

- Any directive that does **not** contain a top-level `?` keeps its current parse path bit-for-bit.
- Any directive that *previously parsed and ran* continues to parse and run identically.
- Any directive that *previously failed* with `"Frontmatter shell executable may not come from interpolation"` and contained no top-level `?` still fails with the same error.
- The only newly accepted inputs are well-formed ternaries whose reachable commands satisfy today's allowlist policy.

## Test Coverage

The existing test module in `frontmatter_shell_expansion.rs` (lines 419–668) already exercises the executable-interpolation rule on chained pipelines. The new tests live alongside and cover:

- **Accepts:** literal-true ternary executes the then-branch; literal-false executes the else-branch; interpolated condition selects the correct branch; empty-string branch produces `""` without entering the shell; both branches as command pipelines; argument interpolation inside a branch.
- **Rejects:** interpolated executable in the then-branch; interpolated executable in the else-branch; branch that is neither a pipeline nor `''`/`""`; ternary with missing `:`; ternary with multiple top-level `?`; off-allowlist command in either branch (existing allowlist test infrastructure).
- **Quote handling:** `?` and `:` inside single or double quotes do not split.
- **Allowlist policy:** if the then-branch's command is allowlisted but the else-branch's command is not, the directive fails at parse/validation, before either runs.

## Risks

- **Risk:** Ternary detection is heuristic (top-level `?` then `:`). An author who writes `$(echo "hello?world" : "x")` could trigger a false positive. *Mitigation:* the quote-aware splitter must respect single and double quotes (the existing `find_unquoted_closing_paren` already establishes this pattern). A directive whose body contains a top-level `?` but no top-level `:` errors with a clear message rather than silently falling back to pipeline parsing — otherwise authoring mistakes hide.
- **Risk:** Condition evaluation surfaces the expression engine to a new caller. *Mitigation:* the engine is already invoked by `::block when="..."` and by `Expr::Ternary` inside `{{ }}` interpolation; the lookup struct is the same one the interpolation pass already builds. No new surface.
- **Risk:** Allowlist coverage drift. If a branch's command is allowlisted today but removed later, both branches must be re-audited. *Mitigation:* none required — this is how allowlist policy already works for pipelines with `&&` / `||` chains.

## Out-of-Scope Alternative Considered

**Expression-engine shell function.** An orthogonal approach would expose `shell("cmd", args...)` as a function in the template expression engine, so authors could write `{{ has_spec ? shell("basename", spec) : "" }}` entirely within `{{ }}` and skip the shell-directive layer. This is strictly more powerful (nestable, composable with other template functions) but a much larger surface change — it requires the expression engine to gain side-effecting calls, allowlist enforcement at expression-evaluation time, and a story for caching. Out of scope here; this spec is the minimal change that solves the motivating problem.
