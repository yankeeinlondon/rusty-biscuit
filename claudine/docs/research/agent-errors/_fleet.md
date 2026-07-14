---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/agent-errors/{{state.file}}"
# Transient explicit outcome report written by the deterministic gate (spec
# D10). Its frontmatter status is clean, findings, or gate_error; absence never
# means clean. Never committed (lives under `.findings/`).
findings: "{{ctx.repo_root}}/claudine/docs/research/agent-errors/.findings/{{state.slug}}.md"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local source repositories and error corpora when researching.
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
              - message: "The provider **{{state.name}}** needs to research its **error vocabulary**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has current **agent-errors** research; skipping."
              - skip
success:
    stack:
        # 1. The step claimed success but never wrote the document. Resume the
        #    live session so it can satisfy the missing postcondition without
        #    discarding its research context.
        - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - action: resume
                message: "You reported success, but the **agent-errors** research document for **{{state.name}}** was not saved at `{{file}}` with `last_updated: {{ctx.today}}`. Complete the requested research, save the document with today's date, and run `md schema validate '{{file}}'` before finishing."
                max_attempts: 2
        # 2. Deterministic gate (spec D10): seed preservation, needle hygiene,
        #    provenance coherence (including empirical capture fixtures),
        #    invented-seed, and motivating-class coverage.
        #    A persistence failure stops the stack so stale state cannot be
        #    consumed. Completed executions atomically replace the report.
        - action:
              - action: shell
                command: "claudine providers agent-errors check {{state.slug}} --findings '{{findings}}'"
        # 3. A missing, invalid, or gate-error outcome is a failed gate, never a
        #    clean research result.
        - when: "!file_exists(findings)"
          action:
              - message: "deterministic gate produced no outcome report ({{state.name}})"
              - error: "deterministic gate produced no outcome report"
        # 4. Research-document and schema failures belong to the live research
        #    session. Authoritative gate inputs do not.
        - when: "frontmatter(findings, 'status') == 'gate_error' && frontmatter(findings, 'error_scope') == 'research_document'"
          action:
              - warn: "The deterministic gate could not validate **{{state.name}}**'s research document — resuming to correct it."
              - action: resume
                message: "The **agent-errors** research document for **{{state.name}}** could not be validated. Read the `gate_error` outcome at `{{findings}}`, correct only the provider-authored document at `{{file}}`, then save it and run `md schema validate '{{file}}'`. Do not alter the immutable seed or checker implementation."
                max_attempts: 2
        - when: "frontmatter(findings, 'status') == 'gate_error' && frontmatter(findings, 'error_scope') == 'gate_input'"
          action:
              - message: "deterministic gate input failed and requires maintainer intervention ({{state.name}})"
              - error: "deterministic gate input failed"
        - when: "frontmatter(findings, 'status') == 'gate_error' && frontmatter(findings, 'error_scope') != 'research_document' && frontmatter(findings, 'error_scope') != 'gate_input'"
          action:
              - message: "deterministic gate produced an unknown error scope ({{state.name}})"
              - error: "deterministic gate produced an unknown error scope"
        - when: "frontmatter(findings, 'status') != 'clean' && frontmatter(findings, 'status') != 'findings' && frontmatter(findings, 'status') != 'gate_error'"
          action:
              - message: "deterministic gate produced an unknown outcome status ({{state.name}})"
              - error: "deterministic gate produced an unknown outcome status"
        # 5. When the gate reports findings, resume the SAME research session with
        #    them so the model corrects its own output with full context.
        #    All resume branches share one two-additional-turn run budget; on
        #    exhaustion dispatch falls through to finalize, where the durable
        #    outcome is required to be clean.
        - when: "frontmatter(findings, 'status') == 'findings'"
          action:
              - warn: "Deterministic checks flagged **{{state.name}}**'s agent-errors research — resuming to correct it."
              - action: resume
                message: "The **agent-errors** research for **{{state.name}}** failed the deterministic gate. Read the outcome report at `{{findings}}` — its frontmatter lists each failed check (missing seed needle, non-lowercase needle, missing provenance data, invented `seed` provenance, or uncovered capacity/overload class). Fix every listed issue in `{{file}}` (re-add dropped seeds with `evidence: seed`, cite non-seed additions, attach a scrubbed `./_fixtures/...` file and capture notes to empirical rows, research or record-as-gap the capacity vocabulary), then re-save and re-run `md schema validate '{{file}}'`."
                max_attempts: 2
        # 6. Clean: the document is written and explicitly passed the gate.
        - when: "frontmatter(findings, 'status') == 'clean'"
          action:
              - info: "The **Agent Errors** research on **{{state.name}}** passed the deterministic gate: {{ link(file) }}"
              - message: "🎉  the **Agent Errors** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Agent Errors research on **{{state.name}}** failed to complete!"
    warn: "The Agent Errors research on **{{state.name}}** failed to complete! (err: {{err.message}})"
    stack:
        - when: "err.category == 'timeout'"
          action:
              - warn: "The research session timed out — resuming it with its existing context."
              - action: resume
                message: "The **agent-errors** research for **{{state.name}}** timed out. Continue from the existing session, finish the research document at `{{file}}`, and run `md schema validate '{{file}}'` before finishing."
                max_attempts: 2
        - when: "err.is_transient"
          action:
              - warn: "The research session hit a transient provider failure — retrying with backoff."
              - action: retry
                max_attempts: 2
                backoff: exponential
                delay: "30s"
finalize:
    stack:
        # Recovery re-enters before finalize, so this guard runs only after the
        # resume budget falls through or another terminal path completes.
        - when: "!file_exists(findings) || frontmatter(findings, 'status') != 'clean'"
          action:
              - message: "deterministic gate did not reach a clean outcome ({{state.name}})"
              - error: "deterministic gate did not reach a clean outcome"
---
# Error Vocabulary Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

Survey how **{{state.desc}}** reports errors in its **non-interactive structured output** and turn that into a proposed, provenance-attested **error-classification vocabulary**: the ordered keyword and numeric-code buckets that Claudine's shared cascade (`SemanticErrorKind` classification) walks to render an error's kind and summary.

The consumer is `lib/src/stream/providers/common.rs::classify_error_by_keywords`. It ASCII-lowercases the input and does **case-insensitive substring matching**, walking each provider's buckets **in order** — the first hit wins. Two things follow: every needle must be lowercase, and **bucket order is the behavior contract** (it encodes real precedence quirks). Your proposed vocabulary must respect both.

**Boundary against `signals/` (spec D9).** This topic owns the *rendering/summary* vocabulary — the `SemanticErrorKind` classification of error kind/message text. It does **not** own *detection*: wire-level records that fire `SignalKind` events (usage caps, rate-limit extraction, exit-code mapping) are the `signals/` topic's territory. If your research surfaces a detection record (a payload shape that should fire a signal), note it in the prose and cite the `signals/` document — do not encode it in this frontmatter. Where the two topics cover the same provider surface, cite each other rather than duplicating.

**Seeds are the starting point, not a ceiling.** Each parser-backed provider has an immutable Phase-A baseline in `docs/research/agent-errors/_seeds/{{state.slug}}.yaml`. Read it first. Every seeded needle and code **must** reappear in your output with `evidence: seed` unless you upgrade it to a stronger evidence class with a citation. Seeds are sticky — research proposes additions and orderings, it never silently removes, re-kinds, or reorders an observed row.

## Document Structure

The research deliverable is a **prose document** a maintainer can learn the provider's error surfaces from; the frontmatter vocabulary is distilled from this body afterward, never invented separately. Prefer verified error strings, exact enum discriminators, and numeric codes with source citations over broad guesses. Write the body of `{{file}}` using these sections:

- `## Overview`
    - One or two paragraphs: where the provider surfaces errors in non-interactive/structured mode (stream JSON error frames, structured error-kind discriminators, numeric wire codes, free-form message text), how documented/stable those surfaces are, and whether the CLI is open source.
- `## Error Surfaces`
    - One subsection per surface (e.g. `### Structured Error Kinds`, `### Message Text`, `### Numeric Codes`). For each: what it carries, its format, and whether it is a first-class contract or a diagnostic side-channel.
- One `## <Error Family>` section per researched family
    - Group related error strings: rate-limit / quota / billing, authentication / permission / config, interruption / cancellation / abort, upstream / server / provider errors, and the capacity/overload family (see below). For each family document the exact strings or codes, which surface carries them, the `SemanticErrorKind` they should classify to, and the ordering relative to other buckets (call out any precedence quirk).
- `## Capacity and Overload`
    - Explicitly research the **motivating incident** class: capacity/overload vocabulary (`overloaded`, `at capacity`, `resource_exhausted`, `429`/`503` phrasings). Codex's documented *"Selected model is at capacity"* matched no seeded needle — closing exactly this gap, with provenance, is the reason this topic exists. If you cannot confirm a provider's capacity phrasing, say so and record it in `gaps` rather than guessing.
- `## Collisions and Precedence`
    - For any broad substring you propose (`rate`, `model`, `auth`, `401`, `403`, bare HTTP numbers), check it against representative **success/non-error** prose and against earlier buckets. Evidence that a phrase exists does not prove it is a safe substring classifier; note the winning bucket and any shadowing.
- `## Quirks and Gaps`
    - Provider-specific traps, unsafe substrings, and claims you could not verify (these become `gaps` entries — never silently dropped).
- `## Changelog` (update runs only)
    - Summarize what changed since the prior research.
- `## Sources`
    - Markdown links to every source used; cite provider source code (permalinks + version tags) and official documentation first. Search-result URLs and unversioned repository homepages are not evidence.

## Task

Follow these steps exactly:

::block when="update"
- Read the existing research in `{{file}}`

    > **Note:** Agentic CLIs change rapidly, so assume the prior research is out of date. Read it to report changes into the `## Changelog` section — never substitute old research for doing your own up-to-date research.

::end-block
- Read the immutable seeded vocabulary in `docs/research/agent-errors/_seeds/{{state.slug}}.yaml`. This is your starting point: every seeded needle and code must retain its branch, bucket, semantic kind, and item position in your output unless a change is explicitly adjudicated.
- Perform research on how the provider reports errors

    > **Evidence requirement:** for open-source providers, inspect the source code (error enums, stream error frames, message constants, numeric-code definitions) and cite file permalinks pinned to a version tag. For closed providers, inspect SDK type definitions and official docs. Negative probes are evidence too. Unanswered is not omitted: record a `gaps` entry rather than dropping a family.

::block when="update"
- Update the document body with your research and add a `## Changelog` entry
::end-block
::block when="!update"
- Write and save the research body to `{{file}}`, following the Document Structure above
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml` (it sits next to this sequence file) before filling frontmatter — it is the authoritative field contract, and `md schema validate` enforces it.

- Now capture the facts you documented above into the document's frontmatter:
    ::block when="!update"
    - `created` — set to "{{ctx.today}}"
    ::end-block
    - `last_updated` — set to "{{ctx.today}}"
    - `agent` — set to "{{env.AGENT}}"
    - `model` — set to "{{env.MODEL || 'default' }}"
    - `docs` — the best official URL for the provider's error documentation; omit and record a `gaps` entry if none exists
    - `kind_buckets` — ordered buckets checked against the structured error-kind discriminator (omit entirely for a message-only classifier). Preserve seeded order; each needle carries `text` (lowercase), `evidence`, and — for non-`seed` evidence — a `source` citation. An `empirical` row also carries `empirical.fixture` (an existing, scrubbed `./_fixtures/...` file) and non-empty `empirical.capture_notes`
    - `msg_buckets` — ordered buckets checked against the free-form message; required (non-empty) for a parser-backed provider; same needle shape as `kind_buckets`
    - `code_buckets` — ordered numeric wire-code buckets (only Kimi has these today); each code carries `code`, optional `name` (its protocol constant), `evidence`, and a `source` for non-`seed` evidence; empirical codes use the same `empirical` capture object as needles
    - `gaps` — one entry per error surface you could not confirm; the capacity/overload class MUST appear here if it did not become a needle
    ::block when="update"
    - `changes` — a list of string descriptions of what changed since the last research
    ::end-block
    ::block when="!update"
    - `changes` — set to `[]`
    ::end-block
    - `requires_claudine_update` — `true` when your researched vocabulary would differ from the seeded runtime tables (a Phase C delta), otherwise `false`. Research never changes behavior by itself — this only flags the delta report
        - If `true`, set `reason` to describe the proposed delta

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done when the Markdown "{{file}}" has been saved with:

1. all research in the body, following the Document Structure
2. all frontmatter properties set, every seeded needle/code preserved, every non-`seed` needle/code carrying a `source`, and every empirical row carrying its scoped fixture and capture notes
3. `md schema validate '{{file}}'` returns `true`

- you do not need to run any tests or lints
- this task has no code modifications in it
- the deterministic gate runs automatically after your session; if it finds issues you will be resumed with the specific findings to correct
