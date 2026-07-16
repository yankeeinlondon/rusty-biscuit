# Comment Quality

This document expands the **Comment Quality** rules from
[`CLAUDE.md`](../CLAUDE.md) with worked before/after examples drawn from
the repository. Use it to calibrate judgement when applying the rubric;
the rubric in `CLAUDE.md` is authoritative.

The structural rules in **Rustdoc Convention** (no `# H1`, which `## H2`
sections to use, what order they appear in) are unchanged. This document
is about *content* — what to write inside those sections, when to omit
them, and which patterns to avoid.

## Anti-patterns

### 1. HOW-narration in doc comments

Prose that restates the implementation step-by-step. The code already
says this; the prose drifts when the implementation changes.

**Before** (historical state of `claudine/lib/src/stream/reporting.rs::summary_to_event_meta`, pre-cleanup):

```rust
/// Convert a `StreamExecutionSummary` into an `EventMeta` suitable for
/// JSONL logging.
///
/// The resulting `EventMeta` has:
/// - `event = SessionEnd`
/// - `extra.synthetic = true`
/// - `extra.synthetic_kind = "stream_wrapper_summary"`
/// - `extra.stream_protocol` set to the protocol name
/// - Token usage, cost, duration, and other fields mapped into `extra`
pub fn summary_to_event_meta(...) -> EventMeta { ... }
```

**After** (current state of the same file — verified clean):

```rust
/// Convert a `StreamExecutionSummary` into a synthetic `SessionEnd`
/// [`EventMeta`] for JSONL logging.
pub fn summary_to_event_meta(...) -> EventMeta { ... }
```

The bullet list is a duplicate of what the function body literally does.
Anyone who needs that detail reads the function.

### 2. Tautological examples

Doc examples whose assertion is guaranteed by the signature or by a
literal in the body. They look like coverage but add no information.

**Before** (`claudine/lib/src/prompt_reporting/system_prompt.rs::render_system_prompt_header`):

```rust
/// ## Examples
///
/// ```
/// use claudine::prompt_reporting::render_system_prompt_header;
/// use biscuit_terminal::terminal::Terminal;
///
/// let term = Terminal::new();
/// let header = render_system_prompt_header("appended", &term);
/// assert!(header.contains("System Prompt"));
/// assert!(header.contains("appended"));
/// ```
```

The body of the function is
`format!("…System Prompt (<i>{action}</i>)…")`. Both asserts are
guaranteed by inspection of the signature. Delete the example, or
replace it with a real integration test that verifies the rendered
terminal escape sequence.

### 3. `## Arguments` / `## Returns` blocks that restate field docs

Don't repeat the parameter's own doc inside every consumer. Use these
sections only when the consumer adds a constraint or semantic the
parameter's type doesn't already express.

**Before**:

```rust
/// ## Arguments
///
/// - `path`: The path to write to.
/// - `content`: The bytes to write.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> { ... }
```

**After** (the actual contract in
`claudine/lib/src/config/atomic.rs::atomic_write` — see also
[Anti-pattern 8](#8-stale-comments-past-their-code)):

```rust
/// Write content to a file atomically using temp file + rename.
///
/// ## Concurrency
///
/// Each call uses a unique temp file, so concurrent writers never corrupt
/// each other's in-flight bytes. `rename` serializes on the parent
/// directory inode; the final content is always an intact copy of exactly
/// one writer's payload (last-rename-wins).
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> { ... }
```

The `## Arguments` block in the "before" adds nothing — `&Path` and
`&[u8]` already speak for themselves. The "after" replaces it with a
load-bearing concurrency contract (see Positive Criterion A).

### 4. Format-string, color, or glyph narration

Don't quote literal format strings, ANSI escape codes, color names, or
emoji codepoints in prose. They set up a contradiction the next time the
code changes.

**Before** (`claudine/lib/src/prompt_reporting/system_prompt.rs::render_system_prompt_header`,
present at time of writing):

```rust
/// Format: `<bg-orange-500><white><b>📔 System Prompt(<i>{action}</i>) </b></white></bg-orange-500>`
/// where action is `appended` or `replaced`. The entire header line — icon
/// included — is rendered as white text on an orange-500 background.
```

The code that emits this header is:

```rust
Prose::new(format!(
    "\n<orange-500><b>■ System Prompt (<i>{action}</i>)</b></orange-500>"
))
```

The doc claims a `📔` glyph, white-on-orange background, and `bg-orange-500`
markup; the code emits `■`, foreground-only orange, and no `bg-` prefix.
This is exactly Anti-pattern 8 (stale comments past their code) caused
by Anti-pattern 4 (format-string narration).

**After**:

```rust
/// Render the system-prompt header line shown above the prompt body.
```

If a specific styling invariant matters (e.g. "background must match
the BlockQuote bar so the badge and bar form one continuous shape"),
keep that — it is a Positive Criterion D ("hidden coupling") note. Drop
the literal escape codes.

### 5. Redundant docs on field-accessor methods

A one-line doc that restates the method name and return type adds no
information.

**Before** (`claudine/lib/src/dispatch/loader.rs::RuntimeEventBinding`):

```rust
impl RuntimeEventBinding {
    /// Whether the binding is enabled.
    pub fn enabled(&self) -> bool { self.enabled }

    /// The configured actions for the event.
    pub fn actions(&self) -> &[HookAction] { &self.actions }

    /// The compiled matcher, if any.
    pub fn matcher(&self) -> Option<&RuntimeMatcher> { self.matcher.as_ref() }

    /// Per-action compiled mapper metadata aligned with [`Self::actions`].
    pub fn compiled_mappers(&self) -> &[Option<CompiledMapper>] { &self.compiled_mappers }
}
```

**After**:

```rust
#[allow(missing_docs)]
impl RuntimeEventBinding {
    pub fn enabled(&self) -> bool { self.enabled }
    pub fn actions(&self) -> &[HookAction] { &self.actions }
    pub fn matcher(&self) -> Option<&RuntimeMatcher> { self.matcher.as_ref() }
    pub fn compiled_mappers(&self) -> &[Option<CompiledMapper>] { &self.compiled_mappers }
}
```

The struct itself can carry a docblock that explains what the binding
*is* and what the fields mean (Positive Criterion D — the alignment
between `actions` and `compiled_mappers` is genuine hidden coupling
worth noting once). The accessor methods don't need to repeat any of it.

`#[allow(missing_docs)]` at the impl block is preferable to per-method
`#[allow]` — one site to find, one to clear when the crate-level lint
changes.

### 6. Section-marker `//` comments inside functions

Comments like `// Protocol` immediately above
`extra.insert("protocol", value)` restate what the next line says. They
obscure the comments that *do* add information.

**Before** (`claudine/lib/src/stream/reporting.rs::summary_to_event_meta_with_context`):

```rust
// Synthetic markers
extra.insert("synthetic".into(), Value::Bool(true));
extra.insert("synthetic_kind".into(), Value::String("stream_wrapper_summary".into()));

// Protocol
let protocol_str = match protocol { ... };
extra.insert("stream_protocol".into(), Value::String(protocol_str.into()));

// Model
if let Some(model) = &summary.model {
    extra.insert("model".into(), Value::String(model.clone()));
}

// Token usage
if let Some(usage) = &summary.token_usage { ... }
```

**After**:

```rust
extra.insert("synthetic".into(), Value::Bool(true));
extra.insert("synthetic_kind".into(), Value::String("stream_wrapper_summary".into()));

let protocol_str = match protocol { ... };
extra.insert("stream_protocol".into(), Value::String(protocol_str.into()));

if let Some(model) = &summary.model {
    extra.insert("model".into(), Value::String(model.clone()));
}

if let Some(usage) = &summary.token_usage { ... }
```

If a section *does* carry meaning — for example, "this block has to run
before the others because downstream consumers key off `synthetic_kind`"
— that's a Positive Criterion B comment and worth keeping. The bare
section header is not.

### 7. Heavy-setup doc examples

A docblock example that needs ten or more lines of fixture construction
to make one assertion becomes its own maintenance burden. If the setup
is non-trivial, link to a real test instead.

`compile_canonical_runtime` in
`claudine/lib/src/dispatch/loader.rs` is the natural candidate for this
anti-pattern: it consumes a fully populated `ClaudineConfig`, so any
docblock example would need to construct the event-action map by hand.

**Before** (the shape a tempting `## Examples` block on
`compile_canonical_runtime` would take):

```rust
/// ## Examples
///
/// ```
/// use claudine::config::ClaudineConfig;
/// use claudine::dispatch::loader::compile_canonical_runtime;
/// use claudine::actions::HookAction;
/// use claudine::events::AgenticEvent;
/// use std::collections::HashMap;
///
/// let mut config = ClaudineConfig::default();
/// let mut actions = HashMap::new();
/// actions.insert(
///     AgenticEvent::BeforeTool,
///     vec![HookAction::Bash {
///         command: "echo".into(),
///         params: "before".into(),
///         when: None,
///     }],
/// );
/// config.actions = actions;
///
/// let runtime = compile_canonical_runtime(config, None).unwrap();
/// assert!(runtime.get_binding(&AgenticEvent::BeforeTool).is_some());
/// ```
pub fn compile_canonical_runtime(...) -> Result<CanonicalRuntimeConfig> { ... }
```

**After** (the rustdoc on `compile_canonical_runtime` today):

```rust
/// Compile a [`ClaudineConfig`] into a [`CanonicalRuntimeConfig`].
///
/// This iterates the flat event→actions map, compiles regex mappers for
/// `Call` actions, builds the protect service if enabled, and bridges
/// messenger settings to the existing [`RuntimeMessagingSettings`] type.
pub fn compile_canonical_runtime(...) -> Result<CanonicalRuntimeConfig> { ... }
```

End-to-end usage is covered by
`claudine/lib/tests/canonical_dispatch.rs`, which exercises the full
config-to-runtime pipeline against real dispatch fixtures. The summary
docblock plus the integration test is more durable than a contrived
fixture inlined in rustdoc.

For a smaller-scoped real example, see
`claudine/lib/src/config/merge.rs::merge_repo_override` — overlay
precedence is exercised by adjacent `#[cfg(test)]` tests, not a docblock
example.

Reserve doc examples for APIs where the example *teaches the type* in a
few short lines — those genuinely help. Heavy-setup examples don't.

### 8. Stale comments past their code

A comment that was accurate when written and is now wrong is worse than
no comment — it actively misleads.

**Before**: see Anti-pattern 4 above. The `render_system_prompt_header`
docblock describes a `📔` glyph and `bg-orange-500` background that the
code no longer emits.

**After**: rewrite or delete in the same commit that changes the
behavior. This is the **authoring-discipline** rule from `CLAUDE.md`:
any edit that changes a symbol's behavior must include a pass over

- the symbol's doc comment (`///` or `//!`),
- the surrounding module's doc, and
- any inline `//` comments inside the symbol.

Behavior-changing PRs that don't touch the relevant comments are an
invitation for drift.

## Positive criteria

A comment earns its length when it carries information the code itself
does not express. Each criterion below is paired with a real before/after
to calibrate what "load-bearing" looks like.

### A. Contract or invariant not derivable from types

**Before** (`claudine/lib/src/config/atomic.rs::atomic_write`):

```rust
/// Write content to a file atomically.
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> { ... }
```

The summary is true but leaves the load-bearing question unanswered:
*what happens when two writers race?* `&Path` and `&[u8]` carry no
information about concurrency or torn-write resistance.

**After**:

```rust
/// Write content to a file atomically using temp file + rename.
///
/// ## Concurrency
///
/// Each call uses a unique temp file, so concurrent writers never corrupt
/// each other's in-flight bytes. `rename` serializes on the parent
/// directory inode; the final content is always an intact copy of exactly
/// one writer's payload (last-rename-wins).
pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> { ... }
```

The `## Concurrency` block names the invariant — `last-rename-wins` — and
explains why concurrent calls are safe. Keep this kind of comment.

### B. WHY a counter-intuitive choice was made

**Before** (`claudine/lib/src/config/atomic.rs::atomic_write`):

```rust
tmp.persist(path).map_err(|e| e.error)?;
```

A reader scanning the function might assume the `?` is a forgotten
fallback — why not retry with a copy on cross-device failure?

**After**:

```rust
// `persist` performs an atomic `rename(2)` on the same filesystem.
// If it fails (cross-device, filesystem error), surface the error
// rather than fall back to a non-atomic copy that could leave the
// target truncated on a crash mid-write.
tmp.persist(path).map_err(|e| e.error)?;
```

The comment lives *at the surprising line* and explains why the absence
of a fallback is intentional. Criterion-B comments belong at the
surprising line — not in the surrounding function's docblock, where they
get lost.

### C. Semantics of complex return shapes

**Before** (`claudine/lib/src/harness/parse/frontmatter.rs::extract_frontmatter_text`):

```rust
/// Extract the YAML frontmatter text from a Markdown source.
pub(super) fn extract_frontmatter_text(source: &str) -> Option<(&str, usize)> { ... }
```

The signature has a tuple return — `(&str, usize)` — and a callers
reading this can't tell what the `usize` is. Byte offset? Line number?
Length? Index of the closing fence?

**After**:

```rust
/// Extract the YAML frontmatter text from a Markdown source.
///
/// ## Returns
///
/// `Some((yaml_text, base_line))` where `base_line` is the 1-indexed line
/// number of the first body line of the frontmatter (the line immediately
/// after the opening `---`). `None` if the file does not start with a `---`
/// fence on its first line, or if no closing fence is found.
pub(super) fn extract_frontmatter_text(source: &str) -> Option<(&str, usize)> { ... }
```

The `## Returns` block names the field, fixes the indexing convention,
and distinguishes the two `None` paths so a caller can decide whether to
log or error. Keep this — the type alone could not carry it.

### D. Hidden coupling or external surprise

**Before** (paraphrased from `claudine/lib/src/dispatch/loader.rs`):

```rust
pub struct RuntimeEventBinding {
    pub enabled: bool,
    pub actions: Vec<HookAction>,
    pub matcher: Option<RuntimeMatcher>,
    pub compiled_mappers: Vec<Option<CompiledMapper>>,
}
```

`Vec<HookAction>` and `Vec<Option<CompiledMapper>>` look like two
independent collections. Nothing in the field types says the two are
positionally aligned, or that mutating one without the other corrupts
the binding.

**After**:

```rust
/// Event binding ready for dispatch without per-event regex compilation.
///
/// Note: `compiled_mappers` is aligned with `actions` by index — the two
/// vectors must stay the same length, and `compiled_mappers[i]` describes
/// the mapper for `actions[i]`. Callers iterate them in lockstep.
pub struct RuntimeEventBinding {
    pub enabled: bool,
    pub actions: Vec<HookAction>,
    pub matcher: Option<RuntimeMatcher>,
    pub compiled_mappers: Vec<Option<CompiledMapper>>,
}
```

Document the coupling once on the type, then prune the redundant
per-accessor docs (see Anti-pattern 5).

Other forms of Criterion D:

- "This struct must serialize compatibly with `X`" — protocols, config
  files, log formats.
- "This enum's discriminants are persisted to disk" — changing the
  numbers is a breaking change.
- "This list must be kept in sync with `provider_id::PROVIDERS_DISPLAY_ORDER`."

### E. Link to authoritative design

**Before** (a module that has a topic doc but no link from `//!`):

```rust
//! Composition services for inline and chained document workflows.

use std::path::Path;
// ...
```

The module summary is fine, but a reader who wants the design — what
"inline" means, how `sequence` differs from `compose`, what the
frontmatter precedence rules are — has no signpost to the topic doc.

**After**:

```rust
//! Composition services for inline and chained document workflows.
//!
//! See [`claudine/docs/topics/composition.md`](../../docs/topics/composition.md)
//! for the full design — `compose`, `inline-compose`, `sequence`,
//! frontmatter precedence, harness validations.

use std::path::Path;
// ...
```

Per-function linking is **not required**. Module-level / type-level is
the right granularity; threading the same link through every `pub fn`
produces a maintenance treadmill of broken intra-doc links.

When a topic doc doesn't exist yet, leave the link out. A reviewer's
judgement is the safety net here, not a heuristic.

## Applying the rubric in a cleanup pass

The first cleanup pass (the `2026-05-25-comment-quality` feature) needed
five review cycles. Most of the iterations would have been avoided by
applying this checklist before requesting review:

- **Comment-only commits must be filterable as such.** Run a
  content-only diff (e.g. `git diff -G '^\s*(///|//!|//)'`) on the
  commit; anything outside `///`/`//!`/`//` lines is not a comment
  change and belongs in a separate commit. Rendering tweaks, format
  strings, glyph swaps, and constant changes are *behavior* — split
  them out.
- **Don't bundle behavior changes with comment cleanup.** Even when the
  same file is being touched, a comment-cleanup commit must not change
  what the program does. The two-PR sequence is easier to review than
  one combined diff.
- **Every file path cited in this doc must be a verified "After" by
  feature completion.** Citing a file in the rubric is a public
  commitment that the file passes the rubric. Before declaring the
  cleanup complete, re-read every cited file against every cited
  anti-pattern.
- **The heuristic script (`scripts/check-comments.sh`) must exit clean
  against its own target scope before requesting review.** A non-empty
  baseline either means the scope is not actually cleaned, or each
  remaining finding must be documented in the spec as an accepted
  exception.
- **Heuristic tools need fixture tests covering the language constructs
  they claim to support.** Spot checks against the current tree prove
  only that the current tree is clean; they don't prove the heuristic
  catches what it's supposed to catch. Cover multi-line signatures,
  single-line bodies, and the four flagged categories with fixtures
  before trusting the script as review tooling.
- **Whole-codebase acceptance criteria must be phrased as deltas.**
  Criteria like "`cargo doc` produces no broken intra-doc link
  warnings" are unsatisfiable against a codebase with pre-existing
  warnings. Phrase as "no new warnings introduced," quantify against a
  recorded baseline, or scope to the files the feature touches.

The rubric, not the checklist, is the policy. The checklist is how the
policy gets applied without the iteration cost the first time around.

## When in doubt

Ask: *would deleting this comment lose information that a future reader
needs?*

- **Yes** → keep, and check the language matches the code.
- **No** → delete.

The heuristic check at `scripts/check-comments.sh` flags suspicious
patterns but is intentionally not a CI gate. It surfaces candidates for
human review; the rubric, not the script, is policy.
