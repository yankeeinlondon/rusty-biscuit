---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/resume/{{state.file}}"
# NOTE: `grant:` is not implemented yet. Run with `--yolo` so the provider can
# inspect local session files under {{state.user_dir}} when they exist.
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
              - stderr: "Research for <b>{{state.name}}</b> resume is already up to date ({{ctx.today}}) — skipping."
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

Research session resume support for **{{state.desc}}**. This topic feeds Claudine's
lifecycle `resume` control action, recovery behavior, and future human-in-the-loop
continuation. Write the result to `{{file}}` and include `$schema: ./_schema.yaml` in
frontmatter.

## Required Frontmatter

Populate every applicable field from `./_schema.yaml`:

- `created`, `last_updated`, `agent`, `model`
- `docs`
- `support`
- `session_id_capture`
- `resume_invocations`
- `state_storage`
- `hitl_resume`
- `observability`
- `quirks`
- `changes`
- `requires_claudine_update`
- `reason`

Use `support: none` only when the provider clearly cannot resume a prior session.

## Research Questions

- Can the provider resume previous sessions in interactive mode, non-interactive mode, both, or neither?
- How are session IDs emitted, discovered, or inferred?
- What CLI invocations or slash commands resume a session, and can they accept a new prompt?
- Where is resumable state stored on macOS, Linux, and Windows?
- Can interrupted user questions, permission prompts, or tool approvals be resumed with supplied answers?
- Which stream events, hook events, logs, or files expose session lifecycle and failure modes?
- What caveats matter for Claudine retry, resume, proxy, or human-in-the-loop recovery?

## Body Structure

- `## Overview`
- `## Session ID Capture`
- `## Resume Invocation`
- `## State Storage`
- `## Human-in-the-Loop Resume`
- `## Observability`
- `## Claudine Integration Notes`
- `## Changelog` when `update` is true
- `## Sources`

Use current official documentation and local inspection where available. Cite sources as
Markdown links.
