### Update Documents

We have just completed the implementation of the {{feature}} feature and now need to make sure the documents within this **package area** are up-to-date with the source code. 

#### Documents in a Package Area

When evaluating a **package area**, there are a set of documents which should ALWAYS exist inside a package area:

1. Root README.md

    At the root of the **package area** is a README.md which is meant to:

    - summarize the high-level utility of the packages in this area
    - list out all of the packages in this area; each package should:
        - be a Markdown link which links to that package's README.md
    - describe how the different packages relate to one another

2. Package README.md's

    Every **package** in a **package area** will also have a README.md file which goes into greater detail than the more summary focused Root README.md

Beyond these documents you may also find some **Module README's** nested inside the source code tree. These documents:

- describe the source code in their folder and any sub-folders it contains
- typically -- but not always -- these folders are segmented as a "module" (formally or informally depending on the language)
- these documents are meant to provide useful context to developer reviewing the code

Finally, the other documents which we will concern ourselves with are documents which are defined in the `docs` frontmatter of the Root README.md. 

> **Note:** the `docs` property may or may not exist; both are valid states

#### Tools

You can get a list of the README's in the current package area by running:

```sh
sniff docs --readme "$(sniff package-area)"
```

You can get the list of "other documents" references in the `docs` property of the Root README by running: 

```sh
md get "@README.md" docs || ""
```

#### CLI Documentation

When you are documenting a CLI package's README.md, please consider the following best practices for documentation of a CLI:

- Start by providing a broad overview of the functionality this CLI provides
- If the CLI has "subcommands" then you should organize by these subcommands
    - CLI's may have "subcommands" within "subcommands" so follow this structure recursively where it makes sense
    - Each subcommand should be given a description of what it's used for
    - Each subcommand should enumerate the CLI switches that are in scope for this command and how their use will change the 

#### Task Steps

- you will act as an orchestrator
- create a full list of documents which you are responsible for:
    - make sure that there is a README.md at the package area root, and the root of each package in this package area
    - using the commands in tools to generate this complete list
- iterate over each document and:
    - spawn a subagent to review the document
    - provide the following links to the subagent:
        - Feature Specification is located at `{{base_dir}}/spec.md`
        - Technical Design Specification is located at `{{base_dir}}/tech-design.md`
        - The Review Suggestions made and implemented are located at `{{base_dir}}/review.md`
        
        The subagent should be provided these resources; the subagent can decide whether or not to read all of them but should choose which ones to read so that it has appropriate context to what the {{feature}} feature's scope is.

    - provide the subagent the information in the "#### Documents in a Package Area" so they have context about the document focus and content
    - if the subagent has been assigned the README.md for a CLI package then provide the `#### CLI Documentation` section for context
    - the subagent is responsible for updating the document based on the changes introduced by the {{feature}} feature but if other inconsistencies are detected then they should be corrected too
    - all subagents should be instructed that their updates should be focused on changes in functionality not on rephrasing existing functionality
        
- once all documents have been updated you will log each file's updates (including when no updated was needed)
    - Your log entries should start with a heading of `## Document updates`
    - Append to the file: {{base_dir}}/log.md
    - Set the frontmatter property `docs` to the documents which were reviewed for changes
        - use `fm set "{{base_dir}}/log.md" docs {docs} --save`
- set the `last_updated` frontmatter property on the log file
    - use the command `md set "{{base_dir}}/log.md" last_updated "${YYYY}-${MM}-${DD}" --save`
- communicate to the user that all documents have been updated
