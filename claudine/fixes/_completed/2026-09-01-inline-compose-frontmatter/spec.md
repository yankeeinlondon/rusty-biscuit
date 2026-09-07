---
status: draft
created: 2026-09-01
updated: 2026-09-01
area: claudine
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-09-01
review_iterations: 1
implemented: true
packages:
    - claudine
    - darkmatter
---

# Inline-compose destroys authored frontmatter on write-back, and prompt-requested properties have no sanctioned delivery channel

## Summary

`claudine inline-compose` promises to replace a document's body while
preserving its frontmatter byte-for-byte. Two defects break that promise
today, and a third makes the breakage look like the agent's fault.

1. **The write path re-serializes the frontmatter it just preserved.** The
   closure carefully reconstructs the document from the verbatim original
   frontmatter text, then hands it to the hash-stamping step, which parses it
   into a map and re-emits the whole block through `serde_yaml_ng`. Authored
   formatting is lost on every run. When any line of a multiline `prompt` has
   trailing whitespace — common in hand-authored prompts — the emitter cannot
   use a block scalar and collapses the entire prompt into one double-quoted
   line full of `\n` and `\"` escapes.
2. **Prompt-requested new frontmatter properties fail with obedient agents.**
   The only channel the closure inspects is the post-run on-disk file, but the
   guardrails appended to every inline prompt forbid the agent from editing
   that file and from returning frontmatter — and when the agent returns a
   frontmatter block anyway, `extract_replacement_body` strips it and throws
   it away. The feature only ever worked because earlier models disobeyed the
   guardrails.
3. **The status output misattributes and overstates.** "Agent modified
   frontmatter property X — reverted to original value" fires for any post-run
   difference (including a user's mid-run editor fix, which it then clobbers),
   and the "revert" is value-level only — its output still passes through the
   re-serializer in defect 1, so the visible text never matches the authored
   original.

This fix makes the frontmatter write-back textual end-to-end (a Darkmatter
text-preserving hash-save adopted by both the closure and `md hash --save`),
defines an explicitly authorized response frontmatter block as the sanctioned
channel for generated properties, updates the guardrails to teach that
channel, and makes the closure's rebuild from the pre-run snapshot the
authoritative restore for any mid-run source drift: execution never stops for
a mid-run change, the authored frontmatter is restored verbatim afterward,
and the report states what was restored without guessing who wrote the
drift.

## Established contracts

- `rewrite_inline_document` and its tests
  (`claudine/lib/src/composition/closure/tests.rs:69`,
  `rewrite_adds_last_updated_without_reserializing_frontmatter`) establish
  that inline write-back preserves the original frontmatter text verbatim,
  upserting only `last_updated` and injecting explicitly-handed new
  properties. This contract held in production until `b56c9dbd0`.
- `apply_hash_save`'s module docs (`darkmatter/lib/src/markdown/hash/write.rs`)
  promise byte-exact **body** fidelity. This fix extends the same fidelity to
  the frontmatter block.
- The frontmatter segment of a Simple hash is computed over **parsed values**
  (`hash_frontmatter_map`, `darkmatter/lib/src/markdown/hash/mod.rs:81`:
  sorted keys, JSON-rendered values), so the stored hash is independent of
  YAML text formatting. Stamping the hash textually cannot break the
  `md hash --diff` exit-0 invariant that
  `apply_closure_writes_cleaned_body_and_consistent_hash` asserts.
- The inline closure's merge/revert semantics (2026-04-02, `767926cb3`):
  new keys are merged immediately before `last_updated`; modified keys are
  reported and **not** applied; `prompt` is immutable. This fix intentionally
  retires the unsanctioned on-disk merge channel and adds an explicit exception
  for author-declared generated keys, which may be inserted or refreshed from
  the final response. Undeclared authored keys remain immutable; any mid-run
  on-disk change to them is detected, reported without attribution, and
  overwritten by the closure's rebuild from the pre-run snapshot.
- Inline composition's `--dry-run` performs no mutation; nothing here runs
  under dry-run.

## Observed behavior (verified 2026-08-31)

Evidence gathered from `homelab/docs/unifi/routers.md` and
`homelab/docs/unifi/ap.md` after the 2026-08-30 runs, the claudine hook log
(`~/.claudine/logs/2026-08-30.jsonl`), and a standalone `serde_yaml_ng` 0.10
reproduction:

- The hook log for the entire evening shows **zero** `Write`/`Edit`/`Bash`
  mutations of the target documents by any agent (the only writes were `/tmp`
  scratch scripts). Every frontmatter change was made by claudine's own
  pipeline or by out-of-band fixups between runs.
- `serde_yaml_ng::to_string` of a multiline string whose lines are free of
  trailing whitespace emits a 2-space-indented `|-` block (authored 4-space
  indentation is lost). The same string with one trailing space on any line
  is emitted as a single double-quoted line with `\n`/`\"` escapes. Both
  authored prompts contain trailing-space lines (e.g. `as H2 headings ␣`).
- `routers.md`'s stored hash body segment matches the on-disk body exactly
  while the frontmatter segment does not — and the frontmatter hash is
  computed over parsed values, proving the prompt **value** (not just its
  formatting) changed after the closure's write: a later out-of-band fixup
  converted the escaped one-liner back to a block scalar while keeping the
  `\"` sequences literally, baking backslashes into the value. Each
  run/fixup cycle compounds the damage.
- The run shown in the user's terminal capture printed "Agent modified
  frontmatter property `prompt` — reverted to original value" and no
  "Merged new frontmatter property" lines: the obedient agent had delivered
  the requested `routers`/`agent` properties nowhere the closure looks.

## Root cause

### Defect 1 — hash stamping re-serializes the frontmatter

Since `b56c9dbd0` ("swap inline closure baseline to ComputedHash",
2026-06-23), `apply_inline_closure`
(`claudine/lib/src/composition/closure.rs:126-144`) takes the verbatim
`doc_string` produced by `rewrite_inline_document`, splits it, and rebuilds a
`Markdown` via `with_frontmatter(parsed_map, body)`. `apply_hash_save`
(`darkmatter/lib/src/markdown/hash/write.rs:47`) then clones that document and
serializes it with `as_string()`
(`darkmatter/lib/src/markdown/output/string.rs:24`), which emits the
frontmatter from the parsed map via `serde_yaml_ng`. The verbatim text is
discarded two statements after it was assembled. Before `b56c9dbd0` the
closure wrote `doc_string` directly with `atomic_write`.

The existing tests stayed green because every `apply_inline_closure` test
uses single-line properties (`prompt: test`) that round-trip identically; the
byte-preservation assertions all target `rewrite_inline_document`'s
intermediate string, which is no longer what reaches disk.

`md hash --save` (`darkmatter/cli/src/commands/hash.rs:189`) writes through
the same `apply_hash_save` and has the same latent defect for any document
with formatting serde will not reproduce.

### Defect 2 — no sanctioned channel for prompt-requested properties

`try_inline_closure` (`claudine/cli/src/commands/wrap/inline.rs:131`) sources
post-run frontmatter exclusively from the on-disk file. The guardrails
(`claudine/lib/src/composition/guardrails.rs:16-22`, and the materialized
`.claudine/inline-compose.md`) instruct the agent:

- return the replacement body only,
- do not include frontmatter delimiters or content,
- do not edit the source file directly.

An agent that obeys leaves the file untouched (nothing to diff) and returns
no frontmatter; an agent that half-obeys by putting the requested properties
in a leading frontmatter block loses them to `strip_leading_frontmatter`
(`closure.rs:267`), which discards what it strips. The merge feature
(2026-04-02) therefore only ever fired when the agent violated the
guardrails, and current models (claude-opus-5 in the observed runs) obey.
This is behavior drift exposing a design that depended on disobedience, not a
code regression.

### Defect 3 — dishonest reporting on a correct restore

`inline.rs:167` attributes any pre-run/post-run difference to the agent and
claims a revert "to original value". Two things are wrong with that line, and
neither is the restore itself. First, the attribution is a guess: the closure
cannot distinguish an agent edit from any other writer. Second — the part
that made the restore look like a lie — after defect 1 the "reverted" text on
disk resembles nobody's version: the value was restored but the write path
re-serialized it into the escaped one-liner.

The restore behavior is intentional and stays, and its purpose is authored
readability. The `|-` block scalar is how a human writes and reads a
multi-paragraph prompt; an inline `\n`-escaped string encodes the same value
in a form no one can comfortably read or edit. Agents routinely decide they
"know better" mid-run and rewrite the authored `|-` block into exactly that
inline form (incidentally stripping trailing whitespace, which is what makes
the value-level detector fire). The document's frontmatter is the author's
interface; the closure's job is to hand it back byte-for-byte as authored.
Stopping execution over such a rewrite would fail the majority of long runs
to protect bytes the snapshot already holds — the correct response is to
finish the run and restore the authored bytes afterward.

## Design decisions

### D1 — Frontmatter write-back is textual; the map is for values only

Add a fallible, text-preserving apply to Darkmatter's hash module:

```rust
pub fn apply_hash_save_text(
    document_text: &str,
    decision: &SaveDecision,
    options: &MdHashOptions,
    today: &str,
) -> MarkdownResult<Option<String>>;
```

(The exact namespace is an implementation choice; the semantic requirements
are not.) `SaveDecision::new_stored` already contains the computed value, so a
`&Markdown` receiver must not be added merely to imply an association the
writer cannot verify. `document_text` is the sole byte authority. The fallible
return is required because appending a second semantic key when an existing
key cannot be located safely is worse than refusing the write.

Requirements:

- The managed hash key is `options.property`, not always `hash`. A valid custom
  property selected through `HASH_PROPERTY` receives the same fidelity and
  duplicate-key guarantees.
- Every `StoredHash` representation is supported: shorthand Simple scalars and
  longhand `fm`, `body`, `simple` with ignores, `structured`, and `detailed`
  objects. Claudine forces Simple; `md hash --save` does not.
- An existing top-level managed scalar is replaced in place (position
  preserved). A block-mapped managed value is replaced in place as one YAML
  node: its key line and child lines are removed together, without consuming
  the following top-level property or unrelated comments.
- Plain and quoted YAML keys that parse to `options.property` are recognized.
  If the root uses an unsupported layout (for example, a flow-style mapping),
  the operation returns a typed error and does not write; it must never append
  a duplicate semantic key.
- Multiple source nodes that parse to the managed property are a typed error.
  Zero-indent comments around a managed node are preserved; comments indented
  inside a replaced managed node are part of that node and may be replaced.
- An absent managed key appends at the end of the frontmatter block.
- `decision.bump_last_updated` upserts the `last_updated` line preserving its
  quote style (the logic already proven in claudine's
  `rewrite_last_updated_line`); upserting an already-correct value is a
  no-op. A document without a frontmatter block gains a minimal valid one,
  matching `document_without_frontmatter_gains_a_valid_block`.
- Newline style (LF/CRLF) follows the document; the body is byte-exact; no
  other frontmatter line is touched.
- `decision.new_stored == None` returns `Ok(None)` without parsing or changing
  the text, as today.

Adopt it at both text-authoritative call sites:

- `apply_inline_closure` applies the decision to `doc_string` directly and
  deletes the `with_frontmatter` reconstruction (`closure.rs:126-134`) along
  with its now-moot explanatory comment. It parses `doc_string` only for
  `plan_hash_save` and passes the unchanged `doc_string` to the textual apply.
- `md hash --save` resolves the input through the existing `FileReference`
  path, reads the raw UTF-8 source once, derives the `Markdown` and
  `SaveDecision` from that same string, and passes the string to the textual
  apply. `load_markdown` currently returns only `Markdown`; the implementation
  must add a raw-text-preserving file-input path rather than claiming the CLI
  already has the source bytes. Stdin remains rejected for `--save`.

The map-based `apply_hash_save` remains for callers that legitimately own a
mutated map (the effects verbs); its docs gain a warning that it
re-serializes frontmatter and that text-fidelity callers must use the textual
variant.

Because the frontmatter hash segment is computed over canonical parsed values
(see Established contracts), a textually-stamped document re-parses to the
same hash: the self-consistency and idempotence tests keep passing without
modification.

### D2 — Generated frontmatter is an authored, allowlisted response channel

Add an optional, statically authored property:

```yaml
response_frontmatter:
  - routers
  - generated_by
```

This list is authorization, not merely documentation. Without it, a provider
response cannot add frontmatter. This prevents model output from silently
installing provider selection, lifecycle side effects, a schema, or other
future-run control state just because a natural-language prompt happened to
ask for "metadata."

Authorization is read from the raw source frontmatter and snapshotted during
preparation. Interpolation, command-line setters, schema defaults, provider
output, and mid-run file changes cannot create or expand it. This keeps the
set auditable before the provider launches.

Preparation validates the property before provider launch:

- it is a list of unique, non-empty string keys;
- `prompt`, `response_frontmatter`, `hash`, and `last_updated` are always
  forbidden because they are closure-owned or immutable;
- keys are retained in declaration order. A missing generated key is inserted
  in that order immediately before `last_updated`; an existing generated key
  keeps its position when its complete YAML value node is replaced.

The declaration transfers write ownership for those named properties to the
inline closure. That is necessary for repeat runs: a generated inventory such
as `routers` must be refreshable rather than becoming immutable after its first
insertion. Removing a name from `response_frontmatter` returns the existing
property to ordinary authored/immutable status; it does not delete it.

Claudine-interpreted keys such as `agent`, `model`, `$schema`, and lifecycle
events are not categorically forbidden: the author has explicitly authorized
them by name. The docs and prepare-time status must warn that such a key changes
future execution semantics. In the observed UniFi documents, provenance must
therefore use a metadata name such as `generated_by`; `agent` is already the
provider-selection contract and the requested `claude/claude-opus-5` value is
not a valid selection hint.

`extract_replacement_body` becomes `extract_replacement_parts` (or gains a
sibling), returning the body and an optional parsed leading frontmatter map:

- A response beginning with an exact `---` delimiter and containing a closing
  delimiter is treated as a response-frontmatter attempt. The YAML must parse
  as a string-keyed mapping. Malformed YAML, duplicate keys, a non-map root, or
  a block with no non-empty body is an invalid inline response; the source is
  left untouched. Silently applying only the body would report success while
  discarding an explicitly requested artifact.
- Allowed keys are harvested and may insert or replace their complete top-level
  YAML nodes. Undeclared keys, including proposals for existing authored keys,
  are ignored with source-accurate warnings. Harvested `hash` and
  `last_updated` are ignored silently because the closure owns them, even
  though preparation prevents declaring them.
- Missing allowed keys do not invalidate an otherwise valid body, but are
  reported as not returned. The allowlist grants permission; it does not make
  every property required.
- An unclosed leading `---` is ordinary body text, preserving today's
  conservative delimiter recognition.

Each harvested property is serialized as a one-entry YAML mapping, not by
concatenating the parsed key with `format!("{key}: ...")`. The serializer must
quote keys when required and support scalar, multiline, sequence, and mapping
values. Replacing an existing generated property replaces its whole YAML node,
including indented children, without consuming adjacent comments or the next
top-level key. Only declared generated fragments may be normalized; all other
authored frontmatter remains textual. This closes the invalid-YAML/injection
gap for keys containing YAML-significant characters.

Generated property lines flow into `rewrite_inline_document`'s verbatim
document, and D1 preserves them through hash stamping. An unchanged replacement
body remains an error under the established inline-hash contract; returned
properties do not turn a body-no-op into a successful mutation.

### D3 — Guardrails teach the allowlist; materialized defaults migrate

New `DEFAULT_GUARDRAILS`:

```markdown
> **IMPORTANT:**
>
> - Return the replacement Markdown body content only
> - Do not edit the source file directly
> - If an "Allowed response frontmatter properties" list appears below, you
>   may put exactly those properties in a YAML frontmatter block (`---`
>   fenced) at the very top of your response; they will be merged into the
>   document
> - Frontmatter properties outside that allowed list — including `prompt` —
>   cannot be changed; do not include them in that block
```

When `response_frontmatter` is non-empty, preparation appends an owned
instruction listing the exact allowed names after the loaded guardrails. This
dynamic clause is part of the closure protocol, including when the repository
has customized guardrails; repository text may add constraints but cannot
expand the authored allowlist.

`load_or_create_guardrails` migrates a materialized guardrails file **only**
when its bytes equal a known shipped default; any other content is user
customization and is left untouched. Known shipped defaults:

1. the 2026-03-17 text ("Never change the `prompt` frontmatter property…",
   `33cab0ef1`), and
2. the 2026-03-27 → current text ("Return the replacement Markdown body
   content only…", `1ca43e54e`).

This repairs repos (including this one) whose `.claudine/inline-compose.md`
was materialized from the old default and would otherwise keep suppressing
the new channel forever. Migration uses Claudine's atomic writer. If the
migration write fails, the current run uses the new built-in text in memory,
emits a warning, and retries persistence on a later run; user-customized bytes
remain untouched.

### D4 — Mid-run source drift never stops the run; the snapshot restore is authoritative

During an inline run the document belongs to the closure. The rebuild from
`InlineClosurePlan::original_document_text` (verbatim under D1, plus the
replacement body and any D2-authorized generated keys) is the authoritative
final state, and the single atomic write always proceeds:

- Inside `apply_inline_closure`, at the latest practical point before
  `atomic_write`, read the source again and compare it with the pre-run
  snapshot. Any drift — body or frontmatter — is **detection input only**: it
  is reported, never merged, and never a reason to refuse the write. A failed
  re-read degrades to writing without a drift report, matching today's
  best-effort post-run read.
- The restore is byte-level and unconditional: the authored formatting — a
  `|-` block scalar chosen because it is the readable way for a human to
  author and maintain a multi-paragraph prompt — comes back from the
  snapshot whether the mid-run rewrite changed the parsed value or only its
  rendition. The parsed-value comparison is the *detector's* granularity,
  not the restore's: value-drifted properties are reported per property as
  `Frontmatter property "X" changed on disk during the run — restored the
  authored value`, while a value-preserving reformat is restored silently.
  No writer is named: the closure cannot know whether it was the provider,
  an editor, a formatter, or another process.
- Body drift gets one non-attributing informational line; the replacement
  body overwrites it by design.
- No typed conflict error exists on this path. The last-rename-wins
  `atomic_write` semantics are unchanged; portable locking is not introduced.

> **Reader note — restore-over versus refuse:** the initial draft restored
> and the first review changed this to a typed
> `InlineSourceChangedDuringRun` refusal on any byte drift. Ken overruled the
> refusal on 2026-09-01: agents change the source mid-stride routinely — in
> the overwhelmingly common case merely reformatting the `prompt` property
> (dropping the authored `|-` block scalar for an inline `\n`-escaped
> string) — so refusing would discard most long research runs to protect
> bytes the snapshot already holds. A human edit made during a run loses to
> the restore by design; the documented contract is to edit between runs.
> What made the old behavior unacceptable was never the restore — it was the
> false agent attribution and the defect-1 re-serialization that made the
> restored prompt unrecognizable.

"Preserved original frontmatter and updated `last_updated`" becomes true again
under D1 and keeps its wording. Response warnings name the response as their
source; drift reports name only the observed timing.

## Scope

- `darkmatter/lib/src/markdown/hash/write.rs` — `apply_hash_save_text` (or
  equivalently-shaped fallible textual apply) plus unit tests; docs warning on
  the map-based apply.
- `darkmatter/cli/src/io/mod.rs` and `darkmatter/cli/src/commands/hash.rs` —
  preserve raw file text for `--save` and adopt the textual apply without
  bypassing `FileReference` resolution.
- `claudine/lib/src/composition/closure.rs` — textual stamping in
  `apply_inline_closure`; `extract_replacement_parts`; allowlisted harvest;
  YAML-safe top-level-node insertion/replacement; adjacent pre-write drift
  detection feeding the restore report; tests.
- `claudine/lib/src/composition/types.rs` and prepare/schema-validation code —
  carry and validate `response_frontmatter` in declaration order.
- `claudine/lib/src/composition/guardrails.rs` — new default text; known-
  default atomic migration; tests.
- `claudine/cli/src/commands/wrap/inline.rs` — harvested-frontmatter wiring and
  source-accurate status lines; test updates.
- Drift maintenance: update the inline-compose sections of
  `claudine/docs`/skill composition docs and the portable
  `.claude/skills/claudine/composition.md` snapshot, plus
  `claudine/docs/topics/frontmatter-properties.md` (guardrail text,
  `response_frontmatter`, the conflict rule, and the textual-write contract).

## Acceptance criteria

- **AC1 (byte preservation).** An `apply_inline_closure` test whose original
  document carries a `prompt: |-` block with 4-space indentation, a
  trailing-space line, an interior blank line, and literal `\"` sequences
  asserts the written frontmatter is byte-identical to the original except
  the `last_updated` value and the `hash` line. This is the regression test
  the June change lacked.
- **AC2 (no escaped one-liner).** The AC1 document's written form still
  contains `prompt: |-` and does not contain `prompt: "`.
- **AC3 (hash self-consistency).** For the AC1 document, re-parsing the
  written file and running `compare_hash` against the stored hash reports
  neither frontmatter nor body changed (the `md hash --diff` exit-0
  equivalence), and a second identical run is byte-idempotent.
- **AC4 (structured downgrade, textual).** A block-mapped structured `hash:`
  is replaced in place by the flat Simple form with every other frontmatter
  line byte-identical.
- **AC5 (generic textual hash save).** `md hash --save` on LF and CRLF
  documents with a trailing-space multiline property preserves all bytes
  outside the managed hash/`last_updated` nodes for: default Simple,
  longhand structured/detailed, and a custom `HASH_PROPERTY`. Quoted managed
  keys are replaced; an unsupported flow-style root fails without writing or
  creating a duplicate key.
- **AC6 (authorized response harvest).** With
  `response_frontmatter: [routers, generated_by]`, provider output consisting
  of those keys in a leading frontmatter block plus a body yields: body
  applied, missing keys merged in declaration order immediately before
  `last_updated`, existing generated keys refreshed in place, and each outcome
  reported accurately. A second run can refresh both values without changing
  the declaration.
- **AC7 (authority and immutability).** Undeclared response keys and proposals
  for existing undeclared properties are not applied and produce source-
  accurate warnings. Authored `prompt` survives byte-for-byte; harvested
  `hash`/`last_updated` are ignored silently. Preparing a declaration that
  includes any closure-owned/immutable key fails before launch. Removing a
  generated key from the declaration leaves its current value untouched on
  later runs.
- **AC8 (invalid harvest is non-mutating).** A well-delimited response block
  with invalid YAML, duplicate keys, or a non-map root fails the closure and
  leaves the source byte-identical. A valid response block containing a YAML-
  significant property name proves the inserted one-entry fragment is valid
  YAML and re-parses to the same key/value.
- **AC9 (guardrail migration).** A materialized guardrails file byte-equal to
  either shipped default is rewritten to the new default on load; a
  customized file is returned unchanged and not rewritten. A simulated
  migration-write failure returns the new protocol for the current run and
  warns without truncating the old file.
- **AC10 (end-to-end).** An inline-compose run against an ap.md-shaped
  document declaring `response_frontmatter: [access_points, generated_by]`
  with a provider stub that obeys the new guardrails ends with both generated
  properties present and the `prompt` property byte-identical to its authored
  form. `last_updated` and `hash` are closure-managed and need not be returned.
- **AC11 (mid-run drift restored, run completes).** With a mid-run rewrite of
  the source — the canonical case: the `prompt: |-` block replaced by an
  inline `\n`-escaped string with trailing whitespace stripped — the closure
  still applies the provider response, and the written document's frontmatter
  is byte-identical to the pre-run snapshot outside the managed lines and
  declared generated keys. Each value-drifted property is reported as
  "changed on disk during the run — restored the authored value"; a
  value-preserving rewrite is restored with no per-property report. No status
  line attributes the change to the agent, and no error is returned.
- **AC12 (unchanged body).** A response with valid allowed properties but a
  semantically unchanged body remains an unchanged-body error and writes
  nothing.

Verification runs through the affected package-area recipes (`just test` and
`just lint`, plus `just ci-local` before push) per the monorepo testing
conventions. AC1–AC4, AC6–AC9, AC11, and AC12 are L1; AC5 lives in
Darkmatter's L1 CLI tests; AC10 uses the existing provider-stub harness tier.

## Non-goals

- **Effects-verb write fidelity.** `effects/verbs.rs` writers exist to mutate
  frontmatter maps; making them format-preserving for untouched keys is a
  separate improvement with its own design (it would need original-text
  plumbing through the effect engine).
- **Repairing already-mangled documents.** The `\"` sequences baked into
  `homelab/docs/unifi/{routers,ap}.md` prompt values are data corruption from
  past cycles; restoring them is a one-time manual edit, not code.
- **Detecting who edited the file mid-run.** D4 detects and restores byte
  drift but does not identify the writer. Writer attribution and automatic
  three-way merge are out of scope.
- **Protecting mid-run human edits.** During an inline run the closure's
  snapshot is authoritative; an edit made to the source while the provider
  runs is restored over, with a report. Preserving such edits (locking,
  conflict refusal, or merge) is explicitly not a goal — edit between runs.
- **Replacing or configuring the YAML emitter.** No serde_yaml_ng changes;
  the fix removes frontmatter from its blast radius instead.

## Open questions

None. The two draft questions are resolved by D2 and D4: generated keys require
explicit authored authorization (with warnings for Claudine-interpreted names),
and mid-run source drift never stops the run — the closure restores the
authored bytes from its snapshot and reports the drift without attribution
(Ken's ruling, 2026-09-01, overruling the review's refuse-on-conflict).
