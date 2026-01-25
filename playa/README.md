# Playa

<table>
<tr>
<td><img src="../assets/playa.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Playa</h2>
<p>A shared library that helps you leverage the host's installed applications to play audio files.</p>

<ul>
  <li>macOS, Windows, and Linux</li>
  <li>can leverage a,b,c</li>
  <li>automatic failover between providers</li>
</ul>

<p>
  This library provides the sound effects found in the <code>so-you-say</code> and <code>effect</code> CLIs.
</p>
</td>
</tr>
</table>

## Micro library supporting Music and Sound Effects

This library provides the ability to pass an audio file or stream into the exposed `playa()` function and have the audio rendered on the host system by leveraging software that already exists on the host. This keeps `Playa` nice and lean but allows simple cross-platform audio and sound effects to be played effortlessly.

### Headless Audio Players Supported

| Software                                                                                    | License                                                 | Windows | MacOS                 | Linux | Formats                                                       | Description                                                                                              |
|---------------------------------------------------------------------------------------------|---------------------------------------------------------|---------|-----------------------|-------|---------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| [mpv](https://mpv.io/manual/stable/)                                                        | GPL-2.0+ (some parts LGPL-2.1+)                         | Yes     | Yes                   | Yes   | Broad (FFmpeg-backed; varies by build)                        | Widely used CLI media player that works well headless for audio-only playback.                           |
| [ffplay (FFmpeg)](https://www.ffmpeg.org/ffplay.html)                                       | LGPL-2.1+ or GPL-2+ (depends on build)                  | Yes     | Yes                   | Yes   | Broad (depends on linked FFmpeg build)                        | Minimal CLI player shipped with FFmpeg; common on hosts where FFmpeg is installed.                       |
| [VLC (cvlc)](https://wiki.videolan.org/VLC_command-line_help/)                              | GPL-2.0-or-later (VLC app); libVLC is LGPL-2.0-or-later | Yes     | Yes                   | Yes   | Very broad (many containers/codecs)                           | Ubiquitous media engine; `cvlc` supports headless/dummy-interface playback from the command line.        |
| [MPlayer](https://www.mplayerhq.hu/DOCS/HTML/en/index.html)                                 | GPL-2.0                                                 | Yes     | Yes                   | Yes   | Broad (varies by build)                                       | Classic CLI-oriented player; still commonly available on Unix-like hosts.                                |
| [GStreamer (gst-play)](https://gstreamer.freedesktop.org/documentation/tools/gst-play.html) | LGPL-2.1-or-later                                       | Yes     | Yes                   | Yes   | Plugin-based (varies by installed plugins)                    | CLI front-end to GStreamer pipelines; common for headless playback when GStreamer is installed.          |
| [SoX (play)](https://linux.die.net/man/1/sox)                                               | GPL-2.0-or-later                                        | Yes     | Yes                   | Yes   | Many audio formats (varies by build)                          | CLI “Swiss-army knife” for audio; `play` provides straightforward headless playback.                     |
| [mpg123](https://www.mpg123.de/)                                                            | LGPL-2.1                                                | Yes     | Yes                   | Yes   | MP3 (MPEG audio layers 1/2/3)                                 | Lightweight console MP3 player/decoder library; widely packaged and frequently present.                  |
| [ogg123 (vorbis-tools)](https://github.com/xiph/vorbis-tools)                               | GPL-2.0                                                 | Limited | Yes (via ports/build) | Yes   | Ogg Vorbis (optionally other Xiph formats depending on build) | CLI player focused on Ogg/Vorbis toolchains; often found where Xiph tools are installed.                 |
| [aplay (ALSA)](https://linux.die.net/man/1/aplay)                                           | GPL-2.0                                                 | No      | No                    | Yes   | PCM-focused: WAV/VOC/RAW/AU                                   | Ubiquitous low-level ALSA playback utility on Linux; common for testing/output in headless environments. |
| [paplay (PulseAudio)](https://manpages.ubuntu.com/manpages/trusty/man1/paplay.1.html)       | LGPL (PulseAudio project)                               | Limited | Limited               | Yes   | libsndfile-supported formats (varies by libsndfile build)     | Simple headless playback to a PulseAudio server; common on Linux systems with PulseAudio utilities.      |
| [pw-cat / pw-play (PipeWire)](https://docs.pipewire.org/page_man_pw-cat_1.html)             | MIT                                                     | No      | No                    | Yes   | libsndfile-supported formats (varies by libsndfile build)     | PipeWire CLI playback/capture tool (`pw-play` is commonly an alias for `pw-cat --playback`).             |

### Usage

```rust

```
