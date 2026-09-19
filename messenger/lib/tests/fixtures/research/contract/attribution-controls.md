---
# Platform-derived sender identity, a configuration-level default, a
# conditional per-message override, a per-message avatar, and a content-author
# label; none changes the authenticated sender. Criterion 17. Expect: valid.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: discord
created: &id001 2026-09-17
last_updated: *id001
agent: fixture
model: none
sources:
- id: src.fx.docs
  kind: official_docs
  url: https://docs.example.com/api/messages
  locator: Limits
  retrieved: *id001
- id: src.fx.sdk
  kind: sdk_validator
  location: fixtures/sdk/validate.rs
  locator: MAX_LEN
  revision: 0.17.0
  retrieved: *id001
- id: src.fx.code
  kind: source_code
  url: https://git.example.com/bridge/src/send.rs
  locator: send()
  revision: abc1234
  retrieved: *id001
- id: src.fx.observed
  kind: observed_fixture
  location: messenger/lib/tests/fixtures/research/diagnostics/observed.json
  retrieved: *id001
- id: src.fx.forum
  kind: secondary
  url: https://forum.example.com/t/limits
  retrieved: *id001
  note: community lead only
interfaces:
- interface_id: discord_webhook
  role: sending_adapter
  api_identity: Fixture discord_webhook
  classification: official
  direction: send_only
  adapters:
  - discord-webhook
  operations:
  - execute_webhook
- interface_id: discord_bot_api
  role: sending_adapter
  api_identity: Fixture discord_bot_api
  classification: official
  direction: send_only
  adapters:
  - discord
  operations:
  - create_message
coverage:
- interface: discord_webhook
  categories:
    versions:
      status: gap
      gap: gap.fx.scope
    constraints:
      status: gap
      gap: gap.fx.scope
    formatting:
      status: gap
      gap: gap.fx.scope
    text_bindings:
      status: gap
      gap: gap.fx.scope
    images:
      status: gap
      gap: gap.fx.scope
    attachments:
      status: gap
      gap: gap.fx.scope
    addressing:
      status: gap
      gap: gap.fx.scope
    receipts:
      status: gap
      gap: gap.fx.scope
    attribution:
      status: researched
    location:
      status: gap
      gap: gap.fx.scope
    expression:
      status: gap
      gap: gap.fx.scope
    interactivity:
      status: gap
      gap: gap.fx.scope
    delivery_controls:
      status: gap
      gap: gap.fx.scope
    eligibility:
      status: gap
      gap: gap.fx.scope
    rate_limits:
      status: gap
      gap: gap.fx.scope
    errors:
      status: gap
      gap: gap.fx.scope
- interface: discord_bot_api
  categories:
    versions:
      status: gap
      gap: gap.fx.scope
    constraints:
      status: gap
      gap: gap.fx.scope
    formatting:
      status: gap
      gap: gap.fx.scope
    text_bindings:
      status: gap
      gap: gap.fx.scope
    images:
      status: gap
      gap: gap.fx.scope
    attachments:
      status: gap
      gap: gap.fx.scope
    addressing:
      status: gap
      gap: gap.fx.scope
    receipts:
      status: gap
      gap: gap.fx.scope
    attribution:
      status: researched
    location:
      status: gap
      gap: gap.fx.scope
    expression:
      status: gap
      gap: gap.fx.scope
    interactivity:
      status: gap
      gap: gap.fx.scope
    delivery_controls:
      status: gap
      gap: gap.fx.scope
    eligibility:
      status: gap
      gap: gap.fx.scope
    rate_limits:
      status: gap
      gap: gap.fx.scope
    errors:
      status: gap
      gap: gap.fx.scope
api_versions: []
chronology: []
constraints: []
format_profiles: []
text_bindings: []
image_bindings: []
role_coverage: []
attachment_bindings: []
addressing: []
receipts: []
attribution_bindings:
- id: attr.fx.bot_identity
  interface: discord_bot_api
  operation: create_message
  role: sender_identity
  native_fields:
  - author (response)
  control: platform_derived
  scope: application
  inclusion: always
  override_behavior: not_applicable
  changes_sender: 'no'
  provenance_presentation: Recipients see a BOT label.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: attr.fx.webhook_default
  interface: discord_webhook
  operation: execute_webhook
  role: sender_display_name
  native_fields:
  - webhook name
  control: account_configured
  scope: webhook
  inclusion: always
  override_behavior: not_applicable
  changes_sender: 'no'
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: attr.fx.webhook_override
  interface: discord_webhook
  operation: execute_webhook
  role: sender_display_name
  native_fields:
  - username
  control: caller_supplied
  scope: message
  inclusion: optional
  override_behavior: conditional
  changes_sender: 'no'
  applies_when:
  - kind: message_form
    equals: webhook_owned_by_application
  provenance_presentation: The override changes the label only; the sender remains the webhook.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: attr.fx.webhook_avatar
  interface: discord_webhook
  operation: execute_webhook
  role: sender_avatar
  native_fields:
  - avatar_url
  control: caller_supplied
  scope: message
  inclusion: optional
  override_behavior: honored
  changes_sender: 'no'
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: attr.fx.embed_author
  interface: discord_bot_api
  operation: create_message
  role: content_author
  native_fields:
  - embeds[].author.name
  control: caller_supplied
  scope: message
  inclusion: optional
  override_behavior: not_applicable
  changes_sender: 'no'
  provenance_presentation: A caller label inside a card, not an authenticated identity.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
location_bindings: []
author_geolocation: []
expression_bindings: []
delivery_controls: []
eligibility: []
rate_limits: []
envelopes: []
errors: []
error_fixtures: []
inbound_bindings: []
question_bindings: []
form_bindings: []
interaction_fixtures: []
changes: []
gaps:
- id: gap.fx.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.fx.docs
  unresolved_reason: The fixture exercises one contract representation only.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Fixture

Contract fixture; not research.
