---
hash: ef46db3751d8e999-fce2dc04b4df88da
last_updated: 2026-09-01
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
`codex`, `gemini`, `goose`, `kimi`, `opencode`, `qwen`, `kilo`, `pi`,
`antigravity`). All three
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
   - By default the response is body-only. An authored `response_frontmatter` list may authorize exact property names that the provider can return in a leading YAML frontmatter block.
   - The authorization is captured from the authored source before setters, interpolation, or schema defaults. Invalid declarations fail before provider launch, and authorizing a Claudine-interpreted property emits a warning because it may change later executions.
   - Authorized properties are inserted in declaration order before `last_updated` or refreshed in place on later runs. Undeclared proposals are ignored with response-line warnings; response-provided `hash` and `last_updated` are ignored silently.
   - A delimited metadata attempt with malformed YAML, duplicate keys, a non-mapping root, or no replacement body fails without modifying the source. Missing authorized properties are reported but do not fail a valid body.
   - Authored frontmatter bytes remain authoritative. If the source changes while the provider runs, Claudine completes the run, restores the pre-run frontmatter snapshot byte-for-byte, and reports each added, removed, or value-changed property without attributing a writer. Structurally invalid frontmatter gets a generic restoration warning because property-level comparison is impossible. Value-preserving reformatting remains silent. Mid-run body drift is compared independently, then replaced and reported.
   - `last_updated` is set to today's date (local time, `YYYY-MM-DD`)
   - The file is written atomically
   - A cleanup pass normalizes the body markdown without touching authored frontmatter

### `hash` property (auto-stamped)

Every successful `inline-compose` closure stamps a Darkmatter `Simple` content
hash into the `hash:` frontmatter property as part of the same atomic write
that persists the body.

- **Format** — `<16-hex-fm>-<16-hex-body>` (for example,
  `hash: a1b2c3d4e5f60718-9a0b1c2d3e4f5061`).
- **Kind is forced to `Simple`** — even if the document previously held a valid
  `structured` or `detailed` hash, the next `inline-compose` run normalizes it
  to the `Simple` shorthand.
- **Textual write-back** — the managed hash and `last_updated` nodes are edited
  in the reconstructed source text. Unmanaged authored frontmatter and the
  replacement body are not reserialized through a YAML emitter.
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
`apply_hash_save_text`.

[`apply_inline_closure`]: ../../lib/src/composition/closure.rs

### Inline Conventions


- **`prompt`** (required) — the prompt text; composed through Darkmatter before execution
- **`response_frontmatter`** — optional ordered allowlist of generated properties accepted from a leading response frontmatter block
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

1. **Explicit flag** (`--provider <slug>`, or the catalog-derived shorthand booleans `--claude`, `--codex`, `--gemini`, `--opencode`, `--qwen`, `--goose`, `--kimi`, `--kilo`, `--pi`, `--antigravity`) — highest priority
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

`--dry-run` runs the **composition pipeline through provider/model resolution**, then emits the composed result instead of sending it to an agentic CLI. It is available on `compose`, `inline-compose`, and `sequence`, and is the gate to use for CI rehearsal.

### Pipeline Scope

Everything up to the seam runs normally:

- Schema validation (including the interactive missing-property prompt under a TTY).
- Shell commands in the document graph are **executed for real** — they produce actual side effects and their output is interpolated into the frontmatter and body.
- Shell-command approval and writability checks run normally.
- Provider and model resolution run normally.
- Selected-executable availability validation and path resolution are skipped;
  the selected agent does not need to be installed or present on `PATH`.

The seam sits in `wrap::composition::pipeline::execute_composition_request_inner_with_guard`, immediately after provider/model selection resolves. Everything past it is skipped: selected-executable validation/path resolution, MCP shadow-HOME materialization, argv and system-prompt overlay construction, the child-CWD switch, the lifecycle runtime, and the provider spawn. Installed-provider inventory may still run when agent selection or the rendered resolution breakdown needs it; that inventory never makes the selected executable a dry-run prerequisite.

**Dry run fires no lifecycle events and has no filesystem side effects of its own.** Because the seam is ahead of lifecycle dispatch, `initialize`/`blocked`/`finalize` never fire, so a stack carrying `append_line`, `set_frontmatter`, or `shell` cannot touch the workspace during a run the user asked to be a rehearsal, and no dynamic `proxy` route can be traversed. For `inline-compose` the source file is likewise **never mutated** (`last_updated` is untouched).

Turning dry run into lifecycle simulation is an explicit non-goal. The one caveat to "no side effects" is the bullet above: `::shell` spans inside the document graph are part of *composition*, not the lifecycle, and do run for real.

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

### Caller File Provenance and Materialization

Each explicit caller setter (`key=value` or `--set`) retains an immutable,
per-property record containing the raw value and the file-resolution context
captured at launch. Canonical preparation keeps that record separate from the
effective frontmatter map. Document frontmatter, schema defaults, `proxy.with`,
runtime mutation, and sequence/task-authored values retain their own ownership
and are not relabeled as caller input.

Before frontmatter interpolation pass 1, Darkmatter applies the active
document's effective schema to each caller record. Exactly one applicable file
arm must be selected; ambiguous or unmatched unions remain the responsibility
of normal schema validation. A selected local `file(eager)` value resolves from
the caller origin and must identify an existing file. A selected non-recursive
lazy `file` value binds to the first ordered, lexically normalized candidate
from that same origin without checking whether it exists. Lazy HTTP(S)
references remain remote identities and are never sent through a local
candidate plan or filesystem probe. Recursive lazy references have no single
unprobed identity and fail with guidance to declare `file(eager)`.

The raw caller value remains unchanged for identity and fresh preparation. The
materialized semantic value is the native absolute path or typed remote
identity consumed by frontmatter expressions, path functions, validation, and
lifecycle state. Markdown body interpolation uses a separate portable
presentation value, so Windows can render `/` separators without changing the
native identity. Absent and explicit-null properties, ordinary strings, and
document-owned file references are not caller-materialized.

Proxy, retry, resume, inline-compose, and sequence/task entry preserve the same
raw value and per-property origin. Fresh reads rematerialize them against the
new active schema; a reused loop plan keeps its installed semantic identity.
Neither route recaptures process CWD, and a proxy target cannot re-anchor a
caller-owned value merely because it declares a different file mode.

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

### Documents That Declare `initialize`

A document declaring an `initialize` lifecycle stack is exempt from both the invocation-boundary verdict and the collection prompt above. `initialize` runs before schema validation (R4), and it can add or repair the very property a verdict would reject — writing frontmatter with `set_frontmatter`, or producing a file a `file`-typed property points at. Judging first would fail the document for a violation the next stage is about to fix, and prompting the caller would ask a question the document is about to answer itself.

The verdict is instead reached by the **stabilized reread**: canonical preparation re-reads the document after `initialize` returns and validates that read. A violation that survives `initialize` is therefore reported *after* the document's own `initialize` has run and *through* its own `blocked`/`finalize` stacks, as the same typed `CompositionError` a directly-invoked document reports. A proxied target follows the identical order through its staged bootstrap, which is what makes the diagnostic route-independent.

Documents without an `initialize` are unaffected: nothing can change their frontmatter between the read and the verdict, so they are judged where they are read.

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

## Launch-Anchored Prepared Context

Plain `ctx.*` in a composed document describes the caller's **launch context** — the directory Claudine was invoked from and the repository/package-area facts projected from it — never the prompt document's storage location and never the mutable process CWD (which the wrapper deliberately moves to the repo root).

- **One owner.** The launch anchor and the launch repository/topology/environment/host evidence are paired as a single operation on `InvocationContext` (`capture_launch_context` for a fresh document epoch, `extend_launch_context` for a same-epoch reread). A caller cannot combine a launch directory with prompt-derived evidence, so moving a prompt, task, group, overlay, or system-prompt file cannot change launch-facing `ctx.*` values (`ctx.area`, `ctx.repo_root`, `ctx.current_packages`, …). A source stored in another repository never substitutes that repository for the launch repository.
- **One snapshot per document epoch.** Direct, inline, loop, proxy-target, retry, and resume entry each prepare one target-adjusted early-binding snapshot after provider/model resolution and reuse that exact snapshot through shell preflight, body and effective-frontmatter composition, schema evaluation, loop conditions, and every lifecycle event. The post-`initialize` stabilized reread stays inside its epoch: newly demanded context groups are extended from retained launch evidence, and the anchor, environment capture, and applied target overrides never change. Proxying to another document, and retry/resume re-entry, start a new epoch (at most one new snapshot each).
- **Reuse is observable.** Invocation work accounting records populated prepared-context observations by the stable consumer names `preflight`, `body`, `effective-frontmatter`, `loop-condition`, and `lifecycle`. Performance reports project the sorted observed set alongside launch constructions, same-epoch extensions, and ambient fallbacks. Canonical preparation increments the fallback counter if an invocation owner is present but its prepared context is absent, so dropping the snapshot cannot pass a zero-fallback assertion invisibly.
- **Target identity is layered, not captured.** `ctx.agent`, `ctx.model`, `env.AGENT`, and `env.MODEL` reflect the resolved target's environment overrides applied on top of the launch snapshot, preserving target-identity precedence on every route.
- **The source context stays source-relative.** The active document's `SourceContext` (its authoring base, repository identity, and `FileResolutionContext`) remains authoritative for document-authored file references, transclusion, `$schema` discovery, and provenance. Source-relative resolution is unchanged by this contract.
- **`current.ctx.*` is unchanged.** It remains live event-time state, captured when the event fires, and is explicitly *not* a fallback for a missing prepared `ctx.*`.

In sequences, the graph phase runs before per-task target selection, so graph-resolved shell bytes cannot legally reference target identity: a command referencing `ctx.agent`, `ctx.model`, `env.AGENT`, or `env.MODEL` fails graph preflight with a typed target-identity rejection directing the author to the task that owns the target. Static bracket access such as `ctx["agent"]` is canonicalized to the same identity path. A computed index rooted at `ctx` or `env` also fails closed because its dynamic key could select a target-dependent leaf; computed indexes under other namespaces are unaffected. Per-task and just-in-time audits — where the selected target's environment is available — continue to expand those roots. The capture-owner drift guard in `cli/tests/composition_seams.rs` rejects any new direct prepared-context capture outside the invocation owner and its allowlisted compatibility sites.

## Document Handoffs and the Equivalence Contract

A run has exactly one **active document** at a time. A lifecycle `proxy` action replaces it: the target enters at its own `initialize`, becomes active, and owns everything from there — the remaining lifecycle, the closure, and the output. See [lifecycle.md — Proxy Handoffs](lifecycle.md#proxy-handoffs) for the authoring surface, including the `with:` overlay.

### The equivalence contract

> **A document reached through a proxy behaves like the same document invoked directly.**

`claudine compose target.md` and a `proxy` to `target.md` route through **one** canonical preparation service, so the route is not supposed to be a behavior. A proxy is a change of document, not a downgrade to a reduced pipeline: the target's `initialize`, schema validation, shell discovery and approval, `ctx.*` context, and typed diagnostics are all decided from the target's own stabilized frontmatter.

On the composition commands (`compose`, `inline-compose`, `sequence`) the contract holds for everything preparation owns **and** for the whole launch bundle. A handoff on those commands surfaces to the command-owned coordinator, which re-prepares the target as a fresh document and re-enters the production selection/MCP/argv pipeline, so provider, model, profile/binary sub-selection, argv entrypoint, MCP runtime injection, document-loop ownership, child CWD, and system-prompt delivery are all recomputed from the target's own frontmatter (see [Target-specific behavior](#target-specific-behavior) below).

A command that owns no coordinator to surface to — the **direct provider wrappers**, which prepare no active document — cannot satisfy that contract, so it refuses the handoff rather than running the target under a bundle the target did not choose. See [Handoffs on the direct provider wrappers](#handoffs-on-the-direct-provider-wrappers) below.

### Active-document ownership

Only the **coordinator** — which sits above both the document loop and the provider-attempt harness — may commit a change of document identity. Lifecycle evaluation cannot; it produces an *evaluated proxy request* (target string, resolved overlay, provenance) and hands it up. The provider harness cannot; it *returns* a `Proxy` transition rather than swapping its own source path. The coordinator alone resolves the target through the shared file resolver, checks hop and cycle state against the invocation-wide chain, and atomically commits a resolved handoff. No layer below it resolves the target a second time.

State is owned in four layers, which is what makes "what survives a handoff" answerable rather than incidental:

| Layer | Lifetime | Survives a proxy? |
|---|---|---|
| Invocation inputs (caller overrides, launch inputs) | the command | yes — immutable |
| Run ledger (proxy chain, hop accounting, approval cache, timing anchors) | the command | yes — extended, never reset |
| Prepared document | one active document | no — the target is prepared afresh |
| Active-document execution state (attempt, budgets, session) | one active document | no — discarded |

A failed handoff never half-activates the target: the source stays active for diagnostic attribution, the failure follows the normal event-aware routing, and no duplicate terminal or `finalize` event is synthesized.

### Entry reasons and the stage matrix

Every entry into canonical preparation declares **why**, and each reason has exactly one row — no reason falls through to another's policy:

| Entry reason | Built from | Emits `initialize` | Schema + full shell audit | Loop ownership |
|---|---|:-:|:-:|---|
| Direct document | the caller's resolved source | yes | yes | recognized from this document |
| Proxy target | a fresh read from disk | yes | yes | recognized from this document |
| Retry | a fresh read from disk | no | yes | inherits the active document's |
| Resume | a fresh read from disk | no | yes | inherits the active document's |
| Next loop iteration | the stamped structural plan | no | no | reuses the owning loop's plan |

Direct and proxy-target differ **only** in the read basis — that identity is the equivalence contract in table form. A proxy target reads fresh because the handoff commits to a document the source may never have touched.

`initialize` fires once per **active document**, not once per attempt: a retry or resume re-enters a document that has already initialized. A loop iteration skips validation because it re-materializes against an already-audited structural plan and therefore cannot introduce command bytes the audit never saw.

### Retry and resume re-entry

Retry and resume replace only the **provider-attempt slice** of the active document. They refresh the document canonically (a fresh read, full validation), keep the document's overlay and proxy provenance, and retain and decrement their own budgets — a retry cannot reset its budget by replacing the attempt. Retry drops any live session and starts a fresh attempt; resume keeps the session and delivers its follow-up message. Proxy and the next loop iteration are the two transitions that grant *fresh* budgets, because both are new active-document scope; retry, resume, proxy, and loop counters each have their own labeled home rather than sharing one counter.

**Launch identity is rebuilt at the fresh-read boundary, not snapshotted at adoption.** Every retry/resume re-materializes the active document from disk and recomputes the whole launch bundle against *that* read — provider, the profile and binary it selects, the resume protocol that profile supports, model, interactivity, the permission mode `--yolo` actually achieves for that pair, the structured-output shape it implies, the MCP tag set lexed from the refreshed body, the provider argv, and the environment overlay (`harness_orch/loop_control/target_launch.rs::rebuild_launch_identity`, called at every fresh-read boundary). An attempt therefore launches with the identity the document it is about to run resolves to now, not with an adoption-time snapshot.

**One rebuild per attempt, and it *is* the launch.** The rebuilt bundle is both what the child is spawned with and what the compatibility key below is computed from, so the key can never describe a plan the child did not receive. Argv and MCP injection come from `wrap/launch_plan.rs::build_launch_plan`, a re-entrant, side-effect-free builder the invocation feeds once with the results of every effect it performed (temp files, shadow HOME, MCP tag ambiguity resolutions — which is why a retry never re-prompts for an ambiguous tag). A rebuild whose facets match the invocation's gets that recorded plan back verbatim, so an unchanged document is byte-identical to the invocation by construction. System-prompt *delivery* is re-applied per provider and so moves with the provider; its *content* is composed once at invocation.

Retry and resume want opposite things from that rebuild. A **retry** opens a fresh session, so there is nothing to conflict with: it simply launches under the refreshed plan, and a document that changed `agent:`, `interactive:`, its permission mode, or its MCP `#tag`s before the retry gets a child spawned from the refreshed plan rather than the invocation's. A **resume** reuses a session the old plan opened, so a moved facet is a genuine conflict and refuses.

> **The resume session-compatibility key.** A `resume` retains the live provider session only when a compatibility key of the target's launch properties still matches across the canonical refresh; when a facet changed, the resume refuses with `CompositionError::LifecycleResumeIncompatible { facets }`, names the incompatible facets, and recommends `retry` to start a fresh session. The key is computed from the *final* typed launch bundle (`AttemptLaunch` — the resolved argv plus the effective child environment `build_harness_launch` produces) and digested with `biscuit_hash::xx_hash`; it distinguishes inline (`--append-system-prompt`) from file-backed (`--append-system-prompt-file`) system-prompt delivery, hashing the file's *content* rather than its unstable temp path. Its facets are provider, model, binary, resume protocol, workspace CWD, permission mode, interactivity, structured output, system prompt, and the MCP signal set; each is also projected and pinned at L1 (`harness_orch::session_key::tests`).
>
> **Reachability.** Every facet a document can move drives a real refusal end-to-end. Five isolating L2 rows — `level2_lifecycle_resume_refuses_when_refresh_changes_{model,provider,interactivity,permission_mode,mcp_server_set}` — each prove no second provider launch, the changed facet named on the pane, and `retry` recommended. Three further facets are named by those same rows rather than by a row of their own, because none has a document surface independent of the facet that determines it: `binary` and `resume protocol` move with `provider`, and `structured output` with `interactivity`. The converse — no false refusal — is `level2_lifecycle_resume_with_dropped_launch_flag_stays_compatible`, which proves an intentionally dropped resume-only flag does not trip the key.
>
> The remaining two facets are **immutable invocation inputs**, which R8 defines as such rather than leaving them as unreachable requirements. `workspace CWD` is resolved from the process launch directory before any document is read, and the only document surfaces over launch identity — `agent:`, `model:`, `interactive:` — name no directory. `system prompt` refers to delivered *content*, which is composed once at invocation and captured; the one mutation a lifecycle stack could attempt, rewriting the discovered `system-prompt.md`, provably moves neither delivery path. Both are carried in the key for completeness and held by L1 tests where they are computed. `SessionCompatibilityKey::extra` is deliberately empty: no provider adapter has a precise resume identity to contribute.
>
> **Refusal lifecycle shape.** `start` fires *before* the key comparison, so a refusal is a post-`start`, pre-spawn failure and takes the same shared typed catch protocol as every other failure in that window: `failure`, then exactly one `finalize`, both carrying the `LifecycleResumeIncompatible` diagnostic as `err.*` so a document's cleanup and `err`-aware recovery still run. `success` does not fire. Routing does not weaken the refusal — the re-run `failure` stack's `resume` control action is not dispatched from this path, so the provider is still never spawned a second time. All five L2 refusal rows assert the full trace, and `loop_control::tests::retry_resume::a_refused_resume_routes_through_failure_then_finalize_with_err` pins the routing decision at L1.

### Target-specific behavior

The target's stabilized frontmatter is the basis for the target's decisions. What is rebuilt per target today:

- **Context** — the prepared document stores the exact `ComposeContext` it composed against, captured once per document epoch from the invocation's launch context (see [Launch-Anchored Prepared Context](#launch-anchored-prepared-context)) with the resolved target's identity overrides applied. Body interpolation, effective frontmatter, lifecycle DM2 lookup, schema and file evaluation, and shell preflight all read that one stored snapshot; nothing recaptures ambient context at runtime, which matters because the wrapper deliberately moves the process CWD to the repo root. `current.ctx.*` remains live as a late-binding surface, and is explicitly *not* a fallback for a missing prepared `ctx.*`.
- **`initialize` and shell** — the target runs its own `initialize` behind a narrow safety gate that approves every potentially-selected `initialize` shell command first ("initialize before full pre-flight" never means "execute unapproved shell"), then rereads the stabilized target so initialize-time mutations are visible, then runs the full audit over every remaining lifecycle and template shell surface, reusing approvals the narrow gate already granted rather than re-prompting. An `initialize` proxy may chain another proxy; the chain stabilizes before any launch.
- **Schema and diagnostics** — the target's `$schema` validates the target's effective frontmatter (including any `with:` overlay), and a given failure has one typed identity whichever route reached it.
- **Launch identity** — when the handoff surfaces to the command-owned coordinator, `compose/prep.rs::prepare_and_run_active_document` re-prepares the target as a fresh document and re-enters the production selection/MCP/argv pipeline, rebuilding from the target's own frontmatter under explicit-CLI precedence: provider selection, profile/binary sub-selection, the argv entrypoint and flags, MCP runtime injection, the effective child environment, interactivity and structured-output mode, dispatch/correlation configuration, model selection, document-loop ownership/recognition, child CWD, and system-prompt delivery. A proxied target therefore selects its authored `agent:`/`model:`, gets its own provider binary and MCP server set, and acquires its own `loop:`, matching a direct invocation. Verified by L2 equivalence rows including a provider *switch* (`level2_lifecycle_equivalence_target_launch_bundle_matches_direct_run`, router `goose` → target `codex`; `level2_lifecycle_equivalence_target_mcp_injection_matches_direct_run`, router `codex` → target `gemini`).

#### Handoffs on the direct provider wrappers

The **direct provider wrappers** (`claudine claude`, `claudine goose`, …) take their prompt from argv or stdin and run a provider *memory file* as the harness document. They prepare no active document and carry no run ledger, so there is no coordinator to surface a handoff to and nothing that could re-enter the selection/MCP/argv pipeline for a target.

Wrapper harness detection has two distinct costs and behaviors. **Eligibility** parses only the candidate memory file's authored frontmatter with Darkmatter and checks `has_harness_properties`; a valid ordinary memory file returns “no harness” without composing its body. **Materialization** runs only for an enabled harness and uses the request-owned source context: its source-derived `FileResolutionContext`, demand-driven runtime evidence, explicit source-repository shell policy, and launch-root shell CWD. A malformed frontmatter document still fails with its typed parse error. No prompt means no memory-file lookup, and no candidate means neither parse nor materialization.

**A handoff raised there is refused, not adopted.** `surface_or_adopt_terminal_proxy` produces `CompositionError::LifecycleProxyWithoutOwningCoordinator`, naming the target, the wrapper that cannot host it, and `claudine compose` as a command that can. Nothing is committed and nothing is resolved: the request is refused while it is still an evaluated proxy request, so the source stays active and the refusal routes through the source's own `blocked`/`finalize` exactly as any other refused hop does.

Refusing is what keeps the equivalence contract total. Adopting the target in place would have run it under the *invocation's* profile, binary, argv entrypoint, and MCP injection — a reduced launch path the target's own frontmatter never chose, and one the contract forbids existing at all. There is no such path today; `rebuild_target_launch` runs only for a target the command coordinator already re-prepared, where it supplies the remainder (the lifecycle early-binding context and the `AGENT`/`MODEL`/`YOLO` env for the staged bootstrap).

### Command-level ownership

The coordinator is nested **inside** the command's own ownership — a handoff changes the document, never the command:

- **`compose`** routes the final active document's output to stdout.
- **`inline-compose`** stays inline mode across a handoff, but only the **final** target is eligible for the inline closure. The document that proxied away is not rewritten.
- **`sequence`** contains a proxy within its current step: no step advance, no restart, and the step keeps its scoped inputs and timing identity. There is no cross-step handoff.
- **`--dry-run`** never traverses a dynamic proxy route, and this follows structurally rather than from a per-route check: the dry-run seam returns *before* the lifecycle runtime is constructed, so `initialize` never fires and no `proxy` control can be produced. A dry run always reports the document named on the command line. See [Dry Run](#dry-run).

### Backward compatibility

The authoring surface is additive: positional `proxy: target.md` is unchanged, key/value `{ action: proxy, target: target.md }` is unchanged, caller overrides continue to survive every handoff, cycle protection and hop limits are unchanged, and action parameters other than `proxy.with` still reject direct mapping values.

> Reader note: the runtime refactor intentionally corrects behavior that
> depended on the router path. A proxied target may now execute additional loop
> iterations, select its authored provider/model, request approval for its own
> shell actions, or surface the typed error already produced by direct
> invocation. Those are compatibility fixes required by the equivalence
> contract, not preserved route quirks.

All four fixes named in that note are live today: a proxied target may execute additional loop iterations, select its authored provider/model, request approval for its own shell actions, and surface the same typed error direct invocation produces. Each is a compatibility fix required by the equivalence contract, not a preserved route quirk; see [`notes/acceptance-map.md`](../../features/2026-07-13-proxy-with/notes/acceptance-map.md) for the named criteria and their passing L2 rows.

## Migrating from the Retired Harness DSL

Earlier Claudine releases let composed documents declare `pre_checks`, `post_checks`, `handle_*` handlers, a programmatic `handle`, and `deviate` recovery commands in their frontmatter. That validation-and-handler DSL has been **removed**. Its gating, verification, and recovery roles are now expressed through the [lifecycle stack](lifecycle.md): `when:` guards plus the `error` / `skip` / `proxy` / `retry` / `resume` / `defer` lifecycle actions and `shell` actions.

A document that still declares any of these keys fails composition with a typed `RemovedValidationKey` diagnostic that names the offending key and points at its replacement surface:

| Removed key | Replacement |
|-------------|-------------|
| `pre_checks` | the `initialize` or `start` lifecycle stack |
| `post_checks` | the `success` or `finalize` lifecycle stack |
| `handle_<event>` (e.g. `handle_timeout`, `handle_inline_body_unchanged`) | the `blocked` or `failure` lifecycle recovery actions |
| `handle` | a lifecycle `shell` action or other lifecycle action |
| `deviate` | a lifecycle `shell` action plus a recovery action (`retry`, `resume`, etc.) |

The scan runs before lifecycle event blocks are parsed, so the diagnostic names the removed DSL key rather than falling through to generic unknown-field handling. Like every frontmatter-rooted composition error, it appends the authored frontmatter as a syntax-highlighted YAML block under a TTY.

**Verification** that the agentic loop actually did the work it claimed (the old `post_checks` role) now belongs in the `success` or `finalize` stack: guard a `when:` clause and raise an `Error` lifecycle action when the contract is unmet. **Recovery** (the old `handle_*` role) belongs in a `failure`/`blocked` stack — or any other event's, since flow control is universal — via `retry`, `resume`, or `proxy`. See [lifecycle.md](lifecycle.md) for the full action catalog.

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

Recovery is expressed through the lifecycle stacks, not a separate handler DSL. `failure` and `blocked` are its natural homes, but recovery is **not limited to them** — flow control is universal, so a `success` stack can `resume` an agent that finished cleanly without producing what it promised, and a `finalize` stack can `retry` an error the terminal event downgraded. The available recovery actions are:

- **`retry`** — re-run the prompt with a fresh provider attempt. Its re-entry point is derived from whether the provider had launched, not from the event that asked for it.
- **`resume`** — resume the agent session with its context intact and a follow-up message (provider must support session resume; pre-launch there is no session, and it surfaces `ResumeWithoutSession`).
- **`proxy`** — hand off to a different prompt document at its own `initialize`, optionally parameterized with a `with:` overlay. See [Document Handoffs](#document-handoffs-and-the-equivalence-contract).
- **`defer`** — re-run this prompt later as a fresh scheduled run. **Not implemented**: it surfaces a typed `LifecycleDeferNotImplemented` until its rendezvous backend lands.

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

Sequences use one unified step/task model. A task declares exactly one executable: a prompt file reference, shell command, side effect, group, or task file reference. A step with no executable composes the source document body.

Execution has two phases:

1. **Static preflight** walks the complete task graph before any task runs. Dynamic sources resolve once, referenced documents load transitively with cycle detection, schemas collect into one validation pass, and every reachable shell command is resolved and approved byte-for-byte.
2. **Just-in-time execution** re-reads and composes each task when its turn begins, so prior inline writes and runtime mutations are visible.

`outputs` is the sole task-output accumulator. Groups may run serially or in parallel with bounded `max_parallel`, snapshot isolation, declaration-ordered mutation merge, and all-siblings-complete failure handling. Parallel groups reject write-back collisions during preflight; group-level loops remain unsupported until commit semantics are defined.

A root `prompt` property selects inline closure behavior for the final active target. Step/task `prompt` values are file references; the two meanings occur at different levels. External YAML uses `sequence:` directly or kinded `task`/`group` documents. The retired external `kind: sequence` plus `list:` form is not accepted.

See [Sequences](flow-control/sequences.md) for the complete authoring and execution contract.

## Architecture

Both commands follow the same six-stage pipeline, with lifecycle events woven around the stages:

```
Resolve → Initialize → Pre-Flight → Prepare → Start → Select Provider → Launch → (Success | Blocked | Failure) → Finalize → Loop
```

- **Resolve**: `composition::resolve_composition_source()` loads the Markdown file
- **Initialize**: `LifecycleRunGuard::emit_initialize_once()` fires the `initialize` lifecycle event; a `skip` control action here exits cleanly before any later stage
- **Pre-Flight**: `composition::resolve_shell_approvals()` discovers every shell command in the document graph — template `::shell` directives, top-level frontmatter `$(...)` expressions, and lifecycle `shell` stack actions — checks whitelists, and prompts the user to approve any unapproved commands before proceeding (see [Pre-Flight Shell Approval](pre-flight-checks.md))
- **Prepare**: `composition::prepare::service::prepare_document()` — the canonical preparation service every entry reason routes through (direct, proxy target, retry, resume, loop iteration) — composes through Darkmatter via `prepare_direct()` / `prepare_inline()` with the pre-approved command set and produces a `PreparedComposition` with `effective_frontmatter`. There is exactly one composer per mode; see [Document Handoffs](#document-handoffs-and-the-equivalence-contract)
- **Start**: `LifecycleRunGuard::emit_start_once()` fires the `start` lifecycle event after schema validation and shell audit pass
- **Select**: `composition::select_provider()` applies the precedence chain
- **Launch**: `wrap::composition::execute_composition_request()` runs the provider through the full wrapper pipeline (env, MCP, harness, streaming)
- **Terminal**: `LifecycleRunGuard::emit_terminal()` fires `success`, `blocked`, or `failure`
- **Finalize**: `LifecycleRunGuard::emit_finalize_once()` fires `finalize` once per iteration
- **Loop**: the post-`finalize` gate evaluates `loop:` lifecycle concerns, the `while`/`until` condition, and applies per-iteration mutations when continuing
- **Closure**: `composition::closure::rewrite_inline_document()` reconstructs the document for inline mode; direct mode outputs to stdout

The original six-stage summary (`Resolve → Pre-Flight → Prepare → Select Provider → Launch → Closure`) describes the functional pipeline; lifecycle events are the hooks that run at the boundaries between those stages.

### Request-Owned Context

Canonical wrappers and composition commands create one `InvocationContext` before resolving the first source. It freezes the launch CWD, HOME, environment, launch repository evidence, and launch `FileResolutionContext`, then projects the existing launch, workspace, and event context types from those facts. Once a file reference resolves, a `SourceContext` carries that document's base directory, repository/package roots, repository observation, and source-derived `FileResolutionContext` through preparation, preflight, lifecycle evaluation, sequences, system prompts, and harness materialization.

Darkmatter scans each document for its required `ctx.*` groups and receives the matching evidence from the invocation owner. Canonical paths therefore reuse the same environment and repository facts instead of recapturing ambient CWD, HOME, Git, or topology downstream. Content is still reread at retry, resume, and sequence JIT boundaries; only immutable request and repository evidence is reused.

Repository observations are cached per worktree identity, with topology initialized once per distinct repository even when parallel sequence tasks enter it together. Same-repository sources reuse launch topology; multiple sources in one sibling worktree share one additional observation and topology probe; exact non-repository directories cache explicit absence. File resolution remains source-aware because every compose receives the selected source's retained `FileResolutionContext`.

Shell execution follows workspace intent rather than source location. Composition documents and system prompts run `::shell` from the launch repository root; for a launch outside a repository, the explicit launch CWD is used. A prompt or harness stored in another directory or repository does not move the agent's shell working directory.

A sequence step's explicit task stack is the one exception to "the document a step composes and runs owns its `SourceContext`": `setup`, `teardown`, and the primary `side_effect` action derive their `SourceContext` from the document that authored the task, not the document the step composes and runs. `set_frontmatter` and other file-touching effects in that stack therefore target files next to the task's origin document, including when an externalized `task:`/`group:` file lives in a different repository from the step's `prompt:` document. Prompt-task `params` retain that same authoring origin when the target schema selects a file value; immutable CLI caller records remain a separate, higher-precedence layer with the invocation's launch origin.

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
│  └─ unattributed                   150.2ms   39%
├─ environment setup                  85.7ms   22%
│  ├─ launch discovery                 8.0ms    2%
│  ├─ system prompt                   30.0ms    8%
│  │  ├─ lookup                         2.0ms   <1%
│  │  ├─ runtime capture                8.0ms    2%
│  │  ├─ primary compose               10.0ms    3%
│  │  ├─ appendix compose               5.0ms    1%
│  │  └─ delivery                       5.0ms    1%
│  ├─ child env build                 15.0ms    4%
│  │  ├─ env sanitize                   1.0ms   <1%
│  │  └─ shadow home sync              13.0ms    3%
│  │     └─ repo root detect            1.0ms   <1%
│  ├─ mcp composition                  5.0ms    1%
│  ├─ harness eligibility              3.0ms   <1%
│  ├─ harness materialization         20.0ms    5%
│  └─ unattributed                     4.7ms    1%
├─ agent execution                        —       —  (dry run)
└─ unattributed                       64.9ms   17%
```

The model and its invariants:

- **Headline is true wall-clock.** The `Performance` total is sampled once at report-build from a single process-start baseline, so it can never disagree with the body the way a mid-flight timer could.
- **Structural buckets reconcile.** Every `Structural` node's children (plus a synthetic `unattributed` remainder) sum back to the node's own total. The top-level buckets (`pre-dispatch`, `prep phase`, `environment setup`, `agent execution`) therefore sum back to the headline. A debug assertion enforces this at runtime; a unit test (TR-4) enforces it for every command shape.
- **Breakdown rows itemize without double-counting.** Darkmatter composition stages, system-prompt internals, child-environment internals, and the `pre-dispatch`/agent sub-rows are `Breakdown` children — shown and percentaged, but excluded from the reconciliation sum so no cost appears twice. Single-shot `compose`/`inline-compose` nest the `composition` subtree under `prep phase`; sequence composition appears under the step where it ran.
- **Direct-wrapper setup is attributed.** `environment setup` structurally separates launch discovery, system prompt, child environment, MCP composition, harness eligibility, and enabled-harness materialization. System prompt expands into `lookup`, `runtime capture`, `primary compose`, `appendix compose`, and provider `delivery`; child environment may expand into `env sanitize` and `shadow home sync → repo root detect`. Harness materialization is omitted when eligibility finds no enabled harness.
- **Percent column** shows each row's share of wall-clock (`100%` at the root, `<1%` for sub-one-percent slivers).
- **`HOT` marker** flags the single dominant leaf when it clears the materiality floor (≥20% of wall-clock).
- **Run counts** (`×N`) appear on a composition stage that ran more than once.
- **Dry runs** render `agent execution` as an `—` leaf annotated `(dry run)`; the agent never launches.

For `sequence`, the report is aggregated across all steps: launches and total execution time are summed, first-response latencies are averaged (with the minimum shown in a note), and composition metrics are merged. The report appears exactly once at the end of the run, after the sequence summary.

`--perf` is a stderr-only artifact — it never writes to stdout, so it does not interfere with piped composition output. It is emitted unconditionally when passed, even alongside `--silent` or `--quiet`, because it is an explicit opt-in.

> **Note:** `provider_api_duration` is only populated for structured-streaming providers. Legacy providers (e.g., Goose) omit this line.

## Module Structure Audit (Phase 2.12)

`composition/mod.rs` declares a mix of `pub mod` and `mod` (private) children. The private modules (`error`, `guardrails`, `prepare`, `resolve`, `select`, `types`) are re-exported via `pub use` where their items are part of the public API surface (e.g., `CompositionError`, `prepare_direct`, `resolve_composition_source`). This is an intentional design: the module tree is private, but the public types surface through targeted `pub use` re-exports. No widening of privacy was needed during the Phase 2 audit.
