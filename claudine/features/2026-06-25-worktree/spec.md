This feature introduces the ability to allow claudine to move work on a task to a new worktree and branch in git.

- it will work with `compose`, `inline-compose`, and `sequence` operations

## Key Configuration Dimensions and Conventions

When allowing claudine to use git worktrees the key configuration choices that must be made are:

- Naming
    - worktree source branch
    - new worktree name
    - new branch name
- Location
    - where should the new worktree be located on disk
- Merge Strategy

As you will see in the following sections each of these choices can be made automatically for the caller or the caller can take control over the various areas they want to express themselves.

Across all of this Claudine always makes some simplifying assumptions:

1. All file references used in a claudine call that uses worktrees must be a clean commit in git; if that is not the case a clear error will be raised:

    - given this CLI call: `claudine compose prompts/plan.md plan=features/my-fancy-thing/plan.md`
    - the file reference of `prompts/plan.md` (the prompt file) must resolve to a cleanly committed git file
    - the file reference of `features/my-fancy-thing/plan.md` must resolve to a cleanly committed git file

2. If there is a `.worktreeinclude` in the root of the repo then this will be used to determine what files and directories (which have been _excluded_ in the `.gitignore` file) should be copied over to new worktree/branch combo

3. The default location for where new worktrees are added to the host computer is `~/.claudine/worktrees` but the user can override this default by:

    - setting a value in the user's configuration file at `worktrees.base_dir`
        - Note: the repo's configuration file is NEVER consulted as location on a hosts file system is always going to be host specific
    - if the ENV variable `WORKTREE_DIR` is set to a valid filepath then this will override Claudine's default as well as the user's config
    - from that base directory Claudine will use the following offsets:
        - `{repo-name}/{worktree-name}`

## Opting in via the `--worktree` CLI switch

- a caller must opt-in to this feature and that is done via the `--worktree` CLI switch:
    - the syntax is `--worktree <name>` where _name_ can be:
        - `true` 
            - when using the boolean **true** value the caller is abdicating naming responsibility for the worktree and branch
            - see [auto naming](#auto-naming) section for how this will be done
        - `false`
            - when specifying the boolean **false** value the caller is expressing that they _do not_ want a separate worktree and branch created
            - this is the default behavior that you'd get without the use of `--worktree` but is provided to allow for an explicit stance on this
        - `#{number}` for working on a PR
            - this format will indicate that you expect there to be a PR with the specified numeric ID on the git remote
            - claudine will validate that this PR actually exists
                - if it doesn't then a clear error message will be provided
            - if the PR number has been looked up AND the worktree `pr-{number}` doesn't already exist on the host then the worktree `pr-{number}` will be created with the branch the PR is located on
                - if the PR number was found on remote but there already exists a `pr-{number}` worktree on this host then an error explaining the situation and encouraging the user to simply move into that 
        - `::{string}`
            - allows the caller to set the name of the branch
            - Note: if the string value is not a valid branch name then an error will be returned
            - while allowing Claudine to choose the worktree name
                - see [auto naming](#auto-naming) section for how this will be done
        - `{string}` for naming the worktree
            - an alphanumeric plus `_` and `-` character string (must lead with a alpha character) can be specified as a name
            - when this is provided then this will 


### Auto Naming

TODO

## Initial User Reporting

When a worktree is successfully requested, Claudine will create the worktree and report:

- Status::Info("this session has been moved into the <b><blue>{worktree}</blue></b> worktree inside the <b><purple>{branch}</purple></b> branch.")
- Status::Info("this worktree is located on the host at: <dim><blue><href>{file-ref}</blue></dim></href> on this host")
- using the `git-graph` functionality from **biscuit-terminal** it will then show a graph of the new worktree and it's relationship to:
    - the 'main/master/{default}' branch
    - the branch which this worktree was sourced from
        - obviously if sourced from 'main/master/{default}' then don't duplicate it
- then if there were any files copied over based on the `.worktreeinclude` file then within a BlockQuote add:
    - `Non repo files copied into this branch:`
    - then add an unordered list of the files (each file should be blue and be an OSC8 link)

## Job Closure Strategy

Once the claudine command completes then the **closure strategy** is employed.  Strategies are determined by (from highest to lowest precedence):

1. The `--strategy <strategy>` CLI switch
2. The `WORKTREE_STRATEGY` ENV variable (if a valid value)
3. The Claudine repo configuration file at `worktree.strategy`
4. The User's repo configuration file at `worktree.strategy`

The types of strategies available are:

1. `basic`

    Regardless of whether the job was successful or ended in an error we take the same closure action of providing context to the user but nothing more:

    - the basic strategy just reports additional closure information:
        - Status::Info('this job used the <blue></blue> prompt file and was sourced from the <<blue>{worktree}</blue> worktree')
        - if there are dirty files in the current worktree:
            - Status::Info('the current uncommitted files in this worktree are:')
            - use the sniff library to report; using a reporting format similar to what `sniff repo git-status --compact` provides
        - if there are no dirty files in the current worktree:
            - Status::Info('all files in this worktree have been committed')
            - if this worktree has a clean/no merge-conflict path to merge with the branch it was sourced from then:
                - Status::Info('there are no conflicts with merging this branch back into <blue>{worktree}</blue> when you're ready')
                - if there is a conflict then: Status::Warn('there will be merge conflicts if you decide to merge back to <blue>{worktree}</blue>')
            - if this worktree has a clean/no merge-conflict path to merge with the 'main/master/{default}' branch then:
                - Status::Info('there are no conflicts with merging this branch into <blue>{default}</blue> when you're ready')
                - if there is a conflict then: Status::Warn('there will be merge conflicts if you decide to merge back to <blue>{default}</blue>')

2. `source-into-new`

    ```mermaid
    flowchart LR
        Source(Source Worktree)
        New(New Worktree)
        Complete([Complete Prompt])
        Merge{Merge}

        Source --> |split| New
        New --> Complete

        Complete --> Merge
        Source --> Merge

        Conflict{"Has\nConflict"}
        Stop
        Offer

        Merge --> Conflict

        Conflict --> |yes| Stop
        Conflict --> |no| Offer
    ```

    - this strategy looks to merge the content from worktree/branch it sourced from after completing it's work
    - this only happens if the work was deemed successful, in the case of an error the error is reported and work is stopped
    - however, on success we if we can merge -- without merge conflicts -- we will not only merge but offer to have the 

3. `new-into-source`
4. `new-into-main`
5. `pr`
