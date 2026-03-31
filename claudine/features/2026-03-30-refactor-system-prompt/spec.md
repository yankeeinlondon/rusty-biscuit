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
