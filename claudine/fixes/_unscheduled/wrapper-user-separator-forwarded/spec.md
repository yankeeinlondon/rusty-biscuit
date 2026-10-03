# A user `--` after the positional prompt reaches the provider verbatim

The wrapper documents `-- ...` as "force all remaining args to passthrough
unchanged". In practice Claudine keeps the `--` itself in the provider argv, so
the provider also reads it as end-of-options and treats everything after it as
positional messages rather than options. A profile that appends its own `--`
before the prompt then produces two separators.

## Reproduction

```sh
claudine pi "hello" -i -- --offline
```

Launches `pi -- --offline -- hello`: Pi reads `--offline` as a message and the
second `--` as another message token. Observed live on Pi 0.87.1 on
2026-09-28 while writing `real_pi_interactive_startup.rs` for
`2026-09-18-edit-integration`.

The profiles that append their own `--` today are Pi (interactive), and Codex
and Kilo (for a prompt starting with `-`). Claude also uses `--` for a
`-`-prefixed prompt.

## Direction

Consume the user's `--` as Claudine's own boundary and pass only the arguments
after it, so the provider sees them as the options they were meant to be. Keep
them opaque to Claudine (no wrapper-flag recovery after `--`). Verify each
profile that appends its own `--` still places the prompt after the forwarded
options.

## Not fixed in `2026-09-18-edit-integration`

The spec lists "reinterpreting provider arguments after the explicit `--`" as a
non-goal.
