# Signal Platform Guide

The `signal` provider delivers messages via `signal-cli`'s JSON-RPC interface. It is a single adapter (`SignalProvider`) that sends plain text messages to users, groups, or note-to-self.

| Backend | Transport | Interface |
|---------|-----------|-----------|
| `signal-cli` daemon | `reqwest` | JSON-RPC 2.0 over HTTP |

Signal is **account-centric**, not token-centric. The provider connects to a locally running `signal-cli` daemon that manages a registered or linked Signal account.

## Capability Summary

- `supports_markdown_rendering`: `false` — rendered to plain text
- `supports_reply`: `true` — quoted replies via `quoteAuthor` + `quoteTimestamp`
- `supported_attachment_kinds`: `{}` — attachments drop in best-effort and error in strict
- `supports_location`: `true` — rendered as text fallback
- `supports_silent_delivery`: `false`
- `supports_link_preview_control`: `false`

## Enabling

Library:

```toml
[dependencies]
messenger = { version = "0.1", features = ["signal"] }
```

CLI: `messenger-cli` enables `signal` by default.

## Quick Test

Prerequisites: `signal-cli` daemon running on `http://localhost:7583` with a registered account.

```bash
messenger send --provider signal --signal-account "+15551234567" --recipient "+15559876543" "Hello from messenger"
```

## Authentication

Signal has **no OAuth or bot tokens**. Authentication is implicit: the `signal-cli` daemon holds the account state (registration, keys, device linking), and the provider merely talks to the daemon over HTTP.

Required configuration:

| Field | Description |
|-------|-------------|
| `rpc_url` | URL of the `signal-cli` JSON-RPC daemon, e.g. `http://localhost:7583` |
| `account` | The registered Signal account phone number or UUID, e.g. `+15551234567` |

Library usage:

```rust
use messenger::prelude::*;

let provider = SignalProvider::new(SignalConfig {
    rpc_url: "http://localhost:7583".to_string(),
    account: "+15551234567".to_string(),
});
```

## Setup Walkthrough

1. **Install `signal-cli`** — see [the official repo](https://github.com/AsamK/signal-cli)
2. **Register or link an account**:
   ```bash
   # Register a new phone number
   signal-cli register --voice  # or --captcha if required
   signal-cli verify 123456  # SMS code

   # OR link as a secondary device
   signal-cli link --name "messenger"
   # Scan the QR code with Signal mobile app
   ```
3. **Start the JSON-RPC daemon**:
   ```bash
   signal-cli daemon --http --http-host localhost --http-port 7583
   ```
4. **Verify the daemon is reachable**:
   ```bash
   curl -X POST http://localhost:7583 \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","method":"getUserStatus","params":{"account":"+15551234567","recipient":["+15559876543"]},"id":1}'
   ```

## Target Types

The `SignalTarget` enum supports three destination types:

| Target | `Target` constructor | JSON-RPC method |
|--------|---------------------|-----------------|
| Individual user | `Target::signal_user(SignalAddress::Phone("+1555...".into()))` | `send` |
| Group | `Target::signal_group("base64groupid...".into())` | `sendGroupMessage` |
| Note to self | `Target::Signal(SignalTarget::NoteToSelf)` | `send` with `noteToSelf: true` |

Recipients can be phone numbers (`SignalAddress::Phone`) or Signal UUIDs (`SignalAddress::Uuid`).

## Field Mapping

| Portable | signal-cli JSON-RPC |
|----------|---------------------|
| `body` (plain text) | `message` field |
| `location` | appended to `message` as text fallback |
| `reply_to` (`MessageRef::Signal { author, timestamp_ms }`) | `quoteAuthor`, `quoteTimestamp` |

## Reply Mechanics

Signal replies are **quoted replies**, not threaded conversations. To reply to a message, you need:

- `author` — the phone number or UUID of the original message sender
- `timestamp_ms` — the Unix timestamp (in milliseconds) of the original message

The provider extracts these from `MessageRef::Signal` and sets `quoteAuthor` / `quoteTimestamp` in the JSON-RPC params.

## Receipts

```json
{
  "provider": "Signal",
  "message_ref": {
    "Signal": {
      "thread": {
        "Direct": "+15559876543"
      },
      "author": {
        "Phone": "+15551234567"
      },
      "timestamp_ms": 1712345678000
    }
  },
  "raw_id": "1712345678000"
}
```

The `raw_id` is the server timestamp in milliseconds. Signal uses `author + timestamp` as the composite message identifier for replies, reactions, and receipts.

## Important Gotchas

### Daemon Must Stay Current

`signal-cli` warns that Signal server changes can break releases older than ~3 months. Keep `signal-cli` updated and smoke-test send/receive regularly.

### You Must Receive Messages

Signal state sync depends on receiving updates. If the daemon is not running or not receiving, message delivery may degrade. Run in daemon mode or maintain a deliberate receive loop.

### Auto-Receive Can "Steal" Messages

If you have multiple consumers receiving from the same account, they compete for messages. Use one clear consumer path.

### UTF-16 Text Offsets

Signal mention and style offsets are defined in UTF-16 code units. If you build rich text handling on top of Signal, compute spans in UTF-16, not Rust byte offsets or `.chars()` indices.

### Recipient Identity Is an Enum

Signal supports phone numbers, usernames, ACI, and PNI. Model recipient identity as `SignalAddress`, not a plain string.

## Troubleshooting

- **`Transport` error connecting to signal-cli** — Is the daemon running? Check `signal-cli daemon --http` is active on the expected host/port.
- **`Provider` error with JSON-RPC error** — The daemon rejected the request. Common causes: unregistered account, invalid recipient, group not found, or rate limiting.
- **Messages not delivering** — Ensure the recipient accepts messages from your account. Signal may silently drop messages from unknown senders depending on privacy settings.
- **`InvalidMessage: expected Signal target`** — The `Target` enum variant must be `Signal`, not another provider type.

## Related Documents

- [User Guide](../user-guide.md) — platform setup, CLI configuration, library usage.
- [messenger README](../../README.md) — high-level package overview.
- [messenger-cli README](../../cli/README.md) — CLI flags, route shapes, setup flow.
- [Research: Signal API Deep Dive](../../docs/research/platforms/signal.md) — full API research notes.
