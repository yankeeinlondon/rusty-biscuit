# Goose one-shot prompt starting with `-` is read as a flag

Goose's `run -t/--text` option has no clap `allow_hyphen_values` (upstream
`crates/goose-cli/src/cli.rs`, read 2026-09-28). A value that starts with `-`
after a separate `-t` is therefore parsed as an unknown flag and Goose exits
with a usage error.

Claudine's non-interactive Goose delivery is `run -t <prompt>`
(`cli/src/commands/wrap/profile/goose.rs`), so a one-shot prompt such as a
Markdown list fails:

```sh
claudine goose $'- fix the tests\n- update the docs'
```

The interactive branch already uses the attached form `--text=<prompt>` for a
`-`-prefixed prompt (repaired in `2026-09-18-edit-integration`). The one-shot
branch should do the same.

## Evidence tier

Upstream source only. Goose is not installed on the dev Mac; confirm against a
live binary before and after the fix.

## Not fixed in `2026-09-18-edit-integration`

That fix repaired interactive startup delivery only; changing the
non-interactive shape was outside its scope.
