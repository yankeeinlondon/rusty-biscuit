---
kind: canonical-documentation
---

# `md compose`

The `compose` command assembles a Markdown document by evaluating its expressions,
applying transforms, and including referenced content. It writes the composed
result and, when the root declares lifecycle events, runs those events around
composition.

It is used for document assembly workflows like:

- interpolation (`{{ ... }}`)
- replacement transforms
- transclusion (`::file`, `::code`, `::toc-linking`, `::file-links`, and related directives)
- **link resolve** (converts relative links to absolute paths before transclusion)
- **link normalization** (converts absolute paths back to portable forms in the final document)

Unlike `read`, `compose` focuses on transformed content, not display rendering defaults.

Markdown output exports `::file-links` as readable linked lists without internal
render-tree metadata comments. Local `file:` URLs use the same portable-link
normalization as absolute paths: verified bare tree-root links first, then nearby
relative links and repository references such as `&docs/standards/mode-1004.md`. Terminal, HTML, and JSON
output retain the component's styling and absolute hyperlink targets.

## Reporting

### Usage

```bash
# Compose a file
md compose doc.md

# Compose stdin
cat doc.md | md compose
md compose -

# Provide default values as JSON or JSON5
md compose doc.md --state '{"name":"Alice","env":"prod"}'
md compose doc.md --state '{name: "Alice", env: "prod"}'

# Override values using --set
md compose doc.md --set '{"name":"Bob"}'

# Override values using shorthand setters
md compose doc.md name=Bob iteration=1
md compose - name=Alice draft=false

# Include frontmatter in output
md compose doc.md --frontmatter
md compose doc.md --fm

# Render composed output as HTML or JSON
md compose doc.md --output html
md compose doc.md --output json

# One JSON object: the outcome, every diagnostic, and the document
md compose doc.md --output report-json

# Show output via temp artifact
md compose doc.md --show

# Report shell commands that compose could execute, without running them
md compose doc.md --shell
```

### Shorthand Setter Syntax

Positional arguments matching `key=value` are treated as override setters, equivalent to adding the key-value pair to `--set`. This provides a more ergonomic syntax for common overrides:

```bash
md compose doc.md iteration=1 draft=false name=Alice
```

**Value parsing**: Values are parsed as JSON5 with a string fallback. This means:
- Numbers (`iteration=1`) and booleans (`draft=false`) are parsed as their native types
- Objects and arrays can use JSON5 syntax: `meta={author:"Alice"}`
- Unquoted strings are treated as literal strings: `name=Alice`
- Empty values (`empty=`) resolve to an empty string

**Key grammar**: Keys must match `[A-Za-z_][A-Za-z0-9_-]*`. Tokens with `/`, `:`, or other path-like characters in the key portion are treated as input paths rather than setters. This means `./foo=bar.md` is correctly classified as a file path.

**Limitation**: Shorthand setters only set top-level keys. For nested paths, use `--set` with dot-notation or JSON objects.

### Arguments

- `[ARGS]`: Positional arguments — an optional input file path followed by zero or more `key=value` setters. Use `-` for stdin. If no path is provided, reads stdin when piped; otherwise errors.

### Options

- `--state <JSON>`: default values as JSON or JSON5; fills in null/missing frontmatter keys without overriding existing values.
- `--frontmatter` / `--fm`: include frontmatter in the output (default: body only).
- `--output <markdown|text|markdown-plus|html|browser|json|ast|auto|report-json>`: output format (default: `markdown`). `report-json` prints the [composition report](./report-json.md) instead of the document, and cannot be combined with `--show` or `--shell`.
- `--show`: open output via temp artifact.
- `--allow <ID|all>`: permit a recovery, or reads from an explicitly named host. Repeatable. `all` permits every recovery but never grants a host or authorizes a shell command. See [Policy Flags](./policy.md) for precedence and conflicts.
    - `missing-hyperlinks`, `missing-image-refs`: keep the content when a hyperlink or image target is missing; issues are reported on stderr and the exit code is 0.
    - `missing-transclusions`: remove a transclusion directive whose target is missing; issues are reported on stderr and the exit code is 0.
    - `ctx-override`: replace a non-object `ctx` frontmatter value with the runtime context. By default, a document that defines `ctx` as a non-object (e.g., a string or array) causes a hard error; this allowance downgrades the error to a warning.
    - `invalid-frontmatter-assignment`, `reassigned-frontmatter-property`: recover from an invalid or duplicated `set` assignment on a `::file` directive (see [Block Transclusion](../transclusion/block-transclusion.md)).
    - `shell-timeout`: replace a timed-out shell command's output with an empty string and a warning.
    - `untyped-map-variables`, `untyped-filter-variables`: accept a `::map` or `::filter` source that has no iterable declaration, reporting a warning instead of failing. Only the static typing requirement is waived; a source known to be unsuitable still fails, and shell approval still applies.
    - `host=HOST`: grant network access to a host or address (see below).
- `--deny <ID|all>`: make the selected diagnostics blocking, so compose fails and writes no document. Repeatable. Takes any recovery identifier above, `style` (unknown or deprecated `style:` keys and inline horizontal-rule deprecations), a diagnostic code such as `dm.reference.missing_hyperlink` (`md compose --help` lists them), or `all`. Informational notices never block, and `--deny all` never revokes an `--allow host=HOST`.
- `--disable <ID>`: stop a feature from running. Repeatable; there is no `all`.
    - `baseline-schema`: validate without the default Darkmatter baseline frontmatter schema.
    - `schema-triggers`: skip repository schema-trigger discovery and bare-name schema-root lookup.
    - `collection-blocks`: skip the `::map` and `::filter` directive stage, so those directives are not expanded.
- `--max-iteration-visits <N>`: how many source items one compose request may visit across every `::map` and `::filter`, nested directives and child compositions included. Rejected items count. `N` is a positive integer (zero, a sign, or a value beyond 64 bits is a usage error with exit status 2). When omitted, the library default (100,000) applies; there is no unlimited setting and no recovery for exhaustion.
- `--max-expansion-bytes <N>`: how many bytes of `::map` and `::filter` expansion one request may materialize, nested copies included. Same value rules as `--max-iteration-visits`; the library default is 16,777,216 (16 MiB).
- `--shell`: report the effects discovered in the compose tree and exit without performing them. The report includes body `::shell` directives, top-level frontmatter `$(...)` shell expressions, and the root document's [lifecycle](#lifecycle-events) `shell` actions, including commands discovered through markdown transclusions, followed by the `ping`/`ping_under` probes the tree could send and whether an `--allow host=HOST` entry already grants each one. No command runs and no packet is sent. A [schema trigger skipped as invalid](../topics/schemas/schema-triggers.md#invalid-discovered-triggers) is reported on stderr after the report.
- `--perf`: emit a structured performance report to stderr after compose completes. The report includes both command-level timings (input loading, context capture, validation, option construction) and per-stage compose pipeline timings. The report is printed after any compose warnings and deferred validation issues.
- `--allow host=HOST`: grant network access to a host or address. Repeat for multiple entries. The default policy is deny-all, and the entry's shape decides which capability it grants:
    - a hostname or `*.wildcard` pattern allows remote URL reads from that host, and grants no ICMP;
    - an exact IP literal allows remote URL reads from that exact host **and** grants `ping`/`ping_under` to that address (an IPv6 zone constrains the ICMP grant to that interface, spelled the way the host spells it — the interface name on macOS and Linux (`fe80::1%en0`), the numeric interface index on Windows (`fe80::1%12`));
    - a strict CIDR such as `10.1.0.0/16` grants `ping`/`ping_under` to every address in the range of that family, and authorizes no HTTP at all.

  An ICMP probe to an address no entry grants returns `null` with a compose warning and sends nothing.
- `--remote-concurrency <N>`: maximum concurrent remote fetches. Defaults to `16`, or the value of the `DARKMATTER_REMOTE_CONCURRENCY` environment variable when the flag is omitted. An explicit flag overrides the env var, which overrides the default.
- `--remote-ttl <SECONDS>`: set the freshness lifetime of transport-cached remote bodies. Without this, the server's `max-age` is used when present. It never makes a `no-store` response storable or a `no-cache` response fresh.
- `--remote-refresh`: revalidate every transport-cached remote body with a conditional GET, even when fresh.
- `--remote-freshness <strict|fallback|optimistic>`: choose stale-cache behavior for remote artifacts. Defaults to `fallback`, which serves the stale cached body when revalidation fails on the network. `strict` fails instead; `optimistic` serves any cached body without revalidation. A `no-cache` response is always revalidated and never served stale.
- `--cache-root <DIR>`: the transport-artifact cache root. It persists raw remote URL response bodies only, under `<DIR>/.darkmatter/cache/v1/`, so `--remote-ttl`, `--remote-refresh`, and `--remote-freshness` apply across CLI runs. Semantic results (composed documents, `::file` children, `::code` and `::toc-linking` results, and document snapshots) are never persisted; every run recomposes local content. Nothing is created on disk until a storable remote response is written, and a cache root never authorizes a host. See [Caching](../topics/caching.md).

### Compose Warnings

Compose warnings (context merge collisions, partial runtime capture failures, etc.) are printed to stderr after the composed content is written to stdout.

### Output Behavior

**Default (`--output markdown`)**

- Prints composed markdown content.
- Frontmatter is consumed as pipeline input and stripped from output unless `--frontmatter` is specified.

**`--frontmatter` / `--fm`**

- Includes frontmatter in the markdown output, reflecting any values filled in by `--state`.
- Key order is preserved from the source document; new keys appear at the end.

**`--output auto`**

- Treated the same as markdown for compose.

**`--output html|json`**

- Emits HTML or AST JSON from the composed document.

**`--output report-json`**

- Prints one JSON object with `outcome`, `diagnostics`, and `document` (the Markdown, or `null` on failure), and nothing else on stdout. Warnings are in the object rather than on stderr. See [Composition Report](./report-json.md).

**`--show` behavior**

- For markdown/auto: prints composed content and opens markdown artifact.
- For html/json: opens artifact instead of printing to stdout.

### Precedence

Override values are applied in this order (highest priority last):

1. **Document frontmatter** — base values from the document itself
2. **`--state`** — fills null/missing keys without overriding existing values
3. **`--set`** — unconditional overwrite of matching keys
4. **Shorthand `key=value`** — unconditional overwrite, highest CLI priority

Shorthand setters are merged into the same `--set` override map. On key collision, the shorthand value wins.

```bash
# --state provides defaults, --set overrides, shorthand overrides both
md compose doc.md --state '{"name":"Alice"}' --set '{"name":"Bob"}' name=Charlie
# Result: name is "Charlie"
```

### Transform Context

- If `[INPUT]` is a file path, compose sets source-file context for relative transclusion resolution.
- If input is stdin (`-` or piped with no input arg), source-file-relative path resolution is not available.
- All file path arguments accept the full [file reference grammar](./index.md#file-references) — `@` magic paths search package, package area, repository root, then home; `&` pins the repository root and `^` searches package → package area → repository root.

### Validation and Errors

- **Reference validation** runs automatically before composition when input is a file path. If any references (hyperlinks, images, transclusions, CSS imports, etc.) point to missing targets, compose reports grouped errors and exits with code 2. It is passive: body `::shell` directives are not run for it, so a reference only a command's output would contain is checked when the document composes.

  ```
  Invalid Hyperlink(s)
  - the preparation.md reference to @darkmatter/docs/text-replacement.md is not valid
  - the preparation.md reference to @darkmatter/docs/interpolation.md is not valid

  19 references scanned, 17 valid, 2 issues
  ```

  Issues are grouped by category (hyperlinks, images, transclusions, etc.) with the source document shown as a clickable link and broken targets in red.

- **`--allow` allowances** relax validation for specific categories. When all errors belong to allowed categories, compose proceeds normally: content goes to stdout, issues are reported on stderr, and the exit code is 0. If any errors remain in non-allowed categories, compose still blocks with exit code 2.

  ```bash
  # Compose despite broken links — content to stdout, warnings to stderr
  md compose doc.md --allow missing-hyperlinks

  # Compose despite any broken reference
  md compose doc.md --allow missing-hyperlinks --allow missing-image-refs --allow missing-transclusions

  # Pipe composed output while still seeing warnings
  md compose doc.md --allow missing-hyperlinks > output.md
  ```

- **`--deny` selections** make diagnostics blocking. One found by passive analysis (the call check, reference validation, the lifecycle call check) stops compose before anything runs; one found by shell preparation, which walks every transcluded document whatever its `when=` (such as an invalid schema trigger only a skipped child holds), stops compose before any shell command is approved or runs; one found while composing stops compose before the document is written, without undoing effects that already ran. See [Policy Flags](./policy.md).

- Invalid JSON/JSON5 in `--state` returns an error.
- Non-object JSON (array/string/number/etc.) in `--state` returns an error.
- Transform failures return non-zero exit with error details.
- Remote URL transclusion fails unless the target host is allowed with
  `--allow host=HOST`. Denied hosts are rejected before any network request is
  attempted.

### Lifecycle Events

A root document that authors `initialize`, `composed`, `failed`,
`cancelled`, or `finalize` in its frontmatter composes as a lifecycle run.
`md compose` fires those five events around composition; transcluded
documents never fire theirs. See [Lifecycle Events](../topics/lifecycle/index.md)
for the grammar and [Lifecycle Error Handling](../topics/lifecycle/error-handling.md)
for failure routing.

```yaml
---
composed:
    - stdout: "Generated by md compose; do not edit."
    - shell: "git rev-parse --short HEAD"
finalize:
    - info: "composed {{ title }}"
---
```

- **Order.** `initialize`, then composition, then `composed`, then the
  document is written, then `finalize`. When composition or preflight fails,
  `failed` runs instead of `composed`, then `finalize`.
- **Output.** `stdout` actions and lifecycle command output go to stdout in
  run order, so output from `composed` precedes the document and output from
  `finalize` follows it. `stderr`, `info`, `warn`, `success`, and `message`
  write to stderr. Under `--output report-json` stdout holds only the report,
  so `stdout` actions and command output go to stderr too.
- **`skip`** in `initialize` writes no document, runs no `finalize`, and
  exits 0.
- **Other hosts' actions.** A verb `md` does not run (Claudine's `say`,
  `retry`, `break`, or an unknown verb) is skipped with one warning on stderr
  per occurrence reached, naming the verb and the event; the rest of the stack
  runs and the exit status is unaffected. A malformed or misplaced action
  that `md` does run is an error before anything runs.
- **Shell approval.** Lifecycle `shell` actions are audited with the body's
  commands, under the same blacklist, whitelist, approval, and timeout rules,
  before composition. They run only from `composed` on. `--shell` lists them
  without running any stack.
- **Errors.** An error in `initialize` or a preflight failure runs `failed`
  then `finalize`. An error in `composed` or a failed document write runs
  `finalize` only. Either way the exit is non-zero (2 for unresolved
  references, otherwise 1).
- **Ctrl-C** stops the run, runs `cancelled` (when authored) and `finalize`,
  and exits 130; a second Ctrl-C exits at once.

A lifecycle run does not print the `--perf` report or the verbose compose
summary. There is no dry-run mode; `--shell` is the only report-only mode.

### Remote URL Examples

```bash
# Fetch and transclude an allowed remote Markdown document
md compose doc.md --allow host=example.com

# Cache remote artifacts and serve stale content on revalidation failure
md compose doc.md \
  --allow host=example.com \
  --cache-root . \
  --remote-freshness fallback

# Force revalidation of cached remote artifacts
md compose doc.md \
  --allow host=example.com \
  --cache-root . \
  --remote-refresh
```

## Lessons Learned

- `compose` defaults to markdown because composed document output is the primary workflow.
- Frontmatter acts as transform configuration and is intentionally stripped unless `--fm` is used.
- Explicit `--state` is best for script-driven parameterization; it fills gaps without overwriting intent.
- JSON5 support for `--state` makes shell usage more ergonomic (no quoting keys).

## Issues

_(none known)_
