# More Context Variables

A really strong feature in Darkmatter is it's ability to provide "context variables" and make them accessible to
interpolation through the `ctx` frontmatter property.

- read [context variables](@darkmatter/docs/topics/context-variables.md) for context on what we provide today

In this feature we will:

- add to this set of available context variables, _and_ 
- add additional functions to the expression engine
- and finally we'll also add the initial set of "side effects"

> - Context variables are the properties which hang off of the `ctx` object 
> - inside of a interpolation you can also use the functions and operators provided by the "expression engine" 
>     - the expression engine provides functions and operators which allows the author to mutate state ONLY on the page it resides on
>     - while functions in the expression engine provides way to "read" information from the external environment, it is strictly forbidden for it to _mutate_ any external entities
> - "side effects" are the set of functions which Darkmatter provides which **are** allowed to cause mutations in the external environment
>     - things like setting other files' Frontmatter props, etc. are chosen because they are "relatively safe"
>     - operations which have greater potential to cause harm to the external environment are expressly forbidden
>     - the goal of the side effects, is to provide a set of helpful and ergonomic ways to get information from the environment as well as to mutate them as necessary
>     - side effects can be seen as peers of the expression engine but they contain functions which DO mutate external state
 
In all cases it's critical that we keep an eye on performance. The current set of context variables uses a performant and lazily loaded approach to sourcing the context information that works well but as we add more information we should always take the time to consider the most performant way of achieving the functional goal and then make sure our benchmark testing is able to detect regressions.

## New Context Variables

### Git / Repo

The following context variables are only available in a monorepo:

- `area`
    - returns an empty string when not a monorepo
    - when in a monorepo and in a "package area" but NOT in a "package" folder then we return the "package area"
    - when in a "package" folder then return the package name
    - when in the root folder of a monorepo return an empty string

- `area_description`
    - returns an empty string when not in a monorepo
    - when in a monorepo and in a "package area" but NOT in a "package" folder then we return `{package-area} package area`
    - when in a "package" folder then return `{package} package`
    - when in the root folder of a monorepo return an empty string

- `area_root`
    - returns repo root if not a monorepo
    - returns the fully qualified absolute path to the root of the `area`
    - no trailing `/` character

- `current_packages` - _lists all packages in a monorepo which reside under the current directory_
    - format of the output should mimic:

        ```sh
        💻❯ sniff repo packages --package-area claudine --md -v
        - claudine-cli ({relative-filepath})
        - claudine ({relative-filepath})
        ```

        aka, a Markdown unordered list which shows the relative path (from repo root) to the package area

- `depends_on` - _provides a list of packages in the monorepo that the current {scope} depends on_
    - where `scope` is 
        - the current **area** as defined above by area
        - style:
            - UnorderedList -> `'{package}' depends on:`
                - UnorderedList -> `{dep-package}`
            - if no dependencies then: `'{package}' has no dependencies on other packages in this monorepo`
- `used_by` 
    - provides the same scoping rules of `depends_on` but instead nesting dependencies we list "dependent" packages which rely on the scoped area.
    - top level list are the packages in the scoped area
        - `'{package}' is used by:`
        - or `'{package}' is not used by other packages in this monorepo`
    - nested level is the dependent packages

### Date and Time -> Time Only

- `time_utc` - same as `time` except the time is in UTC and we append ` (UTC)` to the end of the string
- `time_military_utc` - same as `time_military` but the time is in UTC and we append ` (UTC)` to the end of the string

## Small Changes to Existing Context Variables

- `repo_root` is returning the correct directory but it's terminating with a `/` character which problematic. Do not add the closing `/`. This same change was already made to `sniff repo root` CLI so these will be consistent.


## Context Expression Functions

We currently already have a decent set of _operators_ and _utility functions_ which are part of what we're calling the Expression Engine. This is documented in @darkmatter/docs/topics/darkmatter-expressions.md

In this feature we are going to add to the available functions offered. Before we go through the additions let's remind ourselves of some key constraints we have with Expression Functions:

- expected function naming should follow the "snake_case" naming convention
    - all documentation MUST use snake_case, however
    - the expression engine will check for PascalCase too and use it as a fallback
    - this is a convenience and we don't want to overly promote this externally
- all functions are involved in _retrieving_ information and are NOT allowed to mutate any external state (e.g., external to the document they are defined on)
- 


### File System

- `absolute(file) -> file | Error::InvalidFilePath` - take a string file path and using the `FileReference` rules resolves it to a fully qualified file path
- `relative(file) -> file | Error::InvalidFilePath` - takes a string file reference and using the `FileReference` rules, and returns a relative path:
    - if in a repo:
        - relative to the repo's root when filepath is contained within the repo
    - if not a repo:
        - relative the CWD when the filepath is contained within the CWD base directory
    - if the filepath is NOT in path we will try to use an alias:
        - `~` if it's off of the user's home directory
        - ENV variables which are a valid filepath can also be used as an alias
    - NOTE: this relative path logic should already exist in Darkmatter as part of the composition pipeline, see if that can be reused.
- `file_exists(file) -> bool`
    - validates that the passed in filepath exists in the file system
    - if the file passed in is an invalid file path it returns `false`
- `frontmatter(file) -> object | Error::InvalidFilePath`
- `frontmatter(file, prop) -> any | Error::InvalidFilePath`
    - two signatures:
        - when no property is provided then the full frontmatter key/value object is returned
        - when a property is provided then the value of that particular property is returned
    - if the filepath is invalid then an error is raised 
    - to protect against this type of error you can: `{{ file_exists(filepath) ? frontmatter(filepath, prop) : "oops" }}`
    - if the `prop` is provided but the referenced property does not exist in frontmatter we return null
- `markdown_body_empty(filepath) -> bool | Error::InvalidFilePath`
    - checks if the Markdown file referenced is "empty" where "empty" means that the body has no characters other than potentially whitespace
    - the Markdown document _can_ have frontmatter and still be considered "empty" as the only criteria is based exclusively on the body content of the document
- `markdown_title(filepath) -> string | null | Error::InvalidFilePath`
    - results in an error if invalid filepath
    - if valid filepath then 
        - it returns the `title` Frontmatter of the document if it exists
        - if `title` is not present it will look for H1 Heading(s)
            - if single H1 Heading is found then the text for that heading is returned as the title
            - if more than one H1 Heading, the first is returned as a title and a warning message is sent to STDERR 
- `validate_schema(filepath) -> bool | Error::InvalidFilePath`
- `validate_schema(filepath, obj) -> bool | Error::InvalidFilePath`
    - can validate the schema of a Markdown or YAML document
    - if the `$schema` property is not set then this will always return **true**
    - otherwise the schema will be used to validate the frontmatter and returns the boolean outcome
    - review the [schema documentation](@darkmatter/docs/topics/schema-definition.md) implemented in Darkmatter and exposed in Claudine for more on schema support

> NOTE: for all filepath parameters, there should be a small "normalization" step which ensure that filepaths like `foo//bar` are converted to `foo/bar` and references like `file://foo/bar` have the `file://` prefix removed.


### Date/Time Functions

- `date(iso, format)` - allows you to reformat a valid ISO Date or ISO DateTime into a formatted date:
    - `MMMM Do` / `short`:
        - `July 12th`
    - `MMMM Do [YYYY]` / `short-optional`:
        - `July 12th` (for dates in current year)
        - `July 12th 1999` (for dates in a different year)
        - note: `[YYYY]` is a Darkmatter-specific extension meaning "include year only when it differs from the current year"
    - `MMMM Do YYYY`:
        - `July 12th 2026`
    - `D MMMM [YYYY]`:
        - `12 July` (for dates in current year)
        - `12 July 1999` (for dates in a different year)
        - note: same `[YYYY]` optional-year extension as above
    - `D MMMM YYYY`:
        - `12 July 2021`
    - `ddd, MMMM Do, YYYY` / `long`:
        - `Mon, July 12th, 2021`


## Side Effects

Side Effects, unlike the Expression Engine utility functions, **do** mutate external state. They are deliberately scoped to relatively safe operations and are governed by two hard safety boundaries that Darkmatter enforces and the host (e.g. Claudine) configures.

### Ownership & Triggering Model

- The side-effect **engine** is owned by Darkmatter and lives in a dedicated `darkmatter::effects` module — deliberately **not** part of the compose pipeline.
- Composing a document (`md compose`) is **pure**: it never invokes a side effect. This preserves the guarantee that reading or composing a document cannot mutate the filesystem.
- Side effects are a named, callable **action catalog**. Only an external orchestrator — the lifecycle stack in [Claudine](@claudine/features/2026-05-12-lifecycle/spec.md) — invokes them, at well-defined lifecycle moments.
- This mirrors the Expression Engine relationship: Darkmatter owns the engine; the host drives it.

### The Read/Write Split

A side-effect call (`set_frontmatter(@spec.md, "status", upper(env.AGENT))`) is parsed by the **same lexer/parser** as expression functions — it is the same function-call AST. The mutation/read split is enforced at **evaluation**, not syntax:

- **Arguments** to a side effect are evaluated by the existing **read-only** expression evaluator. Expression functions (`upper`, `frontmatter`, …) and `{{ }}` interpolation therefore work inside side-effect arguments for free.
- **Only the outer dispatch** mutates. Read functions can never mutate (they route through `evaluate`); side effects can never run during compose (compose never references `EffectEngine`).

This means zero duplicated parsing, and "reads cannot write" becomes an evaluator-level invariant rather than a naming convention.

### Safety Boundaries

Two boundaries, both enforced inside Darkmatter, both configured by the host:

1. **Mutation root (filesystem sandbox)** — the engine is constructed with a single mutation root. Any filesystem write resolving outside that root is hard-refused with `EffectError::OutsideMutationRoot`. Claudine passes the repo root (or the CWD when not in a repo).
2. **Host allowlist (network sandbox)** — network effects are refused unless their target host is on an allowlist. The allowlist defaults to **deny-all**: no network effect runs unless the host explicitly grants the host. Violations raise `EffectError::HostNotAllowed`.

```rust
EffectEngine::builder()
    .mutation_root(repo_root)   // filesystem writes refused outside this
    .allowed_hosts([])          // network: deny-all by default
    .auto_rehash(true)          // re-hash mutated Markdown (default on)
    .build()
```

> **Forward compatibility — network safety generalizes.** The host allowlist is
> introduced here to gate side-effect network writes (`http_post`). As Darkmatter
> gains URL-based referencing, read-side functions and transclusion will also be
> able to resolve remote URLs, which means **composition itself performs network
> egress** — the same SSRF / privacy surface, even though it is not a mutation.
> The allowlist is therefore designed to graduate from a side-effect-engine knob
> into a **Darkmatter-wide network policy** that gates both compose-time URL
> reads and side-effect network writes (still Darkmatter-enforced,
> host-configured). No change to the `allowed_hosts` shape is required for this
> evolution.

### Filepath Normalization

As with expression filepath parameters, side-effect paths pass through `FileReference` normalization: `foo//bar` → `foo/bar`, and a leading `file://` prefix is stripped.

### Auto Re-hash

When a side effect mutates a Markdown file that carries a `hash:` frontmatter property, the engine recomputes the hash using Darkmatter's Markdown-aware hasher so the file stays self-consistent. This is **on by default** and can be disabled per call. Only Darkmatter can do this correctly, which is part of why the engine lives here.

### v1 Catalog

All verbs accept a `FileReference`-normalized path and return a value (so steps can chain). Naming is **snake_case** canonical with a PascalCase fallback, identical to the expression-function convention.

#### Frontmatter Mutations

- `set_frontmatter(file, prop, value) -> any | Error::InvalidFilePath` — set/overwrite a property; returns the prior value (or null).
- `merge_frontmatter(file, obj) -> object | Error::InvalidFilePath` — shallow-merge an object into frontmatter; returns the merged object.
- `delete_frontmatter(file, prop) -> any | Error::InvalidFilePath` — remove a property; returns the removed value (or null).
- `increment_frontmatter(file, prop) -> number` / `decrement_frontmatter(file, prop) -> number` — numeric mutation (mirrors the loop iteration DSL).
- `append_frontmatter(file, prop, value) -> array` / `prepend_frontmatter(file, prop, value) -> array` — array-property mutation.

All frontmatter writers honor auto-rehash.

#### File & Directory

- `ensure_file(file) -> file` / `ensure_file(file, content) -> file` — idempotent create; returns the absolute path. An existing file is left unchanged; `content` is written only when creating a missing file.
- `ensure_dir(dir) -> dir` — `mkdir -p`; returns the absolute path.
- `append_line(file, text) -> file` — append a line.
- `append_jsonl(file, obj) -> file` — append a JSON object as a JSONL record.

#### Network

- `http_post(url, body) -> object | Error::HostNotAllowed` — POST a body; returns response status/body. Gated by the host allowlist (deny-all by default).

**Expressly forbidden in v1:** full-file overwrite (`write_file`) and deletion (`delete_file`). These exceed the "relatively safe" bar.

### Host Integration Surface

Darkmatter exposes two convergent entry points so a host can use either a short string form or a structured form:

- `engine.execute(call_str, &ctx) -> Result<Value, EffectError>` — parses and dispatches a call string (e.g. `"ensure_file(@out/log.md)"`).
- Typed methods (`engine.set_frontmatter(file, prop, value)`, …) — for hosts that already have structured parameters (e.g. Claudine's long-form `action:` mappings).

Both funnel into the same typed implementations. `EffectError` implements `biscuit_terminal::errors::BlockError` so failures render as rich terminal status blocks, exactly like `ConditionError`.

### Out of Scope (owned by the host)

- *When* effects fire (lifecycle events / the stack).
- *Policy*: which mutation root, which hosts are allowed, command/whitelist pre-flight.
- Error-propagation semantics per lifecycle event and the per-action `no_error` escape hatch.

These are defined in the [Claudine lifecycle spec](@claudine/features/2026-05-12-lifecycle/spec.md).
