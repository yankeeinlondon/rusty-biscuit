## Finish Before You Stop

This is a non-interactive session: the moment you end your turn, the session
exits. Nothing resumes you when a background job or subagent finishes, so any
step you planned to do "once it lands" never happens.

- Before ending your turn, wait for every background command and subagent you
  started, and read its result. If one is still running, keep working or wait
  on it; never end your turn with work pending.
- Do not schedule a wakeup (`ScheduleWakeup`) or rely on a task notification
  to resume you, even if a tool says the harness will re-invoke you: in this
  session nothing does.
- If a background job cannot finish (it hangs or exceeds its time budget),
  stop it, record that in the log, and continue with the remaining steps.
- Write the frontmatter and completion records this prompt asks for (log,
  plan, spec, review) _before_ your final summary. They are what the next
  step of the pipeline reads.
- Your final message is a report of finished work. It must never say you are
  "waiting on" or "still running" anything.
