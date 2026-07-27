# Playa

<table>
<tr>
<td><img src="../assets/playa.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Playa</h2>
<p>This library leverages the host to play audio natively or via a headless audio player installed on the host.</p>

<ul>
    <li>small library (<i>native playback is off by default; opt in per feature</i>)</li>
    <li>audio format detection from files, URLs, or bytes</li>
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
> from the `native` declaration in `.github/ci/areas.json`. To install them by
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

The OS-native backends all sit behind one feature, `sfx-native-audio`, which the
CLI enables by default; `target_os` decides which of them compiles in. To build
without them — and without any ALSA / PulseAudio requirement — opt out:

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
```

Speed and volume are `--fast` / `--slow` / `--speed <MULTIPLIER>` and `--quiet` /
`--loud` / `--volume <LEVEL>`. `--channel <CHANNEL>` picks an output device and
`--force-host` skips the native decoder in favor of a host player.

Full flag and subcommand reference: [`playa/cli/README.md`](./cli/README.md).



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

This library is meant to leverage existing software residing on the host computer for audio playback. While you can specify a provider to use the most common situation is to just provide the library or CLI some audio (a file or a stream) and let it detect what the best software would be to use.
