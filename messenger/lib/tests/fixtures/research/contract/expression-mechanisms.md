---
# An API-addressable effect, an app-only effect, a reaction, sticker content,
# and an automatic client effect, with approximate and unmapped intent
# mappings. Criterion 19. Expect: valid.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: telegram
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
- interface_id: telegram_bot_api
  role: sending_adapter
  api_identity: Fixture telegram_bot_api
  classification: official
  direction: send_only
  adapters:
  - telegram
  operations:
  - send_message
  - set_message_reaction
  - send_sticker
coverage:
- interface: telegram_bot_api
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
      status: gap
      gap: gap.fx.scope
    location:
      status: gap
      gap: gap.fx.scope
    expression:
      status: researched
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
attribution_bindings: []
location_bindings: []
author_geolocation: []
expression_bindings:
- id: ex.fx.effect
  interface: telegram_bot_api
  operation: send_message
  mechanism: message_effect
  support: conditional
  api_availability: api
  control: caller_chosen
  scope: message_bubble
  discovery: fixed_documented
  intent: celebration
  intent_fidelity: approximate
  native_identifier: '5046509860389126442'
  native_fields:
  - message_effect_id
  applies_when:
  - kind: conversation_type
    equals: private
  rendering: Recipient clients without support show no effect.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: ex.fx.app_only
  interface: telegram_bot_api
  operation: send_message
  mechanism: text_effect
  api_availability: app_only
  control: caller_chosen
  scope: selected_text
  discovery: unknown
  intent: emphasis
  intent_fidelity: approximate
  knowledge:
    state: known
    evidence:
    - src.fx.docs
    explanation: Visible in the consumer app; a research lead, not API support.
- id: ex.fx.reaction
  interface: telegram_bot_api
  operation: set_message_reaction
  mechanism: reaction
  support: supported
  api_availability: api
  control: caller_chosen
  scope: not_applicable
  discovery: fixed_documented
  intent: unmapped
  intent_fidelity: unmapped
  native_fields:
  - reaction
  rendering: Targets an existing message; not an effect on the send.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: ex.fx.sticker
  interface: telegram_bot_api
  operation: send_sticker
  mechanism: sticker_content
  support: supported
  api_availability: api
  control: caller_chosen
  scope: not_applicable
  discovery: dynamic_catalog
  intent: unmapped
  intent_fidelity: unmapped
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: ex.fx.auto
  interface: telegram_bot_api
  operation: send_message
  mechanism: message_effect
  api_availability: unknown
  control: content_triggered
  scope: conversation
  discovery: unknown
  intent: celebration
  intent_fidelity: approximate
  knowledge:
    state: unknown
    evidence: []
    gap: gap.fx.scope
    explanation: Keyword-triggered client animation; not a deterministic send capability.
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
