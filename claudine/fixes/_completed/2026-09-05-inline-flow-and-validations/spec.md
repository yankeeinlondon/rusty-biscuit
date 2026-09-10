---
status: draft
created: 2026-09-05
updated: 2026-09-05
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-05
review_iterations: 3
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
   validation is the last check before the run is routed to `success` or
   `failure`; the lifecycle hooks that then run are unrestricted, as today.
3. SimplifiedSchema gains the phase semantics the author already relies on:
   `eager` is a universal constraint controlling when a supplied value is
   validated, while `required` independently controls presence. Darkmatter
   currently accepts `eager` only on `file`, which is the false DMLS error in
   the screenshot.
4. DMLS anchors schema-definition errors at the offending property and
   reports every property's error, not only the first.

The Darkmatter and Claudine boundary is explicit: schema phases, the
completion validator, and the frontmatter text primitives live in Darkmatter;
Claudine owns the prompt wording, provider launch, lifecycle ordering, and CLI
reporting.

## Rulings recorded during review (Ken, 2026-09-05)

These are decisions, not proposals. The design sections below implement
them; if a design section and this list disagree, this list wins.

1. **No declaration of agent-settable properties.** There is no
   `response_frontmatter` allowlist or any equivalent. The prompt author is
   responsible for asking for properties and for checking they were set.
   (Implemented in the uncommitted
   `2026-09-05-inline-compose-frontmatter-no-allowlist` work; carried
   forward here.)
2. **The agent works on the file.** It is told the absolute path, writes
   the body and any requested frontmatter into the file, and returns a
   two-to-three paragraph summary as its final response for the CLI caller.
   This restores the 2026-03-17 approach; the switch to response capture on
   2026-03-27 was never agreed.
3. **`prompt`, `hash`, `last_updated` are Claudine's.** The agent is told not
   to touch them. If it does anyway, Claudine restores or overwrites them and
   warns the caller. This is never an error.
4. **Success is decided at the end, by two checks.** The body must have
   changed meaningfully and be non-empty, judged by the smart body hash that
   whitespace-only edits cannot fool. Darkmatter implements this as the
   non-strict body hash, which ignores leading and trailing whitespace and
   blank lines. The document must satisfy its `$schema` after every actor
   that produces it has run. Schema validation is the last validation. The
   CLI reports each schema problem clearly (missing required property, wrong
   type, failed constraint).
5. **Completion validation applies to `compose` as well as
   `inline-compose`,** through as much shared code as possible.
6. **`eager` versus `required`.** `eager` controls when a supplied value is
   validated; `required` independently controls whether the value must be
   present. A property carrying both must be present and valid at launch.
   `eager` is valid on every type; Darkmatter rejecting it on `string` is a
   regression.
7. **Launch collection.** Interactive collection asks for missing required
   inputs. For a required, non-eager property the check runs
   after the document's own expression has had its chance (the
   `foo: {{ spec ? parent_dir(spec) + '/bar.md' : null }}` pattern). In
   `compose`, a value still null after that triggers interactive collection,
   which requires a non-null answer; when interactive collection is not
   possible the run does not start. In `inline-compose` the user is never
   asked for a required, non-eager property: the agent fills the gaps, and
   the completion check enforces them.
8. **Changes land in Darkmatter first, Claudine only where they must.**
   Schema phases, the completion validator, and frontmatter text primitives
   are Darkmatter's; prompt wording, lifecycle ordering, and CLI reporting
   are Claudine's.

9. **A schema is validated per composition.** A document's `$schema`
   describes that document and that document's lifecycle. A sequence is a
   sequence of separate document compositions, and a loop is the same
   document composed over and over. The completion verdict therefore runs
   at the end of every composition: every sequence step and every loop
   iteration must satisfy the schema when it finishes. (OQ2, ruled
   2026-09-05.)

10. **The completion check routes the flow; it does not police the hooks.**
    Lifecycle events (`initialize`, `start`, `success`, `failure`,
    `finalize`) exist so a document author can respond to a state in the
    execution logic. The agentic run fails when the agent reports failure or
    when the frontmatter is invalid against the schema at the end (no schema
    means everything is valid). That check exists only to direct the run to
    `success` or `failure`. What a hook may do at those states is unchanged:
    a `success` or `finalize` hook may modify the document, including its
    frontmatter, exactly as today. (OQ3, ruled 2026-09-05.)

11. **The inline completion check judges the effective frontmatter with the
    agent's file changes layered on top.** One-off caller inputs (`--set`,
    positional `key=value`, sequence state, `proxy.with`) count toward the
    schema at completion exactly as they do everywhere else in Claudine;
    they are not written to the file. `compose` checks the effective
    frontmatter alone. (OQ4, ruled 2026-09-05; the recommendation to check
    the file on disk was not adopted.)

No open questions remain. OQ1 through OQ4 are recorded below with their
context and options for the record.

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
will create` (an output) versus `spec: file(eager; match(…))` (an optional
input that is validated at launch when supplied).

## Design decisions

### D1 — Schema phases (Darkmatter)

Here, **launch** means the stabilized pre-provider seam: caller overrides,
frontmatter interpolation, schema-aware coercion, and any deferred frontmatter
shell expression have completed, but no provider has started. It does not mean
the instant the `claudine` process was invoked. This definition preserves the
existing compose pipeline while making `eager` a provider-launch gate.

`eager` and `required` are independent axes across every type:

| Constraint | At stabilized launch (before provider start) | At the completion-verdict seam |
|---|---|---|
| `eager` | valid if present | valid if present |
| `required` | may be absent; if present, valid | present and valid |
| `generated` | as today: the host may supply it; absence is not an authoring error | present and valid at runtime completion if also `required` |
| none | if present, valid | if present, valid |

The table is the Darkmatter validator's view. The "may be absent" cell is
what `SchemaPhase::Launch` tolerates; whether a run may actually start with
the property absent is Claudine's mode rule in D5: `compose` fills the gap at
launch through interactive collection or refuses to start, while
`inline-compose` starts and expects the agent (or the closure) to fill it.

`eager` without `required` remains optional. `eager` on a
`file` keeps its existing extra meaning (the reference must resolve to an
existing file) because that is what "valid before launch" means for a file.
Missing and explicit `null` are both absence for phase purposes. Therefore an
optional eager `null` is tolerated, while a `required; eager` null fails at
launch and a required-but-not-eager null fails at completion.

`eager` is a validation-timing constraint, including on nested properties. In
a property union it is hoisted so a supplied value is validated at launch,
while `required` alone controls presence and the selected arm controls value
validation. Array placement does not change presence ownership:
`file(eager)[]` means every present item must resolve and exist, while
`file[](eager)` validates a supplied array at launch. Trigger match
schemas reject `eager` on every type because trigger matching is passive and
has no launch phase; this generalizes the current rejection of eager file
existence checks.

Raw JSON Schema has no Darkmatter phase vocabulary. It retains its current
launch behavior: its `required` entries are enforced before either composition
mode starts, and the same schema is checked again at completion. Authors who
need an agent to produce a required value must use SimplifiedSchema so they can
express the `required`/`eager` distinction.

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

  `Launch` validates the recursively derived eager-property set when present
  and requires only properties carrying both `required` and `eager`.
  `Completion` enforces `required`, including a `generated; required` property
  after its host-supply opportunity. The
  existing unphased `validate` / `validate_with_positions` methods keep their
  static-authoring contract, including the existing generated-property
  exemption; silently redefining them as runtime completion would violate D1's
  promise not to change `generated` semantics.
- Phase projection is derived from the resolved SimplifiedSchema AST and is
  recursive; it does not rewrite the authored schema or mutate the instance.
  Validation remains passive and performs no file resolution, expression
  execution, or I/O.
- Conversion collects every independent per-property conversion failure into
  a structured aggregate whose children retain `property`, `message`, and
  source origin/span when available. Structural root failures that make
  further traversal meaningless remain a
  single error. All `SchemaError` consumers must handle the aggregate without
  flattening its children to prose.
- Docs: the Universal Constraints section documents `eager`, the phase table
  above, raw JSON Schema behavior, union/array placement, and the `generated`
  interplay. The implementation-bound descriptor catalog and every per-type
  constraint table gain `eager`.

### D2 — DMLS reports definition errors per property (DMLS)

- `schema_prepare_diagnostic` anchors `SchemaError::Convert { property }` the
  same way it anchors `Grammar { property }`: at the property's own value span
  under `$schema`, falling back to the block only for `<root>`.
- With D1's aggregate conversion error, DMLS recursively emits one diagnostic
  per child failure; a valid property next to an invalid one carries no
  diagnostic and hover on it shows its type documentation, not a neighbor's
  error. Referenced-schema failures retain their source URI when available;
  when DMLS cannot map that source, the referencing `$schema` span is the
  documented fallback rather than a fabricated local property range.
- Completion and hover consume Darkmatter's typed descriptor catalog rather
  than maintaining another constraint list. They list `eager` for every type
  with the D1 wording.

### D3 — The agent works on the file (Claudine prompt protocol)

The inline prompt delivered to the agent gains a header and new guardrails.
The header states the absolute native path of the active document, verbatim
inside a code span, and, when a `$schema` is present, the resolved schema's
property table (name, type expression, launch status, and completion
requirement). The code span delimits spaces unambiguously and carries
platform-native Windows separators as they are; the path is not JSON-escaped,
because doubled backslashes are a known way to make an agent write to the
wrong path. The guardrails replace the response-block instructions:

```markdown
> **IMPORTANT:**
>
> - The document you are updating is `{absolute native path}`. Write the
>   requested content into its body and any requested properties into its
>   frontmatter, then re-read the file to confirm it is well-formed.
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
- Every provider is launched with the narrowest deterministic non-interactive
  permission posture that can write the resolved document. Existing explicit
  user denies are never silently widened to YOLO/full-host access. When the
  document is outside the provider's writable workspace, Claudine uses a
  provider-supported additional writable root; if the provider has no such
  safe launch shape, launch fails naming the provider, document, and missing
  capability. The implementation records the effective permission/sandbox
  facet so retry, resume, and proxy compatibility checks remain honest. A
  provider matrix test covers macOS, Windows, and Linux argv/environment
  construction without launching real agents.
- The agent's final response is displayed to the CLI caller as the run
  summary and stored as the run output for lifecycle `{{ last(outputs) }}`.
  It is never written to the document.

### D4 — The closure is a guard, not a transcriber (Darkmatter primitive, Claudine caller)

An inline active-document guard captures the full document text when the
document is invoked directly or adopted through `proxy`. It survives
retry/resume for that active document and is discarded on a proxy handoff. On
a non-zero provider exit, interrupt, launch failure after the provider may have
started, or closure parse/edit failure, the guard atomically restores the full
captured text before terminal lifecycle handling. An uncatchable process kill
cannot promise rollback and is documented as the unavoidable limit.
If rollback itself fails, the terminal diagnostic retains the initiating error
and adds a typed rollback cause with the path and I/O error; Claudine must not
claim that the document was restored.

After a successful provider exit, the closure:

1. Reads the document from disk. This is the agent's deliverable.
2. Parses it and cleans the candidate body in memory. Before stamping anything,
   compares the cleaned candidate body with the guard's baseline using
   Darkmatter's non-strict `Simple` body hash and rejects a trimmed-empty or
   unchanged body. Non-strict means leading/trailing whitespace and blank-line
   differences are ignored; internal whitespace and line breaks remain
   significant.
3. Restores the three owned nodes (`prompt`, `hash`, `last_updated`) from the
   pre-run snapshot, textually, without touching any other frontmatter byte
   or the body. If the agent changed any of them, the CLI prints one warning
   per property naming it; this is never an error.
4. Stamps `hash` and `last_updated` against that final candidate.
5. Writes once, atomically.

The restore-and-stamp step is a Darkmatter function on document text, next
to `apply_hash_save_text`:

```rust
pub fn restore_properties_text(
    current: &str,
    snapshot: &str,
    properties: &[&str],
) -> MarkdownResult<RestoredDocument>

pub struct RestoredDocument {
    pub text: String,
    pub restored_properties: Vec<String>,
    pub frontmatter_delta: FrontmatterDelta,
}
```

Claudine's `closure.rs` top-level YAML node editor (`top_level_nodes`,
`semantic_top_level_key`, `rewrite_harvested_frontmatter`) is deleted in
favor of the Darkmatter editor in `hash/write.rs`, closing the duplication
noted on 2026-09-02.

There is no response-capture fallback. If the on-disk body is unchanged the
run fails with "the agent did not update `{path}`"; no closure stamp is written,
the full baseline is restored if necessary, and the summary the agent returned
is still shown so the caller can see why.

Mid-run drift semantics from the 2026-09-01 fix invert: on-disk changes are
the deliverable. The snapshot is used only for the three owned nodes and for
the rollback cases above, so a detected half-written or structurally invalid
document never persists.

If a successful provider attempt produces a valid body but fails completion
schema validation, the written document is intentionally kept (D5). A
`failure` stack may then `retry` or `resume`; those recovery attempts retain
the active document's original body baseline, so a metadata-only repair does
not spuriously fail the already-satisfied body-change requirement. A provider
failure that triggers rollback starts recovery from the restored baseline.

### D5 — Completion verdict, shared by `compose` and `inline-compose` (Claudine, using Darkmatter)

> **Reader note (review correction).** The first draft validated raw on-disk
> frontmatter in both modes. That would make valid caller, sequence, and
> `proxy.with` inputs disappear at completion even though Claudine's established
> contract names effective frontmatter as the downstream source of truth. This
> design instead validates the live effective instance and, for inline mode,
> overlays the agent's semantic on-disk delta plus closure-owned stamps. It
> preserves the intended end-of-run check without rerunning composition or
> requiring transient inputs to be persisted.

A run's terminal signal is decided by a single `completion_verdict` computed
after the last producing actor and before `success` / `failure` fire:

| Check | `inline-compose` | `compose` |
|---|---|---|
| Body changed meaningfully and is non-empty (Darkmatter non-strict `Simple` body hash) | required | not applicable |
| Owned-property restore warnings | reported | not applicable |
| Retained `$schema` at `SchemaPhase::Completion` against the mode's canonical completion instance | live effective frontmatter plus the agent's on-disk delta and closure stamps | live effective frontmatter |

Ordering, both pipelines: `initialize` effects → stabilized launch validation
(`SchemaPhase::Launch`) → `start` effects → provider run → closure candidate
check and write (inline) → **completion verdict** → `success` or `failure` →
`finalize`. The verdict is the final check in the producing slice: caller
inputs, `initialize`, `start`, the provider, and the inline closure have all
had their opportunity. The verdict's only job is to choose between `success`
and `failure`. The hooks that run at those states are the author's response
and are not restricted by this spec (ruling 10): a `success` or `finalize`
hook may still write to the document, and if it does so after the closure
stamp, keeping the stamp coherent is the author's responsibility, as it is
today.

A failed verdict:

- fires `failure` with a typed `err` (`composition.body_unchanged`
  or `composition.completion_schema`, extending the existing two-segment
  diagnostics registry),
- prints a per-property status block using the same renderer as the launch
  report (`build_schema_status_report`), so "required property `products` is
  missing" and "`researched_by` has type number, expected string" look the
  same at both ends of the run,
- enters the ordinary `failure` recovery path. `retry`, `resume`, or `proxy`
  may recover exactly as they do for a provider failure; the process exits
  non-zero only when that path does not recover. For `compose` this is new:
  today a document whose effective frontmatter is left invalid by its own
  producing lifecycle effects exits 0.

The effective schema resolved during the stabilized launch preparation is
retained and reused passively at completion. Completion never resolves a new
schema reference, executes an expression, or performs composition again. If
the agent edits `$schema`, the launch-resolved schema still governs the current
run; the edited declaration governs the next run. This prevents a current run
from weakening its own completion contract without adding `$schema` to the
three closure-owned properties fixed by Ken's ruling.

**How a required, non-eager property is handled at the end of a run.** This
is the half of the `eager` / `required` split that does not exist today, so
it is spelled out step by step. It applies identically to `compose` and
`inline-compose`; the only difference is who was expected to set the value.

1. After the last producing actor, Claudine selects the completion instance.
   Both modes begin with the attempt's live **effective** frontmatter, which
   already includes document values, defaults, caller/sequence/proxy overlays,
   interpolation, coercion, and mirrored active-document lifecycle mutations.
   For `inline-compose`, the closure computes the semantic top-level delta
   between its pre-run authored snapshot and the agent's current on-disk
   frontmatter, applies additions, replacements, and deletions to a copy of the
   live effective map, then applies the restored owned values and fresh stamps.
   This makes agent writes observable without discarding legitimate transient
   inputs. For `compose`, no delta or stamp is applied. Neither mode re-composes
   the source. (Ruling 11.)
2. That instance is validated with the retained effective schema and
   `SchemaPhase::Completion`. Every runtime `required` entry is enforced and
   every present value is type-checked. Nothing is dropped or coerced at this
   phase; completion validation is observational.
3. Each problem becomes one line in the status block, in schema declaration
   order, with the same glyphs and wording as the launch report:
   - `products` — required property is missing
   - `researched_by` — expected string, found number
   - `products[2].uk_price` — expected number, found string
   A property that is present and valid prints as satisfied, so the author
   sees the whole schema, not only the failures.
4. If the block contains any problem the verdict enters failure handling:
   `failure` fires with `err.category = "composition"`,
   `err.code = "composition.completion_schema"`, and
   `err.detail.properties` listing each problem; `finalize` follows as usual, and the
   process exits non-zero unless a normal lifecycle control action recovers or
   proxies the run. If the block is clean, and (inline only) the body changed,
   `success` fires.
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

In `compose` nothing after the producing slice is expected to fill
frontmatter. The document's own expressions run during composition, and if a required
property is still `null` or undefined afterward, the gap is the caller's to
fill, at launch, through interactive collection. When Interactive Mode is
denied, that is a launch error, as it is today. A required property
therefore never enters a `compose` run unsatisfied. The completion phase for
`compose` checks the final live effective state so a value a producing
lifecycle effect changed to something invalid is still caught, while a valid
CLI, sequence, or proxy overlay remains present for validation.

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

`sequence` steps and `--loop` iterations go through the same seam, and each
one is a complete composition with its own verdict (ruling 9). A step or
iteration that ends with a required property missing or a value of the wrong
type is a failing step under the existing `fail_fast` rules. A document that
needs a property to accumulate across steps must not declare it `required`.

**Launch-time collection (Ken's ruling, 2026-09-05).** Interactive
collection still exists and asks for missing required launch inputs, with one
mode-dependent rule for `required`:

- A missing **eager-only** SimplifiedSchema property remains optional and is
  not collected. When supplied, it is validated at launch.
- A missing **required; eager** property is asked for in both modes whenever
  the existing Interactive Mode conditions hold; otherwise it is a launch
  error.
- A missing **required, non-eager** property is judged after the document has
  had its own chance to set it. Authors commonly write
  `foo: {{ spec ? parent_dir(spec) + '/bar.md' : null }}`: the value is
  derived when the input exists and deliberately `null` otherwise. So the
  check runs against effective frontmatter after override application and the
  first interpolation pass, but before any frontmatter shell expression is
  executed, and:
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

Interactive collection remains a top-level-property UI. A missing nested eager
property is a launch error naming its full path; Claudine does not synthesize or
partially edit an enclosing object on the author's behalf.

This requires moving inline's intrinsic `prompt` check behind eager-property
collection and teaching `prepare_inline` to read `prompt` from the effective
input layer rather than only the raw source map. Otherwise a collected or
caller-supplied eager `prompt` could never satisfy the very gate that requested
it. Such a transient prompt remains an invocation input; the closure does not
silently author it into the file. If the schema is raw JSON Schema, current
fail-fast validation remains in force and no phase-aware collection is
attempted.

### D6 — One preparation and completion path (Claudine)

`prepare_direct` and `prepare_inline` already share `prepare_document`; the
completion side has no shared seam. The plan adds
`claudine::composition::completion` with one orchestration entry point and a
pure evaluator:

- `CompletionContext { mode, active_path, retained_schema, live_frontmatter,
  inline_guard, launch_report }`
- `fn complete_active_document(context) -> CompletionOutcome` owns the
  mode-specific read/rollback/atomic-write work.
- `fn evaluate_completion(schema, instance, body_evidence) ->
  CompletionVerdict` is pure and passive.
- one call site in `loop_control.rs` for both modes, replacing the inline-only
  `try_inline_closure` branch. The active-document coordinator supplies the
  current path and resets the inline guard on proxy adoption, not the original
  invocation path.

Mode-specific policy is explicit in `CompletionContext`: inline computes an
authored frontmatter delta, reconciles and persists the artifact, and applies
that delta to the effective completion instance; direct compose selects the
live effective instance and performs no write. Schema validation, status
rendering, error typing, and lifecycle recovery are shared.

### D7 — Ownership

| Concern | Lives in |
|---|---|
| `eager` as universal constraint; `SchemaPhase`; aggregate conversion errors; typed descriptor catalog; docs | Darkmatter lib |
| Per-property anchoring of definition errors; hover/completion catalog | DMLS |
| `restore_properties_text`; semantic frontmatter delta; body-change detection; cleanup; hash stamping | Darkmatter lib |
| Prompt header and guardrails; guardrail migration; provider write approval | Claudine lib |
| Completion orchestration; lifecycle ordering/recovery; CLI status and error rendering | Claudine lib + CLI |
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
  `grammar.rs`, `source.rs`, `types.rs`, `serialize.rs`,
  `darkmatter/lib/src/markdown/schemas/{about.rs,errors.rs,resolve.rs,validate.rs}`,
  `darkmatter/lib/src/markdown/compose/schema_validation.rs` — D1.
- `darkmatter/lib/src/markdown/hash/write.rs` — `restore_properties_text`
  (D4).
- `darkmatter/docs/topics/schema-definition.md`,
  `darkmatter/docs/inline/schema-validation.md`, the `darkmatter` skill — D1.
- `darkmatter/dmls/src/diagnostics/frontmatter.rs`,
  `providers/frontmatter.rs`, `providers/hover.rs` — D2.
- `claudine/lib/src/composition/{guardrails.rs, prepare.rs, closure.rs,
  schema/mod.rs, completion.rs (new), lifecycle/…, types.rs}` — D3–D6.
- `claudine/lib/src/stream/providers/opencode.rs` — D8.
- `claudine/cli/src/commands/compose/{mod.rs,prep.rs}` and
  `claudine/cli/src/commands/wrap/{inline.rs,launch_plan.rs,
  harness_orch/loop_control.rs,harness_orch/session_key.rs,
  live_semantic_sink/event_sink.rs}`, provider launch profiles, and output
  renderers — D3, D5, D6.
- `claudine/docs/topics/{composition.md, frontmatter-properties.md,
  agents.md}`, `claudine/cli/README.md`, `.claude/skills/claudine/`,
  `.claudine/inline-compose.md` — drift maintenance.

## Acceptance criteria

- **AC1 (eager is universal).** `string(required;eager)`, `number(eager)`,
  `object(required;eager)`, and `datetime(eager)` compile; the DMLS document
  in the screenshot carries zero schema diagnostics. Nested, union-hoisted,
  and both array-placement forms follow D1; trigger match schemas reject
  `eager` without performing I/O.
- **AC2 (per-property anchoring).** A `$schema` with two invalid definitions
  yields two DMLS diagnostics, each ranged at its own property; hover on a
  valid neighbor shows type documentation only.
- **AC3 (launch phase).** `prepare_inline` on voip.md (as authored: `prompt`
  required and eager, `last_updated`/`researched_by`/`products` required) succeeds with
  `products` and `researched_by` absent. The same document with `prompt`
  absent prompts when Interactive Mode is allowed and fails before launch
  naming `prompt` otherwise; a collected or CLI-supplied prompt becomes the
  actual delivered prompt.
- **AC4 (completion phase).** After a provider stub that writes the body and
  `researched_by` but not `products`, the run fails with a status block naming
  `products` as missing, fires `failure` with
  `err.code = composition.completion_schema`, and exits non-zero. With `products`
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
- **AC9 (compose completion).** A `compose` document whose `start` effect
  sets a required property to a value of the wrong type enters `failure`
  before `success` can run and, without recovery, exits non-zero with the
  same status block as AC4. The same document with the property left valid
  by `initialize` and untouched by `start` exits 0. This is the only way a
  `compose` run can reach completion with an unsatisfied requirement, since
  launch collection guarantees the property was satisfied when the provider
  started and `compose` applies no on-disk delta.
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
- **AC10 (shared path).** `complete_active_document` has one orchestration call
  site serving both modes; tests drive the pure evaluator with inline and
  direct instances and assert identical status rendering. A three-step
  sequence whose step 2 leaves a required property absent fails at step 2
  under `fail_fast`, and a loop iteration is judged the same way.
- **AC11 (accumulator contract).** Every adapter passes the D8 replay test;
  replaying the 2026-09-05 voip transcript through the OpenCode adapter yields
  a final response equal to the last step's text only.
- **AC12 (closure editor consolidation).** `closure.rs` no longer defines a
  YAML node editor; the byte-preservation tests from the 2026-09-01 fix
  (`|-` block, trailing space, four-space indent, CRLF) pass against the
  Darkmatter primitive. Its semantic delta distinguishes addition,
  replacement, deletion, and value-preserving reformatting while excluding the
  restored owned properties.
- **AC13 (launch collection).** For `compose`: a required caller input with
  no authored expression prompts when missing, as today; a required property
  authored as a conditional expression prompts only when the caller gave no
  value and the expression resolved to `null`; the same property with a
  resolvable expression does not prompt. For `inline-compose`: a missing
  required, non-eager property never prompts and launch proceeds; a missing
  eager property prompts or fails exactly as `compose` does.
- **AC14 (transient effective inputs).** Either mode can satisfy a required
  property through CLI input, sequence state, or `proxy.with` without writing
  that value to the source. Inline agent additions, replacements, and deletions
  are applied over that effective base before completion validation. Removing
  the value from the resulting effective instance fails. The validator performs
  no composition, shell execution, schema resolution, or filesystem I/O.
- **AC15 (raw JSON Schema compatibility).** A raw JSON Schema `required`
  property remains a launch-time requirement in both modes and is rechecked at
  completion; no existing `md schema validate` or unphased validation snapshot
  changes.
- **AC16 (generated compatibility).** Existing unphased validation continues
  to tolerate an absent `generated; required` property. Runtime completion
  validation rejects it after the host-supply opportunity, and accepts it when
  the host supplied a valid value.
- **AC17 (rollback coverage).** Provider exit 1, exit 130, closure parse failure,
  duplicate owned keys, and an unchanged or empty candidate all restore the
  captured document atomically before `failure`; none writes a fresh hash or
  date. An ordinary schema-completion failure keeps the validly written
  artifact as AC9b requires.
- **AC18 (completion recovery).** A completion schema failure can `resume` or
  `retry` through the existing lifecycle path. A metadata-only recovery passes
  the operation-level body-change evidence from the first successful write; a
  provider failure rolls back before retry. `proxy` discards the source guard
  and captures the target's own baseline.
- **AC19 (write capability).** Provider launch-plan tests prove the minimum
  writable posture for every supported provider and all three OS path shapes.
  An explicit deny or a source outside any safely writable root fails before
  provider launch; no fallback selects a bypass/YOLO mode.

Verification: `just test` and `just lint` in the `darkmatter` and `claudine`
package areas (the Darkmatter recipes include DMLS); `just test-l2` in
`claudine` for any provider-stub cases placed in the terminal harness; and
repo-root `just ci-local` before push. AC5–AC8 use the provider-stub harness
tier, AC2 uses the DMLS LSP session tests, and AC11 is an adapter replay fixture
from the saved log. No terminal or browser test may focus a host window.

## Non-goals

- Protecting human edits made to the document while the agent runs. The
  agent is now the expected writer; edit between runs.
- Merging a response-block frontmatter channel alongside the file channel.
  One channel.
- Automatic type coercion of agent-written values. A wrong type is a
  completion failure the author sees; the agent is told the types up front.
- Changing `generated` semantics or the `ctx.*` base schema.
- Changing DMLS behavior for standalone schema files beyond D2's anchoring.

## Open questions

Each question below follows the same shape: what exists today, what the
decision is, three or four ways to settle it with their consequences, the
recommendation, and what changes in this spec once ruled. Repository facts
were checked on 2026-09-05 against `prompts/`, `.claudine/prompts/`, and
`claudine/prompts/`.

### OQ1 — Launch collection for `required` properties (resolved)

Resolved by the review-1 clarification: only missing `required; eager`
properties are collected at launch. Eager-only properties remain optional but
are validated when present. A required, non-eager property is judged after the
document's own expression has run; `compose` asks the user only when that
expression yielded `null` and no caller value exists, and refuses to start
when it cannot ask; `inline-compose` never asks. Folded into D1 and D5.

### OQ2 — When one invocation runs the same document several times, when does `required` get enforced? (resolved)

**Ruling (Ken, 2026-09-05): per composition, option 1.** "Document
composition is always a single document; a sequence is a sequence of
document compositions. The loop feature involves the composition of the same
document over and over. When a document defines a schema it is defining it
for the document and the document's lifecycle, not for a sequence nor for a
number of repeated executions of that document as you get for looping." The
recommendation below (option 2) was not adopted; the options are kept for
the record.

**What exists today.** One `claudine` invocation can run one document more
than once:

- `claudine sequence` composes the template document once per step with the
  step's state overlaid. All steps share the template's one `$schema`. The
  three shipped sequence documents are `prompts/daily.md` (no schema),
  `prompts/_implement/implement-plan.md`, and
  `prompts/_implement/implement-review-findings-plan.md`. In both schema-
  bearing documents every `required` property (`phase`, `total_phases`,
  `plan`, and their review-flow equivalents) is a caller input satisfied at
  launch. No shipped sequence accumulates a required property across steps.
- `claudine compose --loop` composes the document once per iteration until a
  `while` / `until` condition ends it. No shipped prompt uses `loop:`.
- A `proxy` hand-off passes execution to another document, which reaches its
  own verdict; the handing-off document reaches none. Unchanged by this spec.

Each step or iteration currently reaches its own terminal signal, and D5
attaches the completion verdict to that signal. Ruling 6 defines `required`
as "present and valid by the time the composition operation completes", and
does not say whether a step is an operation or the whole sequence is.

**The decision.** For a property that is `required` but not `eager`, and
that is absent at the end of an intermediate step or iteration, is that a
failure of that step, or is it only judged once, at the end of the sequence
or loop?

**Options.**

1. **Per run, strict.** Every step and every iteration must satisfy the
   schema at its own completion. Consequences: the rule is the same as for a
   standalone run and needs no new concept; a document that legitimately
   builds a required property up across steps (step 3 writes `summary`) fails
   at step 1, and with `fail_fast` on it never reaches step 3. No shipped
   document has that shape today, so nothing shipped breaks, but the pattern
   becomes impossible to express with `required`.
2. **Per operation.** Intermediate steps and iterations run the launch phase
   and type-check whatever properties are present. The completion phase runs
   once, on the document as it stands after the last step or the loop's
   terminal iteration, and decides the invocation's exit code. Consequences:
   this matches ruling 6 literally, since a sequence is one composition
   operation regardless of step count; a step still fails immediately on a
   launch problem, on a present value of the wrong type, or on an unchanged
   inline body; a property that nothing ever sets is discovered only at the
   end of a possibly long run.
3. **Per run, with an opt-out constraint.** Option 1 by default, plus a new
   SimplifiedSchema constraint (for example `deferred`) that excludes a
   property from intermediate verdicts. Consequences: early failure for
   everything not marked, explicit authoring for the accumulate-across-steps
   pattern; one more constraint in the grammar, the DMLS catalog, and the
   docs, for a pattern that has no shipped example yet.
4. **Per run, with the sequence deciding.** A `sequence` or `loop` document
   declares once whether its verdict is per step or per operation (for
   example `completion: operation`). Consequences: maximum control; a new
   top-level lifecycle key, two code paths in the verdict seam, and a
   decision every sequence author has to make.

**Recommendation.** Option 2. It is the only option that follows directly
from ruling 6 without adding vocabulary, and the cost it carries (late
discovery of a never-set property) is bounded by the fact that type errors,
launch errors, and unchanged bodies still fail the step that caused them.
Options 3 and 4 add surface for a pattern no document uses today. Option 1
is the simplest to implement but quietly forbids the one multi-step pattern
`required` was defined to allow.

**Once ruled.** D5's sequence paragraph and D6's `CompletionContext` gain an
`operation` versus `run` scope; AC10 gains a sequence case; under option 3
or 4 D1 or the lifecycle key table gains the new declaration.

### OQ3 — May `success` and `finalize` effects change the document after the verdict said it was valid? (resolved)

**Ruling (Ken, 2026-09-05): yes, unrestricted; option 1.** "A lifecycle
event like initialize, success, failure, finalize is there to ALLOW a
document author to respond to a given state in the execution logic. The
agentic run fails when the agent realizes that it failed but also when the
frontmatter state is invalid based on a schema definition (no definition
means everything is valid). That is all done just to direct the process flow
to either the success or failure state. We are not changing what can be done
at each of these states." The recommendation below (option 2) was not
adopted; the options are kept for the record.

**What exists today.** Lifecycle stacks run side-effect verbs supplied by
Darkmatter's effects engine. The verbs that write frontmatter are
`set_frontmatter`, `merge_frontmatter`, `append_frontmatter`,
`prepend_frontmatter`, `delete_frontmatter`, `increment_frontmatter`, and
`decrement_frontmatter`; `append_line`, `append_jsonl`, `ensure_file`, and
`ensure_file_with_content` write file content. Any of them may name any
file, including the document that is running. The `success` and `finalize`
stacks run after the terminal signal is chosen, which under D5 means after
the completion verdict. No shipped prompt currently points one of these
verbs at its own source; the shipped `success` stacks use `say`, `message`,
`success`, and shell actions only. Ken's private prompts were not surveyed.

**The decision.** D5 makes "success" mean "the document is valid and its
hash is coherent at the moment the verdict is taken". A `success` or
`finalize` effect can then delete a required property, change its type, or
rewrite the body so the freshly stamped `hash` no longer matches. Should
Claudine prevent that, detect it, or accept it?

**Options.**

1. **Accept it.** The verdict is a boundary; terminal stacks keep their
   current unrestricted behavior, and the docs say the completion contract
   covers the producing slice only. Consequences: no compatibility break and
   no new effect-engine state; a process can exit 0 with `success` already
   emitted while the active document is invalid or hash-stale, which is the
   exact outcome ruling 4 exists to prevent.
2. **Freeze the active document after a positive verdict.** On the
   successful path, any effect that targets the active document fails with a
   typed lifecycle error; effects that target other files are untouched. On
   the failed path, `failure` may still repair the active document, but only
   when its stack ends in `retry` or `resume`, whose next verdict re-checks
   the repair. Consequences: success keeps its meaning and the lifecycle
   state machine stays as it is; a prompt that mutates its own source in
   `success` or `finalize` must move that work to `initialize` or `start`, a
   deliberate break for a pattern with no shipped user; the effects engine
   needs to know which file is the active document and which phase it is
   in.
3. **Re-audit after `finalize`.** Let every mutation happen, then validate
   the document once more and set the exit code from that second result.
   Consequences: no restriction on lifecycle authors; `success` has already
   fired and its messages have already been sent when the exit code turns
   non-zero, there is no legal second terminal `failure` event to carry the
   error, and `retry` / `resume` are no longer available.
4. **Freeze frontmatter only.** As option 2, but only the frontmatter verbs
   are blocked on the active document; body-writing verbs stay legal and the
   closure re-stamps `hash` after `finalize`. Consequences: schema validity
   is preserved and body edits remain possible; the stamp moves out of the
   single atomic closure write, and a body edited by an effect is never
   validated for the "changed meaningfully" check.

**Recommendation.** Option 2. It is the only option under which both the
`success` signal and the exit code tell the truth about the file, and it
keeps recovery useful. The break it introduces has no shipped victim today;
the plan inventories Ken's private prompts before implementation and lists
any that need their self-mutation moved to `initialize` or `start`. Option 4
is a half-measure that reintroduces a second write outside the atomic
closure, which is how the 2026-06 hash-stamp regression started.

**Once ruled.** Under option 2, D5 gains the freeze rule, Darkmatter's
effects engine gains an active-document phase check (D7 ownership table),
and a new criterion tests a `success` effect that targets the source and is
refused. Under option 1 or 3, D5's ordering paragraph is rewritten to state
the weaker guarantee explicitly.

### OQ4 — For `inline-compose`, is the completion schema judged against the file on disk or the effective frontmatter? (resolved)

**Ruling (Ken, 2026-09-05): effective frontmatter with the agent's on-disk
delta layered on top; option 1.** D5 stands as reviewed. The recommendation
below (option 2, the file on disk) was not adopted; the options are kept for
the record.

**What exists today.** Claudine's established contract is that *effective*
frontmatter is the source of truth for everything downstream of preparation:
document values, schema defaults, `--set` and positional `key=value` setters,
sequence state, and `proxy.with` overlays are merged in memory, and the
body, lifecycle, and schema all see that merged map. Setters are accepted by
`inline-compose` as well as `compose`. Transient inputs are never written
back to the file; `claudine/docs/topics/composition.md` contrasts
`proxy.with` (in-memory, one activation) with `set_frontmatter` (writes the
file) for exactly this reason.

The review changed D5 from my first draft, which validated the raw on-disk
frontmatter in both modes. For `compose` that draft was simply wrong: a
required property satisfied by `--set` would have "disappeared" at
completion. For `inline-compose` the review applies the same fix: the
completion instance is the live effective map with the agent's on-disk
changes overlaid on top, plus the closure stamps.

**The decision.** Ruling 4 says "the document must satisfy its `$schema`".
For `inline-compose`, where the file is the deliverable, does "the document"
mean the file on disk, or the effective frontmatter the run composed with?
The two differ only when a required property is supplied transiently and
the agent does not write it into the file. Under the review's reading that
run succeeds; the next run of the same document, without the setter, sees
the property missing.

**Options.**

1. **Effective frontmatter plus the agent's on-disk delta** (the review's
   choice, as D5 stands). Consequences: consistent with every other
   consumer of effective frontmatter; setters keep working for inline runs;
   a transient value satisfies completion without being persisted, so the
   file can be schema-invalid on its own while the run reports success.
2. **The file on disk alone.** Consequences: the file is guaranteed
   schema-valid after every successful inline run, which is the strongest
   reading of ruling 4; a required property supplied only by a setter fails
   completion unless the agent wrote it, so setters become a way to inform
   the agent, not a way to satisfy the schema; AC14's inline half is
   dropped.
3. **Effective for validation, persist transient required values.** Validate
   as option 1, but when a required property was satisfied only by a
   transient input, the closure writes that value into the file alongside
   the stamps. Consequences: both the run and the file end valid; the
   closure gains a second class of owned writes, and a value the caller
   meant as one-off (`proxy.with` by design) becomes permanent, which
   contradicts the documented `proxy.with` contract.
4. **Effective for validation, warn on transient satisfaction.** Validate as
   option 1 and print one warning per required property that was satisfied
   by a transient input and is absent from the file. Consequences: no
   contract changes; the caller learns the file is not self-sufficient; the
   warning is advisory and the next run still fails at launch or completion.

**Recommendation.** Option 2 for `inline-compose`, with `compose` staying on
effective frontmatter as the review specified. The purpose of inline mode is
to leave a self-sufficient file behind, and ruling 4's wording is about the
document. Setters remain useful on inline runs as inputs the agent can read
(they are in the composed prompt), and an author who wants a transient value
in the file asks the agent to write it. Option 1 is the more uniform rule but
lets an inline run succeed while producing a file that fails its own schema
on the next launch, which is the class of silent success this spec exists to
end. Option 4 is the fallback if uniformity is preferred; option 3 breaks a
documented contract.

**Once ruled.** Under option 2, D5 step 1 and the completion table change
the inline instance to on-disk frontmatter, the D5 "for ratification" note is
deleted, AC14 keeps only its `compose` half, and D6's inline policy drops the
delta overlay for validation (the delta stays for reporting). Under option 1
or 4 the note is deleted and, for option 4, a warning line is added to step 3
and to AC14.
