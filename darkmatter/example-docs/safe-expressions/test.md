---
csv: "foo, bar, baz"
list:
    - foo
    - bar
    - baz
---
# Safe Expressions

Darkmatter provides a set of operators and functions which are deemed to be _safe_ (aka, no side effects) which can be used to gather useful 
information or to mutate a document's Frontmatter.

## Example 1: changing a CSV list into other forms

::block when="length(ctx.dirty_files) == 1"
> **Note:** currently this repo only has a _single_ file that is either untracked or updated since the last commit to **git**; this is fine but
> makes for a less exciting demo.
::end-block

::block when="ctx.dirty_files"
Currently this repo has _dirty_ files which have not yet been committed to **git** and the context variable `ctx.dirty_files` makes those
files very accessible as a CSV list: 

> {{ctx.dirty_files}}

Sometimes, however, the CSV presentation is not what you're looking for and the Darkmatter expression engine provides several functions which
can change the output format of this list:

1. `as_line_separated(ctx.dirty_files)`:

    {{ as_line_separated(ctx.dirty_files) }}

2. `as_space_separated(ctx.dirty_files)`: 

    {{ as_space_separated(ctx.dirty_files) }}

3. `as_unordered_list(ctx.dirty_files)`:

    {{ as_unordered_list(ctx.dirty_files) }}

4. `as_ordered_list(ctx.dirty_files)`:

    {{ as_ordered_list(ctx.dirty_files) }}

::end-block

::block when="!ctx.dirty_files"

> **Note:** 
>
> - this page's normal example leverages presenting the _dirty_ files in this repo but currently all files have been committed to **git**.

::end-block

{{ as_ordered_list(csv) }}

{{ as_ordered_list(list) }}
