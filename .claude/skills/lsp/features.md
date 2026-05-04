# Features an LSP Can Provide

An LSP can provide features in several broad categories:

- Navigation
- Diagnostics
- Editing assistance
- Semantic understanding
- Refactoring
- Workspace intelligence
- Editor UX integration

## Core Code Intelligence Features

| Feature | Description |
|---|---|
| Diagnostics | Reports errors, warnings, hints, style issues, schema violations, parser errors, type errors, lint results, etc. |
| Hover information | Shows documentation, type information, symbol details, links, rendered Markdown, examples, or metadata when hovering. |
| Go to definition | Jumps from a reference to the place where the symbol is defined. |
| Go to declaration | Jumps to the declaration site, which may differ from implementation/definition. |
| Go to type definition | Jumps from a value/expression to the definition of its type. |
| Go to implementation | Jumps from an interface, trait, protocol, or abstract member to concrete implementations. |
| Find references | Finds all usages of a symbol across a file, workspace, or project. |
| Document symbols | Lists symbols in the current document for outline/sidebar navigation. |
| Workspace symbols | Searches symbols across the workspace. |
| Completion | Provides context-aware autocomplete for symbols, keywords, paths, schema fields, enum values, snippets, etc. |
| Signature help | Shows function/method parameter information while typing calls. |
| Inline documentation | Surfaces docs, examples, schema descriptions, deprecated notices, and parameter help. |

## Editing Assistance

| Feature | Description |
|---|---|
| Formatting | Formats whole documents, ranges, or formatting-on-type. |
| Code actions | Offers fixes or transformations, such as “import this symbol,” “convert syntax,” “add missing field,” or “suppress warning.” |
| Quick fixes | Code actions specifically tied to diagnostics. |
| Refactoring actions | Extract function, rename symbol, inline variable, move file, convert syntax, etc. |
| Rename symbol | Safely renames a symbol and updates references. |
| Prepare rename | Validates whether the selected item can be renamed before starting. |
| Organize imports | Sorts, removes, or groups imports/includes/references. |
| Auto-imports | Suggests imports or module paths for referenced but unresolved symbols. |
| On-type formatting | Applies formatting when the user types a trigger character such as `}`, `;`, newline, etc. |
| On-save actions | Applies fixes, formatting, lint cleanup, schema normalization, etc. when saving. |
| Document edits | Performs structured multi-file edits through `WorkspaceEdit`. |

## Structural Language Features

| Feature | Description |
|---|---|
| Folding ranges | Provides foldable regions: headings, blocks, functions, frontmatter, comments, sections, imports, etc. |
| Selection ranges | Enables semantic expansion/shrinking of selection based on syntax tree structure. |
| Document highlights | Highlights all occurrences of the symbol under the cursor. |
| Linked editing ranges | Keeps related ranges synchronized, such as opening/closing tags or paired names. |
| Semantic tokens | Provides syntax-aware highlighting beyond TextMate regex grammars. |
| Inlay hints | Shows inline parameter names, inferred types, implicit values, lifetimes, generic arguments, etc. |
| Inline values | Displays runtime/debugger-related values inline during debugging. |
| Call hierarchy | Shows callers and callees of functions/methods. |
| Type hierarchy | Shows inheritance, implementations, subtypes, supertypes, trait implementations, etc. |
| Monikers | Associates symbols with stable cross-tool identifiers, useful for indexing and code search. |

## Workspace and Project Features

| Feature | Description |
|---|---|
| Workspace diagnostics | Reports problems across the whole project, not just open files. |
| Project graph awareness | Understands modules, packages, references, dependencies, imports, includes, or transclusions. |
| File watching | Reacts to file creation, deletion, renames, config changes, schema changes, dependency changes, etc. |
| Configuration awareness | Reads editor/workspace settings and language-specific config files. |
| Multi-root workspace support | Handles workspaces with several roots/projects. |
| Incremental document sync | Receives only document changes rather than full-file reloads. |
| Workspace folders | Tracks added/removed workspace folders. |
| Apply workspace edits | Requests the editor to apply changes across many files. |
| Create/rename/delete files | Participates in file operations, including validating or updating references. |

## Search and Indexing Features

| Feature | Description |
|---|---|
| Symbol indexing | Builds an index of symbols for navigation, completion, references, and workspace search. |
| Reference indexing | Tracks usage relationships across the workspace. |
| Dependency indexing | Tracks imports, includes, schema references, modules, transclusions, links, etc. |
| Full-workspace validation | Validates files even when they are not open. |
| Cross-file linking | Resolves references between documents, modules, schemas, Markdown links, assets, etc. |
| External dependency resolution | Resolves symbols or schemas from packages, libraries, registries, vendored files, or generated artifacts. |

## Documentation and Markdown-Oriented Features

| Feature | Description |
|---|---|
| Rendered hover docs | Shows rendered Markdown/HTML documentation on hover. |
| Link validation | Detects broken internal links, anchors, file references, images, or external URLs. |
| Anchor completion | Completes headings/anchors for Markdown links. |
| Path completion | Completes relative file paths, image paths, include paths, schema paths, etc. |
| Frontmatter validation | Validates YAML/TOML/JSON frontmatter against schema rules. |
| Schema-aware completion | Suggests valid frontmatter keys, values, enums, nested objects, defaults, etc. |
| Directive/block validation | Validates custom Markdown directives, fenced blocks, embedded DSL syntax, or transclusion syntax. |
| Embedded language support | Delegates or composes support for code fences, YAML, JSON, TOML, HTML, Mermaid, etc. |
| Table of contents support | Provides outline/navigation based on headings and document structure. |
| Heading diagnostics | Detects duplicate headings, invalid hierarchy, missing required sections, etc. |

## Domain-Specific Validation Features

| Feature | Description |
|---|---|
| Schema validation | Validates documents against JSON Schema, custom schemas, frontmatter schemas, or project-specific schemas. |
| DSL validation | Validates embedded DSL constructs. |
| Reference validation | Checks that referenced files, symbols, anchors, schemas, variables, or resources exist. |
| Interpolation validation | Checks that variables used in templates/interpolation are defined and type-compatible. |
| Transclusion validation | Validates include/transclusion directives, detects recursion, missing files, invalid ranges, etc. |
| Configuration validation | Validates project config files and their interactions with source documents. |
| Policy/rule validation | Enforces house style, required metadata, naming conventions, required sections, etc. |
| Generated-content validation | Checks generated artifacts or preview output for consistency. |

## Refactoring and Transformation Features

| Feature | Description |
|---|---|
| Rename references | Renames anchors, variables, schema fields, symbols, files, or directives safely. |
| Move file updates | Updates links/imports/references when a file is moved. |
| Extract section/block | Extracts a Markdown section, code block, directive, or embedded expression. |
| Convert syntax | Converts one directive style to another, one frontmatter format to another, etc. |
| Add missing fields | Inserts required schema/frontmatter fields. |
| Normalize document structure | Reorders fields, sections, imports, references, or metadata. |
| Generate boilerplate | Creates frontmatter, schema stubs, document templates, or code snippets. |
| Migrate deprecated syntax | Replaces old DSL constructs with newer equivalents. |

## UI and Editor Experience Features

| Feature | Description |
|---|---|
| Code lenses | Shows inline actionable annotations above code or document regions, such as “3 references,” “preview,” “run,” “validate,” or “open generated output.” |
| Commands | Exposes server-defined commands callable from the editor command palette. |
| Status reporting | Reports server indexing state, validation status, project load errors, etc. |
| Progress reporting | Shows progress for indexing, validation, code generation, etc. |
| Show message / log message | Sends user-visible or log-only messages to the editor. |
| Workspace commands | Supports commands such as regenerate index, reload schemas, clear cache, preview document, etc. |
| Notebook/document-adjacent workflows | Some editors can use LSP-like features in notebooks or virtual documents, though support varies. |

## Advanced or Less Commonly Implemented Features

| Feature | Description |
|---|---|
| Partial result streaming | Sends large reference/search results incrementally. |
| Work-done progress | Reports long-running tasks with cancellable progress. |
| Cancellation support | Stops expensive operations when the user moves the cursor or closes a request. |
| Lazy resolution | Initially returns lightweight completions/code actions, then resolves details only when needed. |
| Dynamic registration | Registers capabilities at runtime depending on workspace/project type. |
| Pull diagnostics | Lets the editor request diagnostics instead of only receiving push diagnostics. |
| Virtual document support | Provides language features for generated, embedded, or synthetic documents. |
| Cross-language delegation | Coordinates with other language servers for embedded languages. |
| Remote/server-side indexing | Uses precomputed or shared indexes rather than building everything locally. |
| Custom protocol extensions | Adds non-standard features for specific editors or language ecosystems. |

## Feature Group Summary

| Category | Typical Features |
|---|---|
| Navigation | Definition, declaration, references, implementation, type definition, document symbols, workspace symbols |
| Feedback | Diagnostics, linting, schema errors, hover, warnings, hints |
| Completion | Identifiers, paths, anchors, schema fields, snippets, keywords |
| Editing | Formatting, code actions, quick fixes, rename, organize imports |
| Structure | Folding, selection ranges, semantic tokens, inlay hints, document highlights |
| Project awareness | Indexing, dependency graph, workspace diagnostics, config awareness |
| Refactoring | Safe rename, extract, convert, migrate, update references |
| Documentation | Hover docs, links, rendered Markdown, examples |
| Workflow | Commands, code lenses, preview hooks, progress reporting |

## High-Value LSP Features for a Markdown-Derived Language

For a Markdown-derived language like **Darkmatter**, the highest-value LSP features would likely be:

| Priority | Feature | Why It Matters |
|---:|---|---|
| 1 | Diagnostics | Detect parser errors, invalid directives, unresolved references, malformed frontmatter, schema violations, etc. |
| 2 | Schema-aware frontmatter validation | Validate document metadata against project-defined schemas. |
| 3 | Completion | Complete frontmatter keys, schema values, paths, anchors, directive names, variables, and snippets. |
| 4 | Hover docs | Explain directives, schema fields, interpolation variables, transclusions, and document metadata. |
| 5 | Go to definition | Jump to transcluded files, schema definitions, variables, anchors, referenced documents, or included resources. |
| 6 | Link and anchor validation | Detect broken Markdown links, duplicate anchors, missing headings, and stale references. |
| 7 | Document symbols | Provide outline navigation over headings, sections, directives, and frontmatter structure. |
| 8 | Folding ranges | Fold frontmatter, sections, code fences, directive blocks, transclusions, and generated regions. |
| 9 | Semantic tokens | Highlight Markdown, frontmatter, directives, interpolation, embedded expressions, and custom DSL syntax. |
| 10 | Code actions | Add missing fields, fix broken references, create missing files, convert deprecated syntax, normalize metadata. |
| 11 | Safe rename/update of references | Rename headings, anchors, files, variables, schema keys, or directives while updating references. |
