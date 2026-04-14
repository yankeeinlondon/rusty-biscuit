import 'notify.just'

bold := '\033[1m'
reset := '\033[0m'
italic := '\033[3m'
red := '\033[31m'
green := '\033[32m'
yellow := '\033[33m'
blue := '\033[34m'

dim := '\033[2m'

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


# commits all the staged changes with OpenCode (model is MODEL or COMMIT_MODEL)
commit *args="":
    @echo ""
    @echo -e "Committing staged changes in the {{bold}}Rusty Biscuit{{reset}} monorepo to git"
    @echo -e "{{dim}}{{italic}}- using the {{reset}}{{italic}}${MODEL:-${COMMIT_MODEL:-minimax/MiniMax-M2.7-highspeed}} {{dim}}model{{reset}}"
    @echo ""
    @echo -e "{{bold}}{{blue}}Staged Files:{{reset}}"
    @sniff repo staged-files || ( echo "No Staged Files! Nothing to do ..." && exit 1 )
    @claudine compose --opencode "@prompts/commit.md" --op "commit" --quiet --model "${COMMIT_MODEL:-${MODEL:-minimax/MiniMax-M2.7-highspeed}}" -y {{args}}
    @just _speak "git commits completed in rusty-biscuit monorepo"
    @sniff repo git-status 2>/dev/null || exit 0
    @echo


# Lint a package
_lint pkg:
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Linting {{bold}}{{pkg}}{{reset}}"
    echo
    cargo clippy -p {{pkg}} -- -D warnings || ( just _speak "The {{pkg}} has failing lints" && echo -e "\n❌ The {{bold}}{{red}}{{pkg}}{{reset}} has failing lints!" && exit 1 )

_fix_lint pkg agent="claude" *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Fixing the lint problems in {{bold}}{{pkg}}{{reset}}"
    echo
    claudine {{agent}} "The package '{{pkg}}' in rusty-biscuit monorepo has lint errors. Run 'just lint' and fix all lint warnings/errors. Provide a full explanation for what was done." -y {{args}} || (just _speak "Failure trying to fix the lint errors in the package {{pkg}}" && exit 1)
    echo
    echo -e "✅ {{green}}The {{bold}}{{pkg}}{{reset}}{{green}} has had all lints now fixed!"
    echo
    sniff repo git-status

# Show dependencies for this package area
deps:
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    sniff repo deps "$(sniff repo package-area)"

# Run tests with `cargo test` or `cargo nextest run`
_test pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Testing {{bold}}{{pkg}}{{reset}} package"
    echo
    if cargo nextest --version >/dev/null 2>&1; then
        echo -e "ℹ️ using {{green}}cargo nextest run{{reset}}"
        cargo nextest run -p {{pkg}} {{args}} || ( just _speak "There are failing tests in the {{pkg}} package!" && echo -e "\n❌ The {{bold}}{{red}}{{pkg}}{{reset}} has failing tests!" && exit 1 )
    else
        echo -e "ℹ️ using {{green}}cargo test{{reset}}"
        cargo test -p {{pkg}} {{args}} || ( just _speak "There are failing tests in the {{pkg}} package!" && echo -e "\n❌ The {{bold}}{{red}}{{pkg}}{{reset}} has failing tests!" && exit 1 )
    fi
    echo
    echo -e "✅ {{green}} all tests passing in {{bold}}{{pkg}}{{reset}}{{green}} package{{reset}}"
    echo


# Show all README.md files in the package
_readme_files:
    @sniff docs --readme --json "$(sniff repo package-area)"

