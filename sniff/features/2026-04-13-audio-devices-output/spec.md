# Audio Devices Output Redesign

## Goal

Rewrite the audio-devices output produced by `sniff` so it uses the
`biscuit-terminal` components (`Prose`, `UnorderedList`, `Status`) the
way other sniff sections already do, and so it presents input/output
devices in a grouped, styled, easy-to-scan form.

## Scope

The redesign applies to **both** rendering paths in
`sniff/cli/src/output/hardware.rs`:

1. `render_audio_devices_section(...)` — the standalone
   `sniff audio-devices` subcommand.
2. `render_audio_device_list(...)` — the embedded "Audio Devices"
   subsection inside the broader `sniff hardware` output.

Both sites produce the same block (title line, grouped list, footer),
so callers get a consistent presentation regardless of entry point.

## Document Structure

```
\n
<b><uu>Audio Devices</uu></b>\n
<UnorderedList>
  Input
    <UnorderedList>
      {device-line}
      ...
      (or a single <dim><i>none</i></dim> child when the group is empty)
  Output
    <UnorderedList>
      {device-line}
      ...
\n
<Status from_prose, state=Info, theme=Circular> - items with <b><yellow>*</yellow></b> are the <b>default</b> for the input/output
```

The footer is rendered by:

```rust
Status::from_prose(
    "<i><dim> - items with <b><yellow>*</yellow></b> are the <b>default</b> for the input/output</dim></i>",
)
.state(StatusState::Info)
.theme(StatusTheme::Circular)
```

## Grouping

- The **Input** group contains every device whose direction is `Input`
  or `InputOutput`.
- The **Output** group contains every device whose direction is
  `Output` or `InputOutput`.
- A device with direction `InputOutput` therefore appears under both
  groups.
- Within each group, children are sorted alphabetically by device
  name, case-insensitive.

## Device Line Format

```
{name}{suffix?} (<kind>, {rates}){default-marker?}
```

### Name (with duplicate-disambiguation suffix)

When two or more devices in the full detected list share the same
`name`, each colliding device gets a `<dim>{suffix}</dim>` appended to
its name so the reader can tell them apart. The suffix is derived from
the `uid` property:

1. Group devices by `name`.
2. For each collision group with two or more devices, compute the
   longest common prefix of those devices' `uid` values.
3. Each device's suffix is the portion of its `uid` that follows that
   common prefix.
4. If every device in the group produces the same suffix (for example
   because the uids are identical, or all empty after stripping the
   prefix), fall back to a 1-based numeric suffix `:1`, `:2`, etc.,
   assigned in lexicographic uid order.

Example: two devices named `LG UltraFine Display Audio` with uids
ending in `:1` and `:2` are rendered as
`LG UltraFine Display Audio<dim>:1</dim>` and
`LG UltraFine Display Audio<dim>:2</dim>`.

### Kind

Styled per variant of `sniff::hardware::AudioDeviceKind`:

| Variant       | Markup                              |
|---------------|-------------------------------------|
| `BuiltIn`     | `<dim>Built-in</dim>`               |
| `Usb`         | `<indigo-500>USB</indigo-500>`      |
| `Bluetooth`   | `<blue>Bluetooth</blue>`            |
| `Thunderbolt` | `<yellow>Thunderbolt</yellow>`      |
| `Hdmi`        | `<yellow>HDMI</yellow>`             |
| `Virtual`     | `<dim><i>Virtual</i></dim>`         |
| `Unknown`     | plain `Unknown` (no markup)         |

### Sample rates

Rates are expressed in kHz using the shortest sensible form:

- `44100` → `44.1k`
- `48000` → `48k`
- `96000` → `96k`
- `88200` → `88.2k`

The set of rates to render is the union of
`available_sample_rates` and (when non-zero) the current `sample_rate`,
deduplicated and sorted ascending. Rates are listed space-separated
inside the parentheses after the kind. Each rate is `<dim>{item}</dim>`
except the device's current nominal `sample_rate`, which is rendered
as `<b>{item}</b>`.

If the device reports no available rates and no current rate, the
`, {rates}` fragment is omitted and the line ends with just the kind
token inside the parens.

### Default marker

Append ` <b><yellow>*</yellow></b>` to the device line **only** when
the device is the default for the group it is being rendered under:

- Under Input: append when `is_default_input` is true.
- Under Output: append when `is_default_output` is true.

An `InputOutput` device that is default for one direction but not the
other will get the marker under one group and not the other.

## Empty groups

If a group (Input or Output) has no matching devices, render the
group heading with a single child:

```
<dim><i>none</i></dim>
```

## Verbose flags

`sniff audio-devices` continues to accept `-v` / `-vv`, and the
existing verbose extras are preserved:

- `-v`: adds channel-count sub-lines under each device
  (`Output channels: N`, `Input channels: N`).
- `-vv`: adds the `UID: …` sub-line under each device.

Because the default output now already lists all available sample
rates inline, `-vv` no longer prints a separate "Available rates"
line.

## Implementation Notes

- Build the output using `Compose`, `Prose`, `UnorderedList`, and
  `Status` from `biscuit-terminal`, following the pattern in
  `sniff/cli/src/output/network.rs`.
- Keep public entry points
  (`render_audio_devices_section`, `render_audio_device_list`)
  stable; internal helpers change freely.
- Centralize the helpers so both entry points share them:
    - kind-styling helper
    - sample-rate → kHz formatting helper
    - name-suffix disambiguation helper
    - device-line builder
- Preserve JSON output for `OutputFilter::AudioDevices` unchanged.

## Testing

- Update the existing `insta` snapshots touched by the new format.
- Add unit tests for the name-suffix helper covering:
    - no collision (no suffix added)
    - collision with a clean trailing suffix in both uids
    - collision where uids are identical (fallback to `:1`, `:2`, …)
    - collision where uids differ earlier in the string (suffix is the
    full trailing remainder after the longest common prefix)
- Add unit tests for the kHz formatting helper covering 44100, 48000,
  88200, 96000, and a non-integer kHz result.
- Add a rendering test that exercises a small, synthetic device list
  with at least one `InputOutput` device that is default for only one
  direction, and at least one empty group, and asserts the rendered
  markup.

## Out of Scope

- Changes to `sniff::hardware::AudioDeviceInfo` or any of the
  platform-specific detection implementations.
- Changes to the JSON output shape for `audio-devices`.
- Changes to the `sniff hardware` output outside of the audio
  subsection.
