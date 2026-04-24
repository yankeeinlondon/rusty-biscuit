# Compose Pipeline — Block Style Error Ideas

This document covers the 7 error enums in the compose pipeline and identifies key variants where the **Block Style Error** pattern (using `Status` + `StatusBlock` from `biscuit-terminal`) would meaningfully improve the user experience.

## Block Style Error Recap

A Block Style Error has two visual parts:

1. **Title line** — `Status::from_prose()` with `StatusState::Error`, rendering the error enum name in **bold red** followed by a **bold** descriptive title.
2. **Block body** — `StatusBlock` with a red vertical `┃` border, containing `Prose`-formatted descriptive text, contextual hints, and optionally code-block examples.

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>CycleDetected:</b> transclusion cycle detected")
    .body(Prose::new("A → B → C → A"))
    .hint("Remove one of the transclusions that closes the loop.")
```

---

## 1. `TransclusionError`

**File:** `darkmatter/lib/src/markdown/compose/transclusion/types.rs:241`
**Variants (19):** `ParseDirective`, `InvalidReference`, `MissingSourceContext`, `UnsupportedReferenceType`, `UnsupportedFileType`, `NonTextCodeSource`, `CycleDetected`, `MaxDepthExceeded`, `ConditionEval`, `ConditionParse`, `Relevel`, `UrlExecutionDisabled`, `InvalidFrontmatterAssignment`, `InvalidReassignedFrontmatterProperty`, `Io`, `UrlParse`, `FileReference`, `Json`

### Current messages

```
Failed to parse directive at line 42: expected file path
Transclusion cycle detected: ["a.md", "b.md", "a.md"]
Maximum transclusion depth exceeded (max: 10)
Invalid reference './missing.md' at line 15
```

### Key variant: `CycleDetected`

This is a high-value target because the chain is already captured but printed as a debug-format `Vec`. A block-style rendering can visualize the cycle as a path the user can follow.

**Suggested rendering:**

```
⤫ TransclusionError: Cycle detected
┃ A circular dependency was found in the transclusion graph.
┃ 
┃   docs/index.md
┃     → docs/api.md        (::file at line 12)
┃       → docs/index.md    (::file at line 8)
┃ 
┃ The chain loops back to docs/index.md.
┃ 
┃ Remove or restructure one of the transclusions above to break the cycle.
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>TransclusionError:</b> cycle detected")
    .body(Prose::new(&format_cycle_chain(&chain)))
    .hint("Remove or restructure one of the transclusions to break the cycle.")
```

### Key variant: `InvalidReference`

This is a frequent user-facing error. Currently it just says `Invalid reference 'X' at line Y` without explaining why it is invalid or what valid alternatives look like.

**Suggested rendering:**

```
⤫ TransclusionError: Invalid reference
┃ The transclusion target could not be resolved at line 15:
┃ 
┃   ::file ./missing.md
┃ 
┃ The file ./missing.md does not exist relative to the source document.
┃ 
┃ Supported directives: ::file, ::code, ::url
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>TransclusionError:</b> invalid reference")
    .body(Prose::new(&format!(
        "The transclusion target could not be resolved at line {line}:\n\n  \
         ::file {reference}\n\n\
         The file <b>{reference}</b> does not exist relative to the source document."
    )))
    .hint("Supported directives: ::file, ::code, ::url")
```

---

## 2. `DeferredSetError`

**File:** `darkmatter/lib/src/markdown/compose/transclusion/types.rs:96`
**Variants (2):** `InvalidAssignment`, `ReassignedProperty`

### Current messages

These are not `#[derive(Error)]` — they are captured as deferred records and resolved later as either hard errors or warnings depending on `ComposeOptions`. The eventual message is either:
- `"Invalid frontmatter assignment on '::file' directive at line X: reason (value: raw)"`
- `"Invalid reassigned frontmatter property 'name' on '::file' directive at line X"`

### Key variant: `InvalidAssignment`

The user wrote `set=<value>` where `<value>` is not a valid JSON5 object. The current error gives the raw value and a reason, but doesn't show the valid syntax.

**Suggested rendering:**

```
⤫ DeferredSetError: Invalid set= assignment
┃ The set= value on the ::file directive at line 23 is not a valid JSON5 object:
┃ 
┃   ::file ./child.md set={title: "missing quote}
┃ 
┃ The value must be a JSON5 object (e.g., set={title: "My Title"}).
┃ 
┃ Use --allow-invalid-frontmatter-assignment to downgrade to a warning.
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>DeferredSetError:</b> invalid set= assignment")
    .body(Prose::new(&format!(
        "The <b>set=</b> value on the ::file directive at line {line} \
         is not a valid JSON5 object:\n\n  {raw}\n\n\
         The value must be a JSON5 object (e.g., <cyan>set={{title: \"My Title\"}}</cyan>)."
    )))
    .hint("Use --allow-invalid-frontmatter-assignment to downgrade to a warning.")
```

---

## 3. `ConditionError`

**File:** `darkmatter/lib/src/markdown/compose/conditions.rs:14`
**Variants (2):** `Parse`, `Eval`

### Current messages

```
Failed to parse condition 'env.AGENT ==' at line 10: unexpected end of input
Failed to evaluate condition 'unknownFunc(x)' at line 5: Unknown function: unknownfunc
```

### Key variant: `Parse`

This is the more actionable variant. Users are writing `when=` expressions and getting parser errors that currently just say "unexpected end of input" or similar. A block-style error can show the expression and point to available syntax.

**Suggested rendering:**

```
⤫ ConditionError: Failed to parse when= expression
┃ Could not parse the condition expression at line 10:
┃ 
┃   when="env.AGENT =="
┃                       ^ unexpected end of input
┃ 
┃ Available operators: ==, !=, >, >=, <, &&, ||, !
┃ Available functions: and(), or(), contains(), hasKey(), length(), number(), round()
┃ Fallback syntax: {{ var | "default" }}
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>ConditionError:</b> failed to parse when= expression")
    .body(Prose::new(&format!(
        "Could not parse the condition expression at line {line}:\n\n  \
         <b>{expr}</b>\n\n{message}\n\n\
         Available operators: <cyan>== != > >= < && || !</cyan>"
    )))
    .hint("Use {{ var | \"default\" }} for fallback values.")
```

---

## 4. `ShellExpansionError`

**File:** `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs:297`
**Variants (9):** `ParseDirective`, `CommandNotFound`, `Blacklisted`, `ApprovalRequired`, `Denied`, `NotPreApproved`, `Timeout`, `ExecutionFailed`, `PolicyIo`

### Current messages

```
Command not found: 'sniff' at line 20
Command timed out after 10s: 'sleep 30' at line 5
Command failed (exit 1): 'git status' at line 8
Blacklisted command 'rm' at line 12: destructive filesystem operation
```

### Key variant: `ExecutionFailed`

This variant carries `stdout`, `stderr`, exit code, command, and origin. It is the richest error in the entire pipeline and currently condenses all of that into a single line. A block-style rendering can show the command, exit code, and captured output in a structured way.

**Suggested rendering:**

```
⤫ ShellExpansionError: Command failed with exit code 1
┃ Shell command at line 8 exited with a non-zero status:
┃ 
┃   ::shell git status
┃ 
┃ Exit code: 1
┃ 
┃ stderr:
┃   fatal: not a git repository
┃ 
┃ Use --when-error or --when-exit-code to handle expected failures.
```

**Implementation sketch:**

```rust
let mut body = Compose::default();
body.add_prose(Prose::new(&format!(
    "Shell command at {origin} exited with a non-zero status:\n\n  \
     <b>::shell {command}</b>\n\nExit code: <red>{code}</red>"
)));
if !stderr.is_empty() {
    body.add_prose(Prose::new(&format!("\n\nstderr:\n  <dim>{}</dim>", stderr.trim())));
}
if !stdout.is_empty() {
    body.add_prose(Prose::new(&format!("\n\nstdout:\n  <dim>{}</dim>", stdout.trim())));
}

StatusBlock::new(StatusState::Error)
    .header("<b>ShellExpansionError:</b command failed")
    .body(body)
    .hint("Use --when-error or --when-exit-code to handle expected failures.")
```

### Key variant: `CommandNotFound`

A common user mistake. The error should suggest checking PATH or installing the tool.

**Suggested rendering:**

```
⤫ ShellExpansionError: Command not found
┃ The executable could not be found at line 20:
┃ 
┃   ::shell sniff repo packages
┃ 
┃ The command <b>sniff</b> is not on your PATH.
┃ 
┃ Check that the tool is installed and available in your shell.
```

---

## 5. `TocLinkingError`

**File:** `darkmatter/lib/src/markdown/compose/toc_linking/types.rs:11`
**Variants (6):** `ParseDirective`, `InvalidCleanupService`, `InvalidLevel`, `FileNotFound`, `InvalidGlob`, `Io`

### Current messages

```
Invalid cleanup service 'emoji_strip' at line 5
Invalid heading level '7' at line 8
File not found './nav.md' at line 3
Invalid glob pattern '[invalid' at line 2: unclosed bracket
```

### Key variant: `InvalidCleanupService`

The user typed a cleanup service name that doesn't match any of the 5 available options. The current error just says it's invalid. A block-style error should list all valid services.

**Suggested rendering:**

```
⤫ TocLinkingError: Unknown cleanup service
┃ The cleanup service specified at line 5 is not recognized:
┃ 
┃   ::toc-linking ./nav.md --cleanup emoji_strip
┃ 
┃ Available cleanup services:
┃   emoji_leader  — strip leading emoji and trailing space
┃   emoji_trailing — strip trailing emoji and leading space
┃   emoji — strip all emoji sequences
┃   number — strip leading numeric index (e.g., 1.2.3)
┃   capitalize — capitalize first alphanumeric character
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>TocLinkingError:</b> unknown cleanup service")
    .body(Prose::new(&format!(
        "The cleanup service specified at line {line} is not recognized:\n\n  \
         <b>{service}</b>\n\n\
         Available cleanup services:\n  \
         <cyan>emoji_leader</cyan> — strip leading emoji\n  \
         <cyan>emoji_trailing</cyan> — strip trailing emoji\n  \
         <cyan>emoji</cyan> — strip all emoji\n  \
         <cyan>number</cyan> — strip leading numeric index\n  \
         <cyan>capitalize</cyan> — capitalize first character"
    )))
```

---

## 6. `PageBlockError`

**File:** `darkmatter/lib/src/markdown/compose/page_blocks/types.rs:7`
**Variants (4):** `ParseDirective`, `UnmatchedEnd`, `UnterminatedBlock`, `Condition`

### Current messages

```
Unmatched ::end-block at line 42
Unterminated ::block starting at line 10
Failed to parse page block at line 5: expected 'when=' option
```

### Key variant: `UnterminatedBlock`

A user opened `::block` but never closed it. The current error just gives the starting line number. A block-style error can show the opening directive and suggest adding `::end-block`.

**Suggested rendering:**

```
⤫ PageBlockError: Unterminated ::block
┃ A ::block directive starting at line 10 was never closed.
┃ 
┃   10: ::block when="production"
┃   ...
┃   (end of file)
┃ 
┃ Every ::block must have a matching ::end-block.
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>PageBlockError:</b> unterminated ::block")
    .body(Prose::new(&format!(
        "A <b>::block</b> directive starting at line {line} was never closed.\n\n\
         Every ::block must have a matching <cyan>::end-block</cyan>."
    )))
```

---

## 7. `CtxMergeError`

**File:** `darkmatter/lib/src/markdown/compose/context/merge.rs:10`
**Variants (1):** `InvalidUserCtx`

### Current messages

```
document `ctx` must be a JSON object, but found string
```

### Key variant: `InvalidUserCtx`

This is the only variant. The user defined `ctx` in their frontmatter as something other than an object (e.g., a string or array). The error should show the actual value and explain what is expected.

**Suggested rendering:**

```
⤫ CtxMergeError: Invalid document ctx
┃ The document's frontmatter defines ctx as a non-object type.
┃ 
┃   ---
┃   ctx: "some string"
┃   ---
┃ 
┃ The ctx field must be a JSON object. Runtime context keys like
┃ today, year, and os are merged into this object.
┃ 
┃ Use --allow-override to suppress this error and use runtime ctx only.
```

**Implementation sketch:**

```rust
StatusBlock::new(StatusState::Error)
    .header("<b>CtxMergeError:</b> invalid document ctx")
    .body(Prose::new(&format!(
        "The document's frontmatter defines <b>ctx</b> as a <red>{kind}</red> \
         instead of an object.\n\n\
         The ctx field must be a JSON object. Runtime context keys like \
         <cyan>today</cyan>, <cyan>year</cyan>, and <cyan>os</cyan> are merged into it."
    )))
    .hint("Use --allow-override to suppress this error and use runtime ctx only.")
```

---

## Summary of Priority Targets

| Error Enum | Variant | Why Block Style? |
|------------|---------|-----------------|
| `TransclusionError` | `CycleDetected` | Chain data already captured; visual path rendering would be much clearer than `Debug` format |
| `TransclusionError` | `InvalidReference` | High-frequency user error; needs file context and directive syntax hint |
| `DeferredSetError` | `InvalidAssignment` | Shows raw value + reason but no valid syntax example; permissive-mode flag is discoverable through hint |
| `ConditionError` | `Parse` | Parser errors are opaque; listing available operators and functions would reduce friction |
| `ShellExpansionError` | `ExecutionFailed` | Richest error in the pipeline (stdout/stderr/code/command); currently crammed into one line |
| `ShellExpansionError` | `CommandNotFound` | Common mistake; needs PATH/install hint |
| `TocLinkingError` | `InvalidCleanupService` | Enum has only 5 valid values; should list them all |
| `PageBlockError` | `UnterminatedBlock` | Structural error that benefits from showing the opening directive |
| `CtxMergeError` | `InvalidUserCtx` | Single variant; frontmatter example and `--allow-override` hint would be helpful |

## Component Mapping

| Component | Role in Block Style Error |
|-----------|--------------------------|
| `Status::from_prose(header)` | Title line: bold red error name + bold title |
| `StatusBlock::new(StatusState::Error)` | Container: severity-driven border color (red-500) |
| `StatusBlock::header(...)` | Prose-formatted title line rendered via `Status` |
| `StatusBlock::body(Prose::new(...))` | Red-bordered block with styled explanatory text |
| `StatusBlock::body(Compose::default())` | Multi-part body (text + code + output sections) |
| `StatusBlock::hint(...)` | Prose-formatted hint below the block |
| `Prose` tokens: `<b>`, `<red>`, `<cyan>`, `<dim>` | Inline styling within body and hint text |
