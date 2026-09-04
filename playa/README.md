# Playa

<table>
<tr>
<td><img src="../assets/playa.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Playa</h2>
<p>This library leverages the host to play audio natively or via a headless audio player installed on the host.</p>

<ul>
    <li>one native-first automatic pipeline (<i>the library feature remains opt-in</i>)</li>
    <li>audio format detection from files, URLs, or bytes</li>
    <li>completion reports with expected and elapsed duration</li>
    <li>durable, globally serialized background playback</li>
    <li>capability-ranked player matching with automatic failover</li>
    <li>88 embedded sound effects across 6 categories</li>
</ul>

<p>
    This Playa library is the audio playback functionality behind the <code>so-you-say</code> and <code>playa</code> CLI's.
</p>
</td>
</tr>
</table>

## Usage

### Build and Install

You can build and install the CLI binary by running:

```sh
just install
```

The CLI enables the native audio backend for the host platform. On macOS and
Windows those are system frameworks and need no install step. On Linux the
backend links ALSA and PulseAudio, so those development libraries must be present
before the build.

> **Linux prerequisites**: `just init` at the repo root installs these for you,
> from the `native` declaration in `playa/lib/Cargo.toml`'s
> `[package.metadata.ci.native]`. To install them by
> hand:
>
> ```sh
> # debian/ubuntu distros
> sudo apt install just pkg-config libasound2-dev libpulse-dev
> ```
>
> Without them the build fails in the `alsa-sys` build script with
> `Package alsa was not found in the pkg-config search path`.
>
> On WSL this installs a Linux binary, not a native Windows `.exe`. It needs the
> same packages, plus a WSL environment with usable audio support.
>
> On **all** platforms we expect you to have the
> [**just**](https://github.com/casey/just) runner installed.

Automatic file and byte playback uses `native-playback`. OS-specific sound-effect
routing uses `sfx-native-audio`; the CLI enables both by default and `target_os`
decides which backend compiles. The library enables neither unless a consumer
opts in. To build the CLI without native audio—and without any ALSA/PulseAudio
requirement—opt out:

```sh
cargo install --path playa/cli --no-default-features
```

Playback then delegates entirely to host players (`afplay`, `ffplay`, `mpv`, …).

### Using the CLI

```sh
# `play` is implied when the first argument is a file
playa hi.wav
playa play hi.wav --meta

# built-in sound effects
playa list-effects trombone
playa effect sad-trombone --background

# what can this host play with?
playa players list
playa players install
playa spool                     # Redacted detached-queue status
```

Speed and volume are `--fast` / `--slow` / `--speed <MULTIPLIER>` and `--quiet` /
`--loud` / `--volume <LEVEL>`. `--channel <CHANNEL>` picks an output device and
`--force-host` skips the native decoder in favor of a host player.

Full flag and subcommand reference: [`playa/cli/README.md`](./cli/README.md).

## Playback and background delivery

All automatic entry points—builder methods and the `playa*` free functions—use
the same native-first pipeline when `native-playback` is enabled, then fall back
to the capability-ranked host players. APIs that explicitly name an
`AudioPlayer` remain host-only. `play_with_report` and
`play_async_with_report` expose the selected route, expected duration, elapsed
duration, and a non-fatal completion verdict. For positively probed mono input,
the mpv host route requests stereo output to avoid early EOF on devices that
cannot negotiate a mono layout; stereo and multichannel input is unchanged.

`--background` durably publishes a job to Playa's private per-user spool and
returns after a scheduler owns it. Jobs from every participating process are
played in sequence without overlap. Publication survives requester exit;
delivery is best-effort and at-most-once after playback begins, so a scheduler
crash can quarantine an in-flight job rather than replaying audio. Each job is
delegated to the absolute executable that enqueued it, preserving that build's
playback features and options. Missing, replaced, or protocol-incompatible
executables fail the job instead of silently degrading it.

The spool is private (`0700` and owner-checked on Unix, protected per-user on
Windows). Speech text may exist only in private preparation records and is not
shown by `playa spool` or written to the journal. `PLAYA_DRY_RUN=1` and builder
dry-run return before source reads, cache writes, spool publication, journal
writes, or child processes.



## Audio Providers

Players are ranked by capability score (speed control +4, volume control +3, stream input +2).

| Software | OS | Speed | Vol | Stream In | Stream Out | Codecs | File Formats |
|----------|:--:|:-----:|:---:|:---------:|:----------:|--------|--------------|
| [mpv](https://mpv.io/) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [FFplay](https://www.ffmpeg.org/ffplay.html) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [SoX play](https://linux.die.net/man/1/sox) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, MP3, Vorbis | WAV, FLAC, OGG, MP3 |
| [afplay](https://ss64.com/osx/afplay.html) | macOS | ✅ | ✅ | ❌ | ❌ | PCM, FLAC, ALAC, MP3, AAC | WAV, AIFF, FLAC, MP3, M4A |
| [VLC](https://wiki.videolan.org/VLC_command-line_help/) | All | ❌ | ✅ | ✅ | ✅ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [MPlayer](https://www.mplayerhq.hu/) | All | ❌ | ✅ | ✅ | ❌ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [GStreamer gst-play](https://gstreamer.freedesktop.org/documentation/tools/gst-play-1.0.html) | All | ❌ | ✅ | ✅ | ✅ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [paplay](https://manpages.ubuntu.com/manpages/trusty/man1/paplay.1.html) | Linux | ❌ | ✅ | ❌ | ❌ | PCM | WAV |
| [PipeWire pw-play](https://docs.pipewire.org/page_man_pw-cat_1.html) | Linux | ❌ | ✅ | ❌ | ❌ | PCM, FLAC | WAV, FLAC |
| [mpg123](https://www.mpg123.de/) | All | ❌ | ❌ | ✅ | ❌ | MP3 | MP3 |
| [pacat](https://www.freedesktop.org/wiki/Software/PulseAudio/) | Linux | ❌ | ❌ | ✅ | ❌ | PCM | WAV |
| [ogg123](https://github.com/xiph/vorbis-tools) | All | ❌ | ❌ | ❌ | ❌ | Vorbis, Opus, FLAC | OGG |
| [aplay](https://linux.die.net/man/1/aplay) | Linux | ❌ | ❌ | ❌ | ❌ | PCM | WAV |


## Components

- Library [`playa/lib/README.md`](./lib/README.md)
- CLI: [`playa/cli/README.md`](./cli/README.md)

## Overview

This library uses an enabled native backend first and capability-ranked host
software as fallback. Callers may force or explicitly select a host player when
that route is required.
