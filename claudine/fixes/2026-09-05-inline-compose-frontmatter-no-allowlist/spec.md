---
status: draft
created: 2026-09-05
updated: 2026-09-05
area: claudine
implemented: true
packages:
    - claudine
supersedes: _completed/2026-09-01-inline-compose-frontmatter (D2 and D3 only)
---

# Inline-compose applies prompt-requested frontmatter without an allowlist

## Summary

The 2026-09-01 fix made a leading frontmatter block in the provider's final
response the sanctioned channel for prompt-requested properties, but gated it
behind an authored `response_frontmatter` allowlist (its design decision D2)
and taught the guardrails to return body only unless that list appeared (D3).
Every existing inline document asks for its properties in the prompt and
declares no list, so after the fix a fully obedient agent returned nothing and
the run reported clean success with the requested properties missing.

Ken's ruling on 2026-09-05: there must be no requirement to declare which
properties a prompt will set. The prompt author already has the means to check
whether a property was set and handle the case where it was not; modern agents
almost always write requested frontmatter correctly. D2 and D3 are reversed.
D1 (textual write-back) and D4 (snapshot restore of mid-run drift) stand.

## Observed behavior (2026-09-05)

`claudine compose homelab/docs/unifi/products/voip.md --agent opencode` ran
for 251 s, made 23 tool calls (webfetch, read, skill, ls), wrote no file, and
returned a body with no frontmatter block. The closure printed:

```
✓ Applied the captured replacement body to the target document
✓ Preserved original frontmatter and updated last_updated
✓ Cleaned up generated markdown formatting
```

No warning mentioned the `products` and `researched_by` properties the prompt
requested. Nothing was reset by Claudine: the agent never sent them, because
the migrated `.claudine/inline-compose.md` guardrails told it not to.

## Design

### The response block is always the channel

- The default guardrails now instruct the agent unconditionally: if the
  prompt asks for frontmatter properties, return them in a `---` fenced YAML
  block at the top of the final response.
- The 2026-09-01 shipped guardrail text is added to the known-default list so
  a materialized `.claudine/inline-compose.md` byte-equal to it is migrated
  atomically on the next load. Customized files are still left alone.

### Every returned property is applied

- `response_frontmatter` is removed: the plan field, the prepare-time
  validation and warnings, the prompt protocol appendix, and the
  `ResponseFrontmatterInvalid` error. A document that still carries the key
  is treated as ordinary authored frontmatter.
- The closure merges every top-level key in the response block. New keys are
  inserted in response order immediately before `last_updated` (appended when
  it is absent); existing keys are replaced in place as whole YAML nodes. Each
  insertion and update is reported on the status surface.
- The only exceptions are the closure-owned keys, exposed as
  `CLOSURE_OWNED_PROPERTIES`: a returned `prompt` is ignored with a warning
  naming its response line (it is the author's immutable interface); returned
  `hash` and `last_updated` are ignored silently because the closure stamps
  them on every write.
- Nothing else is reserved. Keys Claudine interprets on later runs (`agent`,
  `model`, `$schema`, lifecycle events) are applied exactly as returned; the
  prompt author owns that choice.

### Unchanged

- Byte-preserving textual write-back of authored frontmatter (D1).
- Mid-run on-disk drift is detection input only and is restored from the
  pre-run snapshot without attribution (D4). Agents are still told not to edit
  the source file directly; the response block is the write path.
- A well-delimited response block with invalid YAML, duplicate keys, a
  non-mapping root, or no body still fails without writing.
- An unchanged replacement body is still an error, even when properties were
  returned.

## Scope

- `claudine/lib/src/composition/guardrails.rs` — new default text; 2026-09-01
  text joins the migration set.
- `claudine/lib/src/composition/prepare.rs`, `types.rs`, `error/mod.rs` —
  allowlist plumbing removed.
- `claudine/lib/src/composition/closure.rs` — default-apply harvest;
  `CLOSURE_OWNED_PROPERTIES`; `rewrite_inline_document` takes response order
  from the harvested map.
- `claudine/cli/src/commands/wrap/inline.rs` — status lines.
- Tests in `closure/tests.rs`, `prepare/tests.rs`, `guardrails.rs`,
  `cli/tests/wrap_inline_compose.rs`.
- Docs: `claudine/docs/topics/composition.md`,
  `claudine/docs/topics/frontmatter-properties.md`, `claudine/cli/README.md`,
  `.claude/skills/claudine/composition.md`, `.claudine/inline-compose.md`.

## Acceptance criteria

- **AC1.** A response block containing an existing key, two new keys,
  `prompt`, `hash`, and `last_updated` yields: existing key replaced in place,
  new keys inserted in response order before `last_updated`, `prompt` reported
  as immutable and left byte-identical, `hash` and `last_updated` values from
  the response absent from the written file.
- **AC2.** A second run against the written document refreshes a previously
  inserted key in place without duplicating it.
- **AC3.** A materialized guardrails file byte-equal to the 2026-09-01 text is
  rewritten to the new default; a customized file is untouched.
- **AC4.** End to end with a provider stub returning a frontmatter block and a
  body, and no declaration in the document: both properties are inserted on
  the first run and updated on the second, the authored `prompt` bytes
  survive, and the delivered prompt carries the new guardrail instruction.
- **AC5.** The response-level failure modes and the unchanged-body error from
  the 2026-09-01 fix keep their tests green.

## Non-goals

- Harvesting properties an agent writes to the source file on disk mid-run.
  D4 restores the snapshot over such edits; the guardrails direct the agent to
  the response block instead.
- Warning when a prompt asks for properties the response did not include.
  The closure cannot know what the prompt asked for; that check belongs to the
  prompt author.
