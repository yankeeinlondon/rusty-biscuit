---
# Schema-pilot discord record ported to schema version 1 (Phase 2 freeze gate).
# Not research: values come from the Phase 1 pilot. Categories the pilot did
# not exercise point at `gap.discord.pilot_scope`.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: discord
created: 2026-09-17
last_updated: 2026-09-17
agent: claude-code
model: claude-opus-5
sources:
- id: src.discord.embed_limits
  kind: official_docs
  url: https://docs.discord.com/developers/resources/message#embed-limits
  locator: Embed Limits
  retrieved: 2026-09-17
  note: spec spot-check
- id: src.discord.prose
  kind: secondary
  location: messenger/docs/research/platforms/discord.md
  locator: API Overview; Interactions; Gotcha 6
  retrieved: 2026-03-09
- id: src.twilight_validate
  kind: sdk_validator
  location: twilight-validate-0.17.0/src/message.rs
  locator: MESSAGE_CONTENT_LENGTH_MAX; EMBED_TOTAL_LENGTH
  revision: 0.17.0
  retrieved: 2026-09-06
  note: as analyzed in messenger/fixes/2026-09-06-truncation/spec.md
- id: src.discord.webhook_400
  kind: observed_fixture
  location: messenger/fixes/2026-09-06-truncation/spec.md
  locator: R1 DISCORD_MAX_CONTENT_LENGTH doc comment
  retrieved: 2026-09-06
interfaces:
- interface_id: discord_bot_api
  role: sending_adapter
  api_identity: Discord HTTP API
  endpoint_template: POST https://discord.com/api/v10/channels/{channel_id}/messages
  api_version: v10
  sdk: twilight-http
  sdk_version: '0.17'
  classification: official
  direction: send_only
  adapters:
  - discord
  relationships:
  - kind: requires_companion_interface
    target: discord_gateway
    prerequisites:
    - bot token
    - gateway intents
  operations:
  - create_message
- interface_id: discord_webhook
  role: sending_adapter
  api_identity: Discord HTTP API (webhook execute)
  endpoint_template: POST https://discord.com/api/v10/webhooks/{webhook_id}/{webhook_token}?wait=true
  api_version: v10
  classification: official
  direction: send_only
  adapters:
  - discord-webhook
  operations:
  - execute_webhook
- interface_id: discord_gateway
  role: research_only
  api_identity: Discord Gateway (WebSocket)
  classification: official
  direction: receive_only
  adapters: []
  relationships:
  - kind: receives_events_for
    target: discord_bot_api
    prerequisites:
    - GUILD_MESSAGES intent
    - MESSAGE_CONTENT privileged intent
  operations: []
- interface_id: discord_interactions
  role: research_only
  api_identity: Discord Interactions (callback endpoint)
  endpoint_template: POST https://discord.com/api/v10/interactions/{interaction_id}/{interaction_token}/callback
  classification: official
  direction: callback_only
  adapters: []
  relationships:
  - kind: responds_to_interactions_from
    target: discord_bot_api
    prerequisites:
    - application id
    - interaction token
  operations: []
coverage:
- interface: discord_bot_api
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
      gap: gap.discord.pilot_scope
    addressing:
      status: gap
      gap: gap.discord.pilot_scope
    receipts:
      status: gap
      gap: gap.discord.pilot_scope
    attribution:
      status: gap
      gap: gap.discord.pilot_scope
    location:
      status: gap
      gap: gap.discord.pilot_scope
    expression:
      status: gap
      gap: gap.discord.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.discord.pilot_scope
    eligibility:
      status: gap
      gap: gap.discord.pilot_scope
    rate_limits:
      status: gap
      gap: gap.discord.pilot_scope
    errors:
      status: researched
- interface: discord_webhook
  categories:
    versions:
      status: gap
      gap: gap.discord.pilot_scope
    constraints:
      status: researched
    formatting:
      status: gap
      gap: gap.discord.pilot_scope
    text_bindings:
      status: gap
      gap: gap.discord.pilot_scope
    images:
      status: gap
      gap: gap.discord.pilot_scope
    attachments:
      status: gap
      gap: gap.discord.pilot_scope
    addressing:
      status: gap
      gap: gap.discord.pilot_scope
    receipts:
      status: gap
      gap: gap.discord.pilot_scope
    attribution:
      status: researched
    location:
      status: gap
      gap: gap.discord.pilot_scope
    expression:
      status: gap
      gap: gap.discord.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.discord.pilot_scope
    eligibility:
      status: gap
      gap: gap.discord.pilot_scope
    rate_limits:
      status: gap
      gap: gap.discord.pilot_scope
    errors:
      status: researched
- interface: discord_gateway
  categories:
    versions:
      status: gap
      gap: gap.discord.pilot_scope
    constraints:
      status: gap
      gap: gap.discord.pilot_scope
    formatting:
      status: gap
      gap: gap.discord.pilot_scope
    text_bindings:
      status: gap
      gap: gap.discord.pilot_scope
    images:
      status: gap
      gap: gap.discord.pilot_scope
    attachments:
      status: gap
      gap: gap.discord.pilot_scope
    addressing:
      status: gap
      gap: gap.discord.pilot_scope
    receipts:
      status: gap
      gap: gap.discord.pilot_scope
    attribution:
      status: gap
      gap: gap.discord.pilot_scope
    location:
      status: gap
      gap: gap.discord.pilot_scope
    expression:
      status: gap
      gap: gap.discord.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.discord.pilot_scope
    eligibility:
      status: gap
      gap: gap.discord.pilot_scope
    rate_limits:
      status: gap
      gap: gap.discord.pilot_scope
    errors:
      status: gap
      gap: gap.discord.pilot_scope
- interface: discord_interactions
  categories:
    versions:
      status: gap
      gap: gap.discord.pilot_scope
    constraints:
      status: gap
      gap: gap.discord.pilot_scope
    formatting:
      status: gap
      gap: gap.discord.pilot_scope
    text_bindings:
      status: gap
      gap: gap.discord.pilot_scope
    images:
      status: gap
      gap: gap.discord.pilot_scope
    attachments:
      status: gap
      gap: gap.discord.pilot_scope
    addressing:
      status: gap
      gap: gap.discord.pilot_scope
    receipts:
      status: gap
      gap: gap.discord.pilot_scope
    attribution:
      status: gap
      gap: gap.discord.pilot_scope
    location:
      status: gap
      gap: gap.discord.pilot_scope
    expression:
      status: gap
      gap: gap.discord.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.discord.pilot_scope
    eligibility:
      status: gap
      gap: gap.discord.pilot_scope
    rate_limits:
      status: gap
      gap: gap.discord.pilot_scope
    errors:
      status: gap
      gap: gap.discord.pilot_scope
api_versions:
- id: ver.discord.bot
  interface: discord_bot_api
  versioning: versioned
  latest_stable: v10
  previews: []
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: Prose says v10 is current; preview versions not researched.
    gap: gap.discord.versions
  subject: provider_api
chronology:
- id: chr.discord.api.v10
  interface: discord_bot_api
  subject: provider_api
  version: v10
  stability: stable
  release_date_state: unknown
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    gap: gap.discord.versions
- id: chr.discord.twilight.0_17
  interface: discord_bot_api
  subject: sdk
  version: '0.17'
  stability: stable
  release_date_state: unknown
  knowledge:
    state: known
    evidence:
    - src.twilight_validate
    confidence: medium
constraints:
- id: c.discord.bot.content.service_max
  interface: discord_bot_api
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
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: Docs say characters; counting unit and bot-path service overflow unresolved (the observed 400 is webhook evidence).
- id: c.discord.bot.content.sdk_max
  interface: discord_bot_api
  operation: create_message
  surface: body
  native_locator: content
  kind: hard_max
  value: 2000
  unit: unicode_scalars
  measurement_stage: field_value
  enforced_by: sdk
  overflow_behavior: reject
  overflow_errors:
  - err.discord.bot.sdk_validation
  applies_when:
  - kind: sdk_version
    min: '0.17'
    max: '0.17'
  knowledge:
    state: known
    evidence:
    - src.twilight_validate
    confidence: high
    explanation: value.chars().count() <= 2000 before any request.
- id: c.discord.bot.embed_description
  interface: discord_bot_api
  operation: create_message
  surface: rich_description
  native_locator: embeds[].description
  kind: hard_max
  value: 4096
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: unknown
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.discord.embed_limits
    confidence: medium
- id: c.discord.bot.embed_description.sdk_overflow
  interface: discord_bot_api
  operation: create_message
  surface: rich_description
  native_locator: embeds[].description
  kind: hard_max
  unit: unicode_scalars
  measurement_stage: field_value
  enforced_by: sdk
  overflow_behavior: unknown
  applies_when:
  - kind: sdk_version
    min: '0.17'
    max: '0.17'
  knowledge:
    state: conflicting
    evidence:
    - src.discord.prose
    - src.twilight_validate
    explanation: Sources disagree on what happens when an embed exceeds its limits on the bot path.
    gap: gap.discord.embed_overflow
    claims:
    - statement: Exceeding embed limits silently fails.
      evidence:
      - src.discord.prose
    - statement: twilight req.embeds() rejects with ErrorType::Validation before any request.
      value: 4096
      evidence:
      - src.twilight_validate
- id: c.discord.bot.embeds_total
  interface: discord_bot_api
  operation: create_message
  surface: rich_objects
  native_locator: embeds[*]
  kind: aggregate_max
  value: 6000
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: unknown
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
    - src.discord.embed_limits
    confidence: medium
    explanation: Combined budget across all embeds on one message.
- id: c.discord.bot.embed_count
  interface: discord_bot_api
  operation: create_message
  surface: rich_objects
  native_locator: embeds
  kind: item_count_max
  value: 10
  unit: items
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: unknown
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: 'Secondary-only: visible but not executable.'
- id: c.discord.webhook.content.service_max
  interface: discord_webhook
  operation: execute_webhook
  surface: body
  native_locator: content
  kind: hard_max
  value: 2000
  unit: unspecified_characters
  measurement_stage: field_value
  enforced_by: service
  overflow_behavior: reject
  overflow_errors:
  - err.discord.webhook.content_field
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.discord.webhook_400
    confidence: medium
format_profiles:
- id: fmt.discord.markdown
  family: markdown_subset
  dialect: Discord message markdown
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
    gap: gap.discord.markdown
  auto_interpretation: unknown
text_bindings:
- id: tb.discord.bot.content
  interface: discord_bot_api
  operation: create_message
  surface: body
  native_locator: content
  representation: markup
  content_role: primary
  profiles:
  - fmt.discord.markdown
  constraints:
  - c.discord.bot.content.service_max
  - c.discord.bot.content.sdk_max
  relationships:
  - kind: optional_companion
    target: tb.discord.bot.embed_description
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
  packaging: unknown
  visibility: visible_content
- id: tb.discord.bot.embed_description
  interface: discord_bot_api
  operation: create_message
  surface: rich_description
  native_locator: embeds[].description
  representation: markup
  content_role: primary
  profiles:
  - fmt.discord.markdown
  constraints:
  - c.discord.bot.embed_description
  - c.discord.bot.embeds_total
  knowledge:
    state: known
    evidence:
    - src.discord.embed_limits
    confidence: medium
  packaging: unknown
  visibility: visible_content
image_bindings:
- id: img.discord.bot.file_upload
  interface: discord_bot_api
  operation: create_message
  role: attachment
  fidelity: exact
  native_locator: files[n] (multipart)
  placement: standalone_media
  placement_control: provider_selected
  submissions:
  - multipart_part
  sources:
  - upload_bytes
  collection: provider_selected
  supplied_by: caller_supplied
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: 'alt_text_binding is unresolved: no text binding yet for attachments[].description.'
    gap: gap.discord.images
  multiple_images: unknown
  ordering: unknown
  atomic: unknown
  receipts: unknown
- id: img.discord.bot.embed_image
  interface: discord_bot_api
  operation: create_message
  container_surface: rich_objects
  role: primary
  native_locator: embeds[].image.url
  placement: card_main
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
    gap: gap.discord.images
  fidelity: exact
  multiple_images: unknown
  ordering: unknown
  atomic: unknown
  receipts: unknown
role_coverage:
- interface: discord_bot_api
  roles:
    inline_content:
      state: unknown
      bindings: []
      gap: gap.discord.images
    attachment:
      state: known
      support: supported
      bindings:
      - img.discord.bot.file_upload
    primary:
      state: unknown
      bindings:
      - img.discord.bot.embed_image
      gap: gap.discord.images
    thumbnail:
      state: unknown
      bindings: []
      gap: gap.discord.images
    accessory:
      state: unknown
      bindings: []
      gap: gap.discord.images
    author_icon:
      state: unknown
      bindings: []
      gap: gap.discord.images
    footer_icon:
      state: unknown
      bindings: []
      gap: gap.discord.images
    link_preview:
      state: unknown
      bindings: []
      gap: gap.discord.images
attachment_bindings: []
addressing: []
receipts: []
attribution_bindings:
- id: attr.discord.webhook.username
  interface: discord_webhook
  operation: execute_webhook
  role: sender_display_name
  native_fields:
  - username
  - avatar_url
  control: caller_supplied
  scope: message
  inclusion: optional
  override_behavior: honored
  changes_sender: 'no'
  applies_when: []
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: username and avatar_url in the execute body.
location_bindings: []
author_geolocation: []
expression_bindings: []
delivery_controls: []
eligibility: []
rate_limits: []
envelopes:
- id: env.discord.webhook.json
  interface: discord_webhook
  operations:
  - execute_webhook
  origin: service
  body_format: json
  success_discriminator: http_status
  field_errors_locator: /
  knowledge:
    state: known
    evidence:
    - src.discord.webhook_400
    confidence: low
    explanation: One observed body keyed by field at the root; presence of /code and /message on webhook errors is unknown.
    gap: gap.discord.errors
- id: env.discord.bot.twilight
  interface: discord_bot_api
  operations:
  - create_message
  origin: sdk
  body_format: sdk_error
  code_locator: sdk_variant:twilight_http::error::ErrorType
  knowledge:
    state: known
    evidence:
    - src.twilight_validate
    confidence: medium
errors:
- id: err.discord.webhook.content_field
  interface: discord_webhook
  operations:
  - execute_webhook
  envelope: env.discord.webhook.json
  origin: service
  phase: response
  outcome: failure
  category: invalid_content
  match:
    http_status: 400
    discriminator_locator: /content
  related_facts:
  - c.discord.webhook.content.service_max
  affected_fields:
  - content
  delivery_certainty: rejected
  recovery: after_correction
  replay_safety: unknown
  remediation_note: A /content field error is not necessarily a length error; the message text is explanatory only.
  diagnostic_fields:
  - http_status
  - field names
  fixtures:
  - fx.discord.webhook.content_400
  - fx.discord.webhook.embeds_400
  knowledge:
    state: known
    evidence:
    - src.discord.webhook_400
    confidence: low
  remediation: unknown
- id: err.discord.bot.sdk_validation
  interface: discord_bot_api
  operations:
  - create_message
  envelope: env.discord.bot.twilight
  origin: sdk
  phase: before_submission
  outcome: failure
  category: invalid_content
  match:
    sdk_error_variant: twilight_http::error::ErrorType::Validation
  related_facts:
  - c.discord.bot.content.sdk_max
  - c.discord.bot.embeds_total
  delivery_certainty: not_submitted
  recovery: after_correction
  replay_safety: safe
  remediation: shorten_content
  remediation_note: Validation also covers non-length rules; shorten only when a related length fact is exceeded.
  applies_when:
  - kind: sdk_version
    min: '0.17'
    max: '0.17'
  knowledge:
    state: known
    evidence:
    - src.twilight_validate
    confidence: medium
error_fixtures:
- id: fx.discord.webhook.content_400
  envelope: env.discord.webhook.json
  provenance: observed_in_repo
  expect: match
  expected_error: err.discord.webhook.content_field
  http_status: 400
  body: '{"content": ["Must be 2000 or fewer in length."]}'
  operation: execute_webhook
- id: fx.discord.webhook.embeds_400
  envelope: env.discord.webhook.json
  provenance: constructed_from_docs
  expect: no_match
  http_status: 400
  body: '{"embeds": ["0"]}'
  note: near miss on a different field
  operation: execute_webhook
inbound_bindings:
- id: ib.discord.webhook.none
  interface: discord_webhook
  event: none
  mechanism: none
  reach:
  - none
  content: none
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: Webhook execution is send-only.
  reuses_send_identity: unknown
- id: ib.discord.gateway.message_create
  interface: discord_gateway
  event: MESSAGE_CREATE
  mechanism: persistent_connection
  reach:
  - direct
  - groups
  - mentions
  content: full_text
  prerequisites:
  - MESSAGE_CONTENT privileged intent for guild text
  reuses_send_identity: 'yes'
  knowledge:
    state: known
    evidence:
    - src.discord.prose
    confidence: low
    explanation: Without MESSAGE_CONTENT guild text is withheld; no typed condition kind exists for intents yet.
    gap: gap.discord.inbound
question_bindings:
- id: qb.discord.bot.confirm_buttons
  interface: discord_bot_api
  kind: confirmation
  mechanism: buttons
  operation: create_message
  native_locator: components[].components[] (button)
  companion_interface: discord_interactions
  answer_event: INTERACTION_CREATE (message component)
  answer_locator: /data/custom_id
  answer_type: boolean
  confirmation_mapping:
    affirmative: custom_id confirm.yes
    negative: custom_id confirm.no
  correlation_locator: /message/id
  validation: application
  visibility: conditional
  fidelity: exact
  knowledge:
    state: unknown
    evidence:
    - src.discord.prose
    confidence: low
    gap: gap.discord.interactivity
  native: native
  cancellation: unknown
  responses: attributable
form_bindings:
- id: fb.discord.interactions.modal
  interface: discord_interactions
  container: modal
  opened_by: interaction callback type 9 (MODAL)
  question_kinds:
  - text_input
  submission: single_event
  companion_interface: discord_interactions
  knowledge:
    state: unknown
    evidence:
    - src.discord.prose
    confidence: low
    gap: gap.discord.interactivity
  lifecycle:
    requires_user_action: 'yes'
  absent_vs_empty: unknown
  cancellation: unknown
  validation: unknown
  navigation: unknown
interaction_fixtures: []
changes:
- id: chg.discord.pilot
  kind: added
  facts:
  - c.discord.bot.embeds_total
  - err.discord.bot.sdk_validation
  summary: Pilot records only.
gaps:
- id: gap.discord.embed_budget_preflight
  kind: requires_messenger_update
  status: open
  question: Should the Discord adapters preflight the aggregate embed budget before sending?
  interface: discord_bot_api
  category: constraints
  facts:
  - c.discord.bot.embeds_total
  next_investigation: Review the constraints assessment for both Discord adapters.
- id: gap.discord.versions
  status: open
  question: Which preview API versions exist and when was v10 released?
  facts:
  - ver.discord.bot
  - chr.discord.api.v10
  next_investigation: Read the official API versioning page and changelog.
  kind: research
- id: gap.discord.markdown
  status: open
  question: Which markdown constructs does message content support?
  facts:
  - fmt.discord.markdown
  next_investigation: Read the official message formatting reference.
  kind: research
- id: gap.discord.images
  status: open
  question: Which embed image slots exist and how are attachment descriptions bound?
  facts:
  - img.discord.bot.embed_image
  next_investigation: Read the embed object and attachment object references.
  kind: research
- id: gap.discord.errors
  status: open
  question: Do webhook error bodies carry /code and /message like bot responses?
  facts:
  - env.discord.webhook.json
  next_investigation: Read the opcodes and status codes reference.
  kind: research
- id: gap.discord.embed_overflow
  status: investigated
  question: Does an over-limit embed on the bot path fail silently or loudly?
  facts:
  - c.discord.bot.embed_description.sdk_overflow
  searches:
  - grep -n -i 'silent' messenger/docs/research/platforms/discord.md
  - grep -n 'embeds()' messenger/fixes/2026-09-06-truncation/spec.md
  inspected_sources:
  - src.discord.prose
  - src.twilight_validate
  unresolved_reason: The secondary prose claim is not dated against twilight 0.17 and gives no mechanism.
  blocked_decision: Whether the truncation handoff may treat the bot path as fail-closed for embeds.
  next_investigation: Add a local fixture calling twilight-validate embed checks with a 4097-scalar description.
  kind: research
- id: gap.discord.inbound
  status: open
  question: How should intent prerequisites be typed?
  facts:
  - ib.discord.gateway.message_create
  next_investigation: Propose a permission_grant condition kind.
  kind: research
- id: gap.discord.interactivity
  status: open
  question: What are component and modal limits and lifecycles?
  facts:
  - qb.discord.bot.confirm_buttons
  - fb.discord.interactions.modal
  next_investigation: Read the message components and interactions references.
  kind: research
- id: gap.discord.pilot_scope
  kind: research
  status: open
  question: Categories outside the schema pilot's scope have not been researched for this platform.
  facts: []
  next_investigation: Research in the Phase 7 baseline; this fixture only exercises the contract.
requires_messenger_update: true
reason: 'Pilot illustration: neither Discord adapter preflights the aggregate embed budget (c.discord.bot.embeds_total); see mappings.pilot.yaml.'
---

# Discord (schema pilot, v1 port)

Pilot record only; not research. Values come from the existing prose research,
the truncation fix's SDK analysis, and the spec's 2026-09-17 embed-limit
spot-check. Unverified claims keep `confidence: low` or `state: unknown` with a
gap.

The pilot exercises an individual bound (`content`), an SDK bound that differs
from the service bound in unit, a 6,000-character aggregate with members across
repeated embeds, a send-only webhook, and a bot whose receiving and interactive
capabilities depend on research-only companion interfaces.
