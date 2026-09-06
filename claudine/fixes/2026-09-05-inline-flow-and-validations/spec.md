---
status: draft
created: 2026-09-05
updated: 2026-09-05
area: claudine
packages:
    - claudine
    - darkmatter
    - dmls
supersedes:
    - 2026-09-05-inline-compose-frontmatter-no-allowlist (response-block channel only)
    - _completed/2026-09-01-inline-compose-frontmatter (D2, D3, and the drift half of D4)
---

# Inline-compose is file-aware again, and both compose pipelines validate their schema at completion

## Summary

`claudine inline-compose` has drifted, over four uncommunicated steps, from
"the agent updates this document" to "the agent answers a question and
Claudine transcribes the answer into the document". The transcription model
depends on reconstructing a "final response" from each provider's transcript
format, keeps the agent ignorant of the file it is working on, and has no
notion of validating the result. The 2026-09-05 voip.md run shows every
consequence at once: seven steps of process narration landed in the body, the
agent guessed the wrong target path, and the frontmatter the prompt asked for
never arrived while the CLI reported clean success.

This spec restores the file-aware flow and adds the validation layer that
makes success mean something:

1. The agent is told the absolute path of the document, told which three
   properties it must not touch, and asked to write the research into that
   file. Its final response is a short summary shown to the CLI caller, not
   document content.
2. Both `compose` and `inline-compose` reach a **completion verdict** through
   one shared path: the body must have changed meaningfully (inline only) and
   the document must satisfy its `$schema` at the end of the run. Schema
   validation runs last, after every actor that can shape the frontmatter.
3. SimplifiedSchema gains the phase semantics the author already relies on:
   `eager` is a universal constraint meaning "present and valid before the
   CLI starts", `required` means "present and valid by the time the run
   completes". Darkmatter currently accepts `eager` only on `file`, which is
   the false DMLS error in the screenshot.
4. DMLS anchors schema-definition errors at the offending property and
   reports every property's error, not only the first.

The Darkmatter and Claudine boundary is explicit: schema phases, the
completion validator, and the frontmatter text primitives live in Darkmatter;
Claudine owns the prompt wording, provider launch, lifecycle ordering, and CLI
reporting.

## History of the flow (why this is a restoration)

| Commit | Date | Inline contract |
|---|---|---|
| `33cab0ef1` | 2026-03-17 | Guardrails: "use the prompt from the `prompt` property to update the body of this document"; post-execution validation of the file. The agent edited the file. |
| `1ca43e54e` | 2026-03-27 | Guardrails: "return the replacement Markdown body content only; do not edit the source file". Claudine captures the final response and writes it. Not called out as a design change. |
| `767926cb3` | 2026-04-02 | On-disk merge of new frontmatter keys after the run, which only ever fired when an agent disobeyed the 03-27 guardrails. |
| `4d24f5b6b` | 2026-09-01 | `response_frontmatter` allowlist; guardrails tell obedient agents to return nothing unless the list appears. |
| uncommitted | 2026-09-05 | Allowlist removed; every returned key in a leading response block is applied. |

Ken's ruling, 2026-09-05: the 03-17 shape is the intended one, and the
2026-09-01 allowlist was never wanted. The uncommitted 09-05 work keeps its
guardrail-migration mechanism and the `CLOSURE_OWNED_PROPERTIES` constant;
its response-block harvest channel is retired by this spec.

## Observed behavior (2026-09-05, voip.md, OpenCode + `zai-coding-plan/glm-5.3`)

- The written body begins with seven sentences of narration joined without
  spaces ("…US store listings.The category page rendered…"). The claudine
  log for the run holds seven raw `text` parts and zero `reasoning` parts:
  this is per-step assistant text, not thinking tokens.
- OpenCode emits one completed `tool_use` event per call and no
  `tool_start`. Claudine maps completed `tool_use` to `SemanticEvent::ToolResult`
  only; the final-response accumulator is reset solely by
  `SemanticEvent::ToolCall`. Forty-six tool calls went by without a reset, so
  every text part accumulated into the "final response".
- The narration's first sentence is "The target document
  `.claude/skills/unifi/products/voip.md` is empty": never told the path, the
  agent guessed the skill copy and read the wrong file.
- The prompt asked for `researched_by` and `products`. The agent's last
  sentence: "no 'Allowed response frontmatter properties' list appeared, so
  I'm returning the Markdown body only." The CLI printed three green checks.
- With `$schema` declaring `prompt: string(required;eager)`, DMLS underlines
  all four schema lines with "constraint `eager` is not valid on `string`",
  and hovering any of the four properties shows that one message.

## Root causes

### RC1 — the "final response" is a per-provider transcript artifact

`claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs` holds one
accumulator for every provider: it resets on `SemanticEvent::ToolCall` and
appends on `SemanticEvent::OutputText`. The contract is provider-neutral;
what differs per provider is the stream adapter in
`claudine/lib/src/stream/providers/` that translates raw JSON lines into
those semantic events. The document's body is therefore only as correct as
each adapter's model of its provider's current output format, and nothing
checks that every adapter honors the accumulator's contract.

OpenCode is the case that failed, for two reasons that compound:

- OpenCode is the only provider that reports a tool **after** it completes:
  one `tool_use` line per call, no separate start event. The six other
  adapters emit `ToolCall` from their start events.
- The OpenCode adapter's own module doc states the compensating rule:
  "`tool_use` → paired `ToolCall` + `ToolResult`". The handler that runs for
  it, `handle_tool_use_completed`, emits only the `ToolResult` today. The
  paired `ToolCall` was lost in a later edit, so the reset never fires.

### RC2 — the agent is blind to the document

`prepare_inline` composes the `prompt` value into a temporary document and
sends only that text plus guardrails. The agent never learns the target path,
cannot read the existing body for a refresh run, cannot check sibling
documents for house style except by guessing, and cannot verify its own
output.

### RC3 — `eager` is a `file`-only constraint in Darkmatter

`darkmatter/lib/src/markdown/schemas/simplified/grammar.rs` parses `eager`
for every type (`Constraint::Eager`). `convert.rs` consumes it only in
`file_fragment`; every other type goes through `reject_unsupported`, which
accepts `required`, `default`, `generated`, `example` and rejects the rest
with `SchemaError::Convert { property, "constraint `eager` is not valid on
`string`" }`. The documented "Universal Constraints" list in
`darkmatter/docs/topics/schema-definition.md` omits `eager`. DMLS is
reporting Darkmatter faithfully.

Conversion also aborts at the first failing property, so a schema with two
bad definitions reports one.

### RC4 — DMLS anchors `Convert` errors at the whole `$schema` block

`schema_prepare_diagnostic` in `darkmatter/dmls/src/diagnostics/frontmatter.rs`
anchors `SchemaError::Grammar { property }` at
`entry_by_key_path(["$schema", property])` but routes `SchemaError::Convert`
through the `other =>` arm, whose span is the entire `$schema` value. One
diagnostic covers four lines; hover on any line shows it.

### RC5 — no completion validation, and `required` is enforced at the wrong time

Schema validation runs only during preparation (`prepare_inline_with_schema`,
`prepare_direct_with_schema_and_prompt`, and the post-shell re-validation in
`claudine/lib/src/composition/schema/mod.rs`). There, a missing `required`
property is an error or an interactive prompt. Nothing re-validates after the
agent, the closure, or lifecycle effects have written the document, so a
property the run was supposed to produce is never checked, and a property the
run was supposed to produce cannot be declared `required` without failing
before launch. The author's existing prompts already encode the intended
split: `plan: file(required; match(**/*plan*.md)) -> The plan file this prompt
will create` (an output) versus `spec: file(required; eager; match(…))` (an
input).

## Design decisions

### D1 — Schema phases (Darkmatter)

`eager` becomes a universal constraint with one meaning across every type:

| Constraint | At launch (before any actor runs) | At completion (after the last actor) |
|---|---|---|
| `eager` | present and valid | present and valid |
| `required` | may be absent; if present, valid | present and valid |
| `generated` | as today: the host supplies it at compose time; never authored | present and valid if also `required` |
| none | if present, valid | if present, valid |

The table is the Darkmatter validator's view. The "may be absent" cell is
what `SchemaPhase::Launch` tolerates; whether a run may actually start with
the property absent is Claudine's mode rule in D5: `compose` fills the gap at
launch through interactive collection or refuses to start, while
`inline-compose` starts and expects the agent (or the closure) to fill it.

`eager` without `required` is accepted and equivalent to `required; eager`
for validation purposes; the distinction is documentation only. `eager` on a
`file` keeps its existing extra meaning (the reference must resolve to an
existing file) because that is what "valid before launch" means for a file.

Implementation:

- `convert.rs` accepts `Constraint::Eager` on every type. The compiled JSON
  Schema does not encode phases; the `required` array keeps meaning
  "required at completion", and the `SchemaShape` / `PropertyAtom` surface
  (which already carries `constraints`) is the source of truth for which
  properties are eager.
- A phase-aware validation entry point:

  ```rust
  pub enum SchemaPhase { Launch, Completion }
  pub fn validate_for_phase(schema, instance, phase, ...) -> ValidationReport
  ```

  `Launch` demotes every non-eager `required` entry to optional before
  validating; `Completion` validates the compiled schema as is. The existing
  `validate` / `validate_with_positions` keep their contract and are
  equivalent to `Completion`.
- Conversion collects every per-property `SchemaError::Convert` into one
  error carrying a list, so consumers can report all definitions at once.
- Docs: the Universal Constraints section documents `eager`, the phase table
  above, and the `generated` interplay. The per-type constraint tables gain
  `eager`.

### D2 — DMLS reports definition errors per property (DMLS)

- `schema_prepare_diagnostic` anchors `SchemaError::Convert { property }` the
  same way it anchors `Grammar { property }`: at the property's own value span
  under `$schema`, falling back to the block only for `<root>`.
- With D1's multi-error conversion, one diagnostic is emitted per failing
  property; a valid property next to an invalid one carries no diagnostic and
  hover on it shows its type documentation, not a neighbor's error.
- The completion and hover providers' constraint catalog lists `eager` for
  every type with the D1 wording.

### D3 — The agent works on the file (Claudine prompt protocol)

The inline prompt delivered to the agent gains a header and new guardrails.
The header states the absolute path of the document and, when a `$schema`
is present, the schema's property table (name, type expression, whether it is
already satisfied). The guardrails replace the response-block instructions:

```markdown
> **IMPORTANT:**
>
> - The document you are updating is `{absolute path}`. Write the requested
>   content into its body and any requested properties into its frontmatter,
>   then re-read the file to confirm it is well-formed.
> - Never modify the `prompt`, `hash`, or `last_updated` frontmatter
>   properties. They are owned by the caller and will be restored if changed.
> - If the document declares a `$schema`, every property you set must match
>   its declared type.
> - Your final response is a summary of what you did (two or three short
>   paragraphs), not the document content. Do not repeat the body in your
>   response.
```

- The materialized `.claudine/inline-compose.md` migrates when byte-equal to
  any shipped default (03-17, 03-27, 09-01, or the uncommitted 09-05 text);
  customized files are left alone, as today.
- Every provider is launched with file-write approval in non-interactive
  mode. The plan verifies this per provider against the existing
  `non-interactive.md` and flag tables; a provider that cannot write is a
  launch-time error naming the provider, not a silent no-op.
- The agent's final response is displayed to the CLI caller as the run
  summary and stored as the run output for lifecycle `{{ last(outputs) }}`.
  It is never written to the document.

### D4 — The closure is a guard, not a transcriber (Darkmatter primitive, Claudine caller)

After a successful provider exit, the closure:

1. Reads the document from disk. This is the agent's deliverable.
2. Restores the three owned nodes (`prompt`, `hash`, `last_updated`) from the
   pre-run snapshot, textually, without touching any other frontmatter byte
   or the body. If the agent changed any of them, the CLI prints one warning
   per property naming it; this is never an error.
3. Stamps `hash` and `last_updated` and runs the body cleanup pass.
4. Writes once, atomically.

The restore-and-stamp step is a Darkmatter function on document text, next
to `apply_hash_save_text`:

```rust
pub fn restore_properties_text(
    current: &str,
    snapshot: &str,
    properties: &[&str],
) -> MarkdownResult<RestoredDocument>   // text + list of properties that differed
```

Claudine's `closure.rs` top-level YAML node editor (`top_level_nodes`,
`semantic_top_level_key`, `rewrite_harvested_frontmatter`) is deleted in
favor of the Darkmatter editor in `hash/write.rs`, closing the duplication
noted on 2026-09-02.

There is no response-capture fallback. If the on-disk body is unchanged the
run fails with "the agent did not update `{path}`"; the summary the agent
returned is still shown so the caller can see why.

Mid-run drift semantics from the 2026-09-01 fix invert: on-disk changes are
the deliverable. The snapshot is used only for the three owned nodes and for
restoring the whole document when the provider exits non-zero or is
interrupted, so a half-written document never persists.

### D5 — Completion verdict, shared by `compose` and `inline-compose` (Claudine, using Darkmatter)

A run's terminal signal is decided by a single `completion_verdict` computed
after the last producing actor and before `success` / `failure` fire:

| Check | `inline-compose` | `compose` |
|---|---|---|
| Body changed meaningfully and is non-empty (Darkmatter `Simple` body hash, whitespace-insensitive) | required | not applicable |
| Owned-property restore warnings | reported | not applicable |
| `$schema` at `SchemaPhase::Completion` against the document as it now exists on disk | required | required |

Ordering, both pipelines: `initialize` effects → launch validation
(`SchemaPhase::Launch`) → provider run → closure write (inline) →
**completion verdict** → `success` or `failure` → `finalize`. Schema
validation is the last check because every actor that may legitimately set a
property (initialize effects, the agent, the closure) has run by then.
Effects on `success` and `finalize` are consumers of the verdict and cannot
repair it; the docs say so.

A failed verdict:

- fires `failure` with a typed `err` (`composition.completion.body_unchanged`
  or `composition.completion.schema`, extending the existing diagnostics
  registry),
- prints a per-property status block using the same renderer as the launch
  report (`build_schema_status_report`), so "required property `products` is
  missing" and "`researched_by` has type number, expected string" look the
  same at both ends of the run,
- exits non-zero. For `compose` this is new: today a document whose
  frontmatter is left invalid by its own lifecycle effects exits 0.

**How a required, non-eager property is handled at the end of a run.** This
is the half of the `eager` / `required` split that does not exist today, so
it is spelled out step by step. It applies identically to `compose` and
`inline-compose`; the only difference is who was expected to set the value.

1. After the last producing actor (see the ordering above) Claudine
   re-reads the document from disk. For `inline-compose` that is the file the
   closure just wrote (agent body, agent frontmatter, restored owned nodes,
   fresh stamp). For `compose` it is the source document as the run's
   `initialize` effects and the agent may have left it; `compose` itself
   never writes the body.
2. The on-disk frontmatter is parsed and validated with
   `validate_for_phase(schema, frontmatter, SchemaPhase::Completion)`. This
   is the compiled schema as authored: every `required` entry is enforced,
   whether or not it is also `eager`, and every present value is
   type-checked. Nothing is demoted, dropped, or coerced at this phase; the
   launch-phase "drop invalid optionals and retry" behavior does not apply
   because there is nothing left to retry.
3. Each problem becomes one line in the status block, in schema declaration
   order, with the same glyphs and wording as the launch report:
   - `products` — required property is missing
   - `researched_by` — expected string, found number
   - `products[2].uk_price` — expected number, found string
   A property that is present and valid prints as satisfied, so the author
   sees the whole schema, not only the failures.
4. If the block contains any problem the verdict is a failure:
   `failure` fires with `err.kind = composition.completion.schema` and
   `err.properties` listing each problem, `finalize` follows as usual, and the
   process exits non-zero. If the block is clean, and (inline only) the body
   changed, `success` fires.
5. **The document is not rolled back on a schema failure.** For
   `inline-compose` the closure's write stands: the research the agent
   produced is kept, stamped, and dated, and the failure report tells the
   author which properties to fix or which prompt line the agent ignored.
   Rolling back would discard minutes of work to protect metadata the author
   can add by hand. Rollback remains reserved for a non-zero provider exit
   or an interrupt (D4). For `compose` there is nothing to roll back.

**Why the two modes differ at launch, and what that means at completion.**
In `inline-compose` the agent works directly on the file, and filling the
frontmatter gaps the prompt describes is part of its job. A property such as
`researched_by` is legitimately empty the first time the document is run and
is required to be set by the time the run ends: `last_updated` by Claudine's
closure, `researched_by` by the agent. So the launch phase lets required,
non-eager properties through untouched, and the completion phase is the
first and only place they are enforced.

In `compose` nothing after launch is expected to fill frontmatter. The
document's own expressions run during composition, and if a required
property is still `null` or undefined afterward, the gap is the caller's to
fill, at launch, through interactive collection. When Interactive Mode is
denied, that is a launch error, as it is today. A required property
therefore never enters a `compose` run unsatisfied. The completion phase for
`compose` re-checks the final state so that a value which a lifecycle effect
or the agent changed to something invalid is still caught, but it will not
find a property that was simply never provided.

Worked example, voip.md as authored today:

| Property | Declaration | Launch (inline) | Completion |
|---|---|---|---|
| `prompt` | `string(required;eager)` | must be a present string or the run does not start | re-checked; always passes because the closure restored the authored bytes |
| `last_updated` | `string(required)` | may be absent; never prompted | present, set by the closure stamp; passes |
| `researched_by` | `string(required)` | may be absent; never prompted | the agent was asked to set it; missing → failure line; wrong type → failure line |
| `products` | `object(required)` | may be absent; never prompted | the agent was asked to set it; missing → failure line; a list where an object was declared → failure line |

The same document run through `compose` would prompt for `researched_by` and
`products` at launch (no expression, no caller value) and would refuse to
launch when Interactive Mode is denied, because in `compose` no later actor
is expected to supply them.

`sequence` steps and `--loop` iterations go through the same call. Whether
the completion phase runs after every step or once after the last one is
OQ2; either way a failing step is a failing step under the existing
`fail_fast` rules.

**Launch-time collection (Ken's ruling, 2026-09-05).** Interactive
collection still exists and still asks for both eager and required
properties, with one mode-dependent rule for `required`:

- A missing **eager** property is asked for in both modes whenever the
  existing Interactive Mode conditions hold; otherwise it is a launch error.
- A missing **required, non-eager** property is judged after the document has
  had its own chance to set it. Authors commonly write
  `foo: {{ spec ? parent_dir(spec) + '/bar.md' : null }}`: the value is
  derived when the input exists and deliberately `null` otherwise. So the
  check runs against the effective frontmatter (after override application
  and interpolation, where `post_shell_validate` already runs), and:
  - for `compose`, when the caller supplied no value **and** the authored
    expression resolved to `null` or undefined, interactive collection kicks
    in and requires a non-null value before launch. A property with no
    authored expression at all is asked for exactly as today, so the shipped
    prompts declaring caller inputs as `number(required)` keep prompting
    without any edit. When Interactive Mode is denied, the missing property
    is a launch error, exactly as today. Either way a `compose` run never
    starts with a required property unsatisfied.
  - for `inline-compose`, the user is **never** asked for a required,
    non-eager property, and its absence is not a launch error. Setting it is
    the agent's job (or the closure's, for `last_updated`), and the
    completion verdict is where its absence is reported.

`generated` keeps its existing exemption from launch collection.

### D6 — One preparation and completion path (Claudine)

`prepare_direct` and `prepare_inline` already share `prepare_document`; the
completion side has no shared seam. The plan adds
`claudine::composition::completion` with:

- `CompletionInputs { mode, resolved_path, snapshot, schema, launch_report }`
- `fn completion_verdict(inputs) -> CompletionVerdict` (pure; no I/O beyond
  reading the document)
- one call site in `loop_control.rs` for both modes, replacing the inline-only
  `try_inline_closure` branch. The inline closure write happens inside the
  same function before the verdict.

Mode-specific code is limited to: whether a closure write happens, and the
prompt header text. Everything else (schema loading, phase validation, status
rendering, error typing, lifecycle wiring) is shared.

### D7 — Ownership

| Concern | Lives in |
|---|---|
| `eager` as universal constraint; `SchemaPhase`; multi-error conversion; docs | Darkmatter lib |
| Per-property anchoring of definition errors; hover/completion catalog | DMLS |
| `restore_properties_text`; body-change detection; cleanup; hash stamping | Darkmatter lib |
| Prompt header and guardrails; guardrail migration; provider write approval | Claudine lib |
| `completion_verdict`; lifecycle ordering; CLI status and error rendering | Claudine lib + CLI |
| Final-response accumulator contract, adapter repair, cross-adapter contract test (D8) | Claudine CLI sink + lib stream adapters |

### D8 — The final-response accumulator resets on any tool activity, for every provider (Claudine)

Under D3 the accumulator only feeds the run summary, but a summary carrying
forty sentences of narration is the same defect in a smaller frame, and RC1
shows the current design lets one adapter silently opt out. Three changes,
none specific to a provider:

1. **Provider-neutral contract.** The accumulator in `event_sink.rs` resets
   on `ToolCall` **or** `ToolResult`. "Anything said before the last tool
   activity is narration" holds whether a provider reports the start of a
   tool, its end, or both, so no adapter can miss the reset by emitting only
   one half of the pair.
2. **Adapter repair.** `handle_tool_use_completed` in the OpenCode adapter
   emits the paired `ToolCall` its module doc promises, ahead of the
   `ToolResult`, so tool-count and tool-name rollups also see the call.
3. **Contract test across adapters.** One test per adapter replays a fixture
   containing text, a tool, and more text, and asserts the accumulated final
   response is the trailing text only. The OpenCode fixture is the saved
   2026-09-05 voip.md transcript (AC11). Adapters that pass today keep the
   test as a regression guard.

## Scope

- `darkmatter/lib/src/markdown/schemas/simplified/convert.rs`,
  `grammar.rs`, `types.rs`, `validate.rs`,
  `darkmatter/lib/src/markdown/compose/schema_validation.rs` — D1.
- `darkmatter/lib/src/markdown/hash/write.rs` — `restore_properties_text`
  (D4).
- `darkmatter/docs/topics/schema-definition.md`,
  `darkmatter/docs/inline/schema-validation.md`, the `darkmatter` skill — D1.
- `darkmatter/dmls/src/diagnostics/frontmatter.rs`,
  `providers/frontmatter.rs`, `providers/hover.rs` — D2.
- `claudine/lib/src/composition/{guardrails.rs, prepare.rs, closure.rs,
  schema/mod.rs, completion.rs (new), lifecycle/…}` — D3–D6.
- `claudine/lib/src/stream/providers/opencode.rs` — D8.
- `claudine/cli/src/commands/wrap/{inline.rs, harness_orch/loop_control.rs,
  live_semantic_sink/event_sink.rs}` and output renderers — D5, D6.
- `claudine/docs/topics/{composition.md, frontmatter-properties.md,
  agents.md}`, `claudine/cli/README.md`, `.claude/skills/claudine/`,
  `.claudine/inline-compose.md` — drift maintenance.

## Acceptance criteria

- **AC1 (eager is universal).** `string(required;eager)`, `number(eager)`,
  `object(required;eager)`, and `datetime(eager)` compile; the DMLS document
  in the screenshot carries zero schema diagnostics.
- **AC2 (per-property anchoring).** A `$schema` with two invalid definitions
  yields two DMLS diagnostics, each ranged at its own property; hover on a
  valid neighbor shows type documentation only.
- **AC3 (launch phase).** `prepare_inline` on voip.md (as authored: `prompt`
  eager, `last_updated`/`researched_by`/`products` required) succeeds with
  `products` and `researched_by` absent. The same document with `prompt`
  absent fails before launch naming `prompt`.
- **AC4 (completion phase).** After a provider stub that writes the body and
  `researched_by` but not `products`, the run fails with a status block naming
  `products` as missing, fires `failure` with
  `composition.completion.schema`, and exits non-zero. With `products`
  written as a string instead of an object, the block names the type
  mismatch. With both written correctly, the run succeeds.
- **AC5 (file-aware flow).** The delivered prompt contains the document's
  absolute path and the new guardrails. The provider stub writes the file
  directly and returns a two-paragraph summary; the summary appears on the
  CLI and does not appear in the document.
- **AC6 (owned-node restore).** A stub that rewrites `prompt` as an escaped
  one-liner and sets `last_updated: never` ends with `prompt` byte-identical
  to the snapshot, a fresh `last_updated`, one warning per touched property,
  and exit code 0.
- **AC7 (unchanged body).** A stub that returns a summary without writing the
  file fails with "did not update"; the document is byte-identical to the
  snapshot.
- **AC8 (interrupted run).** A stub that writes half a document and exits 130
  leaves the document byte-identical to the snapshot.
- **AC9 (compose completion).** A `compose` document whose `success` effect
  cannot repair a missing required property exits non-zero with the same
  status block as AC4; the same document with the property set by
  `initialize` exits 0.
- **AC9a (compose fills gaps at launch, never later).** A `compose` document
  authoring `foo: {{ spec ? parent_dir(spec) + '/bar.md' : null }}` with
  `foo` required, run without `spec`: with Interactive Mode allowed it prompts
  for `foo` and refuses an empty answer; with Interactive Mode denied it
  fails before launch naming `foo`. Run with `spec`, it neither prompts nor
  fails. The same document run through `inline-compose` without `spec`
  launches without prompting and reports `foo` at completion if the agent did
  not set it.
- **AC9b (document kept on schema failure).** In AC4's missing-`products`
  case the written document contains the agent's body, the agent's
  `researched_by`, a fresh `last_updated`, and a consistent `hash`
  (`md hash --diff` exits 0) despite the non-zero exit.
- **AC10 (shared path).** `completion_verdict` has one call site serving both
  modes; a test drives it with inline and direct inputs and asserts the
  identical status rendering.
- **AC11 (accumulator contract).** Every adapter passes the D8 replay test; replaying the 2026-09-05 voip transcript
  through the OpenCode adapter yields a final response equal to the last
  step's text only.
- **AC12 (closure editor consolidation).** `closure.rs` no longer defines a
  YAML node editor; the byte-preservation tests from the 2026-09-01 fix
  (`|-` block, trailing space, four-space indent, CRLF) pass against the
  Darkmatter primitive.
- **AC13 (launch collection).** For `compose`: a required caller input with
  no authored expression prompts when missing, as today; a required property
  authored as a conditional expression prompts only when the caller gave no
  value and the expression resolved to `null`; the same property with a
  resolvable expression does not prompt. For `inline-compose`: a missing
  required, non-eager property never prompts and launch proceeds; a missing
  eager property prompts or fails exactly as `compose` does.

Verification: `just test` and `just lint` in `darkmatter`, `darkmatter/dmls`,
and `claudine`; `just ci-local` before push. AC5–AC8 use the provider-stub
harness tier; AC2 uses the DMLS LSP session tests; AC11 is an adapter replay
fixture from the saved log.

## Non-goals

- Protecting human edits made to the document while the agent runs. The
  agent is now the expected writer; edit between runs.
- Merging a response-block frontmatter channel alongside the file channel.
  One channel.
- Automatic type coercion of agent-written values. A wrong type is a
  completion failure the author sees; the agent is told the types up front.
- Changing `generated` semantics or the `ctx.*` base schema.
- Changing `dmls` behavior for standalone schema files beyond D2's anchoring.

## Open questions

### OQ1 — Which run owns a `required` property when one document is run more than once?

**Resolved by Ken's launch-collection ruling:** eager and required are both
collected at launch; required-but-not-eager is judged after the document's
own expression has run; `compose` asks the user only when that expression
yielded `null` and no caller value exists; `inline-compose` never asks.
Folded into D5 above.

### OQ2 — Completion verdict for a document that runs more than once

**Context.** D5 says `required` means "present and valid when the composition
operation completes", and a failed completion verdict makes the run exit
non-zero. That is unambiguous when one `claudine compose` or
`claudine inline-compose` invocation is one run of one document. Claudine has
three constructs where a single invocation runs the same document several
times, and each run currently reaches its own terminal signal:

- **`sequence`** — the template document is composed once per step with the
  step's state overlaid. All steps share one `$schema`.
- **`compose --loop`** — the document is composed once per iteration until
  the `while` / `until` condition ends the loop.
- **`proxy` / hand-off** — the document hands execution to another target and
  never reaches a verdict itself (unchanged by this spec).

**The question.** A `required` property whose value is produced by step 3 of
a five-step sequence (or by the final loop iteration) is absent at the end of
steps 1 and 2. Under a strict per-run verdict, steps 1 and 2 fail and, with
`fail_fast` on, the sequence never reaches step 3. The alternative reading of
"when the composition operation completes" treats the whole sequence or loop
as the operation and takes the verdict once, after the last step or the
terminal iteration.

**Options.**

1. **Per run, strict.** Every step and iteration must satisfy the schema at
   its own completion. Simple, predictable, and identical to the standalone
   case. Authors who accumulate properties across steps must not mark them
   `required`, or must give them an authored default. This changes behavior
   for any existing sequence that builds up frontmatter across steps.
2. **Per operation.** Intermediate steps and iterations run only the launch
   phase and type checks on properties that are present. The completion
   phase runs once, on the document as it exists after the last step or the
   loop's terminal iteration, and decides the invocation's exit code. This
   matches the literal definition of `required` but delays discovering a
   missing property until the end of a possibly long run.
3. **Per run, with an escape hatch.** Option 1 by default, plus a schema
   constraint (for example `deferred`) that excludes a property from
   intermediate verdicts. More surface area for one pattern.

**Recommendation.** Option 2, because it follows directly from the
definition of `required` this spec adopts and from the sequence model, where
one document is one operation regardless of step count. A sequence step or
iteration still fails immediately on a launch-phase problem, a wrong type on a
property it did set, or a body that did not change in inline mode, so most
defects still surface early. The plan should confirm that the existing
sequence and loop integration tests contain no case that depends on a
mid-sequence `required` failure before adopting it.
