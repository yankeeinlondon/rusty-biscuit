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

    ::block when="file_exists('^system-prompt.md')"
    - this repo DOES take advantage of this

    The system prompt is automatic: every agent starts with it. It must therefore never contradict the other sources below, and it must stay general; anything specific to one package, one workflow, or one point in time belongs in a skill or a `{{docs_root}}/` page, not here.
    ::end-block
    ::block when="!file_exists('^system-prompt.md')"
    - this repo DOES NOT take advantage of this feature
    ::end-block

2. `CLAUDE.md` / `AGENTS.md`

    ::block when="file_exists('^CLAUDE.md') && file_exists('^AGENTS.md')"
    ::block when="is_symbolic_link('^CLAUDE.md') || is_symbolic_link('^AGENTS.md')"
    This repo has both files, and one is a symbolic link to the other, so every agent reads the same conventions whichever name it looks for.
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
    - the caller started this agent in the "{{ctx.area}}" {{ctx.area_description}}, which means the "{{ctx.area}}" agent skill is almost surely worth loading for your task
    - this repo often -- _but not always_ -- creates an Agent Skill with the same name as a package or package area
    - when you find yourself working in a different package area than the one you started in, look for that area's skill too
    ::end-block

4. Feature and Fix Specifications

    The primary mechanism of describing how you want to _change_ the functionality in a repo is typically done via a specification file. This change might be new functionality, it might be fixing something that was supposed to or used to work, it might be something in-between.

    In this repo we group specifications into two groups:

    - **Features** - _specs that focus more on new or changed functionality then fixing broken functionality (found in `features` dir)_
    - **Fixes** - _specs that focus primarily on fixing things (found in `fixes` dir)_

    There is no need to be overly precious on whether a specification is a "feature" or a "fix", usually it's pretty clear but there's no real harm if a specification is miscategorized. What's important is that you know where to find them, the naming conventions used, and what a spec is **for**:

    - **a spec is a snapshot in time.** It records what was decided, why, and what the implementation must achieve, as of its date. Once its review cycle closes it moves to `_completed` and is not maintained. It is never the place to learn how something behaves *now*; that is the `{{docs_root}}/` tree (source 5 below)
    - a spec can be dense, because its readers were part of the decision; that is fine for a spec and wrong for a doc
    - **a spec may link to a `{{docs_root}}/` page; a `{{docs_root}}/` page never links to, or names, a spec or fix**, by path or by `{date}-{name}`. A doc that says "tracked in `2026-…`" has delegated its content to a snapshot that will move and go stale
    - when an implementation departs from its spec, correct the docs and record the departure in the implementation log; leave the spec saying what was decided

    ::block when="ctx.is_monorepo"
    - this repo is a monorepo so the `features` or `fixes` directories which provide the root directory for these specifications can be at the repo root but they can -- and often are -- at the package area root or package root.
    - `features` and `fixes` directories of the repo root indicate work that is naturally aligned to a single package area or package
    ::end-block
    ::block when="!ctx.is_monorepo"
    - the `features` and `fixes` directories will be found off of the repo's root directory
    ::end-block
    - every feature/fix is given its own directory off of "features"/"fixes" that has a name that leads with the date it was authored (YYYY-MM-DD) followed by a name; an example of a well structured name would be: `2026-09-09-do-it`
    - refer to a feature/fix by that `{date}-{name}` alone, never by a path that includes `_unscheduled` or `_completed`, because those directories record where a spec is in its life and a reference that spells them out goes stale when it moves
    - the name should be 2-5 words (dasherized) that suggest what work is being done in the spec
    - inside each feature/fix folder is the required specification file: `{{spec}}`
    - in some cases you may find a technical design file ({{design}}) in the same directory

5. The `{{docs_root}}/` tree

    **This is the current record of how the repo behaves.** Where a spec says what was decided on a date, a `{{docs_root}}/` page says what is true today, and it changes in the same commit as the behavior it describes.

    ::block when="ctx.is_monorepo"
    - each package area keeps its own `{{docs_root}}/` directory; the repo root's `{{docs_root}}/` holds what crosses areas
    ::end-block
    - **the audience is a developer with no experience of this repository.** Lead with what the reader can do, give a compact example for each rule, and use a Mermaid diagram wherever a flow, lifecycle, or state change is easier to see than to read. A page that only a repo veteran can follow is not finished
    - **planned behavior is documented before it is built.** When a decision is made but the code has not landed, write the behavior into the page and mark it **planned**; the change that lands the code removes the marker. A marker that outlives its implementation is a defect of that change
    - a page that describes behavior that no longer exists, or omits behavior that does, is a defect of the change that made it so; fix it in that change, not later
    - a page never points at a spec or fix for its content (see source 4); it states the behavior, or the known defect, in its own words

### The source to avoid

Per-project **agent memory** files. They load into the orchestrating agent's context but not into its subagents', so agents working the same task act on different facts. Anything worth remembering goes into an Agent Skill (visible to every agent, version-controlled, reviewable) or, for a repo-wide convention that fits no topic, into `CLAUDE.md`.
