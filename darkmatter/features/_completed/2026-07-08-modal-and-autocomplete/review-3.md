---
$schema:
    ready: "boolean(required) -> is the feature production ready?"
ready: false
implemented: true
agent: codex/default
created: 2026-07-09T20:54:52
---

# Review 3: DMLS Interpolation Assistance

## Findings

### High: member/index-root `ctx.*` hover has no requirement-level verification

D2 explicitly requires catalog-enriched hover when a known `ctx.*` variable is
the root of member or index access. The implementation appears intended to
support this through `root_identifier` and `expression_at`:

- `darkmatter/dmls/src/overlay/expressions.rs:69`
- `darkmatter/dmls/src/overlay/expressions.rs:145`
- `darkmatter/dmls/src/providers/dsl.rs:319`

However, the provider tests cover a direct `ctx.packages` expression and a
direct `ctx.packages` function argument, while the in-memory LSP session covers
only direct `{{ ctx.packages }}` hover. No test exercises an expression such as
`{{ ctx.packages[0] }}` or a member-access chain and proves that the catalog
type, description, and compose-time note surface at the intended cursor
positions.

This is a user-observable requirement with no effective verification level,
which is a high-severity gap under the review's rigor rules. Add at least a
Level 1 provider test for both index and member traversal, plus one in-memory
LSP-session case proving the response content and range. Level 2 and Level 3
would not add relevant coverage because this is an LSP JSON/AST contract, not
terminal rendering or keyboard encoding.

### Medium: the new autocomplete documentation contradicts completion behavior

`darkmatter/dmls/docs/autocomplete.md:73` says a `ctx.*` candidate is only
offered when the cursor already has an explicit `ctx.` prefix. The provider
actually includes fully qualified `ctx.*` candidates for any matching partial,
including an empty partial; its all-formatting-functions test deliberately
builds the candidate set with `partial == ""`.

The explicit-prefix rule belongs to hover classification: bare `{{ today }}`
must not receive `ctx.today` metadata. It does not prohibit discovery of the
fully qualified `ctx.today` completion while manually completing an empty or
short interpolation. Rewrite this section so the public documentation matches
the implementation and D2/D3 distinction. The hover guide also omits the new
typed `ctx.*` and function-identifier hover shapes despite being described as
the feature's hover guide; documenting those shapes would close the same drift.

## Prior-review closure

All three Review 2 blockers are closed in the current working tree:

- The needless lifetime on `expression_at` is removed and the full area lint
  recipe passes.
- The two Darkmatter layout tests now pin the page frame width and pass in the
  canonical Level 1 area run.
- The native stdio smoke test is named
  `native_binary_speaks_lsp_over_stdio`; it now runs in the normal Level 1 tier,
  which reports 336 DMLS tests passed and zero skipped.

The catalog adapter remains single-sourced. Completion metadata is eager and
uses the correct LSP fields, the period trigger is advertised without dropping
the previous triggers, function hover is identifier-scoped, and the passive
no-side-effects sentinel test remains in the Level 1 gate.

## Verification-level matrix

| User-facing requirement | Strongest effective verification | Assessment |
|---|---:|---|
| Direct catalog-backed `ctx.*` interpolation hover and shared frontmatter block | Level 1 provider + in-memory LSP | Appropriate |
| `ctx.*` as the root of member/index access receives the same hover | None | **Gap — high** |
| Bare versus explicitly qualified `ctx.*` classification, including unknown names | Level 1 provider | Appropriate |
| `ctx.*` completion metadata and UTF-16 `textEdit` after an astral character | Level 1 provider + in-memory LSP | Appropriate |
| `.` capability trigger and no prose completions | Level 1 provider + in-memory LSP | Appropriate |
| Catalog-backed function completions, including all six formatters and fallibility | Level 1 provider + in-memory LSP | Appropriate |
| Known function-identifier hover and generic unknown-function hover | Level 1 provider + in-memory LSP | Appropriate |
| Function argument, punctuation, and nested-call hover precedence | Level 1 provider | Appropriate |
| Passive/no-execution behavior | Level 1 in-memory LSP sentinel test | Appropriate |
| Native binary stdio lifecycle | Level 1 subprocess | Appropriate and included in the canonical gate |
| Terminal rendering or terminal input encoding | Not applicable | No feature requirement needs Level 2 or Level 3 |

The feature's observable output is LSP response data. No requirement depends on
terminal glyphs, SGR styling, a terminal emulator's byte encoder, or OS keyboard
events, so Level 1 provider/session tests are the correct ceiling for the
feature itself. The area's Level 2 suite remains a compatibility gate for the
other Darkmatter surfaces.

## Verification performed

- `just test`: passed Darkmatter 5,191/5,191, darkmatter-cli 545/545, and DMLS
  336/336; DMLS had zero skipped tests. One unrelated Darkmatter persistent-cache
  test failed its first attempt and passed its configured retry.
- `just lint`: passed for Darkmatter, darkmatter-cli, and DMLS.
- `just test-l2`: passed 19/19 Darkmatter and 69/69 darkmatter-cli real-terminal
  tests. DMLS selected zero tests because this feature has no terminal-dependent
  behavior.
- Testing was executed on macOS. The feature implementation uses
  platform-neutral LSP, parser, and Rust APIs; Windows and Linux were not
  available for execution in this review environment.

## Production readiness

Not ready for production. The implementation and canonical gates are green,
and all prior-review blockers are closed, but one explicitly specified
user-visible hover path still has no Level 1 verification. The public
autocomplete guide also states a completion restriction the provider does not
enforce. Add the member/index-root tests and correct the documentation before
sign-off.
