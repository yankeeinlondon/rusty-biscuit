# Sniff: Installation and better Type Safety

This feature set will:

- **Sniff Library**
    - improve the type safety of the Sniff library
    - allows the "program" module to not only detect which programs a host has installed but also INSTALL those programs
    - complete the metadata aspects of the "program" module which had been left out before
        - this includes the `.description()` and `.website()` functions that ALL "installed programs" structs are meant to include.
- **CLI**
    - Take advantage of the stronger type guarantees across the various "installed programs" structs to get better code reuse and more consistent outputs
    - Leverage the ``


## Type Safety

We have have a bunch of structs such as `InstalledTerminalApps`, `TtsClients`, `HeadlessAudio`, etc. which all follow the same formula for testing and exposing a usable API surface. Unfortunately the API surface is **not** guaranteed across these various structs currently.

In this release we will both _extend_ the API surface and _guarantee_ it with the introduction of the [`ProgramDetector` trait](../lib/programs/types.rs).


## Installation

An important part of the now guaranteed API surface is the `installable()`,  `install()` and `install_version()` functions which **ProgramDetector** must implement.
