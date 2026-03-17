# Merging `sniff git` and `sniff repo`

The `git` and `repo` subcommands were originally seen as being distinct enough that they deserved their own subcommand but now it's just causing confusion as these both have a lot to do with a repo. In this feature we'll merge the functionality of both subcommands into the `repo` subcommand.

## Current `git` options

```sh
Show git repository information, or inspect a remote by name/URL

Usage: sniff git [OPTIONS] [REMOTE] [COMMAND]

Commands:
  hash       Show details for a specific commit by SHA
  staged     List files staged for commit
  unstaged   List modified but unstaged files
  untracked  List untracked files
  help       Print this message or the help of the given subcommand(s)

Arguments:
  [REMOTE]  Remote name (e.g., "origin"), URL, or owner/repo shorthand to inspect

Options:
  -h, --history <HISTORY>  Number of recent commits to display (default: 10) [default: 10]
      --json               Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
      --refresh-remotes    Refresh remote-tracking data before reporting branch and sync status
  -p, --package <PKG>      Filter to commits and changes within a package
  -v, --verbose...         Increase output verbosity
      --help               Print help
```

## Current `repo` options

```sh
Show only repository/monorepo structure

Usage: sniff repo [OPTIONS] [FILTER] [COMMAND]

Commands:
  deps                                  Render an internal dependency diagram
  packages                              Output only package names as a comma-separated list
  package                               Output the package name for the current directory
  package-area                          Output the package area for the current directory
  dirty-packages                        Output only package names that have uncommitted changes
  dirty-package-areas                   Output only package area names that have uncommitted changes
  package-root                          Output the root directory of the current package
  package-area-root                     Output the root directory of the current package area
  repo-root                             Output the root directory of the repository
  is-current-package-area-dirty         Exit 0 if the current package area has uncommitted changes, exit 1 otherwise
  package-area-has-source-code-changes  Exit 0 if the current package area has source code changes, exit 1 otherwise
  help                                  Print this message or the help of the given subcommand(s)

Arguments:
  [FILTER]  Filter packages by name (or @area); prefix with ! to exclude

Options:
      --latest-versions  Query package registries for latest dependency versions and report available updates
      --json             Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
  -v, --verbose...       Increase output verbosity
  -h, --help             Print help
```

## Merge Strategy

- remove the `help` subcommand; it can be a subcommand but we don't need to see it in the help system
- neither of the base subcommands (repo, git) overlap in terms of their child subcommands which makes the merge more straight forward
- the complication is that both commands have a default behavior when a second subcommand is not provided
    - when we run `sniff repo` it reports on the structure of the repo
        - when we merge we should add this as the command `sniff repo structure` and if `sniff repo` is run without a subcommand we'll make "structure" the default subcommand
        - this means we have a named subcommand but when a user runs `sniff repo` they will get the same results after the merge as they do currently
    - when we run `sniff git` without a child subcommand we get a report on the git status
        - when we merge that same report should be available as `sniff repo git-status`
    - we will rename `sniff git staged` to `sniff repo staged-files`
    - we will rename `sniff git untracked` to `sniff repo untracked-files`
    - we will rename `sniff git unstaged` to `sniff repo unstaged-files`
- in all other cases, the `sniff git <subcommand>` should just be moved over to `sniff repo <subcommand>`

## Global Switch

- we need to add a global switch `--plain` which will strip all terminal escape codes
    - you will use `biscuit-terminal`'s `strip_escape_codes()` function to do this
- if the `--json` and `--plain` flags are used together then `--plain` is ignored and the JSON is provided


## Help System

Today's global help system looks like this:

```sh
💻❯ sniff --help
sniff 0.1.0
Detect system and repository information

Usage: sniff [OPTIONS] [COMMAND]

  -b, --base <BASE>  Base directory for filesystem analysis
      --json         Output as JSON instead of text (with subcommand) or force JSON (no subcommand)
  -v, --verbose...   Increase output verbosity
  -h, --help         Print help
  -V, --version      Print version


Commands:
  Top-level sections:
    sniff os          Show only OS information
    sniff hardware    Show only hardware information
    sniff network     Show only network information
    sniff filesystem  Show only filesystem information
    sniff topics      Show subsection topics as a table

  Hardware details:
    sniff cpu             Show only CPU information
    sniff gpu             Show only GPU information
    sniff memory          Show only memory information
    sniff storage         Show only storage/disk information
    sniff audio-devices   Show only audio devices

  Filesystem details:
    sniff git                        Show only git repository information
    sniff git --refresh-remotes      Refresh remotes before reporting sync status
    sniff git hash HEAD              Show details for the latest commit
    sniff git hash abc1234           Show details for a specific commit
    sniff git staged                 List files staged for commit
    sniff git staged -v              Staged files with action labels
    sniff git unstaged               List modified but unstaged files
    sniff git untracked              List untracked files
    sniff git --package homelab      Scope to commits within a package
    sniff git origin                 Inspect the 'origin' remote
    sniff git owner/repo             Inspect by owner/repo shorthand
    sniff git https://github.com/... Inspect a remote by URL
    sniff repo                       Show only repository/monorepo structure
    sniff repo --latest-versions     Check registries for dependency updates
    sniff repo biscuit               Filter to packages matching "biscuit"
    sniff repo !biscuit              Exclude packages matching "biscuit"
    sniff repo @sniff                Filter to packages in the "sniff" area
    sniff repo deps                  Show internal dependency list (text)
    sniff repo deps --ui             Show internal dependency diagram (Mermaid)
    sniff repo deps biscuit          Filtered text dependency list
    sniff repo packages biscuit      Filtered CSV package names
    sniff repo package               Package name for current directory
    sniff repo package-area          Package area for current directory
    sniff repo dirty-packages        Packages with uncommitted changes
    sniff repo dirty-package-areas   Package areas with uncommitted changes
    sniff repo package-root          Root directory of the current package
    sniff repo package-area-root     Root directory of the current package area
    sniff repo repo-root             Root directory of the repository
    sniff repo is-current-package-area-dirty  Exit 0 if CWD's area is dirty, 1 otherwise
    sniff repo package-area-has-source-code-changes  Exit 0 if CWD's area has source changes
    sniff language                   Show only language detection results
    sniff files                      Show broad file associations
    sniff files --association image  Show only image file statistics
    sniff docs                       Show markdown documents in the repository
    sniff docs --readme              Show only README.md files
    sniff docs --plan                Show only plan-related documents
    sniff docs --src                 Show only documents under src/ directories
    sniff docs --has-prompt          Show only documents with a prompt
    sniff docs homelab               Filter documents matching "homelab"

  Programs:
    sniff programs                   Show all installed programs
    sniff editors                    Show only installed editors
    sniff editors install            Interactive install picker for editors
    sniff editors install vim        Install vim directly
    sniff utilities                  Show only installed utilities
    sniff language-package-managers  Show only language package managers
    sniff os-package-managers        Show only OS package managers
    sniff tts-clients                Show only TTS clients
    sniff terminal-apps              Show only terminal apps
    sniff audio                      Show only headless audio players
    sniff agents                     Show only AI agent CLI tools

  Services:
    sniff services              Show running services (default)
    sniff services --state all  Show all services

Output modes:
  - No subcommand: Show this help (use --json for full JSON output)
  - With subcommand: Text output by default, use --json for JSON

Examples:
  sniff                      # Show this help
  sniff --json               # Full system info as JSON
  sniff cpu                  # CPU info as text
  sniff cpu --json           # CPU info as JSON
  sniff --json cpu           # Same as above (flag position flexible)
  sniff programs             # Programs as text
  sniff programs --json      # Programs as JSON
  sniff filesystem --refresh-remotes --latest-versions  # Enriched filesystem report
  sniff editors install      # Interactive editor install picker
  sniff -b /path/to/repo filesystem  # Analyze specific directory

```

This is too long!

- remove examples from the global help, having examples on a per command basis is a good idea
