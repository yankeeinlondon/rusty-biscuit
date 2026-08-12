# Run record — F2 real-terminal Level-2 evidence under a Linux kernel

Closes the Linux half of review-2's finding *"F2's Level-2 proof is not
theme-independent and Linux is still Level 1"*. A **real kitty emulator**, running
under a **real Linux kernel**, parses the library's `ESC ] 10 ; ? BEL` and writes
the answer; `biscuit-terminal` itself consumes it.

## What was wrong before

The spec's audit table claimed *"Verified (L2) on macOS **and real Linux**"*, and
the closeout narrative said Finding 2's *"Linux L2 gap is closed (its PTY tests
pass under a real Linux kernel)"*. Both were false. The Linux run executed
`level1_terminal_osc_cache.rs`, which **manufactures its own OSC reply bytes**. A
bare PTY is a kernel pipe with `is_tty()` semantics, not a program that parses
OSC and answers. Running a manufactured reply under a Linux kernel makes it a
Level-1 test on Linux — the kernel is not the thing under test. `results.md`
already said so; the spec table did not.

## Host & environment

- Docker host: macOS 26.5.2, Darwin 25.5.0, arm64 (Apple Silicon).
- Container kernel: `Linux 6.12.76-linuxkit #1 SMP aarch64 GNU/Linux` — a real
  Linux kernel, `linux/arm64` native (no QEMU emulation).
- Emulator: **kitty 0.26.5** (Debian bookworm package), on Xvfb `800x600x24`
  with Mesa llvmpipe software GL.
- Probe: `cargo build -p biscuit-terminal --example discovery_probe` (dev
  profile), built inside the container against a **read-only** mount of the repo,
  with a container-local `CARGO_TARGET_DIR` (the host `target/` was never
  written).

## Why this is honest Level 2 (no env spoofing)

`query_osc_color_with_timeout` emits the real query only when the detected app is
on its supported list **and** `detect_multiplexer().is_none()`. Both hold here
without any override:

- Detection keys on `KITTY_WINDOW_ID` / `KITTY_PID` (`detection/app.rs`), which
  **real kitty sets natively** — `KITTY_PID=21`, `KITTY_WINDOW_ID=1`, plus
  `TERM=xterm-kitty` as the backup path. `TERM_PROGRAM` is unset and was **not**
  faked. See `kitty-env.txt` / `container-meta.txt`.
- `TMUX` / `ZELLIJ` / `STY` all unset, so the multiplexer gate passes. (tmux
  remains structurally unusable as a backend — see `results.md` Finding 2.)
- `CI` unset, so the `is_ci()` gate does not suppress the query.

## Method

`runner-pin-3b7f5c.sh`, run as kitty's direct child:

1. `printf '\033]10;#3b7f5c\007' > /dev/tty` — pin the foreground, from kitty's
   own child, so the bytes reach kitty's parser as program output.
2. `sleep 1` — kitty applies it. (The library's `DEFAULT_TIMEOUT` is only 100 ms;
   a probe fired at a cold emulator silently records the fallback.)
3. Run the probe **bare** — stdout stays on kitty's real tty.
4. Recover the output by scraping kitty's screen:
   `kitty @ --to "$KITTY_LISTEN_ON" get-text --extent screen`.

The mirror of the macOS WezTerm harness, which reads output back with
`wezterm cli get-text` for exactly the same reason.

## Results — verbatim

`probe-pin-3b7f5c.txt` (pin `#3b7f5c` = `(59, 127, 92)`):

```text
terminal_cache_count=3
terminal_text_color[0]=Some(RgbValue { r: 59, g: 127, b: 92 })
terminal_text_color[1]=Some(RgbValue { r: 59, g: 127, b: 92 })
terminal_text_color[2]=Some(RgbValue { r: 59, g: 127, b: 92 })
osc10_actual_queries=1
osc11_actual_queries=1
terminal_cache_done
```

`probe-pin-ff0080.txt` — the same run with an independently chosen second pin
`#ff0080`, reporting `RgbValue { r: 255, g: 0, b: 128 }` three times and
`osc10_actual_queries=1`. Two arbitrary pins tracked exactly: the library is
reading kitty's wire, not echoing a constant.

## What each line proves

- **The answer came off kitty's wire** — `(59, 127, 92)` is the pinned value. The
  library cannot produce it without asking: its compiled-in Kitty fallback is
  `(229, 229, 229)`, kitty's real default foreground is `#dddddd`, and `COLORFGBG`
  (unset) could only name an ANSI palette index, never an arbitrary RGB triple.
- **One round-trip, three constructions** — `osc10_actual_queries=1` across
  `terminal_cache_count=3`. This is the F2 cache claim. It rests on the counter,
  not on color equality: a broken cache would re-query and get the same color back
  from the same terminal.
- The two must be read **together**. `osc10_actual_queries=1` alone proves only
  that an attempt was *made*; the pin is what proves an answer was *received*.

## Discarded attempt — recorded because it is the trap

The first attempt wrapped the probe in `script` to capture stdout to a file. It
produced `terminal_text_color=(229, 229, 229)` — the fallback — with the raw query
bytes `]11;?]10;?` **leaking into the capture file**. `script`'s nested PTY stole
the wire: the probe's `/dev/tty` was script's pts, not kitty's, so kitty never saw
the query and never answered, and the 100 ms timeout expired.

Notably `osc10_actual_queries=1` in that run **too** — the attempt fired. Had the
count been the only assertion, this invalid run would have looked like a pass.
That is precisely why the pin is required, and it is the same class of error as
the manufactured-PTY mislabeling this record corrects. It was discarded, not
reported.

Rule that follows: **anything that redirects, pipes, or wraps the probe's stdout**
(`>`, `|`, `tee`, `script`) either suppresses the query via `is_tty()` or steals
the wire. Output must be recovered by scraping the emulator's screen.

## Reproduction

```bash
docker build -t osc-rust -f Dockerfile .

# Build the probe: read-only repo, container-local target dir.
docker run --rm -v <repo>:/repo:ro -v /tmp/osc-lab/target:/target \
  -e CARGO_TARGET_DIR=/target osc-rust \
  cargo build -p biscuit-terminal --example discovery_probe --manifest-path /repo/Cargo.toml

# Run the probe inside real kitty under Xvfb.
docker run --rm -v /tmp/osc-lab/target:/target -v /tmp/osc-lab/out:/out \
  -v ./runner-pin-3b7f5c.sh:/runner.sh:ro \
  osc-rust sh -c 'xvfb-run -a -s "-screen 0 800x600x24" kitty \
    -o allow_remote_control=yes --listen-on unix:/tmp/kbd.sock /runner.sh'
```

## Gotchas (cost real time; recorded for the next person)

- **Fonts are load-bearing.** `kitty` on a slim Debian base pulls neither
  fontconfig nor any font, and kitty then **hangs silently and indefinitely**
  rather than erroring. `fontconfig libfontconfig1 fonts-dejavu-core` +
  `fc-cache -f` fixes it. This masqueraded as a GL problem.
- **GL was never the problem.** Xvfb + llvmpipe reports OpenGL 4.5 core, well past
  kitty's 3.3 requirement. Verify with `glxinfo -B` before blaming GL.
- **Run `xvfb-run` under `sh -c`**, not as the container's argv directly: as PID 1
  it hangs with zero output (it is a shell script and does not reap/signal as
  init).
- `PROBE_STDOUT_IS_TTY=NO` in `probe-pin-ff0080.txt` is a **measurement artifact**,
  not a finding: `$(test -t 1)` runs inside command substitution, so it reports on
  a pipe rather than the probe's fd. The structural proof is stronger — a non-tty
  stdout returns `NotTty` before any attempt event fires, which would have shown
  `osc10_actual_queries=0` and a fallback color.

## Status of this evidence

This is a **retained manual run**, not a gated test. It is not wired into
`just test-l2`: that would put Docker, Xvfb, and a GL stack on the area's gate for
a single assertion. The macOS WezTerm equivalent
(`level2_terminal_osc_wezterm.rs`) **is** gated and runs on every `just test-l2`;
this record is the cross-platform counterpart, reproducible on demand with the
commands above.
