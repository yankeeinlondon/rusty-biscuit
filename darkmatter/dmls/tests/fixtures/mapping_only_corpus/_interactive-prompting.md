---
description: |-
    Provides 
$prompt:
    actor: string(required) -> how we will refer to the human/caller (_by default we use 'caller'_)
    maturity: |-
        enum(na,greenfield,brownfield; required) -> 
            allows caller to specify how mature the functionality they're working on is; by default we disable it with the 'na' option. It's main utility
            is in callers leveraging interactive decision making as often agents will be overly cautious about making "the right decision" because of the
            impact to a large existing customer base. When using this you should consider setting the `scope` property which scopes what you're calling
            "greenfield" or "brownfield".
interactive: false
actor: "caller"
maturity: "na"
---

::file ./_writing_clearly.md actor={{actor}}


## Interactive Prompting

The {{ actor }} has asked for you to interactively go back and forth with them on a set of questions and this brings up it's own set of best practices. The best practices above still apply 100% but in addition please consider:

- ALWAYS ask one question at a time
    - this provides focus but it also allows you to _adjust_ if one question impacts the next
- ALWAYS evaluate the "right" answer not just the "easiest" option
    - occasionally the quick and easy fix is the right option
    - but more often the well considered and strategic solution is the best choice
- NEVER go more than 2-3 questions into an interactive session before re-evaluating your context
::block when="ctx.is_monorepo"
- ALWAYS consider **package** ownership when thinking about solutions
    - its easy to have a {{ actor }} describe a problem as being associated with one package when the solution really should be completely or partially implemented in another package
    - for the long term development of this repo and to improve the reuse potential of the work you're proposing always 
::end-block
- ALWAYS present a set of **options** for a {{ actor }} to choose from:
    - for each option:
        - describe the option thoroughly
        - provide pros and cons for each option
        - describe any design goals that this option addresses well and how it achieves this
        - describe the complexity of implementing this option
        - describe the complexity based on 
    - always **recommend** one of the options and:
        - always describe WHY you choose this option
        - always describe any design goals you think this option does a good job of meeting
        - always mention any assumptions you were making when choosing this option

::block when="maturity == 'greenfield'"
> **Note:** this 

::end-block
