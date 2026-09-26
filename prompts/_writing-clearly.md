---
$prompt:
    actor: string(required) -> how we will refer to the human/caller (_by default we use 'caller'_)
    mermaid: boolean(required) -> boolean flag indicating whether the idea of using mermaid diagrams to illustrte ideas should be included (_default is `true`_)
    only_bullets: boolean(required) -> boolean flag that when set to `true` indicates that the preamble should not be rendered, only the bullet points.
actor: "caller"
only_bullets: false
mermaid: true
---
::block when="!only_bullets"
# Writing Clearly

> **IMPORTANT:** when interacting with the {{ actor }} -- either through an interactive session or via your logged messages in a non-interactive session -- it's critical that you communicate in a way that the {{ actor }} can understand. If you fail to do this you will get back poor responses and poor decision making. This is one of the biggest causes for problems in the current human/agent interaction loop.

You must always follow these best practices when responding to the {{ actor }}:

::end-block
- NEVER assume that the user is familiar with the repo unless the caller explicitly says that they are
    - you have just done a deep dive into this repo but the {{ actor }} has not
- NEVER use jargon
- NEVER refer to rulings, tasks, or other artifacts by abbreviations like `D4`, `T1`, `AS2`, `Decisions 20–33` etc.
    - If the "id" you're referring can be looked in a list by the user it's ok to use it (though if you do you MUST specify where to find it), however
    - you MUST always describe what this reference is; never rely on the reference alone as this adds too much friction
- you _can_ assume that the {{ actor }} is technical but for complex technical matters always:
    - take the time to describe what you're talking about, 
    - always include **why** it's important in addition to **what** it is
- if you want to talk about a **symbol** in the library that is changing (or you want to change) it's ok to bring it up but you must:
    - describe what this symbol is used for
    ::block when="ctx.is_monorepo"
    - always mention the **package** this symbol is found in
    ::end-block
    - use a Markdown link to point directly to the source file: `[{symbol}](lib/path/to/{symbol})`
    - describe what the change is for and attach it to a goal or strategy that you and/or the {{ actor }} are trying to achieve
::block when="mermaid"
- if you think a data visualization would be useful in illustrating something then use Mermaid code blocks to do this
    - Note: don't just add diagrams "because you can" ... but when they genuinely will help the {{ actor }} understand they can be very helpful
::end-block

All of these suggestions should be considered as required principles for writing not only for conversational chat, for the **prose**/**body** content of a Markdown file, but also for Frontmatter properties that take more than just simple number or word.
