In this feature we will add three related library functions to the Sniff library:

- `get_recent_commits_by_duration(duration: T) -> Vec<CommitDesc>`
    - allow inputs like:
        - `2 days`, `2d` (last two days)
        - `6 hours`, `6h` (last six hours)
        - `1 week`, `1wk`, `1w`
        - `3 months`, `3mo`, `3m`,
    - also allows `yesterday` and `today`
- `get_recent_commits_by_hash(hash: T) -> Vec<CommitDesc>`
- `get_recent_commits_by_date(hash: T) -> Vec<CommitDesc>`
    - Takes ISO Dates like `2025-12-04`

All three functions return an array of CommitDesc's which will look something like:

```rust

// this is a rough draft; feel free to modify as needed
pub struct CommitDesc {
    /// the commit's hash
    hash: String,
    /// an ISO Datetime (UTC) of when this commit was committed
    datetime: String,

    /// the packages which were modified (not populated unless a monorepo)
    packages: Option<Vec<String>>,
    /// the package areas which were modified (not populated unless a monorepo)
    package_areas: Option<Vec<String>>,

    /// relative file paths from repo root
    files: Vec<String>,
    /// the description of the commit up until bullet points are discovered
    description: String,
    /// the bullet points of the commit
    bullet_points: Vec<String>,
}
```

This struct should also offer four output functions:

1. `describe()`

    Will output the description as valid Markdown content that looks like (for each commit in the set):

    ```md
    ## {YYYY}-{MM}-{DD} at {HH}:{MM}

    - **Commit:** {commit}
    - **Files:**
        - {file-list}
    - **Description:** {description}
        - {bullet-points}
    ```

1. `describe_for_terminal(term: &Terminal)`

    - will use `darkmatter` library to render for the terminal


1. `source_code_changes(verbose: bool)`
1. `documentation_changes(verbose: bool)`

## Sniff CLI

The Sniff CLI will use these new library functions to provide the following commands:

- `sniff repo recent-commits <period>`
    - uses the `describe_for_terminal` function for standard output
    - uses the serialized data when `--json` is asked for
- `sniff repo source-code-changes <period>`
    - uses the
- `sniff repo documentation-changes <period>`

The `period` parameter should be optional and if left out the default will be 3 days. A user who does use it can provide any of the following:

- the git hash they want to start at (e.g., )
- the ISO Date they want to start at (e.g., 2024-12-25)
- the duration they want to consider (e.g., 1d, 3 months, etc.)

The CLI will be responsible for identify what type of "period" specifier is being used and then calling the appropriate function in the library.
