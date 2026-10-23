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
- The scheduler publishes as soon as each change is applied and stamps each
  batch with the document version so stale results are ignored. (A debounce
  setting is accepted but not yet applied; see
  [Configuration](#configuration).)
- The publisher emits `publishDiagnostics` — including an **empty** array when a
  document becomes clean, so the editor clears old markers.
- A configuration reload (new editor settings, or an edited `.dmls.toml`)
  re-publishes diagnostics for all open documents without a restart.

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
| `dm.links.missing_anchor` | The link resolved to a workspace document, but its `#fragment` anchor does not exist there. The target is parsed for the check; fragments on files outside the workspace file list are not checked. |
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
| `wiki.portability-collision` | Paths in the workspace file list collide under case-fold / NFC normalization (workspace-scope). |
| `wiki.invalid-percent-escape` | A malformed percent escape, treated literally. |
| `wiki.confusing-extension` | Resolved to a visually confusing name (e.g. `note.md.md`). |
| `wiki.ambiguous-heading-spelling` | Resolved by exact text, but a different heading would match by slug. |
| `wiki.ambiguous-after-rename` | A file rename would leave a wiki link with no unique replacement spelling. |

Findings that depend on which files exist wait for the workspace file list,
which DMLS builds in the background after startup: `dm.links.broken_path`,
`dm.links.missing_anchor`, `dm.transclusion.broken_path`,
`dm.transclusion.cycle`, `wiki.unresolved-target`, `wiki.ambiguous-target`,
and `wiki.heading-missing-in-target` are withheld until the list is ready,
then published. If the walk hit an error, the list is incomplete and the
three `wiki.*` codes stay withheld, since wiki resolution is decided by the
list alone. See [DMLS Indexing](../../docs/lsp/indexing.md).

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
| `dm.schema.host_catalog` | **Warning.** A host global catalog in the document's schema roots (a `kind: schema` file with a `$schema` mapping of globals and no `types`, such as Claudine's `globals.yaml`) failed to load. The message names the file, the field, and the reason; it is placed at the top of the document. The globals that catalog declares are unknown to this document, so expressions reading them are checked as document properties; every catalog that did load keeps working. Source `darkmatter.schema`. |
| `dm.style.unknown_key` | A `style:` key the style schema does not recognize. |
| `dm.style.deprecated_key` | A deprecated `style:` key (a canonical replacement exists). |
| `dm.expression.malformed` | An `expression`-typed value that does not parse. Emitted only for untagged single-line plain and quoted scalars; block and tagged values keep the schema problem. Pending `{{ … }}` / `$(…)` values are deferred. |
| `dm.expression.undeclared_property` | **Warning (advisory).** A bare root of an `expression`-typed value that is an undeclared document property: valid, of unknown type, and `null` unless supplied at runtime. A root is classified against the binding view of its site: inside an md lifecycle stack (`initialize`, `composed`, `failed`, `cancelled`, `finalize`) that event's view, which declares `err` and `timing`; elsewhere Darkmatter's baseline view, where `err` and `timing` are document properties like any other. A global a host catalog in the document's schema roots declares (Claudine's `seq`, `loop`, and `group`) is never reported, inside a stack or not. `current` is a reserved namespace and is never reported. |
| `dm.expression.unknown_function` | **Error.** A call to a function outside Darkmatter's closed catalog, ranged on the function name, in every branch. Compose fails on the call. Also checked in every `{{ … }}` span and `when` condition inside an md lifecycle stack. |
| `dm.expression.unavailable_global` | **Error.** A read, inside an md lifecycle stack, of a global the event's binding view declares unavailable: `err` in `initialize` or `composed`, which handle no failure. The message names the event and the view's reason. Checked in `{{ … }}` spans at any depth of the stack, `when` conditions, and expression-typed values. It never falls back to a frontmatter key of the same name, so an `err:` key does not silence it. A host-catalog global is never reported, because the editor cannot know whether the stack runs inside a loop or a group. |
| `dm.expression.incompatible_call` | **Error.** A function call Darkmatter's design-time check proves wrong, so `md compose` rejects the document: an argument that can never bind (`min(flag, 5)` with `flag: boolean(required)`), a wrong argument count, or an ambiguous overload. Ranged on the argument, or on the whole call for a count or overload problem. Checked in every branch, including ones that never run. Emitted on Expression-typed values, every frontmatter `{{ … }}` span, and `when` conditions inside md lifecycle stacks. |
| `dm.expression.uncertain_call` | **Warning.** A call argument that might not bind, such as typed text passed to a number (`min(count, 5)` with `count: string`). With `expressions.strict`, also an argument that may be `null` where the parameter rejects it, or whose type is unknown. |
| `dm.expression.nested_span_in_literal` | A `{{ … }}` inside a quoted string literal on a single-pass lifecycle surface — a whole-value verb operand (in a list item, a `then`/`else` body, a dictionary event, or `loop.gate`), a `set` or `with` value, or a `when` / `while` / `until` predicate — where it is never interpolated. A `when` is checked where the Claudine extension types it as an expression: an event's or `loop.gate`'s list items and the items of their `then`/`else` bodies; deeper guards are accepted loosely, and Claudine itself checks them before launch. Offers **Rewrite with + concatenation** when a safe rewrite exists (not for folded or tagged scalars, or a literal that spans lines). |

Every schema-validation diagnostic (`dm.schema.type_mismatch`,
`dm.schema.constraint`, `dm.schema.missing_required`, `dm.schema.unknown_key`,
`dm.schema.invalid_file_reference`, and the `type-definition`/`schema`
meta-type `dm.schema.invalid_type_definition`/`invalid_schema_shape`) carries
the validator's JSON Pointer in `data.instance_path`, beside its range. The
range alone does not say which list item or field failed: a list item shares
its line with its first field, and a missing key ranges its parent mapping.
The message is the validator's own text; unlike `md schema validate`, it does
not append the failing property's schema description. Hover on the key shows
that description (for example, the guidance Claudine's schema gives on
`composed`).

```yaml
content_policy:
  - TimeSensitive
  - rule: TimeSensitive
    action: delete   # dm.schema.constraint, data: {"instance_path": "/content_policy/1/action"}
  - action: archive  # data: {"instance_path": "/content_policy/2"}
```

The pointer is the same one `md schema validate --format json` reports as
`path`: a missing or unknown key's pointer names the parent object, and the
message names the key. A file-reference failure carries both keys,
`{"instance_path": "/spec", "resolution_failure": "NoMatch"}`.

Two Layer-2 behaviors reach beyond the Markdown document under edit:

- **Standalone schema documents own their problems.** When a schema YAML file
  is itself open, its `invalid_suggestion` warnings and `document_malformed`
  error are published on that document — they are never duplicated onto the
  Markdown documents whose `$schema` consumes it.
- **Trigger-schema load failures are published on the envelope.** When a
  repository-scoped trigger-registry scan fails for an envelope that cannot
  be loaded (invalid YAML, a malformed match, or a payload that does not
  resolve), DMLS publishes a `dm.schema.prepare` diagnostic on that envelope
  file (even when it is not open) instead of failing silently. A closed
  envelope's diagnostic sits at the start of the file. Once it is open, an
  envelope that is not valid YAML is diagnosed from the reported line and
  column to the end of that line, and other failures cover the whole file. Registries are
  first prepared at startup, so a broken envelope in a workspace folder's
  schema roots is reported before any document opens. The last-good registry keeps serving consumers meanwhile,
  and a recovered scan clears the diagnostic. A YAML file that does not
  [claim the envelope](../../docs/topics/schemas/definition.md#which-files-claim-the-envelope)
  is never diagnosed this way.

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
| `dm.expression.undeclared_property` | **Warning** (advisory), matching `md compose`. An identifier, in any operand position and ranged at itself, whose root is an undeclared document property: no frontmatter key, schema-declared property, or function names it, and Darkmatter's binding model does not classify it as a reserved namespace (`fm`, `ctx`, `env`, `current`) or the `null` literal. `doc` and `current_env` are ordinary property names. A bare runtime-context name such as `repo` is a document property, never `ctx.repo`. The message says the property is valid, of unknown type, and `null` unless supplied at runtime. (A key the effective schema declares counts as known even when the document does not set it — it is a compose-time parameter.) Handled absence stays silent, as at compose time: a fallback primary (`x \|\| "d"`), a ternary condition, a direct `is_null`/`is_empty` argument, and a read an enclosing branch proves present — the true branch of its ternary condition (`x ? x : "none"`) or a later `&&` operand (`x && x.name`). The false branch does not inherit the condition's proof (`x ? "some" : x` still warns), and a proof of `x.a` covers neither `x` nor `x.b`. The branch facts are Darkmatter's own, shared with compose. Unlike compose, both ternary branches are checked, each with its own side's facts. A document without frontmatter is never diagnosed, since any name could be a `--set` value. A subtraction whose whitespace-free text is a frontmatter key (`foo--bar`, `a- b`) is reported once, over the subtraction, with a quick-fix. Content inside a `{{{ … }}}` literal is inert and never diagnosed. Also emitted on Expression-typed frontmatter values (`source: darkmatter.frontmatter`). |
| `dm.expression.unknown_function` | **Error**, matching `md compose`, which fails on the call. A call to a function outside Darkmatter's closed catalog, ranged on the function name and reported in every branch. Unlike the advisory, it is reported on a document without frontmatter too. |
| `dm.expression.incompatible_call` | **Error**, matching `md compose`, which rejects the document. As in frontmatter: a call argument that can never bind, a wrong argument count, or an ambiguous overload, in every branch of every body `{{ … }}` span and every directive `when=` condition, including the body of a `::block` whose condition is false. Types come from the document's `$schema` alone, narrowed by guards (`is_number(foo) ? min(foo, 5) : 0` is silent); a frontmatter value never narrows a type. Fenced code and `{{{ … }}}` literals are not checked. |
| `dm.expression.uncertain_call` | **Warning.** As in frontmatter; see [Severity](#severity). |
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
| `dm.expression.unavailable_global` | Error (the read fails when the stack runs) |
| `dm.expression.nested_span_in_literal` | Error |
| `dm.expression.incompatible_call` | Error (`md compose` rejects the document) |
| `dm.expression.uncertain_call` | Warning (the call may fail for some values) |

### Call diagnostics

Every function parameter has a declared type: `min(a: number, b: number)`
takes two numbers. As you type, DMLS compares the type of each argument with
the type of its parameter and places the pair in one of five **argument-type
categories**. The category decides whether you see an error, a warning, or
nothing.

This document has one call per category:

```markdown
---
$schema:
  size: number(required)    # always a number
  maybe: number             # optional, so a number or null
  count: string             # optional, so text or null
  flag: boolean(required)   # always true or false
size: 2
maybe: 3
count: "4"
flag: true
untyped_var: 1              # not in $schema, so its type is unknown
---
{{ min(size, 5) }}
{{ min(maybe, 5) }}
{{ min(count, 5) }}
{{ min(untyped_var, 5) }}
{{ min(flag, 5) }}
```

| Category | Call above | Non-strict (default) | Strict |
|----------|------------|----------------------|--------|
| 1, always binds | `min(size, 5)` | *not emitted* | *not emitted* |
| 2, fits but may be `null` | `min(maybe, 5)` | *not emitted* | Warning (`uncertain_call`) |
| 3, might not bind | `min(count, 5)`: text converts to a number only when it is numeric | Warning (`uncertain_call`) | Warning (`uncertain_call`) |
| 4, type unknown | `min(untyped_var, 5)` | *not emitted* | Warning (`uncertain_call`) |
| 5, never binds | `min(flag, 5)`: no boolean is a number | Error (`incompatible_call`) | Error (`incompatible_call`) |

Category 5 is always an error and category 1 is always silent. Strict mode
(`expressions.strict`, see [Configuration](#configuration)) adds warnings for
categories 2 and 4. The default reflects one principle: an untyped value gets
the benefit of the doubt, and an optional value is not flagged for being
optional, since nearly every typed property is. As soon as a declared type
might not fit, DMLS warns.

**An error is proven; a warning is possible.** An `incompatible_call` error
means no value of the argument's type can ever bind, so the call fails on
every run. Wrong argument counts (`min(1)`) and calls that match more than one
signature are errors too. An `uncertain_call` warning means the call works for
some values and fails for others; the document still composes, and the value
is checked when the call runs. Silence means "not proven wrong", not "proven
right": a silent call can still fail at runtime, for example when a value
breaks a constraint.

**Types come from `$schema` alone.** DMLS reads each variable's declared type,
never its frontmatter value: `maybe: 3` above does not make `maybe` required,
because a `--set` value, a caller, or interpolation can replace it at compose
time. A variable the schema does not declare, a path below a top-level
property (`meta.count`), and the result of another call or operator
(`maybe || 0`) all have an unknown type (category 4). A guard narrows a type
inside the branch it protects, so `is_number(foo) ? min(foo, 5) : 0` is silent
when `foo` is declared as text or a number.

**Every branch is checked**, including one that never runs: the false side of
a ternary, both operands of `||` and `&&`, and the body of a `::block` whose
condition is false. `true ? 'ok' : min(flag, 5)` is still an error, because
`md compose` rejects it. Fenced code and `{{{ … }}}` literals are not
expressions, so they are never checked.

**Darkmatter decides, DMLS displays.** The categories come from Darkmatter's
library, from the same design-time [call check](../../docs/inline/call-check.md)
that `md compose` runs before it evaluates anything. DMLS has no type,
conversion, or category rule of its own; it only maps a category to a
severity. That is why the editor and composition agree: an
`incompatible_call` error appears on exactly the calls that make `md compose`
reject the document, and composition rejects nothing the editor shows as a
warning or leaves silent. How each type converts to another is described in
[Type Coercion](../../docs/topics/schemas/definition.md#type-coercion).

The message names the function, the parameter, both types, and the category
with its reason:

```text
min: argument 1 (`a`) is boolean, expected number (category 5: no value of that type can bind)
min: argument 1 (`a`) is string | null, expected number (category 3: some values of that type convert and others do not)
min: no signature takes 1 argument; expected min(a: number, b: number): number
```

An argument problem ranges the argument (`flag`); a wrong argument count or an
ambiguous call ranges the whole call. To hide one kind in your editor,
suppress its code; the category number stays visible in the message.

#### Fixing a call diagnostic

| You see | Example | Fix |
|---------|---------|-----|
| Error, category 5 | `min(flag, 5)` with `flag: boolean(required)` | Pass a value of the parameter's type, or correct the schema if the declared type is wrong (`flag: number(required)`). |
| Error, a literal that cannot convert | `number("pear")` | Fix the literal, or give a fallback the function returns instead: `number("pear", 7)` is silent and returns `7`. |
| Error, wrong argument count | `min(1)` | Supply the arguments the signature in the message lists: `min(1, 5)`. |
| Warning, category 3 | `min(count, 5)` with `count: string` | Declare the type the function needs: `count: number`, or `count: numberlike` to keep numeric text as written. Or convert with a fallback, `min(number(count, 0), 5)`, which is silent by default (in strict mode the result of `number(…)` has an unknown type, category 4). |
| Warning, category 3, a union | `min(foo, 5)` with `foo` declared as `[string(required), number(required)]` | Narrow it with a guard: `is_number(foo) ? min(foo, 5) : 0`. |
| Strict warning, category 2 | `min(maybe, 5)` with `maybe: number` | Make the property required (`maybe: number(required)`), or guard the call: `maybe != null ? min(maybe, 5) : 5`. |
| Strict warning, category 4 | `min(untyped_var, 5)` | Declare the variable in `$schema`: `untyped_var: number(required)`. |

## Ranging

Ranges come from the concrete syntax tree, never from parsing the message text:

- Value problems (type, constraint, file-reference) range the **value** node.
- `unknown_key` ranges the **offending key**.
- `missing_required` has no node to point at, so it ranges the **parent mapping**
  (a visible, non-zero-width range).
- A YAML parse error ranges the parser's reported position; the last-good tree
  keeps completion and hover alive meanwhile.
- In a standalone schema document, a `dm.schema.invalid_type_definition`
  grammar error ranges the **offending token** inside the definition, even
  when the value continues onto later lines. Here `bogus` is underlined on
  line 5, not the whole value from line 4:

  ```yaml
  kind: schema
  types:
    t: string(min(1);
      bogus(2))
  ```

  When the token cannot be placed exactly — for example, a sibling uses a
  presentation the schema grammar does not support, such as a block scalar —
  the diagnostic falls back to the definition's whole value. It is never given
  an empty or guessed range.
- `dm.transclusion.nullable_target` ranges the complete `{{ ... }}` target
  expression. DMLS suppresses it when an enclosing `::block when` proves the
  same property present, using the same guard rules as compose: truthy `x`,
  `!!x`, `x != null`, `!(x == null)`, `file_exists(x)`, `is_string(x)` and
  the other type predicates, `!is_null(x)`, `!is_empty(x)`, equality with a
  non-null literal (`x == 'main.md'`, `x == ''`), parentheses, and
  conjunctions. Equality is strict and a missing value is `null`, which
  equals only `null`, so `x != ''` and `x == null` prove nothing.
  Predicates match through the function registry, so aliases and case
  (`fileExists`, `isString`, `!isNull`) count too. A guard is never evaluated:
  `file_exists(x)` proves `x` is a present string inside the block without
  checking any file. A guard that does not exclude null on its true side, such as `is_null(x)` or `x || y`, proves nothing.
  Unknown, mixed, and statically non-null targets do not receive the warning.
  For example, only the first target here is reported:

  ```md
  ::file {{ optional }}

  ::block when="isString(chapter)"
  ::file {{ chapter }}
  ::end-block
  ```
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

DMLS reads its settings from a `.dmls.toml` file at the workspace root and
from the editor's settings, which the editor sends with
`workspace/didChangeConfiguration` (DMLS never requests them with
`workspace/configuration`). Editor settings override the file, and the file
overrides the built-in defaults. A change to either takes effect without a
restart. [DMLS Configuration](../../docs/lsp/configuration.md) lists every
key; these change diagnostics:

- **`schema.strict`** — when `true`, `unknown_key` and `missing_required` become
  errors (see [Severity](#severity)).
- **`expressions.strict`** — when `true`, call arguments that may be `null`
  (category 2) or whose type is unknown (category 4) also warn (see
  [Call diagnostics](#call-diagnostics)). Described below.
- **`diagnostics.debounce_ms`** — accepted and type-checked, but not applied
  yet: diagnostics are published as soon as each change is applied.

### `expressions.strict`

A boolean, `false` by default. It is the only setting for call diagnostics;
no function has a setting of its own. Errors (category 5) and category 3
warnings appear whatever its value.

Set it for the whole workspace in `.dmls.toml`:

```toml
# .dmls.toml
[expressions]
strict = true
```

Or set it from the editor, with the same key under `dmls` (how you enter
LSP settings depends on the editor; see [Editor setup](./editors/README.md)):

```json
{ "dmls": { "expressions": { "strict": true } } }
```

Which value applies:

| Editor settings | `.dmls.toml` | Effective |
|-----------------|--------------|-----------|
| `"strict": true` or `false` | anything | the editor's value |
| no `expressions.strict` key (or `"expressions": {}`) | `strict = true` or `false` | the file's value |
| cleared: the editor sends `null` (or `{ "dmls": null }`) as its whole settings | `strict = true` or `false` | the file's value |
| neither sets it | no `[expressions]` table | `false` |

An invalid value does not stop the server:

- **In the editor** (`"strict": null`, `"strict": "yes"`, `"strict": 1`,
  `"expressions": null`, or `"expressions": 5`): DMLS shows an error message
  and ignores the editor's settings until they are fixed. The file's value
  applies, not the last valid editor value:

  ```text
  dmls: editor settings: settings produced an invalid config (invalid type: string "yes", expected a boolean); the file and default settings remain
  ```

  The editor's other sections are ignored too, except `formatting`, which is
  validated on its own.
- **In `.dmls.toml`** (`strict = "yes"`, a duplicate key): the whole file is
  rejected with an error message and the defaults apply (strict off) until the
  file is fixed.

A change applies without a restart, and DMLS re-publishes the diagnostics of
every open document:

- **Editor settings** apply as soon as the editor sends them.
- **An edit to `.dmls.toml`** applies when the editor reports the file change
  (DMLS asks editors that support dynamic file watching to watch
  `**/.dmls.toml`), or else at the next settings change. Every settings change
  re-reads the file.

See [Changes apply without a restart](../../docs/lsp/configuration.md#changes-apply-without-a-restart)
for what else a reload recomputes.

## See also

- [Hover](./hover.md) — how DMLS *explains* symbols (including the static,
  never-executed account of shell and remote content).
- [Autocomplete](./autocomplete.md) — completion behavior and fields.
- [Features](./features.md) — the full capability overview and per-editor matrix.
