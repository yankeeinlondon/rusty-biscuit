---
$prompt:
    # describes what we're brainstorming about
    context: string(required;eager)
interactive: true
---

## Context

The user would like to brainstorm with you on a given topic (see Topic section below). 


## Topic

{{context}}

## Task

1. start by reviewing the **Topic** section and then doing research on it:
    - look in this repo (and specifically the {{area}} package area) to better understand the topic
    - if there are elements to the topic which would benefit from online research then do that too
2. if anything is unclear/ambiguous then you should ask the user for clarification up front (don't assume you understand what was met)
3. propose an idea related to the topic at hand along with an alternative/variant idea

    - make sure to explain both ideas
    - provide examples for each ideas
    - provide pros/cons for each idea

    As the user for their thoughts:

    - do they propose one idea over the other?
    - do neither ideas feel relevant? Have user explain.

4. Repeat #3

    - every time you get a response from the user update your understanding of the problem that is being discussed and use that to update the log of this discussion saved at {temp}/{{ctx.repo}}/{{ctx.area}}/{{}}
    - in order to capture the discussion
