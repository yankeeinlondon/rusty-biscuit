---
# Accepted-scope fleet fixture for discord: every roster interface, a complete
# coverage matrix resting on one investigated gap, and a few researched
# constraints. Used as a publication baseline; not research. Expect: valid
# in Accepted scope.
$schema: ../../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: discord
created: 2026-09-01
last_updated: 2026-09-10
agent: fixture
model: none
sources:
- id: src.discord.docs
  kind: official_docs
  url: https://docs.example.com/discord/messages
  locator: Limits
  retrieved: 2026-09-10
interfaces:
- interface_id: discord_bot_api
  role: sending_adapter
  api_identity: Fixture discord_bot_api
  classification: official
  direction: send_only
  adapters:
  - discord
  operations:
  - create_message
- interface_id: discord_webhook
  role: sending_adapter
  api_identity: Fixture discord_webhook
  classification: official
  direction: send_only
  adapters:
  - discord-webhook
  operations:
  - execute_webhook
- interface_id: discord_gateway
  role: research_only
  api_identity: Fixture discord_gateway
  classification: official
  direction: receive_only
  adapters: []
  operations: []
  relationships:
  - kind: receives_events_for
    target: discord_bot_api
- interface_id: discord_interactions
  role: research_only
  api_identity: Fixture discord_interactions
  classification: official
  direction: callback_only
  adapters: []
  operations: []
  relationships:
  - kind: responds_to_interactions_from
    target: discord_bot_api
coverage:
- interface: discord_bot_api
  categories:
    versions:
      status: gap
      gap: gap.discord.scope
    constraints:
      status: researched
    formatting:
      status: gap
      gap: gap.discord.scope
    text_bindings:
      status: gap
      gap: gap.discord.scope
    images:
      status: gap
      gap: gap.discord.scope
    attachments:
      status: gap
      gap: gap.discord.scope
    addressing:
      status: gap
      gap: gap.discord.scope
    receipts:
      status: gap
      gap: gap.discord.scope
    attribution:
      status: gap
      gap: gap.discord.scope
    location:
      status: gap
      gap: gap.discord.scope
    expression:
      status: gap
      gap: gap.discord.scope
    interactivity:
      status: gap
      gap: gap.discord.scope
    delivery_controls:
      status: gap
      gap: gap.discord.scope
    eligibility:
      status: gap
      gap: gap.discord.scope
    rate_limits:
      status: gap
      gap: gap.discord.scope
    errors:
      status: gap
      gap: gap.discord.scope
- interface: discord_webhook
  categories:
    versions:
      status: gap
      gap: gap.discord.scope
    constraints:
      status: gap
      gap: gap.discord.scope
    formatting:
      status: gap
      gap: gap.discord.scope
    text_bindings:
      status: gap
      gap: gap.discord.scope
    images:
      status: gap
      gap: gap.discord.scope
    attachments:
      status: gap
      gap: gap.discord.scope
    addressing:
      status: gap
      gap: gap.discord.scope
    receipts:
      status: gap
      gap: gap.discord.scope
    attribution:
      status: gap
      gap: gap.discord.scope
    location:
      status: gap
      gap: gap.discord.scope
    expression:
      status: gap
      gap: gap.discord.scope
    interactivity:
      status: gap
      gap: gap.discord.scope
    delivery_controls:
      status: gap
      gap: gap.discord.scope
    eligibility:
      status: gap
      gap: gap.discord.scope
    rate_limits:
      status: gap
      gap: gap.discord.scope
    errors:
      status: gap
      gap: gap.discord.scope
- interface: discord_gateway
  categories:
    versions:
      status: gap
      gap: gap.discord.scope
    constraints:
      status: gap
      gap: gap.discord.scope
    formatting:
      status: gap
      gap: gap.discord.scope
    text_bindings:
      status: gap
      gap: gap.discord.scope
    images:
      status: gap
      gap: gap.discord.scope
    attachments:
      status: gap
      gap: gap.discord.scope
    addressing:
      status: gap
      gap: gap.discord.scope
    receipts:
      status: gap
      gap: gap.discord.scope
    attribution:
      status: gap
      gap: gap.discord.scope
    location:
      status: gap
      gap: gap.discord.scope
    expression:
      status: gap
      gap: gap.discord.scope
    interactivity:
      status: gap
      gap: gap.discord.scope
    delivery_controls:
      status: gap
      gap: gap.discord.scope
    eligibility:
      status: gap
      gap: gap.discord.scope
    rate_limits:
      status: gap
      gap: gap.discord.scope
    errors:
      status: gap
      gap: gap.discord.scope
- interface: discord_interactions
  categories:
    versions:
      status: gap
      gap: gap.discord.scope
    constraints:
      status: gap
      gap: gap.discord.scope
    formatting:
      status: gap
      gap: gap.discord.scope
    text_bindings:
      status: gap
      gap: gap.discord.scope
    images:
      status: gap
      gap: gap.discord.scope
    attachments:
      status: gap
      gap: gap.discord.scope
    addressing:
      status: gap
      gap: gap.discord.scope
    receipts:
      status: gap
      gap: gap.discord.scope
    attribution:
      status: gap
      gap: gap.discord.scope
    location:
      status: gap
      gap: gap.discord.scope
    expression:
      status: gap
      gap: gap.discord.scope
    interactivity:
      status: gap
      gap: gap.discord.scope
    delivery_controls:
      status: gap
      gap: gap.discord.scope
    eligibility:
      status: gap
      gap: gap.discord.scope
    rate_limits:
      status: gap
      gap: gap.discord.scope
    errors:
      status: gap
      gap: gap.discord.scope
api_versions: []
chronology: []
constraints:
- id: c.discord.content.max
  interface: discord_bot_api
  operation: create_message
  surface: body
  native_locator: content
  kind: hard_max
  value: 2000
  unit: unicode_scalars
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: reject
  applies_when: []
  overflow_errors: []
  knowledge:
    state: known
    evidence:
    - src.discord.docs
    confidence: high
- id: c.discord.embed.description
  interface: discord_bot_api
  operation: create_message
  surface: rich_description
  native_locator: embeds[].description
  kind: hard_max
  value: 4096
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: reject
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.discord.docs
- id: c.discord.embed.total
  interface: discord_bot_api
  operation: create_message
  surface: rich_objects
  native_locator: embeds[*]
  kind: aggregate_max
  value: 6000
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: reject
  applies_when: []
  aggregation_scope: message
  members:
  - surface: rich_title
    native_locator: embeds[].title
    repeated: true
  - surface: rich_description
    native_locator: embeds[].description
    repeated: true
  knowledge:
    state: known
    evidence:
    - src.discord.docs
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
- id: gap.discord.scope
  kind: research
  status: investigated
  question: Categories outside this fixture's subject are deliberately not researched.
  facts: []
  searches:
  - 'none: fixture scope'
  inspected_sources:
  - src.discord.docs
  unresolved_reason: The fixture exercises publication, not research.
  blocked_decision: None; fixtures are not research.
  next_investigation: None.
requires_messenger_update: false
---

# Discord fleet fixture

Publication fixture; not research.
