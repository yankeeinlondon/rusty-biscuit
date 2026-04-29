# System Prompt Refactor

Being able to provide the ability to _append_ or _replace_ the system prompt is a powerful feature. Claudine already currently support this to some degree by providing a universal CLI switch instead of requiring the user to remember what each Agent's CLI switch is.

Our goal in this feature is to implement and/or refactor so that Claudine can:

- provide two key CLI switches for system prompts:
    - `--append-system-prompt <file>` (abbreviated alias of `--asp`)
    - `--replace-system-prompt <file>` (abbreviated alias of `--rsp`)
- these CLI switches should be used when you want to point to a "non-standard" file for the system prompt
- the "standard file" for system prompts in Claudine -- something new with this feature -- is `system-prompt.md`
- there are several locations where Claudine will look for a `system-prompt.md`; the general hierarchy of resolution is:
    - at the root of the _package_ that the user started Claudine in 
    - at the root of the _package area_ that the user started Claudine in
    - at the root of the _repo_ which the user started Claudine in
    - at `~/.claudine/system-prompt.md` (note: this is not typically recommended to populate)

**Note:** if a user calls Claudine with one of the CLI switches then no attempt will be made to resolve a `system-prompt.md` in one of the standard directories.

## Composition

Any `system-prompt.md` file is allowed to add Darkmatter directives and _before_ the system prompt is sent into the Agent we will run a Darkmatter [compose pipeline](@darkmatter/docs/darkmatter-compose-pipeline.md) on this document.

This allows for a lot of additional flexibility such as:

- a more detailed `system-prompt.md` in a package area or package could use [transclusion](@darkmatter/docs/topics/transclusion.md) to bring in the broader context that was defined in the repo root's 
- a more detailed `system-prompt.md` that just wanted to ensure that NO system prompt was used could add the `system-prompt.md` with an empty body (this is an explicit indication that we should NOT send in a system prompt)
- a `system-prompt.md` could leverage a targeted shell command to ensure the system prompt is always 100% current.
- you could conditional blocks in the system prompt based on ENV or context state

## Output Format

We need to determine a "default" output format to use for the system prompt. It might be just the Markdown "as is" but we may wrap this Markdown in an XML tag, or possibly even some other format. The design document will leverage the research we did in @claudine/docs/research/system-prompt to determine what the default output is and whether we need to have multiple formats based on something.
