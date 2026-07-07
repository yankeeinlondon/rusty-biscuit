---
prompt: |-
  DMLS (the Darkmatter Language Server) will reuse Darkmatter's own parsers
  as the single grammar authority. Architectural decision AD-6 in
  @darkmatter/features/2026-07-04-dmls/design.md was accepted as a
  library-biased hybrid: anything defining what a construct MEANS lives in
  the `darkmatter` library (span-aware public APIs); anything that only
  locates text or tolerates half-typed input may live in DMLS.

  Your task is a code-level inventory of the `darkmatter` library
  (darkmatter/lib/src/**) answering, for each DSL surface, three questions:
  (a) does a real parser exist, (b) does it expose source spans/positions
  publicly, and (c) what API additions would DMLS need?

  Surfaces to inventory:

  1. Frontmatter extraction (delimiter detection, `extract_frontmatter_text`
     and its `base_line` semantics).
  2. Interpolation expressions (`{{ ... }}`) — the expression parser, its
     AST, identifier/namespace resolution (`doc.*`, `ctx.*`, `env.*`),
     function calls, and whether token positions survive parsing.
  3. Condition expressions (page blocks `::block`/`::end-block`).
  4. Directive parsing: `::file`, `::code`, `::toc-linking`, `::file-links`,
     `::shell`, `::shell-block`, disclosure triple — where are directive
     lines recognized, are options/arguments parsed into structures, and
     with what position info?
  5. `$()` shell-token parsing in frontmatter values.
  6. SimplifiedSchema parsing (`darkmatter::markdown::schemas`): schema AST,
     descriptor catalog, and validation error types — what path/location
     data do validation errors carry today?
  7. `style:` frontmatter parsing and its warning/error types.
  8. `LanguageGrammar` fence-token resolution.
  9. Markdown link / reference extraction (`markdown::reference`) and the
     TOC/heading model (slug generation authority).

  For each: cite file paths and key types, classify as
  ready / needs-span-API / needs-new-parser, and sketch the minimal API
  addition (signature-level) where needed. Close with a prioritized list of
  library changes DMLS v1 requires, flagging any that risk disturbing
  compose behavior.

  This is repository research — read the code, do not guess. Do not modify
  any source files.
last_updated: 2026-07-06
hash: 5b5addb9971651fa-5b4ad508141b3ff2
---
## DMLS Parser Reuse Inventory

DMLS should treat `darkmatter` as the grammar authority for construct semantics, but the current public API is uneven: several production parsers exist, some are crate-private, and most semantic ASTs do not preserve token spans. The main library work for DMLS v1 is to expose span-aware parse products without changing compose-time behavior.

### Classification Summary

| Surface                                             | Parser exists?                                | Public spans/positions?                                                                 | DMLS status    |
|-----------------------------------------------------|----------------------------------------------:|----------------------------------------------------------------------------------------:|----------------|
| Frontmatter extraction                              | Yes, private                                  | Partial via `Frontmatter::raw_source`; no public source block API                       | needs-span-API |
| `{{ ... }}` expressions                             | Yes, public semantic parser                   | Finder has wrapper byte spans; AST/tokens drop spans                                    | needs-span-API |
| `when=` condition expressions                       | Yes, same expression parser in condition mode | Parse errors expose approximate byte position; AST drops spans                          | needs-span-API |
| `::file`, `::code`, `::toc-linking`, `::file-links` | Yes                                           | Directive line spans and line numbers; option/argument spans absent                     | needs-span-API |
| `::shell`                                           | Yes                                           | Directive line span and line number; shell tokens no spans                              | needs-span-API |
| `::shell-block`                                     | Yes, mostly private                           | Block pair spans private; parsed region private                                         | needs-span-API |
| Disclosure triple                                   | Yes, render-tree parser                       | Synthetic event/range is crate-private; opener style has no spans/errors                | needs-span-API |
| Frontmatter `$()`                                   | Yes, private                                  | Key line only; no value/body/token spans                                                | needs-span-API |
| SimplifiedSchema                                    | Yes                                           | Grammar errors have spans; AST has no spans; validation has path + optional line/column | needs-span-API |
| `style:` frontmatter                                | Yes                                           | Warning type has optional span, always `None`; errors path-only                         | needs-span-API |
| `LanguageGrammar` fence token                       | Yes, public                                   | No span needed; caller owns fence range                                                 | ready          |
| Markdown references                                 | Yes                                           | `ReferenceOrigin` has line + byte span                                                  | ready          |
| TOC/heading model                                   | Yes                                           | Heading nodes have byte + line ranges; slug generator private                           | needs-span-API |

## 1. Frontmatter Extraction

Key files and types:

- `darkmatter/lib/src/markdown/frontmatter.rs`
    - `Frontmatter`
    - `Frontmatter::raw_source()`
    - `parse_frontmatter(content, ctx) -> MarkdownResult<(Frontmatter, String)>`
    - private `detect_near_miss_frontmatter_fence`

- `darkmatter/lib/src/markdown/mod.rs`
    - `Markdown::try_from_content`
    - `Markdown::frontmatter_line_count`
    - `Markdown::full_source_context_for_errors`
    - private `Markdown::reconstruct_source`

- `darkmatter/lib/src/markdown/schemas/mod.rs`
    - `positions_for`, offsets raw frontmatter lines by one leading delimiter line

A real parser exists, but it is not public. Frontmatter must start at file line 1 with a line whose trimmed text is exactly `---`; the closing delimiter is the next trimmed `---`. A leading dash-only fence of length `>= 4` can raise `MarkdownError::FrontmatterFenceMismatch` when it looks like YAML frontmatter.

`Frontmatter::raw_source()` publicly exposes only the YAML body between delimiters, not the delimiter spans, body span, closing delimiter line, or remaining body byte offset. `parse_frontmatter` joins body lines with `\n`, so original line endings are not preserved in the returned raw source.

The requested `extract_frontmatter_text` is not present in `darkmatter/lib/src/**`. The only repository references are comment-quality documentation and Claudine examples. Darkmatter’s closest equivalent is `parse_frontmatter`, plus `Frontmatter::raw_source()`.

Classification: needs-span-API.

Minimal API addition:

```rust
pub struct FrontmatterExtraction<'a> {
    pub yaml: &'a str,
    pub yaml_span: std::ops::Range<usize>,
    pub block_span: std::ops::Range<usize>,
    pub body_span: std::ops::Range<usize>,
    pub opening_line: usize,
    pub closing_line: usize,
    pub yaml_base_line: usize,
}

pub fn extract_frontmatter_block(source: &str) -> Result<Option<FrontmatterExtraction<'_>>, MarkdownError>;
```

`yaml_base_line` should be the 1-indexed source line of the first YAML line. For ordinary frontmatter this is `2`, because line `1` is the opening delimiter. That is the `base_line` semantic DMLS needs for YAML diagnostics.

Compose risk: low if added as a new API. Do not rewrite `parse_frontmatter` behavior or line-ending normalization in the same change.

## 2. Interpolation Expressions

Key files and types:

- `darkmatter/lib/src/markdown/compose/expression/lexer.rs`
    - `ExpressionFinder`
    - `ExpressionLocation { start, end, expression }`
    - `Lexer`, `Token`, `LexerError { position }`, `ParseMode`

- `darkmatter/lib/src/markdown/compose/expression/parser.rs`
    - `Parser`, `ParseError { message, position }`
    - `parse`, `parse_condition`

- `darkmatter/lib/src/markdown/compose/expression/ast.rs`
    - `Expr`, `BinaryOp`

- `darkmatter/lib/src/markdown/compose/expression/doc_namespace.rs`
    - crate-private `is_doc_namespace`, `resolve_doc_namespace`

- `darkmatter/lib/src/markdown/compose/expression/functions.rs`
    - function dispatch tables

- `darkmatter/lib/src/markdown/compose/expression/catalog.rs`
    - public `ExpressionFunctionDescriptor`
    - public `EXPRESSION_FUNCTION_DESCRIPTORS`

- `darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs`
    - `Evaluator`, `EvalResult`

A real parser exists and is public. `ExpressionFinder` locates `{{ ... }}` wrappers and returns byte offsets for the full wrapper in source. It intentionally skips fenced and indented code blocks but not inline code spans.

The expression grammar supports variables, literals, unary/binary arithmetic, comparisons, fallback, condition-mode logical OR, ternary, function calls, postfix member access, and bracket indexing. `parse` uses interpolation mode; `parse_condition` uses condition mode, where `||` lowers to `or(...)` instead of fallback.

Identifier and namespace behavior:

- `doc.*` is a reserved frontmatter namespace in `doc_namespace.rs`.
- `ctx.*` is runtime context, implemented through `EvaluationLookup` and `CtxLookup`.
- `env.*` resolves environment variables through lookup implementations.
- Unprefixed names resolve through `EvaluationLookup`; some production lookups fall back to `ctx.*`.

Function calls are authoritative through `functions.rs`; public descriptors live in `catalog.rs`.

Token positions do not survive parsing. `LexerError.position` is a byte offset, but `Parser.position` is only an approximate token counter because `Parser::advance` increments by `1` per token. `Token` has no span, and `Expr` has no span.

Classification: needs-span-API.

Minimal API addition:

```rust
pub type SourceSpan = std::ops::Range<usize>;

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: SourceSpan,
}

pub type SpannedExpr = Spanned<ExprKind>;

pub enum ExprKind {
    Variable(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BoolLiteral(bool),
    UnaryNot(Box<SpannedExpr>),
    UnaryMinus(Box<SpannedExpr>),
    Paren(Box<SpannedExpr>),
    Binary { op: BinaryOp, left: Box<SpannedExpr>, right: Box<SpannedExpr> },
    Index { base: Box<SpannedExpr>, index: Box<SpannedExpr> },
    MemberAccess { base: Box<SpannedExpr>, name: Spanned<String> },
    Fallback { primary: Box<SpannedExpr>, fallback: Box<SpannedExpr> },
    Ternary { condition: Box<SpannedExpr>, then_branch: Box<SpannedExpr>, else_branch: Box<SpannedExpr> },
    Comparison { left: Box<SpannedExpr>, op: Spanned<ComparisonOp>, right: Box<SpannedExpr> },
    FunctionCall { name: Spanned<String>, args: Vec<SpannedExpr> },
}

pub fn parse_spanned(input: &str) -> Result<SpannedExpr, ParseError>;
pub fn parse_condition_spanned(input: &str) -> Result<SpannedExpr, ParseError>;
pub fn lex_spanned(input: &str, mode: ParseMode) -> Result<Vec<Spanned<Token>>, LexerError>;
```

Compose risk: medium if existing `Expr` is replaced. Low if added in parallel and old `parse` lowers from spanned AST to existing `Expr`.

## 3. Condition Expressions

Key files and types:

- `darkmatter/lib/src/markdown/compose/conditions.rs`
    - `evaluate_condition`
    - `evaluate_condition_against`
    - `collect_condition_context_warnings`
    - `ConditionError::Parse { line, span }`

- `darkmatter/lib/src/markdown/compose/page_blocks/parser.rs`
    - `parse_page_blocks`
    - private `parse_block_options`

- `darkmatter/lib/src/markdown/compose/page_blocks/types.rs`
    - `PageBlockRegion { span, body_span, start_line, end_line, options }`
    - `PageBlockOptions { when_expr, unknown_options }`

A real parser exists because conditions use the shared expression parser in condition mode. Page block directive recognition is real and returns exact block byte spans plus line numbers.

However, `PageBlockOptions.when_expr` is only a `String`; it does not carry the `when` key span, value span, or parsed expression AST. Unknown options are only names. `ConditionError::Parse.span` is derived from `ParseError.position`, which is approximate because parser positions are token counts, not byte offsets.

Classification: needs-span-API.

Minimal API addition:

```rust
pub struct PageBlockDirective {
    pub span: std::ops::Range<usize>,
    pub body_span: std::ops::Range<usize>,
    pub start_line: usize,
    pub end_line: usize,
    pub options: Vec<DirectiveOption>,
}

pub struct DirectiveOption {
    pub key: String,
    pub key_span: std::ops::Range<usize>,
    pub value: String,
    pub value_span: std::ops::Range<usize>,
}

pub fn parse_page_blocks_spanned(content: &str) -> Result<Vec<PageBlockDirective>, PageBlockError>;
```

DMLS can then call `parse_condition_spanned` for the `when` value.

Compose risk: low if additive.

## 4. Directive Parsing

### `::file`, `::code`, `::url`

Key files and types:

- `darkmatter/lib/src/markdown/compose/transclusion/parser.rs`
    - `parse_directives`
    - private `parse_directive_line`

- `darkmatter/lib/src/markdown/compose/transclusion/types.rs`
    - `BlockDirective { kind, raw_target, options, span, line }`
    - `BlockOptions`

These directives are recognized line-by-line outside code regions. `::file-links` is explicitly excluded from `::file` prefix matching. Parsed output has the directive line span and line number. Target and options are structured semantically, but no target or option spans are preserved.

Classification: needs-span-API.

### `::toc-linking`

Key files and types:

- `darkmatter/lib/src/markdown/compose/toc_linking/parser.rs`
    - `parse_toc_linking_directives`

- `darkmatter/lib/src/markdown/compose/toc_linking/types.rs`
    - `TocLinkingDirective`
    - `TocLinkingOptions`
    - `CleanupService`, `HeadingGlob`, `LevelFilter`

The parser recognizes `::toc-linking`, parses target fallback chains and options, and returns directive span, line, indent, and inferred indent. Option spans are not retained.

Classification: needs-span-API.

### `::file-links`

Key files and types:

- `darkmatter/lib/src/markdown/compose/file_links/parser.rs`
    - `parse_file_links_directives`

- `darkmatter/lib/src/markdown/compose/file_links/types.rs`
    - `FileLinksDirective { mode, span, line, indent, inferred_indent }`
    - `FileLinksMode`

The parser recognizes exact `::file-links` keyword boundaries, supports glob and `--dir` forms, and returns directive line span and line number. Argument/flag spans are not retained.

Classification: needs-span-API.

### `::shell`

Key files and types:

- `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs`
    - `parse_directives`

- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs`
    - `tokenize`
    - `ShellToken`
    - `parse_pipeline`

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
    - `ShellDirective { raw_command, executable, args, span, indent, origin, ... }`
    - `ShellPipeline`, `PipelineAction`, `CommandAction`, `RedirectionConfig`

The parser recognizes `::shell ` after indentation/block quote prefix handling, skips code regions, tokenizes shell-like syntax, parses error-handling options, timeout, cache opt-out, and command chains. The directive has a line span. Shell tokens and options have no spans.

Classification: needs-span-API.

### `::shell-block` / `::end-block`

Key files and types:

- `darkmatter/lib/src/markdown/compose/block_pairs.rs`
    - crate-private `scan_block_pairs`
    - crate-private `BlockPair`

- `darkmatter/lib/src/markdown/compose/shell_blocks/parser.rs`
    - crate-private `parse_shell_block_region`

- `darkmatter/lib/src/markdown/compose/shell_blocks/types.rs`
    - crate-private `ShellBlockRegion`
    - crate-private `ShellBlockCommand { physical_span, start_line }`

A real parser exists but is mostly crate-private. The shared block-pair scanner handles both page blocks and shell blocks with exact block/body spans. Shell block opener options are parsed into `ErrorHandling`, timeout, and cache settings. Command body parsing tracks physical spans for logical commands.

Classification: needs-span-API, because the relevant types/functions are not public.

### Disclosure triple

Key files and types:

- `darkmatter/lib/src/markdown/render_tree/block_extension.rs`
    - crate-private `BlockExtensionProcessor`
    - crate-private `BlockExtensionEvent::Disclosure { summary_events, body_events, inline_style, range }`

- `darkmatter/lib/src/markdown/render_tree/disclosure_style.rs`
    - public `parse_disclosure_opener_style`

- `darkmatter/lib/src/markdown/types.rs`
    - `MarkdownError::MalformedDisclosure { reason, range }`

A real parser exists in the render-tree extension pass. It recognizes `::disclosure`, `::details`, and `::end-disclosure`, rejects malformed triples, supports nesting in the body, and returns a synthetic disclosure event with a byte range. This is not a public parse API. Inline opener style parsing returns `Option<CommonStyle>` and optional summary text, but no token spans and no structured warnings for invalid style tokens; invalid style-looking tokens simply stop style parsing and become summary text.

Classification: needs-span-API.

Minimal directive-wide API addition:

```rust
pub enum MarkdownDirectiveKind {
    File,
    Code,
    Url,
    TocLinking,
    FileLinks,
    Shell,
    ShellBlock,
    PageBlock,
    Disclosure,
}

pub struct ParsedDirective {
    pub kind: MarkdownDirectiveKind,
    pub span: std::ops::Range<usize>,
    pub line: usize,
    pub keyword_span: std::ops::Range<usize>,
    pub target: Option<Spanned<String>>,
    pub options: Vec<DirectiveOption>,
}

pub struct ParsedDirectiveBlock {
    pub opener: ParsedDirective,
    pub closer_span: Option<std::ops::Range<usize>>,
    pub body_span: std::ops::Range<usize>,
}

pub fn scan_darkmatter_directives(content: &str) -> Result<Vec<ParsedDirective>, DirectiveParseError>;
pub fn scan_darkmatter_blocks(content: &str) -> Result<Vec<ParsedDirectiveBlock>, DirectiveParseError>;
```

Compose risk: medium if scanners are unified. Low if existing parsers keep behavior and the new API is built by sharing lower-level cursor helpers.

## 5. Frontmatter `$()` Shell-Token Parsing

Key files and types:

- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
    - crate-private `parse_shell_value`
    - crate-private `FrontmatterShellAst`
    - crate-private `Branch`
    - crate-private `FrontmatterShellDirective`

- `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs`
    - public `tokenize`, `tokenize_simple`, `parse_pipeline`

- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
    - `ShellCommandOrigin::Frontmatter { key, line }`
    - `frontmatter_key_line`

A real parser exists, but the frontmatter `$()` entrypoint and AST are crate-private. It recognizes whole-value `$(...)`, suffixes `::timeout:N` and `::no-cache`, top-level ternary shell expressions, value-producing branches, command branches, and command pipelines. It uses `frontmatter_key_line` to map a top-level key back to a file line when `SourceContext` has a frontmatter range.

The public shell tokenizer has no spans. Frontmatter shell parse errors are key-scoped and may include a line, but not a value span, command body span, suffix span, branch span, or token spans.

Classification: needs-span-API.

Minimal API addition:

```rust
pub struct FrontmatterShellParse<'a> {
    pub key: &'a str,
    pub value_span: std::ops::Range<usize>,
    pub body_span: std::ops::Range<usize>,
    pub ast: FrontmatterShellAstPublic,
    pub suffixes: Vec<DirectiveOption>,
}

pub enum FrontmatterShellAstPublic {
    Pipeline(ShellPipelineSpanned),
    Ternary {
        condition: Spanned<SpannedExpr>,
        then_branch: FrontmatterShellBranch,
        else_branch: FrontmatterShellBranch,
    },
}

pub fn parse_frontmatter_shell_value_spanned(
    key: &str,
    value: &str,
    value_base_offset: usize,
    ctx: &SourceContext,
) -> Result<Option<FrontmatterShellParse<'_>>, ShellExpansionError>;
```

Compose risk: medium if private AST is made public directly. Prefer a public mirror or read-only diagnostic AST.

## 6. SimplifiedSchema

Key files and types:

- `darkmatter/lib/src/markdown/schemas/simplified/types.rs`
    - `SimplifiedSchema`
    - `SchemaShape`
    - `SchemaArm`
    - `PropertyDef`
    - `PropertyAtom`
    - `TypeExpr`
    - `SimplifiedType`
    - `Constraint`

- `darkmatter/lib/src/markdown/schemas/simplified/grammar.rs`
    - `parse_type_expr`
    - private lexer token spans

- `darkmatter/lib/src/markdown/schemas/simplified/mod.rs`
    - `parse_yaml_schema`

- `darkmatter/lib/src/markdown/schemas/about.rs`
    - descriptor catalog functions and descriptor structs

- `darkmatter/lib/src/markdown/schemas/errors.rs`
    - `SchemaError::Grammar { property, message, span }`

- `darkmatter/lib/src/markdown/schemas/mod.rs`
    - `DarkmatterSchemas`
    - `EffectiveSchema`
    - `ValidationReport`
    - `ValidationProblem`

- `darkmatter/lib/src/markdown/schemas/validate.rs`
    - `PositionMap`
    - `build_position_map`

A real parser exists. The string grammar lexer internally tracks byte spans and maps grammar failures to `SchemaError::Grammar { span }`. The parsed AST does not retain spans for properties, types, constraints, array suffixes, union arms, descriptions, or inline object fields.

The descriptor catalog is public through `about.rs` and expression-like descriptor structs. Validation errors carry:

- JSON pointer path: `ValidationProblem.path`
- message: `ValidationProblem.message`
- kind: `ValidationProblemKind`
- missing property name for required failures: `ValidationProblem.property`
- optional source line/column: `ValidationProblem.line`, `ValidationProblem.column`
- optional root union arm: `ValidationProblem.arm_index`
- optional schema description: `ValidationProblem.description`

Line/column data is top-level only. `validate::build_position_map` scans top-level YAML keys only and returns 1-based `(line, column)`. `schemas::positions_for` offsets raw frontmatter positions by one line because the raw source excludes the opening delimiter.

Classification: needs-span-API.

Minimal API addition:

```rust
pub struct SpannedSimplifiedSchema {
    pub schema: SimplifiedSchema,
    pub spans: SimplifiedSchemaSpans,
}

pub struct SimplifiedSchemaSpans {
    pub properties: indexmap::IndexMap<String, std::ops::Range<usize>>,
    pub atoms: Vec<std::ops::Range<usize>>,
    pub constraints: Vec<std::ops::Range<usize>>,
    pub descriptions: Vec<std::ops::Range<usize>>,
}

pub fn parse_type_expr_spanned(
    property: &str,
    input: &str,
) -> Result<Spanned<PropertyAtom>, SchemaError>;

pub fn parse_yaml_schema_spanned(
    value: &serde_yaml_ng::Value,
    source: &str,
) -> Result<SpannedSimplifiedSchema, SchemaError>;
```

For validation, DMLS would also benefit from nested YAML path positions:

```rust
pub type YamlPositionMap = indexmap::IndexMap<String, SourceSpan>;

pub fn build_yaml_position_map(yaml: &str) -> YamlPositionMap;
```

Compose risk: low for additive parse APIs. Medium if validation position mapping changes visible diagnostics.

## 7. `style:` Frontmatter

Key files and types:

- `darkmatter/lib/src/style/parse.rs`
    - `from_json_value`
    - `from_frontmatter`
    - `into_strict`
    - `ACTIVE_STYLE_WIRING_SUB_SPEC`

- `darkmatter/lib/src/style/schema/mod.rs`
    - `StyleFrontmatter`

- `darkmatter/lib/src/style/descriptor.rs`
    - schema leaf catalog and canonicalization helpers

- `darkmatter/lib/src/style/error.rs`
    - `StyleParseError`

- `darkmatter/lib/src/style/warning.rs`
    - `StyleWarning`
    - `StyleWarningKind`
    - `StyleSpan`

A real parser exists. It walks the JSON value under `style:`, emits warnings for unknown keys, deprecated snake_case aliases, and known-but-inactive keys, pre-validates typed leaves, and deserializes into `StyleFrontmatter`.

Errors are path-only (`style.page.left-margin`, raw value, reason, etc.). Warnings have `source_span: Option<StyleSpan>`, but the module documents that v1 always produces `None`.

Classification: needs-span-API.

Minimal API addition:

```rust
pub fn from_frontmatter_spanned(
    fm: &Frontmatter,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError>;

pub fn from_json_value_spanned(
    value: &serde_json::Value,
    positions: &YamlPositionMap,
) -> Result<(StyleFrontmatter, Vec<StyleWarning>), StyleParseError>;
```

Also add spans to `StyleParseError` variants:

```rust
pub fn source_span(&self) -> Option<StyleSpan>;
```

Compose risk: low if existing warning/error text is unchanged.

## 8. `LanguageGrammar` Fence-Token Resolution

Key file and type:

- `darkmatter/lib/src/markdown/language_grammar.rs`
    - `LanguageGrammar`
    - `LanguageGrammarError`
    - `from_token`
    - `from_extension`
    - `from_name`
    - `from_filename`
    - `from_lossy`
    - `from_token_or_plain_text`
    - `resolve_default`
    - `resolve`

A real public parser/resolver exists and is the declared grammar authority. `from_token` handles Markdown fence info strings by taking the first token or quoted token and ignoring metadata. Infallible fallback APIs are available for UI paths.

Classification: ready.

Minimal API addition: none required for DMLS v1. DMLS can compute the fence info-string span itself from Markdown tokenization and call:

```rust
LanguageGrammar::from_token_or_plain_text(info_string)
```

Optional convenience only:

```rust
pub fn split_fence_info(input: &str) -> Spanned<String>;
```

Compose risk: none if no change.

## 9. Markdown Link / Reference Extraction and TOC / Heading Model

Key files and types:

- `darkmatter/lib/src/markdown/reference/types.rs`
    - `ReferenceRecord`
    - `ReferenceOrigin { source, line, span, syntax }`
    - `ReferenceKind`
    - `ReferenceSyntax`
    - `ReferenceTarget`
    - `ReferenceSet`

- `darkmatter/lib/src/markdown/reference/mod.rs`
    - `Markdown::transclusions`
    - `Markdown::has_transclusions`

- `darkmatter/lib/src/markdown/reference/local.rs`, `html.rs`, `css.rs`
    - local/HTML/CSS extractors

- `darkmatter/lib/src/markdown/toc/mod.rs`
    - private `generate_slug`
    - private `extract_elements`
    - `impl From<&Markdown> for MarkdownToc`

- `darkmatter/lib/src/markdown/toc/types.rs`
    - `MarkdownToc`
    - `MarkdownTocNode`
    - `PreludeNode`
    - `CodeBlockInfo`
    - `InternalLinkInfo`

Reference extraction is span-ready. `ReferenceOrigin` carries 1-based line number and byte span, plus syntax and source. `Markdown::transclusions` reuses directive parsers and maps them into reference records.

The TOC model is mostly span-ready. `MarkdownTocNode` carries `source_span: (usize, usize)` and `line_range: (usize, usize)`. `PreludeNode` carries source span and line range. `CodeBlockInfo` carries line range. Internal links carry line/byte data in the extraction model.

The slug generator is the authority used by TOC extraction, but `generate_slug` is private. DMLS should not reimplement it if it needs exact slug matching.

Classification:

- Markdown references: ready.
- TOC model: needs-span-API only because slug generation and low-level heading extraction are private.

Minimal API addition:

```rust
pub fn generate_heading_slug(text: &str) -> String;

pub struct HeadingRecord {
    pub level: HeadingLevel,
    pub title: String,
    pub slug: String,
    pub title_span: std::ops::Range<usize>,
    pub heading_span: std::ops::Range<usize>,
    pub line: usize,
}

pub fn extract_headings(content: &str) -> Vec<HeadingRecord>;
```

Compose risk: low if this exposes existing logic unchanged. Medium if slug behavior is changed to match another platform.

## DMLS v1 Library Changes, Prioritized

1. Add a shared public span vocabulary.
    - Suggested: `SourceSpan = Range<usize>`, `Spanned<T>`, and line/column conversion helpers.
    - Risk: low.

2. Add public frontmatter block extraction.
    - DMLS needs delimiter, YAML body, source body, and line-base semantics.
    - Risk: low if additive.
    - Do not change `parse_frontmatter` normalization in this work.

3. Add span-aware expression lexing/parsing.
    - This unblocks interpolation diagnostics, condition diagnostics, completions, hover, and namespace/function semantic highlighting.
    - Risk: medium if existing `Expr` changes; low if `parse_spanned` is parallel.

4. Make directive scanning span-aware and public enough for DMLS.
    - Start with additive span APIs over existing scanners for `::file`, `::code`, `::toc-linking`, `::file-links`, `::shell`, page blocks, shell blocks, and disclosure.
    - Risk: medium because directive scanners are currently distributed and compose-sensitive.

5. Expose frontmatter `$()` parse products without exposing execution internals.
    - DMLS needs AST shape, branch spans, suffix spans, and shell token spans, not execution capability.
    - Risk: medium due to private AST coupling.

6. Add nested YAML position mapping and wire it into SimplifiedSchema and style diagnostics.
    - Current schema validation line/column is top-level only; style warnings have an empty span field by design.
    - Risk: medium if existing CLI diagnostic positions change. Prefer additive DMLS-only APIs first.

7. Export TOC slug generation.
    - DMLS should call Darkmatter’s slug authority directly.
    - Risk: low.

8. Keep `LanguageGrammar` unchanged for v1.
    - It is already the public grammar authority for fence-token resolution.
    - Risk: none.

The changes most likely to disturb compose behavior are any unification of directive scanners, replacement of the expression AST, or changes to frontmatter parsing/line normalization. The safer implementation path is additive: expose span-aware parse products while leaving compose’s existing semantic structs and execution pipeline intact.
