The claudine CLI and library offers a "MCP mode" as a feature to users. This was initially implemented with minimal specification but has just recently been re-specified in greater detail and that detailed specification used to implement a more complete solution.

## Your Task

Your task is to review the current test coverage of the "MCP Mode" functionality. This functionality is described in the following specification documents:

- @claudine/docs/cli/mcp-mode.md
- @claudine/docs/cli/mcp-catalog.md

Once you understand the requirements:

- document a set of recommendations on where we need better test coverage
    - make sure that not only code and functional tests are in place but that we have adequate doctests too
    - each recommendation should be clearly specified both WHAT tests are missing and HOW we need to fill that gap
    - if there are tests which you feel are suspect:
        - they don't really test what they intend to
        - what they're testing is static and doesn't really need a test
        - timeouts are WAY too long
        - etc.

Write all your suggestions to @claudine/reviews/mcp-test-review.md 

Once this is complete, have a look at the @claudine/docs/mcp-features.md document. This document is an attempt to write the specifications as a set of "features". Review these features and based on what you see, consider whether additional recommendations should be written to @claudine/reviews/mcp-test-review.md .
