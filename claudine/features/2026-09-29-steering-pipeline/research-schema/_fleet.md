---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/steering/{{state.file}}"
topic: Steering
# Research rotates over three agents by roster position. The order is chosen
# so that no agent researches its own provider.
agent: "{{ state.index % 3 == 1 ? 'opencode' : (state.index % 3 == 2 ? 'claude' : 'codex') }}"
model: "{{ state.index % 3 == 1 ? 'zai-coding-plan/glm-5.3' : (state.index % 3 == 2 ? 'sonnet' : 'gpt-6.1-sol') }}"
effort: "{{ state.index % 3 == 1 ? 'provider_default' : (state.index % 3 == 2 ? 'high' : 'medium') }}"
# A sequence plans one agent before any step exists, and Claudine cannot yet
# set reasoning effort from a prompt. So the fleet runs once per agent, each
# run naming its agent, model, and effort. A run researches only the providers
# assigned to its agent:
#
#   claudine sequence _fleet.md -y --opencode --model zai-coding-plan/glm-5.3
#   claudine sequence _fleet.md -y --claude --model sonnet -- --effort high
#   claudine sequence _fleet.md -y --codex --model gpt-6.1-sol -- -c model_reasoning_effort=medium
# `only=<slug>` on the command line researches one provider, for a pilot or a repair.
only: all
# One provider's rejected research must not stop the others.
fail_fast: false
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "state.skip_research == true"
          action:
              - stderr: "**{{state.name}}** is marked `skip_research`; skipping {{topic}}"
              - skip
        - when: "only != 'all' && only != state.slug"
          action:
              - skip
        - when: "env.AGENT != agent"
          action:
              - stderr: "**{{state.name}}** is assigned to {{agent}}, and this run launches {{env.AGENT}}; skipping"
              - skip
        - when: "env.MODEL != model"
          action:
              - warn: "**{{state.name}}** is assigned {{model}}, and this run launches {{env.MODEL}}"
              - error: "this run launches a model outside the rotation"
        # Current means the fleet confirmed the document against this revision
        # of the contract within 14 days. The `success` event writes
        # `contract_checked`; a document the fleet rejected never carries it.
        - when: "file_exists(file) && frontmatter(file, 'schema_revision') == 5 && frontmatter(file, 'contract_checked') && !date_delta(frontmatter(file, 'contract_checked'), ctx.today, '14d')"
          action:
              - stderr: "**{{state.name}}** has {{topic}} research confirmed on {{ frontmatter(file, 'contract_checked') }}; skipping"
              - skip
        - action:
              - info: "Researching **{{topic}}** for **{{state.name}}** with {{agent}} / {{model}} (effort: {{effort}})"
start:
    stack:
        - when: "!has_binary(state.binary)"
          action:
              - warn: "**{{state.binary}}** is not installed on this host, so the research cannot inspect it locally"
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - warn: "The agent finished, but {{ link(file) }} was not updated today"
              - error: "the research document was not updated"
        - when: "frontmatter(file, 'agent') != agent || frontmatter(file, 'model') != model"
          action:
              - warn: "{{ link(file) }} records a different agent or model than the one assigned ({{agent}} / {{model}})"
              - error: "the research document records the wrong agent or model"
        - action:
              - set:
                    contract: "$(md schema validate '{{file}}' --no-trigger-schemas --format json)::result"
        - when: "!contract.ok"
          action:
              - warn: "{{topic}} research for **{{state.name}}** does not satisfy its contract"
              - stderr: "{{{ contract.stdout }}}"
              - error: "the research document does not satisfy its contract"
        - action:
              - set:
                    relations: "$(claudine providers steering check '{{state.slug}}' --json)::result"
        - when: "!relations.ok"
          action:
              - warn: "{{topic}} research for **{{state.name}}** breaks a relation its contract requires"
              - stderr: "{{{ relations.stdout }}}"
              - error: "the research document breaks a relation its contract requires"
        - action:
              - set_frontmatter: ["{{file}}", "contract_checked", "{{ctx.today}}"]
              - success: "{{topic}} research for **{{state.name}}** satisfies its contract: {{ link(file) }}"
              - message: "✅ {{topic}} research for **{{state.name}}** completed and validated"
failure:
    stack:
        - action:
              - warn: "{{topic}} research for **{{state.name}}** failed: {{ err.msg }}"
              - message: "💥 {{topic}} research for **{{state.name}}** failed: {{ err.msg }}"
finalize:
    stack:
        # A usage limit is not cured by trying again. The provider stays
        # unresearched until the limit resets.
        - when: "err && err.category == 'cap'"
          action:
              - warn: "{{env.AGENT}} has reached a usage limit, so **{{state.name}}** was not researched"
        # Anything else gets one more attempt. The retried run reads what the
        # gates rejected, so the researcher sees what to correct.
        - when: "err && err.category != 'cap'"
          action: { retry: 1 }
---
<!--
DRAFT. Not live: the live prompt is docs/research/steering/_fleet.md.

Differences from the live prompt:
- lifecycle events follow the research contract standard
- researchers rotate; the prompt records the assigned agent, model, and effort
- the contract is revision 5: channels, message templates, and field paths
- a rejected document is retried once, with the findings shown to the researcher

Before this goes live:
- `claudine providers steering check` must understand revision 5 and print JSON
- the generator must read the revision 5 fields
-->
# Steering Research: {{state.name}}

You are the assigned provider researcher, already running inside the fleet.
Perform the research yourself. Do not launch another agent, `claudine sequence`,
or another research coordinator to carry out this assignment.

This fleet uses revision 5 of the steering contract. Revision 5 records what
goes on the wire as exact values: a carrier, a message template, a field path.
Claudine configures one delivery pipeline from these values, so a sentence
where a value belongs makes the provider unusable. On refresh, retain useful
evidence and investigate specific gaps rather than repeating completed work.

::block when="(contract && !contract.ok) || (relations && !relations.ok)"
## Your previous attempt was rejected

The document you wrote was rejected. Fix every problem below. Each one names
the property, what is wrong, and what the property is for.

```json
{{contract.stdout}}
{{relations.stdout}}
```

::end-block
Use the Claudine skill. Research the CLI identified by roster slug
`{{state.slug}}`, binary `{{state.binary}}`, and official site `{{state.site}}`.
Write only `{{file}}` and any explicitly assigned sanitized evidence fixtures.
Do not modify production code, this prompt, the schema, or another provider's report.

## Execution Contract

You are running as `{{agent}}` with the model `{{model}}` at effort
`{{effort}}`. Record exactly those values. When the provider exposes the
resolved model, confirm it in the body; when it does not, say that the values
are the ones the fleet assigned.

This first pass is read-only investigation plus writing research artifacts.
Do not message, interrupt, or alter any existing agent session. Do not launch
live delivery experiments in this pass. Describe a disposable-session test when
behavior cannot be established by documentation or passive inspection; mark the
claim unverified. Do not focus terminal or browser windows or print credentials.
Run non-interactively and do not request or wait for human approval.

Use official documentation, versioned source, release notes, and passive local
inspection. Use Sniff for host/process discovery; read its skill before doing so.
Use the biscuit-file skill and `FileReference` when implementing file-reference
resolution. For OpenAI product research, inspect locally available source/help
first and use official OpenAI documentation or the official Codex repository
when browsing is needed. Previous topic research is a lead, not current proof.

## Product Questions

Claudine will let users select working or idle sessions on their host under their
OS user, including both native and Claudine-launched sessions. Manual steering
may offer interruption only after explaining the effect and obtaining an explicit
interactive choice. Automatic loop warnings must never interrupt to deliver a
message. Undocumented mechanisms are allowed only with evidence and compatible
runtime checks. Sending into a new conversation is not steering the original one.
Every mechanism also requires a successful disposable-session delivery test before
Claudine enables it, including documented mechanisms. This passive research pass
does not satisfy that gate. Report candidate capabilities without implying that
they are activated; propose tests for actual delivery, conversation identity, and
interruption effects under each claimed OS/version/launch condition.

Determine independently:

1. Can sessions be discovered, identified, deduplicated, and checked for liveness?
   Distinguish provider session identity from process IDs, reused IDs, history,
   helper processes, server processes, and multiple conversations in one process.
2. Can a message reach the same working conversation without canceling its turn
   or tool? Establish the delivery boundary, including during generation and a
   long-running tool. Queuing until the next tool or turn is not immediate delivery.
3. If interruption is necessary, what exactly stops, what context is preserved,
   and how is the message submitted afterward? Is this one operation or a sequence
   with a possible failure after interruption? Describe partial-failure outcomes.
4. Can an idle-but-open session accept the message, and does that start a turn?
5. Are documented SDK/server features actually available in ordinary CLI sessions,
   or do they require a special launch mode, retained input pipe, hook, or extension?
   Specify separate setup instructions and whether setup can enable an already
   open session or only future sessions. `claudine steer` will explain prerequisites
   but will not install extensions or change provider configuration during delivery.
6. What differs on native macOS, Linux, and Windows? Treat WSL as Linux-side
   evidence and explicitly label it; it does not prove native Windows behavior.
7. What confirms acceptance, queuing, or delivery? A successful write to a socket
   alone does not confirm acceptance. Determine acknowledgment framing, errors,
   timeouts, ordering, duplication, cancellation, and message-size constraints.
8. Which read-only checks can establish compatibility before sending, especially
   for undocumented interfaces? Record limitations when version or protocol
   compatibility cannot be established without mutation.

Investigate native peer messaging, SDK/server protocols, structured input,
hooks, and extensions. Do not assume stdin remains writable or feeds the active
turn. Document terminal-keystroke injection as a distinct approach if encountered;
do not equate it with a stable messaging protocol.

For a future Claudine-managed launch profile, prefer a verified provider prompt
RPC where available. Define the verified fallback and its warning when RPC is
unavailable. The profile must preserve enabled extensions, skills, prompt
templates, context files, and explicit provider settings; transport selection
must not silently disable them. Non-interactive operation cannot depend on a
receiving human approving messages or provider UI requests.

For Claude Code, independently investigate the user's report of PID-named JSON
records in `~/.claude/sessions/`, `messagingSocketPath`, authenticated Unix sockets
under `/tmp/cc-socks/` with sibling `.key` files, and next-tool-round delivery.
Also verify non-interactive startup, registry `kind`/entrypoint semantics, and idle
notices. None of these claims is established merely by appearing in this prompt.

## Metadata Contract

Read `./_schema.yaml`, `./_types.yaml`, and `../_types.yaml` before writing.
They define every property, its allowed values, and what to record in it.
Include `$schema: ./_schema.yaml` in the report. Use `schema_revision: 5`, the
roster slug in `provider`, today's date (`{{ctx.today}}`) in `last_updated`,
and preserve `created` on refresh. Set `agent: {{agent}}`, `model: {{model}}`,
and `reasoning_effort: {{effort}}`. Do not write `contract_checked`; the fleet
writes it after your document passes validation.

When a fact cannot be established, record `unknown` and add an entry to `gaps`
or `discovery_gaps` that names the check that would settle it. An `unknown`
blocks activation and is a valid finding. A guess is not. Preserve useful prior findings on refresh, reverify claims, and describe
changes. Do not treat the current date as evidence of a current provider version.

Define stable launch-profile IDs before writing cases. Each profile states its
applicable OS values, launch modes, origins, endpoint scope, lifetime, startup
requirements, and whether it preserves extensions, skills, prompt templates,
and context. Mark ordinary provider launch coverage with `baseline: true`.
Baseline profiles collectively provide exactly the 24 ordinary combinations:
three OS values × two launch modes × two origins × working/idle.

For every profile, provide exactly one case for every member of its declared
applicable OS × launch-mode × origin product, for both working and idle states.
An impossible combination needs an explicit case and reason. Do not merge
ordinary, exposed-server, attached-client, retained-stdio, extension, or SDK
profiles. Every case names one `profile_id`. Multiple baseline profiles must
partition the 24 combinations without overlap.

Evaluate managed control interfaces for non-interactive execution explicitly;
an API used by an interactive client is not inherently interactive. Explain
excluded modes/origins with provider evidence rather than today's wrapper limits.
Separate ordinary one-shot lifetime from a retained process where capability
differs, and distinguish active-turn steering from starting an idle turn in
mechanism records, not only prose. Delivery states must not include permission
holds that occur after delivery when the agent attempts a tool.

Populate `interface_inventory` before selecting launch profiles. Each considered
interface records included, excluded, or unknown disposition with evidence and a
reason. Included entries reference profiles; every profile must be represented.
Explain omitted modes/origins/OS values explicitly; unexamined combinations remain
unknown. A complete Cartesian product of selected profiles does not establish
that all useful interfaces were considered. Compare delivery behavior and feature
preservation, not simply whether a protocol is called RPC or ACP.

`support` describes researched provider capability, not implemented Claudine
support. Use:

- `non_interrupting`: evidence establishes delivery or queuing into the same
  conversation without canceling running work, subject to listed prerequisites.
- `interruption_required`: a context-preserving interruption mechanism exists,
  but non-interrupting delivery is unavailable for this case.
- `unsupported`: evidence establishes that the case cannot be steered. Explain
  the limitation; lack of search results alone does not justify this verdict.
- `unknown`: evidence is absent, contradictory, or insufficient. Record next checks.

Use evidence IDs to attach claims to official URLs, commit-pinned source locations,
or sanitized local observations. Give the examined version or explicitly say
`unknown`; never invent version bounds. Keep tested versions separate from claimed
compatibility ranges. Inference alone cannot establish verified runtime support.
Empty mechanism or discovery lists are valid for unknown/unsupported cases, with
an explanation. Empty evidence lists must be identified as gaps, not verification.
Set `verification: []` when no disposable-session test record exists. Each actual
test record identifies its mechanism, exact provider version, OS, launch mode,
origin, session state, outcome, sanitized fixture, assertions, and limitations.
Give each record a stable `id` and type every prose assertion in
`assertion_kinds`. Use `expected_loss` whenever the record documents a loss or
failure boundary; such a record is regression evidence and can never activate
delivery. Typed assertions are evidence, not activation grants: activation is
reviewed separately in `docs/providers/steering-activation.yaml`.
An empty list leaves activation blocked; it is valid research. Do not fabricate
test records or use source inspection as a substitute for a live test.
When refreshing an existing report, preserve prior disposable-test records and
their evidence, exact scope, and limitations. If newer evidence conflicts, record
the conflict for coordinator review; do not erase a test or broaden its scope.

For target safety, distinguish a state query followed by a send from a provider
operation that atomically checks the intended session. Investigate session changes
during submission, who can initiate them (including extensions), and what happens
to acknowledged pending messages. Do not infer race safety from a successful
identity query or a sequential happy-path test.

For failure handling, distinguish stdin EOF, full transport loss, provider kill,
controller failure, and host failure. Track pending queues, saved conversation,
and model consumption separately. For cancellation, distinguish cooperative
in-process tools from built-in shell tools and their subprocess trees. State
whether cleanup is performed by the provider, its wrapper, or a fixture watchdog;
an idle response or canceled tool result alone cannot establish process cleanup.

Populate `access_findings` for every mechanism/profile/OS tuple referenced by a case.
Distinguish an external sender being able to use a mechanism (`available`) from
requiring deliberate setup (`setup_required`), lacking a necessary credential or
interface (`blocked`), or unresolved evidence (`unknown`). These are researched
access facts, not activation: live verification and an implemented adapter remain
separate requirements. Explain whether prerequisites can help existing sessions.

Populate `delivery_states` for each mechanism. Record the full known state
vocabulary, including held-for-approval versus queued-for-delivery, and whether
an external sender can observe and correlate those states. Do not assume states
visible to a provider's built-in agent tool are exposed to
an independent client using the underlying protocol.

Populate one `receipt_guarantees` record per mechanism with only what a
successful initial acknowledgment proves. Keep request acceptance, message
persistence, delivery scheduling, and confirmed conversation delivery
independent; use `unknown` rather than deriving one from another. List later
provider signals without upgrading the initial receipt. Record its correlation.

Populate one `receipt_observations` record per mechanism. `early` means a separate
admission response is available without waiting for turn completion; `terminal`
means the first established successful response arrives after execution;
`multi_phase` means cancellation and replacement have independent receipts;
`none` requires evidence that no acknowledgment exists; otherwise use `unknown`.
Name the exact confirming signal and attach evidence. Receipt guarantees refer
to that signal, not to a successful transport write. For multi-phase operations,
state which phase the guarantee describes; cancellation acknowledgment never
proves replacement admission. `not_persisted` requires evidence of volatile-only
storage, not merely missing evidence of durability. A terminal response cannot
justify returning an accepted result immediately after submission.

### Channels

Record one `channels` entry for each distinct way a sender connects to a
session. Several mechanisms usually share one channel, so the carrier, the
credential, the setup exchange, and the way state is read are recorded once.

- `carrier`, `envelope`, and `events` say what carries the bytes, how messages
  are wrapped, and where events arrive.
- `endpoint` and `credential` say where the sender learns the address and what
  it must present. Record where a secret is found, never the secret.
- `correlation` says how a response is matched to its request, and what marks
  a response as success.
- `setup` lists, in order, the messages exchanged before the channel can carry
  a request, and what each answer tells the sender.
- `state` says how the sender learns whether the session is working or idle,
  and which turn is running.
- `provider_requests` lists the requests the provider may send to its client,
  with the answer to give when no person is present.

### Message templates

A message template is the complete message as a JSON value, copied from the
provider's documentation or source. Put a placeholder where a value varies.
These are the only placeholders:

| Placeholder | Stands for |
| --- | --- |
| `<message>` | the text being delivered |
| `<conversation>` | the provider's identifier for the conversation |
| `<operation>` | the provider's identifier for the running turn |
| `<request_id>` | a fresh identifier for this request |
| `<message_id>` | the sender's identifier for this message |
| `<directory>` | the session's working directory |
| `<provider_request_id>` | the identifier of a request the provider sent |
| `<provider_request_method>` | the method of a request the provider sent |
| `<claudine_version>` | the version of the sending Claudine |

A field path is a JSON Pointer from the root of the message being read, such
as `/params/turn/id`. When the exact shape of a message is not established,
omit the template, record `unknown` in the rule that depends on it, and add a
gap. Never write a description of a message where the message belongs.

### Mechanisms

Each mechanism performs exactly one operation over one channel. Distinguish
exact-active-turn steering, idle-turn start, interrupt-then-submit, and
follow-up queueing.

- `send` is the message that delivers the text.
- `target_guard` says whether the provider itself refuses a message aimed at a
  turn or conversation that is no longer current. A guard is a field of the
  request, such as an expected turn identifier, so it appears in `send`.
- `acceptance` says what the provider's answer proves, and no more.
- `cancel` and `stop_proof` apply to interrupt-then-submit. An answer to the
  cancel request never proves the turn stopped; record what does.

### Discovery

For each discovery method, record where the session records are, how each is
encoded, and the field path of each fact a record carries. Omit a fact the
record does not carry. `working_when` and `idle_when` say what in a record
shows the session's state.

Describe long-tool and complete-tool-batch behavior, queue ordering/drain and
persistence, and whether text is literal or may invoke skills, templates,
extension commands, or input transformations. Record sender message IDs,
correlation, duplicate suppression, and conservative retry policy; an ambiguous
response with unknown idempotency is never safe to retry. For interruption,
list ordered phases and the partial outcome when interruption succeeds but
submission fails, including queue retention or clearing.

Assess automatic rescue separately from manual messaging. Interruption and
next-turn-only delivery cannot rescue an endlessly running current turn.
Tool-boundary delivery may help repeated tool rounds but cannot help generation
or a tool that never reaches that boundary. Derive these restrictions from
operation intent, conversation effect, and delivery boundary; do not introduce
an independent support boolean or infer useful timing from request acceptance.
Unknown delivery boundaries remain unknown even when acceptance is confirmed.

For input processed by extensions, skills, or templates, distinguish receipt,
input handling, model scheduling, and model-visible delivery. Require versioned
evidence for both lifecycle event definitions and forwarding through the chosen
interface before treating an event as full settlement. Reference the execution
topic for its interface selection and unattended request contract; do not assume
that low-level turn completion includes queued work or extension settlement.

## Required Prose

Write an overview followed by sections for session discovery, non-interrupting
delivery, interruption fallback, idle sessions, protocol details, OS/version
compatibility, disposable-test proposals, Claudine integration, gaps, and sources.
Include a changelog on refresh. Distinguish observed, documented, source-derived,
and inferred behavior throughout. Explicitly discuss whether the mechanism could
help a token-generation loop that never reaches another tool call.

## Completion and Review Gates

The fleet runs `md schema validate '{{file}}' --no-trigger-schemas` and
`claudine providers steering check '{{state.slug}}'` when you finish, and
rejects a document that fails either. Run both yourself before you finish. A missing command is a failed gate, not permission to
skip it. Check the 24 unique
baseline combinations and each profile's declared Cartesian product × both
states; unique IDs; valid profile/evidence/discovery/mechanism references;
case-matching OS/origin/profile discovery records; and
mechanism/profile-matching compatibility records. Every supported
case must have concrete evidence, an identified delivery mechanism, and explicit
discovery coverage or a documented discovery gap. The generator must distinguish
delivery capability from whether Claudine can actually find the session.

Every supported case without discovery must have a matching `discovery_gaps`
record keyed by profile, OS, origin, launch mode, and session state, with a concrete
next check. An unrelated document-level gap does not satisfy this requirement.
Check state/operation consistency: an idle-start operation cannot be the only
mechanism offered for a working case, and an interrupt-only mechanism cannot
establish non-interrupting support. Require one receipt observation per mechanism
and valid, evidenced interface-inventory references.

Treat these relational and evidence checks as a separate review gate: the sidecar
validates shape, not truth or references. Do not report a schema pass as a complete
review. Flag missing read-only compatibility checks and unavailable acceptance
acknowledgments as implementation blockers rather than silently inventing them.
Source or documentation evidence alone must never be reported as passing the
mandatory live-test activation gate. `disposable_test` evidence may be cited only
when a real test record exists and its conditions match the claim.

Finish with the saved artifact path, validation result, and unresolved gaps.
Retain existing verification; use `verification: []` only when none exists.
Do not run live tests, code tests, or lints in this passive research fleet. The coordinator reviews
the pilot before the full roster and permits at most two corrective research turns
per provider; unresolved findings remain explicit after that budget.
