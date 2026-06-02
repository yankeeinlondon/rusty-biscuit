# Context Variables

Context variables are variables which Darkmatter provides to the **Interpolation** process as a key/value dictionary under the name of `ctx`.

## Overcoming `ctx` Conflicts

- It is recommended that document authors not use the `ctx` frontmatter variable because of the namespace collision it causes
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
| **Repo** | `GitRepo::discover` + `detect_repo_structure` | `repo`, `repo_root`, `is_monorepo`, `package_root`, `package_area_root`, `packages`, `packages_list`, `package_areas`, `package_areas_list`, `current_package`, `current_package_area`, `area`, `area_description`, `area_root`, `current_packages`, `depends_on`, `used_by` |
| **FileChanges** | `GitRepo::file_changes()` | `dirty_files`, `dirty_files_list`, `dirty_source_code_files`, `dirty_source_code_files_list`, `staged_files`, `staged_files_list`, `untracked_files`, `untracked_files_list`, `dirty_packages`, `dirty_packages_list`, `dirty_package_areas`, `dirty_package_areas_list`, `staged_packages`, `staged_packages_list`, `staged_package_areas`, `staged_package_areas_list`, `current_package_has_*`, `current_package_area_has_*` |
| **Languages** | Reads from already-captured repo info (no additional I/O) | `programming_languages_in_repo`, `programming_language`, `package_manager` |
| **Documents** | `detect_docs_with_packages` | `docs_readme`, `docs_blast_radius`, `docs_drift`, `docs_skill` |
| **OS** | `detect_os_with_request` | `os`, `os_distro`, `os_package_manager`, `os_version` |
| **Hardware** | `detect_hardware_summary` | `memory_total`, `memory_used`, `memory_avail`, `cpu_cores`, `cpu_arch` |
| **GPU** | `detect_gpus` (subprocess on macOS) | `gpu` |


## Information Provided

We will now provide a grouped overview of all the information stored in Darkmatter's `ctx` variable:

> **Note:** all date and time related information is reported using _local_ time but there will be a `_utc` variant that provides the same utility only using UTC time to resolve.

### Date and Time Information

#### Date Only

| Variable                | Type     | Description                                 |
|-------------------------|----------|---------------------------------------------|
| `today`                 | `String` | ISO Date string (`YYYY-MM-DD`), local time  |
| `today_utc`             | `String` | ISO Date string (`YYYY-MM-DD`), UTC         |
| `yesterday`             | `String` | Yesterday's date (`YYYY-MM-DD`), local time |
| `yesterday_utc`         | `String` | Yesterday's date (`YYYY-MM-DD`), UTC        |
| `tomorrow`              | `String` | Tomorrow's date (`YYYY-MM-DD`), local time  |
| `tomorrow_utc`          | `String` | Tomorrow's date (`YYYY-MM-DD`), UTC         |
| `start_of_week_sun`     | `String` | Start of week (Sunday), `YYYY-MM-DD`        |
| `start_of_week_sun_utc` | `String` | Start of week (Sunday), UTC                 |
| `start_of_week_mon`     | `String` | Start of week (Monday), `YYYY-MM-DD`        |
| `start_of_week_mon_utc` | `String` | Start of week (Monday), UTC                 |
| `end_of_week_sun`       | `String` | End of week (Saturday), `YYYY-MM-DD`        |
| `end_of_week_sun_utc`   | `String` | End of week (Saturday), UTC                 |
| `end_of_week_mon`       | `String` | End of week (Sunday), `YYYY-MM-DD`          |
| `end_of_week_mon_utc`   | `String` | End of week (Sunday), UTC                   |

#### Date and Time

| Variable  | Type     | Description                                                |
|-----------|----------|------------------------------------------------------------|
| `now`     | `String` | ISO Datetime string for local time (`YYYY-MM-DDThh:mm:ss`) |
| `now_utc` | `String` | ISO Datetime string for UTC (`YYYY-MM-DDThh:mm:ssZ`)       |
| `utc`     | `String` | **Alias** for `now_utc` (backward compatibility)           |

#### Time Only

| Variable             | Type     | Description                                          |
|----------------------|----------|------------------------------------------------------|
| `time`               | `String` | Time in `hh:mm AM/PM` format (e.g., `12:43 PM`)       |
| `time_military`      | `String` | Time in 24-hour format (e.g., `22:30`)               |
| `time_utc`           | `String` | UTC time in `hh:mm AM/PM` format (e.g., `7:43 PM (UTC)`) |
| `time_military_utc`  | `String` | UTC time in 24-hour format (e.g., `19:43 (UTC)`)     |
| `timezone`           | `String` | Timezone abbreviation (e.g., `PDT`, `UTC`)           |
| `timezone_offset` | `String` | UTC offset (e.g., `-0700`)                      |
| `timezone_iana`   | `String` | UTC offset (e.g., `America/Los_Angeles`)                      |

#### Calendar

| Variable                | Type     | Description                                         |
|-------------------------|----------|-----------------------------------------------------|
| `day`                   | `String` | Day of the week (e.g., Monday, Tuesday)             |
| `dow`                   | `String` | **Alias** for `day` (backward compatibility)        |
| `day_abbr`              | `String` | Abbreviated day (e.g., Mon, Tue)                    |
| `dow_abbr`              | `String` | **Alias** for `day_abbr` (backward compatibility)   |
| `day_utc`               | `String` | Day of the week, UTC                                |
| `day_abbr_utc`          | `String` | Abbreviated day, UTC                                |
| `year`                  | `String` | Four-digit year, local time                         |
| `year_utc`              | `String` | Four-digit year, UTC                                |
| `day_of_month`          | `String` | Numeric day of month                                |
| `day_of_month_suffixed` | `String` | Day with ordinal suffix (1st, 2nd, 3rd, etc.)       |
| `month`                 | `String` | Two-digit month (01-12)                             |
| `month_name`            | `String` | Full month name (e.g., January)                     |
| `month_name_abbr`       | `String` | Abbreviated month name (e.g., Jan)                  |
| `season`                | `String` | Meteorological season: Spring, Summer, Fall, Winter |

#### Timestamps

| Variable       | Type     | Description                     |
|----------------|----------|---------------------------------|
| `timestamp`    | `Number` | EPOCH timestamp in seconds      |
| `timestamp_ms` | `Number` | EPOCH timestamp in milliseconds |

### Filesystem and Git

> **Note:** the CWD in all file/git operations is the directory which _executed_ the `md compose` command **not** the directory where the composed document lives
> 
> **Note:** most discovery in this section leverages the `sniff` library

| Variable               | Type              | Description                                                                        |
|------------------------|-------------------|------------------------------------------------------------------------------------|
| `repo`                 | `String \| null`   | Repository name; null if not in a git repo                                         |
| `repo_root`            | `String \| null`   | Absolute path to repo root (no trailing separator); null if not in a git repo      |
| `is_monorepo`          | `bool`            | Whether the repo is a monorepo; false if not in a repo                             |
| `package_root`         | `String \| null`   | Absolute path to current package root; null if not monorepo or not in a package    |
| `package_area_root`    | `String \| null`   | Absolute path to current package area root; null if not monorepo or not in an area |
| `packages`             | `[String] \| null` | List of package names; null if not a monorepo                                      |
| `package_areas`        | `[String] \| null` | List of unique package areas; null if not a monorepo                               |
| `current_package`      | `String \| null`   | Current package name; null if not in a monorepo package                            |
| `current_package_area` | `String \| null`   | Current package area; null if not in a monorepo area                               |
| `area`                 | `String`          | Scope name: package name in a package, area name in an area; empty string at root or when not a monorepo |
| `area_description`     | `String`          | `"{package} package"` in a package, `"{area} package area"` in an area; empty string at root or when not a monorepo |
| `area_root`            | `String`          | Absolute path to the `area` root (no trailing separator); repo root when not a monorepo |
| `current_packages`     | `String`          | Markdown bullet list (`- {name} ({relative})`) of packages under the current directory; empty string outside a monorepo |
| `depends_on`           | `String`          | Nested Markdown list of workspace-internal packages the scoped `area` depends on; empty string outside a monorepo |
| `used_by`              | `String`          | Nested Markdown list of workspace-internal packages that depend on the scoped `area`; empty string outside a monorepo |

#### Changed Files

| Variable                       | Type     | Description                                      |
|--------------------------------|----------|--------------------------------------------------|
| `dirty_files`                  | `String` | Comma-separated dirty file paths (empty if none) |
| `dirty_files_list`             | `String` | Markdown bullet list of dirty files              |
| `dirty_source_code_files`      | `String` | Comma-separated dirty source code file paths     |
| `dirty_source_code_files_list` | `String` | Markdown bullet list of dirty source code files  |
| `staged_files`                 | `String` | Comma-separated staged file paths                |
| `staged_files_list`            | `String` | Markdown bullet list of staged files             |
| `untracked_files`              | `String` | Comma-separated untracked file paths             |
| `untracked_files_list`         | `String` | Markdown bullet list of untracked files          |

#### Package-Level Changes

| Variable                                | Type     | Description                                   |
|-----------------------------------------|----------|-----------------------------------------------|
| `dirty_packages`                        | `String` | Comma-separated dirty package names           |
| `dirty_packages_list`                   | `String` | Markdown bullet list of dirty packages        |
| `dirty_package_areas`                   | `String` | Comma-separated dirty package area names      |
| `dirty_package_areas_list`              | `String` | Markdown bullet list of dirty package areas   |
| `staged_packages`                       | `String` | Comma-separated staged package names          |
| `staged_packages_list`                  | `String` | Markdown bullet list of staged packages       |
| `staged_package_areas`                  | `String` | Comma-separated staged package area names     |
| `staged_package_areas_list`             | `String` | Markdown bullet list of staged package areas  |
| `current_package_has_staged_files`      | `bool`   | Whether current package has staged files      |
| `current_package_area_has_staged_files` | `bool`   | Whether current package area has staged files |
| `current_package_has_dirty_files`       | `bool`   | Whether current package has dirty files       |
| `current_package_area_has_dirty_files`  | `bool`   | Whether current package area has dirty files  |

### Programming Language

| Variable                        | Type            | Description                                                                 |
|---------------------------------|-----------------|-----------------------------------------------------------------------------|
| `programming_languages_in_repo` | `String \| null` | Comma-separated unique languages across all packages; null if not in a repo |
| `programming_language`          | `String \| null` | Context-sensitive primary language (see rules below); null if not in a repo |
| `package_manager`               | `String \| null` | Context-sensitive package manager (see rules below); null if not in a repo  |

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

| Variable            | Type            | Description                                                          |
|---------------------|-----------------|----------------------------------------------------------------------|
| `docs_readme`       | `String`        | Comma-separated README paths, scope-filtered                         |
| `docs_blast_radius` | `String`        | Comma-separated docs with `blast_radius` frontmatter, scope-filtered |
| `docs_drift`        | `String`        | Comma-separated docs at risk of drift from source changes            |
| `docs_skill`        | `String \| null` | Repo-relative path to best matching SKILL.md; null if none found     |

**Scope filtering** (for monorepos):

- In a package: filter to that package
- In a package area: filter to packages in that area
- Otherwise: repo-wide

**`docs_drift` algorithm:** Intersects dirty source code files with markdown docs that have `blast_radius` metadata matching those files.

**`docs_skill` discovery:** Scans `{repo_root}/.claude/skills/*/SKILL.md` and `{repo_root}/.agents/skills/*/SKILL.md`, preferring skills whose directory name matches the current package, area, or repo name.

### Operating System

| Variable             | Type            | Description                                                   |
|----------------------|-----------------|---------------------------------------------------------------|
| `os`                 | `String \| null` | `"Windows"`, `"macOS"`, or `"Linux"`; null for other OS types |
| `os_distro`          | `String`        | Linux distribution name; empty string on macOS/Windows        |
| `os_package_manager` | `String \| null` | Primary system package manager; null if not detected          |
| `os_version`         | `String`        | Operating system version                                      |

### Hardware

| Variable       | Type            | Description                                                |
|----------------|-----------------|------------------------------------------------------------|
| `memory_total` | `Number`        | Total system memory in bytes                               |
| `memory_used`  | `Number`        | Percentage of memory currently used                        |
| `memory_avail` | `Number`        | Available memory in bytes                                  |
| `cpu_cores`    | `Number`        | Number of logical CPU cores                                |
| `cpu_arch`     | `String`        | CPU architecture (e.g., `aarch64`, `x86_64`)               |
| `gpu`          | `String \| null` | GPU device name(s), comma-separated; null if none detected |
