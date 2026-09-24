Kitty L2 leg (plan Wave 3: L2 on WezTerm, Kitty, tmux, Apple Terminal without focus).

This host has kitty installed but no remote-control instance, so every `*_in_kitty` test skips
in `just test-l2` (test-l2.log). For this leg a background instance was started without taking
focus (`open -g -n -a kitty --args -o allow_remote_control=yes --listen-on unix:/tmp/kitty-w2p4
--start-as=minimized`) and stopped afterward.

  after (moved tree):          25 run, 5 passed, 20 failed   (test-l2-kitty.log)
  before (/tmp/w2-base, 5a396aeb5, unmigrated): 25 run, 6 passed, 19 failed   (test-l2-kitty-base.log)

The failures are the environment, not the move: the minimized instance gives a pane a few columns
wide (`bef-` / `o-` / `r-` wrapping) and no graphics detection, and the unmigrated base fails the
same way with an overlapping, run-to-run varying set. So Kitty gives no discriminating evidence on
this host. Identity and tier for all 25 Kitty tests are proven by `comparison-darwin.md`.
