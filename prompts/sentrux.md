---
root: "{{ctx.repo_root}}"
area: "{{ctx.current_package_area}}"
baseline: "{{ctx.repo_root}}/{{ctx.current_package_area}}/.sentrux/baseline.json"
start: 
    stderr: "Starting a [Sentrux](https://sentrux.dev) based review on the {{area}} package area"

success:
    stderr: "The [Sentrux](https://sentrux.dev) based review for **{{area}}** completed successfully"
    say: "The Sentrux review in {{area}} completed successfully"
---

::block when="ctx.current_package_area"

Evaluate the package area "{{area}}" of the Rusty Biscuit monorepo (directory: "{{root}}/{{area}}") using the **Sentrux** MCP tools to evaluate the quality of the following metrics and provide a list of suggestions on how to improve on the score:

1. Modularity - _do edges cluster into modules?_ -- from **Newman 2004**
2. Acyclicity - _are there circular edges?_ -- from **Martin 2003**
3. Depth - _how deep are edge chains?_ -- from **Lakos 1996**
4. Equality - _are node properties concentrated?_ -- from **Gini 1912**
5. Redundancy - _are there unnecessary nodes?_ -- from **Kolmogorov**

> **Note:** the sentrux CLI has been run to create a baseline measurement which can be found at "{{baseline}}"

## Document Structure

The document should be broken up with H2 headings indicating the various "packages" found in the "{{area}}" package area:

::shell sniff repo packages --package-area {{area}}

Within each H2 section list of suggestions should be ordered by priority where priorities are (from highest to lowest):

- critical
- urgent
- important
- nice-to-have

Each suggestion should be an H3 section in the document where the H3's title should be '### `{priority}`: {name}' and each suggestion should:

- describe the problem
- indicate what source files are touched by the problem
- describe how to fix the problem
    - give code example where appropriate

## Closure

> Review File: `{{root}}/{{area}}/reviews/{{ctx.today}}-sentrux/review-1.md`

To complete the task:

- save all your suggestions to the file: `{{root}}/{{area}}/reviews/{{ctx.today}}-sentrux/review-1.md`
- add a `suggestions` frontmatter property to the review file which is the number of suggestions across all priorities
- add a `suggestions_critical` frontmatter property to the review file which is the number of suggestions which are marked as being "critical"
- add a `suggestions_urgent` frontmatter property to the review file which is the number of suggestions which are marked as being "urgent"
- once the suggestions are saved to the Markdown body of the review file and the frontmatter properties above are saved too then you are done with the review

::end-block
::block when="!ctx.current_package_area"
Evaluate the full monorepo of Rusty Biscuit.
::end-block
