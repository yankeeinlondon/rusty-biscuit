#!/usr/bin/env bash
set -euo pipefail

operation="$1"
script_dir=$(CDPATH= cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd -P)
label=com.ken.rusty-biscuit-sweep
uid=$(id -u)
domain="gui/$uid"
plist="$HOME/Library/LaunchAgents/$label.plist"
log="$HOME/Library/Logs/rusty-biscuit-sweep.log"

case "$operation" in
    run)
        source "$script_dir/cargo-path.sh"
        exec bash "$script_dir/sweep.sh" "$repo_root"
        ;;
    install | uninstall) ;;
    *) printf 'Unknown sweep schedule operation: %s\n' "$operation" >&2; exit 2 ;;
esac

if ! launchctl print "$domain" >/dev/null 2>&1; then
    domain="user/$uid"
fi

if [[ "$operation" == uninstall ]]; then
    launchctl bootout "$domain/$label" >/dev/null 2>&1 || true
    rm -f "$plist"
    echo "Removed the macOS sweep schedule."
    exit 0
fi

mkdir -p "$(dirname "$plist")" "$(dirname "$log")"
temporary=$(mktemp "$plist.XXXXXX")
trap 'rm -f "$temporary"' EXIT
python3 - "$temporary" "$label" "$script_dir/macos-cargo-sweep.sh" "$repo_root" "$log" <<'PYTHON'
import plistlib
import sys

path, label, script, root, log = sys.argv[1:]
with open(path, "wb") as output:
    plistlib.dump({
        "Label": label,
        "ProgramArguments": ["/bin/bash", script, "run"],
        "WorkingDirectory": root,
        "StartCalendarInterval": [{"Weekday": day, "Hour": 4, "Minute": 0} for day in (0, 3)],
        "StandardOutPath": log,
        "StandardErrorPath": log,
    }, output)
PYTHON
plutil -lint "$temporary" >/dev/null
launchctl bootout "$domain/$label" >/dev/null 2>&1 || true
mv -f "$temporary" "$plist"
launchctl bootstrap "$domain" "$plist"
echo "Installed the macOS sweep schedule for Sunday and Wednesday at 04:00."
