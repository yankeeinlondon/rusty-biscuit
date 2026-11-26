---
sequence: "claudine/docs/providers.yaml"
agent: codex
model: gpt-6.1-sol
yolo: true
$schema:
    research_dir: "string(required; not-empty) -> Directory where provider research reports are written and checked"
    reports: "string[] -> Report paths collected by the success lifecycle event"
    invalid_reports: "string[] -> Report paths that failed validation against the research report schema"
research_dir: "{{ctx.repo_root}}/claudine/docs/research/sandboxing/providers"
initialize:
    - info: "Preparing provider sandboxing research"
start:
    - info: "Researching sandboxing across the provider roster"
success:
    - set:
          reports: "{{ find_files(research_dir + '/[a-z]*.md') }}"
    - when: "length(reports) != length(seq.outputs)"
      then:
          - error: "expected one sandboxing report per completed provider; found {{length(reports)}} reports for {{length(seq.outputs)}} providers"
    - when: "length(filter(reports, report => markdown_body_empty(report))) > 0"
      then:
          - error: "a sandboxing report has no prose research"
    - set:
          invalid_reports: "{{ filter(reports, report => !validate_schema(report, {'$schema': ctx.repo_root + '/claudine/docs/research/sandboxing/providers/_schema.yaml'})) }}"
    - when: "length(invalid_reports) > 0"
      then:
          - stderr: "Invalid sandboxing reports: {{ invalid_reports }}"
          - error: "sandboxing reports failed schema validation"
    - success: "Sandboxing research completed; {{length(reports)}} reports validated"
failure:
    - warn: "Sandboxing research failed: {{err.msg}}"
blocked:
    - warn: "Sandboxing research could not start: {{err.msg}}"
---

# Sandboxing in {{seq.name}}

Research the sandboxing **{{seq.name}}** supplies out of the box and write a
report to `{{research_dir}}/{{seq.file}}`.

The primary deliverable is a prose research document that explains what this
agent can access, what enforces its limits, and how those limits can be changed.
Write that research first. Then capture structured data from its findings in
the report's frontmatter.

## Write the Research

Explain the default behavior of a normal installation separately from optional
sandboxing available through the CLI's officially supported settings and
integrations. Describe any dependencies, setup, account requirements, or version
conditions. Focus on this CLI rather than unrelated products or third-party wrappers.

Develop a coherent explanation of:

- **Default protection:** what is enabled on macOS, Linux, native Windows, and
  WSL2, in interactive and non-interactive sessions. Use a compact comparison
  table and explain meaningful differences, including first-run choices.
- **Isolation mechanisms:** the OS facilities, containers, virtual machines,
  or remote environments involved; their prerequisites; and whether they
  contain the agent itself or only selected tools and subprocesses.
- **Access boundaries:** filesystem reads and writes, workspace and additional
  paths, home and credentials, temporary/protected files, symlinks, process
  execution, and network access. Distinguish tool network restrictions from
  the agent's own model/API transport.
- **Tool coverage:** whether shell tools, built-in file tools, subagents, hooks,
  extensions/plugins, and MCP execution share the boundary or retain host access.
- **Configuration and failures:** exact flags, environment variables, config
  keys, administrator controls, precedence, and scope. Include practical command
  or configuration examples. Explain denied operations, unavailable backends,
  escalation, unsandboxed retries, fallback, and headless behavior.

Distinguish enforced isolation from approval prompts, tool allowlists, plan
modes, and project trust. Explain whether YOLO or other approval bypasses also
change sandbox enforcement. WSL2 support does not establish native Windows
support, and read-only access does not establish confidentiality.

When no sandbox is supplied, say so directly and explain the actual guardrails.
Distinguish verified absence from unresolved support. Avoid padding the report
with hypothetical mechanisms or repeating identical claims for each platform.
Use a diagram when it helps show which processes are inside the boundary.

## Research Sources

Start with the official documentation at {{seq.site}}.
::block when="seq.repo"
Official repository: {{seq.repo}}.
::end-block
Use current documentation, release notes, and source. Corroborate with read-only
local inspection, including `{{seq.binary}} --version` and help, when available.
Existing user configuration describes that installation, not clean-install defaults.

Link sources alongside the claims they support. Identify the examined versions
and dates, and distinguish documented behavior, source-derived conclusions,
observations, and inferences. Source inspection is not a runtime test. Negative
findings also require evidence; missing documentation alone does not prove absence.
Record unresolved questions and specific checks that could settle them.

For an existing report, recheck its claims and preserve useful coverage. Add a
dated changelog entry explaining substantive changes discovered in the research.

## Capture Structured Data from the Research

After writing the prose, read
`{{ctx.repo_root}}/claudine/docs/research/sandboxing/providers/_schema.yaml` and
populate the report's frontmatter from the findings already established. The
structured data is a summary of the research, not a substitute for it. Every
mechanism, control, default, and limitation must agree with the prose and cite
the supporting evidence records. Keep unknown values explicit rather than
inventing facts to fill the schema.

Set `$schema: ./_schema.yaml`, `schema_revision: 5`, `provider: {{seq.slug}}`,
`agent: {{env.AGENT}}`, and `model: {{env.MODEL}}`. Use UTC dates for `created`
and `last_updated`; preserve the original creation date and existing content
policy. New reports use `content_policy: [ValidFor(14d)]` and `changes: []`.
On refresh, summarize the findings that changed in `changes`.

Ensure `default_profiles` covers each of the eight platform/session combinations
exactly once. When sandboxing is absent, leave `mechanisms` empty and record the
actual defaults and approval controls. Put unresolved findings in `gaps` with
the next check that would resolve them.
