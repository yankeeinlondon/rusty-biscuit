#!/usr/bin/env python3
"""`just ci-seed`: pinning, reuse, per-leg outcomes, interruption, and resume.

The rigs and the planner are stubbed: a leg is a function that returns an exit
code and may publish proof, and proof is a set the verifier stub reads. The
real producer and verifier behind them are covered by `test_cross_check.py`
and `test_evidence_reuse.py`.
"""
from __future__ import annotations

import io
import json
import subprocess
import tempfile
import unittest
import unittest.mock
from pathlib import Path

import seed
from seed import Pass, SeedError


def plan_of(*cells: tuple[str, str, str, str]) -> dict:
    return {"cells": [
        {"package": package, "environment": environment, "gate": gate, "state": state}
        for package, environment, gate, state in cells
    ]}


PLAN = plan_of(
    ("alpha", "windows-latest", "L1", "pending"),
    ("alpha", "wsl2-ubuntu", "L1", "pending"),
    ("alpha", "windows-latest", "L2", "accepted-gap"),
    ("beta", "windows-latest", "L1", "pending"),
    ("beta", "wsl2-ubuntu", "L1", "pending"),
    ("gamma", "ubuntu-latest", "L1", "pending"),
)
HOSTS = {"windows": "build-win", "wsl": "build-wsl"}


class Rigs:
    """Stub legs and proof: `outcomes[(package, os)]` is the exit code, and a
    leg listed in `publishes` proves its cells."""

    def __init__(self, outcomes: dict | None = None, publishes: set | None = None,
                 proved: set | None = None) -> None:
        self.outcomes = outcomes or {}
        self.publishes = publishes if publishes is not None else {
            ("alpha", "windows"), ("alpha", "wsl"), ("beta", "windows"), ("beta", "wsl")}
        self.proved: set[tuple[str, str, str]] = set(proved or ())
        self.calls: list[tuple[str, str]] = []

    def verify(self, _plan: Path, _commit: str) -> set[tuple[str, str, str]]:
        return set(self.proved)

    def leg(self, package: str, os_name: str, log: Path) -> int:
        self.calls.append((package, os_name))
        log.write_text("leg output\n", encoding="utf-8")
        if (package, os_name) in self.publishes:
            self.proved.add((package, seed.SEED_ENVIRONMENTS[os_name], "L1"))
        return self.outcomes.get((package, os_name), 0)


class SeedRunTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)
        self.state_path = self.directory / "state.json"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def seed(self, rigs: Rigs, oses=("windows", "wsl"), packages=None) -> Pass:
        result = Pass(self.directory / "plan.json", PLAN, "c" * 40, list(oses), packages, rigs.verify)
        result.refresh()
        return result

    def run_seed(self, rigs: Rigs, hosts=HOSTS, **kwargs) -> Pass:
        result = self.seed(rigs, **kwargs)
        seed.run(result, hosts, self.state_path, {"schema_version": 1}, rigs.leg)
        return result

    @staticmethod
    def statuses(result: Pass) -> dict[str, str]:
        return {cell.key: cell.status for cell in result.cells}

    def test_only_the_scheduled_rig_environments_are_requested(self) -> None:
        keys = [cell.key for cell in self.seed(Rigs()).cells]
        self.assertNotIn("gamma/ubuntu-latest/L1", keys)
        self.assertEqual(5, len(keys))
        self.assertEqual(
            ["alpha/windows-latest/L1", "alpha/windows-latest/L2", "beta/windows-latest/L1"],
            [cell.key for cell in self.seed(Rigs(), oses=("windows",)).cells],
        )

    def test_a_full_pass_proves_every_cell_and_leaves_gaps_alone(self) -> None:
        rigs = Rigs()
        result = self.run_seed(rigs)
        self.assertEqual(
            [("alpha", "windows"), ("alpha", "wsl"), ("beta", "windows"), ("beta", "wsl")],
            rigs.calls,
        )
        self.assertEqual("gap", self.statuses(result)["alpha/windows-latest/L2"])
        self.assertEqual([], result.unproved())

    def test_already_proved_cells_are_reused_and_never_run(self) -> None:
        rigs = Rigs(proved={("alpha", "windows-latest", "L1"), ("beta", "wsl2-ubuntu", "L1")})
        result = self.run_seed(rigs)
        self.assertEqual([("alpha", "wsl"), ("beta", "windows")], rigs.calls)
        self.assertEqual("reused", self.statuses(result)["alpha/windows-latest/L1"])
        self.assertEqual("proved", self.statuses(result)["alpha/wsl2-ubuntu/L1"])

    def test_each_leg_outcome_is_reported_apart(self) -> None:
        rigs = Rigs(
            outcomes={("alpha", "windows"): 75, ("alpha", "wsl"): 255,
                      ("beta", "windows"): 1, ("beta", "wsl"): 0},
            publishes=set(),
        )
        statuses = self.statuses(self.run_seed(rigs))
        self.assertEqual("busy", statuses["alpha/windows-latest/L1"])
        self.assertEqual("unreachable", statuses["alpha/wsl2-ubuntu/L1"])
        self.assertEqual("failed", statuses["beta/windows-latest/L1"])
        # A passing run that published nothing proves nothing.
        self.assertEqual("unproved", statuses["beta/wsl2-ubuntu/L1"])

    def test_a_missing_rig_is_explicit_and_runs_nothing_there(self) -> None:
        rigs = Rigs()
        result = self.run_seed(rigs, hosts={"windows": "build-win", "wsl": None})
        self.assertEqual([("alpha", "windows"), ("beta", "windows")], rigs.calls)
        self.assertEqual("no-host", self.statuses(result)["alpha/wsl2-ubuntu/L1"])
        self.assertEqual(2, len(result.unproved()))

    def test_proof_published_before_an_interruption_survives_it_and_resume_runs_the_rest(self) -> None:
        rigs = Rigs()

        def interrupted(package: str, os_name: str, log: Path) -> int:
            if (package, os_name) == ("beta", "windows"):
                raise KeyboardInterrupt
            return rigs.leg(package, os_name, log)

        with self.assertRaises(KeyboardInterrupt):
            seed.run(self.seed(rigs), HOSTS, self.state_path, {"schema_version": 1}, interrupted)
        written = json.loads(self.state_path.read_text(encoding="utf-8"))
        self.assertEqual({"alpha/windows", "alpha/wsl"}, set(written["attempts"]))

        rigs.calls.clear()
        resumed = self.run_seed(rigs)
        self.assertEqual([("beta", "windows"), ("beta", "wsl")], rigs.calls)
        self.assertEqual("reused", self.statuses(resumed)["alpha/windows-latest/L1"])
        self.assertEqual([], resumed.unproved())

    def test_a_retry_after_a_failure_runs_only_the_failed_leg(self) -> None:
        rigs = Rigs(outcomes={("beta", "wsl"): 1}, publishes={
            ("alpha", "windows"), ("alpha", "wsl"), ("beta", "windows")})
        self.run_seed(rigs)
        rigs.calls.clear()
        rigs.outcomes.clear()
        rigs.publishes.add(("beta", "wsl"))
        self.assertEqual([], self.run_seed(rigs).unproved())
        self.assertEqual([("beta", "wsl")], rigs.calls)

    def test_attempts_accumulate_across_passes(self) -> None:
        rigs = Rigs(outcomes={("alpha", "windows"): 75}, publishes=set())
        state = {"schema_version": 1}
        seed.run(self.seed(rigs), HOSTS, self.state_path, state, rigs.leg)
        seed.run(self.seed(rigs), HOSTS, self.state_path, state, rigs.leg)
        attempts = json.loads(self.state_path.read_text(encoding="utf-8"))["attempts"]
        self.assertEqual([75, 75], [entry["exit_code"] for entry in attempts["alpha/windows"]])
        self.assertTrue(attempts["alpha/windows"][1]["log"].endswith("alpha-windows-2.log"))

    def test_a_package_filter_limits_the_pass(self) -> None:
        rigs = Rigs()
        self.run_seed(rigs, packages={"beta"})
        self.assertEqual([("beta", "windows"), ("beta", "wsl")], rigs.calls)

    def test_a_pending_higher_tier_is_reported_unsupported_and_never_run_as_level_1(self) -> None:
        # Level 1 proved, Level 2 pending: the transport runs Level 1 only, so
        # no leg may start for alpha, and the Level 2 cells must stay unproved.
        plan = plan_of(
            ("alpha", "windows-latest", "L1", "pending"),
            ("alpha", "windows-latest", "L2", "pending"),
            ("alpha", "wsl2-ubuntu", "L1", "pending"),
            ("alpha", "wsl2-ubuntu", "L2", "pending"),
            ("beta", "windows-latest", "L1", "pending"),
            ("beta", "wsl2-ubuntu", "L1", "pending"),
        )
        rigs = Rigs(proved={("alpha", "windows-latest", "L1"), ("alpha", "wsl2-ubuntu", "L1")})
        for attempt in ("first pass", "resume"):
            with self.subTest(attempt=attempt):
                rigs.calls.clear()
                result = Pass(self.directory / "plan.json", plan, "c" * 40, ["windows", "wsl"], None, rigs.verify)
                result.refresh()
                seed.run(result, HOSTS, self.state_path, {"schema_version": 1}, rigs.leg)
                self.assertNotIn(("alpha", "windows"), rigs.calls)
                self.assertNotIn(("alpha", "wsl"), rigs.calls)
                statuses = self.statuses(result)
                for environment in ("windows-latest", "wsl2-ubuntu"):
                    self.assertEqual("unsupported", statuses[f"alpha/{environment}/L2"])
                    self.assertEqual("reused", statuses[f"alpha/{environment}/L1"])
                self.assertEqual(
                    {"alpha/windows-latest/L2", "alpha/wsl2-ubuntu/L2"},
                    {cell.key for cell in result.unproved()},
                )
                out = io.StringIO()
                seed.render(result, HOSTS, out)
                self.assertIn("2 unsupported", out.getvalue())
                self.assertIn("Level 1", out.getvalue())

    def test_a_pending_higher_tier_beside_a_pending_level_1_does_not_inherit_its_outcome(self) -> None:
        plan = plan_of(
            ("alpha", "windows-latest", "L1", "pending"),
            ("alpha", "windows-latest", "L2", "pending"),
            ("alpha", "wsl2-ubuntu", "L1", "pending"),
            ("alpha", "wsl2-ubuntu", "L2", "pending"),
        )
        rigs = Rigs()
        result = Pass(self.directory / "plan.json", plan, "c" * 40, ["windows", "wsl"], None, rigs.verify)
        result.refresh()
        seed.run(result, HOSTS, self.state_path, {"schema_version": 1}, rigs.leg)
        self.assertEqual([("alpha", "windows"), ("alpha", "wsl")], rigs.calls)
        statuses = self.statuses(result)
        for environment in ("windows-latest", "wsl2-ubuntu"):
            self.assertEqual("proved", statuses[f"alpha/{environment}/L1"])
            self.assertEqual("unsupported", statuses[f"alpha/{environment}/L2"])

    def test_the_seed_exits_nonzero_while_a_higher_tier_stays_unproved(self) -> None:
        plan = plan_of(
            ("alpha", "windows-latest", "L1", "pending"),
            ("alpha", "windows-latest", "L2", "pending"),
            ("alpha", "wsl2-ubuntu", "L1", "pending"),
        )
        rigs = Rigs(proved={("alpha", "windows-latest", "L1"), ("alpha", "wsl2-ubuntu", "L1")})
        self.directory.joinpath("plan.json").write_text(json.dumps(plan), encoding="utf-8")
        with unittest.mock.patch.object(seed, "pin_revision", return_value=("c" * 40, "d" * 40)), \
                unittest.mock.patch.object(seed, "fetch_notes", return_value=[]), \
                unittest.mock.patch.object(seed, "resolve_plan", side_effect=lambda path: (
                    path.write_text(json.dumps(plan), encoding="utf-8"), plan)[1]), \
                unittest.mock.patch.object(seed, "verified_cells", rigs.verify), \
                unittest.mock.patch.object(seed, "run_cross_check", rigs.leg), \
                unittest.mock.patch.object(seed, "git", return_value="git@example.com:o/r.git"), \
                unittest.mock.patch.dict("os.environ", {"BUILD_WIN": "build-win", "BUILD_WSL": "build-wsl",
                                               "BISCUIT_CI_EVIDENCE_DIR": str(self.directory)}), \
                unittest.mock.patch("sys.stdout", new_callable=io.StringIO):
            self.assertEqual(1, seed.main([]))
            self.assertEqual(0, seed.main(["--plan"]))
        self.assertEqual([], rigs.calls)
        (state_path,) = self.directory.glob("seed/*/state.json")
        cells = json.loads(state_path.read_text(encoding="utf-8"))["cells"]
        self.assertEqual("unsupported", cells["alpha/windows-latest/L2"])

    def test_the_preview_lists_every_cell_with_its_host(self) -> None:
        out = io.StringIO()
        seed.render(self.seed(Rigs(proved={("alpha", "windows-latest", "L1")})),
                    {"windows": "build-win", "wsl": None}, out)
        text = out.getvalue()
        self.assertIn("reused", text)
        self.assertIn("no BUILD_WSL", text)
        self.assertIn("5 cell(s)", text)


class StateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.directory = Path(self.temporary.name)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    STATE = {
        "schema_version": 1,
        "seed_id": "cccccccccccc-0123456789ab",
        "repository": {"remote": "git@example.com:o/r.git"},
        "pinned": {"revision": "c" * 40, "tree": "d" * 40},
        "plan": {"sha256": "e" * 64, "cell_count": 5},
        "attempts": {},
        "cells": {"a": "proved"},
    }

    def test_state_round_trips_and_leaves_no_temporary_file(self) -> None:
        path = self.directory / "seed" / "state.json"
        seed.write_state(path, self.STATE)
        self.assertEqual({"a": "proved"}, seed.read_state(path, self.STATE["seed_id"])["cells"])
        self.assertEqual(["state.json"], [entry.name for entry in path.parent.iterdir()])

    def test_unreadable_state_is_ignored_not_trusted(self) -> None:
        path = self.directory / "state.json"
        good = json.dumps(self.STATE)
        for text in ("{broken", "[]", good + " trailing",
                     good.replace('"schema_version": 1', '"schema_version": 1, "schema_version": 1')):
            with self.subTest(text=text[:40]):
                path.write_text(text, encoding="utf-8")
                self.assertIsNone(seed.read_state(path))
        self.assertIsNone(seed.read_state(self.directory / "absent.json"))

    def test_state_matrix_every_field_edit_makes_it_ignored(self) -> None:
        # The control: the unedited state is read.
        path = self.directory / "state.json"
        seed.write_state(path, self.STATE)
        self.assertIsNotNone(seed.read_state(path, self.STATE["seed_id"]))
        edits = {"absent": ..., "null": None, "wrong type": ["x"]}
        for field in seed.STATE_FIELDS:
            for label, value in edits.items():
                with self.subTest(field=field, edit=label):
                    state = dict(self.STATE)
                    if value is ...:
                        del state[field]
                    else:
                        state[field] = value
                    seed.write_state(path, state)
                    self.assertIsNone(seed.read_state(path, self.STATE["seed_id"]))
        for field, value in (("schema_version", 2), ("schema_version", True), ("seed_id", ""),
                             ("attempts", [])):
            with self.subTest(field=field, value=value):
                seed.write_state(path, {**self.STATE, field: value})
                self.assertIsNone(seed.read_state(path, self.STATE["seed_id"]))

    def test_another_seeds_state_is_not_resumed(self) -> None:
        path = self.directory / "state.json"
        seed.write_state(path, self.STATE)
        self.assertIsNone(seed.read_state(path, "000000000000-ffffffffffff"))

    def test_a_second_pass_on_the_same_seed_is_refused(self) -> None:
        with seed.pass_lock(self.directory):
            with self.assertRaises(SeedError), seed.pass_lock(self.directory):
                pass
        with seed.pass_lock(self.directory):
            pass

    def test_the_seed_id_binds_the_revision_and_the_plan(self) -> None:
        plan = self.directory / "plan.json"
        plan.write_text("{}", encoding="utf-8")
        first = seed.seed_id("a" * 40, plan)
        plan.write_text('{"cells": []}', encoding="utf-8")
        self.assertNotEqual(first, seed.seed_id("a" * 40, plan))
        self.assertNotEqual(seed.seed_id("a" * 40, plan), seed.seed_id("b" * 40, plan))


class PinTests(unittest.TestCase):
    """The checkout must be the pinned revision, clean, and on `origin/main`."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        root = Path(self.temporary.name)
        self.origin = root / "origin.git"
        self.work = root / "work"
        git = lambda *args, cwd=root: subprocess.run(  # noqa: E731
            ["git", *args], cwd=cwd, check=True, capture_output=True)
        git("init", "--quiet", "--bare", "--initial-branch=main", str(self.origin))
        git("clone", "--quiet", str(self.origin), str(self.work))
        for key, value in (("user.name", "t"), ("user.email", "t@example.com"), ("commit.gpgsign", "false")):
            git("config", key, value, cwd=self.work)
        (self.work / "file.txt").write_text("one\n", encoding="utf-8")
        git("add", "file.txt", cwd=self.work)
        git("commit", "--quiet", "-m", "one", cwd=self.work)
        git("push", "--quiet", "origin", "HEAD:main", cwd=self.work)
        git("fetch", "--quiet", "origin", cwd=self.work)
        self.git = git

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_a_clean_main_checkout_is_pinned(self) -> None:
        commit, tree = seed.pin_revision("HEAD", cwd=self.work)
        self.assertEqual(40, len(commit))
        self.assertEqual(40, len(tree))

    def test_uncommitted_or_untracked_changes_are_refused(self) -> None:
        for change in ("file.txt", "new.txt"):
            with self.subTest(change=change):
                (self.work / change).write_text("changed\n", encoding="utf-8")
                with self.assertRaises(SeedError) as raised:
                    seed.pin_revision("HEAD", cwd=self.work)
                self.assertIn("uncommitted", str(raised.exception))
                self.git("checkout", "--quiet", "--", ".", cwd=self.work)
                (self.work / "new.txt").unlink(missing_ok=True)

    def test_a_revision_other_than_the_checkout_is_refused(self) -> None:
        first = seed.pin_revision("HEAD", cwd=self.work)[0]
        self.git("commit", "--quiet", "--allow-empty", "-m", "two", cwd=self.work)
        with self.assertRaises(SeedError) as raised:
            seed.pin_revision(first, cwd=self.work)
        self.assertIn("not the pinned", str(raised.exception))

    def test_a_revision_not_on_origin_main_is_refused_but_previewable(self) -> None:
        self.git("commit", "--quiet", "--allow-empty", "-m", "unpushed", cwd=self.work)
        with self.assertRaises(SeedError) as raised:
            seed.pin_revision("HEAD", cwd=self.work)
        self.assertIn("origin/main", str(raised.exception))
        self.assertEqual(40, len(seed.pin_revision("HEAD", cwd=self.work, preview=True)[0]))


if __name__ == "__main__":
    unittest.main()
