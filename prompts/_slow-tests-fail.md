### A Slow Test Is a Failing Test

An L1 test over 5 s is a **failing test**, even when nextest says it passed
and the recipe exits 0. Every canonical recipe prints a line such as
`🐌 L1 run recorded: 185 of 11917 test(s) at or above 5.0s` and nextest marks
each one `SLOW`; a nonzero count means the test gate is **red** until each
slow test is cleared. Never report such a run as "all tests passing".

On a host under load (load average well above the core count in `uptime`)
the allowance is 10 s:

- a test at 10 s or longer fails, whatever it takes alone
- a test over 5 s but under 10 s is cleared only by re-measuring it alone at
  5 s or less; record both timings and the load average

- `target/slow-tests/current.md` (at the repository root) lists every slow
  test recorded for the working tree's current state; `just slow-tests fresh`
  exits 0 when it describes the tree as it is now
- a slow test is red wherever it lives, including in code this work did not
  touch
- measure each one alone (`-E 'test(=<name>)'`) and inside its package run:
  slow alone means the test itself is slow; fast alone but over the bar in the
  package run (over 5 s on a quiet host, 10 s or longer on a loaded one) means
  it competes for a shared resource such as process spawns or CPU. Both are
  defects; load the `rust-testing` skill (`test-speed.md`) for causes
  and fixes
- a `slow_` prefix, a `slow-timeout` override, `#[ignore]`, or removing cases
  is not a fix
- load is never an exemption beyond the 10 s allowance
