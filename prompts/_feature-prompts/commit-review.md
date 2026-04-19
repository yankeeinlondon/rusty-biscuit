### Commit Review Changes (`commit-review`)

This section describes how to perform the actions necessary for the `commit-review` stage.

- capture the current repo status before making any changes
- after the implementation was completed, we made sure that the current package area was clean (no dirty files)
- now that we've finished finished both a review and implemented the suggestions in the review we want to again stage and commit the changes which happened.
- we use commit message which follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard
- in this case the `action` for the commit will be **feat**
- the `scope` for the commit will almost surely be current **package area** (`sniff package-area`)
- this means that the first line of the commit message will be something like: `feat($(sniff package-area)): implemented the review suggestions on the "{{feature}}" feature`
- the remaining lines of the commit should summarize the change that were made
- Append to the log file:
    - the log file is located at `{{base_dir}}/log.md`
    - Start your log entry with the heading `## Committed Review Changes`
    - Add the initial git status prior to our commit and put that information in `### Before Commit`
    - Add the git status after our commit and put that information in `### After Commit`
- set the `last_updated` frontmatter property on the log file
    - use the command `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`

You are done when:

- you have staged all the files mutated during the review and the implementation of its suggestions
- you have committed these same files with an appropriate Conventional Commit based message
- the log file has been appended to

DO NOT PUSH ANY COMMITS to a REMOTE!

