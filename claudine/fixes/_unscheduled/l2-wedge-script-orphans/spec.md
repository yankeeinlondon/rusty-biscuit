# The L2 loop-wedge test leaves an immortal shell loop behind on every run

`level2_double_ctrl_c_force_exits_loop_wedged_between_iterations`
(`cli/tests/level2/level2_wrap_ctrl_c_loop_wedge_tmux.rs`) writes a
`wedge.sh` that traps `INT`, `TERM`, and `HUP` and then runs
`while : ; do /bin/sleep 1; done`. The test passes when Claudine's second
Ctrl+C force-exits the wrapper with code 130, then it kills the tmux session
and drops the `tempdir`. Nothing kills the wedge. It is reparented to
`launchd` (PID 1), ignores the `HUP` from the session teardown, and keeps
running after its temp directory has been deleted.

## Evidence

On 2026-10-16 the macOS dev host had 46 orphaned
`/bin/sh /var/folders/…/T/.tmp*/wedge.sh` processes. All had PPID 1, the
oldest had been running for about 18 days, and each started a fresh
`/bin/sleep 1` every second. They were found while a full claudine
`just test` run was crawling at about 30 tests/min under a load average
of about 200. After `kill -KILL` on the 46 orphans, the same run went to
about 450 tests/min. Other load was also changing at the time, so the
speedup is not all due to the orphans, but the process churn they cause is
real and grows with every L2 run.

## Reproduction

```sh
cd claudine && just test-l2   # with tmux available
ps -A -o pid,ppid,etime,command | grep '[w]edge\.sh'
```

Expected: no `wedge.sh` process after the run. Actual: one new orphan
(PPID 1) per run of the test.

## Open question

The test makes the wedge ignore signals on purpose: only Claudine's
second-press force exit can end the run. Is it intended that this force exit
leaves the lifecycle `::shell` child running? Shell actions are supposed to
run in owned process trees. If the force exit should tear that tree down,
this is a product defect, and the orphan is the evidence for it. If leaving
it running is accepted, the defect is only in the test fixture. Decide this
before choosing the fix below.

## Closes when

- The test reaps its wedge whatever the outcome (pass, fail, or panic).
  For example, the wedge writes its PID to the workspace, and a drop guard
  sends that PID `SIGKILL`. Or the loop exits when its workspace directory
  disappears (`while [ -d '<workspace>' ]`).
- If the open question rules that the force exit must tear down owned
  shell trees, Claudine does so, and the test asserts the wedge PID is gone
  after the 130 exit.
- A repeated `just test-l2` leaves no `wedge.sh` process behind.
