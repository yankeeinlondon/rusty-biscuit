

# ask AI via Claudine
ask prompt agent="claude":
    #!/usr/bin/env bash
    set -euo pipefail
    claudine {{agent}} "{{prompt}}" --silent 2>/dev/null
