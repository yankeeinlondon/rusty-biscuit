# Setting Key/Values in Frontmatter

## Summary

Today, callers who want to override frontmatter values in a prompt must pass a JSON blob via `--set '{ ... }'`. This is awkward for quick one-off overrides and tedious at the shell (JSON quoting, escaping, etc.).

This feature adds an inline `key=value` positional syntax to the relevant `claudine` subcommands, mirroring the behavior already shipped in `darkmatter`. Both styles coexist, with inline overrides winning on conflict.

## Motivation

- `--set '{"review":"review.md"}'` is verbose and quoting-prone.
- Users already get this ergonomic shorthand from `darkmatter`; claudine should match.
- Point-of-use overrides are often more specific than a pre-built JSON blob and should be allowed to win.

## In Scope

Inline `key=value` positional arguments are added to these subcommands:

- `claudine compose`
- `claudine inline-compose`
- `claudine sequence`

Example (explicit subcommand, corrected from the prior draft):

```sh
claudine compose @prompts/review.md review="review.md"
```

This composes `@prompts/review.md` after setting the frontmatter key `review` to `"review.md"`.

## Out of Scope

- Subcommand-less default routing (e.g. `claudine @prompts/review.md ...` auto-dispatching to `compose`). This may be filed as a separate follow-up spec.
- Nested / dot-path keys via shorthand (see Key Namespace below). Users needing nesting continue to use `--set`.
- Subcommands other than the three listed above.

## Syntax and Parsing Rules

Positional tokens on the listed subcommands are classified as either:

- **File inputs** (existing behavior), or
- **Inline overrides** of the form `key=value`.

A token is treated as an inline override only if it matches the `key=value` shape with a valid key (see Key Namespace). All other tokens are passed through as file inputs, matching darkmatter's classifier.

### Splitting

- Split on the **first** `=` only. Everything after that first `=` is the raw value.
- Example: `url=https://example.com/path?a=b&c=d` yields key `url` and value `https://example.com/path?a=b&c=d`.

### Value parsing (mirrors darkmatter's `parse_shorthand_value`)

1. Attempt to parse the raw value as JSON5.
   - On success, use the typed value (number, boolean, array, object, null, string).
2. On JSON5 parse failure, fall back to treating the raw value as a plain string.
3. An empty right-hand side (`key=`) yields an empty string `""`.

### Examples

| Token                             | Parsed key | Parsed value                                       |
| --------------------------------- | ---------- | -------------------------------------------------- |
| `review=review.md`                | `review`   | `"review.md"` (string)                             |
| `count=3`                         | `count`    | `3` (number)                                       |
| `enabled=true`                    | `enabled`  | `true` (bool)                                      |
| `tags=["a","b"]`                  | `tags`     | `["a","b"]` (array)                                |
| `note="hello world"`              | `note`     | `"hello world"` (string; shell quoting passed through) |
| `url=https://x/?a=b`              | `url`      | `"https://x/?a=b"` (string)                        |
| `empty=`                          | `empty`    | `""` (empty string)                                |

Note: values containing spaces must be quoted at the shell layer; this is a normal shell concern, not a claudine concern.

## Key Namespace

- Keys are **flat**. No dot-path nesting is supported via this shorthand.
- Valid key regex: `^[A-Za-z_][A-Za-z0-9_-]*$`
  - First character: ASCII letter or underscore.
  - Remaining characters: ASCII letter, digit, underscore, or hyphen.
- `foo.bar=baz` is **not** a valid inline override. It is passed through as a positional file input per the classifier rules (matching darkmatter).
- Users who need nested structure continue to use `--set '{"foo":{"bar":"baz"}}'`.

## Precedence vs `--set`

Both mechanisms are supported together. The effective override map is built in this order:

1. Start empty.
2. Apply keys from `--set '{...}'` JSON.
3. Apply inline `key=value` pairs, **overwriting** any matching keys from step 2.

Inline shorthand wins on conflict. This matches darkmatter.

### Worked example

```sh
claudine compose @prompts/review.md \
  --set '{"review":"old.md","mode":"draft"}' \
  review="new.md"
```

Resulting overrides:

- `review` → `"new.md"` (inline overwrote `--set`)
- `mode`   → `"draft"` (from `--set`, untouched)

## Error Behavior

- **Token starting with `=`** (e.g. `=foo`): error. The key is missing; emit a clear message identifying the offending token.
- **Invalid key on a `key=value`-shaped token** (e.g. `9foo=bar`, `foo.bar=baz`): the token is treated as a positional file input (classifier fallback), matching darkmatter. It is not silently reinterpreted as an override.
- **Missing `=`**: token is a positional file input (existing behavior).
- **Duplicate inline keys** within one invocation: last-write-wins, consistent with the precedence rule.

## Reference Implementation

Darkmatter is the source of truth for parsing, classification, and precedence semantics. Claudine should mirror darkmatter's behavior. Relevant darkmatter locations:

- `darkmatter/cli/src/args.rs` — positional `args: Vec<String>` pattern replacing a single `file` positional.
- `darkmatter/cli/src/commands.rs` — `parse_compose_setter`, `parse_compose_positionals` (classifier), `parse_shorthand_value` (JSON5-then-string), and the precedence merge where `--set` seeds first and inline pairs overwrite.

## Acceptance Criteria

- [ ] `claudine compose`, `claudine inline-compose`, and `claudine sequence` each accept zero or more `key=value` positional tokens alongside existing file positionals.
- [ ] The example `claudine compose @prompts/review.md review="review.md"` sets `review` to `"review.md"` before composition.
- [ ] JSON5-typed values (numbers, booleans, arrays, objects, null) parse to their typed form; unparseable values fall back to plain strings.
- [ ] `key=` parses to an empty string.
- [ ] Splitting uses the first `=` only; values may contain further `=` characters.
- [ ] Keys are validated against `^[A-Za-z_][A-Za-z0-9_-]*$`; dot-path keys are not accepted as overrides.
- [ ] `--set` and inline overrides can be combined; inline overrides win on key collision.
- [ ] Tokens not matching the override shape are passed through as file inputs unchanged.
- [ ] Tokens beginning with `=` produce a clear error identifying the bad token.
- [ ] Behavior matches darkmatter's existing implementation for all of the above cases (covered by parity tests where practical).
- [ ] The typos `inline-compse` and `sequences` are corrected in all user-facing docs and help text touched by this feature.
