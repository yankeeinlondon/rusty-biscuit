---
blast_radius:
  - sniff/cli/src/args.rs
  - sniff/cli/src/commands.rs
  - sniff/cli/src/output/remote.rs
  - sniff/lib/src/remote/mod.rs
  - sniff/lib/src/remote/types.rs
  - sniff/lib/src/remote/provider.rs
  - sniff/lib/src/remote/url_parser.rs
  - sniff/lib/src/remote/github.rs
  - sniff/lib/src/remote/gitlab.rs
  - sniff/lib/src/remote/gitea.rs
  - sniff/lib/src/remote/bitbucket.rs
---

# The `sniff repo remote` Subcommand

Inspects a remote git repository via its hosting provider API and renders a structured report covering metadata, pull requests, issues, tags, CI/CD configuration, and key URLs.

## Argument Forms

The `REMOTE` positional argument accepts three forms:

```
sniff repo remote <URL>
sniff repo remote <remote-name>
sniff repo remote <owner/repo>
```

**Full URL** — any HTTPS, SSH, or `git://` URL pointing to a repository:

```
sniff repo remote https://github.com/rust-lang/cargo
sniff repo remote git@github.com:rust-lang/cargo.git
sniff repo remote ssh://git@gitlab.com/inkscape/inkscape
sniff repo remote https://gitlab.example.com/team/project
sniff repo remote https://codeberg.org/forgejo/forgejo
```

The `.git` suffix is stripped automatically. Self-hosted instances are detected from the URL host.

**Remote name** — a git remote name from the current repository (e.g., `origin`, `upstream`). The name is resolved to a URL via the local git config and then handled identically to a full URL:

```
sniff repo remote origin
sniff repo remote upstream
```

This requires that the current directory (or `--base`) is inside a git repository containing the named remote.

**Owner/repo shorthand** — exactly one `/` separating two non-empty segments. No URL scheme or host:

```
sniff repo remote rust-lang/cargo
sniff repo remote owner/repo
```

With a shorthand, the provider is not known in advance. Sniff probes configured providers sequentially (GitHub, GitLab, Bitbucket) and uses the first one that returns a successful response. Gitea is excluded from shorthand probing because it is self-hosted with no default base URL.

## Supported Providers

| Provider | Detected hosts |
|---|---|
| **GitHub** | `github.com`, GitHub Enterprise (URL-based) |
| **GitLab** | `gitlab.com`, any host beginning with `gitlab.` or containing `.gitlab.` |
| **Gitea / Forgejo** | `codeberg.org`, any host beginning with `gitea.`, `forgejo.`, or `git.`; unknown hosts default to Gitea |
| **Bitbucket** | `bitbucket.org` |

GitLab supports nested groups in URLs (e.g., `group/subgroup/repo`). The entire group path becomes the owner.

## Authentication

Each provider reads credentials from environment variables. Credentials are required for all operations — there is no unauthenticated fallback.

| Provider | Environment variables |
|---|---|
| GitHub | `GITHUB_TOKEN` or `GH_TOKEN` |
| GitLab | `GITLAB_TOKEN` or `GITLAB_PRIVATE_TOKEN` |
| Gitea | `GITEA_TOKEN`; for Codeberg, `CODEBERG_TOKEN` takes precedence over `GITEA_TOKEN` |
| Bitbucket | `BITBUCKET_USERNAME` **and** `BITBUCKET_APP_PASSWORD` (Basic auth) |

Bitbucket app passwords can be created at `https://bitbucket.org/account/settings/app-passwords/`.

When using the `owner/repo` shorthand, providers with no credentials configured are silently skipped. If no configured provider finds the repository, sniff exits with a `ShorthandNotFound` error listing which providers were tried.

## Default Output

The default (non-verbose) output renders the following sections using biscuit-terminal components:

**Header** — repository full name (`owner/repo`) and the provider name.

**Description** — the repository's configured description, if any.

**Stats line** — stars, forks, and open issue count in a compact inline row.

**Metadata list** — language, license (SPDX ID when available), default branch, and flags for archived or private repositories.

**Topics** — repository topics/tags, if any.

**Documents** — categorized list of documentation files found in the repository: README files (bolded), a summary entry for `docs/` folder contents (with file count), and other root-level documents.

**CI/CD** — detected CI/CD providers with their config file paths (e.g., `.github/workflows/ci.yml`).

**Pull Requests** — table of up to 10 open pull requests with number, title, state (`open`, `closed`, `merged`), and author. Draft PRs are annotated with `[draft]`.

**Issues** — table of up to 10 open issues with number, title, state, and author.

**Tags** — list of up to 5 recent tags with annotated/lightweight distinction.

**URLs** — key links: repository, homepage, issues page, pull requests page, releases page, and wiki (each shown only when present).

## Verbose Mode (`-v`)

Adding `-v` or `--verbose` fetches the README content and renders it as markdown after the standard report using darkmatter's terminal renderer.

```
sniff repo remote rust-lang/cargo -v
sniff repo remote origin -v
```

The README is identified as the first document in the report with category `Readme`. If no README is found or the fetch fails, the verbose flag has no visible effect (it does not cause an error).

## JSON Output (`--json`)

```
sniff --json repo remote <REMOTE>
```

Returns the complete `RemoteReport` as pretty-printed JSON:

```json
{
  "provider": "GitHub",
  "metadata": {
    "name": "cargo",
    "full_name": "rust-lang/cargo",
    "description": "The Rust package manager",
    "private": false,
    "default_branch": "master",
    "language": "Rust",
    "stars": 12345,
    "forks": 1234,
    "open_issues": 42,
    "archived": false,
    "created_at": "2014-01-01T00:00:00Z",
    "updated_at": "2025-01-01T00:00:00Z",
    "pushed_at": "2025-01-01T00:00:00Z",
    "license": { "spdx_id": "MIT", "name": "MIT License" },
    "topics": ["rust", "package-manager"],
    "has_issues": true,
    "has_wiki": false,
    "homepage": "https://doc.rust-lang.org/cargo/",
    "html_url": "https://github.com/rust-lang/cargo"
  },
  "org_info": { "name": "rust-lang", "display_name": "The Rust Programming Language", "description": null, "avatar_url": null, "html_url": null },
  "documents": [
    { "path": "README.md", "category": "Readme", "size": 4096 },
    { "path": "CHANGELOG.md", "category": "Other", "size": 65536 }
  ],
  "pull_requests": [
    {
      "number": 1234,
      "title": "Fix dependency resolution",
      "state": "open",
      "author": "username",
      "draft": false,
      "source_branch": "fix/dep-resolution",
      "target_branch": "master",
      "created_at": "2025-01-01T00:00:00Z",
      "updated_at": "2025-01-02T00:00:00Z",
      "merged_at": null,
      "html_url": "https://github.com/rust-lang/cargo/pull/1234"
    }
  ],
  "issues": [
    {
      "number": 5678,
      "title": "Error when workspace has circular deps",
      "state": "open",
      "author": "username",
      "comment_count": 3,
      "labels": ["bug"],
      "created_at": "2025-01-01T00:00:00Z",
      "updated_at": null,
      "closed_at": null,
      "html_url": "https://github.com/rust-lang/cargo/issues/5678"
    }
  ],
  "tags_and_releases": {
    "tags": [
      {
        "name": "0.80.0",
        "commit_sha": "abc123",
        "annotated": true,
        "message": "Release 0.80.0",
        "tagger": "username",
        "tagged_at": "2025-01-01T00:00:00Z"
      }
    ],
    "releases": [
      {
        "name": "Cargo 0.80.0",
        "tag_name": "0.80.0",
        "draft": false,
        "prerelease": false,
        "published_at": "2025-01-01T00:00:00Z",
        "html_url": "https://github.com/rust-lang/cargo/releases/tag/0.80.0"
      }
    ]
  },
  "ci_cd": [
    {
      "provider": "GitHub Actions",
      "config_path": ".github/workflows/main.yml",
      "name": "CI",
      "status": "completed",
      "conclusion": "success",
      "html_url": "https://github.com/rust-lang/cargo/actions/runs/12345",
      "started_at": "2025-01-01T00:00:00Z"
    }
  ],
  "org_repos": [],
  "key_urls": {
    "repo": "https://github.com/rust-lang/cargo",
    "homepage": "https://doc.rust-lang.org/cargo/",
    "docs": null,
    "issues": "https://github.com/rust-lang/cargo/issues",
    "pull_requests": "https://github.com/rust-lang/cargo/pulls",
    "wiki": null,
    "ci_cd": "https://github.com/rust-lang/cargo/actions",
    "insights": null,
    "releases": "https://github.com/rust-lang/cargo/releases",
    "settings": null
  }
}
```

All fields use the provider-normalized types. `null` values represent optional fields that are absent or unsupported by the provider. `org_info` is `null` when the owner is not an organization.

## Plain Output (`--plain`)

Adding `--plain` strips all ANSI escape codes (colors, bold, hyperlinks) from the text output.

```
sniff repo remote origin --plain
sniff --plain repo remote rust-lang/cargo
```
