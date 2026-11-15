#!/usr/bin/env python3
"""Configure and verify the `permission-widening` deployment environment.

CI pauses a run whose change may widen a repository's permissions until a
reviewer approves it. GitHub implements that pause as a deployment
environment with required reviewers. This script is how the environment is
set up and checked; it never approves a deployment.

* `plan` (the default) is read-only: it reads the environment, reports how
  it differs from the required protection, and prints the exact request
  `--apply` would send. Nothing is changed.
* `plan --apply` sends that request (`PUT .../environments/{name}`), then
  reads the environment back and fails unless it now matches.
* `verify` is read-only and exits 1 unless the environment exists with the
  required protection.

Requests go through the existing non-interactive `gh` authentication
(`GH_TOKEN` or `gh auth`); this script never reads, prints, or stores a
credential. User-facing guide: `docs/topics/ci-cd.md`
("Permission-widening approval").

## Examples

```bash
python3 scripts/ci/permission_environment.py            # dry run
python3 scripts/ci/permission_environment.py --apply    # configure, then verify
python3 scripts/ci/permission_environment.py verify     # read-only check
```
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections.abc import Callable
from typing import Any

REPOSITORY = "yankeeinlondon/rusty-biscuit"
ENVIRONMENT = "permission-widening"
# The people allowed to merge: today only the repository owner.
REVIEWERS = ({"type": "User", "id": 980757, "login": "yankeeinlondon"},)

# The one request that configures the environment. `prevent_self_review` is
# off because the only eligible reviewer authors every change; what the gate
# needs is a deliberate approval, not a second person. Admins may not bypass
# it, no wait timer delays it, and any branch may request it.
PAYLOAD: dict[str, Any] = {
    "wait_timer": 0,
    "prevent_self_review": False,
    "reviewers": [{"type": reviewer["type"], "id": reviewer["id"]} for reviewer in REVIEWERS],
    "can_admins_bypass": False,
    "deployment_branch_policy": None,
}

Api = Callable[[str, str, "dict[str, Any] | None"], Any]


class ApiError(Exception):
    """A GitHub request failed; `status` is the HTTP status when known."""

    def __init__(self, message: str, status: int | None = None) -> None:
        super().__init__(message)
        self.status = status


def environment_endpoint(repository: str = REPOSITORY, environment: str = ENVIRONMENT) -> str:
    return f"repos/{repository}/environments/{environment}"


def gh_api(method: str, endpoint: str, payload: dict[str, Any] | None = None) -> Any:
    """One `gh api` request; never prompts and never prints its credential."""
    command = ["gh", "api", "--method", method, endpoint, "-H", "Accept: application/vnd.github+json"]
    if payload is not None:
        command += ["--input", "-"]
    completed = subprocess.run(
        command,
        input=json.dumps(payload) if payload is not None else None,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if completed.returncode != 0:
        status = None
        for token in completed.stderr.split():
            if token.strip("():").isdigit() and len(token.strip("():")) == 3:
                status = int(token.strip("():"))
                break
        raise ApiError(f"gh api {method} {endpoint} failed: {completed.stderr.strip()}", status)
    return json.loads(completed.stdout) if completed.stdout.strip() else None


def read_environment(api: Api, endpoint: str) -> dict[str, Any] | None:
    """The environment, or `None` when it does not exist (HTTP 404)."""
    try:
        return api("GET", endpoint, None)
    except ApiError as error:
        if error.status == 404:
            return None
        raise


def problems(environment: dict[str, Any] | None) -> list[str]:
    """How `environment` falls short of the required protection; empty when it
    matches exactly. Weaker and missing settings are both problems."""
    if environment is None:
        return [f"the `{ENVIRONMENT}` environment does not exist"]
    found: list[str] = []
    rules = environment.get("protection_rules") or []
    reviewers_rules = [rule for rule in rules if rule.get("type") == "required_reviewers"]
    if len(reviewers_rules) != 1:
        found.append("it has no required-reviewers rule")
    else:
        rule = reviewers_rules[0]
        actual = sorted(
            (entry.get("type"), (entry.get("reviewer") or {}).get("id")) for entry in rule.get("reviewers") or []
        )
        expected = sorted((reviewer["type"], reviewer["id"]) for reviewer in REVIEWERS)
        if actual != expected:
            found.append(f"its required reviewers are {actual}, not exactly {expected}")
        if rule.get("prevent_self_review") is not False:
            found.append("`prevent_self_review` must be off, so the only reviewer can approve")
    for rule in rules:
        if rule.get("type") == "wait_timer" and rule.get("wait_timer", 0) != 0:
            found.append(f"its wait timer is {rule.get('wait_timer')} minutes, not 0")
        if rule.get("type") == "branch_policy":
            found.append("it restricts deployment branches, so a pull request could not request it")
    if environment.get("can_admins_bypass") is not False:
        found.append("administrators can bypass it (`can_admins_bypass` must be false)")
    if environment.get("deployment_branch_policy") is not None:
        found.append("it restricts deployment branches, so a pull request could not request it")
    return list(dict.fromkeys(found))


def apply_command(repository: str = REPOSITORY) -> str:
    """The exact command `--apply` runs, for the docs and a dry run."""
    return (
        f"gh api --method PUT {environment_endpoint(repository)} "
        f"-H 'Accept: application/vnd.github+json' --input - <<'JSON'\n"
        f"{json.dumps(PAYLOAD, indent=2)}\nJSON"
    )


def plan(api: Api, apply: bool, out: Callable[[str], None] = print) -> int:
    endpoint = environment_endpoint()
    current = read_environment(api, endpoint)
    found = problems(current)
    if not found:
        out(f"`{ENVIRONMENT}` already has the required protection; nothing to change.")
        return 0
    out(f"`{ENVIRONMENT}` differs from the required protection:")
    for problem in found:
        out(f"  - {problem}")
    if not apply:
        out("Dry run: nothing was changed. `--apply` sends this request:")
        out(apply_command())
        return 0
    api("PUT", endpoint, PAYLOAD)
    remaining = problems(read_environment(api, endpoint))
    if remaining:
        out("The request was accepted, but the environment still differs:")
        for problem in remaining:
            out(f"  - {problem}")
        return 1
    out(f"`{ENVIRONMENT}` now has the required protection.")
    return 0


def verify(api: Api, out: Callable[[str], None] = print) -> int:
    found = problems(read_environment(api, environment_endpoint()))
    for problem in found:
        out(f"permission-widening: {problem}")
    if found:
        out("Set it up with `python3 scripts/ci/permission_environment.py --apply`.")
        return 1
    out(f"`{ENVIRONMENT}` has the required protection.")
    return 0


def main(argv: list[str] | None = None, api: Api = gh_api, out: Callable[[str], None] = print) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("command", nargs="?", choices=("plan", "verify"), default="plan")
    parser.add_argument("--apply", action="store_true", help="configure the environment (plan only)")
    arguments = parser.parse_args(argv)
    if arguments.apply and arguments.command != "plan":
        parser.error("--apply only configures; `verify` is read-only")
    try:
        if arguments.command == "verify":
            return verify(api, out)
        return plan(api, arguments.apply, out)
    except ApiError as error:
        out(f"permission-widening: {error}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
