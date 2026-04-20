# Discord Webhook Provider Remediation Plan 1

## Goal

Close the gaps identified in `review-1.md` for the Discord webhook feature, keep
the existing Discord bot path behavior unchanged, raise coverage on the webhook
send paths, and finish with clean tests and zero lint warnings in the
`messenger` package area.

## Execution Constraints

- Work only in the `messenger` package area plus this feature folder unless a
  fix explicitly requires a shared file already used by `messenger`.
- Do not disturb unrelated workspace edits. At plan time, unrelated modified
  files exist outside `messenger/` (`.ai/.vscode/settings.json`,
  `prompts/implement-review-suggestions.md`) and must be left alone.
- Keep the existing Discord bot provider behavior stable. Any shared-helper
  refactor must be protected by existing Discord bot tests plus the new webhook
  tests added below.
- Prefer tightening behavior before broad cleanup. Add the missing assertions
  and regression tests before refactoring shared webhook/bot helper code.

## Phase 1: Correctness Hardening

### Purpose

Fix the real correctness issue in webhook thread handling, remove the current
panic-oriented CLI construction path, and tighten the provider's secret/call
construction so future logging cannot accidentally expose the webhook token.

### Work

1. Harden thread-id handling in
   `messenger/lib/src/provider/discord_webhook.rs`.
   - Add a dedicated helper that validates a webhook `thread_id` as a numeric
     Discord snowflake before building the request.
   - Stop raw string concatenation of `thread_id` into the URL. Build the query
     through validated values so malformed input cannot inject query
     parameters.
   - Return `MessengerError::InvalidMessage` for invalid thread IDs.

2. Reduce webhook token exposure risk in
   `messenger/lib/src/provider/discord_webhook.rs`.
   - Stop storing the full token-bearing webhook URL in a casually loggable
     `base_url` field.
   - Split the parsed webhook into safer internal pieces, keeping the token in
     a secret-bearing field or otherwise limiting accidental exposure.
   - Remove the current dead-code suppression around the parsed token and make
     the parsed representation match the actual provider storage.

3. Make CLI/provider construction fallible in
   `messenger/cli/src/main.rs`.
   - Replace `DiscordWebhookProvider::new(...)` usage with `try_new(...)` so
     malformed user-supplied webhook URLs surface as normal errors instead of a
     panic.
   - Update surrounding comments/docs in `discord_webhook.rs` so constructor
     semantics are honest.
   - If `new(...)` becomes unused after this change, either remove it or keep
     it only if it still serves an intentional ergonomic purpose.

4. Address the low-priority API ergonomics callout in
   `messenger/lib/src/dispatch.rs`.
   - Evaluate whether `DiscordWebhookOverrides` can safely be marked
     `#[non_exhaustive]` in this branch.
   - If that is semver-risky for the current release posture, leave behavior
     unchanged and add a short code comment/TODO explaining why it remains a
     follow-up rather than silently ignoring the review note.

### Verification

- Add/adjust provider unit tests in `discord_webhook.rs` for invalid thread-id
  values and any constructor parsing changes.
- Add/adjust CLI tests covering malformed webhook URL registration so the error
  path is exercised without a panic.
- Run targeted checks:
  - `cargo test -p messenger --lib discord_webhook`
  - `cargo test -p messenger-cli --bins discord_webhook`

## Phase 2: Webhook Coverage Expansion

### Purpose

Close the largest remaining test blind spots so the webhook transport is pinned
end-to-end before cleanup refactors begin.

### Work

1. Strengthen JSON-path assertions in
   `messenger/lib/src/tests/discord_webhook_integration.rs`.
   - Update the existing text-send test to assert the JSON payload body, not
     only method/path/headers.
   - Add an end-to-end markdown send test showing Discord markdown reaches the
     webhook payload unchanged after renderer integration.

2. Add multipart attachment integration coverage in
   `messenger/lib/src/tests/discord_webhook_integration.rs`.
   - Send a bytes-backed attachment through the webhook provider.
   - Assert the request is multipart.
   - Assert `payload_json` includes the expected `content` and `attachments`
     metadata.
   - Assert the multipart body includes the uploaded file part and expected
     bytes/filename.

3. Cover missing response/error branches in
   `messenger/lib/src/tests/discord_webhook_integration.rs`.
   - Add a 400-response test that verifies the generic
     `MessengerError::Provider` mapping.
   - Add a response-without-`webhook_id` test that verifies fallback to the
     provider's configured webhook ID.

4. Expand correctness regression coverage around the new Phase 1 behavior.
   - Add an integration-shaped invalid-thread-id test that proves the provider
     fails before sending any request.
   - Keep the existing strict-mode and best-effort `reply_to` no-network tests
     green.

### Verification

- Run focused library tests while iterating:
  - `cargo test -p messenger --lib discord_webhook_integration`
- Before leaving the phase, rerun the full library tests:
  - `cargo test -p messenger --lib`

## Phase 3: Internal Cleanup and Documentation Closure

### Purpose

Reduce duplication introduced during the initial implementation, keep the bot
and webhook attachment paths coherent, and land the documentation updates the
review flagged as incomplete or at risk of drift.

### Work

1. Consolidate attachment-building internals with minimal blast radius.
   - Compare `messenger/lib/src/provider/discord_webhook.rs` and
     `messenger/lib/src/provider/discord.rs`.
   - Extract the shared "attachment source -> filename + bytes/part inputs"
     logic into a private helper module such as
     `messenger/lib/src/provider/http_helpers.rs` or a new
     `attachment_helpers.rs`.
   - Keep provider-specific error wording if needed, but eliminate the current
     copy/paste implementation.

2. Simplify webhook multipart assembly in
   `messenger/lib/src/provider/discord_webhook.rs`.
   - Remove the awkward two-pass `owned` + `metas` construction if possible.
   - Move the attachment-part builder out of the impl block if it still does
     not require `self`.

3. Apply the response-handling cleanup only if it stays webhook-scoped or very
   low risk.
   - The review explicitly marked a broader shared HTTP classification refactor
     as follow-up territory. Do not expand this remediation into a multi-
     provider rewrite unless the change remains small and fully test-backed.
   - A webhook-local helper or a small `http_helpers` extension is acceptable;
     a repo-wide normalization pass is not required for this remediation.

4. Close documentation gaps once behavior is stable.
   - Verify and update as needed:
     - `.claude/skills/messenger/SKILL.md`
     - `.claude/skills/messenger/providers.md`
     - `.claude/skills/messenger/cli-reference.md`
     - `.claude/skills/messenger/markdown-rendering.md`
     - `messenger/docs/user-guide.md`
     - `messenger/README.md`
   - Ensure the docs clearly distinguish Discord bot vs Discord webhook
     capabilities and note that both Discord adapters share the same markdown
     renderer.

5. Keep unrelated edits out of the final diff.
   - Verify no `biscuit-terminal/.../detection.rs` drive-by change is included.
   - If such a change appears locally during implementation, remove it from the
     remediation diff unless it becomes an explicit dependency of a messenger
     fix.

### Verification

- Re-run webhook-focused tests after cleanup:
  - `cargo test -p messenger --lib discord_webhook`
- Re-run CLI tests if any docs/examples or route plumbing changed:
  - `cargo test -p messenger-cli --bins`
- Use targeted searches to confirm docs are consistent:
  - `rg -n "Discord-Webhook|discord-webhook|supports_reply|webhook_url_env|DISCORD_WEBHOOK_URL" messenger .claude/skills/messenger`

## Phase 4: Full Messenger Validation and Lint Closure

### Purpose

Finish the remediation with a clean `messenger` package area: tests passing,
lint warnings fixed, and no regressions introduced by the cleanup/doc pass.

### Work

1. Run full package-area tests.
   - `cargo test -p messenger --lib`
   - `cargo test -p messenger-cli --bins`

2. Run full package-area linting and fix everything reported in `messenger`.
   - Preferred hard gate:
     - `cargo clippy -p messenger -p messenger-cli --all-targets -- -D warnings`
   - Also run the package recipe to match local workflow expectations:
     - `cd messenger && just lint`

3. If any lint fixes change code, rerun all tests immediately afterward.
   - `cargo test -p messenger --lib`
   - `cargo test -p messenger-cli --bins`

4. Confirm the final diff is cleanly scoped.
   - Inspect `git status --short` and `git diff --stat`.
   - The intended touched set is the `messenger/` package area, the
     `.claude/skills/messenger/` docs it owns, and this feature plan/doc set.

### Exit Criteria

- All review-mandated fixes implemented or explicitly documented as deliberate
  follow-up only where the review itself marked them out-of-scope.
- `cargo test -p messenger --lib` passes.
- `cargo test -p messenger-cli --bins` passes.
- `cargo clippy -p messenger -p messenger-cli --all-targets -- -D warnings`
  passes.
- `cd messenger && just test` and `cd messenger && just lint` are green.
- No unrelated non-messenger edits are included in the remediation result.

## Ordering Notes and Risks

- Phase 1 must land before Phase 2's no-network invalid-thread coverage,
  otherwise the tests cannot assert the intended failure mode.
- Phase 2 should land before Phase 3 refactors so the new tests pin current
  webhook behavior and protect the bot/webhook shared-helper cleanup.
- The attachment-helper cleanup is the highest regression risk because it can
  accidentally affect the existing Discord bot path. Keep the change small and
  rely on the full `messenger` library test suite before moving on.
- The `DiscordWebhookOverrides` `#[non_exhaustive]` suggestion may be a public
  API compatibility decision rather than a pure bug fix. Treat it deliberately,
  not as a casual drive-by attribute change.
