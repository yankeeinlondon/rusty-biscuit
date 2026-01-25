# Sniff

> System Sniffer (lib and CLI)

<table>
<tr>
<td><img src="../assets/sniff-512.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>Sniff</h2>
<p>A shared library and CLI which </p>

<ul>
  <li>hardware, network, filesystem, programs</li>
  <li>cross-platform</li>
  <li>automatic failover between providers</li>
</ul>

<p>
  This library provides the sound effects found in the <code>so-you-say</code> and <code>effect</code> CLIs.
</p>
</td>
</tr>
</table>

## Modules

1. Sniff Library `sniff/lib`

    - a set of utilities for detecting information about the host computer
    - leverages the `hardware-query`, `sysinfo`, and `getifaddrs` to detect hardware and network setup on the host machine
    - we use `sysinfo`

2. Sniff CLI `sniff/cli`

    - a simple CLI which exposes the metadata which the **Sniff** library can procure.

## CLI Usage

**Syntax:**

> **sniff** [_options_]


### Switches

- `--base`, `-b` \<dir\>

    - some utilities are evaluated from the perspective of the ${CWD} so using this switch allows you to relocate to a different directory


