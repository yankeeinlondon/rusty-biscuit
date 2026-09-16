#!/usr/bin/env python3
"""Contract tests for the AC8 baseline revision constructor.

The constructed revision is the pre-cutover half of a comparison that decides
whether the single-owner architecture is accepted, so two properties have to
hold mechanically rather than by inspection: it carries the Phase 1
instrumentation, and it carries none of the ownership cutover. Each test builds
a real temporary repository — seeded from the actual pre-cutover revision, so a
moved anchor is caught rather than mocked away — and runs the construction
against it. Objects are written to the temporary repository only.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Dict, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))

import build_baseline_revision as constructor  # noqa: E402
from workflow_reading import step_run_lines  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]

#: The files the constructor reads out of the base revision. A temporary
#: repository seeded with exactly these exercises every anchor.
SEEDED_PATHS = (
    ".github/workflows/ci.yml",
    ".github/workflows/_area-ci.yml",
    ".github/workflows/_package-ci.yml",
    ".github/workflows/_wsl-ci.yml",
    "just/devops.just",
)

#: Spellings that only exist because a cell stopped compiling for itself. None
#: may appear in the constructed scheduling surface. `--archive-file` and
#: `--workspace-remap` are deliberately absent from this list: the WSL2 guest
#: already ran from an archive before the cutover, so their presence at the base
#: revision is the schedule being measured rather than a leak.
CUTOVER_MARKERS = (
    "_ci_build_consumer",
    "_ci_build_verify",
    "produce-owner.sh",
    "ci-build produce",
    "ci-build verify",
    "inputs.builds",
    "build-source-mismatch",
    "sidecars.json",
)

GIT_ENV = {
    "GIT_AUTHOR_NAME": "Baseline Fixture",
    "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
    "GIT_COMMITTER_NAME": "Baseline Fixture",
    "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
}


def _base_revision_is_available() -> bool:
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "cat-file", "-e", constructor.BASE_REVISION + "^{commit}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


BASE_AVAILABLE = _base_revision_is_available()


def _git(repo: Path, *args: str) -> str:
    env = dict(os.environ)
    env.update(GIT_ENV)
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        check=True,
    )
    return result.stdout.decode("utf-8")


def _seed(scratch: Path, edits: Dict[str, Sequence[Sequence[str]]] = None) -> Dict[str, object]:
    """A repository whose single commit is the pre-cutover revision's anchors.

    `edits` maps a seeded path to `(old, new)` replacement pairs, so a test can
    move one anchor and assert the constructor refuses instead of guessing.
    """
    repo = scratch / "repo"
    repo.mkdir(parents=True)
    _git(repo, "init", "--quiet", "--initial-branch=main")
    _git(repo, "config", "commit.gpgsign", "false")

    for path in SEEDED_PATHS:
        content = constructor.read_blob(REPO_ROOT, constructor.BASE_REVISION, path)
        for old, new in (edits or {}).get(path, ()):
            if old not in content:
                raise AssertionError("edit anchor {!r} is absent from {}".format(old, path))
            content = content.replace(old, new, 1)
        destination = repo / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(content, encoding="utf-8")

    _git(repo, "add", "-A")
    _git(repo, "commit", "--quiet", "-m", "pre-cutover anchors")
    return {"repo": repo, "base": _git(repo, "rev-parse", "HEAD").strip()}


def _tree(repo: Path, revision: str, paths: Sequence[str]) -> Dict[str, str]:
    return {path: constructor.read_blob(repo, revision, path) for path in paths}


@unittest.skipUnless(BASE_AVAILABLE, "the pre-cutover revision is not in this checkout's history")
class ConstructedRevision(unittest.TestCase):
    def test_the_constructed_tree_carries_the_instrumentation(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(Path(scratch))
            result = constructor.construct(seed["repo"], seed["base"], REPO_ROOT)
            built = _tree(seed["repo"], result["revision"], list(result["files"]))

            for path in (
                ".github/workflows/ci.yml",
                ".github/workflows/_area-ci.yml",
                ".github/workflows/_package-ci.yml",
                ".github/workflows/_wsl-ci.yml",
            ):
                self.assertIn("measure-compiler-work:", built[path], path)

            for job in constructor.MEASURED_JOBS:
                workflow = built[job["workflow"]]
                label = "{} {}".format(job["gate"], job["environment"])
                self.assertIn('run: just _ci_build_counter "{}"'.format(label), workflow)
                self.assertIn("BISCUIT_CI_BUILD_CONFIGURATION: {}".format(label), workflow)
                self.assertIn(
                    "name: measurement-${{ inputs.package }}-"
                    + job["gate"]
                    + "-"
                    + job["environment"],
                    workflow,
                )

            self.assertIn("_ci_build_counter label:", built["just/devops.just"])
            self.assertIn("_ci_build_report wrapper", built["just/devops.just"])
            self.assertIn("EVENT_SCHEMA_VERSION", built["scripts/ci-build.rs"])

    def test_the_constructed_tree_carries_none_of_the_cutover(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(Path(scratch))
            result = constructor.construct(seed["repo"], seed["base"], REPO_ROOT)
            scheduling = [
                path
                for path in result["files"]
                if path.startswith((".github/", "just/", "scripts/ci/"))
            ]
            built = _tree(seed["repo"], result["revision"], scheduling)

            for path, content in built.items():
                for marker in CUTOVER_MARKERS:
                    self.assertNotIn(marker, content, "{} carries cutover marker {!r}".format(path, marker))

            # The stronger statement, and the one that makes the list above a
            # convenience rather than the proof: the scheduling surface is the
            # base revision plus added lines, so it cannot be scheduling
            # differently no matter what a marker list forgot.
            self.assertEqual(
                result["removed"],
                [".github/workflows/ci.yml: {}".format(constructor.DISPATCH_PLACEHOLDER)],
            )

    def test_the_measured_commands_are_byte_identical_to_the_base(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(Path(scratch))
            result = constructor.construct(seed["repo"], seed["base"], REPO_ROOT)

            for job in constructor.MEASURED_JOBS:
                path = job["workflow"]
                base_lines = constructor.read_blob(seed["repo"], seed["base"], path).split("\n")
                built_lines = constructor.read_blob(seed["repo"], result["revision"], path).split("\n")
                base_run = step_run_lines(base_lines, job["gate_step"])
                built_run = step_run_lines(built_lines, job["gate_step"])
                self.assertEqual(base_run, built_run, "{} / {}".format(path, job["gate_step"]))

    def test_the_construction_is_reproducible(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(Path(scratch))
            first = constructor.construct(seed["repo"], seed["base"], REPO_ROOT)
            second = constructor.construct(seed["repo"], seed["base"], REPO_ROOT)
            self.assertEqual(first["revision"], second["revision"])
            self.assertRegex(first["revision"], r"^[0-9a-f]{40}$")

    def test_a_moved_anchor_refuses_rather_than_guessing(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(
                Path(scratch),
                edits={
                    ".github/workflows/_package-ci.yml": (
                        ("      - name: L1 tests\n", "      - name: L1 suite\n"),
                    )
                },
            )
            with self.assertRaises(constructor.ConstructionError) as raised:
                constructor.construct(seed["repo"], seed["base"], REPO_ROOT)
            self.assertIn("L1 tests", str(raised.exception))

    def test_a_non_additive_construction_is_refused(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(Path(scratch))
            files = constructor.build_files(seed["repo"], seed["base"], REPO_ROOT)
            devops = files["just/devops.just"].split("\n")
            files["just/devops.just"] = "\n".join(line for line in devops if "_storage_preflight:" not in line)
            with self.assertRaises(constructor.ConstructionError) as raised:
                constructor.verify_additive(seed["repo"], seed["base"], files)
            self.assertIn("not additive", str(raised.exception))

    def test_the_commit_is_unreferenced_and_parented_on_the_base(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-test-") as scratch:
            seed = _seed(Path(scratch))
            before = _git(seed["repo"], "rev-parse", "HEAD").strip()
            result = constructor.construct(seed["repo"], seed["base"], REPO_ROOT)
            self.assertEqual(before, _git(seed["repo"], "rev-parse", "HEAD").strip())
            self.assertEqual("", _git(seed["repo"], "status", "--porcelain").strip())
            self.assertEqual(
                seed["base"],
                _git(seed["repo"], "rev-parse", "{}^".format(result["revision"])).strip(),
            )
            branches = _git(seed["repo"], "branch", "--contains", result["revision"], "--format=%(refname)")
            self.assertEqual("", branches.strip())


class RecipeExtraction(unittest.TestCase):
    """The counter recipes travel with their prose and without their neighbours'."""

    def test_a_recipe_stops_before_the_next_recipes_comment_block(self) -> None:
        source = "\n".join(
            [
                "# counter prose",
                "_ci_build_counter label:",
                "    echo {{ label }}",
                "",
                "# report prose",
                "_ci_build_report wrapper out:",
                "    echo {{ out }}",
                "",
                "# unrelated prose",
                "_native_path path:",
                "    echo {{ path }}",
                "",
            ]
        )
        extracted = constructor.extract_recipes(source, constructor.CARRIER_RECIPES)
        self.assertIn("# counter prose", extracted)
        self.assertIn("# report prose", extracted)
        self.assertNotIn("_native_path", extracted)
        self.assertNotIn("# unrelated prose", extracted)
        self.assertEqual(1, extracted.count("# report prose"))

    def test_an_absent_recipe_refuses(self) -> None:
        with self.assertRaises(constructor.ConstructionError):
            constructor.extract_recipes("_native_path path:\n    echo hi\n", ("_ci_build_counter",))


class DocumentRefresh(unittest.TestCase):
    """`--update-documents` moves every documented hash but the base revision."""

    def test_only_the_constructed_revision_is_rewritten(self) -> None:
        with tempfile.TemporaryDirectory(prefix="baseline-docs-") as scratch:
            root = Path(scratch)
            document = root / constructor.DOCUMENTED_IN[0]
            document.parent.mkdir(parents=True)
            stale = "0" * 40
            fresh = "a1b2c3d4" * 5
            document.write_text(
                "base `{}` measured `{}`\n".format(constructor.BASE_REVISION, stale),
                encoding="utf-8",
            )

            changed = constructor.update_documents(root, fresh)

            self.assertEqual([constructor.DOCUMENTED_IN[0]], changed)
            refreshed = document.read_text(encoding="utf-8")
            self.assertIn(constructor.BASE_REVISION, refreshed)
            self.assertIn(fresh, refreshed)
            self.assertNotIn(stale, refreshed)


@unittest.skipUnless(BASE_AVAILABLE, "the pre-cutover revision is not in this checkout's history")
class PublishedRevision(unittest.TestCase):
    """The id `baseline-2026-09-12.md` names must be the one this code produces.

    A red result here is drift, not a puzzle: the counter tool changed and the
    documents still send a reader to a commit the script no longer produces.
    `python3 scripts/ci/build_baseline_revision.py --update-documents` fixes it.
    """

    def test_the_documented_revision_matches_the_construction(self) -> None:
        document = (
            REPO_ROOT / "fixes" / "2026-09-12-single-os-compile" / "baseline-2026-09-12.md"
        ).read_text(encoding="utf-8")
        documented = re.findall(r"`([0-9a-f]{40})`", document)
        self.assertTrue(documented, "the baseline document names no revision")
        with tempfile.TemporaryDirectory(prefix="baseline-published-") as scratch:
            # `--shared` borrows this repository's objects through `alternates`
            # instead of copying them, so the real pre-cutover commit is the
            # constructed revision's parent — the id is the documented one —
            # while every object this test writes lands in the temporary clone.
            clone = Path(scratch) / "clone"
            _git(REPO_ROOT, "clone", "--quiet", "--local", "--shared", "--no-checkout", str(REPO_ROOT), str(clone))
            built = constructor.construct(clone, constructor.BASE_REVISION, REPO_ROOT)
        self.assertIn(built["revision"], documented)
        self.assertIn(constructor.BASE_REVISION, documented)


if __name__ == "__main__":
    unittest.main()
