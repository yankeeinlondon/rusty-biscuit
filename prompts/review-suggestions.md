---
$schema:
    review: file(required)
    spec: file
ready: "{{ review && file_exists(review) ? frontmatter(review, 'ready') : null }}"
index: "{{ is_indexed_file(review) ? file_index(review) : '' }}"
has_index: "{{ is_number(index) }}"
directory: "{{ dir(review) }}"
spec_path: "{{ join(directory, 'spec.md') }}"
spec: "{{ file_exists(spec_path) ? spec_path : '' }}"

start:
    message "🏃 starting the implementation of _review_ **{{ title_case(parent_dir(review)) }}**\'s suggestions"
success:
    message: "✅ the **{{ title_case(parent_dir(review)) }}** review suggestions were implemented successfully!"
failure:
    message: "❌ the review suggestions from **{{ title_case(parent_dir(review)) }}** failed to complete!"
---
- use the '{{ctx.area}}' agent skill
- use the 'rust' skill when writing code
- use the 'rust-testing' skill when writing or debugging tests

::block when="ready"
The review -- {{ review }} -- was marked as being **production ready** so there is no longer a need to continue the review-to-implement loop.

Explain this to the caller and then exit.
::end-block
::block when="!ready"
The review {{ review }} has completed in the {{ctx.area}} package area with suggestions for implementation.

Your task is to:

1. act as an orchestrator and iterate over each suggestions serially
2. for each suggestion call a subagent to:
    - implement the suggestion,
    - add tests to provide full test coverage for the suggestion,
    - and make sure that the implementation passes all tests (just test)
    - and has no lints (just lint)

::block when="spec_path"
> **Note:**
>
> - this review's suggestions were based on evaluating the current implementation of the spec file:
>
> {{ directory + '/spec.md' }}
::end-block
::end-block
