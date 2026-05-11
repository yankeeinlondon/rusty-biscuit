# WhatsApp Platform Guide

The `whatsapp` provider delivers messages via Meta's **WhatsApp Business Platform Cloud API**. It is a single adapter (`WhatsAppProvider`) that sends text and location messages to individual recipients.

| Host | Backend | Delivery path |
|------|---------|---------------|
| Meta Cloud API | `reqwest` | `POST https://graph.facebook.com/{version}/{phone-number-id}/messages` |

WhatsApp uses **recipient-based** targeting: every send specifies a phone number. There is no channel or group concept at the API level (group messaging is handled through separate template mechanisms not covered in v1).

## Capability Summary

- `supports_markdown_rendering`: `false` — rendered to plain text
- `supports_reply`: `true`
- `supported_attachment_kinds`: `{}` — attachments drop in best-effort and error in strict
- `supports_location`: `true` — native `location` message type
- `supports_silent_delivery`: `false`
- `supports_link_preview_control`: `false`

## Enabling

Library:

```toml
[dependencies]
messenger = { version = "0.1", features = ["whatsapp"] }
```

CLI: `messenger-cli` enables `whatsapp` by default.

## Quick Test

```bash
export WHATSAPP_ACCESS_TOKEN="your-access-token"
export WHATSAPP_PHONE_NUMBER_ID="123456789012345"
messenger send --provider whatsapp --recipient "15551234567" "Hello from messenger"
```

## Authentication

WhatsApp Cloud API uses **Bearer token** authentication via Meta's Graph API.

Required credentials:

| Credential | Source |
|------------|--------|
| `access_token` | Meta Business Manager → System User token, or temporary User token |
| `phone_number_id` | WhatsApp Business Account → Phone Numbers → ID |

The provider defaults to Graph API version `v23.0`. Override via `api_version` in config or `api_base_url` for testing with wiremock.

Library usage:

```rust
use messenger::prelude::*;
use secrecy::SecretString;

let provider = WhatsAppProvider::new(WhatsAppConfig {
    access_token: SecretString::from(std::env::var("WHATSAPP_ACCESS_TOKEN").unwrap()),
    phone_number_id: "123456789012345".to_string(),
    api_version: None,
    api_base_url: None,
});
```

## Setup Walkthrough

1. **Create a Meta Business Account** at [business.facebook.com](https://business.facebook.com)
2. **Add a WhatsApp Business Account** (WABA) and verify a phone number
3. **Generate an access token**:
   - For testing: use a User access token from the Graph API Explorer (expires in ~24 hours)
   - For production: create a System User with `whatsapp_business_messaging` permission and generate a permanent token
4. **Get the phone number ID** from the WhatsApp Business Manager dashboard
5. **Subscribe your app** to the WABA if you need inbound webhooks:
   ```bash
   curl -X POST "https://graph.facebook.com/v23.0/{WABA-ID}/subscribed_apps" \
     -H "Authorization: Bearer {token}"
   ```

## Field Mapping

| Portable | WhatsApp Cloud API |
|----------|-------------------|
| `body` (plain text) | `text.body` |
| `location` | `location` object with `latitude`, `longitude`, `name`, `address` |
| `reply_to` (`MessageRef::WhatsApp { message_id }`) | `context.message_id` |
| `messaging_product` | always `"whatsapp"` |

When a location is present, the provider sends a `type: "location"` message instead of `type: "text"`. Text and location are mutually exclusive in a single send.

## The 24-Hour Messaging Window

A critical WhatsApp rule: after a user last messaged you, you have a **24-hour session window** to send free-form text messages. Outside that window, plain text messages may fail to deliver. You must use an **approved template message** instead.

The current provider does not implement template messaging. If you need to message users outside the 24-hour window, you must:

1. Create message templates in the WhatsApp Business Manager
2. Get them approved by Meta
3. Send using the template mechanism (not yet supported by this provider)

## Receipts

```json
{
  "provider": "WhatsApp",
  "message_ref": {
    "WhatsApp": {
      "message_id": "wamid.HBgM..."
    }
  },
  "raw_id": "wamid.HBgM..."
}
```

WhatsApp message IDs are `wamid.` prefixed strings. Preserve these for replies and status tracking.

## Error Handling

| Error code | Meaning | Action |
|------------|---------|--------|
| `190` | Authentication failed | Regenerate access token; check token has not expired |
| `13` | Rate limited | Back off and retry |
| Various Graph errors | Invalid recipient, message type not allowed | Check 24-hour window; verify phone number format |

## Troubleshooting

- **`Authentication` error with code 190** — The access token is invalid or expired. User tokens expire after ~24 hours; move to a System User token for production.
- **Message sends successfully but user does not receive it** — Check the 24-hour window. If expired, the message may be silently dropped by WhatsApp. Use the WhatsApp Business Manager to check message status.
- **No inbound messages arriving** — Ensure your app is subscribed to the WABA (`POST /{WABA-ID}/subscribed_apps`). Webhooks alone are not enough.
- **`InvalidMessage: expected WhatsApp target`** — The `Target` enum variant must be `WhatsApp`, not another provider type.
- **Phone number format** — Use E.164 format (`15551234567`) without `+` prefix in the recipient field.

## Related Documents

- [User Guide](../user-guide.md) — platform setup, CLI configuration, library usage.
- [messenger README](../../README.md) — high-level package overview.
- [messenger-cli README](../../cli/README.md) — CLI flags, route shapes, setup flow.
- [Research: WhatsApp API Deep Dive](../../docs/research/platforms/whatsapp.md) — full API research notes.
