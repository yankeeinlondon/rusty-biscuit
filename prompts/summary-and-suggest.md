---
name: summary-and-suggest
description: Provides a summary of recent activitity in a given area of the monorepo along with suggestions for improvements
timeframe: "2 weeks"
today: "{{ctx.today}}"
review_dir: "@{{ctx.area}}/reviews/{{today}-summary-and-suggest"
---

## Context

You are working in the {{ctx.area}} of the **rusty-biscuit** monorepo and have been asked to perform both a summary and a review focused on the '{{area}}' area.

The review document you will produce will consist of the following sections:

- `# Achievements and Suggestions for the **{{ctx.area}}** Package Area (_{{timeframe}}_)`
    - `## `

- you will start by summarizing the activity and achievements in '{{area}}' during the past {{timeframe}} timeframe
    - report on new functionality that has been delivered
    - report on functionality that was fixed
    - report on any new package that was added to the package area
- at the same time that you're evaluating the activity/achievements, you should be evaluating:
    - what aspects of the code do NOT follow the DRY standard? How could better reuse and less codepaths be achieved to ensure we don't end up with duplicative code or even worse, slightly variant implementations of the same functionality.
        - you are allowed to consider design patterns which include not only packages in the '{{area}}' but also other packages in the monorepo which are a direct dependency of the packages in '{{area}}'
    - are there ways to make the 