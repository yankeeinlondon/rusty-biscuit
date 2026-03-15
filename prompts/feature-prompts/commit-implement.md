### Commit files from Implementation and Design

This section describes how to perform the actions necessary for the `commit-implement` stage.

- before the implementation was executed, we made sure that the current package area was clean (no dirty files)
- now that we've finished the implementation of the plan for the "{{feature}}" feature we need to stage and commit the files which we have modified in the implementation
- we use commit message which follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) standard
- in this case the `action` for the commit will be **feat**
- the `scope` for the commit will almost surely be current **package area** (`sniff package-area`)
- this means that the first line of the commit message will be something like: `feat($(sniff package-area)): implemented the "{{feature}}" feature`
- the remaining lines of the commit should summarize the functionality provided in the feature in a number of bullet points.

You are done when:

- you have staged all the files mutated during the implementation
- you have committed these same files with an appropriate Conventional Commit based message

DO NOT PUSH ANY COMMITS to a REMOTE!
