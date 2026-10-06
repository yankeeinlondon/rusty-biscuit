# `sniff filesystem query`

**Planned:** this command is not implemented yet.

Inspect processes associated with a file or directory tree:

```sh
sniff filesystem query ./checkout
sniff filesystem query ./checkout --json
sniff filesystem query ./checkout --target-only --timeout 2000
```

Directories will include descendants by default. `--target-only` will inspect
the named target alone. Relative paths will resolve against the invocation
directory. `--timeout` will set a positive deadline in milliseconds; the default
will be 2000. The existing bare `sniff filesystem` report will remain available.

Human output will show PID and available process identity, matched paths, evidence
type, and discovery limitations. `--plain` will disable styling. `--json` will
emit the library report with the same evidence and coverage; performance output
and diagnostics will stay on stderr.

A successful empty report means no usage was observed. It does not establish
that there are no watchers or writers. In particular, macOS FSEvents
subscriptions cannot be enumerated systemwide. Read the
[discovery contract](../topics/filesystem-query.md) for platform limits.

Exit 0 will mean a usable report, including partial coverage or no matches.
Exit 1 will mean query failure or failure of every applicable mechanism. Match
presence alone will not change the exit code. The command will not stop
processes or ask to change filesystem content.
