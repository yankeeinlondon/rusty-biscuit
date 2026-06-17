---
$schema:
    review: file(required)
    spec: file
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"
index: "{{ is_indexed_file(review) ? file_index(review) : '' }}"
has_index: "{{ is_number(index) }}"
dir: "{{ dir(review) }}"
spec: "{{ file_exists(join(dir, 'spec.md')) ? join(dir, 'spec.md') : '' }}"
---

::block when="ready"
The review {{ link(review) }} was marked as being **production ready** so there is no longer a need to continue the review-to-implement loop.
::end-block
::block when="!ready"
The review {{ link(review) }} has completed in the {{ctx.area}} package area with suggestions for implementation.

Your task is to:

1. act as an orchestrator and iterate over each suggestions serially
2. for each suggestion, call a subagent to implement the suggestion, add tests to provide full test coverage for the suggestion, and make sure that the implementation passes all tests (just test) and has no lints (just lint)

::block when="spec"
> **Note:** this review's suggestions were based on evaluating the current implementation of the {{link(spec)}} spec file.
::end-block
::end-block
