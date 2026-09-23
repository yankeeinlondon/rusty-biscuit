---
title: Topic-Based Fleet Research
status: draft-spec
created: 2026-09-19
updated: 2026-09-19
area: messenger
packages:
    - messenger
    - messenger-cli
reviewed: false
implemented: false
supersedes: ../2026-09-17-research-metadata-pipeline/spec.md
---
# Topic-Based Fleet Research

## Status and decisions

This draft captures the research design agreed with Ken. It is written now so
the design survives the upcoming Claudine sequence refactor. Sequence syntax,
parameter propagation, scheduling, and failure behavior must be reconciled with
that refactor before this spec is finalized. This feature does not implement
the sequence redesign or prescribe its internal architecture.

The initial topic map, refresh input policy, and supporting artifact names
explicitly labeled proposed below remain open decisions. The agreed shared
entry point is `messenger/docs/research/fleet.md`, without an underscore.
This draft does not claim the described workflow already exists.

## Purpose

Make Messenger's knowledge of messaging platforms readable, evidence-backed,
structured, and inexpensive to refresh. Research is divided into focused topics
so agents can investigate deeply, individual topics can refresh independently,
and independent work can run concurrently.

Research first produces a useful prose document. A separate agent then extracts
typed metadata, researches missing information, and integrates new findings into
the prose. Schema completeness is a valuable second perspective on research;
it must not reduce the first pass to collecting fields.

Limited overlap between topics is intentional. Independent investigations can
produce contradictory claims that reveal scope differences, version differences,
ambiguous evidence, or mistakes. Surface these contradictions for investigation;
do not silently normalize them into agreement.

This replaces the abandoned [Research Metadata Pipeline](../2026-09-17-research-metadata-pipeline/spec.md).
Its implementation is material to evaluate for reuse, not a set of requirements
inherited by this specification.

## Scope

The initial platform roster covers Discord, Slack, Telegram, WhatsApp, and
Signal. Research distinguishes Discord bot/webhook interfaces, Slack Web API/
incoming webhooks, Telegram Bot API, WhatsApp Cloud API, and signal-cli JSON-RPC.
Relevant companion receive/interaction interfaces belong in topic research.
Consumer-app behavior must not be mistaken for API behavior; a Signal REST
wrapper must not stand in for signal-cli JSON-RPC.

Deliver the executable research workflow, topic schemas, initial research,
refresh support, topic and platform summaries, optional adversarial review,
useful deterministic validation/projection, and maintained usage documentation.
Email, desktop notifications, APNs, and FCM remain outside the initial roster.

This feature does not change message delivery behavior, implement truncation,
retries, receivers, or interactive sessions. It does not implement Darkmatter's
documentation formalization or future publishing system. Automatic prompt
improvement from `research_gaps` is a possible future feature, not this feature.

## Documents and ownership

The agreed base directory is `messenger/docs/research/`:

```text
research/
  platforms.yaml
  topics.yaml
  fleet.md
  <topic>/
    _research.md
    _metadata.md
    _schema.yaml
    <platform>.md
```

`platforms.yaml` is a thin index with stable identity, a display name, output
identity, and a few useful entry-point URLs or short interface hints. It is not
a capability dictionary or a large curated evidence collection. Move the
existing `messenger/docs/platforms.yaml` role here during implementation and
update its consumers; avoid maintaining two independent platform rosters.

`topics.yaml` similarly indexes topic identity, purpose, and any necessary
prompt references. Research questions, topic evidence, and schema definitions
live in topic documents rather than inflating either index.

`fleet.md` is an executable Claudine sequence parameterized by topic. It uses
the shared platform roster and runs research followed by metadata for each
selected platform. An outer sequence uses `topics.yaml` to run all selected
topics through the same fleet. Its executable filename and configuration are
settled after the sequence refactor; do not duplicate fleet logic per topic.

Each topic owns its `_research.md`, `_metadata.md`, and `_schema.yaml`.
Common evidence, writing, and metadata-extraction instructions should be
composed from shared documents. Thin topic wrappers are appropriate; do not
introduce abstractions beyond demonstrated reuse.

Detailed research documents are the durable source artifacts. Summaries,
catalogs, reports, and skill content are derived views with links back to them.
The schema defines metadata shape; evidence establishes external facts;
Messenger's code and tests establish implementation behavior. Keep these
authorities distinct.

## Baseline research

The baseline is complete when every active topic/platform pair has a research
document with useful prose, validated metadata, citations, and honest gaps.
An individual completed document is useful immediately; there is no mandatory
all-platform promotion transaction before research can be read or used.

### Stage 1: prose-first discovery

Compose the topic's `_research.md` with platform identity, entry-point URLs,
output destination, and high-level research questions. Launch a fresh research
agent without introducing the metadata schema or field checklist.

The prompt asks for a readable explanation of the topic: what mechanisms exist,
how they work, relevant prerequisites and distinctions, examples, caveats,
edge cases, and unresolved questions. It gives sufficient direction to cover
the topic while allowing the researcher to follow important platform-specific
details. Do not force uniform headings or equal space for every question when
that would weaken the document's clarity.

Prefer primary documentation, versioned source, and release notes. Community
discussions can reveal useful leads; distinguish those leads from established
facts. Cite claims sufficiently precisely that another agent or a human can
revisit their evidence. Prior AI output is not independent corroboration.

The stage writes prose before metadata extraction begins. Successful process
exit alone does not establish that the expected document was produced.

### Stage 2: metadata and additional investigation

Launch a separate agent with the prose, the topic schema, and composed guidance
on SimplifiedSchema. Use Darkmatter's maintained
[schema index](../../../../darkmatter/docs/topics/schemas/index.md) and
[authoring topic](../../../../darkmatter/docs/topics/schemas/authoring-schemas.md)
as entry points, including relevant guidance through composition. Verify the
available grammar after the pending schema changes land; do not copy prospective
syntax into executable prompts as if it were already implemented.

The agent must:

1. Read the prose and its evidence, then populate schema-required metadata.
2. Perform additional research when the initial prose does not establish a
   required answer. A plausible inference is not an evidenced fact.
3. Integrate new findings into appropriate prose sections, preserving flow,
   legibility, and useful nuance rather than appending a field checklist.
4. Record second-pass investigations in `research_gaps` frontmatter.
5. Validate the document and correct actionable schema/semantic failures.

An unknown remains unknown when reasonable investigation cannot establish the
answer. Preserve conflicting evidence and distinguish unsupported capability
from missing knowledge. If the schema cannot represent a finding faithfully,
report the mismatch; do not change the schema merely to pass validation or
force a claim into an inaccurate category.

Schema blindness means intentional separation of stage inputs and prompts.
Do not include the schema indirectly through shared research instructions or
automatically provide the previous document's metadata to the prose agent.
This is not a claim of filesystem sandboxing. Tests should inspect the actual
composed research prompt as well as the metadata prompt.

### Recording research gaps

`research_gaps` records information that required additional investigation
during metadata extraction. Proposed fields are the research question, why the
first pass was insufficient, what was investigated, the outcome, evidence
references, and the prose section enriched by the result when applicable.

Distinguish answered second-pass questions from still-unresolved questions.
A topic may legitimately contain no second-pass gaps. Do not treat every gap
as a deficient prompt: some questions arise only through discovery and some
answers are not publicly available.

Keep enough run/date context to distinguish refresh investigations from older
ones; the exact retention shape is an open schema decision. Future learning
may use recurring gaps to suggest subtle improvements to research questions.
It must preserve exploratory freedom; automatic prompt changes are out of scope.

## Initial topic map (proposed)

Finalize the boundaries through discussion and a real pilot. These eight topics
carry forward useful questions from the abandoned work without inheriting its
single-document contract:

| Topic | Questions and intended use |
| --- | --- |
| `message-constraints` | What limits apply to fields, aggregates, payload bytes, and collections; how are they measured; who enforces them; what happens on overflow? Supports future content-preservation policy. |
| `formatting` | How do text representations, grammars, escaping, parsing, field selectors, and notification/accessibility fallbacks work? Supports renderer and payload design. |
| `images-and-attachments` | Which roles, placements, media types, upload mechanisms, captions, alt text, and transformations exist? Supports portable media intent. |
| `addressing-and-delivery` | Which destinations, replies, threads, permissions, eligibility rules, delivery controls, and receipt semantics exist? Supports correct dispatch and setup. |
| `identity-and-context` | Who is identified as sender or author, what can callers control, and what do shared/author/live locations actually represent? |
| `expressive-content` | Which reactions, stickers, emoji, and presentation effects are API-accessible, with what conditions and meaning? |
| `interactivity` | How can integrations receive text, ask typed questions, collect forms, correlate answers, and handle acknowledgment/expiry? |
| `errors-and-recovery` | How are errors and warnings recognized, what is known about delivery, and what remediation, rate-limit handling, or replay precautions follow? |

Versions, interface scope, evidence, and uncertainty accompany facts within each
topic. Keep API, SDK, and bridge versions distinct. Overlap should follow real
questions: formatting can investigate parsed-text semantics while constraints
investigates the bound on that representation. Cross-topic disagreement is
visible research output, not an automatic reason to discard either document.

## Refreshes

Support refreshing a single topic/platform document, a topic fleet, a platform
across topics, or the full baseline. Selection should explain what is missing,
due, invalid, or explicitly requested. Prompt/schema changes can require
revisiting a document independently of its age. Exact freshness configuration
and defaults remain to be chosen; inherit no mandatory numeric budgets or
approval workflow from the abandoned spec.

Recheck current evidence rather than treating old prose as proof. Preserve useful
explanations, source identities where useful, and relevant change history.
Only advance evidence dates for evidence actually checked. A failed attempt
must not present the old document as newly researched.

**Open input-policy decision:** the recommended approach is fresh prose research,
with the previous document introduced during metadata/reconciliation. An
alternative is to supply the previous body, without metadata, to the prose
stage. The former favors independent discovery; the latter can reduce repeated
work but carries more anchoring. This choice has not yet been agreed.

Use modest per-document staging or an equivalent recoverable write strategy so
a failed refresh preserves the last usable document. Do not require durable
approval records, publication manifests, or an all-fleet transaction for this.
Working paths and direct inline-compose behavior must be finalized with the
sequence refactor, including how a single-document refresh uses the same
instructions without exposing metadata to its prose stage.

## Summarization fleets

After baseline research, run both topic and platform summarization sequences.
Allow targeted regeneration later. Each agent reads the relevant research as
credible input while actively considering that some findings may be mistaken.

A topic summary compares all platforms on one subject: common approaches,
areas without uniformity, meaningful differences, implementation implications,
and justified opinions about capability, flexibility, or performance. State the
criteria and evidence for judgments; API elegance alone is not performance
evidence. A platform summary reads across topics to explain the whole platform,
how capabilities interact, prerequisites, and cross-topic inconsistencies.

Both produce readable synthesis rather than concatenating reports. Preserve
important caveats and link to detailed research. If an input is missing or stale,
make that coverage limitation explicit rather than implying a full comparison.

Every questioned fact goes into a separate dated sidecar:
`YYYY-MM-DD-questioned-facts-on-{topic}.md`, with the analogous name for a platform.
Proposed homes are `summary/topics/` and `summary/platforms/`, which also separate
topic and platform filename namespaces. Each question identifies the source
document and claim, explains WHY it may be wrong, and states evidence or a useful
next investigation. Surprising behavior alone is not proof of an error. Preserve
multiple findings from same-day reruns rather than overwriting them silently.

Summary agents report questions; targeted follow-up investigates and corrects
source research, keeping prose and metadata aligned. This avoids concurrent
summary agents editing the same source document. Findings may improve research
and yield useful human documentation and compact skill content.

## Optional adversarial review

Provide a reusable executable prompt that can target one research document,
a topic/platform fleet, or questioned-fact sidecars. It actively challenges
claims against evidence and can report findings, correct source documents, or
do both according to the requested mode. Corrections must be evidence-backed,
update prose and metadata together, and pass relevant validation. Unresolved
claims remain visible. Serialize corrections to any one document.

Adversarial review is available when useful, especially for disputed or
consequential facts. It is not a mandatory extra agent pass over every document.

## Execution and lifecycle quality controls

The desired orchestration contract is: invoke a parameterized child sequence
for each roster item, with bounded concurrency across children and ordered
research then metadata within each child. One effective concurrency policy
must prevent topic/platform nesting from multiplying agent launches unexpectedly.
Independent jobs own distinct output paths; no two agents write one document
concurrently. Metadata must not execute after its research stage fails.

Use native Claudine sequence/composition primitives and Darkmatter composition.
Do not implement a parallel Messenger scheduler or require an agent to act as
the scheduler. The refactored sequence design determines executable syntax and
how state, selection, skips, failures, interruption, and results propagate.

Lifecycle controls should:

- Report selection, stage starts, completion, skipped work, and failure reasons.
- Check expected output identity and substantive prose presence after research.
- Validate schema and relevant consumer semantics after metadata extraction.
- Resume with precise, bounded corrective instructions when recoverable.
- Preserve useful work on failure and explain how to continue the failed stage.
- Surface research gaps and questioned facts without claiming validation proves truth.

Do not substitute arbitrary word counts or rigid headings for writing quality.
Retries and resumes must be bounded, but the old persisted per-platform budget
ledger is not mandatory. Ordinary operator controls should be usable without
named approval records, a multi-command promotion ceremony, or all-platform
sign-off. Human discussion and ordinary repository review remain valuable.

Public-source research is the baseline access model. This feature does not
authorize sending messages, authenticated probes, account creation, or purchases.
Inaccessible sources produce honest gaps rather than invented verification.

### Revisit after the sequence refactor

The earlier inspection found nested sequences/groups explicitly rejected while
parallel groups already supported `max_parallel`. It also found static preflight
requiring transcluded files to exist before tasks started. These are historical
inspection findings, not constraints to preserve in the redesign.

Before finalizing this spec, verify the new nesting/iteration model, parameter
and source-path context, fresh-agent boundaries, concurrency scope, failure and
skip propagation, and how metadata consumes prose created earlier in the run.
Do not invent future YAML syntax now or rely on unverified dry-run behavior.
Link to the refactor's maintained documentation once available.

## Schemas, consumers, and migration

Design topic schemas around actual findings and useful consumer questions.
Reuse sound attributes and vocabularies from the existing work, but simplify
representations that burden authoring without improving decisions. Pilot schema
extraction against real prose before freezing the full fleet contract.

Retain meaningful semantic checks: evidence references, interface scope,
state/value consistency, unambiguous units, and valid cross-record references.
Unknown limits are not unlimited; platform support is not Messenger support.
Cross-document contradictions should be reported with scope and provenance,
not resolved by array order or hidden in generated output.

Deterministic tooling may join topic documents into reports/catalogs for
constraints, media/formatting, errors, and implementation gaps. Equivalent inputs
must yield stable output, and generated artifacts must identify their sources.
Partial coverage must remain explicit. Research does not silently change runtime
policy or `CapabilitySet`; implementing researched capabilities is separate work.

During implementation, inventory existing consumers before retaining, adapting,
or removing old machinery:

| Existing material | Migration direction |
| --- | --- |
| Research questions and interface inventory | Reorganize into focused prompts and thin identity hints. |
| Schema attributes, knowledge states, and evidence concepts | Adapt to topic contracts after pilot feedback. |
| Semantic validators and meaningful fixtures | Reuse where they still protect a real contract. |
| Catalog/report and adapter mappings | Adapt to topic inputs; keep facts separate from implementation assessments. |
| Legacy prose | Preserve useful material as historical input; do not relabel it freshly verified. |
| Approval/promotion journals and mandatory budget workflow | Retire or decouple where no longer justified; do not force new fleets through them. |
| Shared Claudine budget support | Assess independent callers before any removal; this spec does not demand its deletion. |

Migration must update affected commands, recipes, tests, dependencies, and skill
guidance together. Avoid leaving two advertised authoritative research workflows.
Do not delete unrelated research or useful historical evidence. The current task
only writes this spec and marks the previous spec abandoned; it does not execute
that migration.

## Maintained documentation and reusable setup

Specs record changes; maintained documents under `docs/` explain current behavior.
Implementation must deliver a research entry page and linked workflow guidance
covering first research, targeted refresh, summaries, questioned facts, review,
and recovery. Readers must not reconstruct operations from historical specs.

Follow progressive disclosure: index for navigation, topic pages with an early
grounding example and broad coverage, and links to detailed references. Use
appropriate `kind` metadata in line with the emerging documentation convention
without implementing the entire
[documentation formalization proposal](../../../../darkmatter/features/2026-07-10-formalizing-documentation/spec.md).

Update the reusable
[typed knowledge pipeline guidance](../../../../docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md)
and provide an executable setup prompt for other packages/repositories. The
prompt should establish consumers, topic boundaries, thin rosters, composed
research/metadata prompts, schemas, quality hooks, and a pilot. It must link to
maintained guidance, distinguish implemented primitives from prerequisites,
and avoid recreating the abandoned publication bureaucracy.

Publish compact research summaries into the Messenger skill with links to
detail. Keep this compatible with today's composition/publication mechanisms;
Darkmatter's future publishing feature is not a dependency. Update the skill
alongside the implementation so it describes the current workflow.

## Verification and acceptance

Implementation must remain portable across macOS, Linux, native Windows, and
WSL2. Use existing library abstractions for file references, content hashing,
and terminal rendering. Markdown hashes, when present, use Darkmatter; other
content fingerprints use biscuit-hash. No tests may steal terminal/browser focus.

Acceptance requires:

1. Shared `fleet.md` and thin rosters drive the agreed topic/platform coverage
   through the refactored sequence primitives, with no duplicated platform lists.
2. Composed prose-stage prompts omit the metadata schema; metadata-stage prompts
   include the correct schema and maintained authoring guidance.
3. Each job executes research before metadata, obeys the effective concurrency
   cap, and preserves distinct output ownership and useful failure diagnostics.
4. Metadata-stage additional research enriches readable prose and records
   `research_gaps`; unresolved and conflicting findings remain explicit.
5. Every active topic/platform pair produces readable, cited research and valid
   metadata. Validation never substitutes for evidence or human readability review.
6. Single-document, topic, platform, and full-fleet refresh use shared instructions;
   a failed refresh preserves prior usable content and honest freshness.
7. Both summary perspectives produce comparative insight and traceable
   questioned-fact sidecars; targeted adversarial review supports report/correct modes.
8. Retained deterministic consumers agree with topic inputs and report incomplete
   coverage honestly; no runtime messaging behavior changes through research alone.
9. Current workflow documentation, reusable guidance/setup prompt, and Messenger
   skill projection are delivered without requiring spec archaeology.
10. Migration leaves one documented workflow and accounts for retained/adapted/
    retired code and artifacts without damaging unrelated consumers.

Begin with a proposed pilot of one topic across two contrasting platforms,
including metadata-driven additional research, a refresh, and a topic summary.
Evaluate prose quality and metadata usefulness together before expanding the
fleet. Exact pilot selection remains open. Full implementation still requires
the baseline and both summary perspectives, not only the pilot.

Use fake agents and local fixtures for deterministic composition, stage ordering,
validation/recovery, and failure-preservation checks. Run package-area `just test`
and relevant lint/check recipes for changed code, using nextest rather than
`cargo test`; add L2/L3 only for actual terminal/browser interaction contracts.
Live research assesses evidence and writing quality separately from those tests.

## Open decisions for the next iteration

1. Final topic boundaries and pilot selection.
2. Fresh-prose versus previous-body input on refresh.
3. Executable outer sequence shape, parameter syntax, concurrency policy, and
   stage handoff after the Claudine refactor.
4. Lightweight freshness defaults and research-gap retention across refreshes.
5. Shared prompt locations, summary/review entry-point names, and skill publication
   mechanism using the available composition capabilities.
6. Exact retained consumer scope and retirement plan following implementation
   inventory. The old schema's breadth is useful input, not an automatic obligation.
