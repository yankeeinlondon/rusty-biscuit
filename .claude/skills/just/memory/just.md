# just — learned context

## Ctrl-C in fan-out loops (`just test`, `just build`, etc.)

A recipe that loops over sub-areas with `if (cd "$area" && just test); then … else … fi`
does **not** abort on Ctrl-C — it silently continues to the next area.

Root cause (bash, not just): bash only auto-aborts a script on SIGINT when the
foreground child is *killed by* the signal (`WIFSIGNALED`). `just`/`cargo`/`nextest`
trap SIGINT, shut down gracefully, and exit *normally* with code 130. Bash then sees
an ordinary non-zero exit, the surrounding `if` swallows it, and the loop marches on.
A `sleep` (which dies by the signal) would abort — so a minimal repro with `sleep`
will NOT reproduce the bug; the child must catch SIGINT to reproduce it.

Fix: install an explicit trap in the recipe body:

```bash
trap 'trap - INT; echo; print_summary; exit 130' INT
```

The parent receives SIGINT too (whole foreground process group), so with a trap
installed bash runs the handler after the current child returns → the loop stops.
Reset the trap first (`trap - INT`) so a second Ctrl-C hard-kills.

To reproduce/verify faithfully without a TTY: run the script under `set -m` so the
backgrounded job becomes its own process-group leader, then `kill -INT -"$pgid"`
to mimic a terminal Ctrl-C to the foreground group.
