# Side Effects Feature

Up to now the primary focus in composition within claudine was to compose a Darkmatter document and make it into a
prompt we could pass into a Agentic CLI provider.

There were a few things one could consider a "side effect" of our agentic call like:

- Communication (TTS, messenger, desktop notifications, etc.)
- we allow "shell expansion" with the primary intent of _querying information_ to put into the prompt but the execution
  of the command _could_ have other side effects

In this feature, however, we'll offer a mechanism to explicitly create desirable side effects:

- side effects will always be configured in the Frontmatter of a document
- we will offer _side effects_ associated with various parts of the lifecyle:
    - start (_at the point that preflight checks have successfully completed and we're ready to start_)
    - success (_at the successful completion of the prompt_)
    - failure (_at the termination of a prompt which ended in the failure state_)
    - blocked (_at the termination of a prompt which ended because the preflight checks failed_)
    - next (_when a prompt file has a `next` property defined and all approvals have been achieved to continue_)
    - loop (_when a **loop** has completed and we're about to iterate into another loop's prompt_)

## How Side Effects are Defined

We provide an enumerated set of popular side effects you can use without needing any pre-flight approval:

- `ensureFile(file)`
    - tries to resolve the passed in file path using the normal `FileReference` rules
    - if file reference found then, then this is a no-op
    - if file wasn't found then the file is created as an empty file:
        - if the file reference is "multi-pathed" (like magic paths, or an implicit relative file) we will resolve to the most _localized_ variant
        - for example, the path `@prompts/foobar.md` could resolve to a file off
- `ensureDir(path)`
- `removeDocumentation(file | glob)`
    - allows the removal of documentation files (.md, .txt, .doc, .docx, .xls, .ppt, etc.) but only within:
        - the current repo
        - the `~/.claudine`, and agent user scoped directories (e.g., `~/.claude`, `~/.codex`, `~/.config/opencode`, `~/.qwen`, etc.)
- `removeData(file | glob)`
    - allows the removal of data files (.json, .toml, .yaml, .csv, .tsv, .xml)
- `removeImage(file | glob)`
- `set_frontmatter(...)`
    - two signatures:
        - `set_frontmatter(file, JSON)`
        - `set_frontmatter(file, prop, value)`
- `append_jsonl(file, content)`
- `append(file, content)`
- `post(url, payload)`

### Pre-Conditions
