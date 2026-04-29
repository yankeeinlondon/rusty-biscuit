We have implemented desktop notifications for macOS, Windows, and Linux but the amount of control we have over the notifications is less than ideal. Fortunately there are "helper utilities" on all OS's which can allows us to do a lot more.

This feature will provide a cross-OS implementation that will:

- _detect_ the existence of these utilities on the host system
- CLI
    - _report_ on the host's current notification state
    - provide the ability to _install_ utilities 
- _leverage_ the utilities to provide better desktop notifications

The amount of notification control you have can be less than ideal at time, however, each OS has some helper programs that can be leveraged to get more mileage.

## Detection

We will add the utility detection to the **Sniff** library:

- the **Sniff** library already has great facilities for detecting and installing application
- the CLI will add the `messenger info` command which will report on:
    - Host OS
    - Notification helpers for OS
    - Other targets which have configurations

## Installation

The CLI will add a `messenger install` command which will leverage the Sniff library to provide the appropriate install plan.

## Configuration

Being able to take advantage of "helper" utilities does not REQUIRE any configuration:

- if the host's OS has one of the recognized helpers then we will leverage additional functionality
- if the host has more than one of the recognized helpers then we will use the _prioritized_ provider for the OS

- Even though we don't NEED configuration, we should probably offer an optional "prefers" configuration option which can be
  set for each OS so that the user can override the default prioritization.
- Library callers should be able to dictate which helpers they prioritize too.
