# Streaming and Failed Files

Part of [Symbol Reports](../symbol-reports.md).

`hug` writes output while it is still scanning, so `hug symbols | head`
returns at once and a large repository starts printing immediately. This page
says how much output a run holds back in each layout, what happens to a file
that cannot be analyzed, and how a failed write or a closed pipe ends a run.

## Streaming

The terminal listing is written as files are analyzed, in file order, and
flushed at least once per window of four files per worker thread.
`hug symbols | head` therefore stops after a window or two, and a large
repository starts printing at once instead of after the whole scan. A run holds only the part of the
listing that later files can still change:

| Layout | Held until written |
|---|---|
| default (`--sort file`), `classes` under any `--sort` | one file |
| `--group-by-file` (alone or with `--group-by-module`), any `--sort` | one file's group |
| `--group-by-module`, any `--sort` | one directory's group |
| `--sort name` or `--sort kind` without grouping | one package: a name or kind order interleaves every file of the package |
| `--prelude` | one package: a prelude lists declarations that live in other files |
| `--jsonl`, `--format table`, `--tree` | one file |
| `--count`, `--stats` | only the counters; the result is written at the end |

For example, `hug symbols --sort name` prints nothing until each package is
fully analyzed, and over a single-package repository that is the whole scan.
Group by file to keep a name order streaming:

```bash
hug symbols --group-by-file --sort name   # each file's declarations by name, printed file by file
```

Package sections add no wait: whether to print them is known before the
first file is analyzed (see [Package sections](./terminal-output.md#package-sections)), and a
package's heading is written when its first file arrives.

To keep each unit complete on arrival, the listing commands analyze files in
the order they are listed: by package, then directory (with
`--group-by-module`), then path. `classes` and the JSON reports analyze files
in path order.

## Failed files

A file that cannot be read or analyzed does not stop the run. It is
reported three ways:

1. on stderr, as `error: <message>`, in the order files are analyzed (see
   [Streaming](#streaming));
2. as one entry in `errors`, with a portable `file`, the error `kind` (`io`,
   `parse_failed`, `unsupported_language`, `too_many_generic_parameters`
   for a selected declaration whose generic parameter positions do not fit
   in `u16`, …) and the `message`;
3. through the exit status, which becomes `1`.

```json
"errors": [
  {
    "file": "src/locked.rs",
    "kind": "io",
    "message": "Failed to read `/repo/src/locked.rs`: Permission denied (os error 13)"
  }
]
```

The other files are listed as usual. Because `errors` is written last, a
report cut short (a killed process, a full disk) never parses as valid JSON,
so a failure can never look like an empty success.

The exit status rules are summarized in
[Symbol Reports](../symbol-reports.md#exit-status).

The stderr line shows every control character in the message (a file name
holding a newline or an escape sequence) as its `\u{..}` escape, so each
failure is one line that cannot drive the terminal; the `errors` entry keeps
the text as written, JSON-encoded. `--verbose` cache lines are shown the same
way.

Every output mode reports a failed file on stderr and exits `1`; the modes
with an envelope (`--json`, `--json-full`, `--stats --json`) also list it in
`errors`, while `--jsonl`, `--count` and the terminal formats keep their
output to the good files.

## The flush window and write failures

Files are analyzed in parallel inside a sliding window of four files per
worker thread: a file starts only while it is within that window of the last
output that was flushed. Output is flushed at least once per window, whenever
the next file's result is not ready yet, and at the end. A file that failed
(unreadable, unparsable, or, for `lint`, an external linter that failed on
it) takes its turn like any other: the flush due at that point still writes
the earlier files' output.

A failed write to stdout, a closed pipe included, stops the run: no file
beyond the window is started, and work already started finishes but is not
written, and no external linter runs on a later file. That holds for the
terminal listing, tables, outlines, JSON Lines, the JSON reports (even when a
window added nothing but the JSON envelope) and the text `imports` and `lint`
reports, whether the files in the window succeeded or failed. `--count`,
`--stats`, `hug refs` and the `--json` form of `imports` and `lint` write
only after the whole scan, so they finish it first. The status is then `0` for a closed
pipe (even if a file had failed) and `1` for any other write error.

## `imports` and `lint`

Text `imports` and `lint` are written file by file as the scan goes, and
`--json` writes its one envelope after the last file. Which files a
multi-file `lint` prints is described under [`lint`](../cli.md#lint). An
unavailable external adapter (`oxlint`) is warned about once on stderr, just
before the report of the first file it was asked to lint.

`imports` and `lint` treat stdout like the declaration commands:
`hug imports | head -1` ends quietly with `0`, and any other write failure
prints `error: cannot write output` and exits `1`. When the whole report is
written, `lint` exits `1` if an error-severity diagnostic remains after
`--deny`/`--warn`/`--allow`/`--strict`, and both exit `1` if a file failed:

```sh
hug lint 'src/**' --deny unwrap-call; echo $?   # 1 when any .unwrap() remains
hug lint 'src/**' --deny unwrap-call | head -1   # 0: the reader closed early
```
