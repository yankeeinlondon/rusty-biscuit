---
last_updated: 2026-05-02
---
# Darkmatter LSP Proposed Features

This document inventories a probable set of feature we will want in the Darkmatter DSL and translates each feature into the concrete Language Server Protocol (LSP) capabilities required to support it inside an editor.


## 1. DSL Surface Area

Darkmatter's compose pipeline runs in four phases (`Inline Pre`, `Transclusion`, `Inline Post`, `Finalization`). The LSP must understand every textual artifact that authors can write inside the document — both inside the YAML frontmatter and inside the Markdown body.

> **Note:** the _compose_ functionality that Darkmatter's library provides can convert any Markdown document into it's completed form which is always a valid Markdown document

### 1.1 Frontmatter Surface

::file frontmatter.md

### 1.2 Body Surface

The body is CommonMark + GFM augmented with Darkmatter directives. Every line that begins with `::` is a candidate directive; some directives have a paired `::end-block` form.

#### Directive families

| Family                  | Inline form            | Block form                                                                                  |
|-------------------------|------------------------|---------------------------------------------------------------------------------------------|
| Conditional region      | —                      | `::block when="…"` … `::end-block` (nest to arbitrary depth).                               |
| Single shell command    | `::shell <command>`    | —                                                                                           |
| Multi-line shell script | —                      | `::shell-block` … `::end-block` (line-continuation via trailing `\`, key=value parameters). |
| Markdown transclusion   | `::file <path> [opts]` | —                                                                                           |
| Code transclusion       | `::code <path> [opts]` | —                                                                                           |
| Remote transclusion     | `::url <uri> [opts]`   | —                                                                                           |
| TOC-linking             | `::toc-linking <path>` | —                                                                                           |

#### Inline expressions

| Construct       | Example                            | Notes                                                                                                                                                                             |
|-----------------|------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Variable        | `{{ name }}`                       | Pulled from the effective frontmatter state.                                                                                                                                      |
| Nested path     | `{{ user.email }}`                 | Dotted access into objects.                                                                                                                                                       |
| Runtime context | `{{ ctx.today }}`                  | `ctx.now`, `ctx.utc`, `ctx.today`, `ctx.yesterday`, `ctx.tomorrow`, `ctx.dow`, `ctx.dow_abbr`, `ctx.year`, `ctx.month`, `ctx.month_name`, `ctx.month_name_abbr`. Captured lazily. |
| Environment     | `{{ env.HOME }}`                   | Reads `std::env`.                                                                                                                                                                 |
| Fallback chain  | `{{ a \|\| b \|\| "default" }}`        | First truthy value wins.                                                                                                                                                          |
| Ternary         | `{{ active ? "on" : "off" }}`      | Uses condition truthiness rules.                                                                                                                                                  |
| Comparison      | `{{ count > 0 ? "has" : "none" }}` | Operators `==`, `!=`, `>`, `>=`, `<`, `<=`. Numeric strings auto-coerce.                                                                                                          |
| Helpers         | `{{ length(items) }}`              | `length`, `number(x, default)`, `round(x, default)`.                                                                                                                              |

Inline code spans (single backticks) are interpolated by default. Fenced/indented code blocks are skipped unless `interpolate_code_blocks: true`.

#### Directive options vocabulary

A directive line such as `::file ./doc.md when="draft" set.title="Override" exclude="Appendix" replace=parent quotation disclosure="Click"` may carry these keys:

- `when="<expr>"` — boolean DSL evaluated against effective state
- `set=<json5-object>` — object-form frontmatter override (middle layer of the three-layer merge)
- `set.<NAME>=<json5-value>` — property-form frontmatter override (top layer; preserves directive order for last-write tracing)
- `replace=parent` | `replace=child` | `replace={…}` — replacement-map inheritance
- `quotation` (boolean) | `quotation="caption"` — wrap output in a blockquote
- `disclosure="summary"` — wrap output in a `<details>` block
- `exclude="Heading A"` (repeatable) — drop named sections from transcluded markdown
- Shell-block parameters: `when_error="…"`, `when_exit_code=<int>`, `stderr_contains="…"`, `timeout=<seconds>`

#### Horizontal-rule attribute blocks

```markdown
--- { style: waves, width: "50%", color: "#88c0d0" }
```

YAML flow-mapping syntax appended to a CommonMark thematic break. Unknown enums or keys fall back to defaults and emit `tracing::warn!`.

### 1.3 Cross-document Graph

Transclusions form a directed graph. Cycle detection runs on ancestry. Heading levels are re-leveled into the host document. `::toc-linking` reads from a referenced file's *raw source* headings, not its composed output.

## 2. Author Pain Points the LSP Must Solve

Translating the surface area above into authoring needs:

1. **Discoverability** — There are many directive keywords, option keys, and `ctx.*` names. None of them are obvious from a blank document.
2. **Validation** — Misspelled directives, malformed expressions, unknown frontmatter keys, broken transclusion paths, cycles, exceeded depth, unapproved shell commands, and timeouts must surface as diagnostics, not as silent leftover `{{ … }}` literals.
3. **Live feedback** — Authors want to see the resolved value of `{{ user.email }}`, the output of an `::shell` directive, or the contents of a transcluded file without leaving the editor.
4. **Navigation** — Jump from `::file ./chapter.md` to the file. Jump from `{{ user.email }}` to the frontmatter key that defines `user`. Find every place that transcludes the current file.
5. **Refactor safety** — Renaming a frontmatter key should rename every interpolation that refers to it. Moving a transcluded file should fix every directive that points at it.
6. **Schema conformance** — When a `$schema` is declared, frontmatter must be validated continuously, with errors anchored to the offending byte range.
7. **Security visibility** — Shell expansion runs commands. The author must see which commands are about to run, which are blacklisted, which need approval, and what timeouts apply.
8. **Formatting** — Inline Post normalization (cleanup, heading levels) should be available on demand without leaving the editor.
9. **Preview** — Authors should be able to preview the fully composed document (all phases applied) without invoking the CLI.

## 3. Required LSP Capabilities

The following table maps each LSP capability to the DSL feature(s) that motivate it. Capability names follow LSP 3.17 conventions.

| LSP capability                                             | DSL features served                                                                                                                                                                                                                              | Required behaviour                                                                                                                                                                                                                                                                                                                                                                               |
|------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `textDocument/didOpen`, `didChange`, `didClose`, `didSave` | All.                                                                                                                                                                                                                                             | Incremental sync; maintain a Virtual File System keyed by URI; preserve byte→(line, character) tables for translating `pulldown-cmark` ranges.                                                                                                                                                                                                                                                   |
| `textDocument/publishDiagnostics`                          | Frontmatter parse, schema validation, expression parsing, condition DSL, directive parsing, transclusion resolution, cycle detection, depth limits, shell policy, shell timeouts, horizontal-rule attribute warnings.                            | Continuous publish on every doc change. Diagnostic source: `darkmatter`. Severity tiers: `Error` for structural failures (cycles, max depth, schema violations, shell denial), `Warning` for fail-fast=false fallbacks (left-in-place expressions, downgraded set-override errors, unknown option keys, hr attribute fallbacks), `Information` for shell timeouts converted to empty strings.    |
| `textDocument/completion`                                  | Interpolation expressions, `ctx.*`, `env.*`, frontmatter paths, directive keywords, directive option keys, shell-block parameter keys, hr attribute keys/values, `replace:` keys (when authoring substitutions), schema-driven frontmatter keys. | Trigger characters: `{`, `.`, `:`, `=`, ` ` after `::`, `$` for `$schema`. Item kinds: `Variable` for FM/ctx/env paths, `Keyword` for directives, `Property` for option keys, `Value` for enum option values, `Constant` for ctx values. Completion item documentation: type, sample resolved value when available.                                                                              |
| `textDocument/hover`                                       | Interpolation expressions, transclusion targets, shell directives, frontmatter keys, hr attributes, schema-described keys.                                                                                                                       | For `{{ expr }}`: show parsed AST, current resolved value, resolution path (FM key, ctx, env). For `::file/::code/::url`: show resolved absolute path, character/byte size, transitive depth. For `::file-links`: show matched file count, target directory, and depth. For `::shell`: show effective command line, policy verdict, last cached output (if cache hit). For frontmatter keys: schema description and type.                                                  |
| `textDocument/definition`                                  | Frontmatter path interpolation, transclusion directives, `prologue`/`epilogue`.                                                                                                                                                                  | `{{ user.email }}` → frontmatter location of `user.email`. `::file ./x.md` → file URI. `prologue: ./header.md` → file URI.                                                                                                                                                                                                                                                                       |
| `textDocument/declaration`                                 | Same as definition for transclusions; reserved for future "schema declares this key" jumps.                                                                                                                                                      | —                                                                                                                                                                                                                                                                                                                                                                                                |
| `textDocument/references`                                  | Frontmatter keys referenced by interpolation; transclusion targets.                                                                                                                                                                              | Forward and reverse: from a frontmatter key, list every `{{ … }}` referencing it; from a file, list every directive that transcludes it.                                                                                                                                                                                                                                                         |
| `textDocument/rename`                                      | Frontmatter keys; nested paths; `set.NAME=` overrides.                                                                                                                                                                                           | Rename across the whole workspace, traversing the dependency graph. Refuse rename when the new name shadows a `ctx.*`/`env.*` reserved root.                                                                                                                                                                                                                                                     |
| `textDocument/documentSymbol`                              | Markdown headings, `::block` regions, `::shell-block` regions, transclusion sites.                                                                                                                                                               | Hierarchical: headings as the spine; directives nested under their containing heading; option keys exposed as child symbols of their directive.                                                                                                                                                                                                                                                  |
| `workspace/symbol`                                         | Headings and directives across all open files.                                                                                                                                                                                                   | —                                                                                                                                                                                                                                                                                                                                                                                                |
| `textDocument/documentLink`                                | Transclusion targets (`::file`, `::code`, `::url`, `prologue`, `epilogue`, `::toc-linking`, `::file-links`); standard markdown links/images (delegated to base markdown logic).                                                                                  | Provide both target URI and a tooltip showing transclusion options (`when=`, `exclude=`, etc.).                                                                                                                                                                                                         
                                                                                          |
| `textDocument/codeLens`                                    | Transclusion sites; shell directives; page-block sites; schema-validated frontmatter.                                                                                                                                                            | "Compose preview", "Run with approval", "Cache hit/miss", "Skipped (when=false)", "Reference count: N".                                                                                                                                                                                                                                                                                          |
| `textDocument/inlayHint`                                   | Interpolation resolved values; ctx values; resolved transclusion target paths; resolved shell command stdout (truncated); page-block evaluation outcome.                                                                                         | Position hints at the closing `}}` of an expression, end of a directive line, after `::block` opening. Toggleable per category through configuration.                                                                                                                                                                                                                                            |
| `textDocument/codeAction`                                  | Schema gap fixing; shell approval; cycle/depth resolution; permissive `set=` recovery; horizontal-rule attribute correction; cleanup application; heading normalization; "extract section into transclusion" / "inline transclusion".            | Quick fixes for: missing required schema keys, unknown option keys (suggest closest), unapproved shell commands (open approval), invalid `set=` (downgrade flag), broken transclusion path (create-file action). Refactors: extract paragraph → new file + `::file`, inline `::file` content. Source actions: `darkmatter.cleanup`, `darkmatter.normalizeHeadings`, `darkmatter.composePreview`. |
| `textDocument/formatting`, `rangeFormatting`               | Inline Post Cleanup + Normalization.                                                                                                                                                                                                             | Run cleanup pipeline, return text edits. Format-on-save opt-in.                                                                                                                                                                                                                                                                                                                                  |
| `textDocument/foldingRange`                                | YAML frontmatter, `::block`/`::end-block`, `::shell-block`/`::end-block`, transclusion directives spanning multiple lines, code fences.                                                                                                          | Fold by directive boundaries, not just blank lines.                                                                                                                                                                                                                                                                                                                                              |
| `textDocument/documentHighlight`                           | Block-pair matching (`::block`↔`::end-block`, `::shell-block`↔`::end-block`); frontmatter key ↔ interpolation usages; transclusion path ↔ other usages of the same path.                                                                         | Highlight matching pairs as the cursor moves between them.                                                                                                                                                                                                                                                                                                                                       |
| `textDocument/selectionRange`                              | Expression-aware selection: token → operand → operator → full expression → enclosing `{{ … }}` → enclosing line → enclosing block.                                                                                                               | —                                                                                                                                                                                                                                                                                                                                                                                                |
| `textDocument/semanticTokens`                              | Directive keywords, option keys, expression operators, ctx/env namespaces, frontmatter keys vs values, shell command bodies, horizontal-rule attribute blocks.                                                                                   | Token types: `keyword` (directives), `property` (options/frontmatter keys), `variable` (interpolation roots), `namespace` (`ctx`, `env`), `operator` (\`                                                                                                                                                                                                                                          |
| `textDocument/diagnostic` (pull diagnostics, 3.17)         | Same set as `publishDiagnostics`, for editors that prefer pull.                                                                                                                                                                                  | —                                                                                                                                                                                                                                                                                                                                                                                                |
| `workspace/willRenameFiles`, `didRenameFiles`              | Transclusion targets.                                                                                                                                                                                                                            | Auto-edit `::file`/`::code`/`::url`/`prologue`/`epilogue`/`::file-links` references when files are renamed in the editor.                                                                                                                                                                                                                                                                                       |
| `workspace/didChangeWatchedFiles`                          | Transclusion graph; schema files referenced by `$schema`; shell-policy files.                                                                                                                                                                    | Re-validate downstream documents when ancestors change on disk.                                                                                                                                                                                                                                                                                                                                  |
| `workspace/configuration`, `didChangeConfiguration`        | Schema mappings; shell policy/whitelist/blacklist; cache directory; permissive flags (`--allow-invalid-frontmatter-assignment`, `--allow-reassigned-frontmatter-property`, `--allow-shell-timeout`); inlay-hint categories.                      | Pull settings from the editor; hot-reload.                                                                                                                                                                                                                                                                                                                                                       |
| `workspace/executeCommand`                                 | Custom DSL operations beyond LSP standards.                                                                                                                                                                                                      | Required commands: `darkmatter.composePreview`, `darkmatter.renderTransclusion`, `darkmatter.approveShellCommand`, `darkmatter.runShellNow`, `darkmatter.invalidateCache`, `darkmatter.normalize`, `darkmatter.cleanup`, `darkmatter.graph` (open dependency-graph view), `darkmatter.toggleInlayCategory`.                                                                                      |
| `window/showMessage`, `showMessageRequest`, `logMessage`   | Shell approval prompts; first-run config diagnostics; schema fetch failures.                                                                                                                                                                     | Use `showMessageRequest` for the approve/deny flow on `::shell`.                                                                                                                                                                                                                                                                                                                                 |
| `window/workDoneProgress`                                  | Initial workspace scan; transitive graph build; remote `::url` fetches; shell command execution.                                                                                                                                                 | Required for responsive UX during long composes.                                                                                                                                                                                                                                                                                                                                                 |
| `textDocument/signatureHelp`                               | Helper function calls (`length(x)`, `number(x, default)`, `round(x, default)`); future user-defined helpers.                                                                                                                                     | Trigger characters: `(`, `,`. Show full signature, active parameter, return type, and a one-line example. Reuse the same docs surfaced by hover and completion-resolve.                                                                                                                                                                                                                          |
| `textDocument/linkedEditingRange`                          | Paired directive openers/closers: `::block`↔`::end-block`, `::shell-block`↔`::end-block`. Paired interpolation delimiters `{{ … }}`. Paired YAML flow-mapping braces in hr attribute blocks.                                                     | When the cursor is in one half of the pair, return the matching range so editors can synchronize edits (e.g., renaming the `name="…"` of a future labelled block).                                                                                                                                                                                                                               |
| `textDocument/onTypeFormatting`                            | Auto-close `}}` when `{{` is typed; auto-insert `::end-block` after a newly opened `::block`/`::shell-block` line; auto-indent option-key continuation; auto-close YAML flow `}` for hr attribute blocks.                                        | Trigger characters: `{`, `\n`, ` `, `}`. Behaviour must respect existing closers (no double-insertion) and must be toggleable via `darkmatter.editor.autoPair`.                                                                                                                                                                                                                                  |
| `textDocument/prepareRename`                               | Frontmatter keys, nested paths, transclusion targets, headings (when used as `exclude=` or anchor targets), shell-policy alias names.                                                                                                            | Required so editors can show the renameable token range and reject renames over reserved roots (`ctx`, `env`) before the user commits to a new name.                                                                                                                                                                                                                                             |
| `$/cancelRequest`                                          | Long-running operations: `darkmatter.composePreview`, `::url` fetches, `::shell`/`::shell-block` execution, full workspace re-validation, transitive graph rebuild.                                                                              | Every async operation must be cancellation-aware: drop work as soon as the client cancels and return `RequestCancelled` (`-32800`). Shell processes that have already started should be killed via `child.kill()` with their group.                                                                                                                                                              |
| `client/registerCapability`, `unregisterCapability`        | Schema-conditional capabilities (e.g., enabling stricter completion only when a `$schema` is bound); shell-policy-conditional capabilities (e.g., disabling `runShellNow` when no policy file is configured); preview-mode capability surfacing. | Register dynamically rather than hard-coding everything in `InitializeResult` so the editor only sees commands/code actions that will actually do something. Re-register on `didChangeConfiguration`.                                                                                                                                                                                            |
| `$/progress` partial results                               | `workspace/symbol`, `references`, `documentSymbol`, `workspace/diagnostic`.                                                                                                                                                                      | Stream large result sets so Quick Open / Find All References stays responsive on large workspaces.                                                                                                                                                                                                                                                                                               |

## 4. Feature → Capability Matrix

This matrix lists every individual Darkmatter feature and the LSP capabilities that must be implemented to support it end-to-end. "—" marks capabilities that do not apply.

### 4.1 Frontmatter

| Feature                              | Diagnostics                                                 | Completion                             | Hover                                   | Definition / References                       | Inlay                                 | Code Action                                           | Other                                 |
|--------------------------------------|-------------------------------------------------------------|----------------------------------------|-----------------------------------------|-----------------------------------------------|---------------------------------------|-------------------------------------------------------|---------------------------------------|
| YAML parse                           | parse errors with precise spans                             | YAML key/value scaffolding             | key documentation when schema available | —                                             | —                                     | "Convert flow ↔ block"                                | semantic tokens for keys/values       |
| `$schema` resolution                 | unreachable schema, invalid JSON Schema                     | schema URL completion (registry)       | resolved schema title/description       | Definition → schema document                  | —                                     | "Bind workspace schema for this folder"               | `didChangeWatchedFiles` for schema    |
| Schema validation                    | type/required/enum/pattern violations                       | schema-driven keys, enum values        | schema description for any key          | —                                             | hint with expected type next to value | "Add missing required key", "Coerce to expected type" | progress for big schemas              |
| Frontmatter Interpolation            | unresolved expressions, parse errors                        | FM/ctx/env paths inside scalar literal | resolved value when known               | Definition → referenced FM key                | inlay showing resolved value          | —                                                     | semantic tokens inside scalar         |
| Frontmatter Shell Expansion `$(cmd)` | malformed substitution (hard error), policy denial, timeout | shell aliases from policy file         | policy verdict and command preview      | Definition → policy entry                     | inlay showing trimmed stdout          | "Approve command", "Add to whitelist"                 | `executeCommand: runShellNow`         |
| `replace:` map                       | non-scalar values (info), conflicting keys (warn)           | nothing (literal authoring)            | —                                       | References → every literal occurrence in body | —                                     | "Convert long replacement chain to interpolation"     | rename refactors literals across body |
| `prologue` / `epilogue`              | path missing, cycle, depth                                  | path completion                        | resolved path, file size                | Definition → file                             | —                                     | "Inline content", "Convert to `::file`"               | document link                         |
| `style.hr` defaults                  | unknown enum, unknown key                                   | enum values, attribute keys            | resolved style/color                    | —                                             | inlay color swatch when `color` set   | "Pick from palette"                                   | semantic tokens                       |
| `interpolate_code_blocks`            | type mismatch                                               | boolean                                | —                                       | —                                             | —                                     | —                                                     | —                                     |

### 4.2 Page Blocks

| Feature                 | Diagnostics                                                                    | Completion                                        | Hover                           | Inlay                              | Code Action                                               | Other                                                                            |
|-------------------------|--------------------------------------------------------------------------------|---------------------------------------------------|---------------------------------|------------------------------------|-----------------------------------------------------------|----------------------------------------------------------------------------------|
| `::block when="<expr>"` | unmatched pairs, condition parse errors, unknown identifiers                   | condition DSL identifiers (FM/ctx/env), operators | parsed AST + current evaluation | inlay `=> rendered` / `=> skipped` | "Wrap selection in block", "Remove block (keep contents)" | document highlight pairs `::block`↔`::end-block`, folding range, semantic tokens |
| Lazy child evaluation   | informational diag noting that children of a skipped parent were not evaluated | —                                                 | —                               | —                                  | —                                                         | —                                                                                |
| Code-fence protection   | none (informational)                                                           | —                                                 | —                               | —                                  | —                                                         | —                                                                                |

### 4.3 Interpolation

| Feature                             | Diagnostics                                                                      | Completion                                              | Hover                                                      | Definition / Refs                                               | Rename                  | Inlay                   | Code Action                                               |
|-------------------------------------|----------------------------------------------------------------------------------|---------------------------------------------------------|------------------------------------------------------------|-----------------------------------------------------------------|-------------------------|-------------------------|-----------------------------------------------------------|
| `{{ var }}` / `{{ a.b.c }}`         | unknown root, missing nested path, type mismatch in operations                   | every key in effective state, drill-down on `.`         | resolved value, resolution path, type                      | Def → FM site; Refs → every other `{{ … }}` using the same path | rename across workspace | resolved value          | "Move to fallback", "Wrap in ternary"                     |
| `ctx.*`                             | unknown ctx key                                                                  | full ctx catalogue                                      | description + sample value                                 | —                                                               | —                       | resolved value          | —                                                         |
| `env.*`                             | warn on missing env (non-fatal)                                                  | env names from a configurable allowlist (privacy guard) | resolved value (masked when name matches a secret pattern) | —                                                               | —                       | resolved value (masked) | "Inline current value (constant)"                         |
| Fallback `\|\|`                       | malformed chain                                                                  | —                                                       | first truthy operand                                       | —                                                               | —                       | resolved branch         | —                                                         |
| Ternary `?:`                        | malformed expression                                                             | —                                                       | parsed AST                                                 | —                                                               | —                       | resolved branch         | —                                                         |
| Comparisons                         | type mismatch warning                                                            | operators                                               | —                                                          | —                                                               | —                       | resolved boolean        | —                                                         |
| Helpers `length`, `number`, `round` | wrong arity, wrong type                                                          | function names + signatures                             | doc + return type                                          | —                                                               | —                       | resolved value          | —                                                         |
| Code-block opt-in                   | warn when `{{ }}` appears in fenced block but `interpolate_code_blocks` is false | —                                                       | —                                                          | —                                                               | —                       | —                       | "Enable `interpolate_code_blocks`", "Escape literal `{{`" |

### 4.4 Shell Expansion (`::shell`)

| Feature                 | Diagnostics                                                     | Completion                        | Hover                                          | Definition          | Inlay          | Code Action                                             | Custom command                                             |
|-------------------------|-----------------------------------------------------------------|-----------------------------------|------------------------------------------------|---------------------|----------------|---------------------------------------------------------|------------------------------------------------------------|
| Single command          | malformed line, unapproved, blacklisted, timeout, non-zero exit | known shell aliases, common flags | resolved command, policy verdict, exit history | → policy file entry | trimmed stdout | "Approve", "Add to whitelist", "Convert to shell-block" | `darkmatter.runShellNow`, `darkmatter.approveShellCommand` |
| Approval flow           | —                                                               | —                                 | —                                              | —                   | —              | —                                                       | `window/showMessageRequest` for prompt                     |
| `--allow-shell-timeout` | downgraded to warning when flag is on                           | —                                 | —                                              | —                   | "(timed out)"  | toggle the flag                                         | —                                                          |

### 4.5 Shell Blocks (`::shell-block`)

| Feature                                | Diagnostics                                                                   | Completion                                                                                                 | Hover                                         | Inlay                    | Code Action                                                       |
|----------------------------------------|-------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------|-----------------------------------------------|--------------------------|-------------------------------------------------------------------|
| Multi-line script                      | unmatched `::end-block`, missing line continuation, malformed key=value param | parameter keys (`when_error`, `when_exit_code`, `stderr_contains`, `timeout`), values for enum-like params | per-command policy verdict, total output size | per-line stdout snippets | "Split into separate `::shell` calls", "Add `when_error` handler" |
| Pre-execution policy check             | every command's verdict                                                       | —                                                                                                          | —                                             | —                        | "Approve all"                                                     |
| Partial-output preservation on failure | informational                                                                 | —                                                                                                          | error preview shows partial output            | —                        | —                                                                 |

### 4.6 Transclusion

| Feature                                        | Diagnostics                                                                                                | Completion                                                                                                                                              | Hover                                                       | Definition / Refs                                | Rename                         | Inlay                                                             | Code Action                                                                                                                        | Document Link |
|------------------------------------------------|------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------|--------------------------------------------------|--------------------------------|-------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------|---------------|
| `::file <path>`                                | missing file, cycle, depth exceeded, parse error in target                                                 | path completion (relative to current file), `when=`/`exclude=`/`replace=`/`set=`/`set.NAME=`/`quotation=`/`disclosure=` keys, enum values for `replace` | absolute path, file size, transitive depth, options summary | Def → file; Refs → every directive pointing here | rename via `willRenameFiles`   | resolved heading-level offset, post-merge effective state preview | "Inline content", "Replace with `::code`", "Add `when=` guard", "Move to `prologue`"                                               | yes           |
| `::code <path>`                                | missing file, unknown language inferred                                                                    | path completion, language hint                                                                                                                          | inferred language, byte size                                | Def → file                                       | rename target                  | first/last line preview                                           | "Switch to `::file`" if path is .md                                                                                                | yes           |
| `::url <uri>`                                  | fetch failure, unauthorised host (policy), TLS errors                                                      | URL templates from config                                                                                                                               | last-fetch status, cache age                                | —                                                | —                              | byte size, content-type                                           | "Snapshot to local file", "Add `when=`"                                                                                            | yes           |
| `::toc-linking <path>`                         | missing file, no headings                                                                                  | path completion, filter options                                                                                                                         | heading count from raw source                               | Def → file                                       | —                              | inlay listing top headings                                        | "Convert to `::file` with `exclude=`"                                                                                              | yes           |
| `prologue`, `epilogue`                         | same as `::file`                                                                                           | path completion                                                                                                                                         | resolved file                                               | Def → file                                       | rename target                  | content preview                                                   | "Convert to inline `::file`"                                                                                                       | yes           |
| `when=` on directives                          | condition parse errors, unknown identifiers                                                                | condition DSL identifiers, operators                                                                                                                    | parsed AST + evaluation                                     | —                                                | —                              | `=> included` / `=> skipped`                                      | —                                                                                                                                  | —             |
| `exclude="Heading"`                            | unknown heading in target                                                                                  | heading names from target                                                                                                                               | —                                                           | Def → that heading in target                     | —                              | —                                                                 | —                                                                                                                                  | —             |
| `set=<json5-object>`, `set.NAME=<json5-value>` | invalid JSON5, reassignment without permissive flag, type conflict with target schema, deferred set errors | property names from child's frontmatter, JSON5 value scaffolding                                                                                        | merged effective state for child after override             | Def → child's FM site                            | rename target across overrides | resolved override value                                           | "Enable `--allow-invalid-frontmatter-assignment`", "Enable `--allow-reassigned-frontmatter-property`", "Move to child frontmatter" | —             |
| `replace=parent\|child\|{…}`                     | invalid JSON5 map, conflicting modes                                                                       | enum + JSON5 map scaffold                                                                                                                               | resolved replacement map                                    | —                                                | —                              | merged effective replace map                                      | —                                                                                                                                  | —             |
| `quotation`, `disclosure`                      | malformed value                                                                                            | enum/string                                                                                                                                             | rendered wrapper preview                                    | —                                                | —                              | —                                                                 | "Toggle quotation/disclosure"                                                                                                      | —             |
| Cycle / max-depth                              | always hard errors with full ancestry chain                                                                | —                                                                                                                                                       | —                                                           | —                                                | —                              | —                                                                 | "Open dependency graph at this node"                                                                                               | —             |
| Heading re-leveling                            | warn on H6 overflow clamp                                                                                  | —                                                                                                                                                       | —                                                           | —                                                | —                              | resolved level offset                                             | —                                                                                                                                  | —             |

### 4.7 Other Body Constructs

| Feature                                                                 | Diagnostics                                            | Completion                  | Hover                         | Inlay         | Code Action                                      |
|-------------------------------------------------------------------------|--------------------------------------------------------|-----------------------------|-------------------------------|---------------|--------------------------------------------------|
| Horizontal-rule `---` with attributes                                   | unknown enum, unknown key, malformed YAML flow mapping | enum values, attribute keys | resolved style + color swatch | color swatch | "Pick from palette", "Move to `style.hr` frontmatter" |
| `YamlBlock` payloads (when authored as fenced YAML with a known marker) | YAML parse errors                                      | —                           | parsed structure preview      | —             | "Convert to typed schema"                        |

### 4.8 Standard Markdown Surface

Darkmatter is a CommonMark/GFM superset, so the LSP must also cover the base Markdown features. These are independent of any DSL directive and apply to every document.

| Feature                                       | Diagnostics                                                                                       | Completion                                                | Hover                                          | Definition / References                                       | Code Action                                          | Other                                                      |
|-----------------------------------------------|---------------------------------------------------------------------------------------------------|-----------------------------------------------------------|------------------------------------------------|---------------------------------------------------------------|------------------------------------------------------|------------------------------------------------------------|
| Inline link `[text](path)` / image `![](…)`   | broken file path, broken `#anchor`, dangling reference-style `[ref]: …`                           | path completion (relative to current file)                | resolved target, inferred MIME, image preview  | Def → file or heading; Refs → every link/image to the target  | "Convert to `::file`", "Convert image to `::code`"   | `documentLink` carries the URI                             |
| Inline anchor `[text](#heading)`              | unknown heading, ambiguous heading                                                                | every heading slug in the current document plus targets   | resolved heading text + slug                   | Def → heading line                                            | "Promote to `::toc-linking`"                         | document highlight pairs link ↔ heading                    |
| Reference-style links `[ref]: target`         | duplicate label, unused label, undefined reference                                                | label name when typing `[ref]`                            | resolved target                                | Def → label definition; Refs → every `[ref]` consumer         | "Inline reference", "Convert to inline link"         | rename ref labels across file                              |
| Headings `#` / `##` / …                       | duplicate slug, level skip (`H1` → `H3`), missing `H1`, `H6` overflow (also flagged by re-level)  | none (free-form text)                                     | computed slug + transclusion-safe level offset | Refs → `exclude=`, `[…](#slug)`, `::toc-linking` consumers    | "Renumber sub-tree", "Add anchor"                    | document symbols, folding, semantic tokens                 |
| Code fences with language tag                 | unknown language tag (info), interpolation inside fence without opt-in (warn — see §4.3)          | language identifiers from registered grammars             | language name, syntect theme used              | —                                                             | "Enable `interpolate_code_blocks`", "Inline code"    | folding, semantic tokens (delegated to embedded — see §4.9)|
| Tables (GFM)                                  | malformed alignment row, column count mismatch                                                    | alignment markers (`:--`, `:-:`, `--:`)                   | column count summary                           | —                                                             | "Format table", "Convert to YAML frontmatter"        | folding per row group                                      |
| Task lists `- [ ]` / `- [x]`                  | malformed marker                                                                                  | task marker                                               | —                                              | —                                                             | "Toggle task state"                                  | document symbols (when configured)                         |
| Footnotes (GFM)                               | undefined footnote, unused footnote                                                               | footnote label                                            | resolved footnote body                         | Def → footnote body; Refs → every `[^foot]` consumer          | "Inline footnote"                                    | rename footnote labels                                     |

### 4.9 Embedded Language Support

Code fences with a recognized language tag (`rust`, `ts`, `python`, `bash`, `yaml`, `json`, `toml`, `mermaid`, …) are first-class authoring surfaces. The LSP should provide tiered support without becoming a polyglot server itself.

| Tier  | Strategy                                                                                                                                 | Scope                                                                                                              |
|-------|------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------|
| **0** | Treat the fence body as opaque text. Highlight via `syntect` semantic tokens; fold the fence; surface the language tag in hover.         | Default. Always available.                                                                                         |
| **1** | Parse the body with the embedded grammar (e.g., `serde_yaml_ng` for `yaml`, `serde_json` for `json`, `toml` for `toml`).                 | Diagnostics for parse errors anchored back to fence-body offsets. Required for `yaml`, `json`, `toml`, `json5`.    |
| **2** | Forward LSP requests to a sidecar language server keyed by language tag, translating ranges between virtual document and host document. | Optional, opt-in via `darkmatter.embedded.<lang>.server`. Driven by `workspace/configuration` and dynamic registration.|

`mermaid` fences should additionally surface a code lens to render the diagram via `executeCommand: darkmatter.previewMermaid`, which delegates to `biscuit-visualized`.

### 4.10 Cleanup & Normalization (Inline Post)

| Feature                                 | Capability                                                                                               |
|-----------------------------------------|----------------------------------------------------------------------------------------------------------|
| Cleanup (spacing, tables, list markers) | `textDocument/formatting`, `rangeFormatting`, `executeCommand: darkmatter.cleanup`.                      |
| Normalization (heading levels)          | `executeCommand: darkmatter.normalizeHeadings`, source action surfaced in code actions.                  |
| Compose preview                         | `executeCommand: darkmatter.composePreview` returning the fully-composed text + `ComposeReport` summary. |
| Migrate deprecated syntax               | `source.fixAll.darkmatter.migrate` source action that rewrites superseded directive forms in place.       |

## 5. Cross-Cutting Requirements

### 5.1 Performance / Latency Budgets

- Every DSL feature above must keep keystroke latency under ~50 ms on typical documents. This rules out a pure batch recompose on every `didChange`.
- The server should cache parse/AST/effective-state per URI and invalidate transitively along the dependency graph. Cache hits should also propagate to inlay hints (`Cache hit` lens) and shell directives (skip execution if hash matches and policy permits).
- Long operations (`::url`, `::shell`, full graph rebuild) must be reported via `window/workDoneProgress` so the editor stays responsive.

### 5.2 Configuration Surface

The server needs editor-side configuration for:

- `darkmatter.schemas` — per-glob `$schema` defaults
- `darkmatter.shell.policyPath` — where to load whitelist/blacklist/aliases
- `darkmatter.shell.allowTimeout` — boolean (matches `--allow-shell-timeout`)
- `darkmatter.frontmatterOverrides.allowInvalid` — matches `--allow-invalid-frontmatter-assignment`
- `darkmatter.frontmatterOverrides.allowReassigned` — matches `--allow-reassigned-frontmatter-property`
- `darkmatter.cache.directory`
- `darkmatter.inlay.{interpolation, ctx, env, transclusion, shell, pageBlock}` — per-category toggles
- `darkmatter.env.allowList` / `darkmatter.env.secretPatterns` — control which `env.*` values are exposed in hovers/inlay hints
- `darkmatter.compose.failFast`
- `darkmatter.transclusion.maxDepth`

### 5.3 Security Posture

- `::shell`, `::shell-block`, and frontmatter `$(…)` MUST never execute silently from the LSP. Default mode is "preview only"; execution requires explicit user approval surfaced via `window/showMessageRequest` or a code action.
- `env.*` values shown in hovers/inlay hints MUST be filterable via secret-name patterns (e.g., `*_TOKEN`, `*_KEY`, `*_SECRET`).
- `::url` fetches MUST honour an allowlist of hosts and a kill-switch in configuration.

### 5.4 Diagnostics Anchoring

Every diagnostic must carry a precise byte range that can be translated to LSP `Range`. Concrete anchoring requirements:

- Frontmatter schema errors → byte range of the offending key/value pair inside the YAML block (achievable via a position-aware YAML parse pass keyed by the JSON Pointer that `jsonschema` returns).
- Expression errors → byte range from `{{` to `}}` plus a sub-range when a single token is at fault.
- Directive errors → range covering only the offending option (`when=…`) when possible, falling back to the full directive line.
- Block-pair errors → range from the unmatched opening to end-of-document, with a "related information" link to the unmatched closer (or vice versa).
- Transclusion errors → range covering only the path string, with related information for cycle ancestry chains.

### 5.5 Telemetry / Reporting

The `ComposeReport` already produced by the library should be projected into the editor UI:

- A `darkmatter.composePreview` execute-command result returns the report alongside composed text.
- A `Compose Status` virtual document (or webview) shows aggregate counts (`replacements_applied`, `interpolations_applied`, `transclusions_applied`, `shell_expansions_applied`, …) and warnings.

### 5.6 Cancellation & Long-Running Work

LSP requests can be cancelled by the client at any time via `$/cancelRequest`. Every async path must honour cancellation:

- Compose runs check a `CancellationToken` between phases and between transclusion siblings.
- `::url` fetches are aborted via `reqwest`'s cancellation; partial bodies are dropped.
- `::shell` / `::shell-block` processes are killed (process-group level) when the request that initiated them is cancelled.
- Cancelled requests return `RequestCancelled` (`-32800`); they do not surface as user-visible errors.
- Cached results from a cancelled-but-completed compose are still committed to the cache so the next request benefits.

### 5.7 Capability Negotiation Strategy

The server should not advertise everything statically. Use dynamic registration to keep the editor's UI honest:

- Static capabilities: textDocumentSync, hover, completion, definition, references, document/workspace symbols, formatting, folding, semantic tokens, document links, code lens, inlay hints.
- Dynamically registered: `executeCommand` entries gated on configuration (e.g., `darkmatter.runShellNow` only when a shell policy is bound), schema-driven completion when a `$schema` resolves, embedded-language delegation when a sidecar server is configured.
- Re-evaluate registration on `workspace/didChangeConfiguration` and on `workspace/didChangeWatchedFiles` for policy/schema files.

## 6. Capability Summary (initialize result)

Concretely, the server should advertise the following in its `InitializeResult.capabilities`:

```json
{
  "textDocumentSync": { "openClose": true, "change": "Incremental", "save": { "includeText": false } },
  "completionProvider": { "triggerCharacters": ["{", ".", ":", "=", " ", "$", "(", ","], "resolveProvider": true },
  "signatureHelpProvider": { "triggerCharacters": ["(", ","], "retriggerCharacters": [","] },
  "hoverProvider": true,
  "definitionProvider": true,
  "declarationProvider": true,
  "referencesProvider": true,
  "renameProvider": { "prepareProvider": true },
  "documentSymbolProvider": true,
  "workspaceSymbolProvider": true,
  "documentLinkProvider": { "resolveProvider": true },
  "codeLensProvider": { "resolveProvider": true },
  "inlayHintProvider": { "resolveProvider": true },
  "codeActionProvider": {
    "codeActionKinds": [
      "quickfix",
      "refactor.extract",
      "refactor.inline",
      "refactor.rewrite",
      "source.fixAll.darkmatter",
      "source.fixAll.darkmatter.migrate",
      "source.organizeImports"
    ],
    "resolveProvider": true
  },
  "documentFormattingProvider": true,
  "documentRangeFormattingProvider": true,
  "documentOnTypeFormattingProvider": { "firstTriggerCharacter": "{", "moreTriggerCharacter": ["\n", "}", " "] },
  "foldingRangeProvider": true,
  "documentHighlightProvider": true,
  "linkedEditingRangeProvider": true,
  "selectionRangeProvider": true,
  "semanticTokensProvider": { "legend": { "...": "..." }, "full": true, "range": true },
  "diagnosticProvider": { "interFileDependencies": true, "workspaceDiagnostics": true },
  "executeCommandProvider": {
    "commands": [
      "darkmatter.composePreview",
      "darkmatter.renderTransclusion",
      "darkmatter.approveShellCommand",
      "darkmatter.runShellNow",
      "darkmatter.invalidateCache",
      "darkmatter.cleanup",
      "darkmatter.normalizeHeadings",
      "darkmatter.graph",
      "darkmatter.toggleInlayCategory",
      "darkmatter.previewMermaid",
      "darkmatter.migrateSyntax"
    ]
  },
  "workspace": {
    "workspaceFolders": { "supported": true, "changeNotifications": true },
    "fileOperations": {
      "willRename": { "filters": [{ "pattern": { "glob": "**/*.md" } }] },
      "didRename": { "filters": [{ "pattern": { "glob": "**/*.md" } }] }
    }
  }
}
```

## 7. Implementation Priority

Mapping the [generic Markdown-DSL high-value features](../../../.claude/skills/lsp/features.md#high-value-lsp-features-for-a-markdown-derived-language) (priorities 1–11) onto Darkmatter's surface, and grouping into delivery tiers:

| Tier | Goal                       | Capabilities                                                                                                                                                                                                                                                                                            | Maps to features.md priority |
|------|----------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------|
| **P0** | Authoring is not painful | Diagnostics (frontmatter parse, expression parse, directive parse, transclusion path missing, cycle, depth); document symbols; folding; semantic tokens; path completion; hover (resolved value, target path); document links.                                                                          | 1, 6, 7, 8, 9                |
| **P1** | Authoring is correct     | Schema-aware frontmatter completion + validation; full interpolation completion (FM/ctx/env paths, helper signatures); directive option-key completion; condition DSL completion; quick fixes (missing required keys, unknown option, broken path); rename across workspace; references.                | 2, 3, 4, 10, 11              |
| **P2** | Authoring is delightful  | Inlay hints (resolved values, transclusion outcomes, shell stdout); code lens (compose preview, run/approve, cache state); compose preview virtual document; mermaid preview; code actions for refactors (extract/inline transclusion, convert directive forms); migrate-deprecated-syntax source action.| (extends 4, 8, 10)           |
| **P3** | Server is production-grade | Full cancellation; dynamic capability registration; embedded-language delegation tier 2; partial-result streaming for workspace symbols/references; pull diagnostics; progress for long composes; secret-pattern masking; URL allowlist enforcement.                                                    | (cross-cutting)              |

This sequencing lets the server be useful after P0, schema-aware after P1, and only then takes on the heavier UX/refactor and infrastructure work.

## 8. Out of Scope (Explicit Non-Goals)

The following LSP features from `features.md` are deliberately not part of the Darkmatter LSP surface, with rationale:

| Feature                          | Rationale                                                                                                                                       |
|----------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------|
| Go to type definition            | The DSL has no static type system distinct from value resolution; hover already shows the inferred type of an expression.                       |
| Go to implementation             | No interface/trait equivalent. The closest analogue (transclusion) is already covered by definition + references.                               |
| Type hierarchy                   | No type system to walk.                                                                                                                         |
| Call hierarchy                   | Replaced by the explicit `darkmatter.graph` dependency-graph view, which is a richer fit for transclusion-style "calls".                        |
| Inline values                    | Debugger-only feature; documents are not stepped through.                                                                                       |
| Monikers                         | Cross-tool symbol indexing (LSIF/SCIP) is not currently a target; revisit if Darkmatter content is ever indexed by Sourcegraph-like tools.      |
| Notebook documents               | Darkmatter is file-oriented; no notebook host is planned.                                                                                       |
| Auto-imports                     | There is nothing analogous to import; closest equivalent ("auto-add missing FM key on undefined `{{ x }}`") is delivered as a quick-fix instead.|
| Organize imports (literal)       | Repurposed: `source.organizeImports` is reserved for "sort frontmatter keys / consolidate `replace:` map" if the user opts in.                  |

## 9. Gaps & Open Questions

These items are not yet decided and warrant follow-up before implementation begins:

1. **Schema discovery rules.** Should `$schema` in frontmatter take precedence over editor-level `darkmatter.schemas` glob mappings, or vice versa? What is the precedence relative to the JSON Schema `$ref` registry?
2. **Approval persistence.** Are approved shell commands cached per-workspace, per-user, or both? How does this map onto the existing `compose::cache` module?
3. **Environment variable exposure.** Should the LSP read `env.*` directly, or proxy through a per-workspace allowlist file? Default secret patterns?
4. **`::url` semantics.** Is `::url` always cached? Does the LSP fetch on `didOpen`, on demand, or only on explicit user action?
5. **Permissive-flag UX.** Should `--allow-invalid-frontmatter-assignment` and `--allow-reassigned-frontmatter-property` be per-document directives, per-workspace settings, or both?
6. **Multi-root workspaces.** How are dependency graphs scoped across workspace folders? Are `::file` references allowed to cross folder boundaries?
7. **Live preview.** Is there an editor-agnostic mechanism (e.g., a `darkmatter.composePreview` virtual document URI) that all clients can render, or do we ship per-editor preview UI?
8. **Telemetry surface.** Should `ComposeReport` warnings appear as diagnostics, as a separate virtual document, or both?

These questions feed directly into the next planning phase and should be resolved before the protocol-level design (which library, which graph layout, etc.) is locked in.
