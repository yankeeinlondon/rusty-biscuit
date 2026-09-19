---
# Schema-pilot signal record ported to schema version 1 (Phase 2 freeze gate).
# Not research: values come from the Phase 1 pilot. Categories the pilot did
# not exercise point at `gap.signal.pilot_scope`.
$schema: ../../../../../docs/research/platforms/_schema.yaml
schema_version: 1
platform_id: signal
created: 2026-09-17
last_updated: 2026-09-17
agent: claude-code
model: claude-opus-5
sources:
- id: src.signal.prose
  kind: secondary
  location: messenger/docs/research/platforms/signal.md
  locator: Capabilities; Gotchas table
  retrieved: 2026-03-09
- id: src.signal.adapter
  kind: source_code
  location: messenger/lib/src/provider/signal.rs
  locator: send_prepared JSON-RPC params and JsonRpcError
  revision: d57faf7e8
  retrieved: 2026-09-17
  note: Establishes what Messenger sends and parses, not bridge behavior.
- id: src.signal.platform_guide
  kind: secondary
  location: messenger/docs/platforms/signal.md
  locator: Setup step 3
  retrieved: 2026-09-17
- id: src.signal_cli.jsonrpc_man
  kind: official_docs
  url: https://github.com/AsamK/signal-cli/blob/master/man/signal-cli-jsonrpc.5.adoc
  note: Cited by the prose; not retrieved in this pilot, so it cannot back a known fact.
interfaces:
- interface_id: signal_cli_jsonrpc
  role: sending_adapter
  api_identity: signal-cli JSON-RPC 2.0 over HTTP
  endpoint_template: POST {rpc_url} (method send | sendGroupMessage)
  bridge: signal-cli
  classification: community
  direction: bidirectional
  adapters:
  - signal
  applies_when:
  - kind: hosting_mode
    equals: daemon_http
  - kind: bridge_version
    gap: gap.signal.versions
  operations:
  - send
  - send_group_message
coverage:
- interface: signal_cli_jsonrpc
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
      status: gap
      gap: gap.signal.pilot_scope
    attachments:
      status: researched
    addressing:
      status: gap
      gap: gap.signal.pilot_scope
    receipts:
      status: gap
      gap: gap.signal.pilot_scope
    attribution:
      status: gap
      gap: gap.signal.pilot_scope
    location:
      status: gap
      gap: gap.signal.pilot_scope
    expression:
      status: gap
      gap: gap.signal.pilot_scope
    interactivity:
      status: researched
    delivery_controls:
      status: gap
      gap: gap.signal.pilot_scope
    eligibility:
      status: gap
      gap: gap.signal.pilot_scope
    rate_limits:
      status: gap
      gap: gap.signal.pilot_scope
    errors:
      status: researched
api_versions:
- id: ver.signal.service
  interface: signal_cli_jsonrpc
  versioning: unresearched
  knowledge:
    state: unknown
    evidence:
    - src.signal.prose
    explanation: The underlying Signal service API is not a public versioned developer API per the prose; not established.
    gap: gap.signal.versions
  subject: provider_api
chronology:
- id: chr.signal.signal_cli.current
  interface: signal_cli_jsonrpc
  subject: bridge
  version: unknown
  stability: unknown
  release_date_state: unknown
  knowledge:
    state: unknown
    evidence:
    - src.signal.prose
    explanation: 'Prose: releases older than about three months may break; no release identified.'
    gap: gap.signal.versions
constraints:
- id: c.signal.send.message
  interface: signal_cli_jsonrpc
  operation: send
  surface: body
  native_locator: params.message
  kind: hard_max
  unit: unknown
  measurement_stage: unknown
  overflow_behavior: unknown
  applies_when:
  - kind: bridge_version
    gap: gap.signal.versions
  knowledge:
    state: unknown
    evidence:
    - src.signal.prose
    - src.signal.adapter
    gap: gap.signal.text_limit
format_profiles:
- id: fmt.signal.plain
  family: plain_text
  malformed_behavior: literal
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
    state: known
    evidence:
    - src.signal.adapter
    confidence: medium
    explanation: The adapter sends plain text; construct behavior is unresearched.
  auto_interpretation: unknown
text_bindings:
- id: tb.signal.send.message
  interface: signal_cli_jsonrpc
  operation: send
  surface: body
  native_locator: params.message
  representation: text
  content_role: primary
  profiles:
  - fmt.signal.plain
  relationships:
  - kind: optional_companion
    target: tb.signal.send.text_style
  constraints:
  - c.signal.send.message
  knowledge:
    state: known
    evidence:
    - src.signal.adapter
    confidence: medium
  packaging: unknown
  visibility: visible_content
- id: tb.signal.send.text_style
  interface: signal_cli_jsonrpc
  operation: send
  surface: body
  native_locator: params.textStyle (unverified)
  representation: entities
  content_role: primary
  profiles:
  - fmt.signal.plain
  entity_offset_unit: utf16_code_units
  knowledge:
    state: unknown
    evidence:
    - src.signal.prose
    explanation: Prose says Signal tooling indexes styles in UTF-16; the JSON-RPC field name is unverified.
    gap: gap.signal.styles
  packaging: unknown
  visibility: visible_content
image_bindings: []
role_coverage:
- interface: signal_cli_jsonrpc
  roles:
    inline_content:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    attachment:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    primary:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    thumbnail:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    accessory:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    author_icon:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    footer_icon:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
    link_preview:
      state: unknown
      bindings: []
      gap: gap.signal.attachments
attachment_bindings:
- id: att.signal.send
  interface: signal_cli_jsonrpc
  operation: send
  media_kinds:
  - other
  upload_mechanisms: []
  applies_when:
  - kind: bridge_version
    gap: gap.signal.attachments
  knowledge:
    state: unknown
    evidence:
    - src.signal.prose
    explanation: Only REST-wrapper evidence (base64_attachments) exists; it does not apply to JSON-RPC.
    gap: gap.signal.attachments
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
- id: env.signal.jsonrpc
  interface: signal_cli_jsonrpc
  operations:
  - send
  - send_group_message
  origin: bridge
  body_format: json_rpc
  success_discriminator: /result
  code_locator: /error/code
  message_locator: /error/message
  knowledge:
    state: known
    evidence:
    - src.signal.adapter
    confidence: medium
    explanation: JSON-RPC 2.0 shape as parsed by the adapter; signal-cli code meanings are unresearched.
errors:
- id: err.signal.jsonrpc.unclassified
  interface: signal_cli_jsonrpc
  operations:
  - send
  - send_group_message
  envelope: env.signal.jsonrpc
  origin: bridge
  phase: response
  outcome: failure
  category: unknown
  delivery_certainty: unknown
  recovery: unknown
  replay_safety: unknown
  diagnostic_fields:
  - error.code
  knowledge:
    state: unknown
    evidence:
    - src.signal.platform_guide
    explanation: The guide lists unregistered account, invalid recipient, missing group and rate limiting without codes.
    gap: gap.signal.errors
  remediation: unknown
error_fixtures: []
inbound_bindings:
- id: ib.signal.receive
  interface: signal_cli_jsonrpc
  event: receive notification
  mechanism: local_event_stream
  reach:
  - direct
  - groups
  content: unknown
  prerequisites:
  - single consumer; auto-receive can steal messages from another receiver
  reuses_send_identity: 'yes'
  knowledge:
    state: unknown
    evidence:
    - src.signal.prose
    gap: gap.signal.inbound
question_bindings: []
form_bindings: []
interaction_fixtures: []
changes:
- id: chg.signal.pilot
  kind: unresolved
  facts:
  - c.signal.send.message
  - att.signal.send
  summary: Pilot records only; key facts remain unresolved.
gaps:
- id: gap.signal.text_limit
  status: investigated
  question: Does signal-cli JSON-RPC send, or the Signal service, bound or transform long message text?
  facts:
  - c.signal.send.message
  searches:
  - grep -n -i -E 'length|long|2000' messenger/docs/research/platforms/signal.md
  - grep -n message messenger/lib/src/provider/signal.rs
  inspected_sources:
  - src.signal.prose
  - src.signal.adapter
  unresolved_reason: Neither source states a bound or long-text transformation; the man page was not retrieved.
  blocked_decision: Whether Signal can join the truncation handoff with an executable bound.
  next_investigation: Read signal-cli send implementation and the JSON-RPC man page at a pinned release.
  kind: research
- id: gap.signal.attachments
  status: investigated
  question: How does JSON-RPC send accept attachments and with what limits?
  facts:
  - att.signal.send
  searches:
  - grep -n -i attachment messenger/docs/research/platforms/signal.md
  - grep -n -i attachment messenger/lib/src/provider/signal.rs
  inspected_sources:
  - src.signal.prose
  - src.signal.adapter
  unresolved_reason: The prose documents the REST wrapper field base64_attachments; the adapter declares no attachment kinds.
  blocked_decision: Attachment preflight and image-role coverage for Signal.
  next_investigation: Read the JSON-RPC man page send parameters at a pinned signal-cli release.
  kind: research
- id: gap.signal.versions
  status: open
  question: Which signal-cli release introduced daemon --http and what is current?
  facts:
  - chr.signal.signal_cli.current
  - ver.signal.service
  next_investigation: Read the signal-cli CHANGELOG.
  kind: research
- id: gap.signal.styles
  status: open
  question: What JSON-RPC field carries text styles and in which units?
  facts:
  - tb.signal.send.text_style
  next_investigation: Read the JSON-RPC man page.
  kind: research
- id: gap.signal.errors
  status: open
  question: Which JSON-RPC error codes does signal-cli return for send?
  facts:
  - err.signal.jsonrpc.unclassified
  next_investigation: Read signal-cli JSON-RPC error handling source.
  kind: research
- id: gap.signal.inbound
  status: open
  question: What does a receive notification contain over HTTP daemon mode?
  facts:
  - ib.signal.receive
  next_investigation: Read the JSON-RPC man page receive section.
  kind: research
- id: gap.signal.pilot_scope
  kind: research
  status: open
  question: Categories outside the schema pilot's scope have not been researched for this platform.
  facts: []
  next_investigation: Research in the Phase 7 baseline; this fixture only exercises the contract.
requires_messenger_update: false
---

# Signal (schema pilot, v1 port)

Pilot record only; not research. Nearly every fact is `unknown`: the pilot
checks that the contract can say so precisely. The interface is signal-cli
JSON-RPC, not the REST wrapper the older prose also describes. Bridge-version
conditions carry no operand, only the gap that investigates them
(`{ kind: bridge_version, gap: gap.signal.versions }`), which makes their
records ineligible for enforcement (SR-CONDITION).
