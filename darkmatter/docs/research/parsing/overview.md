---
sequence: ./parsing-crates.yaml
$schema:
    index: number -> the numeric index of this sequence
    is_first: boolean -> flag indicating whether we are on the first item in sequence
    is_last: boolean -> flag indicating whether we are on the last item in sequence
    next: { name: string, filepath: string }
    state: { name: string, filepath: string }
---
## Context

::block when="is_first"
Your job is to review the detailed research on the Rust crate "{{state.name}}" located at "{{state.filepath}}". 

Once you've digested the information on "{{state.name}}", then proceed to review the on the Rust crate "{{next.name}}" located at: "{{state.filepath}}".

You should now have a good foundational knowledge of to think about parsing in Rust though obviously it's still really only grounded in two crates so if you wish to do any additional research to round out your understanding then you should take the time to do that.
::end-block
::block when="!is_last"

::end-block
::block when="is_last"


## Task

The overall task is to build a comprehensive overview document for how to implement parsing in Rust. The document you're writing should include the following sections:

- `## Parsing Use Cases`
    - describes common use cases for parsing
    - tries to give solid, thought out, non-crate specific based names for each use case
    - describe why the use case comes about, what is important to consider in this type of use case, common pitfalls to watch out for




::block when="is_last"


::end-block
