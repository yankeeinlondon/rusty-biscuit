/claudine act as an orchestrator and create a subagent for every markdown file in @claudine/docs/protect

1. ask the subagents to read the file they'll be assigned to by passing them the filename to the file and ask them to use the frontmatter's `prompt` property as a prompt. This will be all the instruction they need. Ask them to provide a brief summary of the documentation they created and to save an updated version of the document.
2. when all subagents are completed let the user know that the "Research Stage" has completed and then kick off another set of subagents, again one subagent per Markdown file, this time you'll provide a filepath to the document that the subagent will be responsible for and tell them to use the frontmatter's `closure` property as a prompt for their task. In addition pass them a filepath to the the @claudine/docs/protect/schema.ts file and let them know that the file they are responsible will have the frontmatter properties defined in this Typescript type definition.
3. when all subagents are completed let the user know that the "Closure Stage" has completed and that the overall task has now completed successfully

