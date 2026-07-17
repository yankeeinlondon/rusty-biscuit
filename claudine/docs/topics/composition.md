---
hash: ef46db3751d8e999-9feef357d846d4b8
last_updated: 2026-07-17
---
# Claudine Composition

Claudine supports the ability to _compose_ content by leveraging the Darkmatter library's powerful composition features and routing the result through a wrapper-grade execution pipeline.

Two canonical commands:

- **`claudine compose [flags] <arg>...`** — direct (chained) composition
- **`claudine inline-compose [flags] <arg>...`** — inline composition

Both commands share the same five-stage pipeline and inherit full wrapper-grade behavior: environment setup, structured streaming, and lifecycle-driven recovery.

Because composition flows through the same execution path as `claudine claude` / `codex` / etc., it inherits every behavior of the live stderr surface documented in [Non-Interactive Sessions](non-interactive-sessions.md):

- **Tool call rendering** — `→ Name(summary)` / `← Name(slot)` with shell-name prefixing for `Bash` / `shell` / `run_command` and `description → subject → prompt → task` field order for `Task`.
- **Idle flush** — buffered assistant markdown is flushed by an independent 30-second ticker whenever the block buffer has been idle for 30 s, so a dangling final paragraph never sits invisible while a slow-to-close provider waits to exit.
- **Prompt-scoped timing header** — every 10 minutes the stderr surface emits `⏱️ {HH:MM} {TZ} running the <prompt> prompt for <duration>` anchored on the prompt's start time (and a `t=0` header without the duration at run start). See [Timing Surface](#timing-surface) below.
- **Typed error rendering** — `SemanticEvent::Error` is rendered as a colored `BlockQuote` whose label and border come from `SemanticErrorKind` (`Configuration`, `AgentNative`, `ApiRemote`, `Interrupted`, `Unknown`).
- **Reasoning / thinking** — provider reasoning (Claude, Codex, OpenCode, Gemini, Qwen) renders into `Section::Thinking` as a `BlockQuote` with the wider `▌ ` border that matches the System Prompt and Agent Prompt sections.

### Positional Arguments

Each command accepts exactly one file reference plus zero or more `key=value`
setters, in any order:

```sh
claudine compose @prompts/review.md review=review.md
claudine compose review=review.md @prompts/review.md
claudine inline-compose draft=false @notes/update.md
```

A token is a setter when it contains `=` and its key starts with an ASCII
letter or `_` and contains only letters, digits, `_`, or `-`. Dot-paths and
path-like tokens (for example `foo.bar=baz`) are not setters and are treated
as file-reference candidates.

Setter values are parsed as JSON5 first and fall back to strings when JSON5
parsing fails, so `count=3`, `enabled=true`, `tags=["a","b"]`, and
`review=review.md` all resolve to their natural types.

Inline setters override matching keys from `--set`. For `sequence`, reserved
per-step overlay keys still win over both `--set` and shorthand setters.

### Provider Argument Forwarding

Any CLI switch Claudine does not own is forwarded to the underlying agent,
mirroring the direct-wrapper contract. The first non-Claudine switch **after
the composition file** starts an agent tail; every token from there is passed
through verbatim:

```sh
# `-c model_reasoning_effort=low` is forwarded to Codex; the setter-shaped
# value is NOT applied as a frontmatter override.
claudine sequence fleet.md --codex -c model_reasoning_effort=low
```

No `--` is required. An explicit `--` after the file still works and forwards
its tail opaquely (no Claudine flag is extracted from it). Claudine-owned flags
always win before a `--` — a colliding native switch (e.g. Codex's own `-m`)
must be placed after `--`. The composition file must come first: an unowned
switch (or a `--`) before the file is an error with ordering guidance.

A generic INFO status names the forwarded switches (values redacted); `--dry-run`
shows the forwarded tail in its metadata table so a launch can be audited.
Because unknown switches are always forwarded, a genuinely invalid one may be
rejected by the agent at startup. See the mechanism in
[argv-normalization.md → Provider-argument partition](argv-normalization.md#provider-argument-partition).

### Shell Completion

Dynamic completion fires at markdown-expecting argument positions on all
three composition commands and on the `--append-system-prompt` /
`--replace-system-prompt` flag values — both on compose/inline-compose/
sequence themselves and on every wrapped provider subcommand (`claude`,
`codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`). All three
composition commands share one markdown-only contract; there is no
per-command frontmatter validation at completion time.

The generated bash/zsh/fish scripts shell out to `claudine __complete` on
every `<TAB>`. The supplement engine applies these rules in order:

- **Candidates are markdown files only** (`*.md`). Directories,
  non-markdown files, `./`/`../` traversal tokens, `!` package sigils,
  `vault:`, `/abs`, `%`, and `{{…}}` prefixes all return zero candidates.
- **Two supported entry forms**: `@`-prefixed magic paths (enumerated
  against repo root + user home) and implicit-relative paths like
  `prompts/…` (enumerated against the repo root only).
- **Typed-length scope**: 0–2 "meaningful characters" (leading `@` and
  segments before a `/` don't count) use the curated scope only —
  `prompts/` and `sequences/` under `<repo>/`, `<package-root>/`,
  `<package-area-root>/`, `~/`, and `~/.claudine/`. 3+ characters extend
  to a `.gitignore`-aware walk of the enclosing git repo.
- **Case-insensitive substring matching** on the filename with `.md`
  stripped for matching only. `@omp<TAB>` matches `prompt.md`.
- **`KEY=<TAB>` setters** offer schema-aware completion when the prompt
  declares `$schema` (property names before `=`; enum members and
  `file`-glob paths after `=`). Without a schema, the value slot still
  supports `@`-gated file completion; plain string/number values yield
  no candidates so shell default behavior kicks in. See
  [Shell Completions](completions/shell-completions.md).

Install with `claudine completions <shell>` — regenerate and reinstall
after a Claudine upgrade that changes the callback wiring. The hidden
`claudine __complete` subprocess always tracks the running binary, so
the completion candidates themselves never go stale. PowerShell and
Elvish retain the legacy one-line `COMPLETE=<shell>` bootstrap; users
who installed an older `COMPLETE=<shell>` snippet continue to reach the
legacy completion path on every shell until they regenerate. See
[completions/shell-completions.md](completions/shell-completions.md) for the full install
matrix, supported token shapes, and the open-questions list.

## Direct Composition

Direct composition takes a Markdown file, composes it through Darkmatter, and sends the composed content as a prompt to an agentic CLI. No files are mutated.

```sh
claudine compose @commit.md
claudine compose --codex @commit.md
```

Steps:

1. **Resolve** — resolve the file reference using `biscuit-file::FileReference`. A bare **implicit** path (`foo.md`, `dir/foo.md`) resolves repository-root first, then the source document's directory; an **explicit** `./`/`../` path resolves from the source directory only; `@` is a magic-root search, `!` a monorepo-package path, `~/` the user's home, `vault:` a configured vault, `%` a recursive modifier, and absolute paths resolve to themselves
2. **Compose** — run the Markdown through Darkmatter's compose pipeline (transclusion, interpolation, shell commands, conditionals)
3. **Prepare** — extract the effective (composed) frontmatter; this is the single source of truth for all downstream decisions
4. **Select provider** — choose which agentic CLI to use (see Provider Selection below)
5. **Execute** — run a non-interactive session (or interactive with `-i`) through the wrapper-grade pipeline

The composed prompt is sent to the provider. Output streams to the terminal with Markdown-to-terminal rendering in non-interactive mode.

> **Deferred lifecycle keys.** The Prepare stage composes every frontmatter key *except* the seven lifecycle event keys (`initialize`, `start`, `success`, `blocked`, `failure`, `finalize`, `loop`). Those keep their authored `{{ … }}` spans raw in `effective_frontmatter`; Claudine interpolates them through Darkmatter a **second time, at event-time**, so they can read the runtime globals (`err`, `timing`, `current`) and the live document state. See [lifecycle.md — When Lifecycle Properties Interpolate](lifecycle.md#when-lifecycle-properties-interpolate). The single exception is `shell` commands (positional `shell: "…"` or key/value `command:`), resolved against early-binding surfaces at pre-flight so the approved command is byte-identical to the executed one.

## Inline Composition

Inline composition uses the `prompt` frontmatter property as input and replaces the document's body with the provider's output.

```sh
claudine inline-compose @research.md
claudine inline-compose --claude @research.md
```

Steps:

1. **Resolve** — resolve the file reference
2. **Validate permissions** — confirm read + write access to the file
3. **Compose** — extract the `prompt` property, compose through Darkmatter, append inline guardrails
4. **Prepare** — extract effective frontmatter, capture pre-execution hashes for closure
5. **Select provider** — choose the agentic CLI
6. **Execute** — run the provider session
7. **Closure** — Claudine rewrites the file:
   - The replacement body is the agent's **final response only** — the output text emitted after the agent's last tool call. Interstitial narration between tool calls (e.g. "Let me read the docs…") is dropped, so process commentary never leaks into the artifact. Providers that recover their final message post-hoc (e.g. Codex's `--output-last-message`) supply that message directly.
   - The provider returns replacement body content only (no frontmatter)
   - Original frontmatter properties are preserved byte-for-byte
   - If the provider modified an existing frontmatter property, Claudine reverts it to the original value and emits a warning
   - If the provider added a new frontmatter property, Claudine merges it into the document (inserted before `last_updated`)
   - `last_updated` is set to today's date (local time, `YYYY-MM-DD`)
    - The file is written atomically
    - A cleanup pass normalizes the body markdown without touching frontmatter

### `hash` property (auto-stamped)

Every successful `inline-compose` closure stamps a Darkmatter `Simple` content
hash into the `hash:` frontmatter property as part of the same atomic write
that persists the body.

- **Format** — `<16-hex-fm>-<16-hex-body>` (for example,
  `hash: a1b2c3d4e5f60718-9a0b1c2d3e4f5061`).
- **Kind is forced to `Simple`** — even if the document previously held a valid
  `structured` or `detailed` hash, the next `inline-compose` run normalizes it
  to the `Simple` shorthand.
- **Self-reference stability** — `hash` and `last_updated` are excluded from the
  frontmatter segment when the hash is computed, so re-running `inline-compose`
  on an already-stamped, otherwise-unchanged document does not perturb the
  stored value. You can verify this with `md hash --diff <file>`, which exits
  `0` when the file matches its stored hash.
- **Malformed existing hash** — if the source file contains a malformed
  `hash:` value, the closure fails with `CompositionError::InlineHashMalformed`
  before any write occurs, leaving the file on disk untouched.

This behavior is implemented by [`apply_inline_closure`] in the closure module,
using `inline_hash_options`, `parse_inline_stored_hash`, `plan_hash_save`, and
`apply_hash_save`.

[`apply_inline_closure`]: ../../lib/src/composition/closure.rs

### Inline Conventions


- **`prompt`** (required) — the prompt text; composed through Darkmatter before execution
- **`last_updated`** — auto-updated by Claudine on each successful write
- **`hash`** — auto-stamped Darkmatter `Simple` content hash on each successful write (see [`hash` property (auto-stamped)](#hash-property-auto-stamped))
- **`agent`** — optional provider hint (see Provider Selection)
- **`policy`** — content freshness policy (coming soon)
- **`blast_radius`** — list of source files that trigger re-generation when changed

### Inline-Compose / Sequence Mismatch

A document that authors **both** a non-null `prompt` and a non-null `sequence`
defines an *inline sequence*: each sequence state is meant to invoke an
inline-compose operation using the `prompt`. Running such a document with
`inline-compose` would execute the prompt once, ignoring the sequence — so
`inline-compose` rejects it and directs the user to `claudine sequence`
instead.

```yaml
prompt: |-
  How do you say "{{state.name}}" in Italian?
sequence:
  - name: Hello
  - name: Goodbye
```

Detection rules:

- The mismatch triggers when the **authored** frontmatter has a `prompt` key
  whose value is not `null` **and** a `sequence` key whose value is not `null`.
  Value type and validity are never inspected — empty strings, empty lists,
  scalars, mappings, and other wrong-type-but-non-null values all count.
- `prompt: null`, `sequence: null`, or an absent key does **not** trigger the
  mismatch; ordinary `inline-compose` validation continues.
- Detection reads authored frontmatter only. Command-line `key=value` overrides
  and `--set` neither create nor suppress the mismatch.

The check runs **before** prompt-property validation, schema processing,
override application, composition, provider selection, and execution, so it is
fully fail-fast: no shell commands run, no provider launches, and the source
file is never mutated. Malformed frontmatter retains its existing
`FrontmatterParse` diagnostic, which takes precedence because no reliable
frontmatter keys are available to inspect.

The diagnostic identifies the resolved document (OSC8-linked when supported),
names both `prompt` and `sequence`, points to `claudine sequence`, and notes the
upcoming `sections` feature. Like every frontmatter-rooted composition error, it
appends the authored frontmatter as a syntax-highlighted, line-numbered YAML
block (see [Frontmatter YAML blocks in errors](#frontmatter-yaml-blocks-in-errors)).
When stderr is not a TTY the block is withheld to avoid exposing frontmatter,
and there is no flag to reveal it.

## Whole-Value Frontmatter Expansion Is Executable State

A frontmatter value whose trimmed content is *exactly one* expansion form —
either a single `{{ ... }}` interpolation span or a single `$(...)` shell
expression — is not ordinary text. It is executable state that downstream
pipeline stages (and the provider prompt) consume as a resolved value, so it
must parse and resolve successfully. Such a value must never leak into the
effective frontmatter as raw expansion syntax.

- **Whole-value `{{ ... }}` interpolation** must parse and evaluate
  successfully. A parse failure (e.g. the malformed `spec_path: "{{ dirname(review) + '/spec.md') }}"`)
  or an evaluation failure aborts composition with a precise
  `Interpolation parse failed` / `Interpolation evaluation failed` diagnostic
  naming the frontmatter key — **even when `fail_fast` is off**. Undefined
  variables remain lenient: a bare `{{ missing }}` still resolves to `null`
  rather than aborting.
- **Whole-value `$(...)` shell expansion** must parse and expand when
  frontmatter shell expansion is enabled. If shell expansion is explicitly
  disabled, the `$(...)` value is deferred unchanged. When enabled, a value
  that still trims to a whole-value `$(...)` candidate after the expansion pass
  is rejected as a leak.

This strictness is scoped to whole-value expansion only. **Mixed strings**
(`"prefix {{ x }} suffix"`, `"literal $(echo ok)"`) and **body prose**
interpolation are unchanged: when `fail_fast` is off they keep their lenient
behavior, leaving an unresolved span in place and recording a warning rather
than aborting. The enforcement lives in Darkmatter composition; see
[Frontmatter Interpolation](../../../darkmatter/docs/inline/fm-interpolation.md)
and [Frontmatter Shell Expansion](../../../darkmatter/docs/inline/fm-shell-expansion.md).

## Frontmatter YAML blocks in errors

Every composition error rooted in a prompt file's YAML frontmatter appends the
authored frontmatter — delimiters included — as a `CodeBlock`: syntax
highlighted, line-numbered so block line N equals source-file line N, and with
the offending line highlighted when the error's property maps to a locatable
key. This covers the lifecycle guards (interpolation leak, undefined variable,
say/effect/shape errors), the prompt/agent/model/interactive type errors, the
schema errors (load, validation, missing, unsupported-interactive), the
inline-compose / sequence mismatch, and body-composition failures
(`ComposeFailed`, `ShellExpansionFailed`, which show the block as context with no
highlight).

The block is **TTY-gated**: it is rendered only when stderr is a TTY, and
withheld in piped / `NO_COLOR` / CI output so frontmatter is never exposed into
logs — unless `FORCE_COLOR=1` overrides the gate. At `ColorDepth::None`,
`report_block_error` strips escapes for every variant. Capture happens at the
render boundary — after all control-flow handling — so the wrapper never
interferes with upstream decisions; the CLI error walker (`output::error_walker`)
renders the deepest typed diagnostic and appends the YAML block after it
(`excerpt.render_appendix`). The motivating case: a `success.message` referencing
`{{review-file}}` (hyphen) when the variable is `review_file` (underscore) now
shows the frontmatter with the offending line highlighted, instead of an opaque
"interpolation leaked" message.

**Near-miss frontmatter fences.** A `----`+ delimiter (instead of `---`) is
detected by Darkmatter as `MarkdownError::FrontmatterFenceMismatch` and mapped by
Claudine to `CompositionError::FrontmatterParse`; `FrontmatterExcerpt::capture_line`
captures the matched fence pair and highlights the delimiter line (typically
line 1).

**Mechanism.** `composition::FrontmatterExcerpt` (module `frontmatter_excerpt`)
captures the block plus its highlight line;
`CompositionError::enrich_frontmatter(source, stderr_is_tty)` wraps a
frontmatter-rooted error in the transparent
`CompositionError::WithFrontmatter { inner, excerpt }` variant at the render
boundary, so upstream variant matching is unaffected. This superseded the
inline-compose mismatch's bespoke verbatim-YAML dump — the `raw_yaml` /
`stderr_is_tty` fields were removed.

## Prepare-time warnings

Unknown expression functions and unknown `ctx.*` references detected during
prepare emit non-fatal did-you-mean warnings to stderr, suppressed by `--silent`.
String literals and code fences do not trigger the `ctx.*` diagnostic because it
is parsed from the interpolation AST, not a raw text scan.

## Provider Selection

Provider selection behaves differently in **TTY** (interactive terminal) and **non-TTY** (piped or CI) modes. In both modes, explicit `--<provider>` flags always win unconditionally.

### TTY Mode

When stdout is a terminal and no explicit `--<provider>` flag is given:

1. **Interactive picker** — a `biscuit-tui` one-shot picker shows all installed providers. Frontmatter `agent` and config `favorite_agent` only influence the **default index** and **row ordering**; they do not bypass the picker.

### Non-TTY Mode

When stdout is not a terminal (e.g., CI, scripts), resolution follows a strict chain with no interactive fallback:

1. **Explicit flag** (`--provider <slug>`, or the shorthand booleans `--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, `--goose`, `--kimi`) — highest priority
2. **Singular frontmatter `agent`** — a single provider name in the effective (composed) frontmatter, fuzzy-matched against known providers
3. **List-valued frontmatter `agent`** — an ordered list of provider names; the first installed provider in the list is chosen
4. **Config favorite** — `favorite_agent` from `~/.claudine/config.json`
5. **Hard error** — if none of the above resolve, the command fails with a structured error

The old "single installed" auto-selection shortcut has been removed. Even when only one provider is installed, non-TTY sessions still require an explicit signal (flag, frontmatter, or favorite).

### Frontmatter `agent` and `model`

Both `agent` and `model` frontmatter properties accept either a single string or an ordered list of strings:

```yaml
agent: codex
agent: [gemini, codex, claude]
model: gpt-4o
model: [gpt-4o, o3-mini]
```

List-valued `agent` is treated as author preference order: the first installed provider wins. List-valued `model` is validated against the provider's model catalog; the first valid entry wins. When a catalog is unavailable (e.g., Gemini, Kimi, Goose in v1), frontmatter `model` is gracefully skipped rather than treated as an error.

### Model Resolution

Model selection follows a single chain independent of TTY mode:

1. **CLI `--model`**
2. **Provider-specific env var** (`CODEX_MODEL`, `CLAUDE_MODEL`, `OPENCODE_MODEL`, etc.)
3. **Generic `MODEL` env var**
4. **Frontmatter `model`** (validated against catalog when available)
5. **Provider default** (`None` — let the provider choose)

### OpenCode Non-TTY Requirement

OpenCode requires a model in non-interactive mode. If no model survives the resolution chain when running OpenCode in non-TTY mode, Claudine emits a hard error before launching the provider:

```
OpenCode requires a model in non-interactive mode; set --model, OPENCODE_MODEL, or MODEL
```

### Shorthand Flags

The shorthand booleans and the `--provider` value both accept fuzzy input (`cl` → `claude`, `gem` → `gemini`, `oc` → `opencode`). The [argv normalizer](argv-normalization.md) rewrites every shorthand into a canonical `--provider <slug>` pair before clap runs, so runtime provider selection only ever reads the single `--provider` field.

### The `--interactive` and `--no-interactive` Flags

`-i` / `--interactive` forces an **interactive provider session**; `--no-interactive` forces a **non-interactive provider session**. Both flags control the session mode, not provider selection, and the composed prompt is still prepared first before being passed to the provider.

Session interactivity is resolved from (highest to lowest precedence):

1. `--no-interactive` CLI flag
2. `-i` / `--interactive` CLI flag
3. `interactive` frontmatter property (`true` / `false`)
4. Default: non-interactive

`--interactive` and `--no-interactive` are mutually exclusive; clap rejects `-i --no-interactive` at parse time. The `interactive` frontmatter property is honored by `compose` and `inline-compose`; `claudine sequence` rejects `interactive: true` because a sequence is serial automation and must be driven by the explicit `--interactive` override when needed.

> **Note:** `inline-compose -i` is provider-gated. Claudine allows it only when the selected provider can recover the final assistant message for the inline rewrite path. When interactive mode is triggered by frontmatter `interactive: true`, the diagnostic names `frontmatter` as the source so the remediation is clear.

### The `--exclude` Flag

`--exclude <PROVIDER>` removes a provider from automatic selection (repeatable). Explicit flags (`--codex`, etc.) override exclusions.

## Dry Run

`--dry-run` runs the **full composition pipeline up to but not including provider launch**, then emits the composed result instead of sending it to an agentic CLI. It is available on `compose`, `inline-compose`, and `sequence`, and is the gate to use for CI rehearsal: the path it exercises is identical to a real run, minus the provider spawn.

### Pipeline Scope

Everything before launch runs normally:

- Schema validation (including the interactive missing-property prompt under a TTY).
- Shell commands in the document graph are **executed for real** — they produce actual side effects and their output is interpolated into the frontmatter and body.
- Shell-command approval and writability checks run normally.
- Provider and model resolution run normally.

The provider is **never launched**; for `inline-compose` the source file is therefore **never mutated** (`last_updated` is untouched).

### Output Split

Dry-run output follows Unix stream conventions so `claudine compose --dry-run doc.md > body.md` captures only the body:

- **stdout** — the composed document body (the data product).
- **stderr** — the finalized YAML frontmatter (syntax-highlighted) followed by a metadata table.

The metadata table rows, in order: **Document** (frontmatter `name`, or the relative path, rendered as a blue OSC8 link), **Description** (italic + dim, only when set), **Agent** (the resolved provider name when one is selected, or a classified resolution breakdown — no-agent, invalid frontmatter hint, not-installed hint, multi-suggestion list, auto-selected single suggestion, or zero-installed list — rendered as a multi-line cell), **Model** (the resolved model, or `default`), **YOLO** (`true`/`false`), **Session** (`interactive` or `non-interactive` with the resolved source in parentheses, e.g. `interactive (frontmatter)` or `non-interactive (--no-interactive)`), **Area** (the focused monorepo area, only when inside a monorepo), and **Deferred** (the lifecycle event keys left raw in the YAML block above because they interpolate at event-time, only when at least one such key is present — so a raw `{{err.code}}` span there reads as intentional, not as an unresolved-variable bug).

`--quiet` and `--silent` have **no effect** in dry-run mode: the full output is always rendered.

### Non-TTY Shell-Approval Gate

In a TTY, an unapproved shell command triggers the same interactive approval prompt as a normal run. In a non-TTY environment (e.g. CI) there is no way to prompt, so the dry-run **fails fast**: it exits non-zero with

```
Cannot dry-run: shell command 'X' requires interactive approval. Run with --yolo to auto-approve, or pre-approve the command in your configuration.
```

This is the gate working correctly — if production would prompt, the dry-run fails. Bypass it with `--yolo` (auto-approve) or by pre-approving the command in configuration.

### Sequence Dry Run

`sequence --dry-run` exercises the whole sequence as one logical command. Each step is composed and rendered in order: all bodies are concatenated to **stdout**, while each step's frontmatter and metadata table go to **stderr**, separated by a `=== Document N of M ===` divider before every document after the first. A composition failure in any step renders the error to stderr and stops the sequence immediately (fail-fast), exiting non-zero without launching any provider.

Any composition error (schema validation failure, missing file, denied shell command, writability failure) is rendered to **stderr** and exits the process non-zero, leaving stdout clean.

## Schema Validation

Composition documents can declare a `$schema` in their frontmatter to constrain the property values that drive the prompt. Schema processing is anchored on Darkmatter's `SimplifiedSchema` and runs as a stage inside the existing `Resolve → Pre-Flight → Prepare → Select → Launch → Closure` pipeline — between override application and shell expansion. The wrapper layer translates Darkmatter's structural failures into typed claudine errors so users see actionable reports instead of a generic compose failure.

## Lifecycle Integration

Composition runs execute the full seven-event lifecycle declared in the prompt's frontmatter:

```
initialize → start → (success | blocked | failure) → finalize → loop
```

- **`initialize`** fires after the prompt file is resolved and frontmatter has parsed, but before schema validation and shell pre-flight. A `skip` control action here opts the whole document out cleanly.
- **`start`** fires after schema validation and the lifecycle shell-audit pass succeed, immediately before provider invocation.
- **`success`/`blocked`/`failure`** are the terminal events. Schema-validation failures and shell-audit denials produce `blocked`; provider errors produce `failure`.
- **`finalize`** fires once per iteration, immediately after the terminal event.
- **`loop`** is the post-`finalize` gate. Lifecycle concerns authored inside the `loop:` block run first, then the `while`/`until` condition is evaluated, then per-iteration mutations are applied only when continuing.

Legacy prompts that only declare `start`, `success`, `blocked`, and `failure` continue to behave the same way. See [lifecycle.md](lifecycle.md) for the full lifecycle reference, including stacks, control actions, the `err`/`timing`/`current` globals, and examples.

Each lifecycle property interpolates **when its event fires**, not during the initial compose — Darkmatter defers the seven lifecycle keys from compose-time resolution (so their `{{ … }}` spans survive raw in `effective_frontmatter`) and Claudine re-interpolates each property/action string through Darkmatter just-in-time, against the live document state plus the in-scope late-binding globals. Resolution fails closed before any side effect dispatches. See [lifecycle.md — Binding Time: Early vs Late](lifecycle.md#binding-time-early-vs-late).

### Loop vs lifecycle interpolation

Claudine renders `{{ … }}` templates on two frontmatter surfaces: **loop action values** (`set`/`append`/`prepend`/`merge`, via `looping::actions::render_action_value`) and **lifecycle event text** (via the Darkmatter DM2 substrate `SubtreeCompose`). Both consume the *same* Darkmatter expression core — `parse` / `evaluate` / `ExpressionFinder` / `scalar_string` over an `EvaluationLookup` — so the loop renderer is **not** a second expression engine; it is a loop-specific value renderer sharing that core. A [shared conformance matrix](../../lib/src/composition/interpolation_conformance.rs) pins the overlap: literal/mixed strings, whole-value typed expansion, arrays/objects, the `doc` namespace, functions, string-literal escaping, and malformed-expression fail-closed behavior all resolve **identically** from the same input and state.

Three semantic differences are deliberate and keep the two renderers separate rather than merging the loop path into DM2:

| Concern | Loop action renderer | Lifecycle DM2 (`SubtreeCompose`) |
|---------|----------------------|-----------------------------------|
| Mixed string that forms valid JSON (e.g. `"{{a}}{{b}}"` with `a=1, b=2`) | Re-parsed as JSON → `12` (number). See [looping.md](flow-control/looping.md). | Kept as string → `"12"`. |
| Error on a malformed/invalid template | Contextual `CompositionError::InvalidAction` carrying iteration + action index (`InvalidAction at iteration N, action M of K`) | Generic `MarkdownError::Transform` |
| Unknown variable root in a mixed string (e.g. `"x={{typo}}"`) | Lenient → resolves empty (`"x="`), matching loop **condition** evaluation | Strict / fail-closed → typed error before any side effect dispatches |

The loop renderer's leniency and JSON re-parse serve state mutation (a loop action writes frontmatter, where an empty/typed result is the natural outcome and mirrors `while`/`until` evaluation), while DM2 strict mode serves side-effect dispatch (a lifecycle message must never reach Discord/TTS/stderr carrying an unresolved reference). Both engines are held to the shared matrix so the overlap cannot silently drift.

### Loop Execution

A frontmatter `loop:` block turns the prompt into a repeating run. The first iteration runs `initialize` once; later iterations re-enter at `start` without re-running `initialize`, schema validation, or shell pre-flight. `success`, `failure`, and `finalize` fire once per iteration, and the loop condition is evaluated at the post-`finalize` gate after any `loop:` lifecycle concerns.

### Authoring

`$schema` accepts the same forms Darkmatter accepts: inline `SimplifiedSchema` mappings, references to external YAML/JSON schema files (resolved through the shared `FileReference` contract — a bare implicit reference is repository-root first, then the prompt document's parent directory; an explicit `./`/`../` reference is the document's parent only), and root-level unions. Raw JSON Schema also validates, but it does not expose typed property metadata, so it does not feed the interactive prompts or shell completion described below.

```yaml
$schema:
  topic: 'string(required)'
  tier: 'enum(small, medium, large; required)'
  draft: boolean
  cover: "file(match('*.png'))"
```

Remote `http://` / `https://` schema references remain unsupported.

### Required vs Optional

For each property declared in `$schema`, claudine routes the validation outcome through three categories:

| Outcome                | Required                                                            | Optional                                                                                |
| ---------------------- | ------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Present and valid      | continue                                                            | continue                                                                                |
| Present as `null`      | hard `SchemaValidation` abort                                       | treated as absent (valid)                                                               |
| Missing                | prompt in Interactive Mode when allowed; otherwise `MissingProperties` | continue                                                                                |
| Present but invalid    | hard `SchemaValidation` abort (no prompt, no recovery)              | the value is dropped from the prompt context, a `tracing::warn!` fires, composition retries once |

The drop-and-retry for invalid optionals is automatic; users see the discarded value via the `dropping optional schema property with invalid value` log line.

### Interactive Mode

When required properties are missing, claudine offers to collect them interactively. Interactive Mode is allowed only when **every** condition is true:

1. `prompt_for_missing` is `true` (the default user-config value),
2. stdin is attached to a TTY,
3. stderr is attached to a TTY (stdout may be piped),
4. `--silent` is not set,
5. no required value is present-but-invalid (the abort above wins),
6. at least one required value is actually missing.

When Interactive Mode is denied, claudine emits a `MissingProperties` error with the prompt file (OSC8-linked), the list of missing property names in declaration order with their type labels and descriptions, the frontmatter `description` (when present), and a remediation hint: `Pass key=value, use --set, or set prompt_for_missing to true in an interactive terminal.`

When Interactive Mode is allowed, claudine first prints a per-property status report to stderr (required and optional, with valid/invalid/missing glyphs) and then drives `biscuit-tui` widgets sized to each property's `SimplifiedType`:

| Property type                                  | Widget                                  |
| ---------------------------------------------- | --------------------------------------- |
| `enum(...)`                                    | single-choice picker                    |
| `enum(...)[]`                                  | multi-choice picker                     |
| `boolean` / `boolish`                          | boolean switch                          |
| `number` / `numberlike`                        | text input with parse-and-retry         |
| `string` / `date` / `datetime` / `time` / `url` / `email` / `file` | text input with format hint |
| `object`, `any`, property-level union, root-level union without projection | `UnsupportedInteractiveSchema` |

Numeric inputs reprompt with an inline error on parse failure instead of aborting. Collected values feed back into the override set; composition re-runs with the new overrides before any provider session starts.

### Provided Partial File References

A `file`/`file[]` property declared with a `match(...)` glob accepts more than a literal path. When the user **provides** a value (via `key=value` or `--set`) that does not resolve to an existing file, that value is treated as a **partial** — a substring to match against the property's `match(...)` glob candidates — rather than an immediate hard abort.

```yaml
$schema:
  spec: 'file(required; match(**/*spec*.md))'
```

`claudine compose plan spec=everywhere` (with no literal `everywhere` file) now:

1. walks the `match(**/*spec*.md)` glob from the **launch area** (the same `property_value_root` anchor completion uses, so *offered == accepted*),
2. filters candidates whose path contains `everywhere` (case-insensitive), and
3. drives a **confirmation dialog** on a single match, a **chooser** on multiple, then rewrites the property override to the chosen path and re-validates once — mirroring the missing-property collection loop above.

The `match(...)` glob is consulted **only after** literal path resolution fails, so valid explicit paths keep their existing behavior. Both required and `eager`-optional file properties reach this resolution. Zero glob+substring matches, a declined confirmation, or a cancelled chooser fall back to the original `no existing file matched reference` schema-validation error unchanged. When Interactive Mode is denied (not both stdin and stderr TTYs, `--silent`, etc.), the original error is preserved byte-for-byte so scripts and CI output are unaffected. The glob compile and walk live in `claudine-cli`; the library only classifies the failure into the typed `UnresolvedFileReference { property, provided, patterns }` signal and never gains a `globset`/`ignore` dependency.

### Schema Collection Independence

The decision to prompt for missing required values depends **only** on the six signals listed under [Interactive Mode](#interactive-mode) above and **must not** depend on the resolved `session_interactive` value. Collection completes during the pre-flight schema-validation stage, before the provider child process is ever spawned. This means an `interactive: true` document with missing required properties still collects them interactively under a TTY, and a `--no-interactive` invocation of an `interactive: true` document still emits a typed `MissingProperties` error rather than launching a non-interactive session.

### User Config

The user-scoped Claudine config (`~/.claudine/config.json` or `.json5`) exposes a single switch:

```json
{ "prompt_for_missing": true }
```

The default is `true`. The field is scoped to the user config only — the repo-scoped config rejects it. The interactive `claudine config` TUI offers a boolean toggle, and the non-interactive setter accepts `claudine config set prompt-for-missing true|false`.

### Validation Timing

For every flavor (`compose`, `inline-compose`, `sequence`):

1. Parse `--set` and shorthand `key=value` setters (JSON5-first).
2. For `sequence`, merge per-step overlay values after caller setters; reserved overlay keys win.
3. Compose through Darkmatter.
4. Build the effective schema for the composed document.
5. Validate effective frontmatter.
6. If required values are missing and Interactive Mode is allowed: collect, apply as overrides, re-compose, re-validate.
7. Proceed to provider/model resolution only after validation succeeds.

`inline-compose` keeps its existing `prompt` checks: a missing or non-string `prompt` still surfaces as `PromptPropertyMissing` / `PromptPropertyWrongType` before schema validation runs. The original `$schema` declaration is preserved byte-for-byte during the inline rewrite — interactive values collected for one run are never written back to the source file.

Source loading (shared by all three commands) parses frontmatter strictly. A document whose `---` block contains malformed YAML — e.g. inconsistent block-scalar indentation — surfaces as a `FrontmatterParse` error that renders Darkmatter's rich frontmatter-parse block (file link, YAML location, offending-line excerpt), not as a misleading `PromptPropertyMissing`.

`sequence` validates every step during Phase 1a, before any provider session starts. When multiple steps share the same missing property with the same shape and description, the user is prompted once and the answer is reused for later steps (unless the step overlay supplies a different value). Non-interactive failures are aggregated into a single `SequenceMissingProperties` error so the user can fix the entire sequence in one edit pass.

### Schema-Aware Shell Completion

The composition completion engine consults `$schema` when the cursor sits on a setter slot AND a positional prompt-file argument is already committed. See [shell-completions.md — Schema-Aware Setter Completion](completions/shell-completions.md#schema-aware-setter-completion) for the full contract.

## Migrating from the Retired Harness DSL

Earlier Claudine releases let composed documents declare `pre_checks`, `post_checks`, `handle_*` handlers, a programmatic `handle`, and `deviate` recovery commands in their frontmatter. That validation-and-handler DSL has been **removed**. Its gating, verification, and recovery roles are now expressed through the [lifecycle stack](lifecycle.md): `when:` guards plus the `Error` / `Skip` / `Proxy` / `Retry` / `Resume` / `Requeue` lifecycle actions and `shell` actions.

A document that still declares any of these keys fails composition with a typed `RemovedValidationKey` diagnostic that names the offending key and points at its replacement surface:

| Removed key | Replacement |
|-------------|-------------|
| `pre_checks` | the `initialize` or `start` lifecycle stack |
| `post_checks` | the `success` or `finalize` lifecycle stack |
| `handle_<event>` (e.g. `handle_timeout`, `handle_inline_body_unchanged`) | the `blocked` or `failure` lifecycle recovery actions |
| `handle` | a lifecycle `shell` action or other lifecycle action |
| `deviate` | a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) |

The scan runs before lifecycle event blocks are parsed, so the diagnostic names the removed DSL key rather than falling through to generic unknown-field handling. Like every frontmatter-rooted composition error, it appends the authored frontmatter as a syntax-highlighted YAML block under a TTY.

**Verification** that the agentic loop actually did the work it claimed (the old `post_checks` role) now belongs in the `success` or `finalize` stack: guard a `when:` clause and raise an `Error` lifecycle action when the contract is unmet. **Recovery** (the old `handle_*` role) belongs in the `failure`/`blocked` stack via `Retry`, `Resume`, `Requeue`, or `Proxy`. See [lifecycle.md](lifecycle.md) for the full action catalog.

## Timeouts

Claudine supports two timeout properties — `timeout` (wall-clock) and
`step_timeout` (stream-silence) — that share the same human-readable
duration grammar and the same termination path. Both are settable via
markdown frontmatter, CLI flags (`--timeout`, `--step-timeout`), and
env-var defaults (`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`), with
precedence CLI > frontmatter > env > built-in default.

```yaml
timeout: 10m          # opt-in hard ceiling on total runtime
step_timeout: 45s     # kill the child if it goes silent for 45s
```

Both timeouts surface as the same timeout failure and route to the
`failure` lifecycle event, where a `Retry` or `Resume` action can recover.

**Relational validation.** When both properties are present,
`step_timeout` must be less than or equal to `timeout`. Documents that
violate this invariant fail parse-time validation with
`HarnessError::InvalidTimeout`.

**Streaming-only.** `step_timeout` requires structured streaming. If the
selected provider runs in capture or passthrough mode (the non-streaming
Goose wrapper is the primary example), Claudine emits a warning and
ignores the field.

See [`topics/timeouts.md`](timeouts.md) for the canonical reference,
including the full precedence table, defaults rationale, env-var knobs
(`CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`, `CLAUDINE_KILL_GRACE`,
`CLAUDINE_WATCHDOG_INTERVAL`), termination path, exit reasons, and the
subagent diagnostics that ride alongside `step_timeout`.

## Timing Surface

Every composition run (whether harness-enabled or not) shares a single
user-visible timing surface rendered to stderr. Two emitters drive it:

1. **Periodic prompt-scoped header.** Emitted at `t=0` when the prompt
   begins and then at monotonic offsets from `t=0` (`t=10m`, `t=20m`,
   `t=30m`, …). Ticks are anchored on the prompt's start time, not on
   wall-clock `:00 :10 :20` boundaries.

    - The `t=0` header reads `⏱️ {HH:MM} {TZ} running the {prompt} prompt`
      (no duration segment).
    - Subsequent ticks read `⏱️ {HH:MM} {TZ} running the {prompt} prompt
      for {duration}`.
    - `{prompt}` is an OSC8 link whose visible text is the path relative
      to the repo root (falling back to CWD, `$HOME`, then absolute).

2. **Fire-once warnings** (harness frontmatter only):

    - **`timeout_warn`** — prompt-scoped. Fires once when the prompt has
      been running for this long. Message variants:
        - *With `timeout` also set:* `the {prompt} has been running for
          {elapsed}, this is longer than we'd expect it to take but we
          won't timeout this prompt until we reach {HH:MM} in
          {remaining}.`
        - *Without `timeout`:* `the {prompt} has been running for
          {elapsed}, this is longer than we'd expect it to take. Press
          CTRL+C to terminate this prompt if you're convinced that the
          prompt has hung.`
    - **`step_timeout_warn`** — step-scoped, fires once per stall episode
      using the same silence clock as `step_timeout`. Message variants:
        - *With `step_timeout` also set:* `the {prompt} has not produced
          output for {silence}, this is longer than we'd expect, but we
          won't abort this step until we reach {HH:MM} in {remaining}.`
        - *Without `step_timeout`:* `the {prompt} has not produced output
          for {silence}, this is longer than we'd expect. Press CTRL+C to
          terminate this prompt if you're convinced that the prompt has
          hung.`

```yaml
timeout: 10m
timeout_warn: 5m         # warn at 5m; still kill at 10m
step_timeout: 45s
step_timeout_warn: 20s   # warn at 20s silence; still kill at 45s silence
```

**Duration rendering.** All user-visible durations use a single format:
`{N}s` under 60 seconds (e.g. `45s`), `{N}m` between 1 and 59 minutes
(e.g. `12m`), `{H}h {M}m` at 60 minutes or more (e.g. `1h 30m`, `2h 5m`).
No zero-padding, no colon-separated forms.

**Preflight validation.** Each `*_warn` must be strictly less than its
corresponding hard threshold when both are present. `timeout_warn >=
timeout` or `step_timeout_warn >= step_timeout` is rejected at parse
time with `HarnessError::InvalidTimeout`. `*_warn` values `<= 0` are
also rejected (the underlying duration parser requires positive values).
A warn set without its corresponding hard threshold is legal — the
"without hard threshold" message variant applies.

### Watchdog and Exit Reasons

The wrapper enforces `timeout` and `step_timeout` via a unified watchdog
ticker. On breach, the synthesised `session_end` JSONL summary records
`error_kind: "timeout"` or `"step_timeout"`, and the rendered `Agent
Error` `BlockQuote` enumerates any outstanding subagents (id, name,
elapsed since last progress) so the operator knows which workers
stalled. While `step_timeout` is enabled, the live stderr surface also
emits at most one ` ⏳ Awaiting subagent: <name-or-id> (<elapsed>)`
diagnostic line per active subagent per silence window.

See [`topics/timeouts.md`](timeouts.md) for the full env-var table,
precedence chain, termination path, and worked examples.

### Recovery

Recovery from a failed run is expressed through the `failure` and `blocked` lifecycle stacks, not a separate handler DSL. The available lifecycle recovery actions are:

- **`Retry`** — re-run the prompt (re-runs the pre-flight/start path from `blocked`, or the agentic loop from `failure`)
- **`Resume`** — resume the agent session with its context intact and a follow-up message (provider must support session resume)
- **`Proxy`** — hand off execution to a different prompt document at its own `initialize`
- **`Requeue`** — push the prompt onto the deferred-execution queue

See [lifecycle.md](lifecycle.md) for the full recovery-action reference and the [migration table](#migrating-from-the-retired-harness-dsl) for the mapping from the removed `handle_*` keys.

### Shell Policy

All shell commands — `::shell` directives in the template, top-level frontmatter `$(cmd)` expressions, and lifecycle `shell` stack actions — are approved upfront during the pre-flight phase, before the provider session starts. See [Pre-Flight Shell Approval](pre-flight-checks.md) for the full flow.

## Retired Interfaces

The following interfaces have been removed and replaced by the two canonical commands above:

| Removed | Replacement |
|---------|-------------|
| `claudine <agent> --compose <file>` | `claudine compose --<agent> <file>` |
| `claudine <agent> --frontmatter-prompt <file>` | `claudine inline-compose --<agent> <file>` |
| `claudine compose inline <file>` | `claudine inline-compose <file>` |
| `claudine compose-inline <file>` | `claudine inline-compose <file>` |
| `AGENT` environment variable | `--claude`, `--codex`, etc. flags or `agent` frontmatter |

**Removed without replacement:**

| Removed | Reason |
|---------|--------|
| `claudine <agent> --prompt-file <file>` | Sent file content verbatim as a prompt. `claudine compose` performs full Markdown composition (frontmatter, template substitution, `::shell` directives) so it is not a drop-in replacement. Callers that need raw prompt delivery should use the provider CLI directly. |

## Sequence Composition

Sequence composition runs a single source document multiple times, once per step in a defined list, with step-specific state injected into the composition context on each run.

```sh
claudine sequence @deploy.md
claudine sequence --fail-fast false @batch.md
```

### When to Use Sequence

Use `claudine sequence` when you have a fixed list of items and need to compose the same template document against each item independently. Each step is a full one-shot composition run — with its own provider selection, lifecycle evaluation, and pre-flight shell approval. The sequence command is serial; steps do not run in parallel.

### Compose vs Inline Steps

A sequence runs each step as either a **compose** step or an **inline** step, decided once for the whole run by the same signal that splits the top-level `compose` and `inline-compose` commands: the presence of a `prompt` frontmatter property on the source document.

- **No `prompt` property** — each step is a `compose` (chained-document) run: the composed **body** is sent as the agent prompt and no file is mutated.
- **`prompt` property present** — each step is an `inline-compose` run: the composed **`prompt`** (with per-step `{{state}}` interpolation) is sent as the agent prompt, and the provider's output **replaces the document body** on disk, preserving the original frontmatter and bumping `last_updated` (see [Inline Composition](#inline-composition)).

Because steps run serially and the body is written back after each one, an inline step's agent reads the body that the previous step wrote. A `prompt` property that is present but not a string is rejected up front with `PromptPropertyWrongType` — before any step launches — exactly as `inline-compose` does.

### Inline Sequence Definition

Sequences can be defined directly in the source document's frontmatter as a scalar list or an object list.

**Scalar list** — each step value is a plain string:

```yaml
sequence:
  - one
  - two
  - three
fail_fast: false
```

**Object list** — each step value is an object; `name` is required:

```yaml
sequence:
  - name: one
    color: red
  - name: two
    color: blue
```

### External YAML Sequence Definition

When the `sequence` frontmatter property is a string, Claudine resolves it through the shared `FileReference` contract: a bare implicit reference is repository-root first, then the source document's directory; an explicit `./`/`../` reference is the source directory only; `@` is a magic-root search and `~/` is home-pinned.

**Plain list form** — the external file contains a `sequence:` key:

```yaml
# steps.yaml
sequence:
  - name: Codex CLI
    site: https://developers.openai.com/codex/cli
  - name: Claude Code
    site: https://claude.ai/code
```

**Template form** — the external file uses `kind/list/template` to apply a shared template across all items:

```yaml
# steps.yaml
kind: sequence
template:
  desc: "{{name}} (_site: {{site}}, repo: {{repo || 'n/a'}}_)"
list:
  - name: Codex CLI
    site: https://developers.openai.com/codex/cli
    repo: https://github.com/openai/codex
  - name: Claude Code
    site: https://claude.ai/code
```

Template rules:

- `kind: sequence` is optional; when present it must equal `sequence`
- `list` must be a non-empty list of objects, each with `name`
- `template` is only supported in the `kind/list/template` external-file form
- Template values must be strings; each template string is rendered against the item's own fields
- Rendered template fields are merged into the item; they may not overwrite reserved step keys

### Template Evaluation

Each step runs the source document through Darkmatter's composition pipeline with a set of reserved variables injected as overrides. These variables are always set by the sequence runner and cannot be overridden by `--set`:

| Variable | Type | Description |
|---|---|---|
| `state` | string or object | The current step value (scalar string or full object) |
| `previous_state` | string, object, or null | The previous step's value, or null for the first step |
| `next_state` | string, object, or null | The next step's value, or null for the last step |
| `is_first` | boolean | `true` when this is the first step |
| `is_last` | boolean | `true` when this is the last step |
| `step` | integer | One-based index of the current step |
| `total_steps` | integer | Total number of steps in the sequence |

For object steps, fields are accessed through `state`: `{{state.name}}`, `{{state.color}}`, etc. Field values are not promoted to top-level variables to avoid collisions with reserved keys or other frontmatter properties such as `agent` or `timeout`.

The `FAIL_FAST` environment variable is also injected per step so that `{{env.FAIL_FAST}}` and `::shell` directives see the same policy as the child provider process.

### Fail-Fast Behavior

By default, a sequence stops on the first failed step. Failure means any of: pre-flight failure, preparation failure, non-zero provider exit, or unrecovered lifecycle failure.

The effective fail-fast policy is determined by:

1. **`--fail-fast` CLI flag** — overrides the document default for this invocation
2. **`fail_fast` frontmatter property** — document-level default; must be a boolean
3. **Built-in default** — `true` when neither is specified

```yaml
# document default: continue on failure
fail_fast: false
```

```sh
# CLI override: stop on first failure regardless of document default
claudine sequence --fail-fast true @batch.md
```

The `--fail-fast` flag accepts boolish values: `true`, `false`, `1`, `0`, `yes`, `no`.

### The `FAIL_FAST` Environment Variable

Claudine injects `FAIL_FAST=true` or `FAIL_FAST=false` into the composition environment for each step. This makes the effective policy visible to `{{env.FAIL_FAST}}` interpolation inside the template and to any `::shell` directives that inspect the environment.

### Error Handling Semantics

When `fail_fast` is `true` (the default), Claudine stops immediately after the first failed step and exits with code `1`. Steps after the failure are not executed.

When `fail_fast` is `false`, Claudine records each step's result and continues through all steps regardless of failures. After the last step, Claudine exits with `0` if all steps succeeded, or `1` if one or more steps failed.

Lifecycle recovery actions (`Retry`, `Resume`, `Requeue`, `Proxy`) apply within a single step only. There is no cross-step recovery mechanism.

> **Note:** The `fail_fast` frontmatter key is reserved for sequence control. It is not passed to Darkmatter's internal compose options.

## Architecture

Both commands follow the same six-stage pipeline, with lifecycle events woven around the stages:

```
Resolve → Initialize → Pre-Flight → Prepare → Start → Select Provider → Launch → (Success | Blocked | Failure) → Finalize → Loop
```

- **Resolve**: `composition::resolve_composition_source()` loads the Markdown file
- **Initialize**: `LifecycleRunGuard::emit_initialize_once()` fires the `initialize` lifecycle event; a `skip` control action here exits cleanly before any later stage
- **Pre-Flight**: `composition::resolve_shell_approvals()` discovers every shell command in the document graph — template `::shell` directives, top-level frontmatter `$(...)` expressions, and lifecycle `shell` stack actions — checks whitelists, and prompts the user to approve any unapproved commands before proceeding (see [Pre-Flight Shell Approval](pre-flight-checks.md))
- **Prepare**: `composition::prepare_direct()` or `composition::prepare_inline()` composes through Darkmatter with the pre-approved command set and produces a `PreparedComposition` with `effective_frontmatter`
- **Start**: `LifecycleRunGuard::emit_start_once()` fires the `start` lifecycle event after schema validation and shell audit pass
- **Select**: `composition::select_provider()` applies the precedence chain
- **Launch**: `wrap::composition::execute_composition_request()` runs the provider through the full wrapper pipeline (env, MCP, harness, streaming)
- **Terminal**: `LifecycleRunGuard::emit_terminal()` fires `success`, `blocked`, or `failure`
- **Finalize**: `LifecycleRunGuard::emit_finalize_once()` fires `finalize` once per iteration
- **Loop**: the post-`finalize` gate evaluates `loop:` lifecycle concerns, the `while`/`until` condition, and applies per-iteration mutations when continuing
- **Closure**: `composition::closure::rewrite_inline_document()` reconstructs the document for inline mode; direct mode outputs to stdout

The original six-stage summary (`Resolve → Pre-Flight → Prepare → Select Provider → Launch → Closure`) describes the functional pipeline; lifecycle events are the hooks that run at the boundaries between those stages.

## Performance Reporting

Composition commands (and the provider wrappers) support an opt-in `--perf` flag that prints a performance breakdown to stderr after execution completes. The report is a single **reconciling tree** rooted at the `Performance` headline:

```text
Performance                         384.0ms  100%
├─ pre-dispatch                       29.1ms    8%
│  ├─ arg parsing                     20.3ms    5%
│  ├─ tracing init                     3.5ms   <1%
│  └─ config loading                  23.3ms    6%
├─ prep phase                        204.3ms   53%
│  ├─ frontmatter load                33.7ms    9%
│  ├─ schema validation                4.1ms    1%
│  ├─ shell approval                  12.3ms    3%
│  ├─ composition                      4.0ms    1%
│  │  └─ shell expansion                33µs   <1%  ×2
│  └─ unattributed                   148.4ms   39%
├─ environment setup                  85.7ms   22%
│  ├─ system prompt                   81.6ms   21%  ▇ HOT
│  └─ unattributed                     1.2ms   <1%
├─ agent execution                        —       —  (dry run)
└─ unattributed                       65.0ms   17%
```

The model and its invariants:

- **Headline is true wall-clock.** The `Performance` total is sampled once at report-build from a single process-start baseline, so it can never disagree with the body the way a mid-flight timer could.
- **Structural buckets reconcile.** Every `Structural` node's children (plus a synthetic `unattributed` remainder) sum back to the node's own total. The top-level buckets (`pre-dispatch`, `prep phase`, `environment setup`, `agent execution`) therefore sum back to the headline. A debug assertion enforces this at runtime; a unit test (TR-4) enforces it for every command shape.
- **Breakdown rows itemize without double-counting.** Darkmatter composition stages and the `pre-dispatch`/agent sub-rows are `Breakdown` children — shown and percentaged, but excluded from the reconciliation sum so no cost appears twice. Single-shot `compose`/`inline-compose` nest the `composition` subtree under `prep phase`; `sequence` attaches the merged composition under `environment setup`.
- **Percent column** shows each row's share of wall-clock (`100%` at the root, `<1%` for sub-one-percent slivers).
- **`HOT` marker** flags the single dominant leaf when it clears the materiality floor (≥20% of wall-clock).
- **Run counts** (`×N`) appear on a composition stage that ran more than once.
- **Dry runs** render `agent execution` as an `—` leaf annotated `(dry run)`; the agent never launches.

For `sequence`, the report is aggregated across all steps: launches and total execution time are summed, first-response latencies are averaged (with the minimum shown in a note), and composition metrics are merged. The report appears exactly once at the end of the run, after the sequence summary.

`--perf` is a stderr-only artifact — it never writes to stdout, so it does not interfere with piped composition output. It is emitted unconditionally when passed, even alongside `--silent` or `--quiet`, because it is an explicit opt-in.

> **Note:** `provider_api_duration` is only populated for structured-streaming providers. Legacy providers (e.g., Goose) omit this line.

## Module Structure Audit (Phase 2.12)

`composition/mod.rs` declares a mix of `pub mod` and `mod` (private) children. The private modules (`error`, `guardrails`, `prepare`, `resolve`, `select`, `types`) are re-exported via `pub use` where their items are part of the public API surface (e.g., `CompositionError`, `prepare_direct`, `resolve_composition_source`). This is an intentional design: the module tree is private, but the public types surface through targeted `pub use` re-exports. No widening of privacy was needed during the Phase 2 audit.