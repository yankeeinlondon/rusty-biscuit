---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-12T13:08:51-07:00
---

# Review 3 — DMLS Semantic Tokens

## Verdict

Not ready for production. The emitted semantic-token stream and refresh-response routing now have
credible Level 1 coverage, and the focused suite passes. However, the intended presentation and
live repaint behavior remain unverified in every claimed real editor, and the revised Zed delivery
contract is still explicitly pending reapproval. Both are high-severity release blockers under the
required test-rigor rubric.

## Findings

### High — Real-editor behavior remains unverified

The repository still contains no completed Level 2 real-editor evidence for this feature. The
manual smoke checklist labels all VS Code, Zed, Neovim, and Helix checks as outstanding. The Level
1 provider and in-process JSON-RPC tests establish what DMLS emits, but they cannot establish how
an editor decodes, combines, themes, clips, or refreshes those tokens.

The following user-observable requirements therefore remain verified at the wrong level:

- the documented VS Code, Zed, and Neovim recipes visibly mute interpolations/directives and make
  wiki segments read link-like;
- fenced-code contents retain grammar highlighting without unintended Darkmatter token styling;
- Unicode and multiline tokens style the intended editor columns;
- full and range responses produce equivalent visible styling at viewport boundaries; and
- changes to `semantic_tokens.enable` and `wiki.enable` repaint a refresh-capable editor without a
  restart.

Execute and record the real-editor smoke matrix. At minimum, validate each documented recipe in
its claimed editor, verify Unicode/multiline and full/range rendering, confirm fenced-code grammar
interaction, and exercise live refresh in a client that advertises refresh support. Level 3 is not
applicable because this feature has no keyboard, mouse, paste, or IME interaction.

### High — The revised Zed product contract is still pending reapproval

The implementation provides a copyable `experimental.theme_overrides` recipe because the Zed
extension API cannot inject semantic-token colors. That may be the correct platform-compatible
design, but the specification's revision note still says this replacement for extension-shipped
styling defaults is “pending reapproval.” Installing the extension and enabling semantic tokens
does not by itself produce the intended presentation; the user must also edit theme configuration.

Obtain approval of the documentation-only Zed contract and remove the pending-reapproval note, or
implement another approved extension-owned delivery surface. Production readiness cannot be
claimed while the specification itself identifies an unresolved acceptance-contract change.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| F1 interpolation and inert classification | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| F2 directive classes, exclusions, and closers | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| F4 wiki structure and resolution-independent output | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| Ordering, non-overlap, UTF-8/UTF-16, multiline split | Level 1 encoder and session tests | Appropriate for wire data; real-editor columns remain unverified |
| Fenced-code exclusion | Level 1 scanner/provider tests | Token absence is covered; visible grammar interaction is not |
| Capability gating | Level 1 capability/session tests | Appropriate |
| Live configuration and refresh protocol | Level 1 session tests, including two in-flight IDs and success/error responses | Appropriate for server routing; real-client repaint remains unverified |
| No side effects | Level 1 no-side-effects harness | Appropriate |
| Range clipping and family precedence | Level 1 provider/session tests | Appropriate for wire behavior; visible viewport behavior remains unverified |
| Editor recipes and intended presentation | Documentation and an explicitly outstanding manual checklist | Required real-editor verification is missing |
| Zed delivery mechanism | Revised specification marked pending reapproval | Acceptance contract remains unresolved |

## Prior-review closure

- Refresh response routing: closed. The new Level 1 session test sends two token-affecting config
  changes before responding, verifies distinct request IDs, routes success and error responses
  through the router loop, and confirms the session remains responsive.
- Real-editor verification: not closed. The checklist remains explicitly outstanding.
- Zed delivery contract: not closed. The specification still marks the revision pending
  reapproval.

## Verification performed for this review

- Inspected the complete specification, Reviews 1 and 2, semantic-token provider, capability and
  configuration gates, router refresh ledger, Level 1 session tests, editor guides, implementation
  plan, and smoke checklist.
- `cargo nextest run -p dmls -E 'test(/semantic_tokens/)' --color never`: 64 passed.
- No Level 2 or Level 3 test was executed; the repository provides no completed real-editor
  semantic-token run to execute or assess, and Level 3 is not applicable to this feature.
