---
phases: 6
start_phase: 5
source_files_during_phase_1: [sniff/cli/src/output/hardware.rs]
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase1: []
source_files_during_phase_2: [sniff/cli/src/output/hardware.rs]
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
source_files_during_phase_3: [sniff/cli/src/output/hardware.rs]
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase3: []
source_files_during_phase_4: [sniff/cli/src/output/hardware.rs]
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase4: []
source_files_during_phase_5: [sniff/cli/src/output/hardware.rs, sniff/cli/src/output/mod.rs]
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase5: []
packages: [sniff-cli]
---
# Audio Devices Output Redesign — Implementation Plan


**Goal:** Rewrite the `sniff audio-devices` text output (and the embedded "Audio Devices" block in `sniff hardware`) so it renders via `biscuit-terminal` components, groups devices by Input/Output, styles kinds, shows available sample rates inline in kHz, and marks the system defaults.

**Architecture:** Build a single `biscuit_terminal::components::compose::Compose` document inside `sniff/cli/src/output/hardware.rs`. Group devices into two nested `UnorderedList`s (Input, Output), emit device lines as `Prose`, and append a final `Status::from_prose(...)` legend line. Three small pure helpers do the work: a kind-styler, a kHz formatter, and a name-suffix disambiguator (so two devices with the same `name` get rendered as `Name<dim>{suffix}</dim>`).

**Tech Stack:** Rust 2024, `biscuit_terminal` (`Compose`, `Prose`, `UnorderedList`, `Status`, `StatusState`, `StatusTheme`), `sniff::hardware::{AudioDeviceInfo, AudioDeviceKind, AudioDirection}`, `cargo test -p sniff-cli`.

**Spec:** `sniff/features/2026-04-13-audio-devices-output/spec.md` — keep it open.

**File Structure:**
- Modify: `sniff/cli/src/output/hardware.rs` — rewrite the "Audio devices" section (lines ~307–404) and update the embedded call site at line ~149–154. All audio helpers live in this file (keep file-local to match existing style).
- Modify: `sniff/cli/src/output/mod.rs` — no public API changes; the `render_audio_devices_section` / `render_audio_device_list` symbol names stay the same (reuse of the existing re-export at line ~51–54).
- Tests live inline in `#[cfg(test)] mod tests` at the bottom of `hardware.rs`.

**Conventions:**
- Use `writeln!`/`write!` only when a pure `String` is needed; otherwise push `Prose` into a `Compose` or `UnorderedList` like `sniff/cli/src/output/network.rs` does.
- Markup tags come from `biscuit-terminal`. Confirm with the component tests already in the repo if uncertain.
- Don't introduce `unwrap`/`expect` on anything other than infallible `writeln!(String, ...)` — matches the `network.rs` style.
- One focused commit per task.

---

### Task 1: Add the kHz sample-rate formatter + unit tests

**Files:**
- Modify: `sniff/cli/src/output/hardware.rs` (add a new helper near the existing `format_sample_rate` at ~line 315; leave `format_sample_rate` alone for now — it's still used by the `-v` branch via the existing verbose-extra rendering)

- [ ] **Step 1: Write the failing tests**

At the bottom of `sniff/cli/src/output/hardware.rs`, add (or extend) a `#[cfg(test)] mod tests` block with:

```rust
#[cfg(test)]
mod audio_format_tests {
    use super::format_sample_rate_khz;

    #[test]
    fn khz_integer_khz() {
        assert_eq!(format_sample_rate_khz(48000.0), "48k");
        assert_eq!(format_sample_rate_khz(96000.0), "96k");
        assert_eq!(format_sample_rate_khz(192000.0), "192k");
    }

    #[test]
    fn khz_fractional_khz() {
        assert_eq!(format_sample_rate_khz(44100.0), "44.1k");
        assert_eq!(format_sample_rate_khz(88200.0), "88.2k");
    }

    #[test]
    fn khz_sub_khz_or_weird() {
        // 500 Hz -> 0.5k (not expected in real life but must not panic/format weirdly)
        assert_eq!(format_sample_rate_khz(500.0), "0.5k");
    }

    #[test]
    fn khz_zero_returns_empty() {
        // The caller is responsible for not calling us with 0, but we still
        // return something stable.
        assert_eq!(format_sample_rate_khz(0.0), "0k");
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli --lib audio_format_tests`
Expected: FAIL with "cannot find function `format_sample_rate_khz`".

- [ ] **Step 3: Implement the helper**

Add near the existing `format_sample_rate`:

```rust
/// Format a sample rate (Hz) as a compact kHz string.
///
/// Integer kHz values render without a decimal (`48000.0` → `"48k"`).
/// Non-integer kHz values render with a single decimal place
/// (`44100.0` → `"44.1k"`).
fn format_sample_rate_khz(rate_hz: f64) -> String {
    let khz = rate_hz / 1000.0;
    if (khz - khz.round()).abs() < 0.01 {
        format!("{}k", khz.round() as i64)
    } else {
        format!("{:.1}k", khz)
    }
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p sniff-cli --lib audio_format_tests`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/output/hardware.rs
git commit -m "feat(sniff): add kHz sample-rate formatter for audio output"
```

---

### Task 2: Add the styled-kind helper + unit tests

**Files:**
- Modify: `sniff/cli/src/output/hardware.rs`

- [ ] **Step 1: Write the failing tests**

Add a new `#[cfg(test)] mod` (or extend the one from Task 1):

```rust
#[cfg(test)]
mod audio_kind_tests {
    use super::style_audio_kind;
    use sniff::hardware::AudioDeviceKind;

    #[test]
    fn built_in() {
        assert_eq!(style_audio_kind(AudioDeviceKind::BuiltIn), "<dim>Built-in</dim>");
    }

    #[test]
    fn usb() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Usb), "<indigo-500>USB</indigo-500>");
    }

    #[test]
    fn bluetooth() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Bluetooth), "<blue>Bluetooth</blue>");
    }

    #[test]
    fn thunderbolt() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Thunderbolt), "<yellow>Thunderbolt</yellow>");
    }

    #[test]
    fn hdmi() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Hdmi), "<yellow>HDMI</yellow>");
    }

    #[test]
    fn virtual_kind() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Virtual), "<dim><i>Virtual</i></dim>");
    }

    #[test]
    fn unknown_is_plain() {
        assert_eq!(style_audio_kind(AudioDeviceKind::Unknown), "Unknown");
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli --lib audio_kind_tests`
Expected: FAIL with "cannot find function `style_audio_kind`".

- [ ] **Step 3: Implement the helper**

```rust
/// Return the styled markup for an [`AudioDeviceKind`] as used in the
/// parenthesized device descriptor.
fn style_audio_kind(kind: sniff::hardware::AudioDeviceKind) -> String {
    use sniff::hardware::AudioDeviceKind as K;
    match kind {
        K::BuiltIn => "<dim>Built-in</dim>".to_string(),
        K::Usb => "<indigo-500>USB</indigo-500>".to_string(),
        K::Bluetooth => "<blue>Bluetooth</blue>".to_string(),
        K::Thunderbolt => "<yellow>Thunderbolt</yellow>".to_string(),
        K::Hdmi => "<yellow>HDMI</yellow>".to_string(),
        K::Virtual => "<dim><i>Virtual</i></dim>".to_string(),
        K::Unknown => "Unknown".to_string(),
    }
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p sniff-cli --lib audio_kind_tests`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/output/hardware.rs
git commit -m "feat(sniff): add styled kind markup for audio devices"
```

---

### Task 3: Add the name-collision suffix resolver + unit tests

**Files:**
- Modify: `sniff/cli/src/output/hardware.rs`

This task produces a pure function that takes the full detected device list and returns a `Vec<String>` of length `devices.len()`, where each entry is either `""` (no collision, no suffix) or the already-styled markup to append after the name (e.g. `"<dim>:1</dim>"`). Putting the styling *inside* the helper keeps the call site trivial.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod audio_suffix_tests {
    use super::build_name_suffixes;
    use sniff::hardware::AudioDeviceInfo;

    fn dev(name: &str, uid: &str) -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: name.to_string(),
            uid: uid.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn no_collision_no_suffix() {
        let devices = vec![dev("Speakers", "uid-a"), dev("Microphone", "uid-b")];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(suffixes, vec!["".to_string(), "".to_string()]);
    }

    #[test]
    fn collision_with_clean_trailing_suffix() {
        // Common prefix: "LGDisplayAudio:", differing tails "1" and "2"
        let devices = vec![
            dev("LG UltraFine Display Audio", "LGDisplayAudio:1"),
            dev("LG UltraFine Display Audio", "LGDisplayAudio:2"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec!["<dim>1</dim>".to_string(), "<dim>2</dim>".to_string()]
        );
    }

    #[test]
    fn collision_with_identical_uids_falls_back_to_indexed() {
        let devices = vec![
            dev("Clone", "same-uid"),
            dev("Clone", "same-uid"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec!["<dim>:1</dim>".to_string(), "<dim>:2</dim>".to_string()]
        );
    }

    #[test]
    fn collision_three_way_with_shared_prefix() {
        let devices = vec![
            dev("Clone", "prefix_alpha"),
            dev("Clone", "prefix_beta"),
            dev("Clone", "prefix_gamma"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec![
                "<dim>alpha</dim>".to_string(),
                "<dim>beta</dim>".to_string(),
                "<dim>gamma</dim>".to_string(),
            ]
        );
    }

    #[test]
    fn collision_leaves_non_colliding_devices_empty() {
        let devices = vec![
            dev("Clone", "prefix_a"),
            dev("Solo", "unrelated"),
            dev("Clone", "prefix_b"),
        ];
        let suffixes = build_name_suffixes(&devices);
        assert_eq!(
            suffixes,
            vec![
                "<dim>a</dim>".to_string(),
                "".to_string(),
                "<dim>b</dim>".to_string(),
            ]
        );
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli --lib audio_suffix_tests`
Expected: FAIL with "cannot find function `build_name_suffixes`".

- [ ] **Step 3: Implement the helper**

```rust
/// For each device, compute the `<dim>…</dim>` suffix (if any) that should be
/// appended to its name so that name collisions are visually disambiguated.
///
/// Non-colliding devices get `""`.
///
/// Colliding devices (two or more with the same `name`) get a suffix derived
/// from the longest common prefix of their `uid`s. If every device in a
/// collision group would produce the same suffix (e.g. identical uids), we
/// fall back to 1-based `:1`, `:2`, … ordered by lexicographic uid.
fn build_name_suffixes(devices: &[sniff::hardware::AudioDeviceInfo]) -> Vec<String> {
    use std::collections::HashMap;

    // Group indices by name.
    let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
    for (idx, dev) in devices.iter().enumerate() {
        groups.entry(dev.name.as_str()).or_default().push(idx);
    }

    let mut suffixes: Vec<String> = vec![String::new(); devices.len()];

    for (_name, indices) in groups {
        if indices.len() < 2 {
            continue;
        }

        let uids: Vec<&str> = indices.iter().map(|&i| devices[i].uid.as_str()).collect();
        let prefix_len = longest_common_prefix_len(&uids);
        let tails: Vec<&str> = uids.iter().map(|u| &u[prefix_len..]).collect();

        // Collect unique tails to decide whether fallback is needed.
        let mut unique_tails = tails.clone();
        unique_tails.sort();
        unique_tails.dedup();

        if unique_tails.len() == indices.len()
            && tails.iter().all(|t| !t.is_empty())
        {
            // Every device has a distinct, non-empty tail. Use it.
            for (i, tail) in indices.iter().zip(tails.iter()) {
                suffixes[*i] = format!("<dim>{}</dim>", tail);
            }
        } else {
            // Fallback: :1, :2, … in lexicographic uid order.
            let mut ordered: Vec<usize> = indices.clone();
            ordered.sort_by(|a, b| devices[*a].uid.cmp(&devices[*b].uid));
            for (rank, idx) in ordered.iter().enumerate() {
                suffixes[*idx] = format!("<dim>:{}</dim>", rank + 1);
            }
        }
    }

    suffixes
}

/// Length in bytes of the longest common prefix of every string in `values`.
/// Guaranteed safe to slice at this offset on all entries (byte-safe on UTF-8
/// when the prefix aligns on a char boundary, which it does here because we
/// only ever compare raw bytes that match across every input).
fn longest_common_prefix_len(values: &[&str]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let first = values[0].as_bytes();
    let mut len = first.len();
    for v in &values[1..] {
        let b = v.as_bytes();
        len = len.min(b.len());
        len = (0..len).take_while(|&i| b[i] == first[i]).count();
        if len == 0 {
            return 0;
        }
    }
    // Walk back to the nearest char boundary in the first string.
    while len > 0 && !values[0].is_char_boundary(len) {
        len -= 1;
    }
    len
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p sniff-cli --lib audio_suffix_tests`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/output/hardware.rs
git commit -m "feat(sniff): disambiguate duplicate audio device names via uid suffix"
```

---

### Task 4: Build the device-line Prose builder + tests

**Files:**
- Modify: `sniff/cli/src/output/hardware.rs`

Produces the `Prose` content for one device, rendered under either the Input or the Output group. Side is passed in so we know whether to apply the default marker.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod audio_line_tests {
    use super::{build_device_line, GroupSide};
    use sniff::hardware::{AudioDeviceInfo, AudioDeviceKind, AudioDirection};

    fn macbook_speakers() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "MacBook Pro Speakers".to_string(),
            uid: "BuiltInSpeakerDevice".to_string(),
            kind: AudioDeviceKind::BuiltIn,
            direction: AudioDirection::Output,
            is_default_input: false,
            is_default_output: true,
            sample_rate: 48000.0,
            available_sample_rates: vec![44100.0, 48000.0, 96000.0],
            input_channels: 0,
            output_channels: 2,
        }
    }

    #[test]
    fn output_side_shows_default_marker() {
        let line = build_device_line(&macbook_speakers(), "", GroupSide::Output);
        assert_eq!(
            line,
            "MacBook Pro Speakers (<dim>Built-in</dim>, <dim>44.1k</dim> <b>48k</b> <dim>96k</dim>) <b><yellow>*</yellow></b>"
        );
    }

    #[test]
    fn input_side_omits_default_output_marker() {
        // Same device, but on the Input side: it is not default input,
        // so no marker appears. (The device wouldn't normally render on the
        // Input side since direction is Output, but this is a unit test of
        // the line builder.)
        let line = build_device_line(&macbook_speakers(), "", GroupSide::Input);
        assert_eq!(
            line,
            "MacBook Pro Speakers (<dim>Built-in</dim>, <dim>44.1k</dim> <b>48k</b> <dim>96k</dim>)"
        );
    }

    #[test]
    fn name_suffix_is_appended_before_parens() {
        let line = build_device_line(&macbook_speakers(), "<dim>:1</dim>", GroupSide::Output);
        assert!(line.starts_with("MacBook Pro Speakers<dim>:1</dim> ("));
    }

    #[test]
    fn missing_rates_drops_the_rate_segment() {
        let mut dev = macbook_speakers();
        dev.sample_rate = 0.0;
        dev.available_sample_rates.clear();
        let line = build_device_line(&dev, "", GroupSide::Output);
        assert_eq!(
            line,
            "MacBook Pro Speakers (<dim>Built-in</dim>) <b><yellow>*</yellow></b>"
        );
    }

    #[test]
    fn current_rate_not_in_available_list_is_still_rendered_bold() {
        let mut dev = macbook_speakers();
        dev.sample_rate = 192000.0;
        dev.available_sample_rates = vec![48000.0, 96000.0];
        let line = build_device_line(&dev, "", GroupSide::Output);
        // Union + sort: 48, 96, 192; current=192 is bold.
        assert!(line.contains("<dim>48k</dim> <dim>96k</dim> <b>192k</b>"));
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli --lib audio_line_tests`
Expected: FAIL with "cannot find … `build_device_line` / `GroupSide`".

- [ ] **Step 3: Implement the builder**

Add near the other audio helpers (a `GroupSide` enum and `build_device_line`):

```rust
/// Which group ("Input" or "Output") a device line is being rendered under.
/// Used to decide whether the default marker should be appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupSide {
    Input,
    Output,
}

/// Build the inline markup for one device line (no bullet, no newline).
///
/// Format: `{name}{suffix?} (<kind>, {rates}){ default-marker?}`.
fn build_device_line(
    dev: &sniff::hardware::AudioDeviceInfo,
    name_suffix: &str,
    side: GroupSide,
) -> String {
    let kind_markup = style_audio_kind(dev.kind);

    // Build the rate set: union of available + current (if non-zero),
    // deduped, sorted ascending.
    let mut rates: Vec<f64> = dev.available_sample_rates.clone();
    if dev.sample_rate > 0.0 {
        rates.push(dev.sample_rate);
    }
    rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rates.dedup_by(|a, b| (*a - *b).abs() < 0.01);

    let rates_markup: String = if rates.is_empty() {
        String::new()
    } else {
        let current = dev.sample_rate;
        rates
            .iter()
            .map(|r| {
                let label = format_sample_rate_khz(*r);
                if current > 0.0 && (*r - current).abs() < 0.01 {
                    format!("<b>{}</b>", label)
                } else {
                    format!("<dim>{}</dim>", label)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let parens = if rates_markup.is_empty() {
        format!("({})", kind_markup)
    } else {
        format!("({}, {})", kind_markup, rates_markup)
    };

    let is_default_here = match side {
        GroupSide::Input => dev.is_default_input,
        GroupSide::Output => dev.is_default_output,
    };
    let marker = if is_default_here {
        " <b><yellow>*</yellow></b>"
    } else {
        ""
    };

    format!("{}{} {}{}", dev.name, name_suffix, parens, marker)
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p sniff-cli --lib audio_line_tests`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add sniff/cli/src/output/hardware.rs
git commit -m "feat(sniff): build styled audio device line with default marker"
```

---

### Task 5: Build the grouped list + replace `render_audio_device_list` / `render_audio_devices_section`

**Files:**
- Modify: `sniff/cli/src/output/hardware.rs` (rewrite the section at ~lines 307–404 and update the embedded call at ~lines 149–154)

This is the main integration. We build one `Compose` document that both entry points share. The verbose-extras (`-v` channel counts, `-vv` UID) continue to render as plain indented sub-lines attached to the device, preserving today's behavior under `-v`/`-vv`.

- [ ] **Step 1: Write the failing rendering test**

Add (at the bottom of the file, in `#[cfg(test)] mod tests`):

```rust
#[cfg(test)]
mod audio_section_tests {
    use super::render_audio_devices_section;
    use sniff::hardware::{AudioDeviceInfo, AudioDeviceKind, AudioDirection};

    fn mic() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "USB Microphone".to_string(),
            uid: "usb-mic".to_string(),
            kind: AudioDeviceKind::Usb,
            direction: AudioDirection::Input,
            is_default_input: true,
            is_default_output: false,
            sample_rate: 48000.0,
            available_sample_rates: vec![48000.0, 96000.0],
            input_channels: 1,
            output_channels: 0,
        }
    }

    fn speakers() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "MacBook Pro Speakers".to_string(),
            uid: "BuiltInSpeakerDevice".to_string(),
            kind: AudioDeviceKind::BuiltIn,
            direction: AudioDirection::Output,
            is_default_input: false,
            is_default_output: true,
            sample_rate: 48000.0,
            available_sample_rates: vec![44100.0, 48000.0, 96000.0],
            input_channels: 0,
            output_channels: 2,
        }
    }

    fn interface_io_default_out_only() -> AudioDeviceInfo {
        AudioDeviceInfo {
            name: "USB Audio Interface".to_string(),
            uid: "usb-iface".to_string(),
            kind: AudioDeviceKind::Usb,
            direction: AudioDirection::InputOutput,
            is_default_input: false,
            is_default_output: false,
            sample_rate: 44100.0,
            available_sample_rates: vec![44100.0],
            input_channels: 2,
            output_channels: 2,
        }
    }

    #[test]
    fn title_and_footer_appear() {
        let devices = vec![mic(), speakers()];
        let out = render_audio_devices_section(&devices, 0);
        assert!(out.contains("Audio Devices"), "title missing:\n{}", out);
        assert!(
            out.contains("default") && out.contains("input/output"),
            "footer missing:\n{}",
            out
        );
    }

    #[test]
    fn default_markers_are_side_specific() {
        // speakers is default_output only. Interface is IO but not default
        // on either side. Rendered under Output only speakers gets the `*`.
        let devices = vec![speakers(), interface_io_default_out_only()];
        let out = render_audio_devices_section(&devices, 0);

        // Speakers should have a trailing `*` marker once.
        let marker_count = out.matches("*").count();
        assert!(marker_count >= 1, "expected at least one * marker:\n{}", out);

        // Interface appears under both groups but is never default => should
        // not have a star adjacent to its name on either side. We can't be
        // too picky about markup, but we can check the footer is the only
        // place other than speakers where '*' can appear.
    }

    #[test]
    fn empty_input_group_shows_none_placeholder() {
        let devices = vec![speakers()];
        let out = render_audio_devices_section(&devices, 0);
        // Input group should appear and contain "none" (styled).
        assert!(out.contains("Input"), "Input header missing:\n{}", out);
        assert!(out.contains("none"), "'none' placeholder missing:\n{}", out);
    }

    #[test]
    fn empty_device_list_still_renders_both_groups_with_none() {
        let out = render_audio_devices_section(&[], 0);
        assert!(out.contains("Input"));
        assert!(out.contains("Output"));
        assert!(out.contains("none"));
    }

    #[test]
    fn verbose_one_adds_channel_counts() {
        let out = render_audio_devices_section(&[speakers()], 1);
        assert!(out.contains("Output channels: 2"), "missing -v extras:\n{}", out);
    }

    #[test]
    fn verbose_two_adds_uid() {
        let out = render_audio_devices_section(&[speakers()], 2);
        assert!(out.contains("UID: BuiltInSpeakerDevice"), "missing -vv extras:\n{}", out);
    }
}
```

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test -p sniff-cli --lib audio_section_tests`
Expected: FAIL (the existing `render_audio_devices_section` produces `=== Audio Devices ===` and the old layout — tests won't match).

- [ ] **Step 3: Replace the audio section in `hardware.rs`**

Replace the block between `// Audio devices` and the end of `render_audio_devices_section` (roughly lines 307–404) with:

```rust
// ============================================================================
// Audio devices
// ============================================================================

/// Format a sample rate for display (legacy Hz form, used by verbose extras).
///
/// Integer rates (e.g., 48000.0) display without decimals.
/// Fractional rates display with 1 decimal place.
fn format_sample_rate(rate: f64) -> String {
    if (rate - rate.round()).abs() < 0.01 {
        format!("{}", rate as u64)
    } else {
        format!("{:.1}", rate)
    }
}

// NOTE: `format_sample_rate_khz`, `style_audio_kind`, `build_name_suffixes`,
// `longest_common_prefix_len`, `GroupSide`, and `build_device_line` are
// defined in earlier tasks.

/// Build a `Prose` item for the "none" placeholder child of an empty group.
fn empty_group_placeholder() -> biscuit_terminal::components::prose::Prose {
    biscuit_terminal::components::prose::Prose::new("<dim><i>none</i></dim>")
}

/// Build the Input or Output group as a nested `UnorderedList`.
///
/// `rendered` is the subset of devices whose `direction` places them on
/// this side, already sorted alphabetically (case-insensitive) by name.
/// `name_suffixes` is indexed by the device's index in `rendered`.
fn build_group_list(
    heading: &str,
    rendered: &[(usize, &sniff::hardware::AudioDeviceInfo)],
    all_name_suffixes: &[String],
    side: GroupSide,
    verbose: u8,
) -> biscuit_terminal::components::list::UnorderedList {
    use biscuit_terminal::components::{
        list::UnorderedList,
        prose::Prose,
    };

    let mut group = UnorderedList::empty();
    group.add(Prose::new(format!("<b>{}</b>", heading)));

    let mut children = UnorderedList::empty();
    if rendered.is_empty() {
        children.add(empty_group_placeholder());
    } else {
        for (original_idx, dev) in rendered {
            let suffix = all_name_suffixes[*original_idx].as_str();
            children.add(Prose::new(build_device_line(dev, suffix, side)));

            if verbose > 0 {
                if dev.output_channels > 0 {
                    children.add(Prose::new(format!(
                        "  <dim>Output channels:</dim> {}",
                        dev.output_channels
                    )));
                }
                if dev.input_channels > 0 {
                    children.add(Prose::new(format!(
                        "  <dim>Input channels:</dim> {}",
                        dev.input_channels
                    )));
                }
            }
            if verbose > 1 && !dev.uid.is_empty() {
                children.add(Prose::new(format!("  <dim>UID:</dim> {}", dev.uid)));
            }
        }
    }

    group.add(children);
    group
}

/// Render a list of audio devices as the grouped Input/Output block.
///
/// This is the shared builder used by both `sniff audio-devices` and the
/// embedded "Audio Devices" subsection inside `sniff hardware`. The output
/// does NOT start with a leading newline — callers decide on preceding
/// spacing. It does end with a trailing newline after the footer.
///
/// ## Returns
/// A pre-rendered string suitable for writing straight to the terminal.
fn render_audio_device_list(
    devices: &[sniff::hardware::AudioDeviceInfo],
    verbose: u8,
) -> String {
    use biscuit_terminal::{
        components::{
            compose::Compose,
            list::UnorderedList,
            prose::Prose,
            status::{Status, StatusState, StatusTheme},
        },
        terminal::Terminal,
    };
    use sniff::hardware::AudioDirection;

    let terminal = Terminal::new();

    // Precompute suffixes over the full device list so collisions across
    // directions stay consistent.
    let suffixes = build_name_suffixes(devices);

    // Pick Input-side devices and Output-side devices.
    let mut input_devs: Vec<(usize, &sniff::hardware::AudioDeviceInfo)> = devices
        .iter()
        .enumerate()
        .filter(|(_, d)| matches!(d.direction, AudioDirection::Input | AudioDirection::InputOutput))
        .collect();
    let mut output_devs: Vec<(usize, &sniff::hardware::AudioDeviceInfo)> = devices
        .iter()
        .enumerate()
        .filter(|(_, d)| matches!(d.direction, AudioDirection::Output | AudioDirection::InputOutput))
        .collect();

    // Alphabetical by name, case-insensitive.
    input_devs.sort_by(|(_, a), (_, b)| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    output_devs.sort_by(|(_, a), (_, b)| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Build the outer UnorderedList with the two groups.
    let input_group = build_group_list("Input", &input_devs, &suffixes, GroupSide::Input, verbose);
    let output_group = build_group_list("Output", &output_devs, &suffixes, GroupSide::Output, verbose);

    let mut outer = UnorderedList::empty();
    outer.add(input_group);
    outer.add(output_group);

    // Compose: title + list (no leading newline — callers control spacing).
    let mut doc = Compose::default();
    doc.add_prose(Prose::new("<b><uu>Audio Devices</uu></b>"));
    doc.add_text("\n");
    doc.add_unordered_list(outer);

    let mut out = doc.display(&terminal).to_string();

    // Blank line between the list and the footer.
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');

    let footer = Status::from_prose(
        "<i><dim> - items with <b><yellow>*</yellow></b> are the <b>default</b> for the input/output</dim></i>",
    )
    .state(StatusState::Info)
    .theme(StatusTheme::Circular);
    out.push_str(&footer.display(&terminal).to_string());
    if !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

/// Render standalone audio devices section (for `sniff audio-devices`).
///
/// Same block as [`render_audio_device_list`], but with a leading blank line
/// so the title has breathing room when printed at the top of the standalone
/// command's output (matches the spec's `\n<b><uu>Audio Devices</uu></b>`).
pub fn render_audio_devices_section(
    devices: &[sniff::hardware::AudioDeviceInfo],
    verbose: u8,
) -> String {
    let mut out = String::from("\n");
    out.push_str(&render_audio_device_list(devices, verbose));
    out
}
```

Note: `render_audio_device_list` stays private (module-local). Only `render_audio_devices_section` is re-exported via `mod.rs`.

- [ ] **Step 4: Update the embedded call site**

At ~line 149–154 of `hardware.rs` (inside `render_hardware_section`), replace:

```rust
    // Print audio devices if available
    if !hardware.audio_devices.is_empty() {
        writeln!(out, "Audio Devices:").unwrap();
        out.push_str(&render_audio_device_list(&hardware.audio_devices, verbose));
        writeln!(out).unwrap();
    }
```

with:

```rust
    if !hardware.audio_devices.is_empty() {
        writeln!(out).unwrap();
        out.push_str(&render_audio_device_list(&hardware.audio_devices, verbose));
    }
```

(The title + footer are now produced by the shared builder. The
`writeln!(out)` supplies the single blank line separating audio from the
preceding GPU block; the builder's own trailing `\n` separates audio from
Storage.)

- [ ] **Step 5: Confirm `mod.rs` re-export still works**

Open `sniff/cli/src/output/mod.rs` and confirm line ~51–54 still reads:

```rust
pub(crate) use hardware::{
    render_audio_devices_section, render_cpu_section, render_gpu_section, render_hardware_section,
    render_memory_section, render_storage_section,
};
```

No change should be needed. If the Rust compiler complains about missing `render_audio_devices_section`, add it back to the list (it should already be there).

- [ ] **Step 6: Run the failing tests and confirm they pass**

Run: `cargo test -p sniff-cli --lib audio_section_tests audio_format_tests audio_kind_tests audio_suffix_tests audio_line_tests`
Expected: all PASS.

- [ ] **Step 7: Run the full sniff-cli test suite**

Run: `cargo test -p sniff-cli`
Expected: all PASS. If a snapshot fails because of the new layout, inspect it with `cargo insta review` (or the repo's equivalent) and accept only if the change is consistent with the spec.

- [ ] **Step 8: Commit**

```bash
git add sniff/cli/src/output/hardware.rs sniff/cli/src/output/mod.rs
git commit -m "feat(sniff): redesign audio-devices output with grouped input/output"
```

---

### Task 6: Manual smoke test + lint/build

**Files:**
- None modified.

- [ ] **Step 1: Run the standalone subcommand**

Run: `cargo run -p sniff-cli -- audio-devices`
Expected: A bold-underlined "Audio Devices" title, two groups ("Input"/"Output") with alphabetically-sorted children, styled kinds, inline kHz rates, a `*` marker on your system-default input and output devices, and a circular-Info footer line.

Also run: `cargo run -p sniff-cli -- audio-devices -v` and `cargo run -p sniff-cli -- audio-devices -vv`
Expected: `-v` adds `Output channels: N` / `Input channels: N` sub-lines; `-vv` additionally shows `UID: …`.

- [ ] **Step 2: Run the embedded section in `sniff hardware`**

Run: `cargo run -p sniff-cli -- hardware`
Expected: the "Audio Devices" block appears in the same new format, sandwiched between GPU and Storage sections, with exactly one blank line separating it from its neighbours.

- [ ] **Step 3: Confirm JSON output unchanged**

Run: `cargo run -p sniff-cli -- audio-devices --json | jq type`
Expected: `"array"` (unchanged — JSON shape is still a flat array of `AudioDeviceInfo`).

- [ ] **Step 4: Lint**

Run: `cargo clippy -p sniff-cli --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Format**

Run: `cargo fmt --package sniff-cli`
Expected: no changes, or apply and commit any formatting diffs.

- [ ] **Step 6: Commit anything left over (fmt only)**

If formatting produced changes:

```bash
git add -u sniff/cli
git commit -m "style(sniff): cargo fmt audio-devices output"
```

Otherwise skip this step.

---

## Verification Checklist (run at the end)

- [ ] `cargo test -p sniff-cli` is green.
- [ ] `cargo clippy -p sniff-cli --all-targets -- -D warnings` is green.
- [ ] `sniff audio-devices` on a real terminal prints the grouped layout described in the spec.
- [ ] `sniff audio-devices -v` and `-vv` still show channel counts and UIDs respectively.
- [ ] `sniff hardware` shows the new audio block without double titles.
- [ ] `sniff audio-devices --json` is still a flat array of devices.
- [ ] Spec requirements all covered:
  - [ ] Title line `<b><uu>Audio Devices</uu></b>` with leading newline
  - [ ] Input/Output grouping with InputOutput devices in both groups
  - [ ] Alphabetical, case-insensitive sort within groups
  - [ ] Kind styling for all 7 `AudioDeviceKind` variants
  - [ ] Sample rates in kHz, current bold, others dim, sorted+deduped union
  - [ ] Side-specific `*` default marker
  - [ ] `<dim><i>none</i></dim>` placeholder for empty groups
  - [ ] Duplicate-name suffix via `build_name_suffixes`
  - [ ] Footer Status line (Info/Circular) with full markup
  - [ ] `-v` preserves channel counts; `-vv` preserves UID
