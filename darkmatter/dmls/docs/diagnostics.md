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

Computing diagnostics is read-only. DMLS resolves local paths through each
repository's file-resolution context (see [File references](./file-references.md))
and reads its in-memory workspace graph; the only filesystem touches are the
existence probes resolution makes and the one repository discovery per context
build. It never executes a shell command (`$(...)`, `::shell`),
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

### `dm.*` registry rules

- **Ownership.** `dm.*` is Darkmatter's product diagnostic namespace, not a DMLS
  one. A code names a *condition*, and every surface that detects that
  condition — the editor, `md compose` warnings — reports it under the same
  string. [`codes.rs`](../src/diagnostics/codes.rs) is the registry, and this
  page documents every code.
- **One condition, one code.** Never mint a second code for a condition another
  surface already reports, and never reuse a code for a different condition.
- **Severity is per surface.** Each surface picks the severity that fits its
  moment, but the same condition should not contradict itself without reason.
  `dm.expression.undeclared_property` is a **Warning** both here and in
  `md compose`. `dm.expression.malformed` is a Warning squiggle while you type,
  while compose fails outright on the same expression.
- **Adding a code.** Add a documented constant to `codes.rs` and a row to the
  layer table below. A compose-side emitter declares the same string on
  `ComposeWarning` (as `UNDECLARED_PROPERTY_CODE` does). There is no shared
  cross-crate constant module until a second shared code exists.

### File-resolution context (`source: darkmatter.context`)

| Code | Meaning |
|------|---------|
| `dm.context.build_failure` | **Error**, at line 0, column 0. The document's file-resolution context could not be built (for example, repository discovery failed on a corrupt `.git/config`), or an untitled buffer's workspace folders do not lie in exactly one repository. No file reference in the document is resolved, including its `$schema`, so no link, transclusion, or schema diagnostic appears beside it. See [File references](./file-references.md#when-a-context-cannot-be-built). |

How the context-failure diagnostic behaves:

- **One per document.** It is the document's only file-reference
  diagnostic, zero-width at line 0, column 0, however many references the
  document holds.
- **The message names the failure.** It gives the failure class and the
  typed error, whose text names the directory the build was anchored at.
  When discovery fails, that is the document's own folder, because no
  repository root could be found:

  ```text
  file references are not resolved in this document (MissingContext):
  repository discovery failed at `/work/repo/docs`: …
  ```

  An untitled buffer's message counts the repositories its workspace
  folders lie in and lists the folders.
- **Reference features are skipped.** Document links, go-to-definition,
  link and transclusion diagnostics, anchor completion for another
  document, and schema validation, hover, and completion all return nothing
  for the document, since its `$schema` resolves through the same context. Features that need
  no path (folding, symbols, expression diagnostics) keep working.
- **Logged once.** The failed build is logged at `error` level with the
  directory and class. The failure is cached in place of the context, so it
  is neither rebuilt nor logged again on later requests.
- **Clearing it.** The cached failure is dropped, and the diagnostic
  re-evaluated, on a watched-file event at or below the failing directory
  or inside the repository's `.git/` (a repaired `.git/config`, say), on any
  configuration change, or when a save-triggered rescan finds a changed
  document or package manifest below the failing directory. The rescan
  does not read `.git/config`, so with an editor that has no file watcher,
  change a DMLS setting or restart the server after repairing it.

Every diagnostic about a reference that did not resolve (`dm.links.broken_path`,
`dm.transclusion.broken_path`, `dm.schema.invalid_file_reference`, and
`dm.context.build_failure`) carries `data: {"resolution_failure": "<Class>"}`,
the biscuit-file failure class (`InvalidReference`, `MissingContext`,
`NoMatch`, `Io`, `UnsupportedRemote`).

### Layer 0 — Markdown links (`source: darkmatter.links`)

| Code | Meaning |
|------|---------|
| `dm.links.broken_path` | A link path matched no existing file and no open document, resolving `&`, `^`, `@`, `~`, and relative paths as `md compose` does. A file outside the workspace folder is not broken. |
| `dm.links.missing_anchor` | The link resolved to an indexed document, but its `#fragment` anchor does not exist there. Fragments on unindexed files are not checked. |
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
| `dm.expression.malformed` | An `expression`-typed value that does not parse. Emitted only for untagged single-line plain and quoted scalars; block and tagged values keep the schema problem. Pending `{{ … }}` / `$(…)` values are deferred. |
| `dm.expression.undeclared_property` | **Warning (advisory).** A bare root of an `expression`-typed value that is an undeclared document property: valid, of unknown type, and `null` unless supplied at runtime. DMLS classifies against Darkmatter's baseline binding view and supplies no host globals, so Claudine's `err`, `timing`, and `group` are reported like any other undeclared property, beneath a lifecycle event or not. `current` is a reserved namespace and is never reported. |
| `dm.expression.unknown_function` | **Error.** A call to a function outside Darkmatter's closed catalog, ranged on the function name, in every branch. Compose fails on the call. |
| `dm.expression.nested_span_in_literal` | A `{{ … }}` inside a quoted string literal on a single-pass lifecycle surface — a whole-value communication field, stack action operand, or `proxy … with` value, or a `when` / `while` / `until` predicate — where it is never interpolated. Offers **Rewrite with + concatenation** when a safe rewrite exists (not for folded or tagged scalars, or a literal that spans lines). |

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
| `dm.transclusion.broken_path` | A `::file` / `::code` / `::toc-linking` target matched no file. |
| `dm.transclusion.nullable_target` | **Warning.** A whole-value `::file`, `::code`, or `::url` expression is statically nullable and is not narrowed by an enclosing guard. |
| `dm.transclusion.cycle` | A `::file` / `::code` transclusion cycle (ancestry in `relatedInformation`). |
| `dm.expression.malformed` | A malformed `{{ … }}` interpolation or `when=` expression. |
| `dm.expression.undeclared_property` | **Warning** (advisory), matching `md compose`. An identifier, in any operand position and ranged at itself, whose root is an undeclared document property: no frontmatter key, schema-declared property, or function names it, and Darkmatter's binding model does not classify it as a reserved namespace (`ctx`, `env`, `doc`, `current`, `current_env`) or the `null` literal. A bare runtime-context name such as `repo` is a document property, never `ctx.repo`. The message says the property is valid, of unknown type, and `null` unless supplied at runtime. (A key the effective schema declares counts as known even when the document does not set it — it is a compose-time parameter.) Handled absence stays silent, as at compose time: a fallback primary (`x \|\| "d"`), a ternary condition and its guarded root (`x ? x : "none"`), and a direct `is_null`/`is_empty` argument. Unlike compose, both ternary branches are checked. A document without frontmatter is never diagnosed, since any name could be a `--set` value. A subtraction whose whitespace-free text is a frontmatter key (`foo--bar`, `a- b`) is reported once, over the subtraction, with a quick-fix. Content inside a `{{{ … }}}` literal is inert and never diagnosed. Also emitted on Expression-typed frontmatter values (`source: darkmatter.frontmatter`). |
| `dm.expression.unknown_function` | **Error**, matching `md compose`, which fails on the call. A call to a function outside Darkmatter's closed catalog, ranged on the function name and reported in every branch. Unlike the advisory, it is reported on a document without frontmatter too. |
| `dm.fence.unknown_language` | A fenced-code language token no grammar recognizes (with a nearest-match suggestion). |
| `dm.security.disallowed_command` | A `::shell` / `::shell-block` / `$()` command the shell policy disallows. |
| `dm.shell.invalid_suffix` | A frontmatter `$()` suffix `md compose` would reject, ranged at the suffix: an unrecognized suffix or text after a suffix (the message lists all five: `::ok`, `::exit-code`, `::result`, `::timeout:<seconds>`, `::no-cache`), an empty `::`, a repeated suffix, a second result suffix (the message names both), or an invalid timeout. Source `darkmatter.compose`. |

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

The expression family follows one ladder: a **warning** means the construct
*might* be wrong, an **error** means it *will never work*.

| Code | Severity |
|------|----------|
| `dm.expression.malformed` on a schema-typed frontmatter value | Error |
| `dm.expression.malformed` on a body `{{ … }}` span | Warning (the braces may be foreign template syntax) |
| `dm.expression.undeclared_property` | Warning (advisory: the property is valid) |
| `dm.expression.unknown_function` | Error (the function catalog is closed) |
| `dm.expression.nested_span_in_literal` | Error |

## Ranging

Ranges come from the concrete syntax tree, never from parsing the message text:

- Value problems (type, constraint, file-reference) range the **value** node.
- `unknown_key` ranges the **offending key**.
- `missing_required` has no node to point at, so it ranges the **parent mapping**
  (a visible, non-zero-width range).
- A YAML parse error ranges the parser's reported position; the last-good tree
  keeps completion and hover alive meanwhile.
- `dm.transclusion.nullable_target` ranges the complete `{{ ... }}` target
  expression. DMLS suppresses it when an enclosing supported guard proves the
  same property present, including `file_exists(x)`, truthy `x`, `!!x`,
  successful `x != null` / `x != ''`, parentheses, and conjunctions. Unknown,
  mixed, and statically non-null targets do not receive the warning.
- `dm.transclusion.broken_path` applies only to concrete local targets of
  `::file`, `::code`, and `::toc-linking`, resolved through the document's
  context like every other reference. A `::toc-linking` fallback chain is
  broken only when no alternative exists and it does not end in `false`; the
  diagnostic spans the whole chain and carries the first alternative's
  failure class. A `#` in a directive target is part of the filename, as in
  composition, never an anchor. Interpolated targets, including a
  `{{VAR}}/file.md` environment path, are excluded because their resolved
  path is not known statically.

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
