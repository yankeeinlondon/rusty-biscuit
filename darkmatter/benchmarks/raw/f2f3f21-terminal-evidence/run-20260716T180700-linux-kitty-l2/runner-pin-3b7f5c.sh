#!/bin/bash
{
  uname -a; kitty --version
  echo "KITTY_PID=${KITTY_PID:-UNSET} KITTY_WINDOW_ID=${KITTY_WINDOW_ID:-UNSET}"
  echo "TERM=${TERM:-UNSET} TERM_PROGRAM=${TERM_PROGRAM:-UNSET}"
  echo "TMUX=${TMUX:-UNSET} ZELLIJ=${ZELLIJ:-UNSET} STY=${STY:-UNSET} CI=${CI:-UNSET}"
  echo "stdout_is_tty=$(test -t 1 && echo YES || echo NO)"
} > /out/meta.txt 2>&1

printf '\033]10;#3b7f5c\007' > /dev/tty
sleep 1
# stdout stays on kitty's REAL tty -- no redirect, no script
PROBE=terminal_cache PROBE_TERM_CONSTRUCTIONS=3 NO_COLOR=1 /target/debug/examples/discovery_probe
echo "probe_exit=$?" >> /out/meta.txt
sleep 1
kitty @ --to "$KITTY_LISTEN_ON" get-text --extent screen > /out/probe.txt 2>/out/gettext.err
echo "gettext_exit=$?" >> /out/meta.txt
