---
file: "{{ctx.repo_root}}/claudine/docs/research/sandboxing/providers/{{seq.file}}"
topic: Provider Sandboxing
# Research rotates over three researchers by roster position. The order is
# chosen so that no agent researches its own provider. This is the only place
# the rotation is written; `agent`, `model`, and `effort` follow from it.
# Preflight can evaluate this before a roster row supplies its index.
assigned: "{{ (seq.index || 1) % 3 == 1 ? 'opencode' : ((seq.index || 1) % 3 == 2 ? 'claude' : 'codex') }}"
# `covering=<agent>` on the command line names a researcher that has reached
# its usage limit. The agent this run launches then researches that
# researcher's providers as well as its own, except a provider that is itself.
covering: none
agent: "{{ covering != 'none' && assigned == covering ? env.AGENT : assigned }}"
model: "{{ agent == 'opencode' ? 'zai-coding-plan/glm-5.3' : (agent == 'claude' ? 'sonnet' : 'gpt-6.1-sol') }}"
effort: "{{ agent == 'opencode' ? 'provider_default' : (agent == 'claude' ? 'high' : 'medium') }}"
# A sequence plans one agent before any step exists, and Claudine cannot yet
# set reasoning effort from a prompt. So the fleet runs once per researcher,
# each run naming its agent, model, and effort. `just research <topic>` does
# this, and covers for a researcher that reaches its usage limit.
# `only=<slug>` on the command line researches one provider, for a pilot or a repair.
only: all
# Read only the documents this run writes. Another run may be in the middle
# of writing one of the others.
update: "{{ env.AGENT == agent && file_exists(file) && !markdown_body_empty(file) }}"
initialize:
    - when: "seq.skip_research == true"
      then:
          - stderr: "**{{seq.name}}** is marked `skip_research`; skipping {{topic}}"
          - skip
    - when: "only != 'all' && only != seq.slug"
      then:
          - skip
    - when: "env.AGENT != agent"
      then:
          - stderr: "**{{seq.name}}** is assigned to {{agent}}, and this run launches {{env.AGENT}}; skipping"
          - skip
    - when: "agent == seq.slug"
      then:
          - stderr: "**{{seq.name}}** would be researched by its own agent; skipping"
          - skip
    - when: "env.MODEL != model"
      then:
          - warn: "**{{seq.name}}** is assigned {{model}}, and this run launches {{env.MODEL}}"
          - error: "this run launches a model outside the rotation"
    # Current means the fleet confirmed the document against this revision
    # of the contract within 14 days. The `success` event writes
    # `contract_checked`; a document the fleet rejected never carries it.
    - when: "file_exists(file) && frontmatter(file, 'schema_revision') == 1 && frontmatter(file, 'contract_checked') && !date_delta(frontmatter(file, 'contract_checked'), ctx.today, '14d')"
      then:
          - stderr: "**{{seq.name}}** has {{topic}} research confirmed on {{ frontmatter(file, 'contract_checked') }}; skipping"
          - skip
    - info: "Researching **{{topic}}** for **{{seq.name}}** with {{agent}} / {{model}} (effort: {{effort}}){{ agent != assigned ? ', covering for ' + assigned : '' }}"
start:
    - when: "!has_binary(seq.binary)"
      then:
          - warn: "**{{seq.binary}}** is not installed on this host, so the research cannot inspect it locally"
success:
    - when: "!file_exists(file) || frontmatter(file, 'last_updated') != ctx.today"
      then:
          - warn: "The agent finished, but {{ link(file) }} was not updated today"
          - error: "the research document was not updated"
    - when: "frontmatter(file, 'agent') != agent || frontmatter(file, 'model') != model"
      then:
          - warn: "{{ link(file) }} records a different agent or model than the one assigned ({{agent}} / {{model}})"
          - error: "the research document records the wrong agent or model"
    - set:
          contract: "$(md schema validate '{{file}}' --disable schema-triggers --format json)::result"
    - when: "!contract.ok"
      then:
          - warn: "{{topic}} research for **{{seq.name}}** does not satisfy its contract"
          - stderr: "{{ contract.stdout }}"
          - error: "the research document does not satisfy its contract"
    - set:
          relations: "$(python3 '{{ctx.repo_root}}/claudine/docs/research/sandboxing/providers/_relations.py' '{{file}}')::result"
    - when: "!relations.ok"
      then:
          - warn: "{{topic}} research for **{{seq.name}}** breaks a relation its contract requires"
          - stderr: "{{ relations.stdout }}"
          - error: "the research document breaks a relation its contract requires"
    - set_frontmatter: ["{{file}}", "contract_checked", "{{ctx.today}}"]
    - success: "{{topic}} research for **{{seq.name}}** satisfies its contract: {{ link(file) }}"
    - message: "✅ {{topic}} research for **{{seq.name}}** completed and validated"
failure:
    - warn: "{{topic}} research for **{{seq.name}}** failed: {{ err.msg }}"
    - message: "💥 {{topic}} research for **{{seq.name}}** failed: {{ err.msg }}"
finalize:
    # A usage limit is not cured by trying again. The provider stays
    # unresearched until the limit resets.
    - when: "err && err.category == 'cap'"
      then:
          - warn: "{{env.AGENT}} has reached a usage limit, so **{{seq.name}}** was not researched"
    # Anything else gets one more attempt. The retried run reads what the
    # gates rejected, so the researcher sees what to correct.
    - when: "err && err.category != 'cap'"
      then:
          - retry: 1
---

# Sandboxing Research on {{seq.name}}

You are the assigned researcher, already running inside the fleet. Do the
research yourself. Do not launch another agent or another research run.
Use the 'claudine' skill.

## Scope

Establish what **{{seq.name}}** ({{seq.desc}}) supplies **out of the box**
to isolate agent actions from the host, and what additional isolation its own
supported settings can enable. The subject is this roster entry's agentic CLI,
not a similarly named IDE, hosted coding product, model API, or an upstream
project it forked. Label those surfaces separately if they explain a boundary.

Out of the box has two distinct answers that the report must preserve:

1. What a clean installation actually enforces on a normal launch, with no
   user, repository, administrator, or environment overrides.
2. What sandboxing the provider ships or officially integrates but requires
   the user to enable, configure, install dependencies for, or obtain access to.

Approval prompts, plan/read-only modes, tool allowlists, working directories,
and project trust do not by themselves establish enforced isolation. Explain
which controls are application policy and which deny access at an OS,
container, virtual-machine, or remote execution boundary. A permission to
execute a shell command does not establish what that command can access.

The sibling `agent-permissions` topic owns the full approval and rule grammar.
Refer to it for context, but independently verify facts needed to explain
sandboxing. Third-party wrappers and general-purpose isolation products are
outside scope; provider-documented Docker or other runtime integrations belong
here with their installation requirements stated explicitly. Do not propose
or implement a Claudine sandbox in this research.

::block when="(contract && !contract.ok) || (relations && !relations.ok)"
## Your previous attempt was rejected

Fix every validation problem below before finishing.

::end-block
::block when="contract && !contract.ok"
```json
{{contract.stdout}}
```

::end-block
::block when="relations && !relations.ok"
```text
{{relations.stdout}}
```

::end-block
## Contract and Evidence

Read `./_schema.yaml` and `../../_types.yaml` before writing. Set
`$schema: ./_schema.yaml`, `schema_revision: 1`, and `provider: {{seq.slug}}`.
Record `agent: {{agent}}`, `model: {{model}}`, and
`reasoning_effort: {{effort}}`. Set `last_updated: {{ctx.today}}`; preserve
`created` on refresh or set it to today's date for a new report. The fleet
writes `contract_checked` only after both gates pass; do not write it yourself.

Use current official documentation, release notes, and source. Browse those
sources during this run; old research and memory are not evidence of current
behavior. Pin source links to a commit or release when possible. Each finding
cites `evidence_ids`; each evidence record states the examined version, date,
method, claim, and limitations. Separate observed behavior from documented or
source-derived behavior. Never report a source inspection as a successful
runtime test. Inference alone cannot establish support or a default.

Use the exact value `unknown` for an unresolved typed field; for an unresolved
part of a prose field, explicitly describe what is unknown. Add a `gaps` record
whose `location` names that finding and whose `next_check` could settle it.
Empty lists must be explained in the body. A lack of search results is not
proof of no sandbox support. Negative findings need evidence too.

## What to Establish

1. **Default posture.** Populate eight `default_profiles`, one for each of
   macOS, Linux, native Windows, and WSL2 crossed with interactive and
   non-interactive launch. Give the actual launch form and separate first-run
   trust/onboarding choices from steady-state defaults. If defaults depend on
   account, version, managed policy, or backend detection, state the condition
   instead of choosing a universal answer. WSL2 is a Linux guest, not proof of
   native Windows support. Mark an unsupported launch mode `unavailable` with
   evidence, and a case you cannot establish `unknown` with a gap.
2. **Available mechanisms.** Enumerate shipped and officially integrated
   sandboxes, including opt-in or experimental modes. Name the exact backend
   per environment, its dependencies and privileges, and whether the entire
   agent or only selected tool subprocesses run inside it. Split mechanisms
   when platform backends or their guarantees differ materially.
3. **Filesystem boundary.** Separate read from write access. Cover the working
   tree, additional roots, home directory, credentials, temporary directories,
   protected provider or repository files, and symlinks. Explain whether rules
   apply to native file tools as well as shell children, and whether read-only
   still allows sensitive host reads.
4. **Network boundary.** Cover outbound and inbound access, domains and ports,
   DNS, loopback, local services, proxies, and Unix sockets or named pipes.
   Separate restrictions on tool traffic from the agent's own model/API
   transport. State what is actually filtered and what remains reachable.
5. **Processes and tools.** Establish subprocess inheritance and remaining
   process, IPC, environment, and credential access. Explicitly cover shell
   tools, built-in file tools, subagents, hooks, plugins/extensions, local MCP
   servers, and remote MCP calls. Identify processes launched outside the
   boundary rather than assuming all tools share it.
6. **Controls and precedence.** Copy exact CLI flags, config keys, environment
   variables, session commands, and managed settings that change isolation.
   Provide compact examples for default launch, enabling the strongest
   supported sandbox, adding a path or network exception, and disabling or
   bypassing it. State unsupported examples honestly. Explain scope and
   precedence, including whether a session-only invocation is possible
   without editing user config. Record a YOLO/approval bypass only to explain
   whether it preserves or removes sandbox enforcement.
7. **Failure and escape paths.** What happens when the backend is missing,
   unsupported, cannot initialize, or denies access? Does the agent fail
   closed, prompt, retry unsandboxed, or fall back silently? How does this
   differ headlessly? Cover per-command exclusions, elevation, sandbox escape
   requests, and whether administrators can prohibit fallback. Do not claim
   unconditional fail-closed behavior when an escape path is enabled.
8. **Limitations.** Describe the actual protection and remaining host access.
   Identify unknown guarantees, version differences, and gaps in Claudine's
   current understanding without treating a sandbox as protection against
   every threat or equating it with protection of model inputs.

## Method

::block when="update"
- Read `{{file}}` to preserve useful coverage and describe changes. Recheck
  every retained finding against current evidence.
::end-block
- Start at {{seq.site}} and {{seq.repo || 'the vendor documentation and any officially published source'}}.
- If installed, inspect `{{seq.binary}} --version`, top-level help, relevant
  subcommand help, and sandbox-related config shapes under
  `{{seq.user_dir || 'the documented provider configuration directory'}}`.
  Record exact releases in `versions_examined`. Local user settings are
  observations of this installation, not clean-install defaults. Do not copy
  secrets or unrelated personal configuration into the report.
- If the provider is absent or another OS cannot be inspected, say so and use
  official docs or source with the limitation recorded. Load the `os` skill
  before concluding another environment cannot be tested from this host.
- This fleet is passive research. Do not launch agent sessions, install
  runtimes, change user configuration, or run sandbox escape experiments.
  List a disposable verification proposal for important untested guarantees.
- Write only `{{file}}`. Leave provider code, catalog metadata, other research
  outputs, and the shared roster unchanged.
- On creation use `changes: []`; on refresh summarize changes in frontmatter
  and add a dated changelog entry. Explain `requires_claudine_update` in
  `reason` whether true or false.

## Document Body

Write for a developer who has not used {{seq.name}}. Use these sections:

- `## Out-of-the-Box Posture` — direct answer plus a table of the eight defaults
- `## Available Sandboxes` — backend, prerequisites, and activation of each mechanism
- `## Isolation Boundaries` — filesystem, network, processes, and tool coverage
- `## Configuration and Examples` — exact supported controls and their precedence
- `## Approvals, Escalation, and Failure` — interaction with policy and fallback
- `## Limitations and Verification Gaps` — uncertainties and concrete proposed checks
- `## Sources` — links to the official sources used
- `## Changelog` — initial research or dated changes on refresh

Use a Mermaid diagram where the agent/tool/backend execution boundary is
clearer as a flow, labeling any process that remains on the host.

## Completion

Save the report, then run both gates and fix any failures:

```sh
md schema validate '{{file}}' --disable schema-triggers
python3 '{{ctx.repo_root}}/claudine/docs/research/sandboxing/providers/_relations.py' '{{file}}'
```

A schema pass establishes shape, not truth. Review every default and sandbox
claim against its evidence. Finish with the saved path, both gate results,
and unresolved gaps. No code tests or lints are needed for the report.
