# The Markdown struct

This feature is 95% about implementing this struct but let's discuss the other 5%:

- **The `Markdown` struct**
    - this feature is 95% about getting this struct built!
    - The **Markdown** struct is described in detail in: [Markdown struct](../../../docs/markdown-struct.md).
    - you MUST read the `markdown-struct.md` document and ALL markdown documents it refers to BEFORE you start!
- **The Remaining 5%**
    - [Terminal Color Inspection](../../../docs/terminal_color.md) - we need to add two small utility functions for callers who want to detect information about the terminal's capability with regard to color.
    - **Testing the Terminal**
        - read the document [How to Test Terminal Output](../../../docs/how-to-test-the-terminal.md)
        - spend the time to think and research how we can and will test terminal output effectively
        - document your findings and make sure that all testing following this work is informed of what is expected of them
    - **Implement `mat` CLI**
        - The `mat` CLI has a directory and a bare Cargo.toml file but it's not implemented at all yet.
        - The good news is that it will be VERY easy once we have the `Markdown` struct
        - Further good news is that this CLI will expose the `Markdown` struct to some easy interactive testing which might be highly valuable. 

## Sequencing

The plan for this feature is going to be a multi-phase project. The precise sequencing will be decided as part of the planning process, however, here are some broad ideas on how things should be sequenced:

1. The Early Stuff

    - get the Terminal Color Inspection done and out of the way
    - make sure we have a good idea of how to be effective in testing terminal output early!
    - get the `Markdown` crate basics completed with the _exception_ of:
        - any of the output features (this is where a lot of the work resides)
        - the "clean" feature
    - test and iterate

2. The Initial Touch Points

    - get the "clean" implementation completed 
    - get the `mat` CLI implemented
    - at this stage you can use the `mat` CLI and validate that it's clean() function works.

3. The `pulldown-cmark`, `syntect`, `two-face` parsing

    - implement the HTML and terminal outputs
    - avoid adding any DSL features to start

4. Fancy Stuff

    - add the DSL features and continue to test and iterate

5. AST

    - we use the `markdown-rs` create to export an AST representation
    - this is cool and will be quite useful but it can wait until the backend of the project to be implemented

6. Documentation and Knowledge Management

    - all documents in the `docs/*` folder should be evaluated for _drift_, _completeness_, and _consistency_.
    - we should make sure all summary level docs like `shared/README.md` `/README.md`, and `/CLAUDE.md` are all updated in an appropriate manner.

> **NOTE:** documentation updates SHOULD NOT wait until the end; every phase of the project should go through a documentation refresh but at the end we will dedicate a little more time for it and make sure we can "see the forest through the trees" with regard to documentation and knowledge reuse.
