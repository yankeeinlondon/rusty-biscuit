

1. Can we propagate context to subagents?

   Yes, via the experimental.chat.system.transform hook in the bridge plugin. This hook fires for every LLM call (including subagent sessions) and can inject text into the system prompt. Adding something like "You are in a non-interactive session. Never ask questions." would propagate to all subagents automatically.

   The tool.execute.before hook is another option — it fires for input.tool === "task" and can mutate output.args.prompt before the subagent starts. This is more targeted (only affects subagents, not every LLM call).

2. Should we see subagent hooks/events?
   In hook mode (claudine handle): Yes. The bridge plugin's tool.execute.before fires for every tool including task, and permission.ask / question.asked fire on the event bus. Claudine receives these.
   
   In stream mode (claudine compose): Partially. The JSON stream shows tool_use/tool_result events for the task tool, but the stream parser currently maps ALL tool events to generic on_before_tool/on_after_tool — it doesn't distinguish task from other tools. Subagent-specific events (question.asked, permission.asked) do NOT appear in the NDJSON stream at all; they only fire on the internal event bus.

   In neither mode does Claudine currently emit SubagentStart/SubagentStop events — those are defined in the AgenticEvent enum but only Claude Code has native hook support for them. For OpenCode, they'd need to be inferred from tool_use/tool_result where tool_name == "task".
