## This Session Is Headless: Ending Your Turn Ends the Run

Nobody resumes this session. When you end your turn the process exits, any subagent still
working is abandoned, and every task you have not started is silently skipped, while the
run still reports success. Earlier runs failed exactly this way: the orchestrator sent a
follow-up to a subagent, ended its turn with "waiting on the follow-up", and the remaining
tasks were never attempted.

- run every subagent in the **foreground** and take its result in the same turn; never
  start one in the background
- for concurrency, launch several foreground subagents in a single message; that runs them
  in parallel and still returns every result before you continue
- never continue a subagent by messaging it (Claude Code's `SendMessage`); that resumes it
  in the background. For follow-up work, launch a **new foreground** subagent and give it
  what the previous one reported
- never end your turn while you are "waiting" on anything; waiting is not a final state
- end your turn only after the task's closing steps are done (final log entries, metadata
  updates, or final report). If something truly blocks you, say so in that closing output
  and record what is unfinished; do not end silently mid-task
