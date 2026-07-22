---
features:
  - 2026-07-04-dmls
  - 2026-07-09-suggest-constraint
  - 2026-07-10-interpolation-literal
  - 2026-07-10-schema-triggers
---
# DMLS Diagnostics

Diagnostics are how DMLS reports problems in a document — broken links, missing
anchors, schema violations, malformed directives, disallowed shell commands, and
more. They surface as the squiggly underlines and Problems-panel entries in your
editor.

DMLS uses **push diagnostics**: the server computes them itself and sends
`textDocument/publishDiagnostics` whenever a document's analysis changes. It does
not wait for the editor to pull them.

## Guiding principle: diagnose edit-time problems, not compose-time ones

A Darkmatter/Claudine document is a **template**. Values arrive at *compose time*
— via CLI `--set`, seed values, `$(...)` shell expansion, `{{ }}` interpolation,
or Claudine's interactive prompt. None of that exists while you are *editing* the
file.

So DMLS deliberately **does not diagnose things that are resolved at compose
time**, because it cannot know statically whether they are actually wrong:

- A frontmatter value holding a deferred `{{ }}` or `$(...)` construct is **not**
  flagged (its type can't be checked until the value is real).
- A schema-`required` key that is absent from the document is **not** flagged in
  the default mode — it is expected to be injected at launch. (Strict mode
  re-enables it; see [Severity](#severity).)

The real enforcement for those lives in the right phase: `md compose` and
`md schema validate` run *with* the injected values and catch genuine
violations there. This keeps the editor free of false positives.

## Passivity

Computing diagnostics is read-only. DMLS resolves local paths and reads its
in-memory workspace graph; the only filesystem touch is an existence `stat` for
broken-path checks. It never executes a shell command (`$(...)`, `::shell`),
fetches a remote URL, or mutates a file. Shell and remote content is *explained*
statically (see the `darkmatter.security` source), never run.

## Pipeline

```
open / change / save / config-reload
        │
        ▼
  provider chain  ──►  DiagnosticsScheduler  ──►  DiagnosticsPublisher
 (per-capability)      (debounce, version-      (version-stamped
                        stamped)                 publishDiagnostics)
```

- Every provider in the registry contributes diagnostics for the document; the
  results are merged.
- The scheduler debounces rapid edits (configurable — see
  [Configuration](#configuration)) and stamps each batch with the document
  version so stale results are ignored.
- The publisher emits `publishDiagnostics` — including an **empty** array when a
  document becomes clean, so the editor clears old markers.
- A `didChangeConfiguration` reload re-publishes diagnostics for all open
  documents without a restart.

## Sources and codes

Every diagnostic carries a stable **`source`** (a namespace) and **`code`**
(a specific problem). These strings are a user-facing contract — editors key
suppression config on them, so they do not get renamed. They are organized by
feature layer.

### Layer 0 — Markdown links (`source: darkmatter.links`)

| Code | Meaning |
|------|---------|
| `dm.links.broken_path` | A relative link path matched no indexed document. |
| `dm.links.missing_anchor` | The link resolved to a document, but its `#fragment` anchor does not exist there. |
| `dm.links.duplicate_heading` | Two or more headings generate the same GitHub anchor slug (carries `relatedInformation` linking the twins). |

### Layer 1 — Wiki links (`source: darkmatter.wiki`)

| Code | Meaning |
|------|---------|
| `wiki.unresolved-target` | A `[[target]]` matched no document. |
| `wiki.ambiguous-target` | A `[[target]]` matched multiple documents. |
| `wiki.heading-missing-in-target` | The file resolved but its `#heading` fragment did not. |
| `wiki.empty-target` | An empty target (`[[]]` / `[[\|alias]]`). |
| `wiki.empty-heading` | An empty heading query (`[[target#]]`). |
| `wiki.unsupported-syntax` | A v1-unsupported form (embed, block ref, interwiki). |
| `wiki.portability-collision` | Indexed paths collide under case-fold / NFC normalization (workspace-scope). |
| `wiki.invalid-percent-escape` | A malformed percent escape, treated literally. |
| `wiki.confusing-extension` | Resolved to a visually confusing name (e.g. `note.md.md`). |
| `wiki.ambiguous-heading-spelling` | Resolved by exact text, but a different heading would match by slug. |
| `wiki.ambiguous-after-rename` | A file rename would leave a wiki link with no unique replacement spelling. |

### Layer 2 — Frontmatter & schema (`source: darkmatter.frontmatter` / `darkmatter.schema` / `darkmatter.style`)

| Code | Meaning |
|------|---------|
| `dm.frontmatter.yaml_parse` | The frontmatter YAML could not be parsed. |
| `dm.schema.invalid_schema_shape` | The `$schema` value is not a valid schema shape. In a standalone schema document this also covers a rejected **outer** declaration — an empty root union, an illegal union arm, or an invalid whole-file reference — ranged at that value or arm. |
| `dm.schema.prepare` | The schema could not be resolved, merged, or compiled. |
| `dm.schema.type_mismatch` | A value did not match its declared type. |
| `dm.schema.constraint` | A non-type constraint failed (range, length, pattern, enum, …). |
| `dm.schema.missing_required` | A required key is absent. **Strict mode only** — see [Severity](#severity). |
| `dm.schema.unknown_key` | A key the schema does not declare is present. |
| `dm.schema.deprecated_key` | A deprecated key is present. |
| `dm.schema.invalid_file_reference` | A `file(...)`-typed value failed to parse, resolve, or match a file. |
| `dm.schema.invalid_suggestion` | A `suggest(...)` candidate is invalid metadata for its target schema (type, range, integer, length, not-empty, or pattern violation, or unrepresentable number syntax). **Warning.** |
| `dm.schema.document_malformed` | A recognized standalone SimplifiedSchema envelope is malformed (missing or non-mapping `types`, unsupported tagged-envelope keys, or an invalid payload) and no more precise declaration or definition diagnostic claims the failure. Ranged over the whole schema document. |
| `dm.style.unknown_key` | A `style:` key the style schema does not recognize. |
| `dm.style.deprecated_key` | A deprecated `style:` key (a canonical replacement exists). |

Two Layer-2 behaviors reach beyond the Markdown document under edit:

- **Standalone schema documents own their problems.** When a schema YAML file
  is itself open, its `invalid_suggestion` warnings and `document_malformed`
  error are published on that document — they are never duplicated onto the
  Markdown documents whose `$schema` consumes it.
- **Trigger-schema load failures are published on the envelope.** When a
  repository-scoped trigger-registry scan fails for an envelope whose payload
  cannot be loaded, DMLS publishes a file-level `dm.schema.prepare` diagnostic
  on that envelope file (even when it is not open) instead of failing
  silently; the last-good registry keeps serving consumers meanwhile, and a
  recovered scan clears the diagnostic.

### Layer 3 — Darkmatter DSL (`source: darkmatter.compose` / `darkmatter.security`)

| Code | Meaning |
|------|---------|
| `dm.directive.unknown` | A `::` keyword the DSL does not recognize. |
| `dm.directive.unclosed_block` | A `::block` / `::shell-block` / disclosure triple left unclosed. |
| `dm.directive.unmatched_end` | A `::end-block` closer with no matching opener. |
| `dm.directive.malformed_option` | An option key a directive family does not recognize. |
| `dm.directive.malformed_disclosure` | A `::disclosure` triple left structurally malformed. |
| `dm.transclusion.broken_path` | A `::file` / `::code` / prologue / epilogue target matched no file. |
| `dm.transclusion.cycle` | A `::file` / `::code` transclusion cycle (ancestry in `relatedInformation`). |
| `dm.expression.malformed` | A malformed `{{ … }}` interpolation or `when=` expression. |
| `dm.expression.unknown_identifier` | An identifier naming no frontmatter key, schema-declared property, `ctx.*`, `env.*`, or function. (A key the effective schema declares counts as known even when the document does not set it — it is a compose-time parameter. Content inside a `{{{ … }}}` literal is inert and never diagnosed.) |
| `dm.fence.unknown_language` | A fenced-code language token no grammar recognizes (with a nearest-match suggestion). |
| `dm.security.disallowed_command` | A `::shell` / `::shell-block` / `$()` command the shell policy disallows. |

The `darkmatter.markdown` source is reserved for CommonMark/GFM structural
problems and grows without renaming the codes above.

## Severity

Most schema value problems (`type_mismatch`, `constraint`,
`invalid_file_reference`, `invalid_schema_shape`, `prepare`) and `yaml_parse`
are **errors**. Two codes vary with `schema.strict`:

| Code | Non-strict (default) | Strict |
|------|----------------------|--------|
| `dm.schema.unknown_key` | Warning | Error |
| `dm.schema.missing_required` | *not emitted* | Error |

`missing_required` is suppressed by default because `required` is a compose-time
contract (the value is injected via CLI / seed / interactive prompt), so a
statically-absent required key is not an editor error. Turn on strict mode when
you want edit-time enforcement of required keys.

## Ranging

Ranges come from the concrete syntax tree, never from parsing the message text:

- Value problems (type, constraint, file-reference) range the **value** node.
- `unknown_key` ranges the **offending key**.
- `missing_required` has no node to point at, so it ranges the **parent mapping**
  (a visible, non-zero-width range).
- A YAML parse error ranges the parser's reported position; the last-good tree
  keeps completion and hover alive meanwhile.

## `relatedInformation`

Some diagnostics attach secondary locations:

- `dm.links.duplicate_heading` links each duplicate heading to its twin(s).
- Schema problems whose origin is a referenced schema file point at that file.
- `dm.transclusion.cycle` lists the transclusion ancestry that forms the cycle.

## Configuration

Via `.dmls.toml` (layered under LSP `workspace/configuration`, reloadable without
restart):

- **`schema.strict`** — when `true`, `unknown_key` and `missing_required` become
  errors (see [Severity](#severity)).
- **Diagnostics debounce** — how long the scheduler coalesces rapid edits before
  recomputing, to keep typing responsive on large documents.

## See also

- [Hover](./hover.md) — how DMLS *explains* symbols (including the static,
  never-executed account of shell and remote content).
- [Autocomplete](./autocomplete.md) — completion behavior and fields.
- [Features](./features.md) — the full capability overview and per-editor matrix.
