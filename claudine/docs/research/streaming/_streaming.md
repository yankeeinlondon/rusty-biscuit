---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/streaming/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local stream/log fixtures under {{state.user_dir}} when they exist.
grant:
    read:
        - "{{state.user_dir}}"
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action:
              - stderr: "Research for <b>{{state.name}}</b> streaming is already up to date ({{ctx.today}}) — skipping."
              - skip
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action:
              - stderr: "The step reported success but <b>{{file}}</b> was not updated — <code>last_updated</code> is not {{ctx.today}}."
              - error: "research file was not updated"
---

## Skills

Use the 'claudine' skill.

## Scope

Research structured response streaming for **{{state.desc}}**. This topic feeds
Claudine's `stream::protocol` models and parser factory. Write the result to `{{file}}`
and include `$schema: ./_schema.yaml` in frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `docs`
- `support`
- `cli_params`
- `transport`
- `events`
- `correlation`
- `metadata`
- `parser_notes`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: none` only when the provider has no machine-readable streaming or
event-output mode.

## Research Questions

- Which CLI flags, modes, or environment variables enable structured streaming?
- Does output arrive on stdout, stderr, SSE, WebSocket, files, or another medium?
- What framing is used: JSONL, NDJSON, SSE, JSON objects, plain text, or mixed output?
- Which native events map to Claudine stream events?
- How are tool calls and tool results correlated?
- Where are session IDs, step IDs, model IDs, provider IDs, usage, and costs emitted?
- What parser caveats matter: noisy stderr, malformed JSON, partial chunks, retries, or duplicated events?

## Body Structure

- `## Overview`
- `## Enabling Streaming`
- `## Transport and Framing`
- `## Event Inventory`
- `## Correlation and Metadata`
- `## Parser Notes`
- `## Claudine Integration Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
