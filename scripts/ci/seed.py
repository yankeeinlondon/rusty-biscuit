#!/usr/bin/env python3
"""Seed the nightly's native Windows and WSL2 evidence on the test rigs.

`just ci-seed` proves the cells a full-workspace nightly would schedule on
`windows-latest` and `wsl2-ubuntu` at one pinned `main` revision, using the
rigs `BUILD_WIN` and `BUILD_WSL` name, so a bootstrap nightly does not have to
run the whole workspace on hosted runners.

The pass is resumable by construction rather than by bookkeeping:

1. One plan is resolved: `affected_scope.py --all --event schedule` at the
   pinned revision, which must be the clean checkout and reachable from
   `origin/main`.
2. The cells already proved are found by the same verifier the nightly runs
   (`local_evidence.verify_cells` with the all-zero bootstrap base), so a
   cell proved by any earlier pass, the pre-push hook, or another host is
   reused and never run again.
3. Each package still owing a cell runs `scripts/cross-check.sh <package>
   --os windows|wsl`. That script publishes one observation per finished leg
   the moment the leg ends, so an interruption loses only the leg in flight.
4. After every run the package's cells are verified again; the state file
   (`<evidence dir>/seed/<seed id>/state.json`) records each attempt. It is a
   progress report, never a source of proof: deleting it changes nothing a
   rerun decides.

Running the command again continues where it stopped. A pass pinned to an
older revision keeps its proof; the nightly reuses it wherever the cell's
inputs are unchanged and runs the rest.

The exit code is 0 only when every requested cell is proved or a governed
accepted gap. A busy or unreachable rig, a missing `BUILD_*` host, a failed
run, and a run that published nothing each leave their cells unproved and
are reported separately.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Iterator

sys.path.insert(0, str(Path(__file__).resolve().parent))
import local_evidence  # noqa: E402
import schema  # noqa: E402

#: `cross-check.sh --os` name → the plan environment its leg proves.
SEED_ENVIRONMENTS = {"windows": "windows-latest", "wsl": "wsl2-ubuntu"}
#: The variable that declares each rig, as `cross-check.sh` reads it.
HOST_VARIABLES = {"windows": "BUILD_WIN", "wsl": "BUILD_WSL"}
STATE_SCHEMA_VERSION = 1
#: `cross-check.sh` exits 75 while another run holds the rig's clone lock.
EXIT_BUSY = 75
#: ssh exits 255 when it cannot reach the host.
EXIT_UNREACHABLE = 255
#: This command's own exit when another seed already holds the pass.
EXIT_LOCKED = 75

ROOT = Path(__file__).resolve().parents[2]


class SeedError(RuntimeError):
    """A precondition the pass refuses to start without."""


def git(*args: str, cwd: Path = ROOT, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, timeout=120
    )
    if check and result.returncode != 0:
        raise SeedError(f"git {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout.strip()


def pin_revision(revision: str, cwd: Path = ROOT, preview: bool = False) -> tuple[str, str]:
    """The pinned `(commit, tree)`, refusing anything but a clean `main` checkout.

    `cross-check.sh` tests the local tree and ships a dirty one as a synthetic
    commit, so the checkout must BE the pinned revision, with nothing on top,
    for its observations to describe that commit. Requiring it on
    `origin/main` keeps seed proof on commits every later nightly descends
    from.
    """
    commit = git("rev-parse", "--verify", f"{revision}^{{commit}}", cwd=cwd)
    head = git("rev-parse", "HEAD", cwd=cwd)
    if commit != head:
        raise SeedError(
            f"the checkout is at {head[:12]}, not the pinned {commit[:12]}; "
            f"check it out first (`git switch --detach {commit[:12]}`)"
        )
    if git("status", "--porcelain", "--untracked-files=normal", cwd=cwd):
        raise SeedError("the checkout has uncommitted or untracked changes; a seed tests a committed revision only")
    on_main = subprocess.run(
        ["git", "merge-base", "--is-ancestor", commit, "origin/main"],
        cwd=cwd, capture_output=True, timeout=60,
    ).returncode == 0
    if not on_main and preview:
        print(
            f"warning: {commit[:12]} is not on origin/main; a seed run would refuse it",
            file=sys.stderr,
        )
    elif not on_main:
        raise SeedError(
            f"{commit[:12]} is not reachable from origin/main; seed a revision "
            "the nightly will descend from (fetch origin first if it is new)"
        )
    return commit, git("rev-parse", f"{commit}^{{tree}}", cwd=cwd)


def fetch_notes(cwd: Path = ROOT) -> list[str]:
    """Bring published observations in without ever overwriting local ones.

    A non-forced fetch refuses to replace a local ref that holds notes origin
    lacks (an unpublished observation), and that refusal is reported rather
    than resolved here: the local proof stays and origin's is not consulted.
    """
    warnings = []
    for environment in SEED_ENVIRONMENTS.values():
        ref = f"{local_evidence.NOTES_PREFIX}/{environment}"
        result = subprocess.run(
            ["git", "fetch", "--quiet", "origin", f"{ref}:{ref}"],
            cwd=cwd, capture_output=True, text=True, timeout=300,
        )
        if result.returncode != 0 and "couldn't find remote ref" not in result.stderr:
            warnings.append(f"{ref}: not updated from origin ({result.stderr.strip() or 'fetch failed'})")
    return warnings


def resolve_plan(path: Path, cwd: Path = ROOT) -> dict[str, Any]:
    subprocess.run(
        [sys.executable, str(ROOT / "scripts/ci/affected_scope.py"), "--all", "--event",
         "schedule", "--resolved-plan", "--plan-out", str(path)],
        cwd=cwd, check=True, stdout=subprocess.DEVNULL, timeout=600,
    )
    return schema.loads_strict(path.read_text(encoding="utf-8"))


@dataclass
class Cell:
    package: str
    environment: str
    gate: str
    state: str
    status: str = "pending"
    detail: str = ""

    @property
    def key(self) -> str:
        return f"{self.package}/{self.environment}/{self.gate}"


@dataclass
class Pass:
    """One seed pass over one plan; every decision is re-derived from proof."""

    plan_path: Path
    plan: dict[str, Any]
    commit: str
    oses: list[str]
    packages: set[str] | None = None
    verify: Callable[[Path, str], set[tuple[str, str, str]]] = field(default=None)  # type: ignore[assignment]
    cells: list[Cell] = field(default_factory=list)

    def __post_init__(self) -> None:
        if self.verify is None:
            self.verify = verified_cells
        environments = {SEED_ENVIRONMENTS[name] for name in self.oses}
        for cell in self.plan["cells"]:
            if cell["environment"] not in environments:
                continue
            if self.packages is not None and cell["package"] not in self.packages:
                continue
            self.cells.append(Cell(cell["package"], cell["environment"], cell["gate"], cell["state"]))
        self.cells.sort(key=lambda cell: (cell.package, cell.environment, cell.gate))

    def refresh(self) -> None:
        proved = self.verify(self.plan_path, self.commit)
        for cell in self.cells:
            if cell.state == "accepted-gap":
                cell.status, cell.detail = "gap", "governed accepted gap; nothing to prove"
            elif (cell.package, cell.environment, cell.gate) in proved:
                cell.status, cell.detail = "reused", "already proved; not run again"

    def owing(self) -> list[tuple[str, str]]:
        """`(package, os)` pairs that still have an unproved cell, in order."""
        by_environment = {value: key for key, value in SEED_ENVIRONMENTS.items()}
        pairs: list[tuple[str, str]] = []
        for cell in self.cells:
            if cell.status in ("pending", "failed", "unproved", "busy", "unreachable"):
                pair = (cell.package, by_environment[cell.environment])
                if pair not in pairs:
                    pairs.append(pair)
        return pairs

    def unproved(self) -> list[Cell]:
        return [cell for cell in self.cells if cell.status not in ("reused", "proved", "gap")]


def verified_cells(plan_path: Path, commit: str) -> set[tuple[str, str, str]]:
    accepted, _ = local_evidence.verify_cells(str(plan_path), local_evidence.ZERO_REVISION, commit)
    return {(cell["package"], cell["environment"], cell["gate"]) for cell in accepted}


def classify(exit_code: int, proved: bool) -> tuple[str, str]:
    if proved:
        return "proved", "proved by this pass"
    if exit_code == EXIT_BUSY:
        return "busy", "the rig's clone is locked by another run; rerun to retry"
    if exit_code == EXIT_UNREACHABLE:
        return "unreachable", "ssh could not reach the rig"
    if exit_code == 0:
        return "unproved", "the run passed but published no qualifying observation; see its log"
    return "failed", f"cross-check exited {exit_code}; a completed failure is published and blocks reuse"


def run_cross_check(package: str, os_name: str, log: Path) -> int:
    """One `cross-check.sh` leg, streamed to the terminal and kept in `log`."""
    with log.open("w", encoding="utf-8") as sink:
        process = subprocess.Popen(
            [str(ROOT / "scripts/cross-check.sh"), package, "--os", os_name],
            cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
        )
        assert process.stdout is not None
        for line in process.stdout:
            sys.stdout.write(line)
            sink.write(line)
        return process.wait()


def file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def seed_id(commit: str, plan_path: Path) -> str:
    return f"{commit[:12]}-{file_digest(plan_path)[:12]}"


def write_state(path: Path, state: dict[str, Any]) -> None:
    """Atomic: a reader sees the previous state or this one, never half of it."""
    path.parent.mkdir(parents=True, exist_ok=True)
    handle, temporary = tempfile.mkstemp(dir=path.parent, prefix=".state-", suffix=".json")
    with os.fdopen(handle, "w", encoding="utf-8") as stream:
        json.dump(state, stream, indent=2, sort_keys=True)
        stream.write("\n")
    os.replace(temporary, path)


#: The state fields a resumed pass appends to, and their types.
STATE_FIELDS: dict[str, type] = {
    "schema_version": int,
    "seed_id": str,
    "repository": dict,
    "pinned": dict,
    "plan": dict,
}


def read_state(path: Path, identity: str | None = None) -> dict[str, Any] | None:
    """The previous attempts, or None when absent, malformed, or another seed's.

    The state decides nothing, so a bad one is replaced rather than repaired:
    the pass re-derives every cell from published proof either way, and only
    the attempt history of the unreadable file is lost.
    """
    try:
        state = schema.loads_strict(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    if not isinstance(state, dict):
        return None
    for name, kind in STATE_FIELDS.items():
        value = state.get(name)
        if type(value) is not kind or (kind is str and not value):
            return None
    if state["schema_version"] != STATE_SCHEMA_VERSION:
        return None
    if identity is not None and state["seed_id"] != identity:
        return None
    if not isinstance(state.get("attempts", {}), dict):
        return None
    return state


@contextlib.contextmanager
def pass_lock(directory: Path) -> Iterator[None]:
    """One pass per seed at a time: a second would race the same rig clones."""
    directory.mkdir(parents=True, exist_ok=True)
    lock = directory / "lock"
    try:
        lock.mkdir()
    except FileExistsError:
        raise SeedError(f"another seed holds {lock}; remove it if no seed is running") from None
    try:
        yield
    finally:
        lock.rmdir()


def render(seed: Pass, hosts: dict[str, str | None], out: Any = sys.stdout) -> None:
    by_status: dict[str, int] = {}
    for cell in seed.cells:
        by_status[cell.status] = by_status.get(cell.status, 0) + 1
        os_name = next(key for key, value in SEED_ENVIRONMENTS.items() if value == cell.environment)
        host = hosts.get(os_name) or f"no {HOST_VARIABLES[os_name]}"
        print(f"  {cell.status:<11} {cell.key:<55} {host}  {cell.detail}".rstrip(), file=out)
    summary = ", ".join(f"{count} {status}" for status, count in sorted(by_status.items()))
    print(f"\n{len(seed.cells)} cell(s): {summary or 'none'}", file=out)


def run(
    seed: Pass,
    hosts: dict[str, str | None],
    state_path: Path,
    state: dict[str, Any],
    leg: Callable[[str, str, Path], int],
) -> None:
    attempts: dict[str, list[dict[str, Any]]] = state.setdefault("attempts", {})
    for package, os_name in seed.owing():
        cells = [
            cell for cell in seed.cells
            if cell.package == package and cell.environment == SEED_ENVIRONMENTS[os_name]
            and cell.status not in ("reused", "proved", "gap")
        ]
        if not hosts.get(os_name):
            for cell in cells:
                cell.status, cell.detail = "no-host", f"{HOST_VARIABLES[os_name]} is not set"
            continue
        log = state_path.parent / "logs" / f"{package}-{os_name}-{len(attempts.get(f'{package}/{os_name}', [])) + 1}.log"
        log.parent.mkdir(parents=True, exist_ok=True)
        print(f"\n== {package} on {SEED_ENVIRONMENTS[os_name]} ({hosts[os_name]})", flush=True)
        exit_code = leg(package, os_name, log)
        proved = seed.verify(seed.plan_path, seed.commit)
        for cell in cells:
            cell.status, cell.detail = classify(
                exit_code, (cell.package, cell.environment, cell.gate) in proved
            )
        attempts.setdefault(f"{package}/{os_name}", []).append(
            {"exit_code": exit_code, "log": str(log),
             "cells": {cell.key: cell.status for cell in cells}}
        )
        state["cells"] = {cell.key: cell.status for cell in seed.cells}
        write_state(state_path, state)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--os", choices=("windows", "wsl", "all"), default="all")
    parser.add_argument("--revision", default="HEAD", help="the pinned main revision (default: the checkout)")
    parser.add_argument("--plan", action="store_true", help="preview: list the cells and what would run; prove nothing")
    parser.add_argument("--package", action="append", help="limit the pass to this package (repeatable)")
    args = parser.parse_args(argv)

    oses = ["windows", "wsl"] if args.os == "all" else [args.os]
    hosts = {name: os.environ.get(HOST_VARIABLES[name]) or None for name in oses}
    evidence = Path(os.environ.get("BISCUIT_CI_EVIDENCE_DIR") or Path.home() / ".rusty-biscuit/ci-evidence")
    try:
        commit, tree = pin_revision(args.revision, preview=args.plan)
        for warning in fetch_notes():
            print(f"warning: {warning}", file=sys.stderr)
        with tempfile.TemporaryDirectory(prefix="ci-seed-") as scratch:
            plan_path = Path(scratch) / "plan.json"
            plan = resolve_plan(plan_path)
            identity = seed_id(commit, plan_path)
            directory = evidence / "seed" / identity
            seed = Pass(plan_path, plan, commit, oses, set(args.package) if args.package else None)
            if args.package and not seed.cells:
                raise SeedError(f"no {'/'.join(oses)} cell is planned for {', '.join(args.package)}")
            seed.refresh()
            print(f"seed {identity}: {commit[:12]} (tree {tree[:12]}), {', '.join(SEED_ENVIRONMENTS[o] for o in oses)}")
            if args.plan:
                render(seed, hosts)
                return 0
            with pass_lock(directory):
                state_path = directory / "state.json"
                state = read_state(state_path, identity) or {
                    "schema_version": STATE_SCHEMA_VERSION,
                    "seed_id": identity,
                    "repository": {"remote": git("remote", "get-url", "origin")},
                    "pinned": {"revision": commit, "tree": tree},
                    "plan": {"sha256": file_digest(plan_path), "cell_count": len(seed.cells)},
                }
                run(seed, hosts, state_path, state, run_cross_check)
                state["cells"] = {cell.key: cell.status for cell in seed.cells}
                write_state(state_path, state)
            print(f"\nseed {identity}: state in {state_path}")
            render(seed, hosts)
            return 1 if seed.unproved() else 0
    except SeedError as error:
        print(f"ci-seed: {error}", file=sys.stderr)
        return EXIT_LOCKED if "another seed holds" in str(error) else 2


if __name__ == "__main__":
    sys.exit(main())
