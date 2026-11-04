"""Check coverage and evidence relations after the sandbox report's shape gate.

Usage: python3 _relations.py <research document>...
Uses PyYAML like the other research relations gates; BaseLoader preserves tokens.
"""

from collections import Counter
from itertools import product
from pathlib import Path
import re
import sys

import yaml


SECTIONS = (
    "Out-of-the-Box Posture", "Available Sandboxes", "Isolation Boundaries",
    "Configuration and Examples", "Approvals, Escalation, and Failure",
    "Limitations and Verification Gaps", "Sources", "Changelog",
)
ROTATION = {
    "opencode": ("zai-coding-plan/glm-5.3", "provider_default"),
    "claude": ("sonnet", "high"),
    "codex": ("gpt-6.1-sol", "medium"),
}


def check(path):
    text = path.read_text()
    header, body = text.removeprefix("---\n").split("\n---\n", 1)
    fm = yaml.load(header, Loader=yaml.BaseLoader)
    problems = []

    def bad(message):
        problems.append(message)

    if fm["provider"] != path.stem:
        bad("provider must match the report filename")
    if fm["agent"] == fm["provider"]:
        bad("a provider must not research itself")
    if ROTATION.get(fm["agent"]) != (fm["model"], fm["reasoning_effort"]):
        bad("agent, model, and reasoning_effort do not match the research rotation")

    evidence = {entry["id"]: entry for entry in fm["evidence"]}
    mechanisms = {entry["id"]: entry for entry in fm["mechanisms"]}
    for name in ("evidence", "default_profiles", "mechanisms", "controls"):
        for entry_id, count in Counter(entry["id"] for entry in fm[name]).items():
            if count > 1:
                bad(f"{name}/{entry_id} has a duplicate ID")

    expected = set(product(("macos", "linux", "windows", "wsl2"),
                           ("interactive", "non_interactive")))
    cases = [(entry["environment"], entry["launch_mode"])
             for entry in fm["default_profiles"]]
    if set(cases) != expected or len(cases) != len(expected):
        bad("default_profiles must cover each environment and launch mode exactly once")

    for name in ("default_profiles", "mechanisms", "controls"):
        for entry in fm[name]:
            where = f"{name}/{entry['id']}"
            refs = entry["evidence_ids"]
            unknown = entry.get("status") == "unknown"
            if not refs and not unknown:
                bad(f"{where} needs evidence, including for negative findings")
            if refs and all(evidence.get(ref, {}).get("method") == "inference" for ref in refs):
                bad(f"{where} rests on inference alone")
            for ref in refs:
                if ref not in evidence:
                    bad(f"{where} references missing evidence {ref}")
            for ref in entry.get("mechanism_ids", []):
                if ref not in mechanisms:
                    bad(f"{where} references missing mechanism {ref}")

    for profile in fm["default_profiles"]:
        refs = profile["mechanism_ids"]
        if (profile["status"] == "enabled") != bool(refs):
            bad(f"default_profiles/{profile['id']} needs mechanisms exactly when enabled")
        for ref in refs:
            if ref in mechanisms and profile["environment"] not in mechanisms[ref]["environments"]:
                bad(f"default_profiles/{profile['id']} uses {ref} in an unsupported environment")

    support = fm["support"]
    if support in ("none", "policy_only", "unknown") and mechanisms:
        bad(f"support is {support} but enforced mechanisms are listed")
    if support in ("native", "official_integration") and not mechanisms:
        bad(f"support is {support} but no mechanism is described")

    gaps = [entry["location"].strip("/") for entry in fm["gaps"]]

    def has_gap(where):
        return any(where == gap or where.startswith(gap + "/") for gap in gaps)

    def unknowns(node, where):
        if isinstance(node, dict):
            for key, value in node.items():
                unknowns(value, f"{where}/{key}".strip("/"))
        elif isinstance(node, list):
            for index, value in enumerate(node):
                key = value.get("id", str(index)) if isinstance(value, dict) else str(index)
                unknowns(value, f"{where}/{key}")
        elif node == "unknown" and not has_gap(where):
            bad(f"{where} is unknown without a gap at that path or an ancestor")

    unknowns({key: value for key, value in fm.items() if key not in ("evidence", "gaps")}, "")
    if not fm["versions_examined"] and not has_gap("versions_examined"):
        bad("versions_examined is empty without a gap explaining why")
    for section in SECTIONS:
        if not re.search(rf"(?m)^## {re.escape(section)}\s*$", body):
            bad(f"missing body section '## {section}'")
    return problems


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    failed = False
    for filename in sys.argv[1:]:
        path = Path(filename)
        try:
            problems = check(path)
        except (OSError, ValueError, KeyError, TypeError, yaml.YAMLError) as error:
            problems = [f"cannot check report; run the shape gate first: {error}"]
        print(f"{path.name}: {len(problems)} relation problems")
        for problem in problems:
            print(f"  {problem}")
        failed |= bool(problems)
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
