---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-12T13:16:06-07:00
---

# Review 4 — DMLS Semantic Tokens

## Verdict

Not ready for production. The implementation's Level 1 protocol behavior is well covered and the
focused 64-test suite passes, including the refresh-response routing added after Review 3. The
feature still has no completed Level 2 evidence from any claimed editor, however, and the changed
Zed delivery contract remains explicitly pending reapproval in the specification. Under the
required test-rigor rubric, both are high-severity release blockers.

## Findings

### High — Intended presentation and live repaint remain unverified in real editors

The smoke checklist explicitly marks its Level 2 real-editor work as outstanding for VS Code, Zed,
Neovim, and Helix on macOS, Windows, and Linux. The provider unit tests and in-process JSON-RPC
session tests verify the token data DMLS emits, but they do not exercise an editor's semantic-token
decoder, grammar-token combination, theme rules, viewport range requests, or refresh behavior.

The following user-observable requirements therefore have only Level 1 evidence where Level 2 is
required:

- the documented VS Code, Zed, and Neovim recipes visibly mute interpolations and directives and
  distinguish wiki-link segments;
- fenced-code contents retain their normal grammar highlighting without unintended Darkmatter
  styling;
- Unicode and multiline spans style the intended columns after a real editor decodes UTF-8 or
  UTF-16 positions;
- full and range responses produce equivalent visible styling at viewport boundaries; and
- changes to `semantic_tokens.enable` and `wiki.enable` repaint a refresh-capable editor without a
  restart.

Execute and record the real-editor smoke matrix. At minimum, validate every documented styling
recipe in its claimed editor, Unicode and multiline positioning, fenced-code grammar interaction,
full/range viewport behavior, and live configuration refresh. Level 3 is not applicable because
the feature has no keyboard, mouse, paste, or IME behavior.

### High — The revised Zed acceptance contract is still pending reapproval

The specification says the original extension-owned Zed styling defaults were replaced with a
user-copyable `experimental.theme_overrides` recipe because the Zed extension API cannot inject
semantic-token colors. The same revision note explicitly says this product-contract change is
pending Ken's reapproval. Consequently, installing the extension and enabling semantic tokens no
longer provides the originally accepted presentation without an additional manual theme edit.

Obtain approval for the documentation-only Zed delivery contract and remove the pending-reapproval
note, or implement another approved extension-owned delivery surface. A feature cannot be
production-ready while its specification identifies an unresolved acceptance-contract change.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| F1 interpolation and inert classification | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| F2 directive classes, exclusions, and closers | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| F4 wiki structure and resolution-independent output | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| Ordering, non-overlap, UTF-8/UTF-16, multiline splitting | Level 1 encoder and session tests | Wire data is covered; real-editor columns remain unverified |
| Fenced-code exclusion | Level 1 scanner/provider tests | Token absence is covered; visible grammar interaction needs Level 2 |
| Capability gating | Level 1 capability/session tests | Appropriate |
| Live configuration and refresh protocol | Level 1 session tests, including distinct in-flight IDs and success/error responses | Server routing is covered; real-client repaint needs Level 2 |
| No side effects | Level 1 no-side-effects harness | Appropriate |
| Range clipping and family precedence | Level 1 provider/session tests | Wire behavior is covered; visible viewport behavior needs Level 2 |
| Editor recipes and intended presentation | Documentation plus an outstanding manual checklist | Required Level 2 evidence is missing |
| Zed delivery mechanism | Revised specification marked pending reapproval | Acceptance contract remains unresolved |

## Prior-review closure

- Refresh response routing: closed. Two token-affecting changes now receive distinct request IDs;
  success and error responses travel through the router loop and retire the tracked requests
  without disrupting the session.
- Real-editor verification: not closed. The checklist remains explicitly outstanding and contains
  no recorded Level 2 results.
- Zed delivery contract: not closed. The specification still marks the revision pending
  reapproval.

## Verification performed for this review

- Inspected the complete specification, Review 3, the semantic-token provider and handlers,
  capability/configuration gates, refresh ledger, Level 1 session tests, editor documentation, and
  smoke checklist.
- `cargo nextest run -p dmls -E 'test(/semantic_tokens/)' --color never`: 64 passed.
- The broader package-area `just test` run was stopped at the non-interactive execution limit after
  1,769 of 5,481 tests passed; no failure occurred before interruption. This incomplete broad run
  is not counted as feature verification.
- No Level 2 run was available or executed because the repository records the real-editor matrix
  as outstanding. Level 3 is not applicable.
