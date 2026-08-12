---
prompt: "Do deep research into what features a modern LSP (language server protocol) for Markdown should implement.\n\n- Start by just creating a list of features (describing each one and making sure to describe what value/utility this feature has to the user)\n- Then once you're convinced you have a full list of features go throug a grouping/categorization exercise to organize these features:\n\n    - for each group, summarize the group's focus\n    - then list each member in the group along with their decription\n        - if you see any inconsistencies in quality, completeness, or language style of these descriptions then you should update it to improve the description\n- Now as a final step, iterate through each list item in each group and add: \n    - an example of how this feature might be functionally implemented (we're not looking for code but rather a rich description of how the user might experience this feature, along with any nuance to the feature and variances that should be considered)\n    - then list the parts of the LSP API surface that would be used to implement this feature\n    - then list any technical complexities which might make this feature challenging"
agent: Opencode
model: zai-coding-plan/glm-5.2
last_updated: 2026-07-04
hash: 6e4fea55641415d2-94c3463c9e975032
---

# Modern Markdown Language Server — Feature Research

## Introduction

Markdown occupies an unusual position in the tooling ecosystem. It is simultaneously (a) a prose format authored by non-programmers (READMEs, docs, notes), (b) a structured document format with a parseable grammar (CommonMark, GFM), (c) a structured-data container via frontmatter, (d) a hypertext format with links, anchors, includes, and transclusions, and (e) a host for embedded languages (YAML, TOML, JSON, Mermaid, code fences in dozens of languages). A modern Markdown language server must serve all five roles at once, and the feature set below reflects that breadth.

The reference implementations — VS Code's `vscode-markdown-languageservice`, Marksman, markdown-oxide, IWE, and Markmark — each cover a subset. The goal of this document is to enumerate the *complete* feature surface a modern Markdown LSP should consider, organized so that implementers can prioritize, scope, and identify the hard parts up front.


## Categorized Features

### Group 1 — Navigation & Cross-Reference Discovery

**Group focus.** Turning Markdown from flat text into a navigable hypertext graph. These features answer "where does this go?" and "what points here?" — the foundational moves that distinguish a language server from a syntax highlighter. Every other group depends on the document and workspace symbol indexes these features establish.

#### 1.1 Go to Definition

**Description & value.** Jumps from a Markdown link, wiki-link, anchor reference, block reference, link-definition use, or frontmatter `$schema`/`file(eager)` reference to its resolved target. The target may be in the same file (anchor), a different file in the workspace, or an external URL (opened in a browser). Value: readers and authors navigate documents as a graph rather than as scrollable text.

**Example user experience.** With the cursor inside `[setup guide](install.md#prerequisites)`, invoking Go to Definition opens `install.md` and places the cursor on the `## Prerequisites` heading. On a wiki-link `[[architecture]], the jump targets`architecture.md`if it exists, or`architecture/overview.md`if the project uses directory-per-note conventions, or offers a disambiguation menu if multiple candidates match. On a`[ref][1]`use, the jump targets the`[1]: https://…`definition line within the same file. Nuances: ambiguous targets (two files named`index.md`) require a location list; external `https://\` URLs are typically delegated to the OS browser rather than the editor; broken links should fall through to a "Create file" code action rather than silently failing.

**LSP API surface.** `textDocument/definition`; `textDocument/declaration` and `textDocument/typeDefinition` for symmetry; `Definition` / `DefinitionLink` result types; `Location` with `Range`; client capability `textDocument.definition.linkSupport`.

**Technical complexities.** Resolving a link requires unifying four syntaxes (inline, reference, wiki-link, block-ref) into a single "link target" model. Anchor slug generation must match the renderer (GitHub uses one algorithm; CommonMark another; Obsidian preserves case). Path resolution must honor project root, relative-to-file vs. relative-to-root conventions, and directory-index modes (`foo/` → `foo/index.md` or `foo/readme.md`). Ambiguous targets need ranking heuristics. External URL handling differs per client.

#### 1.2 Find References

**Description & value.** Locates every reference to a heading anchor, link definition, file, wiki-link, or block-id across the open file or the whole workspace. Value: impact analysis before edits; the engine that powers safe rename and backlink views.

**Example user experience.** With the cursor on `## Authentication`, Find References returns every `[[auth]]`, `[auth](#authentication)`, `[…](auth.md#authentication)`, and `[^auth]` across the vault, grouped by file. Selecting one jumps to the exact reference span. Variance: reference searches can be scoped to the open document, open files, or the entire workspace; some users want "references to this heading by name anywhere" even when the anchor slug differs; reference results should distinguish definitional occurrences (the heading itself) from referential ones.

**LSP API surface.** `textDocument/references` with `ReferenceParams.context.includeDeclaration`; `Location[]` result; `workDoneToken` and `partialResultToken` for large workspaces.

**Technical complexities.** Requires a workspace-wide index mapping every anchor/file/block-id to its set of referring spans. Anchor text and link display text often differ, so reference matching must be slug-based, not literal. The index must update incrementally as files change. Cross-workspace references (a vault that imports snippets from another folder) need explicit workspace-folder configuration.

#### 1.3 Document Symbols (Outline)

**Description & value.** Returns the document's heading hierarchy (and optionally frontmatter keys, directive blocks, and defined terms) as a tree the editor renders in its outline panel, breadcrumbs, and minimap. Value: orientation in long documents; the structural backbone most other features hang from.

**Example user experience.** A docs file with three H2s and nested H3s appears in VS Code's Outline view as a collapsible tree; clicking an entry scrolls to the heading. Breadcrumbs show `File › Section › Subsection` as the cursor moves. Nuances: setext headings (`Title\n===`) must produce the same symbol as ATX (`# Title`); frontmatter may appear as a top-level symbol or be hidden per user preference; code fences and directive blocks may be represented as leaf symbols or suppressed; list items are usually too granular and are omitted unless the user opts in.

**LSP API surface.** `textDocument/documentSymbol` returning `DocumentSymbol[]` (hierarchical) with `SymbolKind.String`/`Namespace`/`Field`; client capability `hierarchicalDocumentSymbolSupport`; optionally `textDocument/foldingRange` for the fold-all-but-sections editor affordance.

**Technical complexities.** CommonMark allows heading levels to skip (H1 → H3), so the tree must either promote or insert synthetic parents. Setext and ATX equivalence must be normalized. Lazy continuation lines and heading attributes (`## Title {#custom-id}`) require custom-id extraction. Editors vary in symbol-kind rendering, so a Markdown heading is conventionally `String` but some servers use `Function` or `Module` to differentiate frontmatter and directive symbols.

#### 1.4 Workspace Symbols

**Description & value.** Searches headings, frontmatter titles, defined terms, and (optionally) link labels across all workspace documents, returning matches ranked by relevance. Value: jump-to-anywhere over an entire vault or docs site without remembering paths.

**Example user experience.** Typing `auth` in the editor's "Go to Symbol in Workspace" palette surfaces every `## Authentication`, `# Authorize`, and frontmatter `title: Authorize your account` across the project, with file paths and the matching heading snippet. Fuzzy/subsequence matching is expected. Variance: workspace symbols may include link-definition labels, defined abbreviations, and even code-fence titles (```` ```rust title="example"````); very large vaults need result count caps and async streaming.

**LSP API surface.** `workspace/symbol` with `WorkspaceSymbolParams.query`; `WorkspaceSymbol[]` (or `WorkspaceSymbol3.17` with `Location`/`location` flexibility); `workspace/symbol/resolve` for lazy detail.

**Technical complexities.** Requires a workspace-wide symbol index, typically persisted to disk. Subsequence matching (Marksman's approach) or trigram indexes are common. The index must invalidate per-file on change. Cross-folder disambiguation (`index.md` in 12 directories) needs the symbol result to carry enough path context. The 3.17 `WorkspaceSymbol` with deferred `resolve` avoids sending full `Location`s eagerly.

#### 1.5 Backlinks (Reverse References)

**Description & value.** A dedicated view of every document and heading that references the current document or heading — the Obsidian/Roam feature. Distinct from Find References in that backlinks are presented as a persistent panel (or code lens on headings) rather than a transient result list. Value: surfaces the implicit graph of "what depends on me" that flat text hides.

**Example user experience.** An Obsidian-style backlinks panel under the editor lists "Referenced in: `design.md` (3), `meeting-notes/2026-06.md` (1)", each expandable to show the surrounding paragraph. Code-lens variant: each heading shows `3 backlinks` inline; clicking opens the list. Nuances: backlinks must be filterable by "unresolved references" (broken backlinks where the user typed the link before creating the file); aliases must be honored (a link to `[[auth]]` resolves as a backlink to `Authentication.md` when `aliases: [auth]` is set); some users want "forward links" mirrored in the same panel.

**LSP API surface.** `textDocument/references` over the document URI (treating the document itself as the referenced symbol); `textDocument/codeLens` to render per-heading backlink counts; command-based custom views via `workspace/executeCommand` for the panel form; `window/workDoneProgress` while the initial index builds.

**Technical complexities.** Backlinks are conceptually the inverse of the reference graph but must update in near real time as the user types. The reverse index must survive file renames and alias changes. Code-lens-based backlink counts can thrash on every keystroke, so they should be debounced and computed against the persistent index. Custom panels require a client-side extension; pure-LSP servers expose the data via `textDocument/references` and let the editor render it.

#### 1.6 Document Highlights

**Description & value.** When the cursor rests on a heading, link, anchor, or term, highlights every other occurrence of the same logical entity in the current document. Value: instant visual scan of "where else does this appear" without invoking search.

**Example user experience.** Cursor on `## Authentication` subtly underlines the `[[authentication]]` wiki-link, the `[login](#authentication)` link, and the `## See Also: Authentication` reference. Click types are distinguished: `Read` (a normal reference), `Write` (the heading definition itself), `Text` (mention by literal text). Variance: some users want string-equality highlights (every occurrence of the word "authentication"); others want strict semantic highlights (only resolved references to *this* heading). Editors render the three kinds with different styles.

**LSP API surface.** `textDocument/documentHighlight` returning `DocumentHighlight[]` with `DocumentHighlightKind` (`Read`, `Write`, `Text`).

**Technical complexities.** Strict semantic matching requires resolving every potential occurrence to its target and filtering; cheap string matching produces false positives. CommonMark link labels are case-insensitive but reference IDs may not be, complicating equality. Heading text duplication (two `## Setup` sections) must be disambiguated by anchor.

---

### Group 2 — Diagnostics & Validation

**Group focus.** Telling the author what is wrong, suspect, or non-idiomatic. Diagnostics are the highest-value feature for any Markdown LSP because the format is so permissive that errors silently degrade rendering. The members of this group differ chiefly in *where* the authority comes from — the CommonMark/GFM spec, a frontmatter schema, a prose-style rule set, an embedded language, or a custom directive grammar.

#### 2.1 Link Validation

**Description & value.** Detects broken internal links (target file missing), dead anchors (target heading absent), unresolved wiki-links, missing image assets, and optionally unreachable external URLs. Value: documentation that doesn't 404; cross-references that resolve.

**Example user experience.** `[API](api.md#endpoints)` is underlined red with "Anchor 'endpoints' not found in api.md (did you mean: 'endpoint', 'endpoints-list'?)" when the heading was renamed to `## Endpoint List`. Missing files suggest a "Create file" code action. External links are validated lazily in the background with a configurable allowlist and timeout; dead external links show as warnings, not errors. Nuances: relative-vs-absolute path conventions, case-insensitive filesystems (macOS) vs. case-sensitive (Linux) require normalization; directory-index links (`./install/`) resolve per project config; anchor slug rules must match the target renderer.

**LSP API surface.** `textDocument/publishDiagnostics` (push) or `textDocument/diagnostic` + `workspace/diagnostic` (pull, 3.17); `Diagnostic` with `severity`, `source: "markdown-link"`, `code`, `relatedInformation` (the target file), `data` for round-tripping to code actions; `DiagnosticTag.Unnecessary` for redundant links.

**Technical complexities.** File existence checks must be cache-backed and invalidated by file watchers. Anchor validation requires parsing the target document's headings and computing slugs under the target renderer's algorithm. External URL validation needs HTTP HEAD requests with retries, rate limiting, redirect handling, and a separate low-priority task pool so it never blocks interactive features. Path normalization must respect `.gitignore`-style exclusions and per-project ignore files.

#### 2.2 Heading Hierarchy Diagnostics

**Description & value.** Flags duplicate headings (which produce ambiguous anchors), skipped levels (H1 → H3), missing H1, multiple H1s, and overly deep nesting. Value: predictable anchor generation, accessibility, and clean auto-generated TOCs.

**Example user experience.** A document with `# Title` followed by `### Subsection` shows a warning: "Heading level 3 skipped from 1 — heading levels should not increase by more than one." Two `## Setup` sections show: "Duplicate heading — anchors will collide; rename or add a custom `{#id}`." Nuances: documentation generators (mkdocs, Docusaurus) have their own conventions; some users want strict mode that treats every violation as an error; project config may allow multiple H1s (one per included file) or forbid them.

**LSP API surface.** `textDocument/publishDiagnostics`; `Diagnostic.code` for rule identifiers (`md003-title-style`, `md001-heading-increment`); `DiagnosticTag.Deprecated` for old heading styles (setext under ATX-only conventions).

**Technical complexities.** Rule sets (markdownlint MD001/MD003/MD025/MD043, remark-lint rules) overlap and conflict — the server must present a coherent rule registry, not 50 ad-hoc checks. Custom heading IDs (`{#foo}`) must be parsed before duplicate detection. Setext headings must be normalized to ATX levels. Inclusion/transclusion-aware diagnostics require resolving the *effective* heading sequence after composition, not the raw file's sequence.

#### 2.3 Frontmatter Schema Validation

**Description & value.** Validates YAML, TOML, or JSON frontmatter against a JSON Schema declared via `$schema:` frontmatter, a project config, or a baseline. Reports type mismatches, missing required fields, out-of-range values, and unknown keys. Value: elevates frontmatter from free text to typed data with a contract.

**Example user experience.** In a project where `$schema: ./schemas/post.json` declares `title: string`, `date: date`, `tags: string[]` required, omitting `date` shows: "Missing required property: date." Typing `tags: "rust"` instead of an array shows: "Expected array, got string." Hover over the `tags` key reveals the schema description and allowed enum values. Nuances: schema can be inlined in frontmatter, referenced by file path, or set globally; multiple schemas may apply via glob patterns; schema drafts differ (Draft 2020-12 vs. Draft 7); coercion (`"true"` → `true`) is desirable but opt-in; the validation should run before shell expansion and again after, to catch both static and dynamic errors.

**LSP API surface.** `textDocument/publishDiagnostics` with `source: "json-schema"`; `textDocument/hover` for schema field docs; `textDocument/completion` for schema-aware key/value suggestions; `textDocument/codeAction` for "Add missing required fields"; embedded-document delegation (`textDocument/diagnostic` on the frontmatter virtual document) is an alternative pattern.

**Technical complexities.** The frontmatter must be extracted, parsed into a position-aware tree (not just a `serde_json::Value`), validated against the compiled schema, and then validation errors — which arrive as JSON Pointers like `/tags/0` — must be mapped back to precise byte offsets in the source. This requires a position-aware YAML/TOML/JSON parser (or a Tree-sitter grammar) layered under the schema validator. Schema caching, schema fetch policy for remote `$ref`s, and Draft compatibility all matter.

#### 2.4 Markdown Syntax Diagnostics

**Description & value.** Detects unclosed fenced code blocks, malformed tables, broken emphasis (`*unclosed`), ambiguous list nesting, hard line breaks where soft was intended, HTML block boundaries that don't match, and other CommonMark/GFM violations the renderer would silently reinterpret. Value: catches authoring errors the renderer would silently misinterpret.

**Example user experience.** An unclosed ```` ```rust ```` at EOF shows "Fenced code block is never closed" with a quick-fix to append ```` ``` ````. A table row with the wrong column count shows "Row has 4 cells; table header has 3". A `1. item\n2. item\n5. item` list shows "List item numbering skipped". Nuances: GFM permits constructs CommonMark forbids (and vice versa); the active dialect must be detected per project; some "errors" are stylistic (ATX vs. setext) and belong behind a lint rule, not a syntax check.

**LSP API surface.** `textDocument/publishDiagnostics`; `Diagnostic.code` keyed to a stable rule taxonomy; `Diagnostic.relatedInformation` for the matching opening fence; code actions paired with each diagnostic.

**Technical complexities.** The parser itself produces some of these as errors; the LSP must translate parser events into LSP diagnostics with stable codes. Tables and task lists are GFM extensions and need a parser that supports them. Diagnostics must survive incremental edits — a stale diagnostic on a now-closed fence is worse than none.

#### 2.5 Embedded Language Validation

**Description & value.** Validates the content of fenced code blocks by delegating to the embedded language's linters or language server. A Rust fence is checked by rustc's parsing rules; a JSON fence by a JSON validator; a YAML frontmatter-like block by a YAML validator; a Mermaid block by a Mermaid parser. Value: a snippet in docs is checked like real code.

**Example user experience.** A ```` ```json ```` block with a trailing comma shows a diagnostic at the comma. A ```` ```rust ```` block with an unmatched brace shows a syntax error from rust-analyzer. A ```` ```mermaid ```` block with an invalid arrow shows "Unknown diagram syntax". Nuances: delegation is expensive and should be opt-in per language; syntax-only checks are cheap, full lint is slow; some embedded servers (rust-analyzer) need a workspace context to be useful; diagnostics must be offset-shifted back into the fence's source position.

**LSP API surface.** Custom `workspace/executeCommand` to drive an embedded server out-of-band; or virtual-document pattern (`workspace/didOpen` on a synthetic URI for the fence region, then `textDocument/publishDiagnostics` relayed with offset shifts); `textDocument/codeAction` for "Format this block" / "Run embedded linter".

**Technical complexities.** The LSP has no native "embedded language" abstraction; implementations fake it with virtual documents (synthetic URIs like `darkmatter://fence/file.md#L12-L30`) and translate diagnostics back. Each embedded language needs its own delegation strategy. Markdown-oxide, Marksman, and the VS Code server all skip this; IWE experiments with it. Caching is essential — re-running rust-analyzer on every keystroke for a doc snippet is unworkable.

#### 2.6 Style & Prose Linting

**Description & value.** Line length, trailing whitespace, multiple consecutive blank lines, terminology consistency, biased or exclusionary language (alex, write-good), readability, sentence length, passive voice, and other prose concerns. Value: consistent voice, accessibility, and inclusive documentation.

**Example user experience.** "Click here" is flagged with "Avoid non-descriptive link text." "Master/slave" is flagged with "Consider inclusive terminology (primary/replica)." A 200-word sentence is flagged with "Sentence length 198; consider breaking up." Markdownlint-style MD013 (line length) optionally enforces a soft wrap column. Nuances: prose lints are deeply personal and project-specific; the server must read rule config (markdownlint's `.markdownlint.json`, remark-lint's `.remarkrc`, or its own config) and let users disable rules per-file via comments. Spell-check is typically delegated to a dedicated checker (cspell, codespell) rather than reimplemented.

**LSP API surface.** `textDocument/publishDiagnostics`; `textDocument/codeAction` for "Disable this rule" / "Disable for this line" (writing `<!-- markdownlint-disable -->`); `workspace/configuration` to read rule files.

**Technical complexities.** Prose lint tools are typically separate processes (markdownlint-cli2, alex, write-good, cspell); the LSP must either embed them, spawn them, or reimplement them. Natural-language analysis is far slower than syntactic checks; these lints should run on save or debounced, not on every keystroke. Rule sets overlap (MD013 vs. prettier's printWidth) and must be reconciled.

#### 2.7 Custom Directive Validation

**Description & value.** Validates extended-syntax directives: GFM admonitions (`:::note`), Obsidian callouts (`> [!warning]`), `::disclosure`/`::details`/`::end-disclosure` (Darkmatter), `:::mermaid` blocks, Asciidoctor-style blocks, and project-specific directives. Value: catches malformed extensions the parser would silently ignore or render as plain text.

**Example user experience.** A `:::note` without a closing `:::` shows "Directive ':::note' is never closed." A `> [!warning]` callout in a project that doesn't enable callouts shows "Unknown callout type — did you mean 'warn'?" A `::file` transclusion pointing to a non-existent file shows a broken-link diagnostic. Nuances: directive grammars are project-specific and often user-defined; the server must read a directive registry (project config) to know which directives are valid, what attributes they accept, and whether they nest.

**LSP API surface.** `textDocument/publishDiagnostics`; `Diagnostic.code` namespaced by directive (`directive.unclosed`, `directive.unknown-kind`); `textDocument/codeAction` for "Close directive" / "Convert to standard Markdown".

**Technical complexities.** Directives are not part of CommonMark; the parser must be extended (pulldown-cmark supports custom events; markdown-it accepts plugins). The directive registry must be configurable. Nesting validation requires tracking open/close pairs across block boundaries. Some directives (transclusions, includes) must be validated against the file graph, requiring the same dependency machinery as link validation.

#### 2.8 Reference Definition Diagnostics

**Description & value.** Detects unused link reference definitions (`[unused]: …` that nothing references), duplicate definitions (`[foo]: a` and `[foo]: b` — first wins per CommonMark), and shadowed definitions. Value: tidy source, predictable link resolution, smaller diffs.

**Example user experience.** A `[old-link]: https://example.com` that was once used but no longer is shows as faded (unused) with a "Remove unused definition" quick-fix. Two `[ref]:` definitions show: "Duplicate reference definition — the first will be used." Nuances: a definition used only in another transcluded file is not "unused" in the composed document; transclusion-aware analysis changes the answer. Some style guides require definitions to be sorted; others forbid inline links entirely.

**LSP API surface.** `textDocument/publishDiagnostics` with `DiagnosticTag.Unnecessary` for unused; `textDocument/documentHighlight` to show the definition paired with its usages; `textDocument/codeAction` for "Convert to inline link" / "Sort definitions".

**Technical complexities.** Reference resolution is order-sensitive in CommonMark (first definition wins for a given label); the analyzer must replicate that semantics. "Unused" must be evaluated against the effective document after transclusion, not the raw file, or it will produce false positives in composed documents.

---

### Group 3 — Completion & Authoring Assistance

**Group focus.** Helping the author write the next token correctly. These features reduce typing, eliminate path/anchor typos, and surface the project's vocabulary (schemas, tags, snippets) at the point of insertion. They are the second-highest-value group after diagnostics because they convert every diagnostic into a prevention opportunity.

#### 3.1 Path & File Completion

**Description & value.** Autocompletes relative and absolute file paths inside link, image, and include syntax, filtered by what exists on disk. Value: never mistype a filename; discover what's available without leaving the editor.

**Example user experience.** Typing `](./` inside a link pops up the directory listing filtered to `.md`, `.mdx`, and configured extensions, with folders shown for drill-down. Typing `](../auth` filters to files starting with "auth" in the parent directory. Image syntax `![alt](` shows image files preferentially and offers to insert alt text from the filename. Nuances: completion must honor the project's link-style convention (relative-to-file vs. relative-to-root); `.gitignore`'d files should be excluded; completion may suggest `index.md` when the user types `./folder/`.

**LSP API surface.** `textDocument/completion` with `CompletionContext.triggerCharacter` (`/`, `(`, `[`); `CompletionItemKind.File` / `Folder`; `CompletionItem.textEdit` to insert the relative path; `commitCharacters` for path separators; client capability `dynamicRegistration` to register the trigger characters.

**Technical complexities.** Path completion requires reading directories on demand and caching them. Path normalization (POSIX vs. Windows backslashes) must round-trip cleanly. The "root" for relative paths must be inferred from the project type (Obsidian uses vault root; mkdocs uses `docs/`; pure CommonMark uses the file's directory). Completion must be debounced so it doesn't enumerate huge directories on every keystroke.

#### 3.2 Heading & Anchor Completion

**Description & value.** Autocompletes `#anchor` fragments in `[text](file#…)` and in-page links by enumerating the target file's headings (or the current file's headings if no file part is given). Value: discoverable in-page navigation without manually computing slug names.

**Example user experience.** Typing `](#` lists every heading in the current document as `# Setup`, `# Configuration`, `# Troubleshooting`, with the rendered heading text and the slug side-by-side. Typing `](other.md#` lists headings from `other.md`. Selecting one inserts the canonical slug. Nuances: slug casing and punctuation rules differ between renderers (GitHub lowercases and strips punctuation; Obsidian preserves spaces as `%20`); the server must know the active renderer. Custom heading IDs (`{#custom}`) override the slugified form and must appear in completion.

**LSP API surface.** `textDocument/completion` returning `CompletionItemKind.Reference`; `insertText` with the slug; `detail` showing the heading's source text; `documentation` showing the heading's position.

**Technical complexities.** Requires the workspace heading index (shared with Workspace Symbols and Find References). Slug algorithm selection (GitHub, GitLab, CommonMark, Obsidian, custom) is per-project. Completion must be re-resolved when the target file changes mid-typing.

#### 3.3 Wiki-link Completion

**Description & value.** Autocompletes `[[…]]` wiki-link targets, with file names, aliases, and `[[file#heading]]` forms. Value: first-class support for PKM workflows where wiki-links are the primary navigation syntax.

**Example user experience.** Typing `[[au` lists every file whose name, alias, or display title contains "au": `[[Authentication]]`, `[[author-bio]]` (with alias `[[About the Author]]`), `[[Aurora Theme]]`. Selecting one inserts the canonical form. Typing `[[auth#` after a file is resolved lists that file's headings. Variance: link-style conventions (shortest unique path, full path, or relative) must match project config; aliases declared in frontmatter appear as alternative completions; unresolved targets offer a "Create file" completion item.

**LSP API surface.** `textDocument/completion` with trigger characters `[`; `CompletionItemKind.File`; `insertTextFormat: Snippet` for `[[${1:file}]]${2}` cursor placement; `additionalTextEdits` to create the file when the user accepts a "Create" item.

**Technical complexities.** Wiki-link resolution is fuzzy: `[[auth]]` could match `Authentication.md`, `auth/index.md`, or `topics/auth.md`. Ranking must respect project conventions and explicit aliases. Path-style (`[[folder/file]]`) vs. shortest-path (`[[file]]`) modes must both work. File creation must populate the new file with a template.

#### 3.4 Frontmatter Completion (Schema-Aware)

**Description & value.** Suggests valid frontmatter keys, enum values, defaults, and required fields based on the active JSON Schema. Value: frontmatter becomes self-documenting; authors don't need to consult the schema externally.

**Example user experience.** In an empty frontmatter block, completion offers the schema's top-level keys: `title`, `date`, `tags`, `draft`. Typing `tags:` and starting `[` offers previously-used tags. Typing `status:` offers the enum `draft | published | archived`. The completion item's `documentation` shows the schema description, type, default, and example. Nuances: required fields should be flagged in completion (e.g., marked with `(required)`); conditional schemas (`if/then/else`) change which keys are valid based on other fields; the server may suggest `aliases:`, `permalink:`, or other built-ins even when not in the schema.

**LSP API surface.** `textDocument/completion` with `CompletionItemKind.Property` / `EnumMember` / `Value`; `completionItem/resolve` to lazily fetch full schema docs; `insertTextFormat: Snippet` for `"${1:key}": ` templates; `textDocument/foldingRange` to scope completion to the frontmatter region.

**Technical complexities.** Schema-aware completion requires walking the JSON Schema to enumerate valid properties at the cursor's JSON Pointer location. Conditional schemas, `oneOf`/`anyOf`, and `$ref` resolution all complicate the enumeration. The schema must be re-fetched when it changes. Frontmatter syntax differs (YAML indent vs. TOML tables vs. JSON braces), so completion items must produce syntax-appropriate text per format.

#### 3.5 Snippet Completion

**Description & value.** Templates for tables, callouts, frontmatter stubs, code fences with language hints, and project-specific boilerplate, expanded via snippet syntax. Value: less boilerplate; consistent structure across a docs site.

**Example user experience.** Typing `table` offers a snippet that expands to a 3-column, 2-row table with cursor placement in the first header cell. Typing `note` in a project with admonitions expands to `:::note\n\n:::`. Typing `frontmatter` expands to a complete YAML stub with all required schema fields. Project-supplied snippets (per `.snippets/` directory or config) appear alongside built-ins. Nuances: snippets must be context-sensitive (a table snippet inside a table cell is wrong); placeholders should tab-stop through editable regions; choice placeholders (`${1|draft,published|}`) make snippet expansion a guided form.

**LSP API surface.** `textDocument/completion` with `CompletionItemKind.Snippet`; `insertTextFormat: Snippet`; `insertText` or `textEdit` containing LSP snippet syntax (tab-stops, placeholders, choices, variables).

**Technical complexities.** Snippet grammar must support the full LSP snippet spec (placeholders, choices, variables, transforms). Project snippets need a discovery mechanism (directory or config) and reload on change. Context-sensitive snippets require the parser to identify the surrounding context (list, table, paragraph).

#### 3.6 Code Fence Language Completion

**Description & value.** Autocompletes the info string after ```` ``` ```` — language identifiers, optional titles, line-highlight ranges, and other fence attributes. Value: correct syntax highlighting and language-server routing without consulting a registry.

**Example user experience.** Typing ```` ```r```` offers `rust`, `ruby`, `r`, `rscript`, `regex`. Typing ```` ```rust title="example"```` triggers attribute completion. Typing ```` ```mermaid flowchart```` offers Mermaid diagram types. The grammar registry (Darkmatter's `LanguageGrammar`) is the authority. Nuances: info strings are extensible (```` ```ts {linenos=table} ````); the server should parse and complete attributes, not just the language token; unknown info strings should offer to register or fall back to plain text.

**LSP API surface.** `textDocument/completion` with `CompletionItemKind.Keyword`; `insertTextFormat: Snippet` for `rust title="${1:Example}"\n\n\`\`\``; the same grammar registry used by the renderer (no duplication).

**Technical complexities.** Info string grammar is per-parser; GFM permits anything after the language token, but specific tools (Docusaurus, mkdocs-Material) define their own attributes. The grammar registry must round-trip with the renderer so the highlighted fence and the completed token agree.

#### 3.7 Tag Completion

**Description & value.** Autocompletes `#tags` in body text and tag-array values in frontmatter, drawn from a workspace-wide tag index. Value: consistent taxonomy across a vault; no `#wip` vs. `#WIP` drift.

**Example user experience.** Typing `#ru` in body text offers `#rust`, `#rust-testing`, `#rust-async` with usage counts. Typing `tags: [ru` in frontmatter offers the same set. Selecting `#rust` standardizes the casing to the most common form across the vault. Nuances: tag syntax differs (`#tag` in Obsidian vs. `tags: [tag]` in frontmatter); both must resolve to the same index. New tags should be offered with a "Create new tag" item. Tags declared in a taxonomy file or config may be enforced.

**LSP API surface.** `textDocument/completion` with `CompletionItemKind.Keyword` / `EnumMember`; `detail` with usage counts; `workspace/configuration` to read taxonomy config; `workspace/symbol`-like queries over the tag index.

**Technical complexities.** The tag index must cover both body `#tag` syntax and frontmatter `tags:` arrays, normalized to a canonical form. The index updates on every file change. Tag casing policy (case-sensitive vs. case-insensitive) must match project convention. Diagnostics for non-canonical casing are a natural extension.

---

### Group 4 — Refactoring & Editing Actions

**Group focus.** Transforming documents safely: rename without breaking links, reformat to house style, extract sections into includes, and migrate between dialects. These are the power-editor features that distinguish an LSP from a static linter. They all funnel through `WorkspaceEdit` and require the cross-file reference graph from Group 1.

#### 4.1 Safe Rename

**Description & value.** Renames a heading (and its anchor), a file, a link definition label, a wiki-link target, or a tag, then updates every reference across the workspace. Value: refactor without breaking the link graph.

**Example user experience.** Renaming `## Setup` to `## Installation` updates every `(#setup)`, `(other.md#setup)`, `[[setup]]`, and `[[other#setup]]` across the workspace to use the new slug. Renaming a file updates every link, wiki-link, image reference, and transclusion pointing to it. Renaming a tag `#wip` to `#draft` updates both body and frontmatter occurrences. Nuances: rename should preserve custom heading IDs (`{#custom}`) when present; cross-file renames need a `WorkspaceEdit` with multiple `TextEdit`s; renames may conflict (two files define `[[setup]]`) and require user confirmation; rename over a transcluded heading may need to update the source file, not the host.

**LSP API surface.** `textDocument/prepareRename` to validate the cursor is on something renameable and to return the range; `textDocument/rename` returning `WorkspaceEdit` with `documentChanges` (preferred over older `changes`); `ChangeAnnotation` (3.16+) to label each edit; `workspace/applyEdit` to push the edit to the client.

**Technical complexities.** Requires the full reference graph and an anchor-slug round-trip (old slug → new slug). Heading text and slug can diverge (custom IDs), so rename must offer both. File rename is not natively an LSP operation — it's a `WorkspaceEdit` that includes `CreateFile`, `RenameFile`, and `DeleteFile` resource operations plus `TextEdit`s on every referring file. The client must support resource operations (capability `workspace.workspaceEdit.resourceOperations`).

#### 4.2 Code Actions & Quick Fixes

**Description & value.** Contextual one-click fixes: "Create missing file" for a broken link, "Convert to reference link", "Add missing frontmatter field", "Fix broken anchor", "Close unclosed directive", "Convert wiki-link to standard link", "Generate TOC here", "Disable this lint rule". Value: converts diagnostics into resolutions; the highest-velocity authoring surface.

**Example user experience.** A lightbulb appears on `[setup](setup.md)` where `setup.md` doesn't exist; the action menu offers "Create file 'setup.md'" with an optional template. On a broken anchor `(install.md#prep)`, "Replace with 'prerequisites'" is offered. On a duplicate heading, "Add custom ID" or "Rename heading". On `*[HTML]: HyperText Markup Language*` once but never used, "Remove unused abbreviation". Nuances: actions should be scoped to the cursor's containing diagnostic or syntactic unit; `codeAction/resolve` lets the client fetch the actual `WorkspaceEdit` lazily after the user picks from the menu (cheap menu, expensive edit).

**LSP API surface.** `textDocument/codeAction` returning `CodeAction[]` with `kind` (`quickfix`, `refactor`, `source`); `codeAction/resolve` for lazy `edit` population; `Command` for actions that require client-side execution; `CodeAction.disabled` to show-but-disable inapplicable actions with an explanation; `isPreferred` to flag the single best fix.

**Technical complexities.** Code actions are the integration surface for *every* other feature — diagnostics, schema validation, embedded language, refactoring. Each contributing subsystem must register its actions coherently. Lazy resolution requires the cheap phase to return metadata and the resolve phase to compute the actual edits. Conflict detection (two actions wanting the same range) and ordering (quick fixes before refactors before sources) follow conventions editors rely on.

#### 4.3 Document Formatting

**Description & value.** Reformats the whole document or a selected range to a configurable style: prettier, mdformat, markdownlint-fix, or a house style. Value: consistent style without manual reflow.

**Example user experience.** Save triggers formatting: prose is rewrapped to the configured print width, list indentation is normalized, code fences get consistent info strings, blank-line spacing collapses to the house rule, and tables are aligned. Range formatting reflows just the selection. Nuances: formatters conflict (prettier and markdownlint disagree on list spacing); the server must either pick one or expose a layering config. Prose rewrapping must respect sentence boundaries and existing hard breaks. Frontmatter formatting is typically delegated to the YAML/TOML/JSON formatter.

**LSP API surface.** `textDocument/formatting` (whole document), `textDocument/rangeFormatting` (selection), `textDocument/formattingProvider` capability; `TextEdit[]` results; `workspace/configuration` to read formatter config; `FormattingOptions` for tab size and insert-final-newline.

**Technical complexities.** Markdown formatting is opinionated and contested — no single formatter satisfies every team. The server should integrate established tools (prettier via WASM or subprocess, mdformat, dprint) rather than reimplement. Prose rewrapping must understand CommonMark hard breaks vs. soft breaks (Darkmatter's "incidental newline" concept). Tables need column-width computation with Unicode-width awareness.

#### 4.4 On-Type Formatting

**Description & value.** Reformats incrementally as the user types trigger characters: align table cells when they press Tab or `|`, continue list items on Enter, balance emphasis when they close `*`. Value: tables and lists — historically the worst Markdown UX — become pleasant.

**Example user experience.** In a Markdown table, pressing Tab advances to the next cell and re-aligns the column separators. Pressing Enter at the end of a list item inserts the next bullet (with correct indentation, including for nested lists and task lists). Pressing Enter twice exits the list. Closing `*` after `*emphasis` wraps the inline. Nuances: list continuation must handle ordered-list numbering (`1.` → `2.`), task-list checkboxes (`- [ ]` → `- [ ]`), and blockquote continuation (`> `); table alignment must not fight the user mid-cell.

**LSP API surface.** `textDocument/onTypeFormatting` with `TextDocumentPositionParams` and `ch` (the trigger character); `FormattingOptions`; `TextEdit[]`; capability `textDocument.onTypeFormatting` with `firstTriggerCharacter` and `moreTriggerCharacter`.

**Technical complexities.** On-type formatting is invoked on every keystroke and must be extremely fast (sub-millisecond) or it degrades typing. It must be a no-op when the surrounding context doesn't call for action. List and table state machines require incremental parsing of the line. The formatter must not "fight" the user (e.g., re-align a table while the user is mid-edit on a cell).

#### 4.5 Organize Link Definitions

**Description & value.** Sorts, deduplicates, groups, and removes unused `[ref]: url "title"` definitions at the bottom of the document. Value: clean source, faster lookup, smaller diffs.

**Example user experience.** A `source.organizeMarkdown` code action consolidates definitions alphabetically, removes those marked unused, groups by domain (`github.com` together), and aligns the colons. A "Convert inline links to reference form" variant extracts every inline link to a definition. Nuances: organize must respect the project's preferred ordering (alphabetical, by first use, by domain); transclusion-aware unused-detection avoids removing definitions used in included files; some projects forbid reference links entirely and want an "inline-ify" action.

**LSP API surface.** `textDocument/codeAction` with `kind: "source.organizeImports"` (the LSP's standard "source.organize\*" prefix), or a custom `source.organizeMarkdownLinks`; `CodeAction.edit` containing `TextEdit[]` restricted to the definitions block.

**Technical complexities.** Requires resolving every inline link to know whether it could be reference-form. Deduplication must preserve CommonMark's "first definition wins" semantics during the sort. Grouping heuristics (by domain, by path depth, by usage frequency) are project-specific.

#### 4.6 Extract Section

**Description & value.** Pulls a heading and its content out into a new file, replacing the original location with a link, include, or transclusion to the new file. Value: progressive decomposition of long documents without breaking existing anchors.

**Example user experience.** Right-click on `## Configuration` → "Extract to new file" prompts for a filename, creates `configuration.md` with the section content (preserving the heading and any sub-headings), and replaces the original with `::file configuration.md` (Darkmatter) or `[See: Configuration](configuration.md#configuration)`. Every external reference to `#configuration` is updated to point to the new file. Nuances: extraction can preserve or drop the extracted heading (some projects want the heading to live only in the extracted file); nested sub-headings travel with the parent; cross-references to anchors *within* the extracted section must be updated to point into the new file.

**LSP API surface.** `textDocument/codeAction` with `kind: "refactor.extract"`; `WorkspaceEdit` combining `CreateFile`, the source `TextEdit` (replacing the section), and per-reference `TextEdit`s across the workspace; `ChangeAnnotation` to label each part of the operation.

**Technical complexities.** Requires the reference graph and the document heading tree. The decision to keep vs. drop the heading affects every external reference and must be presented as a choice. Sub-heading anchors in the extracted block become anchors in the new file, and references to them must be rewritten with the new file path.

#### 4.7 Convert Syntax

**Description & value.** Migrates between equivalent Markdown forms: ATX ↔ setext headings, reference ↔ inline links, wiki-links ↔ standard links, fenced ↔ indented code, tight ↔ loose lists, asterisk ↔ underscore emphasis. Value: dialect migration, style normalization, and tool compatibility.

**Example user experience.** A project moving away from wiki-links offers "Convert all wiki-links to standard links" as a document-wide code action: each `[[auth]]` becomes `[auth](authentication.md)` (or `[Authentication](authentication.md)` if the alias was the display name). ATX-to-setext is a one-directional style choice (or vice versa). Indented-code-to-fenced is common when adding language hints. Nuances: not all conversions are lossless (setext only supports H1/H2; reference links lose the inline display); some conversions should be offered per-occurrence, others document-wide.

**LSP API surface.** `textDocument/codeAction` with `kind: "refactor.rewrite"`; `CodeAction.edit` with `TextEdit[]` or `WorkspaceEdit`; per-occurrence actions triggered from a single cursor position; document-wide actions via `source.*` kinds.

**Technical complexities.** Requires a lossiness analysis for each conversion — some conversions are reversible, others aren't. Wiki-link → standard link needs the file-resolution machinery to compute the target path. Display-name vs. link-text decisions need user input or heuristics. Document-wide conversions should be previewable before applying.

#### 4.8 Generate TOC

**Description & value.** Inserts or refreshes a bullet-list or ordered table of contents derived from the document's headings, optionally with anchors and depth limits. Value: maintained TOCs that don't drift as headings change.

**Example user experience.** A code action "Insert table of contents" at the cursor inserts a bulleted TOC with anchor links, scoped to H2 and H3 by default. On later edits, a "Refresh TOC" action updates the list in place. A `<!-- toc -->` … `<!-- /toc -->` marker pair scopes the editable region. Nuances: depth, numbering, and link style are configurable; some renderers generate TOCs automatically (mkdocs) and an author-supplied TOC is wrong; the server should detect this and suppress the action.

**LSP API surface.** `textDocument/codeAction` with `kind: "source.generateToc"` and `kind: "source.refreshToc"`; `CodeAction.edit` inserting/replacing within the marker region; `Command` if the client drives insertion via UI.

**Technical complexities.** Requires the document heading tree with computed slugs under the project's algorithm. The marker-pair convention must be documented; without it, "refresh" risks clobbering user edits. Detection of renderer-generated TOCs requires project-type awareness.

---

### Group 5 — Structural & Syntactic Awareness

**Group focus.** Helping the editor (and the user) understand the document's shape: which regions fold, how to expand selection, how to colorize beyond TextMate, what to annotate as ghost text, and which ranges stay linked. These features are largely invisible when present and painful when absent.

#### 5.1 Folding Ranges

**Description & value.** Declares foldable regions: heading sections (an H2 with everything until the next H2), list items, fenced code blocks, frontmatter, tables, block quotes, and directive blocks. Value: focus on the relevant slice of a long document; the editor's fold-all-headings affordance.

**Example user experience.** Every heading gets a fold chevron in the gutter; clicking collapses the entire section. Frontmatter can be folded as a single block. A 50-row table folds to its header. A `:::note` directive folds as a unit. Nuances: section folding requires the heading tree to compute "until next heading of equal-or-higher level"; folding a directive requires tracking open/close pairs; some users want per-paragraph folding, which is usually too granular and should be opt-in.

**LSP API surface.** `textDocument/foldingRange` returning `FoldingRange[]` with `startLine`/`endLine` and `kind` (`region`, `comment`, `imports`); client capability `textDocument.foldingRange.rangeLimit` and `lineFoldingOnly`.

**Technical complexities.** Markdown's "section" is not a syntactic node — it's the implicit range between two headings. The server must compute these ranges from the heading sequence. Line-only folding (the most common client capability) means character-precise ranges can't be expressed; everything must snap to line boundaries.

#### 5.2 Selection Ranges

**Description & value.** Provides semantically meaningful expand/shrink selection targets: inside emphasis, inside link text, whole link, whole list item, whole list, whole section. Value: precise selection without mouse fiddling; powers the editor's "expand selection" keybinding.

**Example user experience.** Cursor inside a link's text: expanding selects the text, then `[text](url)`, then the whole paragraph, then the section, then the document. Inside a table cell: cell → row → table → section. Inside a list item: item text → whole item → sub-list → outer list. Nuances: selection ranges must form a strict nesting hierarchy at every position; gaps or overlaps confuse the editor.

**LSP API surface.** `textDocument/selectionRange` with `SelectionRange[]` (a tree of `Range`s rooted at each requested position); typically invoked with a list of positions for multi-cursor support.

**Technical complexities.** Requires the full syntax tree to compute the containing-node chain at each position. pulldown-cmark's event stream doesn't naturally give "the node containing offset N" — the server must build a tree (or use a tree-based parser like Tree-sitter or markdown-rs) to answer efficiently.

#### 5.3 Semantic Tokens

**Description & value.** Provides richer highlighting than TextMate grammars by classifying every token: heading level, frontmatter key vs. value, link text vs. target vs. anchor fragment, directive kind, emphasis strength, embedded-language regions, table header vs. body. Value: visually distinguish structural roles at a glance; consistent highlighting across editors.

**Example user experience.** Frontmatter keys appear in one color, string values in another, dates in a third. Heading text is bolded, and the `#` markers are dimmed. Wiki-link brackets are subtle while the file name is prominent. A code fence's contents are highlighted by the embedded language. Nuances: token types and modifiers must be declared up front in the server's capabilities; clients vary in how they render modifiers; some users prefer TextMate-only highlighting for performance.

**LSP API surface.** `textDocument/semanticTokens/full`, `textDocument/semanticTokens/full/delta`, `textDocument/semanticTokens/range`; `SemanticTokensLegend` declaring token types and modifiers; `SemanticTokens` with relative-position-encoded data array; capability `textDocument.semanticTokens` (multiline support, server-side custom modifiers).

**Technical complexities.** Semantic tokens scale linearly with document size and must be delta-computable for incremental edits. The token stream must interleave with the embedded language's tokens (e.g., the YAML inside a frontmatter gets its own tokenization). Multi-line tokens (a paragraph spanning many lines) require the 3.17 multiline capability.Editors cache the full token set and request deltas — the server must support delta computation efficiently.

#### 5.4 Inlay Hints

**Description & value.** Inline ghost-text annotations: resolved link targets next to a wiki-link, the anchor slug next to a heading, the inferred type next to a frontmatter value, the transclusion source next to a `::file` directive. Value: see through abstractions without leaving the cursor.

**Example user experience.** Next to `[[auth]]`, gray ghost text shows `→ Authentication.md`. Next to `## Setup`, gray `{#setup}` shows the resolved anchor. Next to `::file config.md`, an arrow and a one-line preview of the included content appears. Next to `draft: true` in frontmatter, gray `boolean` shows the inferred type. Nuances: inlay hints must be opt-in (some users find them noisy), per-kind toggleable, and clickable to navigate or edit; they should not displace the user's text.

**LSP API surface.** `textDocument/inlayHint` with `InlayHintParams.range`; `inlayHint/resolve` for lazy `tooltip`/`label` details; `InlayHint.label` (string or parts); `InlayHint.textEdits` for click-to-edit; `InlayHintKind` (`Type`, `Parameter`).

**Technical complexities.** Inlay hints are recomputed on every visible-range change; they must be cheap. The label must be width-aware (long URLs truncate). Hints that mirror computed state (resolved link targets) must update when the target changes. Click-to-navigate requires a command or `textEdits` + client cooperation.

#### 5.5 Linked Editing Ranges

**Description & value.** Keeps paired ranges in sync as the user types in one of them: link text ↔ its reference definition, an opening directive tag ↔ its closing tag, an ATX heading text ↔ its generated anchor. Value: rename once, both sides update without invoking the rename refactor.

**Example user experience.** Placing the cursor inside a `[my label][ref]` and editing the label updates the `[ref]: …` definition's display text in lockstep. Editing an opening `:::note` updates the closing `:::`. Editing a heading updates an inlay-hint slug that the user can promote to a custom ID. Nuances: linked editing is a 3.18 refinement; older clients support only HTML-tag-like pairs; Markdown's pairs are looser (label vs. definition may be far apart) and require explicit opt-in.

**LSP API surface.** `textDocument/linkedEditingRange` returning `LinkedEditingRanges` with `ranges` and optional `wordPattern`; client capability `textDocument.linkedEditingRange`.

**Technical complexities.** The server must identify the paired ranges for the cursor position and return them all. Real-time synchronization is handled by the client, which mirrors edits across the returned ranges. Label-vs-definition pairing requires resolving the link to its definition first.

#### 5.6 Document Link

**Description & value.** Marks URLs, file paths, and anchors as clickable links the editor renders with underline + cmd-click navigation. Value: navigate without breaking the reading flow; standard editor affordance for any URL-bearing format.

**Example user experience.** URLs in autolinks (`<https://example.com>`) and bare URLs are underlined; cmd-click opens in a browser. Internal links `](setup.md)` are underlined; cmd-click opens the file in the editor. Anchor fragments are underlined; cmd-click scrolls within the document. Nuances: `documentLink/resolve` populates the `target` lazily (cheap scan, expensive resolution); links inside code spans are typically suppressed; tooltip on hover shows the resolved target.

**LSP API surface.** `textDocument/documentLink` returning `DocumentLink[]`; `documentLink/resolve` to fill in `target` and `tooltip` lazily; client capability `textDocument.documentLink.tooltipSupport`.

**Technical complexities.** Document links are presented for every URL in the document, which can be many; lazy resolution keeps the initial response cheap. Distinguishing navigable internal links from external URLs requires the resolver from Group 1. Links inside code spans, frontmatter values, or fenced code blocks need per-project policy.

---

### Group 6 — Hover, Preview & Documentation

**Group focus.** Surfaces information at the cursor without requiring navigation. Hover is the lowest-friction discovery surface in any LSP and is especially valuable in Markdown, where targets (linked documents, images, schema fields) carry content the user wants to peek at.

#### 6.1 Link & Document Hover Preview

**Description & value.** Hovering over a link shows the target's title, first heading, first paragraph, or a rendered preview. For wiki-links, the alias and any aliases are shown. Value: decide whether to follow a link without losing your place.

**Example user experience.** Hovering `[design](design.md)` shows a popover with `design.md`'s H1, the first paragraph, and an "Open" link. Hovering `[[auth]]` shows the resolved file's title and aliases. Hovering an external URL shows a fetched Open Graph preview (title, description, favicon). Nuances: link previews should be cached and fetched off the critical path; previews of very long documents should truncate to the first screenful; image-bearing targets may inline the first image.

**LSP API surface.** `textDocument/hover` returning `Hover` with `MarkupContent` (`CommonMark` or `markdown`); the hover content itself is Markdown the editor renders; `Hover.range` to bind the hover to the link span.

**Technical complexities.** Hover fires on every cursor dwell; it must be cheap. Target document preview requires reading and parsing the target, which must be cached. External previews require HTTP fetch with the same complexity as external link validation. The hover content is itself Markdown and must round-trip cleanly through the client's Markdown renderer.

#### 6.2 Image Hover Preview

**Description & value.** Hovering over an image reference shows the image inline (thumbnail, dimensions, alt text, file size). Value: visual confirmation of referenced assets without opening them.

**Example user experience.** Hovering `![diagram](assets/diagram.png)` shows the image rendered in the hover popover, with `512×384 · 24 KB · PNG` and the alt text. Hovering a broken image reference shows "Image not found" with a "Locate file" action. Nuances: SVG, PNG, JPG, WebP, and animated GIFs should all work; very large images should be thumbnailed; remote images require fetching with the same policy as external links; terminal clients (Neovim, Helix) may not render images and should receive alt-text-only hovers.

**LSP API surface.** `textDocument/hover` with `MarkupContent` of kind `markdown` containing an `![](file://…)` image reference the client renders; some clients accept `MarkupContent` of kind `markdown` with embedded data URIs; terminal clients get a text-only fallback.

**Technical complexities.** Image preview requires reading and decoding image headers (for dimensions) and producing a path or data URI the client can render. Client capability varies wildly: VS Code, Neovim (with image plugins), and Zed render images differently; a server targeting all three needs capability detection and a graceful text fallback.

#### 6.3 Frontmatter Hover

**Description & value.** Hovering over a frontmatter key shows its schema description, type, allowed values, default, deprecation status, and example. Value: in-context documentation of metadata contracts; eliminates context-switching to read the schema.

**Example user experience.** Hovering `draft:` in frontmatter shows a popover: "draft (boolean, default: false) — When true, this document is excluded from production builds. Example: `draft: true`." Hovering a deprecated key shows a strikethrough and a migration hint. Hovering an enum-typed value shows the allowed set. Nuances: schema descriptions are the source of truth; if the schema lacks a description, the hover should say so rather than fabricate; multiple applicable schemas (glob-matched) should all contribute; the hover should include a "Go to schema definition" link.

**LSP API surface.** `textDocument/hover` returning `MarkupContent` with the schema description; `MarkupContent.value` may include markdown links (`[schema](./schemas/post.json#/properties/draft)`) the editor makes clickable.

**Technical complexities.** Requires the resolved schema and the JSON Pointer path from the cursor to the schema property. Schemas can be remote, inlined, or glob-matched; all three resolve paths must work. Schema description Markdown must be sanitized (schemas may come from untrusted sources).

#### 6.4 Code Block Hover

**Description & value.** Hovering over a fenced code block shows the detected language, available tooling (formatter, linter, runnable), and any titles or attributes. Value: awareness of embedded-language affordances.

**Example user experience.** Hovering over a ```` ```rust ```` block shows "Rust · syntax highlighting: rust-analyzer · runnable: cargo script · formatter: rustfmt." Hovering an unknown info string offers "Register language mapping." Hovering a ```` ```mermaid flowchart ```` block shows "Mermaid flowchart diagram" and an optional "Render preview" action. Nuances: language detection is sometimes ambiguous (`.ts` could be TypeScript or Twig); the hover should show the resolved grammar and allow override.

**LSP API surface.** `textDocument/hover` with `MarkupContent` describing the language; `Command` references in the hover for runnable actions (`cargo script`, `eval JS`); `textDocument/codeLens` to surface the same affordances inline.

**Technical complexities.** Language detection routes through the grammar registry. Runnable detection requires per-language tooling probes (is `cargo` on PATH?). Mermaid and similar diagram languages need a separate rendering path that may not exist in all editors.

#### 6.5 Code Lens

**Description & value.** Inline actionable annotations above headings (`3 references`, `5 backlinks`), links (`validate`, `open`, `preview`), directives (`render`, `expand transclusion`), and code blocks (`run`, `format`). Value: surface affordances without leaving the document; a bridge to commands.

**Example user experience.** Above every heading: `3 references · 2 backlinks` clickable to open the lists. Above a `::file config.md` directive: `expand` (inlines the content) and `open` (opens the file). Above a code block in a runnable language: `▶ run`. Above a broken link: `fix`. Nuances: code lens must be refreshable (the reference count changes as the user types elsewhere); they should be opt-in per kind; `codeLens/resolve` lets the cheap label be returned first and the expensive detail (the full reference list) be resolved on click.

**LSP API surface.** `textDocument/codeLens` returning `CodeLens[]` with `command` and optionally `data` for lazy resolution; `codeLens/resolve` to fill in `command`/`title` lazily; client capability `textDocument.codeLens.resolveSupport`.

**Technical complexities.** Code lens refresh on every edit is expensive; the server should debounce and compute against the persistent index. The `command` field references a command the client must know how to execute (registered via `executeCommand` capability), so server-side commands need client-side handlers.

---

### Group 7 — Embedded Language Support

**Group focus.** Treating the languages nested inside Markdown — code fences and frontmatter — as first-class regions that deserve their own language intelligence. This is the least-implemented group among current Markdown LSPs and the highest-leverage opportunity for a modern server, because most existing servers treat fences and frontmatter as opaque text.

#### 7.1 Code Fence Language Detection

**Description & value.** Identifies the embedded language of every fenced block from the info string, with content-based fallback heuristics when the info string is missing or ambiguous. Routes downstream features (highlighting, embedded LSP delegation, formatting) to the right tool. Value: correct highlighting and language-server routing; the foundation for embedded-language features.

**Example user experience.** A ```` ```py ```` block is detected as Python (alias resolution). A fence with no info string but containing `def main():` is heuristically detected as Python and offered a "Add `python` info string" action. A ```` ```ts ```` block in a project that uses the two-face extended grammar set resolves to TypeScript (not Twig). Nuances: aliases (`py` → `python`, `sh` → `bash`) are grammar-registry concerns; ambiguous tokens (`ts` could be TypeScript or Twig) need project-level override; heuristics must be conservative to avoid misclassification.

**LSP API surface.** No dedicated LSP method; surfaced via semantic tokens (the fence region is tagged with the detected language), hover (the detected language is reported), and code action (to insert the resolved info string).

**Technical complexities.** Detection routes through the project's grammar registry (Darkmatter's `LanguageGrammar`, the two-face set). Content heuristics require sampling the first N lines and matching against language signatures (regex or Bayesian). Aliases and overrides must be configurable. Detection must be deterministic — a fence that resolves differently across runs is a bug.

#### 7.2 Embedded Language Server Delegation

**Description & value.** Forwards completion, diagnostics, hover, formatting, and code actions from inside a code fence to a language server for the embedded language. A ```` ```rust ```` block gets rust-analyzer completion; a ```` ```sql ```` block gets a SQL server's diagnostics. Value: a snippet in docs is checked like real code; the headline feature of modern doc tooling.

**Example user experience.** Inside a ```` ```rust ```` block, typing `String::f` offers `from`, `from_utf8`, … with rust-analyzer's signature help. A ```` ```json ```` block with a trailing comma is flagged. A ```` ```python ```` block gets pylint diagnostics on save. Cmd-click on `print` jumps to the Python docs. Nuances: delegation is expensive and should be opt-in per language; the embedded server may need workspace context (a `Cargo.toml` for rust-analyzer); diagnostics must be offset-shifted back into the fence region; the embedded server is typically spawned as a child process per editor session.

**LSP API surface.** No native LSP abstraction; implemented via virtual documents (`workspace/didOpen` on synthetic URIs like `darkmatter://fence/file.md#L12-30`), relaying `textDocument/*` requests to the embedded server, and translating responses back with offset shifts; `textDocument/codeAction` for "Format this block"; `workspace/executeCommand` for embedded commands.

**Technical complexities.** Each embedded language needs its own delegation strategy, child-process management, and offset translation. The embedded server sees only the fence content (no surrounding context), which limits some features. Spawning and synchronizing multiple embedded servers is heavy; caching and lazy loading are essential. None of the four major Markdown LSPs implements this fully — IWE experiments with it; it is a research frontier.

#### 7.3 Frontmatter as Embedded Language

**Description & value.** Hands the frontmatter region to a YAML, TOML, or JSON language server for its own completion, formatting, hover, and diagnostics — then layers schema validation on top. Value: best-of-both-worlds metadata authoring (native YAML ergonomics plus schema enforcement).

**Example user experience.** Inside frontmatter, YAML completion offers keys based on indentation context. YAML formatting reflows long values. Schema validation overlays diagnostics: YAML is structurally valid, but `draft: "yes"` violates the schema's boolean type. Hover on a YAML key shows both the YAML schema (if any) and the project JSON Schema. Nuances: YAML/TOML/JSON frontmatter formats are detected by leading delimiter (`---` vs. `+++` vs. `{}`); the embedded server must be selected accordingly; offset translation between the frontmatter region and the embedded document must be exact; the layering of YAML-native diagnostics and schema diagnostics needs de-duplication.

**LSP API surface.** Same virtual-document pattern as 7.2; `textDocument/publishDiagnostics` from the embedded server is relayed with offset shifts; schema diagnostics are layered on top; `textDocument/hover` and `textDocument/completion` may merge embedded-server and schema-driven results.

**Technical complexities.** Frontmatter format detection must be robust. The embedded server typically expects a full file, not a region — virtual-document URIs and offset translation are required. Conflict resolution between embedded-server and schema-driven diagnostics (e.g., both flag the same key) needs deduplication. The Red Hat YAML language server and similar tools already do schema validation, so the server may *be* the schema validator rather than layering on top.

---

### Group 8 — PKM & Knowledge Graph Features

**Group focus.** Personal Knowledge Management affordances pioneered by Obsidian, Roam, Logseq, and Foam. These features treat a folder of Markdown files as a graph of atomic notes rather than as isolated documents. They are not part of CommonMark or GFM and are largely absent from VS Code's bundled server, but they are the headline differentiator of markdown-oxide and Marksman.

#### 8.1 Block References

**Description & value.** Resolves `^block-id` references and `![[file#^block-id]]` block embeds to their source block (paragraph, list item, or table row). Value: fine-grained PKM linking pioneered by Obsidian and Logseq.

**Example user experience.** A paragraph ends with `^my-block`; elsewhere, `[[notes#^my-block]]` links to it and `![[notes#^my-block]]` embeds the paragraph inline (rendered in a hover or inlay hint). Code lens on the source paragraph shows `1 reference`. Renaming the block ID updates references. Nuances: block IDs are arbitrary user-chosen strings; embedding (`![[…]]`) versus linking (`[[…]]`) is a syntactic distinction; block granularity varies (paragraph, list item, table row); a block ID with no defined target is a broken reference.

**LSP API surface.** `textDocument/definition`, `textDocument/references`, `textDocument/codeLens`, `textDocument/completion` (for `^block-id` after `#`), all extended to recognize block-reference syntax; inlay hints to render embedded block content.

**Technical complexities.** Block boundaries require parser-level support (the parser must assign block IDs to the preceding block). Embedding semantics (resolve and render in place) require both resolution and rendering — typically approximated with inlay hints or hover previews rather than true in-place expansion. Renaming a block ID touches every referent across the workspace.

#### 8.2 Daily Notes

**Description & value.** Natural-language date completion (`[[daily/2026-07-04]]`, `:Daily two days ago`) and one-action creation of today's note from a template. Value: frictionless journaling and time-based linking; the Obsidian-and-Foam-friendly workflow.

**Example user experience.** Typing `[[2026-07-` offers dates `2026-07-03`, `2026-07-02`, … with day-of-week annotations. A `:Daily today` command creates `daily/2026-07-04.md` from `Templates/Daily.md` if it doesn't exist and opens it. A `:Daily next monday` parses the natural-language date and creates or opens that note. Nuances: date format (`YYYY-MM-DD` vs. `YYYY/MM/DD-MM-DD` vs. ISO week) is project-configurable; templates support variable substitution (`{{date}}`, `{{title}}`); daily notes may be the implicit destination for "today" references in body text.

**LSP API surface.** `textDocument/completion` for date-based targets; `workspace/executeCommand` for the `:Daily` command family; `workspace/applyEdit` to create the note from the template; the command is registered via `ExecuteCommandOptions`.

**Technical complexities.** Natural-language date parsing requires a date library (chrono in Rust) and a configurable "today" reference (timezone matters). Template substitution is a mini mustache/handlebars engine. Daily-note conventions vary (folder, filename, template location) and must be configurable.

#### 8.3 Tags & Properties Index

**Description & value.** A workspace-wide index of tags and frontmatter properties (with their values), queryable for completion, search, and aggregation. Value: structured retrieval across a vault; the foundation for tag-based navigation and dashboards.

**Example user experience.** A `workspace/symbol` query for `#tag/inbox` returns every document with that tag. Hovering over a tag in body text shows "Used in 12 documents." A properties panel (custom view) shows the distribution of `status:` values across the vault. Nuances: tag index unifies body-text `#tag` and frontmatter `tags:`; property index requires schema awareness to know which values are enum-like; the index must update incrementally.

**LSP API surface.** `workspace/symbol` (for tag search), `textDocument/completion` (for tag/property completion), `textDocument/hover` (for tag usage counts), `textDocument/codeLens` (for per-document tag counts); custom views via `workspace/executeCommand` driving client-side panels.

**Technical complexities.** The index covers two distinct sources (body tags and frontmatter properties) and must unify them. Incremental updates on file change require per-document tag extraction. Property values are schema-dependent — without a schema, the index can only collect raw values. Cross-workspace aggregation requires the full workspace to be indexed.

#### 8.4 Aliases & Redirects

**Description & value.** Honors frontmatter `aliases:` (Obsidian), `permalink:` (Jekyll, mkdocs), and `redirect_from:` (Jekyll) so links by any alias resolve to the canonical document, and so renamed files leave redirects behind. Value: rename resilience and multiple naming conventions.

**Example user experience.** `Authentication.md` declares `aliases: [auth, login]`. A link `[[auth]]` resolves to `Authentication.md`. Renaming `Setup.md` to `Installation.md` offers to leave behind a `Setup.md` containing only `---\naliases: [Setup]\nredirect_to: Installation.md\n---\n`. A link checker following `redirect_to` follows the chain. Nuances: aliases must be indexed alongside file names; alias conflicts (two files claiming the same alias) must be diagnosed; permalink structures (`/blog/:slug/`) require template-aware resolution.

**LSP API surface.** `textDocument/definition` and `textDocument/references` extended to consult the alias index; `textDocument/publishDiagnostics` for alias conflicts; `textDocument/codeAction` to leave a redirect on rename.

**Technical complexities.** Alias index maintenance requires parsing frontmatter of every document. Permalink templates require understanding the site generator's conventions (Jekyll, Hugo, mkdocs, Docusaurus, VitePress all differ). Redirect chains must be cycle-detected.

---

### Group 9 — Workspace & Project Intelligence

**Group focus.** Treating a folder of Markdown files as a coherent project, not a bag of independent documents. These features maintain the global state — the file index, the link graph, the active dialect, the ignore rules — that every other feature depends on. They are the unglamorous infrastructure that determines whether the LSP feels instant or sluggish on a real vault.

#### 9.1 Project Awareness

**Description & value.** Detects the project type (Obsidian vault, mkdocs/Docusaurus/VitePress/Astro Starlight docs site, Foam workspace, generic Markdown) by discovering marker files (`.obsidian/`, `mkdocs.yml`, `docusaurus.config.js`, `.foam`, `.marksman.toml`, `.moxide.toml`) and adapts behavior: dialect, link style, anchor algorithm, default ignore patterns, and feature enablement. Value: the LSP's behavior matches the project's conventions without per-project configuration.

**Example user experience.** Opening a folder with `.obsidian/` enables wiki-link syntax, Obsidian-style anchor slugs, and daily-note commands automatically. Opening a folder with `mkdocs.yml` enables strict CommonMark with mkdocs extensions, `docs/`-relative links, and `mkdocs.yml`-aware validation. A mixed folder (no markers) falls back to conservative defaults. Nuances: detection should be hierarchical (a vault nested in a docs site is possible); project-type-specific config files (`mkdocs.yml`'s `markdown_extensions:`) should be read to refine behavior; the user may want to override detected type.

**LSP API surface.** `initialize` with `rootUri`; `workspace/configuration` to read project config; `workspace/didChangeConfiguration` to react to changes; `workspace/didChangeWatchedFiles` to react to marker-file changes that imply a project-type change.

**Technical complexities.** Project detection is a discovery process at startup and must be re-run on workspace-folder changes. Each project type has its own config schema and conventions; supporting them all is open-ended. The detected type gates many behaviors, so mis-detection causes cascading confusion. The user needs a way to inspect and override the detected type.

#### 9.2 File Watching

**Description & value.** Subscribes to filesystem changes for files the user isn't editing in the editor, so that link validation, the reference graph, and the symbol index stay current as files move outside the editor. Value: link validation stays current; the index doesn't drift.

**Example user experience.** In one editor tab the user deletes `old-page.md`; in another tab, every `[old page](old-page.md)` reference immediately re-flags as broken. A git pull brings in new files; workspace symbols picks them up within seconds without a restart. Nuances: file watching is heavyweight at scale (vaults with tens of thousands of files need OS-native watchers, not polling); watcher registration is gated by client capability (`workspace.didChangeWatchedFiles.dynamicRegistration`); some clients (VS Code) provide server-side watchers, others (some Neovim configs) require the server to poll.

**LSP API surface.** `workspace/didChangeWatchedFiles` notification (client → server) with `FileEvent[]`; client capability `workspace.didChangeWatchedFiles.dynamicRegistration` and `relativePatternSupport`; the server registers watch patterns via `client/registerCapability`; `workspace/didChangeWorkspaceFolders` for root changes.

**Technical complexities.** Watcher registration is dynamic and client-dependent. At scale, the server should rely on client-provided watchers (VS Code) rather than its own OS watchers, to coalesce with other tools. The watched-file pattern must be tight (watching every file is wasteful); glob patterns and `.gitignore`-style exclusions must be respected. Events arrive in bursts and must be coalesced.

#### 9.3 Link/Transclusion Dependency Graph

**Description & value.** Maintains the bidirectional graph of which documents reference, include, transclude, or embed which others, with cycle detection and incremental invalidation. Value: incremental recomputation, cycle detection, and impact analysis — the engine behind find-references, backlinks, workspace diagnostics, and "update references on rename."

**Example user experience.** Editing `glossary.md` immediately invalidates and recomputes diagnostics for every document that transcludes it. A circular transclusion (`a.md` includes `b.md` includes `a.md`) is detected at insertion and flagged with a diagnostic before it can hang the server. Renaming `glossary.md` triggers a code action to update 47 referring documents, each listed in the prompt. Nuances: the graph must support both link (logical) and transclusion (compositional) edges, which have different semantics; cycle detection must distinguish intentional recursive data structures from accidental loops; invalidation must propagate through transclusions but not through plain links.

**LSP API surface.** No dedicated LSP method; the graph is internal state consumed by `textDocument/definition`, `textDocument/references`, `textDocument/publishDiagnostics`, `workspace/willRenameFiles`, and others; the LSP 3.18 `textDocument/prepareCallHierarchy` family is occasionally used to model document-to-document relationships but is uncommon for Markdown.

**Technical complexities.** The graph must be arena-allocated (per IWE's design) for O(1) lookups. Cycle detection at edge-insertion time uses Tarjan's SCC or similar. Incremental invalidation propagates only to affected dependents, not the whole graph. The graph must survive file renames (edge retargeting) without a full rebuild. Memory budget matters at vault scale.

#### 9.4 Incremental Document Sync

**Description & value.** Receives only the changed ranges from the editor on each edit, not the full file, so the server can re-parse only the affected region. Value: responsiveness on large documents; lower CPU and memory pressure.

**Example user experience.** A 50,000-line generated API reference is open; the user edits one line. The server receives a `didChange` with a `Range` and the replacement text, updates its internal model, and re-publishes diagnostics for the affected region only — no perceptible delay. Nuances: incremental sync requires careful version tracking; an out-of-order or dropped `didChange` desyncs the server's view from the editor's, producing phantom diagnostics; some clients send full-file sync even when incremental is requested, so the server must handle both.

**LSP API surface.** `textDocument/didChange` with `contentChanges[]` of `TextDocumentContentChangeEvent` (range-based) or full-text; `TextDocumentSyncKind.Incremental` (vs. `Full` or `None`) declared in server capabilities; `TextDocumentItem.version` for ordering.

**Technical complexities.** Incremental application requires translating range-based changes into byte-offset edits in the server's internal text model, then re-parsing the affected region. CommonMark is mostly local (most edits affect nearby parsing), but list and table edits can have non-local effects. The server must track document versions to reject stale changes. Partial parsing of Markdown (re-parse only the affected block) is harder than for languages with explicit block boundaries.

#### 9.5 Workspace Diagnostics

**Description & value.** Validates files that aren't currently open, surfacing broken links and schema violations project-wide. Value: catch broken references before they ship; the docs-equivalent of "build the whole project."

**Example user experience.** A "Workspace Problems" panel lists every broken link in the project, not just in open files. A pre-commit hook can call `workspace/diagnostic` to fail CI on any broken reference. After a refactor that touched 30 files, the panel shows the 2 lingering broken links the author forgot to update. Nuances: workspace diagnostics are expensive (parse every file) and should run on demand or on save, not continuously; pull diagnostics (3.17) let the editor request workspace diagnostics explicitly rather than the server pushing them.

**LSP API surface.** `workspace/diagnostic` (pull, 3.17) returning `WorkspaceDiagnosticReport`; or a project-wide scan surfaced via `textDocument/publishDiagnostics` for each file; `workspace/didChangeWatchedFiles` to invalidate results on change.

**Technical complexities.** Workspace-wide validation requires parsing every file and computing every link target — the full graph. Results must be cached and invalidated incrementally. Pull diagnostics are newer and less widely supported; some clients still expect push. The server must handle very large workspaces without exhausting memory, which argues for on-demand scanning rather than eager full indexing.

#### 9.6 Multi-root Workspace Support

**Description & value.** Handles workspaces with several roots (e.g., a docs site plus a separate vault, or a monorepo with multiple docs trees). Value: realistic multi-project setups; one server instance instead of several.

**Example user experience.** A workspace contains `/code/project/` (with its own docs) and `/notes/vault/` (an Obsidian vault). The LSP applies docs-site conventions to the first root and vault conventions to the second. Cross-root links (rare but possible) are resolved against the right root. Nuances: workspace folders can be added and removed during a session; per-folder configuration must be respected; some features (workspace symbols) span all roots while others (project-type detection) are per-root.

**LSP API surface.** `initialize` with `workspaceFolders`; `workspace/didChangeWorkspaceFolders` notification; `workspace/workspaceFolders` request to query current folders; per-resource configuration via `workspace/configuration` with `scopeUri`.

**Technical complexities.** The link graph and symbol index must be partitioned by root and reunited for cross-root queries. Project-type detection runs per-root. File watching patterns must be per-root. The server must handle a root being removed mid-session without leaking references.

#### 9.7 Configuration Awareness

**Description & value.** Reads editor/workspace settings (`files.trimTrailingWhitespace`, `editor.formatOnSave`) and language-specific config files (`.markdownlint.json`, `.remarkrc`, `docusaurus.config.js`, `mkdocs.yml`) to align its behavior with project policy. Value: behavior matches user expectations and project conventions; one source of truth.

**Example user experience.** A project's `.markdownlint.json` disables MD013 (line length); the server stops emitting that diagnostic. The project's `docusaurus.config.js` declares `titleDelimiter: '|'`; the server uses that delimiter when generating page titles for the symbol index. The editor's `editor.formatOnSave` triggers document formatting on save without a separate server flag. Nuances: config files overlap and conflict (`.markdownlint.json` vs. `.remarkrc`); the server should respect the most specific config and document its precedence; client `workspace/configuration` is per-resource and must be requested, not assumed.

**LSP API surface.** `workspace/configuration` request (server → client); `workspace/didChangeConfiguration` notification (client → server); `ConfigurationItem.scopeUri` for per-resource config; the server may also read config files directly from disk.

**Technical complexities.** Config files come in many formats and locations; discovering and parsing them all is open-ended. Precedence rules (project file > editor setting > default) must be coherent. Config changes must invalidate affected caches. Some clients (notably older Neovim configs) don't implement `workspace/configuration`, forcing the server to fall back to disk reads.

#### 9.8 File Lifecycle Reference Updates

**Description & value.** When the editor renames, moves, or deletes a Markdown file (or a directory containing referenced files), the server computes and proposes link updates across the workspace. Value: renames don't strand broken links; the docs-equivalent of "update imports on rename."

**Example user experience.** The user renames `Setup.md` to `Installation.md` in the editor's file tree. Before the rename completes, the server shows: "Update 23 links across 14 files?" with a preview. Accepting applies all edits in one `WorkspaceEdit`. Deleting a directory offers to remove or update every reference. Nuances: rename may be refactor-style (update links) or destructive (leave broken links intentionally); the server should offer both; will-rename lets the server compute edits *before* the rename is applied, so the editor can present them atomically; resource operations (`RenameFile`, `DeleteFile`) must be supported by the client.

**LSP API surface.** `workspace/willRenameFiles` (server computes edits before rename), `workspace/didRenameFiles` (notification after), `workspace/willCreateFiles`, `workspace/willDeleteFiles`; the response is a `WorkspaceEdit` with text edits across many files plus resource operations; client capability `workspace.fileOperations.*` registers the patterns.

**Technical complexities.** Will-rename must compute the new paths of every reference before the rename is applied, against the *new* filename. Pattern matching (which files to update) requires the file graph. The preview must show all affected files coherently. Resource-operation support varies by client; the server must fall back gracefully.

---

### Group 10 — Performance, Indexing & Protocol Hygiene

**Group focus.** The non-functional requirements that determine whether the LSP feels instant or sluggish on a real workspace. These features are invisible to the user when present and catastrophic when absent — a server that hangs for three seconds on every keystroke is unusable regardless of feature count. They are largely Markdown-agnostic but every Markdown LSP must address them.

#### 10.1 Incremental Parsing & Background Indexing

**Description & value.** Parses each document once on open, updates the parse on incremental edits, and builds the workspace-wide symbol/reference graph in the background without blocking interactive requests. Value: sub-second responses on vaults with thousands of files; the foundation every other feature depends on.

**Example user experience.** Opening a 5,000-file vault shows a brief "Indexing…" progress indicator in the status bar, during which workspace symbols and cross-file references may be incomplete; once indexing completes (a few seconds), all features are fully responsive. Subsequent edits re-parse only the changed file. Nuances: indexing must be resumable (interrupted indexing shouldn't lose progress); the index must persist across server restarts to avoid re-indexing on every launch; the user must be able to invalidate the index manually (a `Reindex` command).

**LSP API surface.** No dedicated LSP method; surfaced via `window/workDoneProgress` during initial indexing and via the speed of every other request. The index is internal state; persistence is the server's own concern.

**Technical complexities.** Parsing must be incremental at two levels: within a file (re-parse only the changed block) and across the workspace (re-parse only changed files). The symbol/reference index must support incremental updates without rebuilds. Persistence (typically a sidecar SQLite or custom binary file) requires schema migration across server versions. Memory budget matters — a 50,000-file vault's full index can be hundreds of megabytes.

#### 10.2 Partial Result Streaming

**Description & value.** Streams large `textDocument/references` and `workspace/symbol` results incrementally, so the editor can show early results while the server continues searching. Value: visible progress and early results on big workspaces; the difference between "instant first result" and "wait three seconds for the full list."

**Example user experience.** A Find References query over a vault returns the first 10 results within 100ms, with a progress indicator showing the search ongoing; the remaining 90 results stream in over the next two seconds. The user can act on the first result before the search completes. Nuances: partial results must arrive in a stable order or be coalesced by the client; cancellation must interrupt the search cleanly; some clients don't support partial results and wait for the full set, so the server must degrade gracefully.

**LSP API surface.** `partialResultToken` in the request params; `$/progress` notifications carrying intermediate `Location[]` results; the final response delivers the complete list (or `null` if the client supports "no final result"). Client capability `textDocument.references.partial` or per-method partial result support.

**Technical complexities.** Partial results require the server to be able to yield intermediate results from its search loop, which implies an internal iterator or channel architecture. The token must be tracked across the request lifetime. Cancellation must propagate into the search. Some clients don't implement partial-result token handling, so the server must fall back to a single final response.

#### 10.3 Request Cancellation

**Description & value.** Aborts expensive operations (workspace symbol search, large rename, full-workspace diagnostics) when the user moves the cursor or closes the request window. Value: the server never blocks the editor on stale work; responsiveness is preserved even with slow queries in flight.

**Example user experience.** The user invokes Find References, then immediately moves the cursor; the original request is cancelled and a new one is started for the new position. A 5-second workspace symbol query is cancelled when the user types another character. The editor stays responsive throughout. Nuances: cancellation is a notification, not a guarantee — the server may have already computed the result; a cancelled request must still return a response (with `RequestCancelled` error code) per the LSP spec; cancellation must propagate into long-running sub-operations.

**LSP API surface.** `$/cancelRequest` notification (client → server) with the request ID; the server responds with `ErrorCodes.RequestCancelled` (-32800); `workDoneToken` is the modern mechanism for cancellable long-running work.

**Technical complexities.** The server must structure its work so cancellation can interrupt it — a tight synchronous loop that doesn't check for cancellation is effectively uncancellable. Async Rust with cooperative cancellation (via `CancellationToken` or `tokio::select!`) is the standard pattern. The server must still return *something* to satisfy the request/response contract.

#### 10.4 Work-done Progress

**Description & value.** Reports long-running operations (initial indexing, large renames, full-workspace diagnostics) with a cancellable progress bar in the editor. Value: smooth UX during slow operations; the user sees that work is happening and can cancel it.

**Example user experience.** Initial workspace indexing shows a progress bar in the editor status area with "Indexing vault… 2,341 / 5,000 files" and a Cancel button. A large rename shows "Updating 47 references across 14 files…". Clicking Cancel aborts the operation. Nuances: progress must update at a reasonable cadence (too frequent floods the client; too rare looks hung); some operations are non-cancellable (in which case no Cancel button is shown); progress tokens must be created via `window/workDoneProgress` before use.

**LSP API surface.** `window/workDoneProgress/create` to create a progress token; `$/progress` notifications with `WorkDoneProgressBegin`, `WorkDoneProgressReport`, `WorkDoneProgressEnd` variants; request params carry `workDoneToken` to associate the request with the progress.

**Technical complexities.** The server must generate progress updates from within the work loop, which requires either an explicit progress channel or shared atomic counters. Cadence throttling matters — too many updates flood the client. Cancellation tokens (per 10.3) integrate with progress. Some clients don't render progress and silently ignore the notifications.

#### 10.5 Dynamic Capability Registration

**Description & value.** Registers (and unregisters) capabilities at runtime depending on what the client supports and what the project needs — for example, registering file watchers only if the client supports `didChangeWatchedFiles` dynamic registration, or registering semantic tokens only after indexing is complete. Value: capability-appropriate behavior without sacrificing features on capable clients; graceful degradation on limited ones.

**Example user experience.** In a client that supports file watchers, the LSP registers them and link validation updates instantly on filesystem changes. In a client that doesn't, the LSP falls back to polling or skips the feature. After initial indexing completes, semantic tokens become available; before then, the LSP doesn't advertise the capability to avoid spurious requests. Nuances: dynamic registration must be idempotent (re-registering a capability is either a no-op or an error); unregistration must clean up cleanly; some capabilities (file watching) are inherently dynamic, others (hover, completion) are static and declared at initialize.

**LSP API surface.** `client/registerCapability` and `client/unregisterCapability` requests (server → client); `Registration` with `method` and per-method `registerOptions`; the client acknowledges before the capability is active.

**Technical complexities.** Dynamic registration requires the server to track what's currently registered and to re-evaluate after capability changes (e.g., a client that gains a capability mid-session). Re-registration on workspace-folder changes is common for file watchers. The server must not call a dynamically-registered method before the client acknowledges, which requires careful sequencing.

---

## Closing Observations

**Priority ordering for a new Markdown LSP.** The four major implementations collectively validate a priority order. Diagnostics (Group 2), navigation (Group 1), and completion (Group 3) are the table-stakes features that any Markdown LSP must ship to be considered modern — the VS Code server, Marksman, markdown-oxide, and Markmark all cover most of these. Refactoring (Group 4) and structure (Group 5) are the next tier; only the VS Code server and Marksman implement substantial slices. Hover and code lens (Group 6) and PKM features (Group 8) differentiate markdown-oxide from the pack. Embedded language support (Group 7) is the research frontier — no current Markdown LSP implements it fully, and the architectural blueprint for the Darkmatter LSP identifies it as the primary reason to build bespoke rather than reuse.

**The cross-cutting dependency.** Nearly every feature in Groups 1, 4, 6, 8, and 9 depends on the same underlying asset: a workspace-wide, incrementally-maintained graph of headings, anchors, files, links, and references (Group 9's dependency graph). This graph is the single highest-leverage investment — once it exists, find-references, backlinks, safe rename, file-lifecycle updates, workspace diagnostics, and partial-result streaming all become straightforward to add. Servers that skip the graph and compute references on demand (Markmark's original approach) cap out at single-file features.

**The Markdown-specific hard parts.** Three problems recur across this catalog and have no clean solution in the LSP spec: (1) anchor-slug algorithm variance across renderers (GitHub, CommonMark, Obsidian, mkdocs all differ); (2) embedded-language regions (code fences, frontmatter) require virtual documents and offset translation because the LSP has no native "embedded language" abstraction; and (3) transclusion-aware analysis (a link definition "used" only in a transcluded file is not unused) requires running every analysis against the composed document, not the raw file. A modern Markdown LSP's quality is largely determined by how it handles these three.

**Performance is a feature.** Groups 9 and 10 are not optional polish. A Markdown LSP that runs against a real Obsidian vault (5,000+ files) or a real docs site (Docusaurus monorepos regularly exceed 2,000 files) must index incrementally, stream partial results, cancel stale work, and report progress — or it will be perceived as broken regardless of how many features it implements.
