---
name: audio-programming
description: Explore how to interact with audio pragmatically on various operating systems including macOS, Linux, Windows, IOS, and Android. Provide code examples using Rust and Typescript.
prompt: |-
    Your job is to act as an orchestrator to carry out the following operations (in this exact order):

    1. Copy the other Markdown files (all except SKILL.md) in this directory to `{repo root}/.claude/skills/audio-programming`
    2. Save the `{repo root}/.claude/skills/audio-programming/SKILL.md` from the output of running `md compose @sniff/docs/audio-programming/SKILL.md`
---

# Audio Programming by Operating System

This skill will provide detailed knowledge useful for programmatic use of the audio subsystems which modern OS's provide. Use the links provided throughout this skill document to get more details on the areas which are most relevant.

## Desktop Operating Systems

To get details on how to handle on audio on a particular desktop operating system, choose from

### macOS

The following links provide details on how to work with audio on the **macOS** operating system:

::toc-linking ./macOS.md level=h2 cleanup=true filter="Table of Contents"

### Windows

The following links provide details on how to work with audio on the **Windows** operating system:

::toc-linking ./windows.md level=h2 cleanup=true filter="Table of Contents"

### Linux

The following links provide details on how to work with audio on the **Linux** operating system:

::toc-linking ./linux.md level=h2 cleanup=true filter="Table of Contents"

## Mobile Operating Systems

### IOS

Apple's mobile platform **IOS**:

::toc-linking ./IOS.md level=h2 cleanup=true filter="Table of Contents"

### Android

Google's mobile platform **Android**:

::toc-linking ./Android.md level=h2 cleanup=true filter="Table of Contents"

## Software Libraries

### Rust Crates

This section will dig into the `crates` that most Rust developers will consider when developing an application that needs audio support.

::toc-linking ./crates.md level=h2 cleanup=true filter="Table of Contents"

### Typescript Libraries

This section will dig into the **npm** libraries that most Typescript developers will consider when developing an application that needs audio support.

::toc-linking ./typescript-libraries.md level=h2 cleanup=true filter="Table of Contents"
