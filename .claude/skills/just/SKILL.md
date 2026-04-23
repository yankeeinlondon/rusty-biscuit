---
name: just
description: >
  Reference for `just`, the command runner. Use when working in a project with
  a `justfile` or when the user mentions `just` or `justfile`.
---

# Just


## Discovery

- `just --dump` Print justfile
- `just --evaluate` Print variable values
- `just --help` Print detailed command-line syntax help
- `just --list` Print recipes with descriptions
- `just --show <RECIPE>` Print recipe source
- `just --summary` Print recipes without descriptions

## Execution


- `just` Run default recipe
- `just <RECIPE>` Run specific recipe
- `just <RECIPE> <ARG1> <ARG2` Run recipe with arguments

## Syntax

```just
executable := 'main'

# compile main.c
compile:
  cc main.c -o {{ executable }}

# run main
run: compile
  ./{{ executable }}

# run test
test name: compile
  ./bin/test {{ name }}

# start webserver
serve port='8080':
  python -m http.server {{port}}

# publish current tag
publish:
  #!/usr/bin/env bash
  set -euxo pipefail
  tag=`git describe --tags --exact-match`
  ./bin/check-tag $tag
  git push origin $tag
```

## Notes

The comment proceeding a recipe is used as its doc-comment, and included in
`just --list`.

By default, each line of a recipe runs in a fresh shell. Recipes whose bodies
start with `#!` are written to a file and executed as a script.

Commonly used commands and scripts should be turned into `just` recipes.

## In this Monorepo

In the rusty-monorepo (where you are currently working), you will find:

- there is a `justfile` at the root of the monorepo
- there is a `justfile` in each "package area" in the mono repo (run `sniff repo package-areas`) for a list of package areas
- the justfile's leverage a set of shared recipes that live in the `just/*` directory:
    - `ai.just` provides some useful recipes for interacting with AI
    - `devops.just` provides useful devops commands like `commit`, `_build`, `_lint`, `_install`, and many more
    - `lifecycle.just`  provides some opinionated lifecycle commands like:
        - `schedule` moves a previously _unscheduled_ feature or fix (in the `_unschedule` subdirectory of features or fixes) into the "features" or "fixes" directory and changes the name to be prefixed with the DATE that this specification has been moved to being scheduled
        - `complete` moves a scheduled feature to the `_completed` directory
        - etc.
    - `notify.just` provides ways to inform the user of status including:
        - `_speak` provides TTS services to allow the host to speak a message
        - `_play` and `_play_background` provides a set of sound effects which the user can choose from to indicate some sort of event
        - etc.
    - `review.just` provides a way for a user to kickoff a review process or implement the suggestions from a review (leveraging the "claudine" service defined in this monorepo)
    - `spec.just` provides a way for a users to:
        - create a new specification file for a feature or fix (`just feature`, `just fix`)
        - clarify the requirements in a specification (`just clarify`)
        - etc.
    - `plan.just` provides a user a way to create and then implement plan for a specification file and/or a technical design (or both)
    - `util.just` provides a number of utility recipes that the other `.just` files leverage

## Memory

### Starting a Task

When you start your work read the memory/just.md file (if it exists) as it provides useful context that has been learned over time.

### Completing a Task

If you feel like you learned something novel about `just` and not within your normal training set that is worth remembering then you should append that learning to the `memory/just.md` file (create the file if it doesn't exist).

