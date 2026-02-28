---
name: audio-programming
description: Explore how to interact with audio pragmatically on various operating systems including macOS, Linux, Windows, IOS, and Android. Provide code examples using Rust and Typescript.
prompt: |-
    Your job is to act as an orchestrator to carry out the following operations (in this exact order):

    1. Copy the other Markdown files in this directory to `@.claude/skills/audio-programming`
    2. Save the `@.claude/skills/audio-programming/SKILL.md` from the output of running `md compose @sniff/docs/audio-programming/SKILL.md`
---

# Audio Programming by Operating System

This skill will provide detailed knowledge useful for programmatic use of the audio subsystems which modern OS's provide. Use the links provided throughout this skill document to get more details on the areas which are most relevant.

## Desktop Operating Systems

To get details on how to handle on audio on a particular desktop operating system, choose from

### macOS

The following links provide details on how to work with audio on the **macOS** operating system:

::toc-links ./macOS.md level=h2

### Windows

The following links provide details on how to work with audio on the **Windows** operating system:

::toc-links ./windows.md level=h2

### Linux

The following links provide details on how to work with audio on the **Linux** operating system:

::toc-links ./linux.md level=h2

## Mobile Operating Systems

### IOS

Apple's mobile platform **IOS**:

::toc-links ./IOS.md level=h2

### Android

Google's mobile platform **Android**:

::toc-links ./Android.md level=h2

## Software Libraries

### Rust Crates

This section will dig into the `crates` that most Rust developers will consider when developing an application that needs audio support.

::toc-links ./crates.md level=h2

### Typescript Libraries

This section will dig into the **npm** libraries that most Typescript developers will consider when developing an application that needs audio support.

::toc-links ./typescript-libraries.md level=h2
