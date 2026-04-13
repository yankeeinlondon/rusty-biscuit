1. Hot keys moved to dedicated status bar (bottom row)

    - mod.rs: Added a third layout chunk (Constraint::Length(1)) for the status bar at the very bottom
    - In Overview mode: shows Press ENTER to Configure only
    - In Detail mode: shows tab-specific hot keys with all uppercase letters
    - Status bar has subtle dark background (Color::Indexed(236)) and light text (Color::Indexed(250))

1. Removed inline help text from all tabs

      - preferences.rs, services.rs, tts.rs, actions.rs, messenger.rs: Removed the per-tab if is_detail { ... help ... } rendering blocks

1. Default Sounds reformatted as indented list

      - Changed from S: confirmation  A: doorbell  E: error-1 (single line)
      - To three indented lines:
        Default Sounds
          Success: confirmation
          Attention: doorbell
          Error: error-1

1. Sound preview hotkey

      - Added P key in the sound selector modal to play the highlighted sound effect
      - Plays in a background thread so it doesn't block the TUI
      - Modal now shows a P: Play preview hint at the bottom via new render_list_modal_with_hint() function

1. Dimmed text contrast improved

      - Replaced Color::DarkGray with Color::Gray across all tab files for better readability
