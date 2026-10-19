---
sequence: ./parsing-crates.yaml
---
## Context

::block when="seq.is_first"
Your job is to review the detailed research on the Rust crate "{{seq.name}}" located at "{{seq.filepath}}".

Once you've digested the information on "{{seq.name}}", then proceed to review the on the Rust crate "{{seq.next.name}}" located at: "{{seq.filepath}}".

You should now have a good foundational knowledge of to think about parsing in Rust though obviously it's still really only grounded in two crates so if you wish to do any additional research to round out your understanding then you should take the time to do that.
::end-block
::block when="!seq.is_last"

::end-block
::block when="seq.is_last"

::end-block


## Task

The overall task is to build a comprehensive overview document for how to implement parsing in Rust. The document you're writing should include the following sections:

- `## Parsing Use Cases`
    - describes common use cases for parsing
    - tries to give solid, thought out, non-crate specific based names for each use case
    - describe why the use case comes about, what is important to consider in this type of use case, common pitfalls to watch out for




::block when="seq.is_last"


::end-block
