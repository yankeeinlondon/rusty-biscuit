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

**Before** (sketch, generalized from `claudine/lib/src/stream/reporting.rs`):

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

**After**:

```rust
/// Convert a `StreamExecutionSummary` into a synthetic `SessionEnd`
/// `EventMeta` for JSONL logging.
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

**Before** (sketch):

```rust
/// ## Examples
///
/// ```
/// use claudine::config::ClaudineConfig;
/// use claudine::config::merge::merge_configs;
/// use std::collections::HashMap;
///
/// let mut base = ClaudineConfig::default();
/// base.events.insert(/* ...12 lines of setup... */);
///
/// let mut overlay = ClaudineConfig::default();
/// overlay.events.insert(/* ...10 more lines... */);
///
/// let merged = merge_configs(base, overlay);
/// assert!(merged.events.contains_key(&AgenticEvent::PreToolUse));
/// ```
pub fn merge_configs(...) -> ClaudineConfig { ... }
```

**After**:

```rust
/// Merge an overlay config into a base config.
///
/// See [`claudine/lib/tests/merge_tests.rs`] for full overlay-precedence
/// examples.
pub fn merge_configs(...) -> ClaudineConfig { ... }
```

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
does not express.

### A. Contract or invariant not derivable from types

**Example** (`claudine/lib/src/config/atomic.rs::atomic_write`):

```rust
/// ## Concurrency
///
/// Each call uses a unique temp file, so concurrent writers never corrupt
/// each other's in-flight bytes. `rename` serializes on the parent
/// directory inode; the final content is always an intact copy of exactly
/// one writer's payload (last-rename-wins).
```

Nothing in the signature `pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()>`
tells the reader about concurrent-writer behavior or that the function
guarantees torn-write resistance. Keep this kind of comment.

### B. WHY a counter-intuitive choice was made

**Example** (`claudine/lib/src/config/atomic.rs::atomic_write`):

```rust
// `persist` performs an atomic `rename(2)` on the same filesystem.
// If it fails (cross-device, filesystem error), surface the error
// rather than fall back to a non-atomic copy that could leave the
// target truncated on a crash mid-write.
tmp.persist(path).map_err(|e| e.error)?;
```

The `?` operator looks unremarkable — a reader might assume a fallback
path was forgotten. This comment lives *at the surprising line* and
explains why the fallback is intentionally absent. That's where a
Criterion-B comment belongs, not in the surrounding docblock.

### C. Semantics of complex return shapes

**Example** (`claudine/lib/src/harness/parse/frontmatter.rs::extract_frontmatter_text`):

```rust
/// ## Returns
///
/// `Some((yaml_text, base_line))` where `base_line` is the 1-indexed line
/// number of the first body line of the frontmatter (the line immediately
/// after the opening `---`). `None` if the file does not start with a `---`
/// fence on its first line, or if no closing fence is found.
pub(super) fn extract_frontmatter_text(source: &str) -> Option<(&str, usize)> { ... }
```

The return type is `Option<(&str, usize)>`. The `usize` could be a
length, a byte offset, an index, or a count. The doc names it: it is a
1-indexed line number, and the `None` arm has two distinct causes the
caller may want to distinguish (logging vs. error handling). Keep this.

### D. Hidden coupling or external surprise

**Example** (paraphrased from `claudine/lib/src/dispatch/loader.rs`):

```rust
/// Event binding ready for dispatch without per-event regex compilation.
///
/// Note: `compiled_mappers` is aligned with `actions` by index — the two
/// vectors must stay the same length, and `compiled_mappers[i]` describes
/// the mapper for `actions[i]`. Callers iterate them in lockstep.
pub struct RuntimeEventBinding { ... }
```

Nothing in `Vec<HookAction>` and `Vec<Option<CompiledMapper>>` tells the
reader the two are positionally coupled. Document the coupling once on
the type, then prune the redundant per-accessor docs (see Anti-pattern 5).

Other forms of Criterion D:

- "This struct must serialize compatibly with `X`" — protocols, config
  files, log formats.
- "This enum's discriminants are persisted to disk" — changing the
  numbers is a breaking change.
- "This list must be kept in sync with `provider_id::PROVIDERS_DISPLAY_ORDER`."

### E. Link to authoritative design

Module-level (`//!`) and module-defining-type (`///`) docs should link
to the design or topic doc that is the source of truth for that area,
when one exists.

**Example**:

```rust
//! Composition services for inline and chained document workflows.
//!
//! See [`claudine/docs/topics/composition.md`](../../docs/topics/composition.md)
//! for the full design — `compose`, `inline-compose`, `sequence`,
//! frontmatter precedence, harness validations.
```

Per-function linking is **not required**. Module-level / type-level is
the right granularity; threading the same link through every
`pub fn` produces a maintenance treadmill of broken intra-doc links.

When a topic doc doesn't exist yet, leave the link out. A reviewer's
judgement is the safety net here, not a heuristic.

## When in doubt

Ask: *would deleting this comment lose information that a future reader
needs?*

- **Yes** → keep, and check the language matches the code.
- **No** → delete.

The heuristic check at `scripts/check-comments.sh` flags suspicious
patterns but is intentionally not a CI gate. It surfaces candidates for
human review; the rubric, not the script, is policy.
