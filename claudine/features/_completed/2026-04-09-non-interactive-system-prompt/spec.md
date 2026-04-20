We have recently refactored how system prompts are added. This is documented in [system-prompts](claudine/docs/topics/system-prompts.md). In the process of doing this, however, we left out an important detail which will be added as part of this feature:

- when the repo's `.claudine` directory has a `non-interactive.md` file then we will append this to the end of the system prompt (if there's already text to be appended) or create a system prompt with only this in it if there is not.
- if there is not a `.claudine/non-interactive.md` in the repo then we will look for `~/.claudine/non-interactive.md` to perform the same function. If neither are present then we will add a built-in fallback prompt that tells the provider not to request permission or user input, and not to run commands that would require an interactive terminal or follow-up stdin input.

This behavior is an important way to avoid having Agent's unknowingly ask for permissions, ask the user questions, or launch stdin-dependent commands in a non-interactive session and thereby hang the process.
