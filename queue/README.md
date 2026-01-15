# Queue

A CLI which queues jobs to be started at some future time.

Example usage:

```sh
# run the command `so-you-say 'hi'` at 7:00am
queue --at 7:00am "so-you-say 'hi'"
# run the command `echo 'hi'` in 15 minutes
queue --in 15m "so-you-say 'hi'"
# run the shell command `echo 'hi'` in 15 minutes; keep in the foreground
queue --in 15m "echo 'good morning'" --fg
```

All queued commands share the current terminal's STDOUT and STDERR. By default the scheduler returns control immediately and runs the command in the background at the scheduled time; use `--fg` to wait for completion. Times are interpreted in the local timezone. Use `--debug` to emit INFO-level logs; default logging is WARN only.

## `--at` switch

The `--at` switch takes:

- a `time` (for example `7:00am`, `07:00`, or `19:30`)
- and a `command` parameter

## `--in` switch

The `--in` switch takes:

- a `delay` which can be in seconds, minutes, hours, or days
- a `command` parameter

Delay units are:

- if no unit is specified it is defaulted to minutes
- seconds represented by `s`
- minutes represented by `m`
- hours represented by `h`
- days represented by `d`

You can add a space between the numeric value and the unit:

- `1m` and `1 m` are both 1 minute delays

