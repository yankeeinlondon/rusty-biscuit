import 'notify.just'

bold := '\033[1m'
reset := '\033[0m'
italic := '\033[3m'
red := '\033[31m'
green := '\033[32m'
yellow := '\033[33m'

# Build the package
_build package pkg_name *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "🏃‍♂️ Building {{bold}}{{pkg_name}}{{reset}} {{italic}}on {{reset}}{{os()}}"
    echo
    cargo build -p {{package}} {{args}} || ( echo -e "{{red}}ERROR:{{reset}} Failed to build {{bold}}{{pkg_name}}{{reset}}" && just _speak "The {pkg_name}} failed to build!" && exit 1)
    echo
    echo -e "✅ {{bold}}{{green}}{{pkg_name}}{{reset}} {{italic}}built successfully!{{reset}}"
    echo


_install path pkg_name *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Installing {{yellow}}{{bold}}{{pkg_name}}{{reset}}"
    echo
    cargo install --path {{path}} {{args}} || ( just _play error-2 && just _speak "Problems encountered when trying to install {{pkg_name}}" && exit 1 )
    echo
    echo -e "✅ {{bold}}{{green}}{{pkg_name}} installed!{{reset}}"
    just _play drop-4 || exit 0
