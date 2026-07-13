---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-12T11:09:47-07:00
---

# Review 1 — DMLS Semantic Tokens

## Verdict

Not ready for production. The token scanners, precedence pipeline, encodings, full/range handlers,
configuration gates, and capability advertisement have strong Level 1 coverage, and the focused
semantic-token suite passes. However, the implementation does not ship the specified Zed styling
defaults, none of the editor-visible behavior has been verified in a real editor, and refresh
requests are sent with a reused ID while all client responses are ignored.

## Findings

### High — The specified Zed styling defaults were replaced with documentation

The acceptance scope requires “per-editor styling defaults” and specifically places Zed styling
defaults in the `zed-dmls` extension. Phase 6 instead records that the extension API cannot inject
semantic-token colors and substitutes a README recipe. No extension-owned theme or distinct
Darkmatter language was added. Consequently, installing the extension and opting into semantic
tokens does not provide the specified muted interpolation/directive presentation or link-like wiki
presentation; users must separately copy theme overrides.

This may be a valid platform constraint, but it changes the accepted product behavior and was not
reflected back into the specification. Either implement a supported extension-owned styling
surface (for example, the deferred distinct language/theme route), or revise and reapprove the
specification so copyable user configuration—not shipped defaults—is the explicit acceptance
contract.

### High — All user-visible editor behavior is verified only at Level 1

`darkmatter/dmls/tests/lsp_session.rs` starts DMLS and manufactures JSON-RPC messages in process.
Those are valuable Level 1 protocol tests, but the plan repeatedly calls them “Level-2 sessions.”
They do not launch VS Code, Zed, Neovim, or Helix; exercise a client's semantic-token decoder,
theme selector syntax, refresh handling, range-request behavior, or UTF-8/UTF-16 negotiation; or
observe rendered editor styling. `just test-l2` targets real terminal rendering and ran no tests
for this feature. The editor smoke checklist remains entirely unchecked.

The following user-observable requirements therefore have verification at the wrong level:

- interpolations/directives appear muted and wiki segments appear link-like in VS Code, Zed, and
  Neovim;
- fenced-code constructs retain only grammar highlighting;
- Unicode and multiline spans style the intended editor columns;
- full and range requests produce visually equivalent styling at viewport boundaries; and
- changing `semantic_tokens.enable` or `wiki.enable` repaints an actual client without restart.

Add automated real-client integration coverage where practical and record executed manual smoke
checks for each supported editor/OS combination. At minimum, each claimed editor recipe must be
validated in that editor, and the completion artifacts must classify the current session tests as
Level 1. Under the review rubric, a feature with these unverified observable requirements cannot be
production-ready.

### High — Semantic-token refresh requests reuse one ID and client failures are discarded

`send_semantic_tokens_refresh` always sends the request ID
`"dmls/semantic-tokens-refresh"`. The router then ignores every `Message::Response`, so it neither
retires an outstanding request nor logs a client error. Two configuration changes before the first
response put two live JSON-RPC requests with the same ID on the connection, making responses
ambiguous. A client rejection also contradicts the specification's requirement that refresh
failure be logged.

Allocate a unique request ID for every server-to-client refresh, track outstanding refresh
requests, and consume their success/error responses. Log error responses without failing the
configuration notification. Add Level 1 session tests for two consecutive outstanding refreshes
and for a client error response; then retain the real-client repaint verification from the prior
finding.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| F1 whole-span and inert interpolation classification | Level 1 unit and in-process JSON-RPC session tests | Appropriate for emitted token data; passed |
| F2 directive classes, exclusions, and closers | Level 1 unit and in-process JSON-RPC session tests | Appropriate for emitted token data; passed |
| F4 wiki structure and resolution-independent shapes | Level 1 unit and in-process JSON-RPC session tests | Appropriate for emitted token data; passed |
| UTF-8/UTF-16 ordering, non-overlap, multiline splitting | Level 1 encoder and in-process session tests | Appropriate for server data; insufficient for editor column/rendering claims |
| Fenced-code exclusion | Level 1 scanner/provider tests | Appropriate for token absence; real-editor grammar interaction remains unverified |
| Capability gating | Level 1 capability and initialization-session tests | Appropriate for server advertisement; passed |
| Live config suppression and refresh | Level 1 session tests that do not answer the refresh request | Insufficient; repeated IDs/error responses are untested and real-client repaint is unverified |
| No analysis side effects | Level 1 no-side-effects harness | Appropriate |
| Range clipping and family precedence | Level 1 unit and in-process session tests | Appropriate for wire data; real-client viewport behavior remains unverified |
| Editor styling defaults and visible presentation | Documentation plus an unchecked manual checklist | Missing required implementation for Zed and missing real-editor verification |

Level 3 is not applicable: the feature has no physical keyboard, mouse, paste, or IME behavior.

## Verification performed for this review

- `cargo nextest run -p dmls -E 'test(/semantic_tokens/)' --color never`: 64 passed.
- Began `just test` from `darkmatter/`; 2,338 tests passed before the run was interrupted to honor
  the non-interactive command-duration limit. No failure occurred before interruption, so this is
  not a completed broad-suite result.
- Inspected the semantic-token provider, router/config refresh path, client capability gates,
  in-process session tests, editor documentation, smoke checklist, and implementation plan.
