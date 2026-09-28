# Messaging

Claudine's `messaging` module provides outbound message delivery for lifecycle
actions and hook actions across four providers, plus the `claudine config` TUI
that manages the routes. Desktop notifications are a separate, zero-config path
(see [Desktop notifications](#desktop-notifications)).

## Routes

A route is one entry of `MessagingRouteConfig`, targeting one of four providers:

| Provider | Delivery mechanisms |
|----------|--------------------|
| Discord | Bot token (channel ID) or incoming webhook |
| Slack | Bot token (channel ID) or incoming webhook |
| Signal | Bot / REST recipient |
| WhatsApp | WhatsApp Business API |

Discord and Slack support **webhook** routes in addition to bot-token routes;
Signal and WhatsApp are token/recipient only. Secrets resolve at send time via
`resolve_secret`, so a route may store either an inline value or the name of an
environment variable that holds it.

## Config TUI

`claudine config` manages bot-token routes and webhook routes interactively:

- **Webhook URLs use masked input** and are validated before the wizard advances.
  Validation is a conservative early check (`validate_discord_webhook_url` /
  `validate_slack_webhook_url`); the authoritative check happens at send time in
  the `messenger` provider's `try_new`.
- **Env-only routes are allowed** — a blank URL plus an environment variable name
  is a valid configuration (the URL is resolved from the env var at send time).
- **Test Connection** — pressing `T` during webhook input runs
  `test_webhook_connection`, sending a test message **without saving** the route.

## Webhook redaction invariants

Webhook URLs embed a secret token in the path, so they are never surfaced raw:

- Inline webhook URLs render masked (shown as `webhook: ********`), never verbatim.
- Secret input buffers are masked during entry in the TUI.
- **All** webhook send errors and test-connection failures pass through
  `redact_webhook_urls` before display, so a failing send cannot leak the token
  via an error string.

## Relationship to lifecycle and hook actions

Message delivery is invoked two ways, both routing through the same `send` layer:

- **Lifecycle actions** — the `message` communication channel in a composition
  document's lifecycle stacks. See [Lifecycle](flow-control/lifecycle.md).
- **Hook actions** — the `message` action fired on a normalized event (see the
  Supported Actions reference). When running inside `claudine handle`, messenger
  actions carry a hard **3-second timeout** by default (overridable via
  `CLAUDINE_MESSENGER_TIMEOUT_SECONDS`).

## Delivery tracking

A send returns immediately; the delivery runs as a background task. Every such
task, message or desktop notification, is registered with one process-wide
delivery tracker in `claudine::messaging`, so a program can wait for it before
it exits instead of killing it mid-send.

A program that embeds the `claudine` library opts in by awaiting
`drain_deliveries` before it exits, on the same Tokio runtime the sends ran on:

```rust
use claudine::messaging::{DELIVERY_DRAIN_BUDGET, drain_deliveries};

let outcome = drain_deliveries(tokio::time::Instant::now() + DELIVERY_DRAIN_BUDGET).await;
outcome.report(); // one Warning naming each delivery still sending
```

- One deadline covers every pending delivery, including any started while the
  drain runs. With nothing pending the drain returns at once.
- A delivery still running at the deadline is cancelled and listed in
  `outcome.pending`. Whether it arrived is unknown. `report()` names it by its
  route name, or as `desktop notification`, and never by URL, token, or body.
- A delivery that fails while the drain waits reports its usual
  "Failed to send …" warning. A delivery task that panics is reported the same
  way. Neither is returned as an error.
- Without an active Tokio runtime a send logs a warning and does nothing.

**Planned:** the `claudine` CLI drains on every ordinary exit, within a
10-second budget (`DELIVERY_DRAIN_BUDGET`) capped by `claudine handle`'s own
deadline, and keeps the command's exit code. Until that lands, a message sent
just before the CLI exits can still be lost.

## Desktop notifications

Desktop notifications are intentionally **not** a messaging route. They are
zero-config and triggered only via the lifecycle `notify` frontmatter action, so
they never appear in the config TUI's route management.
