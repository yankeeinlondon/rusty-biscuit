
# Uses TTS if available but never error
_speak *args:
    @so-you-say "{{args}}" --background >/dev/null 2>&1 || exit 0

# play a sound effect (blocking)
_play effect:
    @playa effect "{{effect}}" >/dev/null 2>&1 || exit 0

_play_background effect:
    @playa effect "{{effect}}" --background >/dev/null 2>&1 || exit 0

_message msg:
    @messenger send "{{msg}}"

_notify message title:
    #!/usr/bin/env bash
    if command -v osascript &> /dev/null; then
        osascript -e 'display notification "message" with title "{{title}}"'
    elif command -v notify-send &> /dev/null; then
        notify-send '{{message}}'
    fi
