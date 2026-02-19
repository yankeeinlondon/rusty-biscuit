Review each document in @claudine/docs/hooks/ for correctness and completeness. You will need to do research on the given Agent CLI's website to make sure that you are completely current on the capabilities of that Agentic CLI before performing this task. In order to be efficient with context windows as well as be time efficient you should act as an orchestrator and have each document perform this operation on a per file basis (which maps to also being on a per Agentic CLI basis). Each subagent which you give responsibility to you will:

- tell the subagent which document they are responsible for (full filepath)
- tell them to do an overview read of the document first so they have an understanding of which Agentic CLI they are responsible for as well as some hints on the most useful URLs to check for documentation
- then tell them to do research online on the given Agentic CLI's documentation to make sure we have a complete picture of the current capabilities of this platform
- they should then update the document they are responsible for and summarize the changes they made when reporting back to you (the orchestrator)

Things you should always make sure the document contains are:

1. **Home Page:** The homepage URL for the Agentic CLI (this can be in the document body but you should also ensure that a frontmatter property called `homepage` is included with this info)
2. **Documentation:** The documentation URL which is the documentation page for this Agentic CLI (this can be in the document body but you should also ensure that a frontmatter property called `docs` has this URL too)
3. **Configuration:** The frontmatter of the document should have a URL which describes how to configure Hooks and the body of the document should then describe in detail where in the filesystem the "User scoped" and "repo scoped" configuration is located as well as provides some example configurations which demonstrate variations you might find for this provider.
4. **Events:** There should be a section in the body of the document called `## Hook Events` and it should enumerate every event that this Agentic CLI supports in their hooks support. Each event needs to have details on:

    - Description of the event
    - Event Payload: a detailed description of the type of data that will be sent when this event is fired
    - Event Response: a detailed description of the _return type_ that the event is supposed to return (payload and/or exit code)
    - Gotchas: the reviewer subagent should validate and update any existing references to "gotchas" (aka, problems other developers have had interacting with this event type) and solutions to those "gotchas" that might have been found to these issues. This will require doing online research to complete.
5. **Matcher System:** What if any "matcher system" does this Agentic CLI provide and in detail how does it work and how is it configured

6. **Sources:** the body of the document should conclude with a `## Sources` section which is a Markdown list of Markdown links to important reference material that was used in research or that is deemed valuable in some way to the topic of Hooks functionality for the given Agentic CLI
