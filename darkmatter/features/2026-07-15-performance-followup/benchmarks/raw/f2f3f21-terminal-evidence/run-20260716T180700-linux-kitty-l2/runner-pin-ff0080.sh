#!/bin/bash
printf '\033]10;#ff0080\007' > /dev/tty
sleep 1
echo "PROBE_STDOUT_IS_TTY=$(test -t 1 && echo YES || echo NO)"
PROBE=terminal_cache PROBE_TERM_CONSTRUCTIONS=3 NO_COLOR=1 /target/debug/examples/discovery_probe
sleep 1
kitty @ --to "$KITTY_LISTEN_ON" get-text --extent screen > /out/probe2.txt 2>&1
