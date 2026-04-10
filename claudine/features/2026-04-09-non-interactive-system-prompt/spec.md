We have recently refactored how system prompts are added. This is documented in [system-prompts](claudine/docs/topics/system-prompts.md). In the process of doing this, however, we left out an important detail which will be added as part of this feature:

- when the repo's `.claudine` directory has a `non-interactive.md` file then we will append this to the end of the system prompt (if there's already text to be appended) or create a system prompt with only this in it if there is not.
- if there is not a `.claudine/non-interactive.md` in the repo then we will look for `~/.claudine/non-interactive.md` to perform the same function. If neither are present then we will simply add: `\n**IMPORTANT:** this is a non-interactive prompt; do not request permission or ask the caller questions!\n`

This behavior is an important way to avoid having Agent's unknowingly ask for permissions or ask the user questions in a non-interactive session and thereby hang the process.
