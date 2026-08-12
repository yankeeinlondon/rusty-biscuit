# Context Variables

Context variables are variables which Darkmatter provides to the **Interpolation** process as a key/value dictionary under the name of `ctx`.

## Overcoming `ctx` Conflicts

- Document authors are strongly discouraged from using the `ctx` frontmatter variable because it collides with Darkmatter's runtime context namespace
- However, when composing a document with `md compose`, if the document DOES have a `ctx` property defined then we will merge the two dictionaries; Darkmatter's runtime values take precedence over the page's when `ctx` keys overlap
- We will report to STDERR this event as a warning with a message of:

    - `Document defines ctx keys that collide with runtime context; runtime values take precedence` (when key collisions occur)
    - No warning when merge succeeds without collisions

- If there is a `ctx` property defined on the page that is _not_ a dictionary then we will:

    - By default return an error and stop composition
    - If the user uses the `--allow-ctx-override` CLI switch, downgrade to a warning and proceed with composition using the runtime context

## Timing in Compose

When composing a document graph, we calculate the context once and reuse it across the full graph of documents.

- This is more efficient
- It also ensures that we have the same date/time info throughout the composed document

However, context capture is also **demand-driven**: the document is scanned for `ctx.*` references and only the groups actually referenced are captured. If a document uses only `{{ ctx.today }}`, no git discovery, OS detection, or hardware probing occurs. Within a captured group, all properties in that group are computed; the laziness is at the group boundary, not per-property.

### Capture Groups

Variables are organized into capture groups. The expensive I/O for each group runs in parallel via `std::thread::scope`; property derivation from the captured data is negligible string formatting.

| Group | Expensive I/O | Properties |
|-------|--------------|------------|
| **DateTime** | `Local::now()` / `Utc::now()` syscalls (near-zero) | `now`, `now_utc`, `today`, `yesterday`, `tomorrow`, all `_utc` date variants, `day`, `day_abbr`, `day_utc`, `day_abbr_utc`, `year`, `year_utc`, `month`, `month_name`, `month_name_abbr`, `day_of_month`, `day_of_month_suffixed`, `time`, `time_military`, `time_utc`, `time_military_utc`, `timezone`, `timezone_offset`, `timezone_iana`, week boundaries, `season`, `timestamp`, `timestamp_ms` |
| **Git** | One `GitRepo::discover` plus branch, worktree, and index-stage reads | `branch`, `worktree`, `merge_conflicts` |
| **Repo** | `GitRepo::discover` + `detect_repo_structure` | `repo`, `repo_root`, `is_monorepo`, `package_root`, `package_area_root`, `packages`, `package_areas`, `current_package`, `current_package_area`, `area`, `area_description`, `area_root`, `current_packages`, `depends_on`, `used_by` |
| **FileChanges** | `GitRepo::file_changes()` | `dirty_files`, `dirty_source_code_files`, `staged_files`, `untracked_files`, `dirty_packages`, `dirty_package_areas`, `staged_packages`, `staged_package_areas`, `current_package_has_*`, `current_package_area_has_*` |
| **Languages** | Reads from already-captured repo info (no additional I/O) | `programming_languages_in_repo`, `programming_language`, `package_manager` |
| **Documents** | `detect_docs_with_packages` | `docs_readme`, `docs_blast_radius`, `docs_drift`, `docs_skill` |
| **OS** | `detect_os_with_request` | `os`, `os_distro`, `os_package_manager`, `os_version` |
| **Hardware** | `detect_hardware_summary` | `memory_total`, `memory_used`, `memory_avail`, `cpu_cores`, `cpu_arch` |
| **GPU** | `detect_gpus` (subprocess on macOS) | `gpu` |
| **Agent** | Reads `AGENT` and `MODEL` env vars | `agent`, `model` |


## Information Provided

The per-variable reference below is **generated** from the schema-derived
`ctx.*` catalog — the exact output of `md schema about --verbose`. It is
projected from the base frontmatter schema
(`darkmatter/docs/schemas/darkmatter.yaml`), so it can never drift from
validation. Do not hand-edit the block between the markers; a parity test
(`context_variables_doc_matches_generated_catalog`) regenerates it and its
failure message prints the up-to-date block to paste back.

> **Note:** all date and time related information is reported using _local_ time but there will be a `_utc` variant that provides the same utility only using UTC time to resolve.

> **List-valued variables.** Variables typed `string[]` (e.g. `packages`,
> `dirty_files`) or `object[]` (`depends_on`, `used_by`) are captured as real
> arrays. A bare `{{ ctx.foo }}` renders an array **line-separated** (one element
> per line). To render other shapes use the list-formatting expression functions:
> `as_csv`, `as_tsv`, `as_space_separated`, `as_line_separated`,
> `as_unordered_list`, and `as_ordered_list`. The Markdown-list renderers
> auto-nest nested arrays and the `depends_on` / `used_by` object shape. The
> former pre-rendered `_list` twin variables have been removed — replace
> `{{ ctx.dirty_files_list }}` with `{{ as_unordered_list(ctx.dirty_files) }}`.

> **Note:** the `ctx.*` Repository, File Changes, Languages, and Documents groups
> derive from the directory that _executed_ the `md compose` command (most discovery
> leverages the `sniff` library), **not** the directory where the composed document
> lives. File *reference* resolution is separate: implicit references resolve
> **repository-root first, then the source document's directory** — see
> [magic paths](./magic-paths.md).

<!-- BEGIN GENERATED: ctx catalog (source: md schema about / context_catalog_markdown) -->
**Date and Time**

- **ctx.now** — `datetime` — Local date and time in ISO-8601 format.
- **ctx.now\_utc** — `datetime` — UTC date and time in ISO-8601 format.
- **ctx.today** — `date` — Local date in ISO-8601 format.
- **ctx.today\_utc** — `date` — UTC date in ISO-8601 format.
- **ctx.yesterday** — `date` — Local date for yesterday in ISO-8601 format.
- **ctx.yesterday\_utc** — `date` — UTC date for yesterday in ISO-8601 format.
- **ctx.tomorrow** — `date` — Local date for tomorrow in ISO-8601 format.
- **ctx.tomorrow\_utc** — `date` — UTC date for tomorrow in ISO-8601 format.
- **ctx.day** — `string` — Full day of week name, local time.
- **ctx.day\_utc** — `string` — Full day of week name, UTC.
- **ctx.day\_abbr** — `string` — Abbreviated day of week name, local time.
- **ctx.day\_abbr\_utc** — `string` — Abbreviated day of week name, UTC.
- **ctx.year** — `string` — Current year, local time.
- **ctx.year\_utc** — `string` — Current year, UTC.
- **ctx.month** — `string` — Current month as a zero-padded number, local time.
- **ctx.month\_name** — `string` — Current month name, local time.
- **ctx.month\_name\_abbr** — `string` — Abbreviated current month name, local time.
- **ctx.day\_of\_month** — `string` — Current day of month.
- **ctx.day\_of\_month\_suffixed** — `string` — Current day of month with ordinal suffix.
- **ctx.time** — `string` — Local time in 12-hour format with AM/PM.
- **ctx.time\_military** — `string` — Local time in 24-hour format.
- **ctx.time\_utc** — `string` — UTC time in 12-hour format with AM/PM.
- **ctx.time\_military\_utc** — `string` — UTC time in 24-hour format.
- **ctx.timezone** — `string` _(optional)_ — Local timezone abbreviation, or null when unavailable.
- **ctx.timezone\_offset** — `string` — Local UTC offset.
- **ctx.timezone\_iana** — `string` _(optional)_ — Local IANA timezone name, or null when unavailable.
- **ctx.start\_of\_week\_sun** — `date` — Start of the current Sunday-based week, local time.
- **ctx.end\_of\_week\_sun** — `date` — End of the current Sunday-based week, local time.
- **ctx.start\_of\_week\_mon** — `date` — Start of the current Monday-based week, local time.
- **ctx.end\_of\_week\_mon** — `date` — End of the current Monday-based week, local time.
- **ctx.start\_of\_week\_sun\_utc** — `date` — Start of the current Sunday-based week, UTC.
- **ctx.end\_of\_week\_sun\_utc** — `date` — End of the current Sunday-based week, UTC.
- **ctx.start\_of\_week\_mon\_utc** — `date` — Start of the current Monday-based week, UTC.
- **ctx.end\_of\_week\_mon\_utc** — `date` — End of the current Monday-based week, UTC.
- **ctx.season** — `string` — Current meteorological season.
- **ctx.timestamp** — `number(integer)` — Current Unix timestamp in seconds.
- **ctx.timestamp\_ms** — `number(integer)` — Current Unix timestamp in milliseconds.
- _Aliases_
  - **ctx.utc** — `datetime` — Alias for now_utc.
  - **ctx.dow** — `string` — Alias for day.
  - **ctx.dow\_abbr** — `string` — Alias for day_abbr.

**Repository**

- **ctx.repo** — `string` _(optional)_ — Repository name from the preferred remote URL, or null when unavailable.
- **ctx.repo\_root** — `string` _(optional)_ — Absolute repository root path, or null when unavailable.
- _Git_
  - **ctx.branch** — `string` _(optional)_ — Current local Git branch name, or null outside a repository or at detached HEAD.
  - **ctx.worktree** — `string` _(optional)_ — Current linked Git worktree name, or null in the main worktree or outside a repository.
- **ctx.is\_monorepo** — `boolean` — Whether the current repository is a monorepo.
- _Packages_
  - **ctx.package\_root** — `string` _(optional)_ — Absolute current package root path, or null when unavailable.
  - **ctx.package\_area\_root** — `string` _(optional)_ — Absolute current package area root path, or null when unavailable.
  - **ctx.packages** — `string[]` _(optional)_ — Package names, or null when unavailable.
  - **ctx.package\_areas** — `string[]` _(optional)_ — Package area names, or null when unavailable.
  - **ctx.current\_package** — `string` _(optional)_ — Current package name, or null when unavailable.
  - **ctx.current\_package\_area** — `string` _(optional)_ — Current package area, or null when unavailable.
- _Scope_
  - **ctx.area** — `string` — Scoped area name.
  - **ctx.area\_description** — `string` — Human-readable scoped area description.
  - **ctx.area\_root** — `string` — Absolute scoped area root path.
  - **ctx.current\_packages** — `string[]` — Packages under the current directory.
  - **ctx.depends\_on** — `object[]` — Workspace-internal package dependencies. Each item is an object with a `package` (string) field and a `dependencies` (string[]) field listing the packages it depends on.
  - **ctx.used\_by** — `object[]` — Workspace-internal package reverse dependencies. Each item is an object with a `package` (string) field and a `users` (string[]) field listing the packages that depend on it.

**File Changes**

- **ctx.dirty\_files** — `string[]` — Dirty file paths.
- _Conflicts_
  - **ctx.merge\_conflicts** — `string[]` — Repository-relative paths currently in an unresolved Git index state.
- **ctx.dirty\_source\_code\_files** — `string[]` — Dirty source-code file paths.
- **ctx.staged\_files** — `string[]` — Staged file paths.
- **ctx.untracked\_files** — `string[]` — Untracked file paths.
- _Packages_
  - **ctx.dirty\_packages** — `string[]` — Dirty package names.
  - **ctx.dirty\_package\_areas** — `string[]` — Dirty package area names.
  - **ctx.staged\_packages** — `string[]` — Staged package names.
  - **ctx.staged\_package\_areas** — `string[]` — Staged package area names.
- _Flags_
  - **ctx.current\_package\_has\_staged\_files** — `boolean` — Whether the current package has staged files.
  - **ctx.current\_package\_area\_has\_staged\_files** — `boolean` — Whether the current package area has staged files.
  - **ctx.current\_package\_has\_dirty\_files** — `boolean` — Whether the current package has dirty files.
  - **ctx.current\_package\_area\_has\_dirty\_files** — `boolean` — Whether the current package area has dirty files.

**Languages**

- **ctx.programming\_languages\_in\_repo** — `string[]` _(optional)_ — Programming languages in the repository, or null when unavailable.
- **ctx.programming\_language** — `string` _(optional)_ — Context-sensitive primary programming language, or null when unavailable.
- **ctx.package\_manager** — `string` _(optional)_ — Context-sensitive package manager, or null when unavailable.

**Documents**

- **ctx.docs\_readme** — `string[]` — README paths, scope-filtered.
- **ctx.docs\_blast\_radius** — `string[]` — Docs with blast_radius frontmatter, scope-filtered.
- **ctx.docs\_drift** — `string[]` — Docs at risk of drift from source changes.
- **ctx.docs\_skill** — `string` _(optional)_ — Repo-relative path to the best matching skill file, or null when unavailable.

**Operating System**

- **ctx.os** — `string` _(optional)_ — Operating system name, or null when unavailable.
- **ctx.os\_distro** — `string` — Linux distribution name, empty on macOS and Windows.
- **ctx.os\_package\_manager** — `string` _(optional)_ — Primary system package manager, or null when unavailable.
- **ctx.os\_version** — `string` — Operating system version.

**Hardware**

- **ctx.memory\_total** — `string` _(optional)_ — Total system memory in bytes, or null when unavailable.
- **ctx.memory\_used** — `string` _(optional)_ — Percentage of memory currently used, or null when unavailable.
- **ctx.memory\_avail** — `string` _(optional)_ — Available system memory in bytes, or null when unavailable.
- **ctx.cpu\_cores** — `number(integer)` _(optional)_ — Number of logical CPU cores, or null when unavailable.
- **ctx.cpu\_arch** — `string` _(optional)_ — CPU architecture, or null when unavailable.
- **ctx.gpu** — `string` _(optional)_ — GPU device names, or null when unavailable.

**Agent**

- **ctx.agent** — `string` — The agentic CLI provider being used in the current session.
- **ctx.model** — `string` — Active model identifier, trimmed from the MODEL env var; defaults to "default".
<!-- END GENERATED: ctx catalog -->

## Notes on Specific Groups

The prose below adds constraints and derivation rules that the per-variable
types above cannot express. Variable names, types, and one-line descriptions
live only in the generated block.

### Languages

**`programming_language` rules:**

- Not in a repo: null
- In monorepo + in a package: that package's primary language
- In monorepo + in a package area: comma-separated unique primary languages across packages in that area
- Not in monorepo: repo's primary language

**`package_manager` rules:**

- Not in a repo: null
- In monorepo + in package: that package's package manager
- In monorepo + in package area: single answer if all packages agree, else null
- Not in monorepo: detected package manager

### Documents

**Scope filtering** (for monorepos):

- In a package: filter to that package
- In a package area: filter to packages in that area
- Otherwise: repo-wide

**`docs_drift` algorithm:** Intersects dirty source code files with markdown docs that have `blast_radius` metadata matching those files.

**`docs_skill` discovery:** Scans `{repo_root}/.claude/skills/*/SKILL.md` and `{repo_root}/.agents/skills/*/SKILL.md`, preferring skills whose directory name matches the current package, area, or repo name.

### Agent

The **Agent** group is captured when `ctx.agent` or `ctx.model` is referenced.
It performs no host probes: it reads the `AGENT` and `MODEL` environment
variables, trims ASCII whitespace, and applies simple defaults.

- Missing or empty values receive the defaults (`"unknown"` for `agent`, `"default"` for `model`).
- Values are trimmed before use.
- There is no model allowlist; any value is accepted as-is.
- The recognized agent names and aliases are: `claude` / `claude_code` /
  `claude-code`, `opencode` / `open_code` / `open-code`, and `codex`.
