---
title: Messenger platform research fleet
description: Shared instructions for the three research passes over each roster platform
---

# Messenger Platform Research Fleet

These are the shared instructions for researching one platform from the
[roster](../../platforms.yaml). Every platform, every pass, and every
single-document refresh uses this file. Do not copy these instructions into a
platform document.

Use the 'messenger' skill.

## Your Role and Its Limits

You research **one platform per run**, in one of three passes. The run's
prepared inputs name the platform, the pass, and the exact paths you may
write. Those paths are the only files you may create or change.

- The roster owns identity: `platform_id`, interface IDs, adapter IDs, and
  curated sources. Use its IDs byte-for-byte. Never invent an interface or
  adapter, and never add a platform.
- You write **candidates**, never accepted research. The accepted documents
  under `messenger/docs/research/platforms/` are read-only inputs. Promotion
  happens only after validation, a delta review, an independent evidence
  review, and the applicable human approval.
- Messenger's code owns implementation status. Never state that Messenger
  implements a feature, and never propose a change to `CapabilitySet` or
  adapter behavior. When research reveals something Messenger should act on,
  record a `requires_messenger_update` gap.
- A zero exit code proves nothing. The run succeeds only when the expected
  artifacts exist, validate, and record the evidence you actually checked.
  Each pass's prepared prompt names the exact output paths and JSON shapes;
  `messenger research check-run` judges them after each pass.

## Access Policy

Preauthorized:

- Reading unauthenticated public sources, including official documentation,
  public source code, release notes, and public developer discussions.
- Writing the candidate artifacts named in your run inputs.
- Running validation (`md schema validate`, `messenger research validate`).

Requires separate human approval. **Stop and record a gap instead**:

- Authenticated access, logging in, or creating accounts, tokens, or apps.
- Live probes: sending messages, calling a provider API, or posting anywhere.
- Adding external services, global installations, or purchases.

If a source is inaccessible, record the attempt and the failure. Never count
it as checked, and never cross an approval boundary to reach it.

## Evidence Rules

These apply in every pass.

- Every fact cites source IDs. Each source records a kind
  (`official_docs`, `source_code`, `sdk_validator`, `observed_fixture`,
  `secondary`), a URL or repository-relative location, a locator within the
  source, the date **you** retrieved it, and a version or revision when one
  exists. A retrieval date is never a release date.
- Prefer official documentation and versioned source. For signal-cli, the
  bridge's own source is primary evidence for bridge behavior.
- Existing research, generated summaries, other AI output, and this
  repository's prose are never independent corroboration.
- Secondary sources (forums, Q&A, social threads, blog posts) are research
  leads. A claim backed only by secondary sources stays visible with
  `confidence: low` and is never treated as authoritative.
- Keep source kind separate from confidence. Official documentation can still
  leave counting semantics ambiguous.
- Store links, concise findings, and necessary attribution only. Never paste
  raw transcripts, whole social threads, credentials, recipient identifiers,
  message bodies, or terminal control sequences into any artifact.

## The Contract You Fill

Platform documents use the schema in
[`_schema.yaml`](./_schema.yaml), with named types in
[`_types.yaml`](./_types.yaml). Read both before writing. The Rust-owned
rules in [`_rules.md`](./_rules.md) also apply. A candidate that passes the
schema but breaks one of those rules is rejected.

- **Knowledge states.** `known` means evidence supports the value within its
  scope. `unknown` means research has not established a usable value.
  `conflicting` means applicable sources disagree; keep every claim.
  `not_applicable` means the fact does not apply; explain why.
  **Unsupported** is a known capability value, not unknown. **An undocumented
  bound is not unlimited.**
- **Coverage.** Each interface has a `coverage` matrix with every category.
  Mark a cell `researched` only when records exist. Use `not_applicable` with
  an explanation, or `gap` with an investigated gap. An empty array never
  means "no constraints".
- **Investigated gaps.** An unknown passes review only as an investigated
  gap. It records the question, the searches performed, the sources
  inspected, why it remains unresolved, the decision it blocks, and the next
  useful investigation. A bare "unknown" is a placeholder, not research.
- **Constraints.** Scope every bound to an interface, operation, and surface.
  Keep field, aggregate, byte, and count limits separate. Record the unit
  exactly as evidenced. "Characters" without a definition is
  `unspecified_characters`, not Unicode scalars. Record the measurement
  stage: source Markdown, parsed text, and JSON-escaped bytes are different
  quantities. Entity offset units do not establish length units. Keep
  recommendations (`recommended_max`) apart from enforced thresholds. When
  two bounds apply at once, keep both.
- **Conditions.** Use typed `applies_when` records. When a condition cannot be
  expressed with an operand, attach a gap. Never write executable logic.
- **Versions.** Record the latest stable and any public preview provider API
  versions per interface. Keep SDK and bridge releases separate (`subject`).
  Distinguish an API established to be unversioned from one whose versioning
  you did not establish. Preserve every earlier chronology entry, and correct
  one only with evidence.
- **Stable IDs.** Reuse existing fact, source, and gap IDs. Never reuse a
  removed ID. Record additions, changes, removals, and unresolved items in
  `changes`.
- **Dates.** Preserve `created`. Set `last_updated` only when the run
  actually checked evidence. Never refresh the observation date of a source
  you did not recheck.

## Research Questions

Answer these for every interface in the roster entry, including research-only
companions where a category applies. Depth follows Messenger's outbound
consumers; do not catalog unrelated administrative APIs.

1. **Versions.** Latest stable and preview API versions, the known release
   chronology, and SDK or bridge releases that change behavior.
2. **Constraints.** Every text surface Messenger emits or could emit: body,
   summary, captions, rich-object fields, blocks, template and interactive
   fields, alt text, and filenames. Record units, stages, enforcers, overflow
   behavior (reject, truncate, split, transform), lower bounds, aggregates,
   request bytes, and item counts.
3. **Formatting.** The grammar per text surface: family, dialect, the full
   construct inventory, escaping, nesting, newlines, automatic link and
   mention interpretation, and malformed-syntax behavior.
4. **Text bindings.** Where each representation is submitted: fields, mode
   selectors with allowed values and defaults, field relationships and
   precedence, and notification or accessibility fallbacks.
5. **Images.** Every native image slot with its canonical role, placement and
   who controls it, submission mechanisms, sources, collection form, counts,
   captions versus alt text, MIME types, and transformations. Fill the role
   matrix for every canonical role.
6. **Attachments.** Media kinds, upload mechanisms, MIME restrictions, size
   and count limits, captions, and filenames.
7. **Addressing and receipts.** Destination kinds, reply versus thread
   support, required reference fields, returned identifiers, and accepted
   versus delivered versus read states.
8. **Attribution.** Who the platform identifies as the sender, what the caller
   can change, and at what scope. Does a field describe the author, or
   change the attributed sender? Answer both separately.
9. **Location.** The subject (author, device, place, or unspecified), origin,
   inclusion, representation, delivery relationship, and live behavior. Also
   give an explicit answer on whether the author's geolocation is exposed.
10. **Expression.** API-addressable effects versus app-only features,
    reactions, emoji, and stickers. Record native identifiers, control,
    scope, discovery, and rendering conditions. Map an effect to an intent
    only with evidence, and label approximate mappings.
11. **Interactivity.** For each integration combination: inbound text reach
    and content, typed questions (`confirmation`, `single_choice`,
    `multiple_choice`, `text_input`) with answer bindings, cancellation,
    correlation, and lifecycle. For forms: packaging and submission
    semantics. Record companion interfaces and their prerequisites. Never
    infer a capability from the platform name.
12. **Delivery controls.** Silent delivery, link-preview control, mention
    control, edit, and delete.
13. **Eligibility.** Authentication, scopes and permissions, membership or
    registration, conversation windows, and template prerequisites.
14. **Rate limits.** Scope, whether a limit is documented, observed, or
    dynamic, the retry-after location and unit, and idempotency support.
15. **Errors.** The response envelope per interface, then scoped error
    records with match signatures built only from structured identifiers or
    documented exact tokens. Record delivery certainty, recovery,
    replay safety, and remediation. Include sanitized fixtures, and label
    each as captured, constructed from documentation, or observed in this
    repository.

Required investigations per interface:

| Interface | Investigate explicitly |
|---|---|
| `discord_bot_api`, `discord_webhook` | Content; summary-to-content mapping; embed description and other text fields; the combined embed budget; attachment descriptions, counts, and sizes; SDK versus HTTP enforcement |
| `slack_web_api`, `slack_incoming_webhook` | Top-level text; notification fallback; recommendation versus truncation; block-specific and aggregate limits; response warnings; interface differences |
| `telegram_bot_api` | Text and media captions; parse modes; parsed text versus markup; entity units and counts; media groups; hosted versus local Bot API |
| `whatsapp_cloud_api` | Free-form text; media captions; template components and interactive fields; API-version and conversation-window conditions |
| `signal_cli_jsonrpc` | Text and attachment behavior; bridge and version constraints; service versus client limits; any long-text transformation; evidence gaps. A REST wrapper's behavior is not evidence for JSON-RPC |

## Pass 1 — Independent Discovery

**Inputs.** The platform's website and API URL, the interface identification
from the roster, these instructions, and the schema. **You receive no
previous research and no curated source list, and you must not seek them
out.** Do not open `messenger/docs/research/`, the roster's
`curated_sources`, or earlier run records.

**Work.** Find the sources needed to answer the research questions: official
documentation, versioned source, changelogs, and developer discussions on
public forums and social platforms. Discussions show where developers hit
undocumented behavior. Treat them as leads.

**Output.** The discovery report at the path in your run inputs:

- findings per research question, each with the sources that support it;
- open questions you could not resolve;
- `suggested_sources`: each URL with the questions it answered and what it
  contributed that other sources did not.

Suggestions are leads. Appearing in this list never makes a source evidence
or a curated source.

## Pass 2 — Curated Reconciliation

**Inputs.** The discovery report, the platform's curated sources from the
roster, the previous accepted document (when one exists), and the candidate
path.

**Work.**

1. Attempt every curated source. Record each attempt in the source-check
   record: URL, date, outcome (`checked` or `inaccessible`), and what it
   established or why it failed. An inaccessible source is never counted as
   checked.
2. Review the accessible sources thoroughly. The curated list is a starting
   point, so cite further sources wherever the evidence is.
3. Reconcile with the previous document and the discovery report. Keep
   stable IDs, `created`, useful explanations, and chronology entries. For
   the initial migration, treat numeric claims in the old prose as
   unverified until a current source re-establishes them.
4. Write the candidate: typed frontmatter that satisfies the schema, and
   prose that explains the facts. Keep existing section structure and useful
   explanations where they remain accurate.
5. Record every evidence-backed change, and every unresolved conflict, in
   `changes`.
6. Validate the candidate's shape with `md schema validate <candidate>` and
   fix every problem. Semantic validation runs after this pass; never edit
   the schema to make a candidate pass.

**Output.** The candidate document and the source-check record.

## Pass 3 — Source-List Maintenance

**Inputs.** The discovery report's `suggested_sources`, the current curated
list, the source-check record, and the per-platform cap from the roster
(`curated_source_cap`, initially 10, shared across the platform's
interfaces).

**Work.** Judge each current and suggested URL on authority, relevance to the
researched interfaces and versions, currency, accessibility, and coverage no
other listed source provides. Broken, obsolete, and redundant links are
removal candidates. Retaining a link requires a current contribution, not
just its presence in the roster.

**Output.** A source-list proposal at the path in your run inputs:

- `retain`, `add`, and `remove` entries, each with its URL, the interfaces it
  serves, and its contribution (or the reason for removal);
- at capacity, every addition names the source it replaces, and the coverage
  gained and lost.

The proposal never edits the roster. A human maintainer approves every
curated-list change.

## Refreshing a Single Document

A platform document's `prompt` delegates here for a single-document refresh.
That refresh is Pass 2 only, and must run against a **candidate copy**,
never the accepted file. Independent discovery must never run through
inline-compose or from a sequence whose source has a top-level `prompt`,
because both hand the worker the existing document and its prior prose.

## Before You Finish

- Every roster interface for the platform appears, with a complete coverage
  matrix.
- Every required investigation above has an evidenced finding or an
  investigated gap.
- Every fact cites sources you actually retrieved, with honest dates.
- Nothing claims Messenger implements anything.
- No credentials, identifiers, message content, raw transcripts, or control
  sequences appear in any artifact.
- `md schema validate` passes on the candidate.
