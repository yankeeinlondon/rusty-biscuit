
# Uses TTS if available but never error
_speak *args:
    @so-you-say "{{args}}" --background >/dev/null 2>&1 || exit 0

# play a sound effect (blocking)
_play effect:
    @playa effect "{{effect}}" >/dev/null 2>&1 || exit 0

_play_background effect:
    @playa effect "{{effect}}" --background >/dev/null 2>&1 || exit 0

# Send a desktop notification via `messenger` when it is on PATH. When
# `messenger` is unavailable (fresh checkouts, CI runners) this falls back
# to a plain stderr echo and still exits 0, so callers like `just test`
# never mask a real test exit code with a 127 "command not found".
_message msg:
    @command -v messenger >/dev/null 2>&1 && messenger send "{{msg}}" || echo "{{msg}}" >&2

_notify message title:
    #!/usr/bin/env bash
    if command -v osascript &> /dev/null; then
        osascript -e 'display notification "message" with title "{{title}}"'
    elif command -v notify-send &> /dev/null; then
        notify-send '{{message}}'
    fi
