#!/usr/bin/env python3
"""Resolve one dispatch row into the execution contract its job runs under.

A row carries dispatch identity alone — `{package, gate, environment, runner}`
— because that is what GitHub renders a matrix job's label from (ruling R1).
Everything else the job needs is already in the resolved plan, so this is the
one reader every producer uses: it joins the row back to its cell, refuses a
row the plan does not schedule, and emits the cell's contract as step outputs.

No job reads `.github/ci/environments.json`, a package manifest, or a Cargo
policy block. The plan is the only input, which is what lets a run that carried
a scope receipt dispatch without re-reading the checkout it was planned from.

Python 3 stdlib only, for the reason `completion.py` is (ruling R2): the WSL2
guest has a thin apt set and no compiler.

## Examples

```bash
python3 scripts/ci/cell_contract.py \
    --plan ci-artifacts/ci-resolved-plan/resolved-plan.json \
    --row "$ROW" \
    --head "$TESTED_REVISION" \
    --github-output "$GITHUB_OUTPUT"
```

## Exit status

`0` and the contract written to `--github-output` when the row names exactly
one executing cell of this plan. `1` when it does not, naming the coded reason
on stderr. There is no partial write: a refused row publishes nothing, so a job
can never run half a contract.

## Notes

`gate` and `tier` are the same string. The plan spells a test cell's gate
`L1`, `L2`, or `browser` — the identity `ci-rollup` keys a status by — and a
compile or lint cell's gate `check` or `lint`. The recipe that proves each is
derived here so the workflow carries no gate-to-recipe table of its own.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import schema  # noqa: E402  (needs the path insert above)


#: The canonical recipe each test gate's producer runs. A gate absent here
#: drives no nextest at all, which is why its cell carries no `profile`.
GATE_RECIPES = {"L1": "_test", "L2": "_test_l2", "browser": "_test_browser"}


class Refused(Exception):
    """A row that cannot be bound to exactly one executing cell of this plan."""

    def __init__(self, code: str, detail: str) -> None:
        super().__init__(f"{code}: {detail}")
        self.code = code
        self.detail = detail


def capability(environment: dict[str, Any], name: str) -> bool:
    """Whether an environment can host a capability, boolean or governed object.

    The same rule `affected_scope.capability` applies, restated here because a
    producer must not import the planner: the planner reads a checkout, and the
    whole point of the plan artifact is that a producer does not.
    """
    value = environment.get("capabilities", {}).get(name)
    if isinstance(value, bool):
        return value
    if isinstance(value, dict):
        return bool(value.get("available"))
    return False


def parse_row(text: str) -> dict[str, str]:
    """The dispatch row, refused rather than defaulted when it is malformed."""
    try:
        row = json.loads(text)
    except json.JSONDecodeError as error:
        raise Refused("cell-contract-row", f"the row is not JSON: {error}") from error
    if not isinstance(row, dict):
        raise Refused("cell-contract-row", f"the row is not an object: {text!r}")
    missing = [field for field in schema.ROW_FIELDS if not row.get(field)]
    if missing:
        raise Refused(
            "cell-contract-row",
            f"the row omits {', '.join(missing)}; a dispatch row is "
            f"{{{', '.join(schema.ROW_FIELDS)}}}",
        )
    return {field: str(row[field]) for field in schema.ROW_FIELDS}


def resolve_cell(plan: dict[str, Any], row: dict[str, str]) -> dict[str, Any]:
    """The one executing cell this row dispatches.

    ## Errors

    Raises [`Refused`] when the plan carries no such cell, more than one, or one
    the plan did not resolve to hosted execution. All three are the same class
    of defect — a row describing work the plan does not — and each is reported
    under its own code so the log says which.
    """
    matches = [
        cell
        for cell in plan["cells"]
        if cell["package"] == row["package"]
        and cell["environment"] == row["environment"]
        and cell["gate"] == row["gate"]
    ]
    key = f"{row['package']}/{row['environment']}/{row['gate']}"
    if not matches:
        raise Refused(
            "cell-contract-unknown",
            f"the plan carries no cell {key}; this row was not derived from it",
        )
    if len(matches) > 1:
        raise Refused(
            "cell-contract-duplicate",
            f"the plan carries {len(matches)} cells for {key}; a cell key is unique",
        )
    cell = matches[0]
    if cell["execution"] != "execute":
        raise Refused(
            "cell-contract-not-executing",
            f"cell {key} is {cell['execution']!r} ({cell['state']}), not scheduled "
            "for hosted execution; running it would publish a result the plan "
            "already accounted for",
        )
    return cell


def resolve_environment(plan: dict[str, Any], row: dict[str, str]) -> dict[str, Any]:
    """The plan's own record for this row's environment, with the runner checked."""
    for entry in plan["environments"]:
        if entry["name"] == row["environment"]:
            if entry["runner"] != row["runner"]:
                raise Refused(
                    "cell-contract-mismatch",
                    f"the row dispatches {row['environment']} to "
                    f"{row['runner']!r}, but the plan resolved it against "
                    f"{entry['runner']!r}",
                )
            return entry
    raise Refused(
        "cell-contract-mismatch",
        f"the plan knows no environment {row['environment']!r}",
    )


def resolve_package(plan: dict[str, Any], row: dict[str, str]) -> dict[str, Any]:
    """The package record carrying this cell's package-wide execution inputs."""
    for entry in plan["packages"]:
        if entry["package"] == row["package"]:
            return entry
    raise Refused(
        "cell-contract-package",
        f"the plan carries a cell for {row['package']!r} but no package record; "
        "the cell's package-wide execution inputs are unresolvable",
    )


def build_record(plan: dict[str, Any], cell: dict[str, Any]) -> dict[str, Any] | None:
    """The build this cell consumes, when the plan bound one to it.

    ## Errors

    Raises [`Refused`] when the cell names a key no build record carries. A
    consumer restores no Cargo cache and has no compiler to fall back on, so a
    dangling reference must stop the job rather than silently compile in place.
    """
    key = cell.get("build")
    if not key:
        return None
    for record in plan["builds"]:
        if record["key"] == key and record["package"] == cell["package"]:
            return record
    raise Refused(
        "cell-contract-build",
        f"cell {cell['package']}/{cell['environment']}/{cell['gate']} consumes "
        f"build {key!r}, which the plan does not carry",
    )


def execution_target(
    plan: dict[str, Any],
    environment: dict[str, Any],
    build: dict[str, Any] | None,
) -> str:
    """The target triple this cell's test binaries were compiled for.

    An archive-only environment declares none — it hosts no compiler — so its
    answer is the producer's. That is the correct one either way: the producer
    is where `cfg` was evaluated, and comparing a listing against a different
    target's expectations is what `completion.py` refuses outright.
    """
    declared = (environment.get("build") or {}).get("target")
    if declared:
        return str(declared)
    if build is None:
        return ""
    for entry in plan["environments"]:
        if entry["name"] == build["producer"]:
            return str((entry.get("build") or {}).get("target") or "")
    return ""


def contract(plan: dict[str, Any], row: dict[str, str], head: str) -> dict[str, str]:
    """The execution contract one dispatch row resolves to.

    Values are strings because they cross a GitHub step-output boundary: a JSON
    document arrives as its serialized text and a flag as `true` or the empty
    string, which is what `if:` and `${{ ... && ... || '' }}` read.
    """
    if head and plan["head"] != head:
        raise Refused(
            "cell-contract-head",
            f"the plan tests {plan['head']} and this run checked out {head}; "
            "a producer must never test a revision its plan was not resolved for",
        )
    cell = resolve_cell(plan, row)
    environment = resolve_environment(plan, row)
    package = resolve_package(plan, row)
    build = build_record(plan, cell)
    seam = package.get("dependent_seam", {})
    companions = [entry["name"] for entry in cell.get("companions", [])]
    gate = row["gate"]

    resolved = {
        "package": row["package"],
        "area": cell["area"],
        "gate": gate,
        "environment": row["environment"],
        "runner": row["runner"],
        "recipe": GATE_RECIPES.get(gate, ""),
        "profile": cell.get("profile", ""),
        "target_kinds": json.dumps(cell["target_kinds"]),
        "test_args": package["test_args"],
        "check_args": package["check_args"],
        "dependents_check_args": seam.get("check_args", ""),
        "dependents": json.dumps(seam.get("dependents", [])),
        "dependents_native": json.dumps(seam.get("native", [])),
        "l1_include_slow": "1" if package["l1_include_slow"] else "",
        "runner_tools": json.dumps(package["runner_tools"]),
        "backends": json.dumps(package["l2_backends"]),
        "companion_suites": json.dumps(companions),
        # The dependency closure's prerequisites for THIS environment, already
        # narrowed. A WSL2 guest is Linux and consumes the `ubuntu-latest`
        # list, which is what `native_key` on the environment record says.
        #
        # A JSON array, not a space-joined string: a package name is an opaque
        # literal, and word-splitting one would corrupt anything carrying a
        # space or a glob character before `_ensure-native-libs` ever saw it.
        "native_packages": json.dumps(
            package["native"].get(environment.get("native_key", row["environment"]), [])
        ),
        "requires_node": "true" if cell.get("requires_node") else "",
        # `requires-toolchain` is a package policy; whether the environment can
        # honor it is the plan's capability table. A cell that needs a toolchain
        # on an environment without one is a governed gap, never a silent skip.
        "requires_toolchain": (
            "true"
            if package.get("requires_toolchain") and capability(environment, "cargo_toolchain")
            else ""
        ),
        # The triple this cell's tests were COMPILED for, which is the one
        # `cfg` was evaluated against and the one the expected listing reports.
        # An archive-only environment compiles nothing and declares none, so
        # its answer is its producer's — the guest runs the Linux binaries.
        "target": execution_target(plan, environment, build),
        "build_key": build["key"] if build else "",
        "build_producer": build["producer"] if build else "",
        "build_artifact": build["artifact"] if build else "",
        "consumes_archive": "true" if build else "",
        # Result identity, spelled once. `ci-rollup` keys a status by tier and
        # the lint cell's status has never carried an environment; deriving the
        # names here keeps that rule in one place instead of in five `name:`s.
        "status_artifact": (
            f"status-{row['package']}-lint"
            if gate == "lint"
            else f"status-{row['package']}-{gate}-{row['environment']}"
        ),
        "junit_artifact": f"junit-{row['package']}-{gate}-{row['environment']}",
        "completion_artifact": (
            f"completion-{row['package']}-{gate}-{row['environment']}"
        ),
        "cell": f"{row['package']}/{row['environment']}/{gate}",
    }
    return resolved


def write_outputs(resolved: dict[str, str], destination: Path | None) -> None:
    """Publish the contract as GitHub step outputs, or to stdout when unset.

    Multi-line values cannot occur here — every value is a single-line string or
    a compact JSON document — so the plain `key=value` form is used rather than
    a heredoc delimiter that would have to be proven unique.
    """
    lines = "".join(f"{key}={value}\n" for key, value in resolved.items())
    if destination is None:
        sys.stdout.write(lines)
        return
    with destination.open("a", encoding="utf-8") as handle:
        handle.write(lines)


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--row", required=True, help="one dispatch row, as JSON")
    parser.add_argument(
        "--head", default="", help="the revision this run checked out"
    )
    parser.add_argument(
        "--github-output",
        type=Path,
        default=None,
        help="where to append the contract; stdout when absent",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        plan = json.loads(args.plan.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"cell_contract: cannot read {args.plan}: {error}", file=sys.stderr)
        return 1
    try:
        row = parse_row(args.row)
        resolved = contract(plan, row, args.head)
    except Refused as refusal:
        print(f"cell_contract: {refusal}", file=sys.stderr)
        return 1
    write_outputs(resolved, args.github_output)
    print(f"resolved {resolved['cell']} from the plan")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
