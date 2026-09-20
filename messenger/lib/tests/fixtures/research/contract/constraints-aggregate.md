---
# An aggregate budget across repeated rich-object fields plus an individual
# field bound that applies simultaneously. Criterion 4. Expect: valid.
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
- interface: discord_bot_api
  categories:
    versions:
      status: gap
      gap: gap.fx.scope
    constraints:
      status: researched
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
constraints:
- id: c.fx.rich.total
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
  - surface: rich_field_name
    native_locator: embeds[].fields[].name
    repeated: true
  - surface: rich_field_value
    native_locator: embeds[].fields[].value
    repeated: true
  - surface: rich_footer_text
    native_locator: embeds[].footer.text
    repeated: true
  - surface: rich_author_name
    native_locator: embeds[].author.name
    repeated: true
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: c.fx.rich.description
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
    - src.fx.docs
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
