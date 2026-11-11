"""Print the four CI workflows with their presentation removed.

Presentation is the workflow title, each job's `name:`, and each step's
`name:`. Everything else is protected structure and is printed as canonical
JSON, so a rename-only change leaves the output byte-identical. Run from the
repository root; see `implementation.md` for the comparison command.
"""

import json
import sys
from pathlib import Path

import yaml

WORKFLOWS = ("ci.yml", "_area-ci.yml", "_package-ci.yml", "_wsl-ci.yml")


def keyed(value):
    # PyYAML reads the bare `on:` key as the boolean True; JSON keys must be
    # strings, and sorting needs one key type.
    if isinstance(value, dict):
        return {str(key): keyed(item) for key, item in value.items()}
    if isinstance(value, list):
        return [keyed(item) for item in value]
    return value


def structure(document):
    document = keyed(document)
    document.pop("name", None)
    for job in document.get("jobs", {}).values():
        job.pop("name", None)
        for step in job.get("steps", []):
            step.pop("name", None)
    return document


def main():
    root = Path(".github/workflows")
    if not root.is_dir():
        sys.exit("run from the repository root")
    projection = {
        name: structure(yaml.safe_load((root / name).read_text(encoding="utf-8")))
        for name in WORKFLOWS
    }
    print(json.dumps(projection, indent=1, sort_keys=True))


if __name__ == "__main__":
    main()
