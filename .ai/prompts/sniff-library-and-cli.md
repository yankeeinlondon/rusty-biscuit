# Sniff CLI and Library

A new module has been added to this monorepo called Sniff (`/sniff`) which contains both a library module (`/sniff/lib`) and a CLI module (`/sniff/cli`).

- the CLI, like others in this monorepo, will leverage the `clap` crate for it's structuring and CLI functionality. It will then leverage the Sniff library for it's functionality.

The primary utility of this library is to detect information about the host system in three main areas:

1. Hardware
2. Network
3. Filesystem/Repo Info (based on a directory ... CWD as default)

## Functionality

### Hardware

- uses the `hardware-query` and `sysinfo` crates to gather hardware information on the host
- break discovery into discrete areas:
    - aggregate all info that can be gathered quickly together into one utility function
    - any features which have performance considerations should be added separately
- the goal is to gather all the information these crates can provide into a sensible dictionary of information (avoiding any checks which would substantially increase the timeframe to gather info)

**Note:** if `sysinfo` is redundant (aka, `hardware-query` ends up being a functional super set) then we can remove this crate.

### Network

- uses the `getifaddrs` crate to find out as much about the network environment as possible.
- break discovery into discrete areas:
    - aggregate all info that can be gathered quickly together into one utility function
    - any features which have performance considerations should be added separately

### Filesystem/Repo Info

This area, unlike the other two, is sensitive to a particular directory. By default the CWD will be assumed but the library should allow that to be changed to any directory and the CLI will use the `--base`/`-b` switch to specify a directory other than CWD.

- Programming Languages
    - Using the `rust-code-analysis` crate to identify what the programming languages are in the crate.
    - If there is 99% one language and a smattering of Bash scripts too I want to be sure that we do not treat these equally. It's ok to mention the Bash scripts but the utility of a repo like this is driven by the other programming language.
- Git Info
    - Using the `git2` crate to dig into **git** info
    - **Remote**
        - What remote's does this repo have?
        - Hosting provider (enum of Github, Bitbucket, GitLab, AwsCodeCommit, Gitea, AzureDevOps, Other)
            - for `origin`
            - for `upstream`
            - ... for others
    - **Status**
        - How many files are dirty and have not been committed?
        - How many of the dirty files are "staged" to be committed?
        - Is the current branch available on remote?
            - If so how many commit's behind?
            - If not, how many commits before there is a record on remote?
    - **Branches**
        - The list of branches locally and remotely
        - each with their last commit hash and last commit date

### Aggregating all Sniff Categories

- the `detect()` utility function in @sniff/lib/src/lib.rs is meant to be a common way for people to interact with the Sniff Module. It will gather information from all three areas above.
