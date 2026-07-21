---
sequence: 
    - name: darkmatter
    - name: more-is-more
    - name: conflict
base: "@darkmatter/fixes/2026-07-20-dm-mega-merge"
darkmatter_log: "{{ base }}/darkmatter-log.md"
more_is_more_log: "{{ base }}/more-is-more-log.md"
conflict: "{{ base }}/conflict-report.md"
---

## Context

We are trying to merge the `more-is-more` branch back into the `darkmatter` branch (note: both have worktrees
by the same name). This will be a complex merge and we want to take all the precautions necessary to make it go
smoothly. One problem we had during the implementation of this work was that the Rust compilation and testing on multiple worktrees was causing a large amount of CPU contention even though the host is a very high spec machine.

> Note: on commit d672388dd0fed4196295e7f21514cac6fa59f0ae the `more-is-more` branch was forked from `darkmatter`

### Features and Fixes

The work that has been performed over the lifetime of these two branches are:

- `2026-07-15-performance-followup` 
    - a cross cutting piece of work that aimed to improve performance
    - most of the performance improvement will have hit the **Darkmatter** package area's source code but some also addressed underlying concerns found in the **sniff** library
    - it is possible that some may have also hit the `biscuit-terminal` package area too (needs confirmation)
    - the good news is that the implementation of this feature was done BEFORE the `more-is-more` branch was split off
    - the bad news is that there were some review/fix cycles that were performed on `darkmatter` but not `more-is-more` after the split took place
- `2026-07-13-more-is-more`
    - this feature provides new context variables, expression functions, and a few more functional addons
    - this work likely is fully isolated to the `darkmatter` package area (needs confirmation)
- `2026-07-13-meta-schema`
    - added some new types to the SimplifiedSchema
    - this work has been done exclusively in the `more-is-more` worktree and branch
- `2026-07-16-redundant-walk`
    - a fairly isolated performance fix
    - was implemented only in the `darkmatter` worktree and branch
- `2026-07-13-fixed-width-lists`
    - addresses a bug in Darkmatter's "clean" operation where it was only addressing a subset of the scope for adjusting the fixed width found in the Darkmatter content
    - was implemented only in the `darkmatter` worktree and branch
- `2026-07-14-invalid-frontmatter`

## Task

::block when="state.name == 'darkmatter'"
Your task is to focus exclusively on the `darkmatter` worktree and branch and document over it's modern history. This document will be saved to: '{{ darkmatter_log }}'

- the first section -- called `## Overview` -- will:
    - review the features/fixes that were executed in `darkmatter`:
        - `2026-07-15-performance-followup`
        - `2026-07-16-redundant-walk`
        - `2026-07-13-fixed-width-lists`
        - and `2026-07-14-invalid-frontmatter`
    - the review section should open with a broad based paragraph introducing the work but then should allow for each of the features/fixes which have been worked on to have a more detailed description of precisely what this work hoped to achieve and what it's acceptance criteria looked like
    - make sure to document the `packages` that were changed in this feature/fix as well as what _modules_ within those packages were changed
- the next section will be a timeline of commits which will go in it's own H2 heading: `## Timeline`
    - this timeline should be an unordered list with every commit made in this branch since (and including) when `more-is-more` was branched off
    - for many of the commits all that's necessary is the first line of the commit message which summarizes what happened
    - however, for commits that have particular significance you should create a nested list of messages describing the commit and describing why it's important
- the final section entitled `## File Blast Radius` will capture a list of all files which were mutated in `darkmatter` branch over the period that `more-is-more` and `darkmatter` were split.

> NOTE: make sure to use the 'darkmatter' skill during this task

::end-block

::block when="state.name == 'more-is-more'"
Your task is to focus exclusively on the `more-is-more` worktree and branch and document over it's modern history. This document will be saved to: '{{ more_is_more_log }}'

- the first section -- called `## Overview` -- will:
    - review the features/fixes that were executed in `more-is-more`:
        - `2026-07-13-more-is-more`
        - `2026-07-13-meta-schema`
    - the review section should open with a broad based paragraph introducing the work but then should allow for each of the features/fixes which have been worked on to have a more detailed description of precisely what this work hoped to achieve and what it's acceptance criteria looked like
    - make sure to document the `packages` that were changed in this feature/fix as well as what _modules_ within those packages were changed
- the next section will be a timeline of commits which will go in it's own H2 heading: `## Timeline`
    - this timeline should be an unordered list with every commit made in this branch since (and including) when `more-is-more` was branched off from `darkmatter`
    - for many of the commits all that's necessary is the first line of the commit message which summarizes what happened
    - however, for commits that have particular significance you should create a nested list of messages describing the commit and describing why it's important
- the final section entitled `## File Blast Radius` will capture a list of all files which were mutated in `more-is-more` branch over the period that `more-is-more` and `darkmatter` were split.

> NOTE: make sure to use the 'darkmatter' skill during this task

::end-block

::block when="state.name == 'conflict'"
We have created to documents which detail the changes in the `darkmatter` and `more-is-more` branches over the time that they were split:

- [`darkmatter` branch]({{ darkmatter_log }})
- [`more-is-more` branch]( {{ more_is_more_log }} )

Your task is to write a conflict report to '{{ conflict }}' which:

- describes in detail the key conflict areas which will need to be carefully managed during this merge
- describes what functionality was developed that exists _outside_ of the **Darkmatter** package area
- describes the appropriate approach we should use to merge this successfully
- define how we will make sure that the underlying "acceptance criteria" of each fix/feature is assured after the merge is complete

If there are any things which can be done to further reduce the merge risk prior to starting please mention that as well.

> NOTE: make sure to use the 'darkmatter' skill during this task

::end-block
