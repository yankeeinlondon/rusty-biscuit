---
kind: canonical-documentation
$schema: 
    related: 
        - file(required)
        - file[](required)
related: "@sniff/docs/cli/repo_recent-commits.md" 
---

# Recent Commits

Sniff can provide useful reports on the _recent commits_ in a repo. The main dimensions which you will control are:

## 1. Report Format

- `json` - pure data format with all information in the payload
- `prose` - text format using the [Prose grammar](../../../../biscuit-terminal/docs/components/prose.md); this formatting is largely Markdown grammar but with a few extra features, such as color tags, that allow colorization
- `terminal` - the Prose report rendered to escape codes (and OSC8 hyperlinks) by the `Prose` component in **biscuit-terminal**
- `markdown` - the same report as portable Markdown: links and bold/italic emphasis, no color
- `plain` - bare text with no links, markup, or escape sequences

## 2. Scoping: Which Commits

There are two primary ways to specify _which commits_ will be reported:

- **Raw Count**
    - the simplest approach is to list a specified number of commits
    - the default -- when no scoping is provided -- is the **10** most recent commits
- **Time Window**
    - often we want to reflect on the commits of the last day, week, etc.
    - the commits can be just as easily determined by providing a duration (from now), a named day (`today`, `yesterday`), or an absolute date
- **Hash**
    - every commit from the starting tip back to, and including, a given commit

History is walked from `HEAD` unless a **branch** is given. A branch is a starting point for the walk, not a filter: history is walked from that branch's tip instead of `HEAD`. A local branch wins over a remote-tracking branch of the same name, and the remote-tracking branch is the fallback. Nothing is fetched.

### Calendar Days

Calendar scopes are computed in the options' timezone, which defaults to the host's current local UTC offset. Selecting a day and labeling a commit's day therefore always agree.

| Scope | Commits selected |
|-------|------------------|
| `today` | From local midnight until now |
| `yesterday` | That single local day only (local midnight to local midnight) |
| `2026-04-01` | That single local calendar day |

Durations, counts, and hashes do not depend on the timezone.

### Filters

In addition to these mechanisms for scoping which commits are reported, you can also _filter_ the commit list by a number of criteria:

- **blast-radius**
    - the `package` and `package-area` (for monorepos) narrow the blast radius you are interested in
    - `scope` offers similar -- though mildly less reliable -- blast-radius limiting for conventional commits
- **file-type**
    - a variant of _blast radius_ filtering: keep commits that changed at least one file of a given category
        - source code
        - web assets (HTML, CSS, font files)
        - images (raster and vector, including SVG)
        - documentation
        - configuration
        - CI/CD
- **operation**
    - if you're using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) then filtering by the `operation` (any word, such as `fix` or `planning`) can be useful for certain tasks
    - several operations may be listed; a commit matching any of them is kept, and non-conventional commits are excluded
- **author**
    - a case-insensitive substring of the author's name **or** email

Filters of different kinds combine with a logical AND. Filtering happens during the history walk, so a count selection keeps walking until it has collected that many _matching_ commits or history runs out.

### File Categories

Every changed path belongs to exactly one category, decided by `sniff::filesystem::path_kind::classify_path` from the path alone. Precedence, first match wins:

1. **CI/CD path rules**, such as `.github/workflows/**`, `.gitlab-ci.yml`, `Jenkinsfile`, and `.circleci/**`
2. **The file-type registry**: programming languages and framework files are `source_code`; stylesheets, fonts, and `.html`/`.htm` are `web_assets`; images (including SVG) are `images`; documentation and configuration map to themselves
3. anything else is `other`, which sets no flag

> **Behavior change:** `.html` files used to count as documentation (or source code) and CSS used to count as source code. Both are now `web_assets`. An Angular `.component.html` template is a registry framework file and stays `source_code`.

### Package Attribution

In a monorepo, each commit is attributed to the packages and package areas that own its changed files. Attribution uses the manifest-only _structure_ catalog (the same one `sniff repo packages` uses), and the `package`/`package-area` filters use the same ownership index.

- a file belongs to its **deepest** owning package only, so a nested package does not also attribute its parent
- a file inside a package area but outside every package (for example `sniff/README.md`) is **unattributed**; there is no fallback to the area directory
- the original path of a moved file counts too
- outside a monorepo there is no attribution, and a package or package-area filter is an error

## 3. Verbosity

There are three report levels:

1. Compact
2. Normal
3. Verbose

> **Note:** verbosity, and whether the author is shown, are irrelevant to the JSON output, which always reports all available data.

### Normal

Let's start with how a "normal" report looks:

```txt
- [{hash}] {operation}({scope}) at {time} {day}: {heading}
  **Files Impacted:**
  - {kind}: {filepath}
  - moved: {filepath} (from {original-path})
```

The normal verbosity level provides basic metadata, the _heading_, and a summary of the files impacted, but leaves off the remaining commentary and bullet points describing the commit. Commits are separated by one blank line.

- the `{hash}` is the 7-character short hash
    - it links to the commit's page on the remote host when linking established a URL (see [Links](#links))
- `{operation}({scope})` appears only for conventional commits, and `({scope})` only when the commit has a scope
- `{time} {day}` looks like `1:01pm Today`, `9:30am Yesterday`, or `9:30am 2026-04-01`, judged in the options' timezone
- the `{heading}` is the first sentence of the commit message: the subject line (after any conventional-commit prefix) up to its first `.`
- enabling _show author_ inserts `by {author}` before `at`; the author is the name, or the email when the name is empty
- a commit with no changed files, such as a no-change merge, renders `**Files Impacted:** none`

### Compact

The compact variant shows only the header line, with no blank lines between commits:

```txt
- [{hash}] {operation}({scope}) at {time} {day}: {heading}
```

### Verbose

The verbose style looks similar to normal but it adds the commit's commentary (prose and bullet points):

```txt
- [{hash}] {operation}({scope}) at {time} {day}: {heading}
  {description}

  **Details:**

  - {bullet-point}

  **Files Impacted:**
  - {kind}: {filepath}
```

The description line appears only when the description is non-empty, and the `Details` block only when there are bullet points. When the commit has neither, verbose output is identical to normal output.

### Styling

The layout above is written once and folded into each text format. Prose output uses these [Prose tags](../../../../biscuit-terminal/docs/components/prose.md#supported-tags); Markdown keeps only the emphasis and links; plain output keeps only the text.

| Part of the report | Prose | Markdown |
|--------------------|-------|----------|
| Short hash (brackets stay unstyled) | `<bold>`, linked to `commit_url` when present | `**`, linked the same way |
| Operation and the parentheses around the scope | `<blue>` | unstyled |
| Scope | `<blue><dim>` | unstyled |
| The word `at` | `<italic>` | `_` |
| Time and day | `<bold>` | `**` |
| `Files Impacted:` and `Details:` labels | `<bold>` | `**` |
| Changed file paths | `[path](file:///…)` link | the same link |
| Sibling heading (see [Projections](#projections)) | `<bold>` line | `##` heading |

- links are written as `[text](url)`; the terminal `Prose` component turns them into OSC8 hyperlinks, or keeps the Markdown form when the terminal lacks OSC8 support
- file links are built from the repository root, so Windows paths become `file:///C:/…`
- **deleted files are never linked**, because the target is gone; a collection that was deserialized from JSON has no repository root and renders every file unlinked
- dynamic text is backslash-escaped for Prose and Markdown (`\`, `*`, `_`, `[`, `]`, `<`, `>`, and in Markdown also `` ` ``), so a heading such as `handle <red>tags</red>` renders literally; link targets percent-encode `(`, `)`, space, `<`, and `>`

> **Behavior change:** plain output strips everything. The previous `--plain` output kept Markdown bold markers such as `**Description:**`; plain reports now contain no markup at all.

### Projections

A _projection_ keeps only the files of one category and drops commits left with no files. Sniff offers two, which back the `source-code-changes` and `documentation-changes` commands:

- `SourceCode` keeps `source_code` files and adds a `Source Code Changes` heading
- `Documentation` keeps `documentation` files and adds a `Documentation Changes` heading

A projection prunes an already-collected selection; it is not a filter. The last 10 commits projected onto source code can therefore show fewer than 10 commits. To _select_ commits that changed source code, use the file-type filter instead.

- a moved file is kept when either its path or its original path is in the category
- commit-level facts (`file_types`, `packages`, `package_areas`, `remote`, `commit_url`) still describe the whole commit
- the identity projection (`All`) keeps every commit, including no-change merges
- the heading is written only when at least one commit remains

## Payload Schema

Each commit's payload is defined as a schema here: [Recent Commits Schema](./recent-commits-schema.md)

## Library Callers

The `sniff` library is the primary interface for collecting recent-commit reports. Rust callers can select commits, apply filters, and obtain the payload described above without invoking the CLI or initializing a terminal. The CLI is a thin layer over the same library contract.

As a simple example, we can collect the last 10 commits and report on them in a variety of ways:

```rust,ignore
use std::path::Path;
use sniff::filesystem::git::{
    GitRepo, RecentCommits, RecentCommitsOptions, RecentCommitsVerbosity,
};

// connect to a git repo
let repo = GitRepo::discover(Path::new("."))?.expect("inside a repository");
// define the reporting options you want
let options = RecentCommitsOptions::new().verbosity(RecentCommitsVerbosity::Verbose);
// collect the last 10 commits
let commits = RecentCommits::collect(&repo, &options)?;
// serialize the commits to a bare JSON array
let json: serde_json::Value = commits.to_json();
// and then use the options to report in different text styles
let prose: String = commits.to_prose(&options);
let plain: String = commits.to_plain(&options);
```

The `RecentCommits` struct is a collection of commits, newest first. We can immediately serialize these commits to JSON with `.to_json()`, and we can also produce string _reports_ with varying verbosity and style characteristics.

### Selection and Filters

In the first example we used `RecentCommitsOptions` to set the reporting verbosity, but options also choose what the collection actually contains:

```rust,ignore
use chrono::Duration;

let options = RecentCommitsOptions::new()
    .duration(Duration::days(5))
    .operation("fix")
    .verbosity(RecentCommitsVerbosity::Compact);
let commits = RecentCommits::collect(&repo, &options)?;
```

Instead of the last 10 commits, this collection holds every commit from the last 5 days that is a conventional commit using the `fix` operation.

`RecentCommitsOptions` is an ordinary (non-generic) builder with a single `selection` that defaults to `Selection::Count(10)`. The selector methods all set that one field, so **the last call wins**:

- `count(count: usize)` - zero is rejected when collecting
- `duration(duration: chrono::Duration)`
- `named_date(name: NamedDate)` - `NamedDate::Today` or `NamedDate::Yesterday`
- `date(date: chrono::NaiveDate)` - exactly one local calendar day
- `hash(hash: impl Into<String>)` - from the tip back to and including this commit
- `selection(selection: Selection)` - an already-built selection

`Selection::parse` (also available through `FromStr`) turns a CLI-style scope string into a `Selection`. Its precedence is `today`/`yesterday` (case-insensitive), then `YYYY-MM-DD`, then a positive all-digit count, then a duration, then a hexadecimal hash of at least 7 characters. An all-digit input is always a count, never a hash. A zero count, or input matching no form, is `SniffError::InvalidPeriod`.

The filters described in this document are also options:

- `package(pkg)`, `package_area(area)`
- `operation(op)` - repeated calls accumulate and match _any_ listed operation (case-insensitive)
- `scope(scope)` - case-insensitive
- `author(author)` - case-insensitive substring of the name or email
- `has_file_type(category: ChangeCategory)` - repeated calls require _every_ listed category
- `branch(branch)` - the history base, as described in [Scoping](#2-scoping-which-commits)

Other single-valued filters keep their last value. Filters of different kinds combine with a logical AND.

The remaining options shape reports rather than selection:

- `verbosity(RecentCommitsVerbosity)` - `Compact`, `Normal` (the default), or `Verbose`
- `show_author(bool)` - adds the author to each header line
- `projection(RecentCommitsProjection)` - `All` (the default), `SourceCode`, or `Documentation`; text reports apply it, while `to_json()` does not (use `projected`, below)
- `timezone(chrono::FixedOffset)` - the offset for calendar selections and day labels, defaulting to the host's current local offset

> **Note:** the timezone is a fixed UTC offset. There is no `chrono-tz` dependency, so named zones that observe daylight saving time cannot be expressed, and a calendar day that spans a DST transition is bounded by a single offset. JSON `datetime` values stay UTC regardless.

### Library Outputs

`RecentCommits` provides both raw serialization of its data to JSON and a few text reports:

- `to_json() -> serde_json::Value` - a bare JSON array of commit objects; verbosity and `show_author` never affect it
- `to_prose(&options) -> String` - the **Prose** report: Markdown-like content plus color tags such as `<blue>fix</blue>`, and links
- `to_markdown(&options) -> String` - the same report as valid Markdown, with color dropped
- `to_plain(&options) -> String` - bare text with no escape sequences, markup, or links
- `projected(RecentCommitsProjection) -> RecentCommits` - a pruned copy (see [Projections](#projections)); this is how JSON output gets a projection

The text reports cannot fail. An empty collection renders an empty string. `commits()`, `len()`, and `is_empty()` give direct access to the `RecentCommit` values.

> **Note:** the library _does not_ provide an output style specifically for the terminal. Take the prose output and render it with the `Prose` component from **biscuit-terminal**. The Sniff CLI renders it once with `WordWrap::WrapProse(None, None)`, which wraps long lines at word boundaries without adding blank lines.

### Links

The links in a recent-commits report connect a commit's hash to the page the remote host provides for that commit. Linking is part of collection, so callers receive a finished `remote` state and URL and only decide how to render them.

- Sniff knows which provider (GitHub, GitLab, Bitbucket, Gitea, etc.) hosts a remote and how that provider builds commit URLs.
- A commit is linked only when a **locally recorded remote-tracking ref** contains it.
    - The check reads local Git data only. It never fetches and never contacts the provider.
    - The result reflects the last fetch; changes made elsewhere are not visible until those refs are refreshed.
- Remote-tracking tips are walked in priority order: `origin`, then the other non-`upstream` remotes alphabetically, then `upstream`. Within a remote, its default branch (the target of `refs/remotes/<remote>/HEAD`) is walked first, then its other branches alphabetically. The first containing remote in that order wins.
- Walking stops as soon as every commit is determined. All walks share one budget of commit visits (`COMMIT_VISIT_BUDGET`, 5,000,000), so a clone with thousands of stale remote branches cannot turn one report into minutes of work.

This produces a three-state `remote` value:

| `remote` | Meaning |
|----------|---------|
| `true` | A locally recorded remote-tracking ref contains the commit |
| `false` | Every remote-tracking walk completed without finding it (including when there are no remote-tracking refs) |
| `null` | Undetermined: the visit budget ran out, or an ancestor could not be read, before the commit was found |

`commit_url` is present only when `remote` is `true` **and** the winning remote's provider yields a browser URL. A self-hosted server, or a remote-tracking ref with no configured remote URL, is therefore `remote: true` with no link. Linking never asserts that a commit is absent when it could not find out.

### Empty Results and Errors

- A valid query with no matching commits, including one against an unborn `HEAD`, returns a successful, empty collection.
- Invalid input and unreadable repositories return typed `SniffError` values, so callers never have to parse CLI messages:
    - `InvalidPeriod` - a zero count or an unparseable scope
    - `UnknownBranch` - the branch is neither a local nor a remote-tracking branch
    - `HashNotReachable` - the hash names no commit, or one that is not an ancestor of the starting tip
    - `NotAMonorepo`, `UnknownPackage`, `AmbiguousPackage`, `UnknownPackageArea` - a package or area filter the catalog cannot satisfy; unknown names list the valid ones
    - `Git` - unreadable refs, objects, or history
- Remote containment that cannot be determined is the documented `remote: null` state, not an error.

### Library Users

The library is a resource to any programmatic caller who wants to use it, and it is also the foundation of how the Sniff CLI provisions its data. The bare `sniff repo --json` aggregate embeds the same arrays under `recent_commits`, `source_code_changes`, and `documentation_changes`: one default collection (the last 10 commits), projected three ways.

## CLI Callers

The base CLI command for recent commits is:

```sh
sniff repo recent-commits [scope] <switches>
```

**Scope** can be any of the following patterns:

- Count: `10`, `25`, etc. (the default is `10`)
- Duration: `3d`, `1w`, `2mo`, `6h` (commits within that duration before now)
- Named Day: `today` (since local midnight), `yesterday` (that local day only)
- Specific Date: `2026-09-12` (that local day only)
- Hash: `ab2c3d4` (from the tip back to and including this commit; at least 7 hex characters)

The sibling commands `sniff repo source-code-changes` and `sniff repo documentation-changes` accept the same scope and switches and apply a [projection](#projections).

See [`sniff repo recent-commits`](../../cli/repo_recent-commits.md) for the complete list of switches, duration units, output modes, and exit codes.
