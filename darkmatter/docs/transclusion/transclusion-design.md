# Stage 2 Design: Transclusion Pipeline

## Skills Used

- rust
- darkmatter

## Purpose

Define a detailed technical design for Stage 2 transclusion in Darkmatter's compose pipeline, based on:

- `darkmatter/docs/block-transclusion.md`
- `darkmatter/docs/code-transclusion.md`
- `darkmatter/docs/fm-transclusion.md`
- `darkmatter/docs/darkmatter-compose-pipeline.md`

This design is meant to be implementation-ready and aligned with the current Stage 1 compose code in `darkmatter/lib/src/markdown/compose/`.

## Scope

In scope:

1. Block/code transclusion (`::file`, `::code`, `::url`) and file-link tree rendering (`::file-links`) design and execution model
2. Frontmatter transclusion (`prologue`, `epilogue`)
3. Recursive processing model, cycle detection, and depth limits
4. State inheritance semantics (parent -> child)
5. Path resolution (`./`, `/`, `~`, `@`)
6. Stage integration with existing `compose()` APIs and report/warning model
7. Error handling and testing strategy

Out of scope for first implementation:

1. Remote fetching implementation (`::url`) beyond parser-level support
2. Non-Markdown local formats for `::file` (`pdf`, `txt`, `csv`, `tsv`) beyond extension points
3. Stage 3 rendering/optimization features

## Current Baseline (Code Today)

Current pipeline implementation exists in `darkmatter/lib/src/markdown/compose/mod.rs` and supports Stage 1:

1. Replacement
2. Interpolation
3. Cleanup
4. Normalization

Relevant existing pieces to leverage:

- `ComposeOptions`, `ComposeReport`, warnings, `fail_fast`
- `EffectiveState` and context/environment snapshots
- Interpolation parser/evaluator for expression syntax and value semantics
- `Markdown::relevel()` for heading-level fitting

Current gaps for transclusion:

- No Stage 2 orchestration
- No source/origin tracking for path-relative includes
- No recursion runtime state (stack/depth/cycle)
- No directive parser for `::file` / `::code` / `::url`

## Functional Contract Summary

From the transclusion docs, Stage 2 must support:

1. Block transclusion syntax:
   - `::file <path> [key=value ...]`
   - `::code <path> [key=value ...]`
   - `::url <url> [key=value ...]` (future execution support)
2. Recursive processing:
   - markdown includes run through full compose pipeline
   - includes inside included markdown docs are supported
3. Cycle safety:
   - looped transclusion dependencies must be detected and rejected
4. Heading fitting:
   - markdown included via `::file` is re-leveled to fit insertion context
   - `::code` includes are fenced code blocks and are not re-leveled
5. Parent -> child state transfer:
   - child frontmatter overrides parent values
   - object values merge by key union
6. Block options:
   - `replace`
   - `quotation`
   - `disclosure`
   - `when`
   - these options are available on both `::file` and `::code`
7. Frontmatter transclusion:
   - `prologue` and `epilogue` as string or string array
8. Invalid reference behavior:
   - default: fail with meaningful error
   - ignore when `IGNORE_INVALID=true` or frontmatter `ignore_invalid=true`
9. Code transclusion behavior:
   - accepts text files (not limited to markdown extensions)
   - wraps included text in fenced code blocks
   - language inferred from extension, fallback to `txt`
   - exactly one blank line above and below generated code blocks

## Stage Placement and Ordering

Pipeline order remains:

1. Stage 1 (Preparation): replacement -> interpolation -> cleanup -> normalization
2. Stage 2 (Transclusion): directive transclusion (`::file`, `::code`, `::file-links`; `::url` future) -> frontmatter transclusion
3. Stage 3 (Rendering)

Rationale:

- Stage 1 runs first so file paths/conditions can depend on interpolation (`{{ env.* }}`)
- Stage 2 inserts already-prepared child content
- Stage 3 operates on fully composed markdown

## Public API and Option Changes

### `ComposeOptions` additions

Add transclusion-specific configuration while preserving existing APIs:

```rust
pub struct ComposeOptions {
    pub stages: Stage1Stages,
    pub stage2: Stage2Stages,
    pub transclusion: TransclusionOptions,
    pub external_state: Option<serde_json::Value>,
    pub fail_fast: bool,
    context: ComposeContext,
}

pub struct Stage2Stages {
    pub block_transclusion: bool,
    pub fm_transclusion: bool,
}

pub struct TransclusionOptions {
    pub source: ComposeSource,
    pub max_depth: usize,
    pub allow_remote: bool,
    pub allow_local_markdown: bool,
    pub allow_local_code_text: bool,
    pub code_fallback_language: String,
    pub ignore_invalid: Option<bool>,
    pub resolve_repo_root: bool,
}

pub enum ComposeSource {
    Unknown,
    File(std::path::PathBuf),
    Url(url::Url),
}
```

Defaults:

- `stage2.block_transclusion = true`
- `stage2.fm_transclusion = true`
- `source = Unknown`
- `max_depth = 16`
- `allow_remote = false`
- `allow_local_markdown = true`
- `allow_local_code_text = true`
- `code_fallback_language = "txt"`
- `ignore_invalid = None` (defer to env/frontmatter)

### `ComposeReport` additions

```rust
pub struct ComposeReport {
    // existing fields...
    pub transclusions_applied: usize,
    pub transclusions_skipped: usize,
    pub max_transclusion_depth: usize,
}
```

## Internal Module Layout

Recommended module layout:

```txt
darkmatter/lib/src/markdown/compose/transclusion/
├── mod.rs          # Stage orchestrator entry points
├── types.rs        # directives/options/errors/runtime structs
├── parser.rs       # block directive and option parsing
├── resolver.rs     # path/url resolution and canonicalization
├── engine.rs       # recursive execution and content assembly
├── code.rs         # code-fence wrapping and language inference
├── conditions.rs   # `when` evaluation
└── wrappers.rs     # quotation/disclosure formatting
```

`compose/mod.rs` adds a Stage 2 call after Stage 1 normalization.

## Core Data Model

```rust
pub enum DirectiveKind {
    File,
    Code,
    Url,
}

pub struct BlockDirective {
    pub kind: DirectiveKind,
    pub raw_target: String,
    pub options: BlockOptions,
    pub span: std::ops::Range<usize>, // replace range in source
    pub line: usize,
}

pub struct BlockOptions {
    pub replace: ReplaceOption,
    pub quotation: Option<String>,  // None = disabled, Some("") = true
    pub disclosure: Option<String>,
    pub when_expr: Option<String>,
}

pub enum ReplaceOption {
    InheritDefault,
    ParentWins,
    OneOff(serde_json::Map<String, serde_json::Value>),
}

pub struct TransclusionRuntime {
    pub stack: Vec<DependencyNode>,
    pub max_depth: usize,
    pub deepest_seen: usize,
}

pub struct DependencyNode {
    pub id: String, // canonical path or normalized URL
}
```

## Directive Parsing Design

### Block directive syntax

Grammar:

```text
directive   := "::" kind ws target (ws option)*
kind        := "file" | "code" | "url"
target      := quoted_string | bare_token
option      := key "=" value
key         := [A-Za-z_][A-Za-z0-9_-]*
value       := quoted_string | bare_token | json_object | json_array
```

Rules:

1. Directive must be on its own logical line (ignoring leading/trailing whitespace)
2. Directives inside inline code/fenced code blocks are ignored
3. Unknown option keys produce warning (or error in `fail_fast`)
4. `when` should be quoted when it contains spaces/operators

Implementation strategy:

1. Build excluded code regions (reuse interpolation finder approach with `pulldown-cmark` offsets)
2. Scan lines for `::` markers outside excluded regions
3. Parse directive head (`file|code|url`) and target
4. Parse trailing options via scanner that supports:
   - quoted values (`"..."`, `'...'`)
   - balanced JSON braces/brackets for `replace={...}` values

### Frontmatter transclusion parsing

1. Read `prologue` and `epilogue` from frontmatter
2. Accept:
   - string
   - array of strings
3. Convert each entry into synthetic internal directives
4. No block options/conditionals allowed for FM transclusion

## Path Resolution and Source Semantics

### Supported path forms

1. `./relative/path.md` or `../relative/path.md`: explicit relative — resolved from the current document directory only
2. `relative/path.md` (no leading `./`): implicit relative — resolved repository-root first, then the current document directory
3. `/absolute/path.md`: absolute path
4. `~/path.md`: home expansion via `HOME`
5. `@/path.md` or `@path.md`: repo-root relative

### Resolution algorithm

1. Resolve according to prefix and current `ComposeSource`
2. Canonicalize with filesystem resolution
3. Validate existence and file type
4. Produce canonical ID for cycle detection

Special handling:

- `ComposeSource::Unknown` cannot resolve `./` or `@` references
- repo-root resolution walks ancestors from current file until `.git` is found
- directive-specific content constraints:
  - `::file`: only markdown extensions (`.md`, `.markdown`) in first implementation
  - `::code`: any UTF-8 text file is allowed; binary/non-text files are rejected

Security boundary:

- repo-root (`@`) references must remain under canonical repo root after canonicalization
- invalid escape attempts fail with explicit error

## Code Transclusion Strategy (`::code`)

`::code` is a variant of block transclusion that reuses path resolution, option parsing, and `when` behavior from `::file`, but changes inclusion rendering:

1. load target as raw text (UTF-8)
2. do not parse it as markdown and do not recurse into child directives
3. optionally apply replacement rules (same `replace` option semantics, but against raw text content)
4. infer language from extension:
   - preferred: use existing syntax metadata from `syntect`/`two-face`
   - fallback: `code_fallback_language` (default `txt`)
5. wrap output in fenced code block
6. enforce exactly one blank line above and below inserted code block

Fence generation rule:

- choose a fence length longer than any consecutive backtick run in the included text (minimum 3) to avoid accidental fence termination.

Wrapper interaction:

- apply code fence first, then apply `quotation`/`disclosure` wrappers in the same order used by `::file`.

## State Inheritance and Merge Semantics

Stage 2 requires deeper merge semantics than current Stage 1 shallow merge.

Required behavior for parent -> child:

1. Parent effective state is passed as inherited defaults
2. Child frontmatter overrides inherited values
3. If both sides are objects, merge recursively by key union
4. For non-object conflicts, child value wins

### `replace` option semantics on transclusion

Baseline (no option):

- child `replace` map precedence over parent `replace`
- for `::code`, inherited/base `replace` map may be applied to raw included text

`replace=true`:

- invert precedence only for the `replace` map (parent wins over child)

`replace=<json/json5>`:

- merge one-off map into effective child `replace` for this transclusion only
- one-off map does not propagate to grandchildren

Implementation note:

- add deep merge helper in state layer (or dedicated transclusion state helper)
- avoid mutating persisted child frontmatter; apply overrides in effective state only

## `when` Condition Evaluation Design

### Reuse strategy

Reuse interpolation parser/evaluator infrastructure with transclusion condition extensions:

1. Add unary `!` support
2. Add function aliases/case-insensitive names:
   - `has_key`/`has_key`
   - `contains`/`contains`
   - `length`/`length`
   - `and`/`and`
   - `or`/`or`
3. Use boolean evaluation mode for `when`

### Semantics

1. Missing values evaluate as falsy
2. Equality/inequality compare normalized scalar strings
3. Numeric comparisons in `when` coerce non-numeric values to `0` (per functional spec)
4. `and(a,b,c)` requires all truthy
5. `or(a,b,c)` requires any truthy

`when` false behavior:

- directive is removed from output
- counted in `transclusions_skipped`

## Recursive Execution Algorithm

For each document in Stage 2:

1. Parse directive blocks from body
2. Resolve directives in source order (apply replacements from end to start)
3. For each directive:
   - resolve `when` (if present)
   - resolve reference (path/url)
   - enforce `max_depth`
   - branch by directive kind:
     - `::file` (markdown):
       - detect cycle via active recursion stack
       - load child markdown
       - build child inherited state
       - recursively run full compose on child (Stage 1 + Stage 2)
       - re-level child headings to fit insertion context
       - apply wrappers (`quotation`, `disclosure`)
     - `::code`:
       - load raw text file
       - apply optional replace map to raw text
       - infer language, generate safe fence, enforce vertical spacing
       - apply wrappers (`quotation`, `disclosure`)
     - `::url`:
       - parser-level support now; execution in future phase
4. After body replacements, process FM `prologue` and `epilogue` and prepend/append content

Cycle detection rule:

- cycle if canonical target ID already exists in current recursion stack
- repeated include in separate branches is allowed (not a cycle)
- cycle checks apply to recursive directive kinds (`::file`, future recursive `::url`)

## Heading Re-leveling Strategy

For markdown `::file` transclusion, compute insertion context level:

1. Determine nearest preceding heading level at directive location
2. Target child root level = `min(parent_level + 1, H6)`
3. Call `Markdown::relevel(target_level)`

If parent context is unknown (no heading before directive):

- keep child heading root unchanged

If re-level would overflow H6:

- `fail_fast=true`: return error
- otherwise: warning + include without re-leveling

`::code` transclusions are fenced blocks and skip heading re-leveling.

## Wrapper Rendering

### `quotation`

`quotation=true`:

- convert included block to blockquote lines (`> ...`)

`quotation="Attribution"`:

- same blockquote conversion plus attribution line:
  `> — Attribution`

### `disclosure`

Wrap included content in the render-time disclosure DSL triple rather than
inline HTML:

```md
::disclosure
Summary text
::details

...included content...

::end-disclosure
```

`disclosure=true` (or an empty summary) uses the default summary `"Details"`.
The DSL is lowered per render target during rendering (terminal block quote,
browser `<details>`/`<summary>`, etc.); no `<details>` HTML is emitted at
compose time. See [Disclosure Blocks](../rendering/disclosure.md).

Wrapper order if both present:

1. apply quotation formatting
2. wrap quoted content in disclosure block

## Error Model

Add typed transclusion errors:

```rust
pub enum TransclusionError {
    ParseDirective { line: usize, message: String },
    InvalidReference { reference: String, line: usize },
    MissingSourceContext { reference: String, line: usize },
    UnsupportedReferenceType { reference: String },
    UnsupportedFileType { path: std::path::PathBuf },
    NonTextCodeSource { path: std::path::PathBuf },
    CycleDetected { chain: Vec<String> },
    MaxDepthExceeded { max_depth: usize },
    Io(std::io::Error),
    Url(url::ParseError),
    ConditionParse { expr: String, line: usize, message: String },
    ConditionEval { expr: String, line: usize, message: String },
    Relevel(String),
}
```

Integrate as either:

1. `MarkdownError::Transclusion(#[from] TransclusionError)` (preferred)
2. `MarkdownError::Compose(String)` wrapping transclusion details

### Invalid reference policy

`ignore_invalid` resolution precedence:

1. explicit `ComposeOptions.transclusion.ignore_invalid`
2. frontmatter `ignore_invalid`
3. env `IGNORE_INVALID`
4. default `false`

Behavior:

- when `ignore_invalid=false`: fail pipeline
- when `ignore_invalid=true`: remove directive and add warning

## Determinism and Performance

Determinism:

1. Keep one `ComposeContext` snapshot for entire root run
2. Stable directive replacement order (source order, applied reverse by byte span)
3. Stable path canonicalization and ID generation

Performance:

1. Parse directives with a single scan per document
2. Reuse interpolation parser/evaluator for `when`
3. Optional optimization: cache raw loaded file content by canonical path

## Testing Strategy

### Unit tests

1. Directive parser:
   - quoted target, options, JSON option values, `file|code|url` kinds
   - ignore inside code spans/fenced code
2. Path resolver:
   - `./`, `/`, `~`, `@`
   - unknown source failures
3. Condition evaluator:
   - unary, comparisons, `and`/`or`, `has_key`, `contains`, `length`
4. State merge:
   - deep merge and precedence
   - `replace` option variants
5. Code helpers:
   - language inference by extension
   - safe fence sizing when source contains backticks
   - exact blank-line normalization around inserted code blocks

### Integration tests

1. Single include from local markdown
2. Nested recursive include chain
3. Cycle detection (A -> B -> A)
4. Code include from `.rs`/`.ts` with inferred language
5. Code include unknown extension falls back to `txt`
6. Code include rejects non-text/binary source
7. Prologue/epilogue insertion order
8. `when` false removes directive
9. quotation/disclosure wrappers (including wrapped code blocks)
10. heading re-leveling fit into parent section for `::file`
11. `ignore_invalid` via env/frontmatter/options

### Test tooling notes

1. Use `tempfile` for fixture trees
2. Use `serial_test` for env-dependent cases (`IGNORE_INVALID`)
3. Keep deterministic context with fixed test context where needed

## Implementation Milestones

1. **Milestone 1: Scaffolding**
   - Add Stage 2 options/types/report fields
   - Add transclusion module skeleton and no-op stage wiring
2. **Milestone 2: Local block transclusion core**
   - `::file`/`::code` parser + path resolver
   - local markdown load (`::file`) + raw text load (`::code`)
   - recursive execution, depth limit, cycle detection for markdown includes
3. **Milestone 3: State inheritance + re-leveling**
   - deep merge semantics
   - `replace` option behavior
   - insertion-level heading fitting
4. **Milestone 4: Conditions and wrappers**
   - `when` evaluator extensions
   - `quotation` and `disclosure`
   - code-fence language inference and vertical-spacing hardening
5. **Milestone 5: Frontmatter transclusion**
   - `prologue`/`epilogue` parsing and execution
   - `ignore_invalid` precedence finalization
6. **Milestone 6: Hardening**
   - warnings/summary polish
   - error message quality
   - full fixture suite and docs alignment

## Open Decisions

1. Should Stage 2 require `ComposeSource::File` for any relative/include usage, or support caller-provided base directory separately?
2. For root `external_state` merge behavior, should we keep current shallow `PreferExternal` semantics, or move all merges to deep/explicit policies?
3. Should unknown block options be warnings (forward-compatible) or errors (strict mode)?
4. If parent context is H6, should inclusion fail or clamp target to H6?
5. Should FM transclusion run before or after block transclusion when both are present? (this design uses block first, then FM prepend/append)
6. For `::code`, should `replace` be enabled by default from inherited state, or only when explicitly set on the directive?

## Summary

This design introduces Stage 2 transclusion as a recursive, cycle-safe composition engine integrated into the existing `compose()` pipeline. It reuses current Darkmatter strengths (state/context, interpolation parser/evaluator, releveling) while adding the missing runtime pieces: directive parsing, source/path resolution, recursion management, and transclusion-specific merge/condition semantics. It now also explicitly supports the `::code` variant with text-file loading, safe fenced-block generation, language inference, and wrapper/condition parity with block transclusion.
