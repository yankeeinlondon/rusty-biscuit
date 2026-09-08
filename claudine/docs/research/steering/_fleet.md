---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/steering/{{state.file}}"
agent: codex
model: gpt-5.6-sol
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "state.skip_research == true"
          action:
              - skip
        - action:
              - stderr: "Researching steering for **{{state.name}}**"
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - error: "Steering research did not produce a current document"
        - action:
              - action: shell
                command: "md schema validate '{{file}}' --no-trigger-schemas"
              - stderr: "Steering research schema check completed for **{{state.name}}**; evidence and coverage review remain required."
failure:
    warn: "Steering research failed for **{{state.name}}**: {{err.message}}"
---
# Steering Research: {{state.name}}

Draft research prompt for review and a pilot before full-fleet execution.
Use the Claudine skill. Research the CLI identified by roster slug
`{{state.slug}}`, binary `{{state.binary}}`, and official site `{{state.site}}`.
Write only `{{file}}` and any explicitly assigned sanitized evidence fixtures.
Do not modify production code, this prompt, the schema, or another provider's report.

## Execution Contract

The research task must run with `gpt-5.6-sol` and low thinking. The sequence
launcher must supply Codex's `-c model_reasoning_effort=low` option and verify
the actual model and effort in execution metadata. A requested model string is
not proof of the resolved model. Do not substitute another model silently.

This first pass is read-only investigation plus writing research artifacts.
Do not message, interrupt, or alter any existing agent session. Do not launch
live delivery experiments in this pass. Describe a disposable-session test when
behavior cannot be established by documentation or passive inspection; mark the
claim unverified. Do not focus terminal or browser windows or print credentials.

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

For Claude Code, independently investigate the user's report of PID-named JSON
records in `~/.claude/sessions/`, `messagingSocketPath`, authenticated Unix sockets
under `/tmp/cc-socks/` with sibling `.key` files, and next-tool-round delivery.
Also verify non-interactive startup, registry `kind`/entrypoint semantics, and idle
notices. None of these claims is established merely by appearing in this prompt.

## Metadata Contract

Read `./_schema.yaml` and include `$schema: ./_schema.yaml` in the report.
Use `schema_revision: 1`, the roster slug in `provider`, today's date
(`{{ctx.today}}`) in `last_updated`, and preserve `created` on refresh.
Record the actual research agent, model, and low effort, with provenance in the
body. Preserve useful prior findings on refresh, reverify claims, and describe
changes. Do not treat the current date as evidence of a current provider version.

Provide exactly 24 baseline cases: three OS values × two launch modes × two
launch origins × working/idle states. An impossible combination (for example,
a launch mode that cannot remain idle) needs an explicit reason, not omission.
List special startup requirements in each applicable case. If multiple launch
profiles differ, explain the alternatives and their mechanism mappings in prose;
do not imply every session in that case meets those prerequisites.

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
An empty list leaves activation blocked; it is valid research. Do not fabricate
test records or use source inspection as a substitute for a live test.

Populate `access_findings` for every mechanism/OS pair referenced by a case.
Distinguish an external sender being able to use a mechanism (`available`) from
requiring deliberate setup (`setup_required`), lacking a necessary credential or
interface (`blocked`), or unresolved evidence (`unknown`). These are researched
access facts, not activation: live verification and an implemented adapter remain
separate requirements. Explain whether prerequisites can help existing sessions.

Populate `delivery_states` for each mechanism. Record the full known state
vocabulary, including held-for-approval versus queued-for-delivery, and whether
an external sender can observe and correlate those states. The mechanism's
`acknowledgment` describes initial acceptance evidence; it does not replace this
lifecycle record. Do not assume states visible to a provider's built-in agent
tool are exposed to an independent client using the underlying protocol.

Mechanism records must describe exact discovery/destination conventions,
authentication without credentials, protocol framing, request/response examples,
and failure behavior. Long exact explanations may live in the body, with a
specific section reference in the corresponding metadata field. Do not encode
runtime shell programs for a generator to execute. Unexpected protocol families
use `other` and an explicit implementation gap rather than a misleading enum.

## Required Prose

Write an overview followed by sections for session discovery, non-interrupting
delivery, interruption fallback, idle sessions, protocol details, OS/version
compatibility, disposable-test proposals, Claudine integration, gaps, and sources.
Include a changelog on refresh. Distinguish observed, documented, source-derived,
and inferred behavior throughout. Explicitly discuss whether the mechanism could
help a token-generation loop that never reaches another tool call.

## Completion and Review Gates

Run `md schema validate '{{file}}' --no-trigger-schemas`. Check all 24 unique
cases, unique record IDs, valid cross-record references, case-matching OS/origin
discovery records, and mechanism-matching compatibility records. Every supported
case must have concrete evidence, an identified delivery mechanism, and explicit
discovery coverage or a documented discovery gap. The generator must distinguish
delivery capability from whether Claudine can actually find the session.

Treat these relational and evidence checks as a separate review gate: the sidecar
validates shape, not truth or references. Do not report a schema pass as a complete
review. Flag missing read-only compatibility checks and unavailable acceptance
acknowledgments as implementation blockers rather than silently inventing them.
Source or documentation evidence alone must never be reported as passing the
mandatory live-test activation gate. `disposable_test` evidence may be cited only
when a real test record exists and its conditions match the claim.

Finish with the saved artifact path, validation result, and unresolved gaps.
Do not run code tests or lints for this research-only task. The coordinator reviews
the pilot before the full roster and permits at most two corrective research turns
per provider; unresolved findings remain explicit after that budget.
