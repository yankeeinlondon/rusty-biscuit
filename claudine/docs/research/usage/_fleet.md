---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/usage/{{state.file}}"
# NOTE: `grant:` is not implemented yet — until it is, run this sequence with
# `--yolo` so the provider can Read files under {{state.user_dir}}; without it
# OpenCode's external_directory permission is auto-rejected in non-interactive
# mode and the research agent stops prematurely.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
# the frontmatter contract for target documents lives in the schema sidecar
# (./_schema.yaml) so the contract is single-sourced and machine-validated
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
# make interrupted fleet runs resumable: skip providers already researched today
initialize:
    stack:
        - when: "!file_exists(file) || !frontmatter(file, 'last_updated') || date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - message: "The provider **{{state.name}}** needs to update its research on **usage**"
        - when: "file_exists(file) && frontmatter(file, 'last_updated') && !date_delta(frontmatter(file, 'last_updated'), ctx.today, '14d')"
          action:
              - stderr: "The provider **{{state.name}}** has research for **usage** that is current; skipping updates"
              - skip
# a provider exiting 0 is not proof the research was written — verify the
# agent actually stamped today's date before accepting success
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
        - when: "frontmatter(file, 'last_updated') == ctx.today"
          action:
              - info: "The **Usage** research on **{{state.name}}** completed successfully: {{ link(file) }}"
              - message: "🎉  the **Usage** research on **{{state.name}}** completed successfully"
failure:
    message: "💥 the Usage research on **{{state.name}}** failed to complete!"
    warn: "The Usage research on **{{state.name}}** failed to complete! (err: {{err.message}})"
---
# Usage Research on {{state.name}}

## Skills

Use the 'claudine' skill.

## Scope

This topic covers how **usage, quota, and plan-limit state can be inspected** for
**{{state.desc}}** — the mechanisms a user or an external tool can use to query current
consumption on demand: how much of the short window (typically 5 hours) and the long
window (typically the week) has been spent, how much headroom remains, and when the
windows reset. The mechanisms of interest are API endpoints, CLI commands/switches,
interactive slash commands, and local config/log artifacts. The research feeds Claudine's
plan-awareness features: surfacing remaining runway to the user and choosing when a run
is worth starting.

The sibling `non-interactive-sessions` topic owns detection of rate-limit and quota
events **during a run** — stream events such as `rate_limit_event`, cap-approaching
warnings, and capped/no-funds failures emitted while an agent is executing. This topic
owns on-demand **inspection**: how usage and limits can be queried outside (or before)
a run. Reciprocally, `non-interactive-sessions` cedes query-style usage lookup to this
topic.

Sibling provider research files that accumulate in this directory are research
**outputs**, not sources — do not open, paraphrase, or cite another provider's
document; your research must be independent.

## Document Structure

- `## Introduction to {{state.name}} Usage Inspection` Section
    - Which plan/quota model does {{state.name}} use — subscription windows, credits,
      pay-as-you-go, or a mix — and which usage windows exist (session, 5-hour, daily,
      weekly, billing cycle)?
    - Where does the provider officially surface usage today (web dashboard, CLI,
      API)? One short orientation paragraph, not a mechanism dump
- `## API Call Opportunities` Section
    - Which HTTP endpoints report the current user's usage, quota, or limits? Give the
      exact endpoint, an example request, and the response fields, with a URL citation
      for each claim
    - Which auth does each endpoint accept — API key, OAuth/session token, session
      cookie — and which users can call it: every subscriber, or only
      admin/enterprise accounts?
    - Which reset-window information does the response carry (reset timestamps, window
      lengths)?
    - Negative probes are evidence: "the endpoint returns 404/403 for subscription
      auth" is a finding — record it with the probe you ran or the doc that says so
- `## CLI Switch Opportunities` Section
    - Which CLI flags or subcommands report usage or limits? Give the exact
      invocation and what it prints
    - Which of those mechanisms yield structured output (JSON or similar) rather than
      styled human text — for example a `--json`/`--output-format` switch on a usage
      subcommand? Structured output is far more valuable to Claudine than prose
    - Which mechanisms work non-interactively (no TTY), and which require an
      interactive session?
- `## Interactive Commands and PTY Scraping` Section
    - Which interactive slash commands surface usage (e.g. `/status`, `/usage`), and
      what exactly do they display? Note that such commands are typically unavailable
      in non-interactive sessions, which is what forces the scraping fallback
    - Produce a mini-design for scraping that display with the Rust crate `expectrl`,
      as a two-pass approach: the first pass matches the current known reporting shape
      (exact markers); the second pass runs only when the first fails — likely because
      the reporting shape changed — and takes a fuzzy-search approach to locate the
      metrics on screen
    - Make the caveat explicit: scraped TUI text carries **no schema and no stability
      contract** — unlike an API response or CLI JSON, nothing versions or validates
      it, so it is strictly a last resort and the design must tolerate drift
    ::block when="state.model_provider"
    - Because {{state.name}} is developed by a model provider, it very likely ships a
      `/status` or `/usage` slash command — verify what it shows and whether it can be
      passed as a preliminary command at launch
    ::end-block
- `## Config and Log Artifacts` Section
    - Which files under the provider's user directory record usage, quota, or limit
      state (logs, caches, session files, statsig-style telemetry)? Give paths and the
      relevant fields
    - How fresh is each artifact — written live, per-session, or only after certain
      events? Stale local data is worth documenting as stale
- `## Metrics and Windows` Section
    - For each mechanism above, which metrics does it actually yield — tokens,
      credits, currency, percent-of-window, requests — and which window does each
      metric apply to?
    - How are reset times expressed (timestamp, countdown, unit, timezone)?
- `## Limit States` Section
    - How can an inspecting tool distinguish the provider's limit states —
      cap-approaching, capped, out of funds, auth required — from each other? Name the
      markers (fields, strings, exit codes) each state produces per mechanism
- `## Sources`
    - add all useful resources that you used in your research as Markdown links

## Task

Follow these steps exactly:

::block when="update"
- Read existing research in `{{file}}`

    > **Note:** the speed at which Agentic CLI's change is rapid and therefore you should assume that the prior research is out of date. You are reading this primarily to be able to effectively report the changes into the `## Changelog` section of the document. Critically, you should never substitute information
    in the old research for doing your own (up-to-date) research.

::end-block
- Perform research on topic
    - take your time and make sure to be complete in your research
    - **Evidence requirement:** you have read access to
      `{{state.user_dir || 'the provider user config directory'}}` on this host.
      Inspect the actual config files, logs, and caches there and prefer what you
      observe over what documentation claims — local artifacts regularly contain
      usage/limit fields the documentation omits. State when no local artifact exists
      to inspect
    - every mechanism claim needs a citation: a URL or an observed-on-host reference
    - unanswered ≠ omitted: when a question cannot be settled, record `unknown` with a
      note rather than silently dropping it
::block when="update"
- Update the document with your research
- Add an entry to the `## Changelog` section
::end-block
::block when="!update"
- Write and save research to `{{file}}`
::end-block
- Set the `$schema` property of `{{file}}` to the string `./_schema.yaml`

    > This is a file reference to this topic's schema sidecar. Read `_schema.yaml`
    > (it sits next to this sequence file) before filling frontmatter — it is the
    > authoritative contract, expressed as a `SimpleSchema`, and `md schema validate`
    > will enforce it against everything you write.

- Now capture the facts you documented above into the document's Frontmatter:
    ::block when="!update"
    - `created` - set to "{{ctx.today}}"
    ::end-block
    - `last_updated` - set to "{{ctx.today}}"
    - `agent` - set to "{{env.AGENT}}"
    - `model` - set to "{{env.MODEL || 'default' }}"
    - `api` - `true` when `## API Call Opportunities` established a clear API path to
      usage data; otherwise `false`
    - `cli_switch` - `true` when `## CLI Switch Opportunities` established a clear CLI
      path; otherwise `false`
    - `structured_output` - whether any mechanism yields machine-parseable output, per
      the body sections above
    - `pty_scrape` - whether the interactive-scrape path from `## Interactive Commands
      and PTY Scraping` is viable for this provider
    - `api_methods` - one record per API opportunity documented in
      `## API Call Opportunities`
    - `cli_methods` - one record per CLI or slash-command opportunity documented in
      `## CLI Switch Opportunities` and `## Interactive Commands and PTY Scraping`
    - `pty_design` - the distilled two-pass design from `## Interactive Commands and
      PTY Scraping` (command, first-pass markers, fuzzy markers, fields, risks)
    - `metrics` - one record per metric documented in `## Metrics and Windows`
    - `limit_states` - one record per state documented in `## Limit States`
    - `docs` - the primary official documentation URL for usage/limits inspection
    ::block when="update"
    - `changes` - add a list of string descriptions which summarize the changes discovered since the last research was done
    ::end-block
    ::block when="!update"
    - `changes` - set to `[]`
    ::end-block
    - `requires_claudine_update` - set to true/false based on whether you believe there will be required code changes to **Claudine** based on the changes discovered in your research. 
        - If you respond with `true` then you must also set the `reason` frontmatter property to describe why you think that

## Output

::file @prompts/make-it-markdown.md

## Exit Criteria

You are done with this task when the Markdown "{{file}}" has been saved with:

1. all research in the body of the document 
2. and all Frontmatter properties have been set
3. running `md schema validate '{{file}}'` returns `true` (indicating that all Frontmatter was set correctly)

- you do not need to run any tests or lints
- this task had no code modifications in it
