#!/usr/bin/env python3
"""Construct the instrumentation-only pre-cutover revision AC8's baseline needs.

`fixes/2026-09-12-single-os-compile/spec.md` accepts the single-owner
architecture only against matched cold and warm measurements of the schedule
that preceded it. Review 2 observed that the instrumentation and the ownership
cutover arrived as one change, so dispatching the shipped workflow can only ever
measure the post-cutover schedule: the pre-cutover half of the comparison has no
revision to run on.

This module builds that revision. It replays the Phase 1 instrumentation — and
nothing else — onto the pre-cutover merge base, writes the result as an
unreferenced Git commit, and prints its id so
`fixes/2026-09-12-single-os-compile/baseline-2026-09-12.md` can name the exact
thing to dispatch.

## Examples

```sh
python3 scripts/ci/build_baseline_revision.py            # print the revision id
python3 scripts/ci/build_baseline_revision.py --explain  # and what it contains
python3 scripts/ci/build_baseline_revision.py --update-documents  # after the tool changes
```

## Notes

Three properties make the output usable as an acceptance baseline rather than as
an approximation:

- **The instrument is the same object.** The counter tool is copied byte for byte
  from the post-cutover tree, so `counter_setup_seconds` is the same cost on both
  sides of the comparison and cancels exactly rather than approximately.
- **The measured commands are byte-identical to the base revision's.** Timing is
  taken by sibling steps and the wrapper arrives through the gate step's `env:`
  mapping; no `run:` body is rewritten. [`verify_additive`] enforces this by
  refusing any construction that removes a line other than the single
  `workflow_dispatch: {}` placeholder that has to grow an input.
- **The cutover is provably absent.** The scheduling surface — workflows,
  `just/`, `scripts/ci/`, `.github/ci/` — is the base revision's plus additions,
  so no owner job, build record, or archive production can be hiding in it.

Nothing here touches the repository index, a branch, or the shared stash: the
tree is assembled through a temporary index file and the commit is written with
`git commit-tree`, which leaves it unreferenced until a human names it.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

#: The pre-cutover merge base. `baseline-2026-09-12.md` already records this as
#: the revision whose schedule the baseline describes; the constructed revision
#: is its child so `git log` shows exactly what was added to it.
BASE_REVISION = "8aa105e7c5a180b2d2a10ffbed2332b88671c996"

#: Fixed so two runs of this script produce the same commit id. A baseline the
#: document names by hash has to be reconstructible by hash.
COMMIT_IDENTITY = {
    "GIT_AUTHOR_NAME": "Ken Snyder",
    "GIT_AUTHOR_EMAIL": "ken@ken.net",
    "GIT_AUTHOR_DATE": "2026-09-15T00:00:00+00:00",
    "GIT_COMMITTER_NAME": "Ken Snyder",
    "GIT_COMMITTER_EMAIL": "ken@ken.net",
    "GIT_COMMITTER_DATE": "2026-09-15T00:00:00+00:00",
}

COMMIT_MESSAGE = """baseline: instrumentation-only pre-cutover revision for AC8

Constructed by scripts/ci/build_baseline_revision.py from the pre-cutover merge
base. It carries the compiler-work counter and the measurement publication of
plan Tasks 1.4 and 1.5 and none of the ownership cutover, so dispatching `ci.yml`
here with `measure-compiler-work: true` measures the schedule the single-owner
architecture is being compared against.

Unreferenced on purpose: it exists to be dispatched, not merged.
"""

#: Copied verbatim from the post-cutover tree. These files are the measurement
#: instrument itself; `scripts/Cargo.toml` and `scripts/Cargo.lock` come with
#: them because the `ci-build` bin and its `build-tools` feature are declared
#: there. `ci-build-archive.rs` is included because `ci-build.rs` `#[path]`-
#: includes it: dropping it would fork the instrument, and an unreachable
#: `produce` subcommand schedules nothing. `fixtures/compiler-work` is the
#: golden document `ci-build-tests.rs` compares its own serialization against;
#: carrying the test without it would leave the constructed tree unable to pass
#: its own suite.
CARRIER_PATHS = (
    "scripts/Cargo.lock",
    "scripts/Cargo.toml",
    "scripts/ci-build-archive-tests.rs",
    "scripts/ci-build-archive.rs",
    "scripts/ci-build-runtime.rs",
    "scripts/ci-build-tests.rs",
    "scripts/ci-build.rs",
    "scripts/fixtures/compiler-work/publisher-documents.json",
)

#: Lifted out of the post-cutover `just/devops.just` by name, with their doc
#: comments, and appended to the base file. Both read only a label and a counter
#: directory — no plan, no build record — which is why the cutover's recipes can
#: stay behind.
CARRIER_RECIPES = ("_ci_build_counter", "_ci_build_report")

#: `ci.yml` at the base revision exposes an argument-less dispatch. Growing it
#: into an input block is the one line the construction is allowed to remove.
DISPATCH_PLACEHOLDER = "  workflow_dispatch: {}"

MEASURE_INPUT_DESCRIPTION = (
    '"Count actual rustc invocations around each measured test command. '
    'Baseline collection only."'
)


class ConstructionError(RuntimeError):
    """An anchor the construction depends on is missing, duplicated, or moved."""


# --------------------------------------------------------------------------
# Git plumbing
# --------------------------------------------------------------------------


def _git(repo: Path, *args: str, stdin: Optional[bytes] = None, env: Optional[Dict[str, str]] = None) -> str:
    merged = dict(os.environ)
    if env:
        merged.update(env)
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        input=stdin,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=merged,
        check=False,
    )
    if result.returncode != 0:
        raise ConstructionError(
            "git {} failed: {}".format(" ".join(args), result.stderr.decode("utf-8", "replace").strip())
        )
    return result.stdout.decode("utf-8")


def read_blob(repo: Path, revision: str, path: str) -> str:
    """Text of `path` at `revision`."""
    return _git(repo, "show", "{}:{}".format(revision, path))


def base_paths(repo: Path, revision: str) -> List[str]:
    listing = _git(repo, "ls-tree", "-r", "--name-only", revision)
    return [line for line in listing.splitlines() if line]


# --------------------------------------------------------------------------
# Instrumentation blocks
# --------------------------------------------------------------------------


def _job_start_block() -> List[str]:
    return [
        "      # Phase 1 measurement only: the reference point for `setup_seconds`. It",
        "      # runs before checkout so setup — checkout, toolchain, native",
        "      # prerequisites, cache restore — falls inside the measured window.",
        "      - name: Mark the job start",
        "        id: job_start",
        "        if: ${{ inputs.measure-compiler-work }}",
        "        shell: bash",
        '        run: echo "epoch=$(date +%s)" >>"$GITHUB_OUTPUT"',
    ]


def _pre_gate_block(label: str) -> List[str]:
    return [
        "      - name: Prepare the compiler-work counter",
        "        id: counter",
        "        if: ${{ inputs.measure-compiler-work }}",
        "        shell: bash",
        '        run: just _ci_build_counter "{}"'.format(label),
        "      # The gate command below is byte-identical to the uninstrumented one.",
        "      # Its window is therefore bracketed by sibling steps rather than by a",
        "      # rewritten `run:` body, which would change what is being measured.",
        "      - name: Mark the gate start",
        "        id: gate_start",
        "        if: ${{ inputs.measure-compiler-work }}",
        "        shell: bash",
        '        run: echo "epoch=$(date +%s)" >>"$GITHUB_OUTPUT"',
    ]


def _gate_env_lines(label: str) -> List[str]:
    return [
        "          # COMMAND-SCOPED. The wrapper is bound to this step alone; every",
        "          # other build in the job inherits the workflow's empty",
        "          # `RUSTC_WRAPPER`, and an unmeasured run sets none of this.",
        "          RUSTC_WRAPPER: ${{ steps.counter.outputs.wrapper }}",
        "          BISCUIT_CI_BUILD_WRAP: ${{ steps.counter.outputs.wrap }}",
        "          BISCUIT_CI_BUILD_COUNTER_DIR: ${{ steps.counter.outputs.counter_dir }}",
        "          BISCUIT_CI_BUILD_PACKAGE: ${{ inputs.package }}",
        "          BISCUIT_CI_BUILD_CONFIGURATION: {}".format(label),
    ]


def _post_gate_block(gate: str, environment: str) -> List[str]:
    return [
        "      - name: Mark the gate end",
        "        id: gate_end",
        "        if: ${{ !cancelled() && inputs.measure-compiler-work }}",
        "        continue-on-error: true",
        "        shell: bash",
        '        run: echo "epoch=$(date +%s)" >>"$GITHUB_OUTPUT"',
        "      # ADVISORY, and structurally so: `continue-on-error` keeps a broken",
        "      # measurement out of `ci-gate`'s fold of this job's result. The cell's",
        "      # outcome is the gate command above and nothing else.",
        "      - name: Report compiler work",
        "        if: ${{ !cancelled() && inputs.measure-compiler-work }}",
        "        continue-on-error: true",
        "        shell: bash",
        "        env:",
        "          BISCUIT_CI_JOB_STARTED_EPOCH: ${{ steps.job_start.outputs.epoch }}",
        "          GATE_STARTED_EPOCH: ${{ steps.gate_start.outputs.epoch }}",
        "          GATE_ENDED_EPOCH: ${{ steps.gate_end.outputs.epoch }}",
        "          COUNTER_SETUP_SECONDS: ${{ steps.counter.outputs.setup_seconds }}",
        "        run: |",
        "          set -euo pipefail",
        "          # A skipped or cancelled neighbour leaves its output empty, and an",
        "          # empty stage is reported as zero rather than as a parse failure:",
        "          # this step may not turn a green gate red.",
        '          counter="${COUNTER_SETUP_SECONDS:-0}"',
        '          gate_start="${GATE_STARTED_EPOCH:-0}"',
        '          gate_end="${GATE_ENDED_EPOCH:-0}"',
        '          job_start="${BISCUIT_CI_JOB_STARTED_EPOCH:-0}"',
        '          stage="{\\"counter_setup_seconds\\":${counter:-0}"',
        '          stage="${stage},\\"gate_seconds\\":$(( ${gate_end:-0} - ${gate_start:-0} ))"',
        '          stage="${stage},\\"setup_seconds\\":$(( ${gate_start:-0} - ${job_start:-0} ))}"',
        '          export BISCUIT_CI_STAGE_SECONDS="$stage"',
        "          just _ci_build_report \\",
        '            "${{ steps.counter.outputs.wrapper }}" \\',
        '            "${{ steps.counter.outputs.counter_dir }}" \\',
        '            "$RUNNER_TEMP/measurement/measurement.json" \\',
        '            "${{{{ inputs.package }}}}" "{}" {}'.format(environment, gate),
        "      - name: Upload the compiler-work measurement",
        "        if: ${{ !cancelled() && inputs.measure-compiler-work }}",
        "        continue-on-error: true",
        "        uses: actions/upload-artifact@v6",
        "        with:",
        "          name: measurement-${{{{ inputs.package }}}}-{}-{}".format(gate, environment),
        "          path: ${{ runner.temp }}/measurement",
        "          if-no-files-found: ignore",
    ]


def _call_input_lines(indent: str, required: bool) -> List[str]:
    lines = [
        "{}measure-compiler-work:".format(indent),
        "{}  description: {}".format(indent, MEASURE_INPUT_DESCRIPTION),
        "{}  type: boolean".format(indent),
    ]
    if not required:
        lines.append("{}  required: false".format(indent))
    lines.append("{}  default: false".format(indent))
    return lines


#: One entry per measured job: which workflow it lives in, the step whose
#: outcome is the cell, and how the measurement document is keyed. The gate
#: label is what `ci-build` records as the configuration, so it has to match the
#: post-cutover spelling exactly or the two sides key differently.
MEASURED_JOBS: Tuple[Dict[str, str], ...] = (
    {
        "workflow": ".github/workflows/_package-ci.yml",
        "job": "check",
        "gate_step": "cargo check (declared example/bench kinds)",
        "gate": "check",
        "environment": "${{ matrix.os }}",
    },
    {
        "workflow": ".github/workflows/_package-ci.yml",
        "job": "test",
        "gate_step": "L1 tests",
        "gate": "L1",
        "environment": "${{ matrix.environment }}",
    },
    {
        "workflow": ".github/workflows/_package-ci.yml",
        "job": "lint",
        "gate_step": "Lint",
        "gate": "lint",
        "environment": "ubuntu-latest",
    },
    {
        "workflow": ".github/workflows/_package-ci.yml",
        "job": "test-l2",
        "gate_step": "L2 tests",
        "gate": "L2",
        "environment": "${{ matrix.environment }}",
    },
    {
        "workflow": ".github/workflows/_package-ci.yml",
        "job": "test-browser",
        "gate_step": "Browser tests",
        "gate": "browser",
        "environment": "${{ matrix.environment }}",
    },
    {
        # The one producer/consumer split that already existed. Its compile
        # belongs to the `wsl2-ubuntu` cell even though another machine performs
        # it, which is how `baseline-2026-09-12.md` reports it.
        "workflow": ".github/workflows/_wsl-ci.yml",
        "job": "archive",
        "gate_step": "Build the nextest archive",
        "gate": "L1",
        "environment": "wsl2-ubuntu",
    },
)


# --------------------------------------------------------------------------
# Anchored editing
# --------------------------------------------------------------------------


def _unique_index(lines: Sequence[str], needle: str, what: str) -> int:
    hits = [index for index, line in enumerate(lines) if line == needle]
    if len(hits) != 1:
        raise ConstructionError("{}: expected exactly one {!r}, found {}".format(what, needle, len(hits)))
    return hits[0]


def _job_span(lines: Sequence[str], job: str) -> Tuple[int, int]:
    start = _unique_index(lines, "  {}:".format(job), "job {!r}".format(job))
    for index in range(start + 1, len(lines)):
        line = lines[index]
        if line and not line.startswith("   ") and not line.lstrip().startswith("#"):
            return start, index
    return start, len(lines)


def _step_span(lines: Sequence[str], start: int, end: int, step_name: str) -> Tuple[int, int, int]:
    """`(comment_start, step_start, step_end)` of one `- name: <step_name>` step.

    `comment_start` walks back over the contiguous comment block above the step
    so an insertion lands before the prose that introduces it rather than
    between the prose and its subject.
    """
    anchor = "      - name: {}".format(step_name)
    hits = [index for index in range(start, end) if lines[index] == anchor]
    if len(hits) != 1:
        raise ConstructionError("step {!r}: found {} anchors, expected 1".format(step_name, len(hits)))
    step_start = hits[0]

    comment_start = step_start
    while comment_start > start and lines[comment_start - 1].startswith("      #"):
        comment_start -= 1

    step_end = end
    for index in range(step_start + 1, end):
        line = lines[index]
        if line.startswith("      - ") or line.startswith("      #"):
            step_end = index
            break
        if line and not line.startswith("       "):
            step_end = index
            break
    return comment_start, step_start, step_end


def _add_gate_env(lines: List[str], step_start: int, step_end: int, env_lines: Sequence[str]) -> List[str]:
    """Add the wrapper bindings to a step, reusing its `env:` map when it has one."""
    for index in range(step_start, step_end):
        if lines[index] == "        env:":
            return lines[: index + 1] + list(env_lines) + lines[index + 1 :]
    for index in range(step_start, step_end):
        if lines[index].startswith("        run:"):
            return lines[:index] + ["        env:"] + list(env_lines) + lines[index:]
    raise ConstructionError("gate step at line {} has neither `env:` nor `run:`".format(step_start + 1))


def instrument_job(text: str, job: str, gate_step: str, gate: str, environment: str) -> str:
    """Bracket one job's gate command with the counter, its clock, and its report."""
    label = "{} {}".format(gate, environment)
    lines = text.split("\n")

    job_start, job_end = _job_span(lines, job)
    comment_start, step_start, step_end = _step_span(lines, job_start, job_end, gate_step)

    lines = _add_gate_env(lines, step_start, step_end, _gate_env_lines(label))
    added = len(lines) - len(text.split("\n"))
    step_end += added

    lines = lines[:step_end] + _post_gate_block(gate, environment) + lines[step_end:]
    lines = lines[:comment_start] + _pre_gate_block(label) + lines[comment_start:]

    job_start, job_end = _job_span(lines, job)
    steps_index = _unique_index(
        lines[job_start:job_end], "    steps:", "job {!r} steps".format(job)
    ) + job_start
    lines = lines[: steps_index + 1] + _job_start_block() + lines[steps_index + 1 :]

    return "\n".join(lines)


def _insert_after_unique(text: str, anchor: str, block: Sequence[str], what: str) -> str:
    lines = text.split("\n")
    index = _unique_index(lines, anchor, what)
    return "\n".join(lines[: index + 1] + list(block) + lines[index + 1 :])


def instrument_ci_yml(text: str) -> str:
    """Expose the switch on dispatch and hand it to every area."""
    lines = text.split("\n")
    index = _unique_index(lines, DISPATCH_PLACEHOLDER, "ci.yml dispatch")
    replacement = [
        "  # `measure-compiler-work` is the Phase 1 baseline switch of",
        "  # `fixes/2026-09-12-single-os-compile/plan.md`. It is OFF for every ordinary",
        "  # run — pull request, push, and an unattended dispatch alike — because it",
        "  # builds one extra tool in each measured job and this repository tracks no",
        "  # rustc wrapper. On, it sets `RUSTC_WRAPPER` on the measured Cargo/Nextest",
        "  # command ALONE and publishes the compiler-work counts beside that cell's",
        "  # existing status. It changes no cell, no artifact name, and no gate.",
        "  workflow_dispatch:",
        "    inputs:",
        *_call_input_lines("      ", required=False),
    ]
    text = "\n".join(lines[:index] + replacement + lines[index + 1 :])
    return _insert_after_unique(
        text,
        "      accepted-gaps: ${{ contains(fromJSON(needs.scope.outputs.gap_areas), matrix.area) }}",
        [
            "      # `inputs` is null on every event but `workflow_dispatch`, and",
            "      # `null == true` is false — so this is `false` for a pull request and a",
            "      # push without needing an event-name guard.",
            "      measure-compiler-work: ${{ inputs.measure-compiler-work == true }}",
        ],
        "ci.yml area call",
    )


def add_call_input(text: str, after_input: str, occurrence: int = 0) -> str:
    """Declare the switch immediately after an existing input of the same block.

    `occurrence` selects which `<name>:` to follow when a workflow declares the
    same input under both `workflow_call` and `workflow_dispatch`; a reusable
    workflow that offers both has to declare it in both or `inputs.` resolves on
    only one of its triggers.
    """
    lines = text.split("\n")
    anchor = "      {}:".format(after_input)
    hits = [index for index, line in enumerate(lines) if line == anchor]
    if occurrence >= len(hits):
        raise ConstructionError(
            "input {!r}: wanted occurrence {}, found {}".format(after_input, occurrence, len(hits))
        )
    index = hits[occurrence] + 1
    while index < len(lines) and lines[index].startswith("        "):
        index += 1
    return "\n".join(lines[:index] + _call_input_lines("      ", required=False) + lines[index:])


def instrument_area_yml(text: str) -> str:
    text = add_call_input(text, "accepted-gaps")
    return _insert_after_unique(
        text,
        "      native: ${{ toJSON(matrix.native) }}",
        ["      measure-compiler-work: ${{ inputs.measure-compiler-work }}"],
        "_area-ci.yml package call",
    )


def instrument_package_yml(text: str) -> str:
    text = add_call_input(text, "dependents-native")
    return _insert_after_unique(
        text,
        "      native: ${{ inputs.native }}",
        ["      measure-compiler-work: ${{ inputs.measure-compiler-work }}"],
        "_package-ci.yml wsl call",
    )


def instrument_wsl_yml(text: str) -> str:
    text = add_call_input(text, "distribution", occurrence=1)
    return add_call_input(text, "distribution", occurrence=0)


#: `just` recipe bodies run until the next unindented, non-comment line.
def extract_recipes(devops_just: str, names: Sequence[str]) -> str:
    lines = devops_just.split("\n")
    chunks: List[str] = []
    for name in names:
        starts = [
            index
            for index, line in enumerate(lines)
            if line.startswith(name + " ") or line == name + ":"
        ]
        if len(starts) != 1:
            raise ConstructionError("recipe {!r}: found {} definitions, expected 1".format(name, len(starts)))
        start = starts[0]
        while start > 0 and lines[start - 1].startswith("#"):
            start -= 1
        end = starts[0] + 1
        while end < len(lines) and (not lines[end] or lines[end][0] in " \t#"):
            end += 1
        # A trailing blank-or-comment run introduces the NEXT recipe, not this
        # one; leaving it attached would copy a doc comment away from its
        # subject and, for two adjacent recipes, copy it twice.
        while end > starts[0] + 1 and (not lines[end - 1].strip() or lines[end - 1].startswith("#")):
            end -= 1
        chunks.append("\n".join(lines[start:end]))
    return "\n\n".join(chunks)


# --------------------------------------------------------------------------
# Construction
# --------------------------------------------------------------------------


def build_files(repo: Path, base: str, source_root: Path) -> Dict[str, str]:
    """Every path the constructed tree changes, keyed by repository path."""
    files: Dict[str, str] = {}

    for path in CARRIER_PATHS:
        carrier = source_root / path
        if not carrier.is_file():
            raise ConstructionError("carrier path is missing from the source tree: {}".format(path))
        files[path] = carrier.read_text(encoding="utf-8")

    base_devops = read_blob(repo, base, "just/devops.just")
    source_devops = (source_root / "just" / "devops.just").read_text(encoding="utf-8")
    recipes = extract_recipes(source_devops, CARRIER_RECIPES)
    files["just/devops.just"] = base_devops.rstrip("\n") + "\n\n" + recipes.rstrip("\n") + "\n"

    workflows = {
        ".github/workflows/ci.yml": instrument_ci_yml,
        ".github/workflows/_area-ci.yml": instrument_area_yml,
        ".github/workflows/_package-ci.yml": instrument_package_yml,
        ".github/workflows/_wsl-ci.yml": instrument_wsl_yml,
    }
    for path, threader in workflows.items():
        files[path] = threader(read_blob(repo, base, path))

    for job in MEASURED_JOBS:
        path = job["workflow"]
        files[path] = instrument_job(
            files[path], job["job"], job["gate_step"], job["gate"], job["environment"]
        )

    return files


def verify_additive(repo: Path, base: str, files: Dict[str, str]) -> List[str]:
    """Refuse a construction that removed anything but the dispatch placeholder.

    This is the cutover-absence proof. Every scheduling decision the cutover
    makes — an owner job, a build record, an archive a consumer must read — would
    have to displace a base line to take effect. A tree that only adds lines to
    the base workflows, `just/devops.just`, and nothing else in the scheduling
    surface cannot be scheduling differently.
    """
    removed: List[str] = []
    for path, content in sorted(files.items()):
        if path in CARRIER_PATHS:
            continue
        original = read_blob(repo, base, path).split("\n")
        updated = set(content.split("\n"))
        for line in original:
            if line and line not in updated:
                removed.append("{}: {}".format(path, line))
    unexpected = [entry for entry in removed if not entry.endswith(DISPATCH_PLACEHOLDER)]
    if unexpected:
        raise ConstructionError(
            "construction is not additive; it removed:\n  " + "\n  ".join(unexpected)
        )
    return removed


def verify_no_cutover(repo: Path, base: str, files: Dict[str, str]) -> None:
    """The scheduling surface gains no path the base revision did not have."""
    known = set(base_paths(repo, base))
    introduced = sorted(path for path in files if path not in known)
    scheduling = [
        path
        for path in introduced
        if path.startswith((".github/", "just/", "scripts/ci/"))
    ]
    if scheduling:
        raise ConstructionError(
            "construction adds scheduling-surface paths absent from the base: " + ", ".join(scheduling)
        )


def write_revision(repo: Path, base: str, files: Dict[str, str]) -> str:
    """Write the tree and an unreferenced commit; return the commit id."""
    with tempfile.TemporaryDirectory(prefix="baseline-revision-") as scratch:
        index = str(Path(scratch) / "index")
        env = {"GIT_INDEX_FILE": index}
        _git(repo, "read-tree", base, env=env)
        for path, content in sorted(files.items()):
            blob = _git(repo, "hash-object", "-w", "--stdin", stdin=content.encode("utf-8")).strip()
            _git(repo, "update-index", "--add", "--cacheinfo", "100644,{},{}".format(blob, path), env=env)
        tree = _git(repo, "write-tree", env=env).strip()

    commit = _git(
        repo,
        "-c",
        "commit.gpgsign=false",
        "commit-tree",
        tree,
        "-p",
        base,
        "-m",
        COMMIT_MESSAGE,
        env=COMMIT_IDENTITY,
    ).strip()
    return commit


def construct(repo: Path, base: str, source_root: Path) -> Dict[str, object]:
    files = build_files(repo, base, source_root)
    removed = verify_additive(repo, base, files)
    verify_no_cutover(repo, base, files)
    revision = write_revision(repo, base, files)
    return {"revision": revision, "base": base, "files": sorted(files), "removed": removed}


#: Documents that name the constructed revision by hash. A stale hash sends a
#: reader to dispatch a commit that does not exist, so `--update-documents`
#: rewrites all of them at once rather than leaving eight hand-edited copies.
DOCUMENTED_IN = (
    "fixes/2026-09-12-single-os-compile/baseline-2026-09-12.md",
    "fixes/2026-09-12-single-os-compile/deferred-performance-measurements.md",
    "fixes/2026-09-12-single-os-compile/plan.md",
)


def update_documents(repo: Path, revision: str) -> List[str]:
    """Point every documented hash at `revision`, leaving the base revision alone."""
    changed: List[str] = []
    for relative in DOCUMENTED_IN:
        document = repo / relative
        if not document.is_file():
            continue
        original = document.read_text(encoding="utf-8")
        updated = re.sub(
            r"\b[0-9a-f]{40}\b",
            lambda match: match.group(0) if match.group(0) == BASE_REVISION else revision,
            original,
        )
        if updated != original:
            document.write_text(updated, encoding="utf-8")
            changed.append(relative)
    return changed


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", default=".", help="Repository whose object database receives the commit.")
    parser.add_argument("--base", default=BASE_REVISION, help="Pre-cutover revision to build on.")
    parser.add_argument(
        "--source",
        default=None,
        help="Tree the instrumentation is copied from. Defaults to --repo.",
    )
    parser.add_argument("--explain", action="store_true", help="List what the constructed tree changes.")
    parser.add_argument(
        "--update-documents",
        action="store_true",
        help="Rewrite the revision every AC8 document names to the one just constructed.",
    )
    args = parser.parse_args(argv)

    repo = Path(args.repo).resolve()
    source = Path(args.source).resolve() if args.source else repo

    try:
        result = construct(repo, args.base, source)
    except ConstructionError as error:
        print("error: {}".format(error), file=sys.stderr)
        return 1

    print(result["revision"])
    if args.update_documents:
        for relative in update_documents(repo, str(result["revision"])):
            print("  updated: {}".format(relative), file=sys.stderr)
    if args.explain:
        print("  base:  {}".format(result["base"]), file=sys.stderr)
        for path in result["files"]:
            print("  file:  {}".format(path), file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
