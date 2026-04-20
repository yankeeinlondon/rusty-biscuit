# Help System Touch Up — Implementation Plan

## Summary

Two changes to `claudine/cli/src/commands/help.rs`:

1. **New "Hook Events and Actions" group** — move `hooks` (from Shared Resources) and `actions` (from Administration) into a new group placed directly after "Shared Resources"
2. **New title format** — replace hardcoded title with `<b><yellow>Claudine</yellow></b>\n<dim><i>{description}</i></dim>` where `{description}` comes from Cargo.toml's `description` field at compile time

## File Changes

### `claudine/cli/src/commands/help.rs`

#### Change 1: Reorganize command groups

In `groups()`:
- Remove `hooks` entry from "Shared Resources" (line 43)
- Remove `actions` entry from "Administration" (line 96)
- Insert new `CommandGroup { name: "Hook Events and Actions", ... }` after "Shared Resources" containing both commands

#### Change 2: Update title rendering

In `run()`:
- Replace the hardcoded title string (lines 140-143) with:
  ```rust
  output.push_str(
      &Prose::new(format!(
          "<b><yellow>Claudine</yellow></b>\n<dim><i>{}</i></dim>",
          env!("CARGO_PKG_DESCRIPTION")
      ))
      .render(&term),
  );
  ```

## Verification

- `just build -p claudine-cli` — compiles
- `just test` in `claudine/` — tests pass
- Manual: `claudine` (no args) shows new group ordering and title
