---
name: Code Comment Quality
description: |-
    This prompt will analyze the source code in a package area and make sure it conforms to the best practices that this monorepo recommends.
favorite: true
operation: "code-comments"
review: {{  }}
---
# Code Comment Quality

Code comments are super useful for both agents and humans and are encouraged but encouraged _only_ when done properly as poor commenting practices can not only undermine the value created but actually move into negative territory.

You will be evaluating code commenting in the "{{ ctx.area }}" package area's code base and suggesting improvements to be made. Your review findings will be saved to "{{review}}".

::block when="ctx.area == ctx.package || null"
- this package area has the following Rust packages in it which should be analyzed:

    {{ raw_markdown(as_unordered_list(ctx.current_packages)) }}
::end-block

## Best Practices

::file @docs/comment-quality.md 

> Note: these best practices are found in the file @docs/comment-quality

## Lessons Learned

The "lessons learned" is accumulated knowledge written by other agents adding "novel" learnings they've had about the "code-comments" task so that you can benefit from others who have executed this process:

{{ctx.repo_root}}/.claudine/memory/code-comments.md

These lessons learned can be useful additions to the more static best practices we've documented. When reading the lessons learned. Be aware that occasionally there will be duplication with agents writing the same idea in different ways and even less often an agent might introduce something that feels like a contraction to the best practices.

If you find anything that feels like a contradiction to our best practices it is important that you add a section `## Memory Issues` to your report and describe the contradiction you are seeing. You should also note where you see what you believe to be duplication.

## Task

- iterate over the packages you are responsible for
- for each package run the following command: `hug suspicious-comments <package>`
    - this will provide you a list of files and function blocks which need to be evaluated
    - the suspicious comment blocks will be tagged as one of the following:
        - `no-comments-on-exported-symbol` - all exported symbols should have a comment so when you see this tag you know there is a comment to be added
        - `suspiciously-large-comment` - comments which are large aren't necessarily bad or wrong but they do require greater scrutiny and 
        - `formatting-instructions` as is discussed in the best practices, comments should not quote literal format strings, ANSI escape codes, color names, or emoji codepoints in prose. When you see this tag a strong majority of the time you're dealing with a real code smell that needs to be fixed but if there you believe this is exception then make sure include a comment on why you feel this code block represents an exception
        - `one-liner-field-` - a one line comment _can_ be valid but often when you see a one line comment the comment is really just an obvious restatement of what the
