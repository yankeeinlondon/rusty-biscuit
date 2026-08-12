# Fleet Prompt Authoring Standard (`_fleet.md`)

This is the canonical shape for research-fleet prompts. It codifies what the proven
prompts do (`model-config`, `agent-permissions`, `non-interactive-sessions`,
`plugins`) so new prompts start right and existing ones converge. It is a standard,
not a runnable sequence file.

## The prime directive: prose first

**The research deliverable is a prose document. Frontmatter is distilled from the
prose afterward — lifted from it, never invented separately.** A reader must be able
to learn the topic from the body alone; the frontmatter must be derivable from the
body alone. Prompts therefore:

- put the narrative **Document Structure before** any frontmatter material;
- give every body H2 **1–6 guidance bullets** (questions to answer, judgments to
  make, examples to show) — never a bare heading list;
- phrase capture instructions as *"capture the facts you documented above"*;
- never describe frontmatter as the deliverable, a "contract", or a "field guide"
  that dominates the prompt — field semantics live in `_schema.yaml` comments and in
  the body-section guidance, stated once.

## Required skeleton (in this order)

```markdown
---
sequence: "@claudine/docs/providers.yaml"
file: "{{ctx.repo_root}}/claudine/docs/research/<topic>/{{state.file}}"
# grant: is not implemented — run with --yolo (see model-config/_fleet.md NOTE)
agent: opencode
model: kimi-for-coding/k2p7
update: "{{file_exists(file) && !markdown_body_empty(file)}}"
initialize:
    stack:
        - when: "file_exists(file) && frontmatter(file, 'last_updated') == ctx.today"
          action: [ stderr: "…already up to date — skipping.", skip ]
success:
    stack:
        - when: "frontmatter(file, 'last_updated') != ctx.today"
          action: [ stderr: "…not updated…", error: "research file was not updated" ]
---

## Skills            — "Use the 'claudine' skill." (+ topic-relevant skills)

## Scope             — what this topic covers for {{state.name}}, the claudine
                       consumer it feeds, AND the boundary against every sibling
                       topic that could claim the same ground. Legacy-quarantine
                       paragraph goes here (see below).

## Document Structure — the narrative H2 sections of the OUTPUT document, each with
                       guidance bullets. This is the heart of the prompt.

## Task              — ordered steps:
                       1. (update mode ::block) read {{file}} for changelog context
                          only — never substitute old research for new research
                       2. Perform research — WITH the evidence requirement HERE:
                          "you have read access to {{state.user_dir}}; inspect the
                          actual config/logs/sessions there and prefer what you
                          observe over what documentation claims"
                       3. Write the body per Document Structure
                       4. THEN capture frontmatter: $schema: ./_schema.yaml first,
                          stamp fields (created/last_updated = {{ctx.today}},
                          agent = {{env.AGENT}}, model = {{env.MODEL || 'default'}}),
                          then per-field capture bullets referencing the body
                       5. changes: [] on fresh runs; changelog entry + changes[] on
                          update runs; requires_claudine_update + reason

## Output            — ::file @prompts/make-it-markdown.md

## Exit Criteria     — body saved per Document Structure; all frontmatter set;
                       `md schema validate '{{file}}'` returns true; no tests/lints
```

## Mandatory ingredients

1. **Evidence requirement inside the research step** (agent-permissions placement) —
   not in the capture list, not a YAML comment. Negative probes are evidence too
   ("the endpoint 404s" is a finding); unanswered ≠ omitted — record `unknown` with
   a note rather than dropping a field.
2. **Legacy quarantine** (in Scope), naming the actual files — include this only
   when prior-generation files are actually present in the topic directory:
   *"Prior-generation research files in this directory (e.g. `gemini-cli.md`,
   `kimi-code-cli.md`) are validation assets for humans — do NOT open,
   paraphrase, or cite them; your research must be independent."*
3. **Capability framing**: ask "which mechanisms exist and how do they work", never
   "does X support Y". Add topic-specific anti-pattern guards where a failure class
   is known (model-config's bridging guard is the archetype).
4. **Boundary sentences** against sibling topics, stated in Scope, reciprocal
   (if model-config cedes enumeration to agent-models, agent-models cedes
   user-extension to model-config).
5. **Concrete-example demands** with citation: every mechanism claim carries a URL
   or an observed-on-host reference.
6. **Template-var hygiene**: `{{env.AGENT}}`/`{{env.MODEL}}` (never `ctx.agent`);
   literal `{{…}}` examples for the output document must be fenced/escaped so the
   compose engine does not evaluate them.
7. **Schema single-sourcing**: the sidecar `_schema.yaml` is the only field
   contract. The prompt references it; it never re-declares field-by-field
   semantics in a parallel "field guide".

## Anti-patterns (reject in review)

- Frontmatter spec before (or larger than) the Document Structure.
- Bare-heading Body Structure lists that mirror schema keys 1:1.
- "Write frontmatter that captures these facts directly" or equivalent
  frontmatter-first framing.
- Success stacks that verify stamps the Task never instructs setting.
- Presence questions; "does not support X" without the mechanism check.
- Duplicated question lists (Deliverables vs Research Questions saying the same
  thing twice).
