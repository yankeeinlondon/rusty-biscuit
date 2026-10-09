---
$schema:
    description: string(required)
    apps_supporting: string[](required)
prompt: |-
    ConEmu defines a family of `OSC 9 ; <n> ; ...` sub-commands (9;1 through 9;12, e.g., 9;9 to report the current working directory, which Windows Terminal also uses). The 9;4 progress sub-command is researched separately in `osc-9-4-protocol.md`, so cover every other sub-command here.

    Your task is to research this informal standard and:

    - provide a detailed overview of the standard.
        - How it works.
        - Whether it can be tested for.
        - What does each sub-command do, and which of them have been adopted outside ConEmu?
        - What visual representation, if any, does each adopted sub-command provide? How does that vary by terminal app?
        - How do these sub-commands conflict with iTerm2's `OSC 9 ; <text>` notification, and how can a program avoid that?
        - Where and when it was introduced? How well it is supported? Is there continuing adoption?
    - which terminal apps support it currently?
        - Are there any known quirks for any of the apps that a developer should be aware of? How can these quirks be worked around?
        - Note: be sure to include _at least_ the following terminal apps:
            - Wezterm
            - Ghostty
            - iTerm2
            - cmux
            - kitty
            - konsole
            - AppleTerminal
            - Alacritty
            - Contour
            - Foot
            - GNOME Terminal
            - more is better so add others which have readily available info

    Once you've completed your research and have written the prose content to the body of this document, you will need to update the Frontmatter properties:

    - set `apps_supporting` as a list of those apps which are known to support ConEmu's OSC 9 sub-commands
    - set `description` as a one to two sentence description of this standard
---
