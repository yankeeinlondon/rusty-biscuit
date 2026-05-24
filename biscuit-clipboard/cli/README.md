# Biscuit Clipboard CLI

CLI client (`clip`) for interacting with the `clipper` clipboard service.

## Autostart

`clip service install` writes a per-user autostart manifest so `clipper`
launches at login.

| Platform | Manifest path |
|----------|---------------|
| macOS    | `~/Library/LaunchAgents/com.biscuit.clipper.plist` |
| Linux    | `~/.config/systemd/user/clipper.service` |
| Windows  | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\clipper.cmd` |

Common flags:

- `--dry-run` — render the manifest body to stdout without touching disk.
- `--binary <path>` — override the absolute path embedded in the manifest
  (defaults to whichever `clipper` binary `clip` would auto-spawn).
- `--prefix <path>` — override the install root (also honoured by the
  `CLIP_AUTOSTART_PREFIX` env var). Mostly useful for testing.

`clip service install` is **idempotent** — running it twice prints
"already present" and exits successfully without rewriting the file.
`clip service uninstall` reverses the operation and is also idempotent.

After install, the next-step hint shown on stderr tells you how to
activate the manifest immediately (e.g. `launchctl load ...` on macOS,
`systemctl --user enable --now clipper` on Linux); otherwise it kicks in
on next login.

## Notes

- Autostart is **per-user** only. System-wide (multi-user, root-owned)
  install is out of scope for v1; manifests always live under the
  current user's home / `%APPDATA%`.
- `clip service install` does **not** invoke `launchctl` /
  `systemctl` / `schtasks` for you in v1; it only writes the manifest
  file. This is intentional — those commands behave differently across
  distributions and macOS releases, and we'd rather print the exact
  next step than silently get it wrong.
- Targets other than macOS, Linux, and Windows return
  `autostart is not supported on this platform`.
