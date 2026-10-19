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

## Subagents Verify Their Slice; You Verify the Phase

Parallel subagents share one host. A package area's full `just test` is about ten thousand
tests. Three subagents each running it at once pushed the host's load average to about
200, and timing-sensitive tests then failed or timed out. Each failure caused another full
rerun, which added more load. A 30–40 minute phase took over three hours that way, and
every timeout passed when it was rerun on its own.

- a subagent's verification is **scoped to what it changed**: the test files it added or
  edited, plus the modules and tests that exercise the code it touched, run with a nextest
  filter (`-E 'test(...)'`) or the area recipe's positional filter. Write those exact
  commands into the subagent's prompt
- never give a subagent the phase's completion gates. The area's full `just test`,
  `just lint`, and `just check-tier-coverage` run **once per phase**, by you, after every
  subagent's work is merged, and while no subagent is running
- a subagent may run `just lint` (it is cheap) but not a full `just test`, and never
  `--no-fail-fast`
- when a test fails, rerun **that test alone** before rerunning anything wider. If it passes
  alone, record it as load-sensitive in the log rather than rerunning the suite
- wait on long commands with a background task and its completion notice, never with
  `sleep` polling. A `sleep 600` costs ten minutes even when the command finished after one
- if the host is loaded while your subagents are running, assume they are the cause
  before blaming anything else on the host
