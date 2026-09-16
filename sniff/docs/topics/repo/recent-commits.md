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

    - `json` - pure data format with all information in payload
    - `prose` - text format with formatting provided with the [prose](@darkmatter/docs/topics/prose-grammar.md); this formatting is largely Markdown grammar but with a few extra features that allow colorization and a few other useful features.
    - `terminal` - the [prose](@darkmatter/docs/topics/prose-grammar.md) format is converted to escape codes to display the formatting in a terminal
    - `plain` - the plain text format that has no links, or formatting

## 2. Scoping: Which Commits

    There are two primary ways we can specify _which commits_ will be reported:

    - **Raw Count**
        - the simplest approach is to list a specified number of commits
        - the default -- when no scoping is provided -- is to provide the 10 most recent commits
    - **Time Window**
        - often we want to reflect on the commits of the last day, week, etc.
        - the commits can be just as easily determined by providing a duration (from now) or an absolute date

    In addition to these two main mechanisms for scoping which commits are reported, we can also _filter_ the commit list down by a number of criterion:

    - **blast-radius**
        - the `package` and `package-area` (for monorepos) can be used narrow the blast-radius you are interested in
        - a similar -- though mildly less reliable mechanism -- is to use `scope` for similar blast-radius limiting
    - **file-type**
        - this is a slightly variant type of _blast radius_ filtering but you can filter on whether any of the following _types_ of files were changed in the commit:
            - source code
            - web assets (HTML, CSS, Font files)
            - images (scalar and vector)
            - documentation
            - configuration
            - cicd
    - **operation**
        - if you're using [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) then filtering by the `operation` can be useful for certain tasks

## 3. Verbosity

There are three top-level report levels:

1. Compact
2. Normal
3. Verbose

> **Note:** the reporting verbosity is irrelevant when choosing the JSON output option as JSON always reports all available data.

### Normal

Let's start with how a "normal" report looks:

```txt
- [{hash}] {operation}({scope}) at {time} {date}: {heading}

  **Files Impacted:**
  - {mutation-type}: {filepath}
  - ... 
```

The normal verbosity level provides basic metadata, the _heading_ description, and a summary of the files impacted but leaves off the remaining commentary and bullet points describing the commit.

- the `{hash}` is a 7 character short hash
    - if the commit has been pushed to it's remote then it will be rendered as a link so a user clicking the link will be brought to the remote's webpage for the commit
- the `{heading}` is the first sentence of the 

### Compact

The compact variant shows only the first line as the normal report:

```txt
- [{hash}] {operation}({scope}) at {time} {date}: {heading}
```

## Payload Schema

Each commit captures the following data:

```yaml
# Simplified Schema
$schema:
    datetime: datetime(required;eager) -> the date and time of the commit
    operation: string(suggest(fix,feat,doc,cicd,config,test)) -> if this follows the _conventional commits_ convention then the commit's operation will be listed
    scope: string -> if the commit follows the _conventional commits_ convention and uses a "scope" typically associated to a monorepo, then that will be found in the "scope" property
    hash: string(required;eager) -> the full hexadecimal git hash for this commit
    remote: boolean -> whether or not the commit has been pushed upstream to a cloud host
    heading: string(required;eager) ->  the first sentence of the commit (terminates on `.` character or `\n`).
    description: string(required;eager) -> the descriptive content for the commit after the first sentence (see _heading_) and before the bullet-points.
    bullet_points: string[](required;eager) -> the bullet points that detail out the commit message
    file_types:
        source_code: boolean(required;eager) -> boolean flag indicating whether any source code was touched in this commit
        web_assets: boolean(required;eager) -> boolean flag indicating whether any web assets (HTML, CSS, Font files, etc.) was touched in this commit
        images: boolean(required;eager) -> boolean flag indicating whether any images (including vectors like SVG) were touched in this commit
        documentation: boolean(required;eager) -> boolean flag indicating whether any documentation was touched in this commit
        configuration: boolean(required;eager) -> boolean flag indicating whether any configuration files were touched in this commit
        cicd: boolean(required;eager)
    
    packages: string[] -> the packages which were effected by the commit (**note:** this is only available when run in a monorepo)
    package_areas: string[](required;eager) -> the package areas which were effected by the commit (**note:** this is only available when run in a monorepo)
types:
    file: 
        kind: enum(modified,added,deleted,moved;required;eager)
        path: string(required;eager) -> the filepath to the mutated file
        added: number -> the number of lines added (_only set when `kind` is **modified**_)
        removed: number -> the number of lines removed (_only set when `kind` is **modified**_)
        symbols: string[] -> future reporting which list exported symbols in the file which were impacted; currently not populated
```


## Library Callers

The `sniff` library is the primary interface for collecting recent-commit reports. Rust callers should be able to select commits, apply filters, and obtain the payload described above without invoking the CLI or initializing a terminal. The CLI uses the same library contract.

As a simple example, we can collect the last 10 commits and report on a variety of ways:

```rust,ignore
use sniff::filesystem::git::{
    GitRepo, RecentCommits, RecentCommitsOptions, ReportVerbosity
};
// connect to a git repo
let repo = GitRepo::discover(".")?;
// get a collection of the last 10 commits
let commits = RecentCommits::default().collect(&repo)?;
// deserialize commits to JSON array
let json = commits.to_json();
// define the reporting options you want
let options = RecentCommitsOptions::default().verbosity(ReportVerbosity::Verbose);
// and then use those options to report in two 
// different output styles
let prose = commits.to_prose(&options)?;
let plain = commits.to_plain(&options)?;
```

The `RecentCommits` struct allows us to collect commits from a git repo. We can immediately deserialize these commits to JSON using the built in `.to_json()` method but we also have the ability to produce string based _reports_ with varying verbosity, and style characteristics.

### Selection and Filters

In our first example we used `RecentCommitsOptions` to set the reporting verbosity but there is a lot more we can do with options to choose what our collection actually contains:

```rust,ignore
let options = RecentCommitsOptions::default()
    .duration(Duration::days(5))
    .operation("fix")
    .verbosity(RecentCommitsVerbosity::Compact);
let commits = RecentCommits::using(&options)?;
```

In this example we have assigned `commits` to a collection of commits but instead of just taking the last 10 commits into the collection we have instead asked for all commits over the last 5 days which are conventional commits and use the "fix" operation to describe themselves.

All selection variants are made available through the builder pattern and because only _one_ selection variant is allowed at a time we use Rust's **typestate** pattern to only allow for one selection (any more will result in compile time error).

The selection builders are:

- `duration(dur: chrono::Duration)`, 
- `named_date(name: NamedDate)`,
- `since(date: chrono::NaiveDate)`, 
- `hash<T: Into<String>(hash: T)`, 
- `count(count: int8)`

> Attempts have been made to push as many invalid inputs for selection to being compile time errors but the real world is messy and some selection types can still generate runtime errors.

All the filters described in this documentation are available to be set in `RecentCommitsOptions` too; including:

- `package(pkg)`, `package_area(area)`
- `operation(op)`, `scope(scope)`
- `author(author)`, `branch(branch)`
- `has_source_code()`, `has_documentation()`, ...

Unlike selection criteria, filtering operations are _additive_ and so all filters will be combined with a logical AND operation to determine which commits are added to the collection.

> **Note**: selection normally establishes the candidate commits before filtering. With count selection, filtering happens during history traversal, which continues until the requested number of matching commits has been collected or the available history is exhausted.

### Library Outputs

The `RecentCommits` struct provides both raw deserialization of it's data to JSON as well as a few text reporting options:

- `to_json()` - deserializes to a JSON array
- `to_plain()` - provides a pure text (no escape sequences, no Markdown formatting hints, etc.)

The library may provide shared, output-independent report composition. Prose and plain text are useful library outputs because callers can save them to files, embed them in other documents, or pass them to a renderer.

| Responsibility | Owner |
|----------------|-------|
| Collect and enrich commit records; apply selection and filters | Library |
| Expose the complete serializable payload | Library |
| Compose Compact, Normal, and Verbose Prose or plain-text reports | Library |
| Parse CLI arguments into the library request and report options | CLI |
| Render Prose as terminal escapes, including terminal hyperlinks | CLI / terminal renderer |
| Render for a browser | Consuming application / browser renderer |
| Write stdout/stderr and choose process exit codes | CLI |

Report options control verbosity and whether authors appear in text. Author display is independent of author filtering. Date/time presentation needs an explicit timezone policy shared by all text projections. Prose can express semantic styling and links, but must not contain terminal escape sequences or browser-specific markup. Plain text contains neither formatting syntax nor links.

### Empty Results and Errors

- A valid query with no matching commits returns a successful report with an empty commit list. 
- Invalid selection, an unknown requested package or area, repository access failures, and failed required enrichment return typed library errors; callers must not have to parse CLI messages. 
- Optional enrichment needs a documented unavailable state rather than silently asserting a negative fact.

### Library Users

The library is a resource to any programmatic caller who wants to use it and it is also the foundation of how the Sniff CLI provisions it's data as well.


## CLI Callers

The base CLI command for recent commits is:

```sh
sniff repo recent-commits [scope] <switches>
```

**Scope** can be any of the following patterns:

- Duration: `3d`, `1w`, `2mo`, `6h` (commits going back the specified duration to latest)
- Named Day: `today`, `yesterday` (commits starting on named day to latest)
- Specific Date: `2026-09-12`
- Hash Start: `ab2c3d` (from this has to latest)
- Count: `10`, `25`, etc.

### Duration Units

| Unit   | Aliases                                |
|--------|----------------------------------------|
| Hours  | `h`, `hour`, `hours` |
| Days   | `d`, `day`, `days` |
| Weeks  | `w`, `wk`, `week`, `weeks` |
| Months | `mo`, `m`, `month`, `months` (30 days) |
| Years  | `y`, `yr`, `year`, `years` (365 days) |

### CLI Switches

#### Filters

- `--package <pkg>` - commits that impacted the specified _package_ in a monorepo
- `--package-area <area>` - commits that impacted the specified _package area_ in a monorepo
- `--operation <op>` - commits which use conventional commits and have the specified option
- `--scope <scope>` - commits which use conventional commits and have a given scope
- `--author <name | email>` - commits provided by a particular author
- `--branch <branch>` - commits on a specified branch
- `--source-code` - commits which have source code changes
- `--documentation` - commits which have documentation changes
- `--images` - commits which have images
- `--configuration` - commits which have config changes
- `--cicd` - commits which have CICD changes
- `--web` - commits which have web asset changes

#### Reporting Verbosity

- `--verbose` / `-v` - switches to verbose reporting
- `--compact` / `-c` - switches to compact reporting
- `--author` - adds the author who made the commit to the commit message

#### Formatting

- `--json` - no formatting, just data
- `--plain` - all formatting removed
- `--terminal` - this is the default formatting that will be used
- `--prose` - formatting prose but plain text
