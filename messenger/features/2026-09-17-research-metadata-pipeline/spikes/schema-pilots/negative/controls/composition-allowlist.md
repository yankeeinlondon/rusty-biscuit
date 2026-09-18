---
$schema: ../../_schema.yaml
schema_version: 0
platform_id: discord
created: 2026-09-17
last_updated: 2026-09-17
agent: claude-code
model: claude-opus-5
sources:
  - { id: src.a, kind: official_docs, url: "https://example.com/a", retrieved: 2026-09-17 }
interfaces:
  - { interface_id: discord.bot, role: sending_adapter, api_identity: Discord HTTP API, classification: official, direction: send_only, adapters: [discord] }
api_versions: []
chronology: []
constraints:
  - id: c.a
    interface: discord.bot
    operation: create_message
    surface: body
    native_locator: content
    kind: hard_max
    value: 2000
    unit: unspecified_characters
    measurement_stage: field_value
    enforced_by: service
    overflow_behavior: unknown
    applies_when: []
    knowledge: { state: known, evidence: [src.a] }
format_profiles: []
text_bindings: []
image_bindings: []
role_coverage: []
envelopes: []
errors: []
error_fixtures: []
capabilities: []
inbound_bindings: []
question_bindings: []
form_bindings: []
changes: []
gaps: []
requires_messenger_update: false
prompt: "Research {{platform}}"
hash: abc123
---

Control: `prompt` and `hash` are allowlisted top-level keys. Expected: valid.
