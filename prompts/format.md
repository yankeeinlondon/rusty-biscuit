---
description: |-
    Applies consistent formatting to this monorepo using `rust fmt`. Once the formatting has
    been applied the changes will be staged and committed as a single commit.
dirty: {{ as_unordered_list( ctx.dirty_files ) }}

initialize:
    stack:
        - when: "length(ctx.dirty_files) > 0"
          action:
              - error: |- 
                    The **format** prompt expects there to be no dirty files in the repo as a pre-requisite to applying consistent Rust formatting across this repo! Please commit the following files before running this prompt:

                    {{dirty}}
        - when: "!repo"
          action:
              - error: |-
                    You ran the **format** prompt outside of a repo! To use this prompt you must be in a repo and the repo needs to be written in Rust.
start:
    stack:
        - stdout: "Applying consistent formatting for all source files in the **{{ctx.repo}}** repo.\n"
        - shell: "rust fmt --all"
        - stdout: |-
            
            Formatting of the source code has completed successfully, resulting in **{{length(current.dirty_files)}}** being updated to meet the expected formatting style. We will now commit these changes to the repo.
        - shell: 'cd {{ctx.repo_root}} && git add . && git commit -m "chore: formatted sourced code to the repos agreed on formatting style"'
        - stdout: |-

            The formatted files have been committed.
        - stop
---

Note: this prompt is fully deterministic in it's execution so there is no prompt needed in the body
