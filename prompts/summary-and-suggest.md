---
name: summary-and-suggest
description: Provides a summary of recent activitity in a given area of the monorepo along with suggestions for improvements
timeframe: "2 weeks"
today: "{{ctx.today}}"
review_dir: "@{{ctx.area}}/reviews/{{today}-summary-and-suggest"
---

## Context

You are working in the {{ctx.area}} of the **rusty-biscuit** monorepo and have been asked to perform both a summary and a review focused on the '{{ctx.area}}' area.

The review document you will produce will consist of the following sections:

- `# Achievements and Suggestions in the **{{ctx.area}}** Package Area (_{{timeframe}}_)`
    - `## Achievements`
    - `## Suggestions`

- The activity and achievements in '{{ctx.area}}' during the past {{timeframe}} timeframe go into the `## Achievements` section:
    - report on new functionality that has been delivered
    - report on functionality that was fixed
    - report on any new package that was added to the package area
    - report on any improvements in test coverage
    - report on any improvements to test benches
    - report on code which has been made more DRY
    - report on code which has been made more performant
    - report on code that has been made more ergonomic
    - report on fixes made to documentation drift

    Where possible you should:

    - reference the git commit(s) associated
    - reference the related `spec.md` specification file(s)
        - found in `@{{ctx.area}}/features` or `@{{ctx.area}}/fixes`

- at the same time that you're evaluating the activity/achievements, you should be evaluating what **more** could or should be done:
    - this content will go in the `## Suggestions` section
    - what aspects of the code do NOT follow the DRY standard? How could better reuse and less code paths be achieved to ensure we don't end up with duplicative code or even worse, slightly variant implementations of the same functionality.
        - you are allowed to consider design patterns which include not only packages in the '{{ctx.area}}' but also other packages in the monorepo which are a direct dependency of the packages in '{{ctx.area}}'
    - are there ways to make the code more ergonomic?
    - how can we make the code more performant?
        - segment these opportunities into opportunities which have only upside versus those which have some sort of complexity such as:
            - Complexity: adding lifetimes, large number of callsites and requires changing signature
            - Ergonomics: the performance gain would come as an ergonomic loss
            - etc.
    - where is test code overage still too weak?
    - is there still documentation drift from the source code's ground truth?
    - are there any "god files" which should be refactored?

## Commits

The commits in the "{{ctx.area}}" package area over the past _{{timeframe}}_ are:

::shell sniff repo recent-commits --package-area "{{ctx.area}}" --plain || "no commits during this timeframe"

## Tools

We listed out the commits in the previous section, however, you are only seeing the commit hash and the first line of the commit message.

In this monorepo we always use conventional commits (with scoped areas) and we provide both a summary commit message as well as several bullet points of information with greater details. To get the full commit message as well as the files involved in any given commit you should run:

```sh
sniff repo commit <hash> --plain
```

If you wanted to see all of the recent commits with all of the underlying details (hash, first line message, detailed message, files involved) you can get that listed
by running

```sh
sniff repo recent-commits "{{timeframe}}" --package-area "{{ctx.area}}" --plain -vv
```

## Task

- before starting, determine your strategy for use of sub-agents, you should favor acting as an Orchestrator so that you may preserve as much of your context window as possible
- Once your strategy is clear, you will go about researching the changes over the past {{timeframe}} as best you see fit
- This strategy MUST result in:
    - Writing the review document to `{{review_dir}}/review.md` which completes all sections mentioned
    - Add and save the following frontmatter properties to the same review document:
        - `agent` set to '{{env.AGENT}}'
        - `created` set to '{{ctx.now}}'
        - `date_range` specify the date range we looked at
        - `interactive` set to '{{env.INTERACTIVE}}'
        - `model` set to '{{env.MODEL || "default"}}'
        - `suggestions` set to a numeric value based on the number of suggestions you have provided
        - `features` list all "features" that contributed to the "{{ctx.area}}" package areas changes
        - `fixes` list all the "fixes" that contributed to the 
        - `commits` list all the commits (hashes only) which contributed to the changes in the "{{ctx.area}}" package area over the past {{timeframe}}
- Once the document is complete, provide a summary description of your findings to the caller
    - save this same summary which you report to STDOUT to the `summary` frontmatter property

> **Note:** the `features` and `fixes` Frontmatter properties refer to the parent directory name from where "spec.md" file was located. If we were in a
> package area called "foobar" then you'd expect directories like `foobar/features` and `foobar/fixes` to exist and an active feature being worked on 
> might be called something like: `foobar/features/2026-06-06-do-something/spec.md`. The "name" of the feature in this case would be `2026-06-06-do-something`.
> 
> - `fixes` are those which were found under the directory "*/fixes/"
> - `features` are those which were found under the directory "*/features/"
>
> **Note:** by convention we locate spec files in:
> 
> - a feature or fix which resides in a `_completed` directory indicates the feature/fix is deemed to be completed/done
> - specifications under a directory called `_unscheduled` are described but not yet implemented
>    - they may or may not have a "plan.md" plan file already created
> - fixes/features under feature/fix but NOT under the `_completed` or `_unscheduled` directories are features/fixes which
>   are currently being worked on
>     - in a few cases a fix/feature might be in this state but actually have "completed" 
>     - our normal process is to run a series of "implementation reviews" which you'll find in the same directory as `review-1.md`, `review-2.md`, etc.
>     - each review writes the `ready` frontmatter property to true/false
>     - when a review reaches a state that it's considered "production ready" the `ready` property will be set to `true` and in 99.9% of cases that 
>       means we consider this fix/feature to be done
