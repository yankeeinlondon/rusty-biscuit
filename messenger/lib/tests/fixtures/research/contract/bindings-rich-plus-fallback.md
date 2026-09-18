---
# Rich structured content plus a plain notification fallback that is not a
# second visible message. Criterion 15. Expect: valid.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: slack
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
- interface_id: slack_web_api
  role: sending_adapter
  api_identity: Fixture slack_web_api
  classification: official
  direction: send_only
  adapters:
  - slack
  operations:
  - chat_post_message
coverage:
- interface: slack_web_api
  categories:
    versions:
      status: gap
      gap: gap.fx.scope
    constraints:
      status: gap
      gap: gap.fx.scope
    formatting:
      status: researched
    text_bindings:
      status: researched
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
constraints: []
format_profiles:
- id: fmt.fx.mrkdwn
  family: provider_markup
  constructs:
    emphasis:
      state: unknown
    strong:
      state: unknown
    strikethrough:
      state: unknown
    underline:
      state: unknown
    inline_code:
      state: unknown
    fenced_code:
      state: unknown
    links:
      state: unknown
    images:
      state: unknown
    headings:
      state: unknown
    lists:
      state: unknown
    block_quotes:
      state: unknown
    tables:
      state: unknown
    spoilers:
      state: unknown
    mentions:
      state: unknown
  auto_interpretation: unknown
  malformed_behavior: unknown
  knowledge:
    state: known
    evidence:
    - src.fx.docs
  dialect: mrkdwn
- id: fmt.fx.plain
  family: plain_text
  constructs:
    emphasis:
      state: unknown
    strong:
      state: unknown
    strikethrough:
      state: unknown
    underline:
      state: unknown
    inline_code:
      state: unknown
    fenced_code:
      state: unknown
    links:
      state: unknown
    images:
      state: unknown
    headings:
      state: unknown
    lists:
      state: unknown
    block_quotes:
      state: unknown
    tables:
      state: unknown
    spoilers:
      state: unknown
    mentions:
      state: unknown
  auto_interpretation: unknown
  malformed_behavior: unknown
  knowledge:
    state: known
    evidence:
    - src.fx.docs
text_bindings:
- id: tb.fx.blocks
  interface: slack_web_api
  operation: chat_post_message
  surface: blocks
  native_locator: blocks
  representation: structured_blocks
  content_role: primary
  packaging: structured_object
  visibility: visible_content
  profiles:
  - fmt.fx.mrkdwn
  relationships:
  - kind: optional_companion
    target: tb.fx.fallback
  knowledge:
    state: known
    evidence:
    - src.fx.docs
- id: tb.fx.fallback
  interface: slack_web_api
  operation: chat_post_message
  surface: notification_fallback
  native_locator: text
  representation: text
  content_role: notification_fallback
  packaging: separate_fields
  visibility: notification_only
  profiles:
  - fmt.fx.plain
  relationships:
  - kind: fallback_for
    target: tb.fx.blocks
  precedence: With blocks present, text is used only for notifications.
  knowledge:
    state: known
    evidence:
    - src.fx.docs
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
