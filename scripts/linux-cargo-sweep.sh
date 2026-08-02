#!/usr/bin/env bash
# Schedule scripts/sweep.sh on a Linux host. Counterpart to
# scripts/windows-cargo-sweep.ps1, which schedules the native Windows policy
# against C:\ and does not see a WSL2 guest's filesystem at all.
#
# Usage: linux-cargo-sweep.sh <run|install|uninstall|status> [ROOT ...]
#
# Prefers a systemd *user* timer and falls back to cron. The distinction that
# matters is missed runs: a dev host is routinely powered off at 04:00, and a
# plain cron entry simply skips those days -- which is how a host reaches 92%
# full with a sweep policy nominally in place. `Persistent=true` runs the timer
# on next boot instead. Cron remains the fallback for hosts without a usable
# systemd user instance (build-linux: `~/.config` is a read-only CIFS mount).
set -euo pipefail

readonly UNIT_NAME="rusty-biscuit-sweep"
readonly CRON_MARKER="# rusty-biscuit-sweep (managed by scripts/linux-cargo-sweep.sh)"

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(cd "$script_dir/.." && pwd -P)
log_path="${SWEEP_LOG:-${XDG_STATE_HOME:-$HOME/.local/state}/rusty-biscuit-sweep.log}"

# Unit files go under XDG_DATA_HOME rather than XDG_CONFIG_HOME because on this
# family of hosts `~/.config` is a CIFS mount: units parked there vanish from
# systemd's view whenever the share is down at boot. `~/.local/share` is local
# disk and is an equally valid user-unit search path.
unit_dir="${XDG_DATA_HOME:-$HOME/.local/share}/systemd/user"

operation="${1:-status}"
shift || true
roots=("$@")
[[ ${#roots[@]} -gt 0 ]] || roots=("$repo_root")

have_systemd_user() {
    command -v systemctl > /dev/null 2>&1 && systemctl --user show-environment > /dev/null 2>&1
}

case "$operation" in
    run)
        # Both schedulers hand this script a minimal PATH that omits Cargo's bin
        # directory, so sweep.sh's `cargo sweep` calls would exit 127. Sourced
        # here rather than pinned into the systemd unit so the cron path gets it
        # too, and so a relocated CARGO_HOME still resolves.
        # shellcheck source=scripts/cargo-path.sh
        source "$script_dir/cargo-path.sh"

        # Sweeping a target/ that a build is actively writing removes artifacts
        # out from under rustc and fails the build. The scheduled path must
        # yield; a hand-typed `just sweep` deliberately does not, matching the
        # equivalent guard in windows-cargo-sweep.ps1.
        if busy=$(pgrep -a -x 'cargo|rustc|clippy-driver|cargo-nextest' 2> /dev/null) && [[ -n "$busy" ]]; then
            echo "sweep: skipped, build processes are active"
            exit 0
        fi
        mkdir -p "$(dirname "$log_path")"
        {
            echo "=== $(date -Is) ==="
            "$script_dir/sweep.sh" "${roots[@]}" 2>&1
        } >> "$log_path"
        # Keep the log from growing without bound, which is the failure this
        # whole script exists to prevent.
        if [[ $(wc -l < "$log_path") -gt 2000 ]]; then
            tail -n 1000 "$log_path" > "$log_path.tmp" && mv "$log_path.tmp" "$log_path"
        fi
        tail -n 20 "$log_path"
        ;;

    install)
        if have_systemd_user; then
            mkdir -p "$unit_dir"
            cat > "$unit_dir/$UNIT_NAME.service" << EOF
[Unit]
Description=Prune rusty-biscuit Cargo target directories
Documentation=file://$repo_root/docs/kache-strategy.md

[Service]
Type=oneshot
WorkingDirectory=$repo_root
ExecStart=$script_dir/linux-cargo-sweep.sh run ${roots[*]}
EOF
            cat > "$unit_dir/$UNIT_NAME.timer" << EOF
[Unit]
Description=Daily rusty-biscuit Cargo sweep

[Timer]
OnCalendar=*-*-* 04:00:00
RandomizedDelaySec=30m
Persistent=true

[Install]
WantedBy=timers.target
EOF
            # `systemctl --user enable` insists on writing its .wants symlink
            # under XDG_CONFIG_HOME, which fails outright on a CIFS `~/.config`
            # ("Operation not supported" -- the share has no symlink support).
            # Linking it by hand into the unit dir reaches the same activated
            # state: systemd honours .wants in every directory on the user unit
            # search path, not just the config one.
            mkdir -p "$unit_dir/timers.target.wants"
            ln -sf "$unit_dir/$UNIT_NAME.timer" "$unit_dir/timers.target.wants/$UNIT_NAME.timer"
            systemctl --user daemon-reload
            systemctl --user start "$UNIT_NAME.timer"

            # Without lingering, the user systemd instance is torn down at
            # logout and the timer only advances while a session happens to be
            # open. Not fatal, so a refusal here is reported rather than raised.
            if ! loginctl show-user "$USER" --property=Linger 2> /dev/null | grep -q 'Linger=yes'; then
                loginctl enable-linger "$USER" 2> /dev/null \
                    || echo "sweep: could not enable linger; run 'sudo loginctl enable-linger $USER' so the timer fires without an open session" >&2
            fi
            echo "Installed systemd user timer '$UNIT_NAME.timer' (daily, catches up missed runs)."
        else
            entry="$script_dir/linux-cargo-sweep.sh run ${roots[*]}"
            { crontab -l 2> /dev/null | grep -vF "$CRON_MARKER" | grep -vF "$entry" || true
              echo "$CRON_MARKER"
              echo "0 4 * * * $entry"
            } | crontab -
            echo "Installed cron entry (daily 04:00). No systemd user instance, so missed runs are skipped."
        fi
        echo "Log: $log_path"
        ;;

    uninstall)
        if have_systemd_user && [[ -f "$unit_dir/$UNIT_NAME.timer" ]]; then
            systemctl --user stop "$UNIT_NAME.timer" 2> /dev/null || true
            rm -f "$unit_dir/timers.target.wants/$UNIT_NAME.timer"
            rm -f "$unit_dir/$UNIT_NAME.timer" "$unit_dir/$UNIT_NAME.service"
            systemctl --user daemon-reload
            echo "Removed systemd user timer '$UNIT_NAME.timer'."
        fi
        if crontab -l 2> /dev/null | grep -qF "$CRON_MARKER"; then
            crontab -l 2> /dev/null | grep -vF "$CRON_MARKER" | grep -v 'linux-cargo-sweep.sh run' | crontab -
            echo "Removed cron entry."
        fi
        ;;

    status)
        if have_systemd_user && [[ -f "$unit_dir/$UNIT_NAME.timer" ]]; then
            systemctl --user list-timers "$UNIT_NAME.timer" --all --no-pager
            loginctl show-user "$USER" --property=Linger 2> /dev/null || true
        elif crontab -l 2> /dev/null | grep -qF "$CRON_MARKER"; then
            crontab -l | grep -A1 -F "$CRON_MARKER"
        else
            echo "sweep: no schedule installed on this host (run 'just install-linux-sweep')"
        fi
        if [[ -f "$log_path" ]]; then
            echo "--- last run ---"
            tail -n 5 "$log_path"
        fi
        ;;

    *)
        echo "usage: linux-cargo-sweep.sh <run|install|uninstall|status> [ROOT ...]" >&2
        exit 2
        ;;
esac
