---
blast_radius:
  - darkmatter/features/2026-04-08-shell-expansion-in-fm/spec.md
  - darkmatter/features/2026-04-08-shell-expansion-in-fm/tech-design.md
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
  - darkmatter/lib/src/markdown/compose/types.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/cli/src/commands.rs
---

# Frontmatter Shell Expansion

Frontmatter Shell Expansion allows shell commands to be executed during the compose pipeline and their stdout output, or their result, stored as frontmatter property values.

## Syntax

A top-level frontmatter property whose entire string value matches one of these patterns is treated as a shell expression:

```text
$(<command and args>)
$(<command and args>)::<suffix>::<suffix>...
```

Five suffixes may follow the closing `)`, in any order:

| Suffix | Effect |
| --- | --- |
| `::ok` | The value is a boolean: `true` when the command exited `0`. |
| `::exit-code` | The value is the exit status as a number, or `null` for an allowed timeout. |
| `::result` | The value is an object `{ ok, code, stdout, stderr }`. |
| `::timeout:<seconds>` | Overrides the shell timeout for this command. |
| `::no-cache` | Runs the command fresh; see [Caching](#caching). |

Examples:

```yaml
---
files: "$(sniff repo dirty-files)"
cwd: "$(pwd)::timeout:1"
build_id: "$(uuidgen)::no-cache"
tree_is_clean: "$(git diff --quiet)::ok"
lint: "$(just lint)::exit-code::timeout:120"
diff: "$(git diff --quiet)::result"
---
```

## Rules

- The **entire** frontmatter value must be the shell expression -- embedded expressions like `"prefix $(cmd) suffix"` are not supported. Leading whitespace before `$(` is allowed.
- The shape is read from the value **the author wrote**, before interpolation. See [Only Authored Commands Run](#only-authored-commands-run).
- Only top-level string-valued frontmatter properties are scanned. Nested objects and array elements are ignored.
- The optional `::timeout:<N>` suffix overrides the global shell timeout for that specific command. `N` must be a positive integer of seconds.
- The optional `::no-cache` suffix bypasses the per-compose command cache so the command executes fresh at each occurrence.
- At most one of the three result suffixes (`::ok`, `::exit-code`, `::result`) may appear, and no suffix may appear twice. A result suffix combines with `::timeout:<N>` and `::no-cache` in any order (e.g. `$(uuidgen)::no-cache::timeout:5::ok`).
- A suffix is never part of the command. Approving `$(git diff --quiet)` approves `$(git diff --quiet)::ok`.
- Once a value matches the `$(` shape, malformed syntax is a hard compose error. Two result suffixes (the error names both), a repeated suffix, an empty `::`, an unrecognized suffix or text after a suffix (the error lists all five), an invalid timeout, tokenizer failures, and rejected executable interpolation are never silently ignored.
- Closing `)` characters inside quoted arguments are supported, so values like `$(printf ')')` parse correctly.

## Reading a Command's Result

Without a suffix, the value is the command's trimmed stdout, and a non-zero exit
fails the composition. A result suffix turns the command's **exit status** into
a value instead, so a document can branch on it:

```yaml
---
tree_is_clean: "$(git diff --quiet)::ok"
diff: "$(git diff --quiet)::result"
---
{{ tree_is_clean ? "Nothing to commit." : "There are local changes." }}
{{ diff.ok ? "" : diff.stderr }}
```

| Outcome | no suffix | `::ok` | `::exit-code` | `::result` |
| --- | --- | --- | --- | --- |
| exits `0` | trimmed stdout | `true` | `0` | `{ ok: true, code: 0, stdout, stderr }` |
| exits `N` ≠ 0 | compose error | `false` | `N` | `{ ok: false, code: N, stdout, stderr }` |
| times out, default | compose error | compose error | compose error | compose error |
| times out, `--allow-shell-timeout` | `""` and a warning | `false` | `null` | `{ ok: false, code: null, stdout: "", stderr: "" }` |

`ok` is always `code == 0`, and `stdout`/`stderr` are trimmed the way plain
stdout is. The values are typed: `::ok` is a real boolean, `::exit-code` a real
number, and `::result` a real object whose members are reachable with dotted
access (`diff.stderr`). `$schema` validates the typed value; see
[Pipeline Placement](#pipeline-placement).

Only an exit status is forgiven. A command that is missing, blacklisted, or
denied fails exactly as it does without a suffix. Three things are never a
value:

- a ternary whose **condition** raises: an expression error, as always;
- a command ended by a **signal**: it has no exit status, so no number is
  invented for it;
- a **user interruption** (Ctrl+C): it keeps its cancellation outcome and never
  reads as `ok: false`.

### Every shape of `$( … )`

Darkmatter launches each command of a `$( … )` itself, so the rules do not
depend on the host's shell. `|` and `;` stay rejected.

| Shape | `code` | `stdout` / `stderr` |
| --- | --- | --- |
| single command | its exit status | its streams |
| `a && b`, `a \|\| b`, longer chains | the status of the **last command that ran**, what `$?` would hold | the streams of every command that ran, in order, joined with a newline |
| ternary selecting a command or chain | as the rows above | as the rows above |
| ternary selecting a **literal** | `0` | `stdout` is the literal's text; `stderr` is empty |

```yaml
handled: "$(git diff --quiet ref || echo handled)::ok"          # true when the fallback ran
lint: "$( has_command('just') ? just lint : 'skipped' )::result" # { ok: true, code: 0, stdout: "skipped", stderr: "" } without `just`
```

To learn **which** command in a chain failed, give each its own property.
Under `--allow-shell-timeout`, a timed-out command in a chain counts as a
success for `&&`/`||`, as it always has, and its code is `null` only when it is
the last command that ran.

## Token Resolution

Inside a `$( … )`, the engine and the shell coexist. A token in **executed
position** (a non-ternary directive body, or a ternary branch) resolves by a
precedence ladder — quoted/numeric/boolean literal → `name(...)` safe expression
function → path-bearing executable → bare name on `PATH` (executable) → bare name
frontmatter property → `null`. `true`/`false` are always booleans, path-bearing
tokens are always executables, and `doc.<name>` forces a frontmatter-property
reading even when a same-named executable exists.

A `$()` must resolve to at least one real shell command in executed position
(for a ternary, at least one branch; the condition never counts). A `$()` that
is entirely expression content — e.g. `"$( file_exists('x') ? 'a' : 'b' )"` —
is rejected with a diagnostic suggesting `{{ … }}` instead. Mixed forms such as
`"$( file_exists('Cargo.toml') ? cargo build : make )"` are fully supported.

See [Token Resolution in `$()` Shell Expressions](../topics/darkmatter-expressions.md#token-resolution-in--shell-expressions)
for the full ladder, the validity rule, and preflight behavior.

## Remote URLs

The `$()` shell ternary condition/branch shares the same local-filesystem-only
resolution context as frontmatter interpolation. A remote URL argument passed
to a read-side function there fails loudly rather than being fetched. Use body
interpolation for remote reads.

## Pipeline Placement

Frontmatter Shell Expansion runs in the **Inline Pre** phase, bracketed by the
two frontmatter interpolation passes and before EffectiveState construction:

1. Merge external/inherited state
2. Apply `--set` overrides
3. **Frontmatter Interpolation (pass 1)** -- resolve `{{ }}` expressions; defer keys that reference a whole-value `$(...)`
4. **Schema Validation** -- validate/coerce frontmatter against `$schema` (values still holding `$(...)` are deferred)
5. **Frontmatter Shell Expansion** -- execute `$(cmd)` expressions
6. **Frontmatter Interpolation (pass 2)** -- resolve only the keys deferred in pass 1, from their authored source, against the now-concrete shell-expanded values; shell output itself is never scanned
7. **Schema Validation (typed shell values)** -- every value a result suffix produced is validated against `$schema`, so a property declared `boolean` is judged against what `::ok` produced and a mismatch is a compose error. An unsuffixed value's stdout text is not re-judged here; a caller that owns post-expansion validation (Claudine) does that
8. Build EffectiveState
9. Body operations continue...

Because interpolation runs first, shell commands can use interpolated values as arguments:

```yaml
---
file: README.md
dir: "$(dirname {{file}})"
---
```

After interpolation, the shell stage sees `$(dirname README.md)`.

## Security

### Only Authored Commands Run

A frontmatter value runs as a command only when **the value the author wrote**
is a whole-value `$( … )`. Interpolation can fill in a command's arguments, but
it cannot create the command shape, and text an operation produced is never
treated as a command however it reads:

| Value | Runs? |
| --- | --- |
| `dir: "$(dirname {{file}})"` | yes: authored `$( … )`, interpolated argument |
| `cmd: "{{ '$(echo X)' }}"` | no: the expression's result is the text `$(echo X)` |
| `cmd: "{{ frontmatter('n.md', 'note') }}"` where that note is `$(echo X)` | no: a file read is data |
| a `--set` value of `$(echo X)` | yes: a person typed it, so it is authored and goes through approval |
| a data override or a decoded [literal token](./interpolation.md#literal-tokens) holding `$(echo X)` | no |
| shell output that prints `$(date)` | no: the output is stored as the text `$(date)` |

```yaml
---
x: "$(echo X)"                # runs (after approval); x is `X`
cmd: "{{ '$(echo Y)' }}"      # never runs; cmd is the text `$(echo Y)`
---
```

Preflight approval and the real run make this decision with the same
predicate, so the approval list for the document above holds `echo X` only.
`md compose --shell` reports exactly that set. The decision does not depend on
whether shell expansion is enabled, so preflight and the run agree in both
modes.

This is why the rule exists: a value that came from a file, a command, or an AI
agent's output can contain `$( … )` without ever becoming a command.

### Executable Token Rule

The executable (first token) of a frontmatter shell command must **not** come from interpolation. Only arguments may be interpolated.

Rejected:

```yaml
cmd: ls
bad: "$({{cmd}} -la)"        # executable from interpolation
```

Accepted:

```yaml
file: README.md
dir: "$(dirname {{file}})"   # only argument is interpolated
```

### Approval

Frontmatter shell commands participate in the same approval flow as body `::shell` directives. They are included in preflight discovery and subject to whitelist, blacklist, and interactive approval. See [Pre-Flight Shell Approval](../topics/pre-flight-checks.md) for the full policy details.

Discovery and runtime execution use the same pre-compose frontmatter preparation path:

- external state is merged first
- `--set` overrides are applied next
- frontmatter interpolation runs before scanning/execution
- both decide whether a key is a command from its authored source value

This keeps approval preflight aligned with the commands that real compose will execute.

## Error Handling

Frontmatter shell expansion has **no error-recovery options**. Without a result suffix, any non-zero exit code, missing executable, blacklisted command, denied approval, or malformed shell expression results in an immediate compose error. A result suffix forgives only the exit status; see [Reading a Command's Result](#reading-a-commands-result). This is intentionally simpler than body `::shell` directives. The error stops the composition in a transcluded file as well as in the root document.

Timeout failures follow the timeout behavior configured via `--allow-shell-timeout` (CLI) or `ComposeOptions::with_allow_shell_timeout()` (library).

### Post-Expansion Leak Guard

When frontmatter shell expansion is enabled, a final pass over every
**authored** top-level string value rejects any value that *still* trims to a
whole-value `$(...)` candidate after expansion has run. An authored value that
is exactly an expansion form is executable state: it must run or fail, never
reach the composed frontmatter as raw syntax. A surviving whole-value candidate
is a hard compose error tagged with the offending frontmatter key and its
source line.

The guard inspects unresolved authored syntax, not data. Shell output, an
expression's result, a data override, and a decoded literal token are exempt
however they read, so `out: "$(printf %s '$(date)')"` composes to the text
`$(date)` rather than failing.

The guard runs only when shell expansion is enabled. When frontmatter shell
expansion is **explicitly disabled**, `$(...)` values are deferred unchanged and
the guard never runs. Mixed and trailing forms (`literal $(echo ok)`,
`$(echo ok) trailing`) are outside the whole-value rule and pass the guard
untouched.

## Output Normalization

- Without a result suffix, only `stdout` is written back into frontmatter. Successful `stderr` output is ignored for value storage; `::result` exposes it as `stderr`.
- The `stdout` from a frontmatter shell command is trimmed of all surrounding whitespace (`.trim()`) before being stored as the frontmatter value.
- The stored output is data. Pass 2, body interpolation, and transcluded children insert it as text and never evaluate a `{{ … }}` or run a `$( … )` it contains.

## Concurrency

When multiple top-level frontmatter properties contain shell expressions, they execute concurrently after approvals and policy checks have been resolved. Results are written back in deterministic top-level frontmatter iteration order.

## Caching

One compose runs an identical command once. The per-compose cache stores the
command's **whole outcome** — status, stdout, and stderr — so every reader
derives its own value from one run:

```yaml
a: "$(git diff --quiet)"          # these three run `git diff --quiet` once
b: "$(git diff --quiet)::result"
c: "$(git diff --quiet)::ok"
```

- An outcome is reused only under an equivalent execution context (working
  directory and color environment) and the same deadline and timeout policy. A
  `::timeout:30` reader never reuses an outcome obtained under `::timeout:31`.
- Concurrent identical requests share one execution: the second waits for the
  first instead of starting its own.
- A cached non-zero outcome still fails an unsuffixed reader, exactly as a
  fresh one would.
- `::no-cache` neither reads nor populates the cache.
- The cache spans one compose, transcluded children included. A caller that
  runs shell values outside a compose (a Claudine lifecycle `set`) gets a fresh
  cache for each execution.

## Timeouts

- Default global timeout: 10 seconds
- Override globally: `--timeout <seconds>` (CLI) or `ComposeOptions::with_shell_timeout()` (library)
- Override per-command: `$(cmd)::timeout:<seconds>`
- Timeout outcome:
    - Default: compose error, with or without a result suffix
    - With `--allow-shell-timeout`: empty string replacement + warning; with a
      result suffix, `false`, `null`, or `{ ok: false, code: null, … }`

## Compose Reporting

The compose report tracks frontmatter shell expansion separately from body shell expansion.

- `ComposeOperation` variant: `FrontmatterShellExpansion`
- Phase: `InlinePre`
- Report field: `frontmatter_shell_expansions_applied`
- Perf metric name: `frontmatter shell expansion`

## Drift Detection

This document may need review when any of these files change:

- `darkmatter/features/2026-04-08-shell-expansion-in-fm/spec.md`
- `darkmatter/features/2026-04-08-shell-expansion-in-fm/tech-design.md`
- `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs`
- `darkmatter/lib/src/markdown/compose/shell_expansion/types.rs`
- `darkmatter/lib/src/markdown/compose/types.rs`
- `darkmatter/lib/src/markdown/compose/mod.rs`
- `darkmatter/cli/src/commands.rs`
