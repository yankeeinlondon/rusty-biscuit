# Terminal Output

The `sniff` CLI has two primary outputs:

1. Terminal (_the defaut output_)
2. JSON

In this document we will explore what the terminal output should look like:

Unlike the JSON output, which lives by a rigid set of rules, the **terminal** output must go through a design with the intent of making the output as easy to understand and take in for a human reading in the terminal as possible.

The one strict rule which the terminal output must follow is:

- the data that the terminal output reports on (in non-verbose mode) must be constrained to the metadata that the JSON output provides for that level of the command structure

We will allow that a verbose command reach out to _related_ peer data and therefore it doesn't need to be constrained to the pure metadata of that level. While, this is "allowed" this should be the exception rather than the rule.

## Design Approach

- during the design process we must consider:
    - our audience is a human and in most cases we want a relatively compact report that doesn't overwhelm them
    - there are two tiers of information each report should consider:
        - the default terminal output
        - the _verbose_ terminal output
    - this two tiered system allows us to design conservatively for the default terminal output and then provide a large amount of the metadata when the user opts into the verbose output
- A well designed output -- leveraging colors, tables, lists, and other formatting or structural techniques, etc. -- allows a denser information output to be shown to the user while retaining clarity and insight to the user. This means:
    - very simple outputs probably don't need to think about this, but
    - when the JSON output provides a large pallette of metadata, all _renderable_ components in `biscuit-terminal` and `darkmatter` should be considered as ways in which to make the information more "digestable"
    - these _renderable_ components (e.g., implement the `TerminalRenderable` trait) are all designed to add visual styling and structure in a way that is battle tested and also will fall back automatically to the capabilities of the actual terminal you will be reporting to.


> TODO: need to talk about:
>
> - approprite use of STDOUT vs STDERR
> - annoucing `--with-network` and `-v` when there is a variant available
