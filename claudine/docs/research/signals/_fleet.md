---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/signals/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local stores and source repositories when researching signal surfaces.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **signals**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **signals** that is current; skipping updates"
              - skip
success:
    stack:
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Signals** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Signals** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Signals research on **{{state.name}}** failed to complete!"
    warn: "The Signals research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Signal Detection Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Survey the **signal detection** semantics of **{{state.desc}}** as it exists today. Produce per-provider detection records for Claudine's normalized signal taxonomy — the operationally critical signals (usage cap approaching, usage capped, rate limited, no funds, invalid API key, permission denied, model fallback, tokens consumed, ...) that Claudine must extract from heterogeneous provider streams, logs, and diagnostics.

This topic IS codegen-wired: the generator compiles `declarative` records into static detection tables in the generated data half; `bespoke` records are cataloged but detection stays hand-written in the behavior half. The frontmatter detection records are the deliverable; the prose body gives context per signal family.

## Signal Taxonomy (Claudine-owned)

Claudine defines the taxonomy — a fixed set of 29 semantic signals with **typed normalized payloads**. Research fills in per-provider mappings; research never invents taxonomy. The canonical signal kinds are:

| Signal Kind | Meaning |
|---|---|
| `usage_cap_approaching` | Usage limit approaching but not yet reached (warning state) |
| `usage_capped` | Usage limit reached; further requests rejected or throttled |
| `rate_limited` | Rate limit exceeded; backoff required |
| `provider_overloaded` | Provider infrastructure overloaded; transient failure |
| `retries_exhausted` | Wrapper or provider retry budget exhausted |
| `no_funds` | Insufficient account balance or credits |
| `auth_invalid` | Authentication credentials invalid or expired |
| `auth_kind_detected` | Authentication mechanism detected (for wrapper metadata) |
| `permission_denied_read` | Read permission denied for a resource |
| `permission_denied_write` | Write permission denied for a resource |
| `tokens_consumed` | Token usage metering event |
| `model_resolved` | Model identifier resolved to a specific version |
| `model_fallback` | Model fell back to an alternative |
| `provider_version` | Provider version detected |
| `generation_retried` | Generation retried after transient failure |
| `stalled_generation` | Generation stalled (no progress within timeout) |
| `repeated_stream_error` | Repeated stream errors within correlation window |
| `timeout` | **(Claudine-internal)** Global session timeout guard |
| `step_timeout` | **(Claudine-internal)** Per-step timeout guard |
| `exit_expression` | **(Claudine-internal)** Exit expression evaluated true |
| `runaway_repetition` | **(Claudine-internal)** Runaway repetition guard triggered |
| `runaway_volume` | **(Claudine-internal)** Runaway volume guard triggered |
| `unsupported_protocol_version` | Protocol version mismatch or unsupported |
| `turn_limit_reached` | Maximum turns per session reached |
| `session_time_limit_reached` | Maximum session duration reached |
| `interrupted` | Session interrupted by user or system (Ctrl+C, SIGTERM) |
| `session_tainted` | A completion claim is contradicted by an earlier error (e.g. Goose emits `error` then a status-less `complete`) — the run outcome must be treated as failed |
| `human_input_requested` | **(Reserved)** Provider requests human input (researchable) |
| `session_resumable` | **(Reserved)** Session can be resumed after interruption (researchable) |

**Note on Claudine-internal guards:** The five volume/time guard kinds (`timeout`, `step_timeout`, `exit_expression`, `runaway_repetition`, `runaway_volume`) are Claudine-internal guards that providers do NOT emit. Research should skip them UNLESS the provider has a native equivalent, in which case record it with `detection: bespoke` and explain the mapping in `bespoke_rationale`.

**Note on reserved signals:** `human_input_requested` and `session_resumable` are reserved but researchable: Claudine has no emitter yet, but record how the provider surfaces them for future integration.

## Research Methodology (Source-Code-First)

Prefer provider source code over documentation whenever public source exists; use official documentation otherwise. Always distinguish **shipped** behavior from **announced/roadmap** behavior — record announced features in the body and in `gaps`, never in the structured frontmatter as if they shipped.

### Source-Code-First for OSS Providers

Codex, Gemini CLI, Goose, Kimi, OpenCode, and Qwen (plus forks) are open source — the exact enum vocabularies, timestamp units, field names, and detection patterns are **in the repo**, not in the docs. Research MUST:

- Locate the type/schema definitions in source code (event types, error enums, stream message shapes, log entry schemas)
- Require file-path citations with permalink + version tag (e.g., `https://github.com/anomalyco/opencode/blob/v1.2.3/src/events.rs#L45-L67`)
- Extract `vocabulary` from exhaustive enum variants, not from examples or guesses
- Use `confidence: source_code` when the fact is derived from code inspection
- `confidence: source_code` beats `observed` beats `documented` beats `inferred`

### SDK Type Definitions for Closed Providers

Claude Code's `SDKMessage` union in `@anthropic-ai/claude-agent-sdk` is the de facto schema for Claude Code streams; similar authoritative artifacts may exist for other closed providers. Name these known surfaces and cite version/tag.

### Per-Signal Depth Over Per-Topic Breadth

Signal research runs as its own sequence (`signals` topic) with one document per provider whose **frontmatter is the detection records** — not a paragraph inside a broad provider-overview doc. The deliverable is machine-readable records with evidence fixtures, not just prose descriptions.

For each signal that the provider emits, produce:

- One or more `records` rows (multiple rows when version drift splits the detection pattern)
- Zero or more `extractions` rows (declarative records only; bespoke extraction stays in code)
- An `evidence` fixture under `claudine/docs/research/signals/fixtures/<provider-slug>/` proving the record

### Unanswered ≠ Omitted

If research cannot establish a unit, zone, or vocabulary member, it must emit `unspecified` or `confidence: inferred` with a `gaps` entry so the gap is tracked, never silently dropped. Unknown unit/zone → `zone: unspecified` or `unit: <omitted>` + `confidence: inferred` + a `gaps` entry.

### Evidence Fixtures (Scrubbed Corpus)

Every record MUST cite an `evidence` fixture under `claudine/docs/research/signals/fixtures/<provider-slug>/`. A seeded corpus already exists there (being added in a parallel change) — reuse seeded fixtures where they prove the record; capture NEW fixtures when a record has no seeded evidence.

**Evidence ladder (in strict preference order):**

1. **Reuse** a seeded/manifested fixture that already proves the record.
2. **Live capture** if the provider binary is available — run it and keep the real bytes (provenance class `capture`).
3. **Verbatim payload lines** lifted from the provider's own committed tests or official docs, with a permalink to the pinned commit/tag or URL (provenance class `source_shape` / `docs_example`).
4. **No fixture obtainable** → mark the record `confidence: inferred` and add a `gaps` entry. **Never author payload bytes yourself — a fixture you typed from memory is not evidence.**

Every NEW fixture REQUIRES an entry in `fixtures/provenance.yaml` (its provenance `class` plus a terse `source`); CI enforces the manifest↔corpus bijection.

**Scrubbing rules:** no session IDs, no real home paths, no user content beyond what the signal shape needs (a capped event needs the envelope and rate-limit fields, not the prompt text). A record with no obtainable fixture must be `confidence: inferred` with a `gaps` entry explaining why.

### Priority Discipline

Where several records in one source group can match the same payload (e.g., an OpenCode 429 that is ALSO a usage-cap message), order them with explicit `priority` (u16, unique within provider×source group) and explain the ordering in `distinguish`. The engine evaluates priority-ordered first-match-wins. Lower `priority` values are evaluated first.

## Document Structure

The research deliverable is a prose document a maintainer can learn the provider's signal behavior from; frontmatter is distilled from this body afterward, never invented separately. Prefer verified paths, observed field names, exact enum vocabularies, and explicit limitations over broad statements. Write the body of `{{file}}` using these sections:

- `## Overview` Section
    - One or two paragraphs: what signal surfaces the provider exposes (stream events, session logs, app logs, sqlite tables, hooks, stderr diagnostics, ACP streams, exit payloads), how mature/stable they are, and whether they are documented or internal
- `## Signal Surfaces` Section
    - One subsection per signal source (e.g., `### Stream Events`, `### Session Logs`, `### stderr Diagnostics`, `### Hook Payloads`)
    - For each: what it emits, its format (JSON lines, structured JSON, plaintext), where it is written (stdout/stderr/file/sqlite), and whether it is a first-class contract or a diagnostic side-channel
- One `## <Signal Family>` Section per Researched Family
    - Group related signals: `## Usage and Rate Limits` (usage_cap_approaching, usage_capped, rate_limited), `## Authentication and Authorization` (auth_invalid, permission_denied_read/write), `## Model Resolution` (model_resolved, model_fallback, provider_version), `## Token Metering` (tokens_consumed), `## Interruption and Recovery` (interrupted, session_resumable), etc.
    - For each signal the provider emits, document:
        - Which source(s) emit it (stream, log, sqlite, hook, stderr, acp, exit)
        - The locator (event `type`, log filename pattern, sqlite table/query, hook event name, exit code)
        - Match conditions (discriminator path + value)
        - Extraction paths for payload fields (for declarative records)
        - Observed vocabulary (for enum-shaped fields)
        - Unit/zone when the payload includes timestamps or quantities
        - Version drift (if detection changed across releases)
        - Confidence level (source_code > observed > documented > inferred)
    - If the provider does NOT emit a signal, do not invent one — omit it from the body and frontmatter, or note in `gaps` that it is unobserved
- `## Version Drift` Section
    - Document any signal whose detection pattern, vocabulary, or payload shape changed across provider versions
    - Each drift requires separate `records` rows with `since`/`until` bounds
- `## Quirks and Gaps` Section
    - Provider-specific traps and unsafe assumptions; claims that could not be verified belong here as gaps rather than being silently dropped
    - Unknown unit/zone, missing fixtures, undocumented fields, vocabulary drift
- `## Changelog` Section (update runs only)
    - Summarize what changed since the prior research
- `## Sources`
    - Add all useful resources you used as Markdown links; cite provider source code (with permalinks + version tags) and official documentation first

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLIs change is rapid and therefore you should assume that the prior research is out of date. You are reading this primarily to be able to effectively report the changes into the `## Changelog` section of the document. Critically, you should never substitute information in the old research for doing your own (up-to-date) research.

::end-block
- Perform research on the topic

    > **Evidence requirement:** for OSS providers, inspect the source code (type definitions, event schemas, error enums, stream parsers). For closed providers, inspect SDK type definitions, official docs, and local stores when available. Negative probes are evidence too. Unanswered is not the same as omitted: record `unknown`, `unspecified`, or a `gaps` entry with a note rather than dropping a field.

::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save the research to `{{file}}`, following the Document Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml` (it sits next to this sequence file) before filling frontmatter — it is the authoritative field contract, and `md schema validate` will enforce it against everything you write.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `docs` - the best official URL for the provider's signal/event documentation; if none exists, omit it and record that in `gaps`
    - `records` - from the signal family sections; one record per (signal × source × version-range) combination with:
        - `id` - unique within this document; convention `<source>-<signal>[-qualifier]` (e.g., `stream-usage_capped`, `log-rate_limited-v2`, `exit-no_funds`)
        - `signal` - from the 29-member taxonomy enum (exactly as spelled in the table above)
        - `source` - from `stream`, `session_log`, `app_log`, `sqlite`, `hook`, `stderr_diagnostic`, `stderr_promoted`, `acp`, `exit`
            - `stderr_diagnostic` = free-form diagnostics on stderr
            - `stderr_promoted` = promoted-structured stderr (e.g., OpenCode `--print-logs`, a contract channel)
            - `acp` = ACP `session/update` streams
            - `exit` = wrapper-synthesized `{exit_code, stderr_tail}` payload
        - `locator` - source-specific: event `type` path for streams (e.g., `rate_limit_event`), path template for logs (e.g., `~/.provider/sessions/*.jsonl`), table/query for sqlite (e.g., `SELECT * FROM events WHERE kind = 'usage_cap'`), hook event name (e.g., `SessionEnd`), exit code for exit source (e.g., `1`)
        - `detection` - `declarative` if single-payload matching via path+operator+value; `bespoke` if cross-record/temporal state required
        - `priority` - u16, unique within this provider's (source) group; lower values are evaluated first; the engine evaluates priority-ordered first-match-wins
        - `match_path` - restricted JSONPath subset: dot segments + numeric bracket indices ONLY (`error.responseBody.code`, `choices[0].finish_reason`); no wildcards/filters/recursive descent; required for declarative records
        - `match_op` - from `eq`, `in`, `substring_ci`, `regex` (regex is anchored, compiled/validated at generate time)
        - `match_value` - for `eq`, `substring_ci`, `regex` (single value)
        - `match_values` - for `in` (value set)
        - `distinguish` - prose for humans; how to tell it apart from near-identical events; the machine guarantee is the generate-time mechanical-overlap check
        - `vocabulary` - observed enum values at this site (for enum-shaped fields the provider emits)
        - `since` - provider version this record applies from (omit if unknown or always)
        - `until` - provider version this record applies until (omit if unknown or current)
        - `confidence` - from `inferred`, `documented`, `observed`, `source_code` (source_code > observed > documented > inferred)
        - `evidence` - path to a committed fixture under `./fixtures/<provider-slug>/` proving the record (file reference, not URL)
        - `notes` - optional clarifications
    - `extractions` - from the signal family sections; one record per extracted payload field (declarative records only) with:
        - `record` - joins to `records[].id`
        - `field` - normalized payload field name of the signal's typed payload (e.g., `lifts_at`, `remaining`, `window`, `message`)
        - `path` - same restricted JSONPath subset as `match_path`
        - `unit` - from `unix_seconds`, `unix_millis`, `unix_nanos`, `iso8601`, `duration_secs`, `percent`, `tokens`, `requests`, `usd` (omit if not a quantity/timestamp)
        - `zone` - from `utc`, `local`, `embedded_offset`, `unspecified` (for timestamps only)
        - `notes` - optional clarifications
    - `bespoke_rationale` - optional prose entries, one per bespoke record, saying WHY it cannot be declarative (cross-record/temporal state)
    - `gaps` - from `## Quirks and Gaps`; unverified claims, missing evidence, unknown unit/zone, undocumented fields
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** based on your research. This topic IS codegen-wired, so `true` is the expected answer when new detection records or changed signal vocabularies are discovered.
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document, following the Document Structure
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
