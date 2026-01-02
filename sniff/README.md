# Sniff

> System Sniffer (lib and CLI)

## Modules

1. Sniff Library `sniff/lib`

    - a set of utilities for detecting information about the host computer
    - leverages the `hardware-query`, `sysinfo`, and `getifaddrs` to detect hardware and network setup on the host machine
    - 

2. Sniff CLI `sniff/cli`

    - a simple CLI which exposes the metadata which the **Sniff** library can procure.

## CLI Usage

**Syntax:**

> **sniff** [_options_]


### Switches

- `--base`, `-b` \<dir\>

    - some utilities are evaluated from the perspective of the ${CWD} so using this switch allows you to relocate to a different directory

