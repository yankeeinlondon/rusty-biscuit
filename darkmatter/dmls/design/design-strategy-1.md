# LSP Technical Strategy

## Executive Summary

The Composition LSP provides language intelligence for documents written in the [DarkMatter DSL](../features/darkmatter-dsl.md). Rather than forking or wrapping an existing Markdown LSP, we will **build a purpose-built LSP** using `tower-lsp` that leverages `pulldown-cmark` (already in our tech stack) as the Markdown parsing foundation, with a custom pre-processor layer to handle DarkMatter-specific syntax.

This approach is chosen because:

1. DarkMatter's block-level directives (`::file`, `::summarize`, `::table`, etc.) require semantic understanding that no existing Markdown LSP provides
2. Our interpolation syntax (`{{variable}}`) and file reference system (`@path`) need context-aware completion tied to frontmatter and project structure
3. Existing Markdown LSPs (marksman, markdown-oxide) are optimized for PKM/wiki-link workflows, not document composition

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                        tower-lsp Server                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────┐    ┌─────────────────┐    ┌────────────────┐  │
│  │ Document Manager│◄───│  Change Events  │◄───│  LSP Protocol  │  │
│  │  (ropey + cache)│    │  didOpen/Change │    │  JSON-RPC      │  │
│  └────────┬────────┘    └─────────────────┘    └────────────────┘  │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                   DarkMatter Parser Layer                    │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │   │
│  │  │ Pre-process │─▶│pulldown-cmark│─▶│ DarkMatter AST     │  │   │
│  │  │ Directives  │  │   Parser     │  │ (enriched nodes)   │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  │   │
│  │                          │                                   │   │
│  │                   ┌──────┴──────┐                            │   │
│  │                   │ Source Map  │                            │   │
│  │                   │ (positions) │                            │   │
│  │                   └─────────────┘                            │   │
│  └─────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                    Feature Handlers                          │   │
│  │  ┌──────────┐ ┌──────────┐ ┌────────────┐ ┌──────────────┐  │   │
│  │  │Completion│ │  Hover   │ │Diagnostics │ │ Navigation   │  │   │
│  │  │ Provider │ │ Provider │ │  Provider  │ │  Provider    │  │   │
│  │  └──────────┘ └──────────┘ └────────────┘ └──────────────┘  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Core Dependencies

```toml
[dependencies]
# LSP Framework
tower-lsp = "0.20"
lsp-types = "0.95"
tokio = { version = "1", features = ["full"] }

# Markdown Parsing (from library module)
pulldown-cmark = "0.13"

# Text Buffer Management
ropey = "1.6"

# Concurrent State
dashmap = "5"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error Handling
thiserror = "1"

# Logging
tracing = "0.1"
```

## Parsing Strategy: Pre-processor + pulldown-cmark

Since `pulldown-cmark` (our chosen Markdown parser) doesn't have the extension point architecture of `markdown-rs`, we'll use a **two-phase parsing strategy**:

### Phase 1: DarkMatter Pre-processing

Before handing content to pulldown-cmark, we extract and track DarkMatter-specific constructs:

```rust
pub struct DarkMatterPreprocessor {
    /// Maps original source positions to processed positions
    source_map: SourceMap,
    /// Extracted DarkMatter directives with their locations
    directives: Vec<Directive>,
    /// Extracted interpolations {{ variable }}
    interpolations: Vec<Interpolation>,
    /// Extracted file references @path
    file_refs: Vec<FileReference>,
}

pub struct Directive {
    pub kind: DirectiveKind,
    pub range: Range,
    pub arguments: Vec<String>,
    pub body: Option<String>,
}

pub enum DirectiveKind {
    File,           // ::file
    Summarize,      // ::summarize
    Consolidate,    // ::consolidate
    Topic,          // ::topic
    Table,          // ::table
    Chart(ChartType), // ::bar-chart, ::line-chart, etc.
    Popover,        // ::popover
    Columns,        // ::columns
    Break,          // ::break
    Summary,        // ::summary
    Details,        // ::details
    End,            // ::end
}
```

The pre-processor:
1. Scans for `::directive` patterns at line starts
2. Extracts `{{variable}}` interpolations inline
3. Identifies `@path` file references after whitespace/break characters
4. Records source positions for all extracted elements
5. Optionally replaces directives with placeholder markers for pulldown-cmark

### Phase 2: Standard Markdown Parsing

Pass the (optionally transformed) content to pulldown-cmark:

```rust
use pulldown_cmark::{Parser, Event, Options, Tag};

pub fn parse_markdown(content: &str) -> Vec<(Event<'_>, std::ops::Range<usize>)> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    Parser::new_ext(content, options)
        .into_offset_iter()
        .collect()
}
```

### Phase 3: AST Enrichment

Merge the pulldown-cmark events with extracted DarkMatter constructs into a unified AST:

```rust
pub struct DarkMatterDocument {
    /// Standard Markdown structure
    pub markdown_events: Vec<MarkdownNode>,
    /// DarkMatter directives at their logical positions
    pub directives: Vec<PositionedDirective>,
    /// Frontmatter variables available for interpolation
    pub frontmatter: HashMap<String, serde_json::Value>,
    /// All interpolation sites in the document
    pub interpolations: Vec<Interpolation>,
    /// All file reference sites
    pub file_refs: Vec<FileReference>,
    /// Source map for position translation
    pub source_map: SourceMap,
}
```

## Document Manager

Efficient state management for open documents:

```rust
use dashmap::DashMap;
use ropey::Rope;
use std::sync::Arc;

pub struct DocumentManager {
    documents: DashMap<lsp_types::Url, DocumentState>,
    /// Project-level context for file completions
    project_context: Arc<ProjectContext>,
}

pub struct DocumentState {
    /// Efficient text buffer for incremental updates
    pub content: Rope,
    /// LSP document version
    pub version: i32,
    /// Cached parse result (invalidated on change)
    pub parsed: Option<DarkMatterDocument>,
    /// Last computed diagnostics
    pub diagnostics: Vec<lsp_types::Diagnostic>,
}

pub struct ProjectContext {
    /// Root directory of the project
    pub root: PathBuf,
    /// Cached list of valid files for @-completion
    pub file_index: RwLock<FileIndex>,
    /// Global frontmatter variables (from parent docs)
    pub inherited_frontmatter: RwLock<HashMap<String, serde_json::Value>>,
}
```

### Incremental Updates

Use `ropey` for efficient incremental text updates:

```rust
impl DocumentManager {
    pub fn apply_changes(
        &self,
        uri: &lsp_types::Url,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            for change in changes {
                if let Some(range) = change.range {
                    // Incremental change
                    let start = self.position_to_char_idx(&doc.content, range.start);
                    let end = self.position_to_char_idx(&doc.content, range.end);
                    doc.content.remove(start..end);
                    doc.content.insert(start, &change.text);
                } else {
                    // Full document replacement
                    doc.content = Rope::from_str(&change.text);
                }
            }
            // Invalidate cached parse
            doc.parsed = None;
        }
    }
}
```

## Feature Implementation

### 1. File Reference Autocomplete (`@`)

Triggered when user types `@` after whitespace or break characters:

```rust
impl Backend {
    async fn complete_file_reference(
        &self,
        doc: &DocumentState,
        position: Position,
        typed_prefix: &str,
    ) -> Vec<CompletionItem> {
        let project = &self.document_manager.project_context;
        let file_index = project.file_index.read().await;

        // Get document scope to filter valid extensions
        let doc_scope = self.determine_doc_scope(doc);
        let valid_extensions = doc_scope.valid_extensions();

        file_index.files
            .iter()
            .filter(|f| f.extension_matches(&valid_extensions))
            .filter(|f| f.path.fuzzy_matches(typed_prefix))
            .sorted_by_popularity(&project.usage_stats)
            .take(20)
            .map(|f| CompletionItem {
                label: f.relative_path.clone(),
                kind: Some(CompletionItemKind::FILE),
                detail: Some(f.absolute_path.clone()),
                insert_text: Some(f.relative_path.clone()),
                ..Default::default()
            })
            .collect()
    }
}
```

**Trigger conditions:**
- `@` at start of line
- `@` after whitespace (` @`, `\t@`)
- `@` after break characters (`(@`, `[@`)

### 2. Interpolation Autocomplete (`{{`)

Triggered when user types `{{`:

```rust
impl Backend {
    async fn complete_interpolation(
        &self,
        doc: &DocumentState,
        position: Position,
    ) -> Vec<CompletionItem> {
        let mut items = vec![];

        // 1. Page frontmatter variables
        if let Some(ref parsed) = doc.parsed {
            for (key, value) in &parsed.frontmatter {
                items.push(CompletionItem {
                    label: key.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(format!("Page variable: {}", value)),
                    insert_text: Some(format!("{}}}}}", key)),
                    ..Default::default()
                });
            }
        }

        // 2. Inherited frontmatter from parent pages
        let project = &self.document_manager.project_context;
        let inherited = project.inherited_frontmatter.read().await;
        for (key, value) in inherited.iter() {
            items.push(CompletionItem {
                label: key.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("Inherited: {}", value)),
                ..Default::default()
            });
        }

        // 3. Built-in utility variables
        items.extend(self.builtin_interpolation_completions());

        items
    }

    fn builtin_interpolation_completions(&self) -> Vec<CompletionItem> {
        vec![
            ("today", "Today's date (YYYY-MM-DD)"),
            ("yesterday", "Yesterday's date"),
            ("tomorrow", "Tomorrow's date"),
            ("day_of_week", "Full day name (Monday, Tuesday, ...)"),
            ("day_of_week_abbr", "Abbreviated day (Mon, Tue, ...)"),
            ("now", "Current UTC datetime"),
            ("now_local", "Current local datetime with timezone"),
            ("timezone", "User's timezone"),
            ("last_day_in_month", "Boolean: is today the last day?"),
            ("month", "Full month name"),
            ("month_abbr", "Abbreviated month"),
            ("month_numeric", "Numeric month"),
            ("season", "Current season"),
            ("year", "Current year"),
        ]
        .into_iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some(detail.to_string()),
            insert_text: Some(format!("{}}}}}", label)),
            ..Default::default()
        })
        .collect()
    }
}
```

### 3. Interpolation Styling (Semantic Tokens)

Provide semantic tokens for `{{variable}}` to enable badge-like highlighting:

```rust
impl Backend {
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let doc = self.document_manager.get(&uri)?;

        let mut tokens = vec![];

        if let Some(ref parsed) = doc.parsed {
            for interp in &parsed.interpolations {
                tokens.push(SemanticToken {
                    delta_line: interp.range.start.line,
                    delta_start: interp.range.start.character,
                    length: interp.length(),
                    token_type: INTERPOLATION_TOKEN_TYPE,
                    token_modifiers_bitset: 0,
                });
            }
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: encode_tokens(tokens),
        })))
    }
}
```

### 4. Directive Completion (`::`)

Autocomplete DarkMatter directives:

```rust
impl Backend {
    fn complete_directive(&self, prefix: &str) -> Vec<CompletionItem> {
        let directives = vec![
            ("file", "Transclude external file content", "::file ${1:./path}"),
            ("summarize", "AI-generated summary of file", "::summarize ${1:./path}"),
            ("consolidate", "Merge multiple files", "::consolidate ${1:./a.md} ${2:./b.md}"),
            ("topic", "Extract topic from files", "::topic \"${1:topic}\" ${2:./files}"),
            ("table", "Create table from data", "::table ${1:./data.csv}"),
            ("bar-chart", "Bar chart visualization", "::bar-chart ${1:./data.csv}"),
            ("line-chart", "Line chart visualization", "::line-chart ${1:./data.csv}"),
            ("popover", "Popover block", "::popover ${1:./content.md}\n${2:trigger text}\n::end-popover"),
            ("columns", "Multi-column layout", "::columns md: ${1:2}, xl: ${2:3}\n\n$0\n\n::end"),
            ("summary", "Disclosure summary", "::summary\n${1:Summary text}\n::details\n${2:Detail text}"),
        ];

        directives
            .into_iter()
            .filter(|(name, _, _)| name.starts_with(prefix))
            .map(|(label, detail, snippet)| CompletionItem {
                label: format!("::{}", label),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect()
    }
}
```

## Diagnostics

### Validation Rules

```rust
impl Backend {
    fn compute_diagnostics(&self, doc: &DarkMatterDocument) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // 1. Validate file references exist
        for file_ref in &doc.file_refs {
            if !self.file_exists(&file_ref.path) {
                diagnostics.push(Diagnostic {
                    range: file_ref.range,
                    severity: Some(self.file_ref_severity(file_ref)),
                    message: format!("File not found: {}", file_ref.path),
                    source: Some("darkmatter".into()),
                    ..Default::default()
                });
            }
        }

        // 2. Validate interpolation variables are defined
        for interp in &doc.interpolations {
            if !doc.frontmatter.contains_key(&interp.variable)
                && !self.is_builtin_variable(&interp.variable)
                && !self.is_inherited_variable(&interp.variable)
            {
                diagnostics.push(Diagnostic {
                    range: interp.range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: format!("Undefined variable: {}", interp.variable),
                    source: Some("darkmatter".into()),
                    ..Default::default()
                });
            }
        }

        // 3. Validate directive syntax
        for directive in &doc.directives {
            if let Err(e) = directive.validate() {
                diagnostics.push(Diagnostic {
                    range: directive.range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e.to_string(),
                    source: Some("darkmatter".into()),
                    ..Default::default()
                });
            }
        }

        // 4. Check for unclosed blocks
        diagnostics.extend(self.validate_block_pairs(doc));

        diagnostics
    }

    /// Required refs (ending with !) error, optional refs (?) silent, normal refs warn
    fn file_ref_severity(&self, file_ref: &FileReference) -> DiagnosticSeverity {
        if file_ref.required {
            DiagnosticSeverity::ERROR
        } else if file_ref.optional {
            return; // Don't report
        } else {
            DiagnosticSeverity::WARNING
        }
    }
}
```

## Server Capabilities Declaration

```rust
async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            // Incremental sync for efficient updates
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                        include_text: Some(false),
                    })),
                    ..Default::default()
                }
            )),

            // Completion for @, {{, and ::
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![
                    "@".into(),  // File references
                    "{".into(),  // Interpolation (triggers on second {)
                    ":".into(),  // Directives (triggers on second :)
                ]),
                resolve_provider: Some(true),
                ..Default::default()
            }),

            // Hover for directives and variables
            hover_provider: Some(HoverProviderCapability::Simple(true)),

            // Go-to-definition for file references
            definition_provider: Some(OneOf::Left(true)),

            // Find references to files
            references_provider: Some(OneOf::Left(true)),

            // Document symbols (headings, directives)
            document_symbol_provider: Some(OneOf::Left(true)),

            // Semantic tokens for interpolation highlighting
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(
                    SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::VARIABLE,  // {{interpolation}}
                                SemanticTokenType::KEYWORD,   // ::directive
                                SemanticTokenType::STRING,    // @file-reference
                            ],
                            token_modifiers: vec![],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        range: Some(true),
                        ..Default::default()
                    }
                )
            ),

            // Code actions for quick fixes
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),

            ..Default::default()
        },
        ..Default::default()
    })
}
```

## File Indexing Strategy

For responsive `@` file completions, maintain a background file index:

```rust
pub struct FileIndex {
    /// All files in project matching valid extensions
    pub files: Vec<IndexedFile>,
    /// Usage frequency for sorting completions
    pub usage_counts: HashMap<PathBuf, usize>,
    /// Last full scan timestamp
    pub last_scan: Instant,
}

impl FileIndex {
    pub async fn scan(root: &Path, extensions: &[&str]) -> Self {
        let mut files = vec![];

        for entry in WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !is_hidden(e) && !is_ignored(e))
        {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if extensions.contains(&ext.to_str().unwrap_or("")) {
                            files.push(IndexedFile::from_path(root, entry.path()));
                        }
                    }
                }
            }
        }

        Self {
            files,
            usage_counts: HashMap::new(),
            last_scan: Instant::now(),
        }
    }

    /// Incremental update on file system events
    pub fn update(&mut self, event: FileSystemEvent) {
        match event {
            FileSystemEvent::Created(path) => self.add_file(&path),
            FileSystemEvent::Deleted(path) => self.remove_file(&path),
            FileSystemEvent::Renamed(old, new) => {
                self.remove_file(&old);
                self.add_file(&new);
            }
        }
    }
}
```

## Integration with Library Module

The LSP should share parsing logic with the `/lib` module:

```
/lib
├── src/
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── preprocessor.rs    # DarkMatter pre-processor (shared)
│   │   ├── markdown.rs        # pulldown-cmark wrapper (shared)
│   │   └── ast.rs             # DarkMatter AST types (shared)
│   └── ...

/lsp
├── src/
│   ├── main.rs
│   ├── backend.rs             # tower-lsp LanguageServer impl
│   ├── document_manager.rs    # Document state management
│   ├── features/
│   │   ├── completion.rs
│   │   ├── hover.rs
│   │   ├── diagnostics.rs
│   │   ├── semantic_tokens.rs
│   │   └── navigation.rs
│   └── file_index.rs          # Project file indexing
```

The `/lsp` crate depends on `/lib` for parsing:

```toml
# /lsp/Cargo.toml
[dependencies]
composition-lib = { path = "../lib" }
```

## Performance Considerations

1. **Debounced Diagnostics**: Don't recompute on every keystroke
   ```rust
   // Debounce 300ms after last change before computing diagnostics
   self.diagnostics_debouncer.trigger(uri, Duration::from_millis(300));
   ```

2. **Lazy Parsing**: Only parse when features require it
   ```rust
   fn get_parsed(&self, doc: &mut DocumentState) -> &DarkMatterDocument {
       if doc.parsed.is_none() {
           doc.parsed = Some(self.parse(&doc.content));
       }
       doc.parsed.as_ref().unwrap()
   }
   ```

3. **Background File Indexing**: Don't block LSP startup
   ```rust
   tokio::spawn(async move {
       let index = FileIndex::scan(&root, &extensions).await;
       *project_context.file_index.write().await = index;
   });
   ```

4. **Incremental Semantic Tokens**: Support range requests
   ```rust
   semantic_tokens_provider: Some(SemanticTokensOptions {
       full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
       range: Some(true),
       ..
   })
   ```

## Testing Strategy

1. **Unit Tests**: Parser and pre-processor logic
2. **Integration Tests**: Full LSP request/response cycles
3. **Multi-Editor Testing**: Verify behavior in VS Code, Neovim, Helix

```rust
#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::*;

    #[tokio::test]
    async fn test_file_reference_completion() {
        let server = TestServer::new();
        server.open_document("test.md", "# Test\n@");

        let completions = server.completion(Position::new(1, 1)).await;

        assert!(completions.iter().any(|c| c.label.ends_with(".md")));
    }

    #[tokio::test]
    async fn test_interpolation_completion() {
        let server = TestServer::new();
        server.open_document("test.md", "---\ntitle: Hello\n---\n{{");

        let completions = server.completion(Position::new(3, 2)).await;

        assert!(completions.iter().any(|c| c.label == "title"));
        assert!(completions.iter().any(|c| c.label == "today"));
    }
}
```

## Implementation Phases

### Phase 1: Foundation
- [ ] tower-lsp server skeleton with stdio transport
- [ ] Document manager with ropey
- [ ] Basic document sync (open/change/close)
- [ ] Integration with lib's pulldown-cmark parsing

### Phase 2: Core Features
- [ ] DarkMatter pre-processor (directives, interpolations, file refs)
- [ ] `@` file reference completion
- [ ] `{{` interpolation completion
- [ ] `::` directive completion

### Phase 3: Intelligence
- [ ] Semantic tokens for syntax highlighting
- [ ] Diagnostics (missing files, undefined variables, syntax errors)
- [ ] Hover information for directives and variables
- [ ] Go-to-definition for file references

### Phase 4: Polish
- [ ] Code actions (create missing file, define variable)
- [ ] Document symbols/outline
- [ ] Performance optimization
- [ ] VS Code extension packaging
