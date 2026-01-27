# Playa

<table>
<tr>
<td><img src="../assets/playa.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Playa</h2>
<p>This library leverages the host to play audio through an available headless audio program:</p>

<ul>
    <li>small library and CLI binaries</li>
    <li>includes <i>cloud</i> support for <a href="https://elevenlabs.io/docs/overview/intro">ElevenLabs</a> TTS</li>
    <li>automatic failover between providers</li>
</ul>

<p>
    This Playa library is the audio playback functionality behind the <code>so-you-say</code>, <code>effect</code>, and <code>playa</code> CLI's.
</p>
</td>
</tr>
</table>

## Audio Providers

| Software | OS | Speed | Vol | Stream In | Stream Out | Codecs | File Formats |
|----------|:--:|:-----:|:---:|:---------:|:----------:|--------|--------------|
| [mpv](https://mpv.io/) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [FFplay](https://www.ffmpeg.org/ffplay.html) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [VLC](https://wiki.videolan.org/VLC_command-line_help/) | All | ✅ | ✅ | ✅ | ✅ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [MPlayer](https://www.mplayerhq.hu/) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [GStreamer gst-play](https://gstreamer.freedesktop.org/documentation/tools/gst-play-1.0.html) | All | ✅ | ✅ | ✅ | ✅ | PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus | WAV, AIFF, FLAC, MP3, OGG, M4A, WebM |
| [SoX play](https://linux.die.net/man/1/sox) | All | ✅ | ✅ | ✅ | ❌ | PCM, FLAC, MP3, Vorbis | WAV, FLAC, OGG, MP3 |
| [mpg123](https://www.mpg123.de/) | All | ❌ | ❌ | ✅ | ❌ | MP3 | MP3 |
| [ogg123](https://github.com/xiph/vorbis-tools) | All | ❌ | ❌ | ✅ | ❌ | Vorbis, Opus, FLAC | OGG |
| [aplay](https://linux.die.net/man/1/aplay) | Linux | ❌ | ❌ | ❌ | ❌ | PCM | WAV |
| [afplay](https://ss64.com/osx/afplay.html) | macOS | ✅ | ✅ | ❌ | ❌ | PCM, FLAC, ALAC, MP3, AAC | WAV, AIFF, FLAC, MP3, M4A |
| [paplay](https://manpages.ubuntu.com/manpages/trusty/man1/paplay.1.html) | Linux | ❌ | ✅ | ❌ | ❌ | PCM | WAV |
| [pacat](https://www.freedesktop.org/wiki/Software/PulseAudio/) | Linux | ❌ | ❌ | ✅ | ❌ | PCM | WAV |
| [PipeWire pw-play](https://docs.pipewire.org/page_man_pw-cat_1.html) | Linux | ❌ | ✅ | ❌ | ❌ | PCM, FLAC | WAV, FLAC |


## Components

- Library [`playa/lib/README.md`](./lib/README.md)
- CLI: [`playa/cli/README.md`](./cli/README.md)

## Overview

This library is meant to leverage existing software residing on the host computer for audio playback. While you can specify a provider to use the most common situation is to just provide the library or CLI some audio (a file or a stream) and let it detect what the best software would be to use.
