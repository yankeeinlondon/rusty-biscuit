---
# Schema-pilot slack record ported to schema version 1 (Phase 2 freeze gate).
# Not research: values come from the Phase 1 pilot. Categories the pilot did
# not exercise point at `gap.slack.pilot_scope`.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: slack
created: 2026-09-17
last_updated: 2026-09-17
agent: claude-code
model: claude-opus-5
sources:
- id: src.slack.post_message
  kind: official_docs
  url: https://docs.slack.dev/reference/methods/chat.postMessage/#truncating-content
  locator: Truncating content
  retrieved: 2026-09-17
  note: spec spot-check
- id: src.slack.prose
  kind: secondary
  location: messenger/docs/research/platforms/slack.md
  locator: APIs; Why text and blocks
  retrieved: 2026-03-09
interfaces:
- interface_id: slack_web_api
  role: sending_adapter
  api_identity: Slack Web API chat.postMessage
  endpoint_template: POST https://slack.com/api/chat.postMessage
  classification: official
  direction: send_only
  adapters:
  - slack
  relationships:
  - kind: requires_companion_interface
    target: slack_events_api
    prerequisites:
    - event subscriptions
    - bot scopes
  operations:
  - chat_post_message
- interface_id: slack_incoming_webhook
  role: sending_adapter
  api_identity: Slack incoming webhook
  endpoint_template: POST https://hooks.slack.com/services/{webhook_path}
  classification: official
  direction: send_only
  adapters:
  - slack-webhook
  operations:
  - post
- interface_id: slack_events_api
  role: research_only
  api_identity: Slack Events API
  classification: official
  direction: receive_only
  adapters: []
  relationships:
  - kind: receives_events_for
    target: slack_web_api
    prerequisites:
    - public request URL
    - signing secret verification
  operations: []
coverage:
- interface: slack_web_api
  categories:
    versions:
      status: researched
    constraints:
      status: researched
    formatting:
      status: researched
    text_bindings:
      status: researched
    images:
      status: researched
    attachments:
      status: gap
      gap: gap.slack.pilot_scope
    addressing:
      status: gap
      gap: gap.slack.pilot_scope
    receipts:
      status: gap
      gap: gap.slack.pilot_scope
    attribution:
      status: gap
      gap: gap.slack.pilot_scope
    location:
      status: gap
      gap: gap.slack.pilot_scope
    expression:
      status: gap
      gap: gap.slack.pilot_scope
    interactivity:
      status: gap
      gap: gap.slack.pilot_scope
    delivery_controls:
      status: gap
      gap: gap.slack.pilot_scope
    eligibility:
      status: gap
      gap: gap.slack.pilot_scope
    rate_limits:
      status: gap
      gap: gap.slack.pilot_scope
    errors:
      status: researched
- interface: slack_incoming_webhook
  categories:
    versions:
      status: researched
    constraints:
      status: gap
      gap: gap.slack.pilot_scope
    formatting:
      status: gap
      gap: gap.slack.pilot_scope
    text_bindings:
      status: gap
      gap: gap.slack.pilot_scope
    images:
      status: gap
      gap: gap.slack.pilot_scope
    attachments:
      status: gap
      gap: gap.slack.pilot_scope
    addressing:
      status: gap
      gap: gap.slack.pilot_scope
    receipts:
      status: gap
      gap: gap.slack.pilot_scope
    attribution:
      status: gap
      gap: gap.slack.pilot_scope
    location:
      status: gap
      gap: gap.slack.pilot_scope
    expression:
      status: gap
      gap: gap.slack.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.slack.pilot_scope
    eligibility:
      status: gap
      gap: gap.slack.pilot_scope
    rate_limits:
      status: gap
      gap: gap.slack.pilot_scope
    errors:
      status: researched
- interface: slack_events_api
  categories:
    versions:
      status: gap
      gap: gap.slack.pilot_scope
    constraints:
      status: gap
      gap: gap.slack.pilot_scope
    formatting:
      status: gap
      gap: gap.slack.pilot_scope
    text_bindings:
      status: gap
      gap: gap.slack.pilot_scope
    images:
      status: gap
      gap: gap.slack.pilot_scope
    attachments:
      status: gap
      gap: gap.slack.pilot_scope
    addressing:
      status: gap
      gap: gap.slack.pilot_scope
    receipts:
      status: gap
      gap: gap.slack.pilot_scope
    attribution:
      status: gap
      gap: gap.slack.pilot_scope
    location:
      status: gap
      gap: gap.slack.pilot_scope
    expression:
      status: gap
      gap: gap.slack.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.slack.pilot_scope
    eligibility:
      status: gap
      gap: gap.slack.pilot_scope
    rate_limits:
      status: gap
      gap: gap.slack.pilot_scope
    errors:
      status: gap
      gap: gap.slack.pilot_scope
api_versions:
- id: ver.slack.web_api
  interface: slack_web_api
  versioning: unresearched
  knowledge:
    state: unknown
    evidence: []
    gap: gap.slack.versions
  subject: provider_api
- id: ver.slack.incoming_webhook
  interface: slack_incoming_webhook
  versioning: unresearched
  knowledge:
    state: unknown
    evidence: []
    gap: gap.slack.versions
  subject: provider_api
chronology: []
constraints:
- id: c.slack.text.recommended
  interface: slack_web_api
  operation: chat_post_message
  surface: body
  native_locator: text
  kind: recommended_max
  value: 4000
  unit: unspecified_characters
  measurement_stage: unknown
  recommended_by: src.slack.post_message
  overflow_behavior: unknown
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.slack.post_message
    confidence: medium
- id: c.slack.text.truncation
  interface: slack_web_api
  operation: chat_post_message
  surface: body
  native_locator: text
  kind: hard_max
  value: 40000
  unit: unspecified_characters
  measurement_stage: unknown
  enforced_by: service
  overflow_behavior: truncate
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.slack.post_message
    confidence: medium
    explanation: Beyond 40,000 characters the service truncates rather than rejects.
- id: c.slack.blocks.section_text
  interface: slack_web_api
  operation: chat_post_message
  surface: block_text
  native_locator: blocks[type=section].text.text
  kind: hard_max
  unit: unknown
  measurement_stage: unknown
  overflow_behavior: unknown
  applies_when:
  - kind: message_form
    equals: blocks
  knowledge:
    state: unknown
    evidence:
    - src.slack.post_message
    explanation: Block limits depend on block type.
    gap: gap.slack.blocks
format_profiles:
- id: fmt.slack.mrkdwn
  family: provider_markup
  dialect: Slack mrkdwn
  malformed_behavior: unknown
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
  knowledge:
    state: unknown
    evidence: []
    gap: gap.slack.mrkdwn
  auto_interpretation: unknown
text_bindings:
- id: tb.slack.text.primary
  interface: slack_web_api
  operation: chat_post_message
  surface: body
  native_locator: text
  representation: markup
  content_role: primary
  profiles:
  - fmt.slack.mrkdwn
  constraints:
  - c.slack.text.recommended
  - c.slack.text.truncation
  knowledge:
    state: known
    evidence:
    - src.slack.prose
    confidence: low
    explanation: Applies when blocks are absent.
  packaging: unknown
  visibility: visible_content
- id: tb.slack.blocks
  interface: slack_web_api
  operation: chat_post_message
  surface: blocks
  native_locator: blocks
  representation: structured_blocks
  content_role: primary
  profiles:
  - fmt.slack.mrkdwn
  relationships:
  - kind: optional_companion
    target: tb.slack.text.fallback
  constraints:
  - c.slack.blocks.section_text
  knowledge:
    state: known
    evidence:
    - src.slack.prose
    confidence: low
  packaging: unknown
  visibility: visible_content
- id: tb.slack.text.fallback
  interface: slack_web_api
  operation: chat_post_message
  surface: notification_fallback
  native_locator: text
  representation: markup
  content_role: notification_fallback
  profiles:
  - fmt.slack.mrkdwn
  relationships:
  - kind: fallback_for
    target: tb.slack.blocks
  precedence: With blocks present, text feeds notifications and accessibility rather than a second visible message.
  constraints:
  - c.slack.text.recommended
  - c.slack.text.truncation
  knowledge:
    state: known
    evidence:
    - src.slack.prose
    confidence: low
  packaging: unknown
  visibility: notification_only
image_bindings:
- id: img.slack.section_accessory
  interface: slack_web_api
  operation: chat_post_message
  container_surface: blocks
  role: accessory
  fidelity: exact
  native_locator: blocks[type=section].accessory (image element)
  placement: section_accessory
  placement_control: explicit
  submissions:
  - request_field
  sources:
  - remote_url
  collection: single
  supplied_by: caller_supplied
  knowledge:
    state: unknown
    evidence: []
    gap: gap.slack.images
  multiple_images: unknown
  ordering: unknown
  atomic: unknown
  receipts: unknown
role_coverage:
- interface: slack_web_api
  roles:
    inline_content:
      state: unknown
      bindings: []
      gap: gap.slack.images
    attachment:
      state: unknown
      bindings: []
      gap: gap.slack.images
    primary:
      state: unknown
      bindings: []
      gap: gap.slack.images
    thumbnail:
      state: unknown
      bindings: []
      gap: gap.slack.images
    accessory:
      state: unknown
      bindings:
      - img.slack.section_accessory
      gap: gap.slack.images
    author_icon:
      state: unknown
      bindings: []
      gap: gap.slack.images
    footer_icon:
      state: unknown
      bindings: []
      gap: gap.slack.images
    link_preview:
      state: unknown
      bindings: []
      gap: gap.slack.images
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
envelopes:
- id: env.slack.web_api.json
  interface: slack_web_api
  operations:
  - chat_post_message
  origin: service
  body_format: json
  success_discriminator: /ok
  code_locator: /error
  warnings_locator: /warning
  knowledge:
    state: unknown
    evidence: []
    explanation: Candidate locators pending official evidence; HTTP 200 may carry ok=false.
    gap: gap.slack.errors
- id: env.slack.webhook.text
  interface: slack_incoming_webhook
  operations:
  - post
  origin: service
  body_format: plain_text
  code_locator: body_text
  knowledge:
    state: unknown
    evidence: []
    gap: gap.slack.errors
errors:
- id: err.slack.web_api.msg_too_long
  interface: slack_web_api
  operations:
  - chat_post_message
  envelope: env.slack.web_api.json
  origin: service
  phase: response
  outcome: failure
  category: content_too_large
  candidate_match:
    http_status: 200
    discriminator_locator: /ok
    discriminator_equals: 'false'
    native_code_string: msg_too_long
  related_facts:
  - c.slack.text.truncation
  delivery_certainty: unknown
  recovery: unknown
  replay_safety: unknown
  knowledge:
    state: unknown
    evidence: []
    explanation: Application failure inside HTTP success; may be moot if the service truncates instead.
    gap: gap.slack.errors
  remediation: unknown
- id: err.slack.webhook.plain_text
  interface: slack_incoming_webhook
  operations:
  - post
  envelope: env.slack.webhook.text
  origin: service
  phase: response
  outcome: failure
  category: unknown
  candidate_match:
    http_status: 400
    exact_text_token: invalid_payload
  delivery_certainty: unknown
  recovery: unknown
  replay_safety: unknown
  knowledge:
    state: unknown
    evidence: []
    explanation: Exact plain-text tokens are eligible only once evidence shows they are stable identifiers.
    gap: gap.slack.errors
  remediation: unknown
error_fixtures: []
inbound_bindings:
- id: ib.slack.webhook.none
  interface: slack_incoming_webhook
  event: none
  mechanism: none
  reach:
  - none
  content: none
  knowledge:
    state: known
    evidence:
    - src.slack.prose
    confidence: low
    explanation: A one-way URL for posting.
  reuses_send_identity: unknown
- id: ib.slack.events.message
  interface: slack_events_api
  event: message
  mechanism: push_webhook
  reach:
  - unknown
  content: unknown
  reuses_send_identity: unknown
  knowledge:
    state: unknown
    evidence: []
    gap: gap.slack.inbound
question_bindings: []
form_bindings: []
interaction_fixtures: []
changes:
- id: chg.slack.pilot
  kind: added
  facts:
  - c.slack.text.recommended
  - c.slack.text.truncation
  - tb.slack.text.fallback
  summary: Pilot records only.
gaps:
- id: gap.slack.versions
  status: open
  question: Is the Web API versioned and how are webhook changes versioned?
  facts:
  - ver.slack.web_api
  - ver.slack.incoming_webhook
  next_investigation: Read the Slack API changelog.
  kind: research
- id: gap.slack.blocks
  status: open
  question: What are per-block-type and aggregate block limits?
  facts:
  - c.slack.blocks.section_text
  next_investigation: Read the Block Kit reference.
  kind: research
- id: gap.slack.mrkdwn
  status: open
  question: Which constructs does mrkdwn support?
  facts:
  - fmt.slack.mrkdwn
  next_investigation: Read the text formatting reference.
  kind: research
- id: gap.slack.images
  status: open
  question: Which image slots exist in blocks and legacy attachments?
  facts:
  - img.slack.section_accessory
  next_investigation: Read the image block and image element references.
  kind: research
- id: gap.slack.errors
  status: open
  question: What are the Web API error envelope and webhook plain-text tokens?
  facts:
  - env.slack.web_api.json
  - env.slack.webhook.text
  - err.slack.web_api.msg_too_long
  - err.slack.webhook.plain_text
  next_investigation: Read chat.postMessage errors and the incoming webhook error table.
  kind: research
- id: gap.slack.inbound
  status: open
  question: Which message events reach the app and with what content?
  facts:
  - ib.slack.events.message
  next_investigation: Read the Events API message event reference.
  kind: research
- id: gap.slack.pilot_scope
  kind: research
  status: open
  question: Categories outside the schema pilot's scope have not been researched for this platform.
  facts: []
  next_investigation: Research in the Phase 7 baseline; this fixture only exercises the contract.
requires_messenger_update: false
---

# Slack (schema pilot, v1 port)

Pilot record only; not research. It separates a 4,000-character
recommendation from the 40,000-character truncation threshold, and binds the
same native `text` field twice: as primary content without blocks and as a
notification fallback for structured blocks. Error records stay `unknown` with
non-executable `candidate_match` signatures until evidence exists.
