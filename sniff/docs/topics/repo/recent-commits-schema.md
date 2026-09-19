## Recent Commits Schema

`RecentCommits::to_json()`, `sniff repo recent-commits --json`, and the commit families of the `sniff repo --json` aggregate all produce a **bare JSON array** of commit objects, newest first. There is no envelope: no period label, repository root, or filter name. See [Recent Commits](./recent-commits.md) for how commits are selected, linked, and projected.

For each commit captured by Sniff's recent commits functionality you will get the following information:

```yaml
# Simplified Schema
$schema:
    hash: string(required;eager) -> the full hexadecimal git hash for this commit
    datetime: datetime(required;eager) -> the commit time as RFC 3339 in UTC (always `+00:00`, e.g. `2026-09-17T10:20:26+00:00`), regardless of the timezone used for selection and display
    author: author(required;eager) -> who made the commit
    operation: string|null(required;eager;suggest(fix,feat,docs,chore,refactor,test)) -> the conventional-commit operation (any word), or `null` when the subject is not a conventional commit
    scope: string|null(required;eager) -> the conventional-commit scope, or `null` when there is none
    heading: string(required;eager) -> the first sentence of the commit: the subject line, after any conventional-commit prefix, up to its first `.` (or the whole subject line when it has none)
    description: string(required;eager) -> the prose after the heading and before the bullet points, joined with single spaces; `""` when there is none
    bullet_points: string[](required;eager) -> the bullet points (`- ` or `* `) that detail the commit, in order
    files: file[](required;eager;min(0)) -> the files changed relative to the first parent; empty for a merge that changes nothing
    file_types:
        source_code: boolean(required;eager) -> whether any source code was touched in this commit
        web_assets: boolean(required;eager) -> whether any web assets (HTML, CSS, font files, etc.) were touched in this commit
        images: boolean(required;eager) -> whether any images (including vectors like SVG) were touched in this commit
        documentation: boolean(required;eager) -> whether any documentation was touched in this commit
        configuration: boolean(required;eager) -> whether any configuration files were touched in this commit
        cicd: boolean(required;eager) -> whether any CI/CD definitions were touched in this commit
    packages: string[] -> the packages owning at least one changed file, sorted (**monorepo only:** always present in a monorepo, possibly empty; omitted everywhere else)
    package_areas: string[] -> the package areas of those packages, sorted (**monorepo only:** always present in a monorepo, possibly empty; omitted everywhere else)
    remote: boolean|null(required;eager) -> `true` when a locally recorded remote-tracking ref contains the commit, `false` when every remote-tracking walk completed without finding it, `null` when that could not be determined
    commit_url: url -> the commit's page on the preferred containing remote; present only when `remote` is `true` and that remote's provider yields a browser URL
types:
    author:
        name: string(required;eager) -> the author's name
        email: string(required;eager) -> the author's email
    file: 
        kind: enum(modified,added,deleted,moved;required;eager) -> renames and copies are both `moved`
        path: file(required;eager) -> the repository-relative path (with `/` separators) of the mutated file
        original_path: file -> the source path of a `moved` file; omitted for every other kind
        added: number -> the number of lines added; omitted when the change is binary or line statistics are unavailable (never reported as `0` in that case)
        removed: number -> the number of lines removed; omitted under the same conditions as `added`
```

### Notes

- **Optional keys are omitted, not `null`.** `commit_url`, `original_path`, `added`, `removed`, `packages`, and `package_areas` are left out when absent. `operation`, `scope`, and `remote` are always present and use `null`.
- **`file_types` describes the whole commit.** A flag is set when any changed file, or the original path of a `moved` file, falls in that category. Every path belongs to exactly one category, and files in the `other` category set no flag. A projection (such as `sniff repo source-code-changes --json`) prunes `files` but leaves `file_types`, `packages`, `package_areas`, `remote`, and `commit_url` unchanged.
- **Attribution is by the deepest owning package.** A file inside a package area but outside every package is not attributed. The original path of a `moved` file counts.
- **Non-UTF-8 bytes** in the message, author name, or author email are replaced with U+FFFD rather than rejected.
- **The heading can end early.** Because it stops at the first `.`, a subject such as `fix: use e.g. foo` has the heading `use e`.

### Example

```json
[
  {
    "author": { "email": "ada@example.com", "name": "Ada Lovelace" },
    "bullet_points": [
      "Replace nested if/match blocks with let-chains",
      "Simplify scope extraction"
    ],
    "commit_url": "https://github.com/owner/repo/commit/34b6d18a0c1e5f4b2d9a7e6c3b8f1a0d2e4c6b8a",
    "datetime": "2026-09-16T14:32:00+00:00",
    "description": "",
    "file_types": {
      "cicd": false,
      "configuration": false,
      "documentation": true,
      "images": false,
      "source_code": true,
      "web_assets": false
    },
    "files": [
      {
        "added": 12,
        "kind": "modified",
        "path": "sniff/lib/src/filesystem/git/recent_commits/collect.rs",
        "removed": 4
      },
      {
        "added": 0,
        "kind": "moved",
        "original_path": "sniff/docs/old-name.md",
        "path": "sniff/docs/new-name.md",
        "removed": 0
      }
    ],
    "hash": "34b6d18a0c1e5f4b2d9a7e6c3b8f1a0d2e4c6b8a",
    "heading": "use let-chains in commit collection",
    "operation": "refactor",
    "package_areas": ["sniff"],
    "packages": ["sniff"],
    "remote": true,
    "scope": "sniff"
  }
]
```
