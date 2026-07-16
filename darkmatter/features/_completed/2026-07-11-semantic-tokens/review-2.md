---
$schema: "@.claudine/schemas/review.yaml"
ready: false
implemented: true
agent: codex/default
created: 2026-07-12T12:37:17-07:00
---

# Review 2 — DMLS Semantic Tokens

## Verdict

Not ready for production. The implementation has credible Level 1 coverage of the semantic-token
wire stream and the prior refresh-ID collision is fixed. However, the user-visible behavior in VS
Code, Zed, and Neovim still has no completed real-editor verification, and the specification now
marks the substituted Zed behavior as pending re-approval. Both are release blockers under the
review rubric.

## Findings

### High — Real-editor behavior remains unverified

The follow-up correctly relabels `darkmatter/dmls/tests/lsp_session.rs` as Level 1 instead of Level
2, but it does not add higher-level coverage. The updated smoke checklist explicitly says every
real-editor check remains outstanding across macOS, Windows, and Linux. The in-process JSON-RPC
fixture validates DMLS's token data; it cannot validate an editor's semantic-token decoder, theme
selector syntax, grammar/semantic-token interaction, refresh repaint, or displayed columns.

The following user-observable requirements therefore still have verification at the wrong level:

- interpolations and directives appear muted while wiki segments appear link-like in VS Code,
  Zed, and Neovim;
- fenced-code contents retain grammar highlighting without unwanted semantic-token styling;
- Unicode and multiline tokens style the intended editor columns;
- full and range requests produce equivalent visible styling at viewport boundaries; and
- changes to `semantic_tokens.enable` and `wiki.enable` repaint a supporting editor without a
  restart.

Execute and record the real-editor smoke matrix, or add automated editor integration where the
client permits it. At minimum, validate each documented recipe in its claimed editor and verify
live refresh in a refresh-capable client. Until then, the required visible presentation is not
production-verified. Level 3 is not applicable because this feature has no keyboard, mouse, paste,
or IME behavior.

### High — The accepted Zed product contract is still pending re-approval

The original contract required extension-shipped Zed styling defaults. The implementation instead
ships a copyable `experimental.theme_overrides` recipe because the Zed extension API cannot inject
semantic-token colors. The spec has been edited to describe that substitution, but its own
revision note says the change is “pending re-approval.” This is a product-level acceptance change,
not merely an implementation detail: installing the extension and enabling semantic tokens still
does not yield the intended presentation unless the user separately edits theme configuration.

Obtain explicit approval of the revised documentation-only contract and remove the pending status,
or implement an approved extension-owned styling surface. A feature cannot be production-ready
while its acceptance contract is explicitly awaiting approval.

### Medium — Refresh response handling is not tested through the router loop

`RefreshLedger` now allocates distinct IDs, retires success and error responses, and logs client
rejections, resolving the underlying collision design. Its new tests call the private ledger
directly, though. They do not prove that two `didChangeConfiguration` notifications issue distinct
requests through `Router`, or that an actual `Message::Response` is routed into the ledger without
terminating or disturbing the session. The prior review specifically requested session tests for
those paths.

Add Level 1 session coverage that sends two token-affecting configuration changes before replying,
asserts distinct request IDs, then returns one success and one error response and confirms the
server remains responsive. This behavior does not require Level 2 once the separate real-client
repaint requirement is covered.

## Requirement-to-verification assessment

| Requirement | Strongest evidence | Assessment |
|---|---|---|
| F1 interpolation and inert classification | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| F2 directive classes, exclusions, and closers | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| F4 wiki structure and resolution-independent output | Level 1 provider and in-process session tests | Appropriate for emitted token data |
| Ordering, non-overlap, UTF-8/UTF-16, multiline split | Level 1 encoder and session tests | Appropriate for wire data; real-editor columns remain unverified |
| Fenced-code exclusion | Level 1 scanner/provider tests | Token absence is covered; visible grammar interaction is not |
| Capability gating | Level 1 capability/session tests | Appropriate |
| Live configuration and refresh | Level 1 session tests plus direct ledger unit tests | Server data/config covered; response routing and real-client repaint remain gaps |
| No side effects | Level 1 no-side-effects harness | Appropriate |
| Range clipping and family precedence | Level 1 provider/session tests | Wire behavior covered; visible viewport behavior remains unverified |
| Editor recipes and intended presentation | Documentation and an explicitly outstanding manual checklist | Required real-editor verification missing |
| Zed delivery mechanism | Revised spec marked pending reapproval | Acceptance contract unresolved |

## Prior-review closure

- Zed defaults gap: not closed; the proposed contract change is documented but still pending
  approval.
- Real-editor verification gap: not closed; test levels are now described honestly, but the smoke
  matrix remains unexecuted.
- Refresh-ID collision: implementation fixed; router-level response-path coverage remains light.

## Verification performed for this review

- Inspected the complete specification, prior review, implementation plan, semantic-token provider,
  router/config refresh changes, session tests, editor guides, and smoke checklist.
- `git diff --check`: passed.
- A focused `cargo nextest run -p dmls -E 'test(/semantic_tokens/)' --color never` was started but
  did not complete within the non-interactive command-duration limit while rebuilding dependencies;
  it was stopped without a test result.
- A subsequent `cargo check -p dmls --color never` was also stopped after contention with the first
  build prevented completion inside the time limit. No compiler failure was observed, but this is
  not a successful check result.
