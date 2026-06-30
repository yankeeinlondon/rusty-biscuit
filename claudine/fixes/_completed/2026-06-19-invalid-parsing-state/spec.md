---
status: draft
created: 2026-06-19
area: claudine
review_iterations: 6
packages:
    - darkmatter
    - claudine
    - claudine-cli
---

# Invalid Frontmatter Expansion State

## Problem

Running:

```text
claudine compose prompts/implement-suggestions.md -y review="features/2026-06-19-review-findings/review-2.md" --claude --dry-run
```

shows this effective frontmatter value:

```yaml
spec_path: '{{ dirname(review) + ''/spec.md'') }}'
```

That value should never survive as a literal string. It is a whole-value
frontmatter expansion because the scalar starts with `{{` and ends with `}}`.
The expression is malformed:

```text
dirname(review) + '/spec.md')
```

There is an extra closing parenthesis. The compose pipeline must report that as
an error instead of treating the value as authored text.

The same invariant applies to whole-value frontmatter shell expansion: a string
that starts with `$(` and ends with `)` is not an ordinary string. It must be
parsed and expanded as shell expansion, and parse or execution failure must be a
composition error.

## Cause

The failure is caused by the interaction between Darkmatter's lenient
interpolation helper and frontmatter composition:

1. `darkmatter/lib/src/markdown/compose/pipeline/mod.rs` runs
   `frontmatter_interpolation::interpolate_frontmatter(...)` before schema
   validation and before frontmatter shell expansion.
2. `frontmatter_interpolation::rewrite_value(...)` delegates string values to
   `interpolation::rewrite::interpolate_value(...)`.
3. `interpolate_value(...)` first tries the whole-value scalar fast path, but
   malformed expressions fail parsing there and fall through to
   `interpolate_text(...)`.
4. `interpolate_text(...)` treats parse and evaluation errors as warnings when
   `fail_fast` is false. It leaves the original `{{ ... }}` span unchanged.
5. The unchanged value is then serialized as a normal string in effective
   frontmatter. In this reproduced case, schema validation only checks that
   `spec_path` is file-shaped, so the raw template string is allowed through.

This leniency is acceptable for mixed body prose, where authors may want
non-fatal warnings, but it is invalid for a frontmatter scalar whose entire
value is an expansion form. Whole-value expansion syntax is executable
frontmatter state, not text.

## Goals

1. A frontmatter string scalar that trims to exactly `{{ ... }}` must either
   evaluate successfully or return a composition error.
2. A frontmatter string scalar that trims to exactly `$(...)` plus supported
   suffixes must either parse and expand successfully or return a composition
   error.
3. Invalid whole-value frontmatter expansion must fail even when global compose
   `fail_fast` is false.
4. Mixed strings keep existing behavior unless they already match a stricter
   shell rule. For example, `prefix {{ missing }}` and `literal $(not shell)`
   are not part of this fix.
5. Errors must include enough context to locate the frontmatter key and the
   invalid expression in the source file.

## Non-Goals

- Do not make every body interpolation parse warning fatal.
- Do not change undefined-variable semantics for ordinary mixed interpolation.
- Do not redesign the expression language.
- Do not add a Claudine-only guard if the invariant can be enforced in
  Darkmatter, where frontmatter composition actually occurs.

## Proposed Design

Add strict whole-value expansion validation in Darkmatter's frontmatter
composition path.

### Strict `{{ ... }}` Frontmatter Values

In `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`, detect
string values whose trimmed contents are exactly one interpolation span.

Use `ExpressionFinder::find_all_plain(...)` and require:

- exactly one span;
- no non-whitespace text before the span;
- no non-whitespace text after the span.

For this shape, parse and evaluate directly. Any parse failure or evaluation
failure is a `MarkdownError::Transform` regardless of `ComposeOptions.fail_fast`.

This check should happen before falling back to `interpolate_text(...)`, so the
existing body and mixed-string warning behavior remains intact.

The successful path should preserve the existing type behavior:

- boolean, number, and null results stay typed JSON values;
- string results become `Value::String`;
- arrays and objects may either stay typed or keep the current string-path
  behavior, but the choice must be explicit and covered by tests.

### Strict `$()` Frontmatter Values

`frontmatter_shell_expansion::parse_shell_value(...)` already errors for
strings that start with `$(` and then fail to close or contain invalid suffixes.
The gap to close is final-state leakage: after the
`FrontmatterShellExpansion` stage is enabled, a top-level frontmatter string
that is exactly a shell expansion shape must not remain unexpanded without an
error.

Add a post-frontmatter-shell validation pass for whole-value `$()` scalars after
`execute_frontmatter_shell_expansion(...)` runs. The pass should inspect
top-level string-valued frontmatter and fail if a value still trims to a shell
expansion candidate.

This validation should use the existing parser rather than ad hoc string
matching where possible. The accepted suffixes must match
`parse_shell_value(...)` (`::timeout:N`, `::no-cache`, in supported order).

If frontmatter shell expansion is explicitly disabled by compose options, keep
the existing deferred behavior. This fix targets enabled expansion that silently
leaks through as a string.

### Error Surface

Errors should identify:

- the frontmatter key;
- whether the failed form was interpolation or shell expansion;
- the expression text;
- the source file and line when `SourceContext` can locate the key.

Claudine should continue to render these through its existing composition error
path. If the error originates in a prompt file's YAML frontmatter, the existing
frontmatter excerpt appendix should highlight the offending key.

### Documentation Updates

Update the composition documentation so the strictness boundary is explicit:

- Whole-value frontmatter `{{ ... }}` is mandatory expression state. It must
  parse and evaluate, and failures are fatal even when mixed/body interpolation
  remains warning-based.
- Whole-value frontmatter `$()` is mandatory shell-expansion state when
  frontmatter shell expansion is enabled. It must parse and expand, and failures
  are fatal.
- Mixed body prose and mixed frontmatter strings may still use the existing
  non-fatal warning behavior when `fail_fast` is false.

At minimum, update:

- `claudine/docs/topics/composition.md`;
- the Claudine skill composition notes in `.claude/skills/claudine/SKILL.md`;
- the relevant Darkmatter compose docs or module docs if they describe
  interpolation warnings, frontmatter interpolation, or `$()` expansion.

The key wording to preserve is: frontmatter values that are exactly an expansion
form are not text and must never leak into effective frontmatter as raw syntax.

## Tests

### Darkmatter Unit Tests

Add tests near `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
and `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`.

Required interpolation cases:

1. `spec_path: "{{ dirname(review) + '/spec.md') }}"` returns an error even
   with `ComposeOptions.fail_fast == false`.
2. `spec_path: "{{ dirname(review) + '/spec.md' }}"` succeeds and produces the
   expected string.
3. `ready: "{{ false }}"` still produces a JSON boolean false.
4. `index: "{{ 2 }}"` still produces a JSON number.
5. `message: "prefix {{ malformed) }}"` keeps the existing warning behavior
   when `fail_fast == false`.

Required shell cases:

1. `value: "$(echo ok)"` expands when frontmatter shell expansion is enabled.
2. `value: "$(echo ok"` returns a parse error.
3. `value: "$(file_exists('x'))"` returns the existing no-command diagnostic.
4. A whole-value shell candidate must not remain in final frontmatter after
   enabled frontmatter shell expansion.
5. A mixed literal such as `value: "literal $(echo ok)"` remains outside this
   strict whole-value rule.

### Claudine CLI Regression Test

Add a CLI-level regression in `claudine/cli/tests/compose_cli.rs` or the
existing schema compose test file.

The test should compose a fixture with:

```yaml
spec_path: "{{ dirname(review) + '/spec.md') }}"
```

and assert that:

- `claudine compose ... --dry-run` exits non-zero;
- stderr mentions `spec_path`;
- stderr mentions interpolation parse failure or an equivalent precise parse
  diagnostic;
- the dry-run effective frontmatter does not print the raw `{{ ... }}` value as
  a successful result.

### Reproduction Fixture

The existing prompt at `prompts/implement-suggestions.md` is the motivating
case. The regression test may use a smaller fixture, but one manual validation
must be recorded against the original command:

```text
claudine compose prompts/implement-suggestions.md -y review="features/2026-06-19-review-findings/review-2.md" --claude --dry-run
```

Expected result after the fix: the command fails during compose preparation
with a frontmatter interpolation parse error for `spec_path`.

## Acceptance Criteria

1. Whole-value `{{ ... }}` frontmatter parse failures are fatal regardless of
   `fail_fast`.
2. Whole-value `{{ ... }}` frontmatter evaluation failures are fatal regardless
   of `fail_fast`.
3. Whole-value `$()` frontmatter parse and expansion failures are fatal when
   frontmatter shell expansion is enabled.
4. The malformed `spec_path` reproduction fails instead of appearing in
   effective frontmatter.
5. Existing mixed-string and body interpolation leniency remains unchanged.
6. Composition docs state the strict whole-value frontmatter expansion contract
   and distinguish it from mixed/body warning behavior.
7. `just test` passes in the `darkmatter` and `claudine` package areas touched
   by the implementation.

## Implementation Notes

- Prefer enforcing the invariant in Darkmatter, not by adding another
  Claudine-only post-check. Claudine only reveals the bad state; Darkmatter
  creates it.
- Avoid duplicating shell grammar. Reuse `parse_shell_value(...)` or factor a
  small helper if the post-expansion validation needs candidate detection.
- Do not run `cargo fmt` in write mode as part of this fix unless explicitly
  requested.
