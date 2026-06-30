---
clarified: claude/claude-opus-4-8
review_iterations: 1
---

# Replace Expression Functions

## Summary

Darkmatter's expression engine already provides a "String Mutations" family
(`lower`, `upper`, `capitalize`, `kebab_case`, `ensure_leading`, …). This
feature adds three substring-replacement mutations to that family:

1. `replace(x, find, replacement)` — replaces **all** occurrences of `find` in `x`.
2. `replace_first(x, find, replacement)` — replaces only the **first** (leading) occurrence.
3. `replace_last(x, find, replacement)` — replaces only the **last** (trailing) occurrence.

All three are total text transforms over a string subject. They are pure
(no I/O, no resolution context), deterministic, and follow the existing
null-propagation / type-mismatch contract shared by the rest of the engine.

## Function Signatures and Semantics

Every existing string mutation takes the subject value as **argument 0**; the
engine has no pipe operator or implicit-subject mechanism, so these functions
are **subject-first** with three arguments. (This corrects the brief's original
two-argument `replace(find, replacement)` sketch.)

| Function | Signature | Behavior |
| --- | --- | --- |
| `replace` | `replace(x, find, replacement)` | Replace **every** non-overlapping occurrence of `find` in `x` with `replacement`. |
| `replace_first` | `replace_first(x, find, replacement)` | Replace only the **first** occurrence (leftmost match); leave the rest of `x` untouched. |
| `replace_last` | `replace_last(x, find, replacement)` | Replace only the **last** occurrence (rightmost match); leave the rest of `x` untouched. |

- `x`, `find`, and `replacement` are all strings.
- The return value is a string (except under null propagation — see below).
- Canonical names keep the underscores (`replace_first`, `replace_last`). The
  existing registration mechanism auto-generates the underscore-free aliases
  (`replacefirst`, `replacelast`), consistent with the other multi-word
  functions (`kebab_case`/`kebabcase`, `ensure_leading`/`ensureleading`, …).
  `replace` is a single word and needs no alias.

## Behavior Decisions

These are ratified contracts; implementers must not re-derive or "improve" them.

### Empty `find` → no-op

If `find` is the empty string `""`, return the subject `x` unchanged. This is
**deliberately not** Rust's `str::replace` behavior (which inserts the
replacement at every character boundary, including the ends). That boundary-
insertion behavior corrupts output silently and is never what an author wants,
so it is explicitly rejected. The empty-`find` guard applies identically to all
three functions.

### No match → return unchanged

If `find` does not occur in `x`, return `x` verbatim. This matches the standard
`str::replace` / `str::find` / `str::rfind` behavior.

These are **total text transforms, not assertions.** A missing `find` is **not**
an error. This is intentionally distinct from the file-reference fatality rule
(where a present-but-unresolvable reference is fatal): there is no "the author
expected a match" contract here, so no error is raised.

### Literal matching, not regex

Matching is **plain substring** matching. The engine has no regex support;
`find` is treated as a literal string, never a pattern. Characters such as `.`,
`*`, `(`, `)`, `[`, `]` carry no special meaning.

### Case-sensitive

Matching is **case-sensitive**, consistent with the existing
`starts_with` / `ends_with` / `contains` predicates. `replace("ABC", "abc", "x")`
returns `"ABC"` unchanged.

### Overlapping matches

Overlapping matches are a non-issue. `replace` uses left-to-right
non-overlapping replacement (`str::replace`); `replace_first` / `replace_last`
each act on a single match located by `str::find` / `str::rfind`. There is no
ambiguity to resolve.

### Argument escaping

No new escaping rules. Argument escaping is already handled by the existing
quoted-string lexer: commas inside quotes are literal, and `\n`, `\t`, `\r`,
`\\`, and escaped quotes are supported. Authors who need to match or insert a
literal comma, newline, or quote use the existing quoting rules.

## Error Handling

| Condition | Result |
| --- | --- |
| Any argument resolves to `null` | The call returns `null` (existing `any_null` helper — null propagation). |
| Any argument is a non-string (number, bool, array, object) | `ExpressionError::Other { function, message }` via the existing `require_string` helper. |
| Wrong argument count (not exactly 3) | Arity error via the existing `require_args` helper. |
| `find` not found in `x` | **Not an error** — returns `x` unchanged (see "No match"). |
| `find == ""` | **Not an error** — returns `x` unchanged (see "Empty find"). |

These functions never perform I/O and never touch a `ResolutionContext`, so
file-reference, remote-URL, and resolution errors are out of scope for them.

## Implementation Notes

These are pointers identified during clarification, not prescriptive code. The
spec governs behavior; match the surrounding conventions.

- **Handlers + registration** —
  `darkmatter/lib/src/markdown/compose/expression/functions.rs`.
  Add three handlers following the `string_mutation` conventions and the
  two-arg pattern of `ensure_leading` / `ensure_trailing` (arity check,
  `any_null` short-circuit, `require_string` per argument). Add their
  `PURE_FUNCTIONS` entries in the "String mutations" block:
  - `replace` — `aliases: &[]`, `signatures: &["replace(x, find, replacement)"]`
  - `replace_first` — `aliases: &["replacefirst"]`, `signatures: &["replace_first(x, find, replacement)"]`
  - `replace_last` — `aliases: &["replacelast"]`, `signatures: &["replace_last(x, find, replacement)"]`
- **Catalog descriptors** —
  `darkmatter/lib/src/markdown/compose/expression/catalog.rs`.
  Add three `ExpressionFunctionDescriptor` entries in the `String Mutations`
  category (continuing the `order` sequence). A parity test enforces a 1:1
  match between runtime signatures (`PURE_FUNCTIONS`) and catalog descriptors,
  so the signature strings must agree exactly.
- **Docs table** —
  `darkmatter/docs/topics/darkmatter-expressions.md`.
  Regenerate / update the function table in the "String Mutations" section. A
  test enforces that this table matches the descriptor catalog.

## Acceptance Criteria

Each behavior maps to at least one concrete input → output case. All are
testable as pure-function runtime tests in `functions.rs` (the existing
`fn_string_mutations` test module is the natural home).

### Core replacement behavior

- `replace("a.b.c", ".", "/")` ⇒ `"a/b/c"` (all occurrences)
- `replace_first("a.b.c", ".", "/")` ⇒ `"a/b.c"` (first only)
- `replace_last("a.b.c", ".", "/")` ⇒ `"a.b/c"` (last only)
- `replace("aaa", "a", "bb")` ⇒ `"bbbbbb"` (non-overlapping, left-to-right)
- `replace_first("foofoo", "foo", "bar")` ⇒ `"barfoo"`
- `replace_last("foofoo", "foo", "bar")` ⇒ `"foobar"`

### Empty `find` no-op

- `replace("abc", "", "X")` ⇒ `"abc"`
- `replace_first("abc", "", "X")` ⇒ `"abc"`
- `replace_last("abc", "", "X")` ⇒ `"abc"`

### No match → unchanged

- `replace("hello", "z", "Q")` ⇒ `"hello"`
- `replace_first("hello", "z", "Q")` ⇒ `"hello"`
- `replace_last("hello", "z", "Q")` ⇒ `"hello"`

### Case sensitivity

- `replace("ABCabc", "abc", "X")` ⇒ `"ABCX"` (only the lowercase run matches)

### Null propagation

- `replace(null, ".", "/")` ⇒ `null`
- `replace("a.b", null, "/")` ⇒ `null`
- `replace("a.b", ".", null)` ⇒ `null`
- The same holds for `replace_first` and `replace_last`.

### Type errors

- `replace(5, ".", "/")` ⇒ `ExpressionError::Other` (non-string subject)
- `replace("a", 1, "/")` ⇒ `ExpressionError::Other` (non-string `find`)
- `replace("a", ".", true)` ⇒ `ExpressionError::Other` (non-string `replacement`)
- The same holds for `replace_first` and `replace_last`.

### Parity / docs

- The signature for each new function in `PURE_FUNCTIONS` matches its
  `ExpressionFunctionDescriptor` (runtime↔catalog parity test passes).
- The function table in `darkmatter-expressions.md` matches the descriptor
  catalog (docs-sync test passes).

### Cross-platform

These are pure in-memory string transforms with no OS-specific behavior (no
paths, no I/O, no locale-dependent casing of the kind path handling needs).
Cross-platform risk is therefore **low**. The acceptance suite should still run
green on macOS, Windows, and Linux; no platform-specific cases or `cfg` gating
are required.

## Out of Scope

The following are explicitly deferred and are **not** part of this feature:

- **Regex matching** — `find` is always a literal substring; no pattern syntax.
- **Case-insensitive variants** — e.g. `replace_ci` / a case-fold flag.
- **Count- or nth-based variants** — e.g. "replace the first N" or "replace the
  Nth occurrence."
- **A strict / asserting variant** — a `replace`-like function that raises a
  fatal error when `find` is absent. The functions in this feature are total
  transforms by design; a no-match-is-fatal variant is a separate, future
  decision.
