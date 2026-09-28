---
created: 2026-09-27
status: draft-spec
clarified: false
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-27
review_iterations: 2
completed: true
implemented: true
$schema:
    status: |-
        enum(
            draft-spec,
            finalized-spec,
            planned,
            implemented,
            review-findings,
            human-in-the-loop,
            completed,
            on-hold,
            abandoned
        ) -> an indicator of progress for this specification
    reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
    reviewed_by: string -> the agent and model used in the spec review
    reviewed_on: date -> the date the spec was reviewed
    review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
    clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
    implemented: boolean -> indicates whether this spec's plan has been implemented
    implemented_by: string -> the agent who implemented the plan
area: claudine
packages:
    - darkmatter
    - claudine
    - claudine-cli
related:
    - 2026-09-27-union-partial-file-completion
    - 2026-09-20-lifecycle-handoff-gaps
human_review: false
message_to_agent: |-
    Phase 5 is complete (see implementation-log.md "## Phase 5"). For Phase 6:
    - Docs already updated in Phase 5 (extend, do not duplicate):
      claudine/docs/topics/composition.md (closure Repair/Encode steps and the
      guardrail plain-scalar rule), claudine/docs/topics/state-management/
      side-effects.md (effect writes store data as tokens),
      darkmatter/docs/inline/interpolation.md (holds_pending_syntax,
      locate_frontmatter_leaves), darkmatter/docs/topics/schemas/definition.md
      (data is never pending). The claudine SKILL.md has a new
      "Inline persistence" row.
    - Behavior to document that the plan's Phase 6 list does not name:
      * Every value a lifecycle set_/merge_/append_/prepend_frontmatter
        writes is data, so a string holding `{{`/`$(` is stored as a token.
        An author can no longer write a template into a file with those
        effects (an authored `{{{ x }}}` literal writes the text `{{ x }}` as
        data). This was I2 B14 "author to confirm"; implemented as planned.
      * Agent-written malformed YAML and duplicate keys now fail as
        CompositionError::InlineAgentFrontmatterRejected (code
        document.invalid_frontmatter, line + agent attribution), not
        InlineArtifactEditFailed.
      * A `sequence:` value that is a literal token is refused
        (SequenceInvalid).
      * Ownership compares list items by index: inserting at the front of an
        authored list encodes shifted `{{ … }}` items.
      * A clipped or kept block scalar that ends the frontmatter is stored as
        compose reads it (no final newline); S2's "reject `|+` at the end" was
        not needed.
    - Known limits, recorded rather than fixed: the Darkmatter `expression`
      format validator is string-only, so decoded data holding `{{` in an
      expression-typed field is still accepted lexically; Claudine's public
      pre_validate_schema[_for_mode] treats overrides as authored (only the
      sequence JIT uses the origin-aware pre_validate_layered_for_mode).
    - Pre-existing, unrelated, Windows only: claudine lib test
      composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message
      fails on native Windows at the unmodified HEAD too (ctx.repo_root renders
      `B:/…`, the test expects `Path::display()`'s `B:\…`). Do not attribute it
      to this fix in the final validation.
---

# Agent-Produced Text Is Data, Never Instructions

## Summary

Text an agent produces can be read back as a template or shell command. The
observed failure is a missing distinction between *authored instructions* and
*data returned by an operation*. This spec establishes that distinction across
Darkmatter's composition passes, keeps captured output as raw data in
Claudine's runtime, and encodes agent-written frontmatter before it is saved.
It also defines a narrow repair for common agent-written YAML errors.

Command-line setters (`key=value`, `--set`) are **not** affected: a person
wrote them, and they remain templates (see *Decisions*).

## How the failures happen

### Observed failure

Running `prompts/_implement/implement-plan.md` in a loop against
`2026-09-27-union-partial-file-completion`: phase N's agent summary mentioned
"templated `{{…}}` and `$(…)` siblings". The loop captured that summary as
`_loop_last_output`, merged it into the next iteration's frontmatter, and
frontmatter fill-in tried to evaluate `…`:

```text
MarkdownError: interpolation failed
The _loop_last_output frontmatter property failed to evaluate `…`:
parse error: Unexpected character: '…' at position 0
Defined in: …/prompts/_implement/implement-plan.md
```

The error blames the document because after the merge nothing records where
the key came from.

### Where agent text re-enters

| Entry point | What goes wrong today |
| ----------- | --------------------- |
| Captured agent output in a loop (`_loop_last_output`) | Merged into frontmatter, then filled in as a template (the observed failure). |
| Captured task output in a sequence (`outputs`, and the step overlay that exposes it) | Same path, recursively through arrays and objects. |
| Frontmatter keys an agent adds or changes in an inline document | Saved to the file. On the next run (rerun, or next loop iteration) they are indistinguishable from author-written templates and are filled in. |
| Lifecycle values derived from the above (`set:` results, `proxy.with:` values) | Stored after evaluation, then merged and filled in again on the next preparation. |
| Agent-written files read by expression (for example `frontmatter(log, 'message_to_agent')`) | The value is inserted correctly, but Claudine's lifecycle message check then sees braces in the finished message and refuses to send it. |

The implementation inventory must confirm each row, including the exact
frontmatter and body preparation path, and inspect other file and runtime
inputs. A file read by an authored expression is data even when an agent
wrote it; authored expressions and shell directives in the document itself
remain instructions.

### Why escaping alone does not work today

Darkmatter's escape for braces is `{{{ … }}}`, meaning "show `{{ … }}` as
text". It lasts for one fill-in step only:

```yaml
area: claudine
note: "fixed {{{ area }}}"
```

With a body of `Body: {{ note }}`, `md compose` prints `Body: fixed claudine`.
Frontmatter fill-in removed the escape (`note` became `fixed {{ area }}`), and
body fill-in then re-read the inserted text as a template. With `{{{…}}}` the
second step fails with the same error as the observed failure.

The shell form has the same shape. Frontmatter fill-in runs *before*
Darkmatter looks for whole-value `$( … )` commands, so a template that
inserts the text `$(cmd)` produces a command. Darkmatter's post-expansion
guard also rejects command *output* that merely looks like `$( … )`.

### Reproduction (Darkmatter only)

```sh
printf -- '---\ntitle: t\n---\nLast: {{ note }}\n' > repro.md
md compose repro.md --set '{"note":"see {{…}} siblings"}'   # interpolation failed
md compose repro.md --set '{"note":"$(echo INJECTED)"}'     # shell approval requested

printf -- '---\narea: claudine\nnote: "fixed {{{ area }}}"\n---\nBody: {{ note }}\n' > esc.md
md compose esc.md                                            # "Body: fixed claudine"
```

### Severity

- **Crash:** any iteration or step whose predecessor wrote malformed
  template syntax fails to prepare. Agents working on this codebase routinely
  write `{{ … }}` in summaries.
- **Silent corruption:** well-formed syntax (`{{ ctx.repo }}`) is replaced by
  its value, so captured output no longer says what the agent said.
- **Command execution:** an agent-written whole-value `$( … )` is offered for
  execution on the next run, and under `yolo` may run unprompted. Agent text
  must never become a command.

## Requirements

### R1: Darkmatter: inserted text is never re-read

1. Scan each authored source segment for `{{ … }}` exactly once. The text
   returned by an expression, file read, shell command, or literal escape is
   data, even when it contains a valid expression or literal escape. Preserve
   that origin while assembling frontmatter, body, and transcluded text;
   flatten to plain text only after all applicable instruction scans finish.
   A single-pass string rewrite without origin tracking is insufficient if a
   later stage scans the assembled result.
2. Execute a frontmatter shell command only when the **authored source value**
   is a whole-value `$( … )`. Interpolation may supply its arguments, subject
   to the existing executable-token rule, but cannot create the command
   shape. Shell output stays data. The shell preflight and runtime must use
   the same decision, including when shell expansion is disabled.
3. The existing `{{{ … }}}` authoring escape still yields literal `{{ … }}`.
   Its result stays data in later passes. This is an intentional change to
   Darkmatter's documented fixed-point behavior for mixed frontmatter and
   body text; authored nested expressions must instead be written as separate
   authored spans or composed with expression concatenation.
4. Darkmatter's post-expansion shell guard and Claudine's lifecycle surviving
   span check inspect unresolved **authored** syntax, not data returned by an
   expression. Neither guard may be removed or reduced to a raw string check
   after provenance is flattened. Authored unresolved whole values still
   fail under the existing strict rules; mixed text retains its documented
   lenient warning behavior.

Cover frontmatter, body, transclusion, lifecycle messages and action operands,
loop actions, `set:`, `proxy.with:`, and sequence overlays. Existing typed
whole-value results, null handling, and Markdown-aware scanning must survive.
The implementation must identify any authored template that relied on
fixed-point rescanning and migrate it or document its new output.

### R2: Darkmatter: a total on-disk literal encoding

An inline document needs to store agent text in frontmatter so that a later
run can distinguish it from authored template syntax. Darkmatter owns a
versioned, unambiguous literal token, for example
`{{!data:v1:<base64url-encoded UTF-8>}}`, and an encoder/decoder for it. This
token represents one *string value*, including the empty string, and is
decoded once after instruction scanning. Decoded bytes are data and cannot
be scanned again. The token is also exempt from whole-value shell detection.
The exact token spelling can change during implementation if it conflicts
with the existing expression scanner, but these semantics cannot.

The encoder must round-trip every Unicode string, including braces, `$(`,
backslashes, newlines, quotation marks, and strings resembling the token
itself. It must produce YAML-safe source when used as a scalar value. A
malformed literal token fails with a source-located diagnostic; it cannot fall
through as an expression or shell command. Encode from the raw agent value
exactly once, based on its origin, never by testing whether the value *looks*
encoded: a raw agent string may itself resemble a valid token. An unchanged
stored token is left alone on a later run. The public decode path returns
ordinary text for consumers and schema checks; the source file retains the
token until a person edits it.

### R3: Claudine: preserve data at runtime and encode at persistence

Captured loop and task output remains raw in `_loop_last_output`, `outputs`,
logs, results, predicates, and functions such as `last(outputs)`. Claudine's
[runtime output store](../../lib/src/composition/runtime_state.rs) keeps those
values typed. Preserve
its data origin when the runtime projects it into effective frontmatter or a
step overlay. Recursive arrays and objects keep their shape; string leaves
remain inert. Do not make encoded tokens visible to these runtime consumers.

For inline documents, one closure-stage function encodes each agent-added
or agent-changed frontmatter **value** after the narrow YAML repair and
restoration of the closure-owned `prompt`, `hash`, and `last_updated`
properties, but before hashing, completion schema validation, and the single
atomic write. Unchanged author values keep their template behavior and
original bytes. Compare parsed value trees when
possible, while retaining source spans for precise edits; whitespace-only
rewrites do not make an authored value agent data. A changed container owns
only its changed string leaves, so unchanged authored siblings still work.
If source spans or ownership cannot be determined safely, fail with an
attributed diagnostic instead of encoding the entire document.

Lifecycle `set:` and `proxy.with:` values derived from expressions retain
their data origin through later preparation. Keep their existing typed
values; do not serialize runtime mutations as templates. A shared boundary
API should make origin preservation the default for all these entry points,
with focused tests guarding against a second ad hoc escaping path.

Inventory every Claudine reader of inline frontmatter and route it through
the decoded-value API. A tool that reads the YAML file directly will see the
literal token; document this on-disk contract, and do not claim transparent
readback for consumers outside Darkmatter and Claudine.

### R4: Claudine: repair agent-written YAML in inline documents

Repair runs before owned-property restoration and R3's value encoding:
Darkmatter's current
[restore_properties_text](../../../darkmatter/lib/src/markdown/hash/write.rs)
function requires parseable YAML, so
restoring first would reject precisely the malformed candidate this step
should repair. The repair must never modify an owned property.
The repairer uses the pre-run document and lexical YAML spans to identify a
new or changed **top-level** key. It may quote only an unquoted, single-line
scalar that is unambiguously meant as text. Preserve its complete source
value, including `#` text and `: `, and preserve line endings. A quoted
scalar, block scalar, number, boolean, null, collection, and unchanged key
must not be rewritten. In particular, do not guess that an authored YAML
comment on an unchanged line is part of the value.

Re-parse the entire frontmatter after repair. Duplicate keys, bad nesting,
missing delimiters, or syntax outside the narrow repair case fail with a
diagnostic that points to the candidate line and
attributes it to the agent when the pre-run comparison supports that claim.
Do not write a partially repaired artifact; retain the existing inline
rollback and recovery behavior. Tests must cover a nested map, a
block scalar, duplicate keys, CRLF, and a value that is already valid YAML.

**Limit of automatic repair:** `summary: fixed #42` is valid YAML, so no
parser can know whether `#42` was meant as a comment or text. For an
agent-added or changed plain-text scalar, this spec chooses to preserve the
whole line as text. If the agent intended a YAML comment, it must put the
value in quotes or use a separate comment line. This choice must be stated
in the inline agent guardrails.

### R5: Diagnostics name the true origin

An interpolation or shell error on a key that came from an override (for
example `--set`) says so and does not present the document frontmatter as the
definition site. Keep the source location for authored document errors;
attribute malformed on-disk literal tokens and repair failures to the agent
edit when that provenance is available. A generated value containing
template-looking text must not produce a false authoring error.

### R6: Documentation

- Darkmatter interpolation and frontmatter-shell docs, and the `darkmatter`
  skill: document R1's single-pass rule, the R2 literal token, the intended
  fixed-point behavior change, and the origin-aware guards.
- Claudine looping, frontmatter-properties, composition, and lifecycle docs,
  and the `claudine` skill: state that captured output stays raw in memory,
  while agent-written frontmatter is stored as a literal token and decoded
  for consumers; update the surviving-span section to R1.4 and the inline
  guardrails to R4's plain-scalar rule.

## Decisions

- **Command-line setters stay templates.** A person typed them, so there is
  no data-origin claim. They continue to be scanned as authored input,
  including whole-value expressions and shell forms, under the existing
  approval rules. This preserves the current command-line contract.
- **Runtime values stay raw and typed.** `outputs` and `_loop_last_output`
  are consumed by expressions and result reporting, so encoding them at
  capture would expose transport syntax and change their meaning. Origin is
  preserved where they enter composition. This is the reader's note for the
  change from the draft's universal escape-at-entry approach.
- **Persisted agent values use a literal token.** This makes later reads
  safe without a sidecar provenance file. The on-disk form is less readable;
  authoring tools should show decoded text, and docs must explain how to
  edit a value without accidentally turning it into a template.
- **Backslash and triple-brace escaping do not serve as the persistence
  format.** Backslashes remain in composed text, and triple braces cannot
  encode every possible string. Existing author-facing escapes keep their
  current syntax.
- **The change to fixed-point interpolation is intentional.** It prevents
  inserted text from becoming a new instruction. Authored templates that
  relied on a later rescan require a source-level rewrite; the migration
  inventory and tests are part of implementation, not optional cleanup.

## Acceptance Tests

- **Darkmatter L1 (R1):** each relevant pass preserves inserted `{{ area }}`,
  `{{…}}`, `{{{ area }}}`, and whole-value `$(echo X)` as exact text: no
  evaluation, parse error, or shell approval. The `esc.md` reproduction
  prints `Body: fixed {{ area }}`. Authored adjacent spans still evaluate.
- **Darkmatter L1 (R1.2 and R1.4):** an authored shell candidate is found by
  preflight and handled by the normal approval and execution policy; an
  authored unresolved template retains its existing strict error or lenient
  warning. An authored executable produced entirely by interpolation is
  still rejected. Shell output resembling a command is retained as data.
- **Darkmatter property test (R2):** for arbitrary Unicode strings,
  decode(encode(s)) == s, including strings resembling literal tokens.
  Invalid tokens fail with source location; a decoded token is not scanned
  again and does not become a shell command.
- **Claudine L1 (loop):** iteration 1 outputs `see {{…}} and $(rm -rf x)`.
  Iteration 2 prepares, `_loop_last_output` is raw to predicates and renders
  byte-for-byte in the prompt, and no shell approval is requested.
- **Claudine L1 (sequence):** a task outputs `{{ ctx.repo }}`. The next step
  sees that raw string in `outputs` and through `last(outputs)`, including
  inside a parallel group's nested array entry.
- **Claudine L1 (inline):** a fake agent adds `summary: fixed {{…}} parsing`,
  `note: see issue #42`, `cmd: "$(echo X)"`, and `title: Fix: colons`. The
  run succeeds, the file holds YAML-valid literal tokens, and a second run
  reads back the original strings. An unchanged author key containing
  `{{ area }}` still fills in. The completion schema sees decoded values,
  and `md hash --diff` agrees with the stored hash.
- **Claudine L1 (inline, unrepairable YAML):** a duplicate key or malformed
  nested value names the line and agent origin; the existing rollback path
  preserves the pre-run document. CRLF and block scalar fixtures retain
  their formatting.
- **Claudine L1 (lifecycle):** a `message:` that inserts
  `frontmatter(log, 'message_to_agent')`, where the agent wrote `{{…}}`,
  sends the exact text. A `set:` or `proxy.with:` derived from agent data
  remains inert after the next preparation.
- **Regression:** `--set '{"x":"{{ title }}"}'` still fills in; authored
  lifecycle nested-span guards and strict whole-value checks still fire.

## Workaround Until Fixed

Restart a failed loop at the next phase with `phase=<n>`. The first iteration
after a restart has an empty `_loop_last_output`. The next iteration whose
agent output mentions template syntax will fail again.
