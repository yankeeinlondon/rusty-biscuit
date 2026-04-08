# Config TUI Bug Fix Plan

**Date:** 2026-04-08
**Scope:** Fix all bugs, inconsistencies, and missing functionality in the claudine config TUI
**Files in scope:**
- `claudine/cli/src/commands/config_tui/mod.rs`
- `claudine/cli/src/commands/config_tui/app.rs`
- `claudine/cli/src/commands/config_tui/widgets/modal.rs`
- `claudine/cli/src/commands/config_tui/widgets/toggle.rs`
- `claudine/cli/src/commands/config_tui/tabs/preferences.rs`
- `claudine/cli/src/commands/config_tui/tabs/services.rs`
- `claudine/cli/src/commands/config_tui/tabs/tts.rs`
- `claudine/cli/src/commands/config_tui/tabs/actions.rs`
- `claudine/cli/src/commands/config_tui/tabs/messenger.rs`

---

## Phase 1: Toggle & Label Consistency Audit

**Goal:** Every toggle-like UI element uses the same `Toggle` widget. Every label with a value uses `Label:` (colon + space). Fix the Protect rendering and sweep the entire UI for similar drift.

### 1.1 Convert Protect to use Toggle widget

**File:** `services.rs:24-50`

The Protect rendering is hand-built with `Line::from(vec![...])` using spaces instead of a colon, no "On/Off" pattern, and an extra "(enabled/disabled)" suffix. It looks nothing like the Logging toggle one line above.

**Change:**
- Replace the manual Protect rendering (lines 24-49) with `Toggle::new("Protect", protect_enabled, is_detail)` rendered into `chunks[2]`
- Remove the `protect_status` variable and the custom `protect_line` construction entirely
- The Toggle widget already handles Bold label, `:`, On/Off with Green/Red coloring

**After:**
```
Logging:  On / Off
                        <-- blank row (chunks[1])
Protect:  On / Off
```

### 1.2 Full consistency audit across all tabs

Sweep every tab renderer for these patterns and verify consistency:

| Pattern | Expected | Check |
|---------|----------|-------|
| Bold label + value | `Label: [value]` or `Label: value` | preferences.rs, messenger.rs |
| Section headings | Bold, with colon + no bracket | "Canonical Sources:", "Default Sounds:", "Configurations:" |
| Indented sub-items | 2-space prefix `"  "` | preferences.rs canonical providers, sounds |
| Toggle on/off | Uses `Toggle` widget | services.rs Logging, services.rs Protect, tts.rs TTS |
| Value highlighting in Detail mode | Yellow fg | preferences.rs agent/providers, tts.rs provider |

**Known issues to also fix:**
- `preferences.rs:33` wraps agent name in `[brackets]` but provider values also use `[brackets]` -- this is fine, it's consistent within preferences
- `messenger.rs:57` uses `Active: [name]` -- consistent with preferences, OK
- `tts.rs:63-64` indents Female/Male voices with 2 spaces -- this will be changed in Phase 5

---

## Phase 2: Sound Dialog Box Fixes

**Goal:** Fix the sound selector modal to have proper hot keys, pre-selection, scrolling, and visual separation.

### 2.1 Pre-select current sound when opening modal

**File:** `preferences.rs:220-237`

**Bug:** When opening the sound selector, `highlighted` is always `0`. It should be the index of the currently selected sound.

**Change:** In `handle_key`, when creating `ModalState::SoundSelector`, compute the highlighted index:

```rust
KeyCode::Char('1') => {
    let current = app.config.default_sounds.success.as_deref();
    let sounds = super::super::get_sound_effect_names();
    let highlighted = current
        .and_then(|name| sounds.iter().position(|s| *s == name))
        .map(|i| i + 1)  // +1 because index 0 is "(none)"
        .unwrap_or(0);    // 0 = "(none)" if no sound set
    app.modal = Some(ModalState::SoundSelector {
        category: SoundCategory::Success,
        highlighted,
    });
}
```

Apply the same pattern for `Char('2')` (Attention) and `Char('3')` (Error).

### 2.2 Fix scrolling -- use ListState with stateful rendering

**File:** `widgets/modal.rs:42-85`

**Bug:** The list modal renders items with manual highlight styling but does NOT use `StatefulWidget` / `ListState`. Ratatui's `List` widget only auto-scrolls to keep the selected item visible when rendered via `frame.render_stateful_widget(list, area, &mut state)` with `state.select(Some(highlighted))`.

Currently the list is rendered with `frame.render_widget(list, list_area)` (line 77) which means it always shows from the top -- items below the viewport are invisible and the selection disappears.

**Change:**

1. In `render_list_modal_with_hint`, switch to `render_stateful_widget`:

```rust
let mut state = ListState::default().with_selected(Some(highlighted));
let list = List::new(items)
    .highlight_symbol(">> ")
    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
frame.render_stateful_widget(list, list_area, &mut state);
```

2. Remove the per-item highlight styling from the `.map()` closure since `ListState` handles it:

```rust
let items: Vec<ListItem> = items
    .iter()
    .map(|item| ListItem::new(Line::from(Span::styled(item.as_str(), Style::default().fg(Color::White)))))
    .collect();
```

3. Add a `Scrollbar` widget to the right of the list area when `items.len()` exceeds `list_area.height`:

```rust
if items_count > list_area.height as usize {
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(None)
        .end_symbol(None);
    let mut scrollbar_state = ScrollbarState::new(items_count)
        .position(highlighted);
    frame.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
}
```

### 2.3 Revamp hot key bar in sound selector

**File:** `preferences.rs:196-203` and `widgets/modal.rs`

**Current:** Single-line hint `"P: Play preview"` in light gray. Doesn't stand out.

**Change:**

1. Add a new function `render_list_modal_with_hotkeys` in `modal.rs` (or extend `render_list_modal_with_hint` to accept structured hotkey pairs instead of a plain string):

```rust
pub fn render_list_modal_with_hotkeys(
    frame: &mut Frame,
    parent_area: Rect,
    title: &str,
    items: &[String],
    highlighted: usize,
    hotkeys: &[(&str, &str)],  // (key, description) pairs
)
```

2. The hotkey bar layout:
   - A blank row separating the list from the hotkey bar
   - Hotkey bar gets a distinct background: `Style::default().bg(Color::Indexed(236))` (dark gray bg, matching the main tab hotkey bar concept)
   - Uses the same `build_hotkey_line` styling as the main content area: Yellow+Bold keys, light gray descriptions, pipe separators

3. Update the sound selector in `preferences.rs` to use dynamic hotkeys:

```rust
let category_label = match category {
    SoundCategory::Success => "Success",
    SoundCategory::Attention => "Attention",
    SoundCategory::Error => "Error",
};
let hotkeys = vec![
    ("P", "Play Sound"),
    ("ENTER", "Select"),
    ("ESC", "Exit"),
    ("D", &format!("Default for {category_label}")),
];
```

4. Handle the `D` key in `handle_sound_selector_modal` -- this should set the sound as the default for the given category and close the modal (same behavior as Enter).

### 2.4 Add `D` key handler for sound selector

**File:** `preferences.rs:345-395`

Add a handler for `KeyCode::Char('d') | KeyCode::Char('D')` in `handle_sound_selector_modal` that behaves identically to `KeyCode::Enter` -- it selects the currently highlighted sound and assigns it to the category. This provides a convenient shortcut.

---

## Phase 3: Exit Message

**Goal:** Replace the current plain `eprintln!` messages with styled, informative output.

### 3.1 Store git info in App for exit message

**File:** `mod.rs:27-53` and `app.rs`

Currently `git_info` is used only to detect `is_in_repo` and find `repo_config_path`. The exit message needs the repo name and branch.

**Change:**
- Add fields to `App`: `pub repo_name: Option<String>`, `pub branch_name: Option<String>`
- Set them from `git_info` in `run()`:
  ```rust
  let repo_name = git_info.as_ref().and_then(|g| g.repo.clone());
  let branch_name = git_info.as_ref().and_then(|g| g.current_branch.clone());
  ```
- Pass to `App::new(...)` (extend the constructor)

### 3.2 Implement styled exit messages

**File:** `mod.rs:74-89`

Replace the current exit block with:

```rust
// After leaving alternate screen...

if app.dirty || app.repo_dirty {
    // Save configs (existing logic)
    if app.dirty {
        claudine::dispatch::loader::save_claudine_config(&app.config, &config_path)?;
    }
    if app.repo_dirty {
        // existing repo save logic...
    }

    // Styled output
    eprintln!();
    eprintln!("\x1b[1mClaudine\x1b[0m configuration was updated:");
    if app.dirty {
        eprintln!("- The \x1b[1mUser\x1b[0m configuration was saved to \x1b[34m~/.claudine/config.json\x1b[0m");
    }
    if app.repo_dirty {
        if let (Some(name), Some(branch)) = (&app.repo_name, &app.branch_name) {
            eprintln!(
                "- The \x1b[33m{name}\x1b[0m(\x1b[2m{branch}\x1b[0m) \x1b[3mrepo configuration\x1b[0m was saved to \x1b[34m./.claudine/config.json\x1b[0m"
            );
        }
    }
    eprintln!();
} else {
    eprintln!();
    eprintln!("No changes were made to the \x1b[1mClaudine\x1b[0m configuration.");
    eprintln!("If you want to view the configuration, they are located at:");
    eprintln!("    - \x1b[1mUser\x1b[0m configuration is found in \x1b[34m~/.claudine/config.json\x1b[0m");
    if app.is_in_repo {
        eprintln!("    - \x1b[1mRepo\x1b[0m config is found at \x1b[34m./.claudine/config.json\x1b[0m off the repo's root directory");
    } else {
        eprintln!("    \x1b[2m\x1b[3m- \x1b[1mRepo\x1b[0m\x1b[2m\x1b[3m config is found at \x1b[34m./.claudine/config.json\x1b[0m\x1b[2m\x1b[3m off the repo's root directory\x1b[0m");
        eprintln!("    \x1b[2m\x1b[3m- because you are not in a repo currently no repo based configuration options were presented\x1b[0m");
    }
    eprintln!();
}
```

**Note:** Consider using `biscuit-terminal` Prose rendering if available from the CLI context, otherwise raw ANSI is fine since we're on stderr after leaving the alternate screen.

---

## Phase 4: Actions Tab Fixes

### 4.1 Change Enter key to be the primary Edit action

**File:** `actions.rs:164-217` and `mod.rs:209-219`

**Bug:** The `E` key is labeled "Edit" but the primary list navigation uses arrow keys. Enter should be the "Edit" (add action to event) key since the user is navigating a vertical list.

**Changes:**

1. In `actions.rs:handle_key`, add `KeyCode::Enter` handler that does what `KeyCode::Char('e')` currently does:

```rust
KeyCode::Enter => {
    if configured_count > 0 {
        // Same logic as current 'e' handler
        let mut event_keys: Vec<AgenticEvent> = ...;
        // ... open ActionTypeChooser
    }
}
```

2. Remove the `KeyCode::Char('e') | KeyCode::Char('E')` handler entirely.

3. Update hotkey bar in `mod.rs:209-219`:
   ```rust
   app::Tab::Actions => {
       let configured_count = ...;
       pairs.push(("A", "Add Event"));
       if configured_count > 0 {
           pairs.extend([("ENTER", "Edit"), ("D", "Delete")]);
       }
   }
   ```

### 4.2 Fix delete confirmation dialog styling

**File:** `actions.rs:93-121`

**Current:** Plain white text with lowercase `y: confirm | Esc: cancel`, no vertical balance, not centered.

**Change:** Replace the dialog content rendering with properly styled, centered, balanced content:

```rust
super::super::widgets::modal::render_modal(
    frame,
    area,
    "Confirm Delete",
    40,
    25,  // slightly taller for balance
    |frame, area| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // top padding
                Constraint::Length(1), // message
                Constraint::Length(1), // blank
                Constraint::Length(1), // hotkeys (centered)
                Constraint::Length(1), // bottom padding
                Constraint::Min(0),
            ])
            .split(area);

        let msg = Paragraph::new(format!("Delete all actions for {}?", event_name))
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        frame.render_widget(msg, chunks[1]);

        // Styled hotkey line, centered
        let hotkey_line = Line::from(vec![
            Span::styled("Y", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(": Confirm", Style::default().fg(Color::Indexed(250))),
            Span::styled("  │  ", Style::default().fg(Color::Indexed(240))),
            Span::styled("N", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(": Cancel", Style::default().fg(Color::Indexed(250))),
        ]);
        let hotkey_widget = Paragraph::new(hotkey_line).alignment(Alignment::Center);
        frame.render_widget(hotkey_widget, chunks[3]);
    },
);
```

Also add `N` key support in `handle_confirm_delete_modal`:
```rust
KeyCode::Char('n') | KeyCode::Char('N') => {
    app.modal = None;
}
```

### 4.3 Fix action display format in main list

**File:** `actions.rs:34-62`

**Current:** `Agent Started  SoundEffect(attention), Message` -- event name bold+yellow, summary gray+italic, separated by spaces.

**Required:**
- Event name: bold (not italic)
- Separator: `: ` (colon + space) between event and actions
- Action config: italic + dimmed
- Truncation strategy for long text values

**Change the ListItem construction:**

```rust
ListItem::new(Line::from(vec![
    Span::styled(
        event.human_name(),
        if i == app.list_index && is_detail {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        },
    ),
    Span::raw(": "),
    Span::styled(
        action_summary,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
    ),
]))
```

**Fix truncation in `summarize_actions`:** Add a `max_width` parameter and truncate the final joined string. Also improve individual action summaries:

```rust
fn summarize_actions(actions: &[HookAction], max_width: usize) -> String {
    let summaries: Vec<String> = actions.iter().map(|action| match action {
        HookAction::SoundEffect { effect, .. } => format!("Sound({effect})"),
        HookAction::Speak { message, .. } => {
            let preview = truncate_str(message, 20);
            format!("Speak(\"{preview}\")")
        }
        HookAction::Message { message, .. } => {
            let preview = truncate_str(message, 20);
            format!("Message(\"{preview}\")")
        }
        HookAction::Bash { command, .. } => {
            let preview = truncate_str(command, 20);
            format!("Shell(\"{preview}\")")
        }
        HookAction::Report { .. } => "Report".to_string(),
        HookAction::Call { command, .. } => format!("Call({command})"),
        _ => action.type_pascal_case().to_string(),
    }).collect();
    let joined = summaries.join(", ");
    truncate_str(&joined, max_width)
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max])
    } else {
        s.to_string()
    }
}
```

Pass a reasonable `max_width` based on the available area width minus the event name length.

### 4.4 Message action needs text input

**File:** `actions.rs:273-314` and `app.rs`

**Bug:** When adding a Message action, it creates a default `HookAction::Message { message: "{{event}} fired", image: None }` without prompting for the message text. Same issue exists for Speak and Shell Command.

**Minimal fix (high confidence):** After the action type is chosen, if the action type requires text input (Message, Speak, Shell Command), open a text input modal instead of immediately inserting the default.

This requires:
1. New `ModalState` variant: `TextInput { event: AgenticEvent, action_type: usize, buffer: String, label: String }`
2. A new text input modal renderer and key handler
3. The action type chooser's Enter handler transitions to the TextInput modal for types 1, 2, 3

**Scope note:** This is the largest single change. If time is limited, an acceptable intermediate fix is to still create the default but immediately open an edit modal for the newly added action. However, the user's complaint is explicit: "it doesn't ask for the text message!!!" -- so we need the input modal.

**TextInput modal rendering:**

```rust
// In a new section of modal.rs or actions.rs
pub fn render_text_input_modal(frame: &mut Frame, area: Rect, label: &str, buffer: &str) {
    render_modal(frame, area, label, 60, 20, |frame, area| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // label
                Constraint::Length(1), // input line
                Constraint::Length(1), // blank
                Constraint::Length(1), // hotkeys
                Constraint::Min(0),
            ])
            .split(area);

        let input_line = Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Yellow)),
            Span::raw(buffer),
            Span::styled("_", Style::default().fg(Color::Yellow).add_modifier(Modifier::SLOW_BLINK)),
        ]);
        frame.render_widget(Paragraph::new(input_line), chunks[1]);

        let hotkeys = Line::from(vec![
            Span::styled("ENTER", key_style()),
            Span::styled(": Confirm", desc_style()),
            Span::styled("  │  ", sep_style()),
            Span::styled("ESC", key_style()),
            Span::styled(": Cancel", desc_style()),
        ]);
        frame.render_widget(Paragraph::new(hotkeys).alignment(Alignment::Center), chunks[3]);
    });
}
```

**TextInput key handler:** Handle printable chars (append to buffer), Backspace (pop), Enter (confirm & create action), Esc (cancel back to action type chooser).

---

## Phase 5: TTS Tab Fixes

### 5.1 Show actual default voice names instead of "provider default"

**File:** `tts.rs:129-173`

**Bug:** `resolve_tts_display` returns `"{slug} default"` when no voice is configured. Users need to see the actual voice name.

**Change:** Use `biscuit_speaks` to query the actual default voice. The `TtsExecutor` trait has `default_voice(gender)` but it's async and requires an executor instance. A simpler approach for the TUI:

1. Add a helper function that resolves default voice names per provider:

```rust
fn resolve_default_voice_name(provider_slug: &str, gender: Gender) -> String {
    // For providers with known defaults, return them directly
    match provider_slug {
        "say" => match gender {
            Gender::Female => "Samantha".to_string(),
            Gender::Male => "Alex".to_string(),
            _ => "Samantha".to_string(),
        },
        "kokoro" => match gender {
            Gender::Female => "af_heart".to_string(),
            Gender::Male => "am_adam".to_string(),
            _ => "af_heart".to_string(),
        },
        "espeak" => "default".to_string(),
        "piper" => "en_US-amy-medium".to_string(),
        _ => format!("{provider_slug} default"),
    }
}
```

2. In `resolve_tts_display`, replace `format!("{slug} default")` with `resolve_default_voice_name(&slug, Gender::Female)` / `resolve_default_voice_name(&slug, Gender::Male)`.

**Better approach (if available):** Check if `biscuit-speaks` exposes a synchronous `default_voice_name(provider, gender)` function. If so, use it directly. If not, the hardcoded map is acceptable for v1 since the TUI only runs on the local machine with known providers.

### 5.2 Remove voice indentation, add blank line separator

**File:** `tts.rs:13-23` (layout) and `tts.rs:56-78` (rendering)

**Current layout:**
```
Provider: kokoro (Kokoro TTS)
  Female Voice: af_heart
  Male Voice: am_adam
```

**Target layout:**
```
Provider: kokoro (Kokoro TTS)
                              <-- blank line
Female Voice: af_heart  *
Male Voice: am_adam
                              <-- blank line
* indicates the default gender to be used
```

**Changes:**

1. Update layout constraints:
```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(2), // Toggle
        Constraint::Length(1), // blank
        Constraint::Length(1), // Provider line
        Constraint::Length(1), // blank (NEW)
        Constraint::Length(1), // Female voice
        Constraint::Length(1), // Male voice
        Constraint::Length(1), // blank (NEW)
        Constraint::Length(1), // default gender legend (NEW)
        Constraint::Min(0),
    ])
    .split(area);
```

2. Remove the `"  "` indent prefix from female/male voice lines.

3. Add `*` marker next to the default gender:
```rust
let female_marker = if default_gender == Gender::Female { "  *" } else { "" };
let female_line = Line::from(vec![
    Span::styled("Female Voice: ", Style::default()),
    Span::styled(&female, female_style),
    Span::styled(female_marker, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
]);
```

4. Add the legend line:
```rust
let legend = Line::from(vec![
    Span::styled("*", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    Span::styled(
        " indicates the default gender to be used",
        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
    ),
]);
frame.render_widget(Paragraph::new(legend), chunks[7]);
```

### 5.3 Fix F and M keys doing nothing

**File:** `tts.rs:202-230`

**Bug:** `F` and `M` keys call `query_voices_for_provider(&provider)` but for many providers (like kokoro), this returns an empty vec, so the function returns early without opening a modal.

The `query_voices_for_provider` function in `mod.rs:327-333` only handles `"say"` and `"espeak"` -- all other providers return `vec![]`.

**Fix options (in order of preference):**

1. **Expand `query_voices_for_provider`** to support more providers (kokoro, piper, etc.). For kokoro, list the known voice IDs. For providers that can't be queried, provide a hardcoded list of common voices.

2. **Show a "no voices available" message** instead of silently returning. Open a small modal:
   ```
   ┌─ Female Voice ─┐
   │                 │
   │ No queryable    │
   │ voices for this │
   │ provider.       │
   │                 │
   │ ESC: Close      │
   └─────────────────┘
   ```

3. **Allow freeform text input** for the voice name if the provider doesn't have a queryable voice list.

**Recommendation:** Implement option 1 for kokoro (it has well-known voice IDs: af_heart, af_bella, af_nicole, af_sarah, af_sky, am_adam, am_michael, bf_emma, bf_isabella, bm_george, bm_lewis). For other providers, implement option 2 so the user gets feedback.

Add to `mod.rs`:
```rust
pub fn query_voices_for_provider(provider: &str) -> Vec<String> {
    match provider {
        "say" | "macos" => query_say_voices(),
        "espeak-ng" | "espeak" => query_espeak_voices("espeak-ng"),
        "kokoro" => vec![
            "af_heart", "af_bella", "af_nicole", "af_sarah", "af_sky",
            "am_adam", "am_michael",
            "bf_emma", "bf_isabella",
            "bm_george", "bm_lewis",
        ].into_iter().map(String::from).collect(),
        _ => vec![],
    }
}
```

---

## Phase 6: Hot Key Bar Consistency

### 6.1 Add background color to main hotkey bar

**File:** `mod.rs:168-172`

**Current:** The hotkey bar is rendered as a centered `Paragraph` with no background. It blends into the content area.

**Change:** Add a background style to the hotkey bar area. Before rendering the hotkey bar paragraph, render a fill widget with the background color:

```rust
let hotkey_bg = Block::default()
    .style(Style::default().bg(Color::Indexed(236)));
frame.render_widget(hotkey_bg, inner_chunks[1]);

let hotkey_bar = Paragraph::new(hotkey_line)
    .alignment(Alignment::Center)
    .style(Style::default().bg(Color::Indexed(236)));
frame.render_widget(hotkey_bar, inner_chunks[1]);
```

### 6.2 Ensure all modal hotkey bars match this style

This was partially addressed in Phase 2.3 (sound selector) and Phase 4.2 (delete confirmation). Ensure every modal that shows hotkeys uses:
- Yellow + Bold for the key letter (uppercase)
- Light gray (Indexed(250)) for descriptions
- Dark gray background (Indexed(236))
- Pipe separators (` | `)

**Modals to check:**
- Sound selector (Phase 2.3) -- will use new hotkey bar
- Delete confirmation (Phase 4.2) -- will use new hotkey bar
- Text input modal (Phase 4.4) -- will use new hotkey bar
- Protect rules modal -- currently has no hotkey hint (add one: `SPACE: Toggle | ENTER: Done | ESC: Cancel`)
- All list modals -- currently have no hotkey hint (add `ENTER: Select | ESC: Cancel` via `render_list_modal_with_hotkeys`)

### 6.3 Extract shared hotkey rendering utility

To avoid duplicating the hotkey line construction logic, extract a shared function:

```rust
// In modal.rs or a new hotkeys.rs
pub fn build_modal_hotkey_line(pairs: &[(&str, &str)]) -> Line<'static> {
    let key_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::Indexed(250));
    let sep_style = Style::default().fg(Color::Indexed(240));

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" | ", sep_style));
        }
        spans.push(Span::styled(key.to_string(), key_style));
        spans.push(Span::styled(format!(": {desc}"), desc_style));
    }
    Line::from(spans)
}
```

Then the existing `build_hotkey_line` in `mod.rs` can delegate to this (or just use the same function, since the only difference is the pipe character `│` vs `|`).

---

## Phase 7: Full Inconsistency Sweep

**Goal:** Systematically walk every screen and dialog looking for inconsistencies beyond what was explicitly reported.

### 7.1 Checklist of things to verify

- [ ] **Preferences tab:**
  - Are all selector modals (Agent, User Provider, Repo Provider) using `render_list_modal_with_hotkeys` with `ENTER: Select | ESC: Cancel`?
  - Do all modals pre-select the current value? (Agent selector starts at 0, not at current agent index -- same bug as sound selector)
  - Is the "Repo Scoped Provider" line properly grayed out when not in a repo?

- [ ] **Services tab:**
  - Protect rules modal: Does it have a hotkey bar? (Currently no)
  - Does the rules modal use `ListState` for scrolling? (It uses raw `List` -- same scrolling bug as sound selector)

- [ ] **TTS tab:**
  - Provider selector modal: Pre-selects current provider? (Currently starts at 0)
  - Voice selector modal: Pre-selects current voice? (Currently starts at 0)

- [ ] **Actions tab:**
  - Event selector modal: Does it have hotkey hints?
  - Action type chooser modal: Does it have hotkey hints?

- [ ] **Messenger tab:**
  - Messenger select modal: Pre-selects current active config?
  - Messenger add modal: Has hotkey hints?
  - The `[+ Add New]` button: Consistent with other interaction patterns?

- [ ] **Cross-tab:**
  - All modals use same border color (Yellow -- verified)
  - All modals use same title style (Bold Yellow -- verified)
  - All highlighted items in modals use same style (Yellow Bold -- verified)
  - No mix of uppercase/lowercase in hotkey labels
  - All colons followed by exactly one space in label:value patterns

### 7.2 Known additional pre-selection bugs to fix

Apply the same "pre-select current value" fix from Phase 2.1 to these modals:

1. **Agent selector** (`preferences.rs:209-211`):
   ```rust
   let highlighted = super::super::get_provider_list()
       .iter()
       .position(|p| *p == app.config.preferred_agent)
       .unwrap_or(0);
   ```

2. **User provider selector** (`preferences.rs:212-214`):
   ```rust
   let highlighted = app.config.canonical_provider
       .and_then(|cp| super::super::get_available_providers().iter().position(|p| *p == cp))
       .map(|i| i + 1)  // +1 for "(clear)" at index 0
       .unwrap_or(0);
   ```

3. **Repo provider selector** (`preferences.rs:215-219`): Same pattern as user provider.

4. **TTS provider selector** (`tts.rs:199-200`):
   ```rust
   let current_slug = match &app.config.tts {
       TtsValue::Config(cfg) => Some(cfg.provider.clone()),
       _ => None,
   };
   let providers = super::super::get_tts_provider_list();
   let highlighted = current_slug
       .and_then(|slug| providers.iter().position(|p| super::super::tts_provider_slug(p) == slug))
       .unwrap_or(0);
   ```

5. **Messenger select** (`messenger.rs:171`): Pre-select active config.

---

## Execution Order

| Order | Phase | Effort | Risk | Dependencies |
|-------|-------|--------|------|-------------|
| 1 | Phase 1: Toggle & Label Consistency | Low | Low | None |
| 2 | Phase 6.3: Extract hotkey utility | Low | Low | None |
| 3 | Phase 2.2: Fix list scrolling (ListState) | Medium | Medium | None |
| 4 | Phase 2.1: Sound pre-selection | Low | Low | None |
| 5 | Phase 7.2: All other pre-selection fixes | Low | Low | None |
| 6 | Phase 2.3-2.4: Sound hotkey bar + D key | Medium | Low | Phase 6.3 |
| 7 | Phase 6.1-6.2: Hotkey bar backgrounds | Low | Low | Phase 6.3 |
| 8 | Phase 4.1: Enter key for Actions edit | Low | Low | None |
| 9 | Phase 4.2: Delete confirmation styling | Low | Low | Phase 6.3 |
| 10 | Phase 4.3: Action list display format | Medium | Low | None |
| 11 | Phase 5.1-5.2: TTS voice display fixes | Medium | Low | None |
| 12 | Phase 5.3: Fix F/M keys | Medium | Medium | None |
| 13 | Phase 4.4: Text input modal for actions | High | Medium | Phase 6.3 |
| 14 | Phase 3: Exit messages | Medium | Low | App struct changes |
| 15 | Phase 7.1: Full sweep & verification | Medium | Low | All above |

## Testing Strategy

- **Manual testing** is primary -- TUI components are visual and interactive
- After each phase, run the TUI and verify:
  1. Tab navigation still works
  2. All modals open and close correctly
  3. Hot keys are responsive
  4. Scrolling works in sound selector with 88+ items
  5. Pre-selection highlights the correct current value
  6. Exit messages display correctly in both changed/unchanged scenarios
- `cargo build -p claudine-cli` after each phase to catch compile errors
- Run `cargo test -p claudine-cli` if any tests exist for the TUI module

## Risk Notes

- **Phase 2.2 (ListState migration):** Changing from `render_widget` to `render_stateful_widget` affects all list modals. Test every modal after this change.
- **Phase 4.4 (TextInput modal):** This is net-new functionality. The `ModalState::TextInput` variant adds complexity to `app.rs`'s match arms. Keep the text input handler simple -- no multi-line, no cursor movement, just append/backspace/enter/esc.
- **Phase 5.1 (default voices):** Hardcoding voice names is fragile. Consider adding a TODO to make this dynamic in a future release. For now, the known providers on macOS (say, kokoro) cover the primary use case.
- **Phase 5.3 (kokoro voices):** The hardcoded voice list will go stale if kokoro updates. Accept this for v1.
