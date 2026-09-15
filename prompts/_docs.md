---
$schema:
    reusable: boolean -> a flag to indicate that this prompt is intended to be a reusable snippet
    stage: enum(design,implementation,fix,review) -> allows a caller to specify the stage of the lifecycle they are in
    docs_root: string(required)
    spec: string(required) -> the default filename spec's get in this repo
reusable: true
docs_root: "docs"
spec: "{{ env.SPEC_FILE || 'spec.md' }}"
design: "{{ env.DESIGN_FILE || 'design.md' }}"
---
## Best Practices for Documentation in **{{ title_case(ctx.repo) }}**

We use the term "documentation" to represent all forms of written knowledge:

- the documentation for this repo will be 99% Markdown content but there may be cases where some other form of documentation is relevant
- however, it's worth discussing the various "sources" this documentation comes from and what each source has to offer
- while evaluating sources we'll try to address the following questions:

    - _when_ should I use this knowledge source?
    - _what_ kind of information is appropriate to be saved here?
    - _what_ sources are actually worth avoiding?
    - _where_ will I find these resources?

### Knowledge Sources

There are five _sources_ of knowledge that Agents can draw deep knowledge from (and one that should be avoided):

1. System Prompt

    - **Claudine** composed prompts allow for the system prompt to be replaced or appended to.
    - this is done by adding a `system-prompt.md` to the root of the repo (or to the root of a package or package-area in a monorepo)

    ::block when="file_exists(^system-prompt.md)"
    - this repo DOES take advantage of this

    The system prompt is an "automatic" ... when an agent starts they start with this prompt. This means it's very important that it never contradicts other forms of evidence nor is it overly specific about matters which 
    ::end-block
    ::block when="!file_exists(^system-prompt.md)"
    - this repo DOES NOT take advantage of this feature
    ::end-block

2. `CLAUDE.md` / `AGENTS.md`

    ::block when="file_exists(!CLAUDE.md) && file_exists(!AGENTS.md)"
    ::block when="is_symbolic_link(!CLAUDE.md) || is_symbolic_link(!AGENTS.md)"
    This repo has a both a `AGENTS.md`
    ::end-block

    Having an `AGENTS.md` file at the root of a repo -- or in a user's home directory -- is a convention that all agents share. It means that there is at least great likelihood that this file will be consulting when solving problems.

3. Agent Skills

    ::block when="has_skill(ctx.repo) || has_skill(ctx.area)"
    All modern agents support the concept of **Agent Skills** and this is a highly valuable way for agent's to gather important information about various topics.

    - all modern agents already know how to look for relevant skills
    - the topics of these skills are most typically going to be surround external dependencies
    - it's also not that uncommon for a repo skill to be provide knowledge around the repo itself (or a package or package area in that repo)
        - this can be very useful but when doing this we must be careful not to introduce redundancies
        - fortunately Darkmatter's _composition_ functionality allows us to avoid this when structured correctly
        - read [building repo skills](^prompts/_repo-skills.md) for details on how to do this
    ::end-block

    ::block when="!has_skill(ctx.repo) && !has_skill(ctx.area)"
    ::block when="ctx.is_monorepo"
    - sometimes a monorepo may provide an Agent Skill to help with with the repo
    - look for skills that hold the name of the package or package area you are working on
    ::end-block
    ::block when="!ctx.is_monorepo"
    - sometimes a repo may provide an Agent Skill to help with with the repo
    ::end-block
    ::end-block
    ::end-block

    ::block when="has_skill(ctx.area) && ctx.is_monorepo"
    - the caller started this agent in the "{{ctx.area}}" {{ctx.area_description}} which means that using the "ctx.area" agent skill is almost surely a good idea during your task
    - this repo often -- _but not always_ -- will create an Agent Skill that holds the same name as a package or package-area in the repo.
    - when you find yourself working on 
    ::end-block

4. Feature and Fix Specifications

    The primary mechanism of describing how you want to _change_ the functionality in a repo is typically done via a specification file. This change might be new functionality, it might be fixing something that was supposed to or used to work, it might be something in-between.

    In this repo we group specifications into two groups:

    - **Features** - _specs that focus more on new or changed functionality then fixing broken functionality (found in `features` dir)_
    - **Fixes** - _specs that focus primarily on fixing things (found in `fixes` dir)_

    There is no need to be overly precious on whether a specification is a "feature" or a "fix", usually it's pretty clear but there's no real harm if a specification is miscategorized. What's important is that you know where to find them and the naming conventions used:

    ::block when="ctx.is_monorepo"
    - this repo is a monorepo so the `features` or `fixes` directories which provide the root directory for these specifications can be at the repo root but they can -- and often are -- at the package area root or package root.
    - `features` and `fixes` directories of the repo root indicate work that is naturally aligned to a single package area or package
    ::end-block
    ::block when=!ctx.is_monorepo"
    - the `features` and `fixes` directories will be found off of the repo's root directory
    ::end-block
    - every feature/fix is given it's own directory off of "features"/"fixes" that has a name that leads with the date it was authored (YYYY-DD-MM) but includes a name as well; an example of a well structured name would be: `2026-09-09-do-it`
    - the name should be 2-5 words (dasherized) that suggest what work is being done in the spec
    - inside each feature/fix folder is the required specification file: `{{spec}}`
    - in some cases you may find a technical design file ({{design}}) in the same directory



5.
