#!/usr/bin/env python3
"""Contract tests for the persisted execution-constraint store (AC17).

The store guards a trigger boundary, so the fixtures here are about refusal:
what blocks a push, what stops blocking it, and what a reader is told. The
refusal fixtures name the directory explicitly; `LocationTests` covers the
default location (OQ2, ruled 2026-09-12) and the fresh session that finds it
with no variable set.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from datetime import date
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import constraints  # noqa: E402
import schema  # noqa: E402


TODAY = date(2026, 9, 11)
MODULE = Path(__file__).resolve().parent / "constraints.py"
#: Shared with the Rust reader's tests (`scripts/ci-push/tests/constraints.rs`);
#: both must reach every expected outcome.
PARITY = Path(__file__).resolve().parent / "fixtures" / "pre-push" / "constraints" / "parity.json"


class StoreFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        # A cleanup, not `tearDown`: cleanups run last-in-first-out, so one a
        # test registers later (restoring permissions) runs before this.
        self.addCleanup(self.temporary_directory.cleanup)
        self.store = Path(self.temporary_directory.name)

    def write(self, name: str, document: object) -> Path:
        path = self.store / f"{name}.json"
        path.write_text(
            document if isinstance(document, str) else json.dumps(document),
            encoding="utf-8",
        )
        return path

    def constraint(self, **overrides: object) -> dict:
        document = {
            "environment": "wsl2-ubuntu",
            "reason": "do not rerun WSL for this branch",
            "owner": "ken",
            "expiry": "2099-01-01",
        }
        document.update(overrides)
        return document

    def load(self, **overrides: object):
        return constraints.load(str(self.store), today=TODAY, **overrides)


class LoadTests(StoreFixture):
    def test_an_unexpired_constraint_is_active(self) -> None:
        self.write("current", self.constraint())
        active, expired, malformed = self.load()
        self.assertEqual(["wsl2-ubuntu"], [entry.environment for entry in active])
        self.assertEqual([], expired)
        self.assertEqual([], malformed)

    def test_a_past_expiry_is_reported_and_stops_binding(self) -> None:
        self.write("current", self.constraint(expiry="2020-01-01"))
        active, expired, malformed = self.load()
        self.assertEqual([], active)
        self.assertEqual(1, len(expired))
        self.assertEqual([], malformed)

    def test_the_expiry_boundary_is_inclusive_of_today(self) -> None:
        self.write("current", self.constraint(expiry=TODAY.isoformat()))
        active, expired, _ = self.load()
        self.assertEqual(1, len(active), "a constraint expiring today still binds today")
        self.assertEqual([], expired)

    def test_an_empty_or_absent_store_binds_nothing(self) -> None:
        self.assertEqual(([], [], []), self.load())
        self.assertEqual(([], [], []), constraints.load("", today=TODAY))
        self.assertEqual(
            ([], [], []), constraints.load(str(self.store / "absent"), today=TODAY)
        )

    def test_a_missing_field_makes_the_record_malformed_not_absent(self) -> None:
        for field in constraints.REQUIRED_FIELDS:
            with self.subTest(field=field):
                document = self.constraint()
                del document[field]
                self.write("current", document)
                _active, _expired, malformed = self.load()
                self.assertEqual(1, len(malformed))
                self.assertIn(field, malformed[0].describe())

    def test_unreadable_json_is_malformed_rather_than_ignored(self) -> None:
        self.write("current", "{not json")
        _active, _expired, malformed = self.load()
        self.assertEqual(1, len(malformed))

    def test_an_unknown_field_is_rejected_so_a_typo_cannot_widen_a_constraint(self) -> None:
        self.write("current", self.constraint(environments="wsl2-ubuntu"))
        _active, _expired, malformed = self.load()
        self.assertEqual(1, len(malformed))
        self.assertIn("environments", malformed[0].describe())

    def test_a_branch_scoped_constraint_binds_only_that_branch(self) -> None:
        self.write("current", self.constraint(branch="feat/unifi"))
        self.assertEqual(1, len(self.load(branches=["feat/unifi"])[0]))
        self.assertEqual([], self.load(branches=["main"])[0])
        # An unnamed branch means "this checkout is not claiming to be another",
        # so an unfiltered caller still sees it.
        self.assertEqual(1, len(self.load()[0]))

    def test_a_branch_scoped_constraint_binds_when_any_supplied_name_matches(self) -> None:
        # A renamed refspec (`git push origin feature:other`) is validated under
        # `other` but was restricted under `feature`; both names are supplied
        # and the record binds on either.
        self.write("current", self.constraint(branch="feature"))
        self.assertEqual(1, len(self.load(branches=["other", "feature"])[0]))
        self.assertEqual(1, len(self.load(branches=["feature", "other"])[0]))
        self.assertEqual([], self.load(branches=["other", "main"])[0])
        # An empty name is no identity and neither matches nor exempts.
        self.assertEqual(1, len(self.load(branches=[""])[0]))
        self.assertEqual([], self.load(branches=["", "main"])[0])

    def test_a_repository_scoped_constraint_matches_every_spelling_of_its_remote(self) -> None:
        self.write("current", self.constraint(repository="github.com/yankeeinlondon/rusty-biscuit"))
        for remote in (
            "git@github.com:yankeeinlondon/rusty-biscuit.git",
            "https://github.com/yankeeinlondon/rusty-biscuit.git",
            "ssh://git@github.com/yankeeinlondon/rusty-biscuit",
            "github.com/yankeeinlondon/rusty-biscuit",
        ):
            with self.subTest(remote=remote):
                self.assertEqual(1, len(self.load(repository=remote)[0]))
        self.assertEqual([], self.load(repository="git@github.com:someone-else/rusty-biscuit.git")[0])
        # A checkout with no origin cannot prove it is another repository.
        self.assertEqual(1, len(self.load(repository="")[0]))

    def test_a_record_may_spell_its_repository_as_a_full_url(self) -> None:
        self.write("current", self.constraint(repository="https://github.com/yankeeinlondon/rusty-biscuit.git"))
        self.assertEqual(1, len(self.load(repository="git@github.com:yankeeinlondon/rusty-biscuit.git")[0]))
        self.assertEqual([], self.load(repository="git@github.com:yankeeinlondon/other.git")[0])


class SatisfactionTests(StoreFixture):
    """A constraint is satisfied by a plan that schedules nothing it forbids."""

    def plan(self, execution: str, environment: str = "wsl2-ubuntu") -> dict:
        return {
            "cells": [
                {"environment": environment, "gate": "L1", "execution": execution},
                {"environment": "ubuntu-latest", "gate": "L1", "execution": "execute"},
            ]
        }

    def active(self, **overrides: object):
        self.write("current", self.constraint(**overrides))
        return self.load()[0]

    def test_a_scheduled_execution_leaves_the_constraint_unsatisfied(self) -> None:
        self.assertEqual(
            1, len(constraints.unsatisfied(self.active(), self.plan("execute")))
        )

    def test_a_reused_cell_satisfies_the_constraint(self) -> None:
        self.assertEqual([], constraints.unsatisfied(self.active(), self.plan("reuse")))

    def test_an_omitted_cell_satisfies_the_constraint(self) -> None:
        self.assertEqual([], constraints.unsatisfied(self.active(), self.plan("omit")))

    def test_no_plan_satisfies_nothing(self) -> None:
        self.assertEqual(1, len(constraints.unsatisfied(self.active(), None)))

    def test_a_cell_the_planner_already_marked_prohibited_is_unsatisfied(self) -> None:
        # `just ci-local --plan` with the store resolves an unsatisfied cell to
        # `omit`/`prohibited`; that plan must not read as satisfied here.
        plan = {
            "cells": [
                {
                    "environment": "wsl2-ubuntu",
                    "gate": "L1",
                    "execution": "omit",
                    "state": "prohibited",
                }
            ]
        }
        self.assertEqual(1, len(constraints.unsatisfied(self.active(), plan)))

    def test_a_gate_scoped_constraint_ignores_other_gates(self) -> None:
        active = self.active(gate="L2")
        self.assertEqual([], constraints.unsatisfied(active, self.plan("execute")))

    def test_another_environments_execution_does_not_violate_it(self) -> None:
        active = self.active()
        plan = {"cells": [{"environment": "macos-latest", "gate": "L1", "execution": "execute"}]}
        self.assertEqual([], constraints.unsatisfied(active, plan))

    def test_the_planner_input_names_every_forbidden_environment(self) -> None:
        self.write("current", self.constraint())
        self.write("other", self.constraint(environment="windows-latest"))
        active, _expired, _malformed = self.load()
        self.assertEqual(
            ["windows-latest", "wsl2-ubuntu"], constraints.prohibited_environments(active)
        )

    def test_every_forbidden_environment_is_a_known_one(self) -> None:
        self.write("current", self.constraint())
        active, _expired, _malformed = self.load()
        for environment in constraints.prohibited_environments(active):
            self.assertIn(environment, schema.ENVIRONMENTS)


class CommandTests(StoreFixture):
    """The CLI the hook and `just ci-local --plan` actually invoke."""

    def run_check(self, *extra: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(MODULE), "check", "--directory", str(self.store), *extra],
            capture_output=True,
            text=True,
            timeout=30,
        )

    def test_an_empty_store_exits_zero_and_says_nothing(self) -> None:
        result = self.run_check()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual("", result.stdout.strip())

    def test_an_active_constraint_blocks_and_names_itself(self) -> None:
        self.write("current", self.constraint())
        result = self.run_check()
        self.assertNotEqual(0, result.returncode)
        self.assertIn("wsl2-ubuntu", result.stderr)
        self.assertIn("do not rerun WSL for this branch", result.stderr)
        self.assertIn("ken", result.stderr)
        self.assertIn("2099-01-01", result.stderr)

    def test_an_expired_constraint_is_announced_and_does_not_block(self) -> None:
        self.write("current", self.constraint(expiry="2020-01-01"))
        result = self.run_check()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertIn("expired", result.stdout)

    def test_a_malformed_constraint_blocks_rather_than_lapsing(self) -> None:
        self.write("current", "{not json")
        result = self.run_check()
        self.assertNotEqual(
            0,
            result.returncode,
            "an instruction that cannot be read is not one that can be ignored",
        )

    def test_a_plan_that_schedules_nothing_forbidden_unblocks_the_push(self) -> None:
        self.write("current", self.constraint())
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {
                    "cells": [
                        {
                            "environment": "wsl2-ubuntu",
                            "gate": "L1",
                            "execution": "reuse",
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        result = self.run_check("--plan", str(plan))
        self.assertEqual(0, result.returncode, result.stderr)

    def test_a_plan_that_schedules_the_forbidden_cell_still_blocks(self) -> None:
        self.write("current", self.constraint())
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {
                    "cells": [
                        {
                            "environment": "wsl2-ubuntu",
                            "gate": "L1",
                            "execution": "execute",
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )
        result = self.run_check("--plan", str(plan))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("wsl2-ubuntu", result.stderr)

    def test_an_unreadable_plan_blocks_even_with_an_empty_store(self) -> None:
        plan = self.store.parent / "plan.json"
        plan.write_text("{not json", encoding="utf-8")
        result = self.run_check("--plan", str(plan))
        self.assertNotEqual(0, result.returncode, "an unreadable plan proves nothing")
        self.assertIn(f"the resolved plan at {plan} is not valid JSON", result.stderr)

    def test_a_plan_without_readable_cells_blocks(self) -> None:
        self.write("current", self.constraint())
        plan = self.store.parent / "plan.json"
        plan.write_text(json.dumps({"cells": "none"}), encoding="utf-8")
        result = self.run_check("--plan", str(plan))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("`cells` must be an array, found a string", result.stderr)

    def test_an_unreadable_store_blocks_and_names_the_path(self) -> None:
        record = self.write("current", self.constraint())
        if not _make_unreadable(self, record):
            self.skipTest("this host cannot make a file unreadable")
        result = self.run_check()
        self.assertNotEqual(0, result.returncode, "a hidden record might bind")
        self.assertIn(f"cannot read the constraint store at {record}", result.stderr)

    def test_a_missing_plan_file_blocks(self) -> None:
        result = self.run_check("--plan", str(self.store.parent / "absent.json"))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("cannot read the resolved plan", result.stderr)

    def test_the_hook_invocation_filters_by_repository_and_branch(self) -> None:
        self.write("other-branch", self.constraint(branch="main"))
        self.write("other-repo", self.constraint(repository="github.com/someone-else/rusty-biscuit"))
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {"cells": [{"environment": "wsl2-ubuntu", "gate": "L1", "execution": "execute"}]}
            ),
            encoding="utf-8",
        )
        result = self.run_check(
            "--plan",
            str(plan),
            "--repository",
            "git@github.com:yankeeinlondon/rusty-biscuit.git",
            "--branch",
            "feat/unifi",
        )
        self.assertEqual(0, result.returncode, result.stderr)
        result = self.run_check("--plan", str(plan), "--branch", "main")
        self.assertNotEqual(0, result.returncode)

    def test_a_repeated_branch_option_binds_a_record_under_any_of_the_names(self) -> None:
        self.write("renamed", self.constraint(branch="feature"))
        plan = self.store.parent / "plan.json"
        plan.write_text(
            json.dumps(
                {"cells": [{"environment": "wsl2-ubuntu", "gate": "L1", "execution": "execute"}]}
            ),
            encoding="utf-8",
        )
        # The matching name comes FIRST: a parser that kept only the last value
        # would exempt this push.
        result = self.run_check("--plan", str(plan), "--branch", "feature", "--branch", "other")
        self.assertNotEqual(0, result.returncode, "a record under the first name must still bind")
        self.assertIn("wsl2-ubuntu", result.stderr)
        result = self.run_check("--plan", str(plan), "--branch", "other", "--branch", "feature")
        self.assertNotEqual(0, result.returncode, "a record under the last name must still bind")
        result = self.run_check("--plan", str(plan), "--branch", "other", "--branch", "main")
        self.assertEqual(0, result.returncode, result.stderr)

    def test_environments_lists_the_planner_input(self) -> None:
        self.write("current", self.constraint())
        result = subprocess.run(
            [
                sys.executable,
                str(MODULE),
                "environments",
                "--directory",
                str(self.store),
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual("wsl2-ubuntu", result.stdout.strip())


def _make_unreadable(test: unittest.TestCase, path: Path) -> bool:
    """Removes every permission bit from `path` until the test ends.

    False where that cannot make it unreadable: Windows ignores the bits, and
    a privileged user reads through them.
    """
    if os.name == "nt":
        return False
    mode = path.stat().st_mode
    path.chmod(0)
    test.addCleanup(path.chmod, mode)
    try:
        if path.is_dir():
            os.listdir(path)
        else:
            path.read_bytes()
    except PermissionError:
        return True
    return False


class StrictReaderTests(StoreFixture):
    """The reader agrees with the Rust one: see `ParityTests` for the cases."""

    def test_a_missing_store_or_a_file_holds_no_records(self) -> None:
        file = self.write("store", self.constraint())
        self.assertEqual(([], [], []), constraints.load(str(file), today=TODAY))
        self.assertEqual(([], [], []), constraints.load(str(file / "below"), today=TODAY))

    def test_an_unreadable_record_raises_rather_than_being_skipped(self) -> None:
        record = self.write("current", self.constraint())
        if not _make_unreadable(self, record):
            self.skipTest("this host cannot make a file unreadable")
        with self.assertRaises(constraints.StoreError) as raised:
            self.load()
        self.assertEqual(record, raised.exception.path)

    def test_an_unreadable_directory_in_the_store_raises(self) -> None:
        nested = self.store / "github.com"
        nested.mkdir()
        (nested / "current.json").write_text(json.dumps(self.constraint()), encoding="utf-8")
        if not _make_unreadable(self, nested):
            self.skipTest("this host cannot make a directory unreadable")
        with self.assertRaises(constraints.StoreError) as raised:
            self.load()
        self.assertEqual(nested, raised.exception.path)

    def test_a_malformed_record_names_its_path_and_field(self) -> None:
        path = self.write("current", self.constraint(gate=None))
        _active, _expired, malformed = self.load()
        self.assertEqual("gate", malformed[0].location)
        self.assertEqual(
            f"{path}: `gate` must be a string when present, found null", malformed[0].describe()
        )

    def test_a_malformed_record_applies_everywhere_and_never_expires(self) -> None:
        self.write("current", self.constraint(branch=7, repository="github.com/other/repo"))
        entry = self.load(repository="github.com/mine/repo", branches=["main"])[2][0]
        self.assertTrue(entry.applies_to("github.com/mine/repo", ["main"]))
        self.assertFalse(entry.expired(date(2200, 1, 1)))
        plan = {"cells": [{"environment": "macos-latest", "gate": "L1", "execution": "reuse"}]}
        self.assertEqual([entry], constraints.unsatisfied([entry], plan))

    def test_a_malformed_record_still_names_a_textual_environment_to_the_planner(self) -> None:
        self.write("named", self.constraint(environments="typo"))
        self.write("unnamed", self.constraint(environment=7))
        malformed = self.load()[2]
        self.assertEqual(["wsl2-ubuntu", ""], [entry.environment for entry in malformed])


class ParityTests(unittest.TestCase):
    """Every case in the fixture the Rust reader also runs."""

    @classmethod
    def setUpClass(cls) -> None:
        cls.fixture = json.loads(PARITY.read_text(encoding="utf-8"))

    def test_every_identity_case_agrees(self) -> None:
        for case in self.fixture["identities"]:
            with self.subTest(remote=case["remote"]):
                identity = constraints.repository_identity(case["remote"])
                self.assertEqual(case["identity"], identity)
                self.assertEqual(case["segments"], constraints.store_segments(identity))

    def test_every_store_case_agrees(self) -> None:
        for case in self.fixture["cases"]:
            with self.subTest(case=case["name"]), tempfile.TemporaryDirectory() as temporary:
                self.check_case(case, Path(temporary))

    def check_case(self, case: dict, temporary: Path) -> None:
        store = temporary / "store"
        if case["store"] is not None:
            store.mkdir()
            for relative, content in case["store"].items():
                path = store.joinpath(*relative.split("/"))
                path.parent.mkdir(parents=True, exist_ok=True)
                text = content if isinstance(content, str) else json.dumps(content)
                path.write_bytes(text.encode("utf-8"))
        active, expired, malformed = constraints.load(
            str(store),
            case["repository"],
            case["branches"],
            date.fromisoformat(case["today"]),
        )

        def names(entries: list) -> list[str]:
            return [entry.path.relative_to(store).as_posix() for entry in entries]

        expected = case["expected"]
        self.assertEqual(expected["active"], names(active), "active")
        self.assertEqual(expected["expired"], names(expired), "expired")
        self.assertEqual(
            [(entry["path"], entry["location"]) for entry in expected["malformed"]],
            [(name, entry.location) for name, entry in zip(names(malformed), malformed)],
            "malformed",
        )
        plan = None
        if "plan" in case:
            path = temporary / "plan.json"
            path.write_bytes(case["plan"].encode("utf-8"))
            if "plan_error" in expected:
                with self.assertRaises(constraints.DocumentError) as raised:
                    constraints.read_plan(str(path))
                self.assertEqual(expected["plan_error"], raised.exception.location)
                return
            plan = constraints.read_plan(str(path))
        self.assertEqual(expected["unsatisfied"], names(constraints.unsatisfied(active, plan)))


class LocationTests(StoreFixture):
    """OQ2 (ruled 2026-09-12, Option B): `<home>/.rusty-biscuit/ci-constraints/<repository>/`.

    The home is relocated through the environment rather than by patching
    `Path.home`, because that is the seam a real session crosses: HOME on
    Unix, USERPROFILE on native Windows, so both are set.
    """

    REMOTE = "git@github.com:yankeeinlondon/rusty-biscuit.git"

    def setUp(self) -> None:
        super().setUp()
        self.home = self.store / "home"
        self.home.mkdir()
        self.environment = mock.patch.dict(
            os.environ,
            {"HOME": str(self.home), "USERPROFILE": str(self.home)},
            clear=False,
        )
        self.environment.start()
        self.addCleanup(self.environment.stop)
        os.environ.pop(constraints.ENVIRONMENT_VARIABLE, None)
        self.root = self.home / ".rusty-biscuit" / "ci-constraints"

    def test_the_default_is_the_repository_directory_beside_the_evidence(self) -> None:
        self.assertEqual(
            str(self.root / "github.com" / "yankeeinlondon" / "rusty-biscuit"),
            constraints.default_directory(self.REMOTE),
        )

    def test_every_spelling_of_one_remote_shares_one_directory(self) -> None:
        expected = constraints.default_directory(self.REMOTE)
        for remote in (
            "https://github.com/yankeeinlondon/rusty-biscuit.git",
            "ssh://git@github.com/yankeeinlondon/rusty-biscuit",
            "github.com/yankeeinlondon/rusty-biscuit",
        ):
            with self.subTest(remote=remote):
                self.assertEqual(expected, constraints.default_directory(remote))

    def test_an_unknown_repository_resolves_to_the_store_root(self) -> None:
        self.assertEqual(str(self.root), constraints.default_directory(""))

    def test_the_directory_never_escapes_the_root_and_is_portable(self) -> None:
        # A filesystem remote's identity is an absolute path; a port or a
        # Windows drive carries a colon. Neither may leave the root or name a
        # segment some OS refuses.
        for remote, segments in (
            ("/tmp/work/origin.git", ("tmp", "work", "origin")),
            ("C:\\Users\\ken\\origin.git", ("C", "Users", "ken", "origin")),
            ("ssh://git@github.com:2222/org/repo.git", ("github.com_2222", "org", "repo")),
        ):
            with self.subTest(remote=remote):
                self.assertEqual(
                    str(self.root.joinpath(*segments)), constraints.default_directory(remote)
                )

    def test_the_override_wins_over_the_default(self) -> None:
        with mock.patch.dict(os.environ, {constraints.ENVIRONMENT_VARIABLE: "/elsewhere"}):
            self.assertEqual("/elsewhere", constraints.resolve_directory(self.REMOTE))
            self.assertEqual("/explicit", constraints.resolve_directory(self.REMOTE, "/explicit"))
        self.assertEqual(
            constraints.default_directory(self.REMOTE), constraints.resolve_directory(self.REMOTE)
        )

    def write_under(self, relative: str, **overrides: object) -> None:
        path = self.root / relative / "current.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(self.constraint(**overrides)), encoding="utf-8")

    def test_the_root_reads_every_repository_and_a_repository_only_its_own(self) -> None:
        self.write_under("github.com/yankeeinlondon/rusty-biscuit", reason="ours")
        self.write_under("github.com/other/repo", reason="theirs")
        at_root = constraints.load(constraints.default_directory(""), today=TODAY)[0]
        self.assertEqual(["ours", "theirs"], sorted(entry.document["reason"] for entry in at_root))
        ours = constraints.load(constraints.default_directory(self.REMOTE), today=TODAY)[0]
        self.assertEqual(["ours"], [entry.document["reason"] for entry in ours])

    def test_a_fresh_session_without_the_variable_is_bound_by_a_recorded_constraint(self) -> None:
        # The real command, in a child that inherits no store variable and
        # whose home is the relocated one: the case that was still open.
        self.write_under("github.com/yankeeinlondon/rusty-biscuit", reason="recorded last session")
        environment = {
            key: value
            for key, value in os.environ.items()
            if key != constraints.ENVIRONMENT_VARIABLE
        }
        result = subprocess.run(
            [sys.executable, str(MODULE), "check", "--repository", self.REMOTE],
            capture_output=True,
            text=True,
            timeout=30,
            env=environment,
        )
        self.assertNotEqual(0, result.returncode, "the record was not found from a fresh session")
        self.assertIn("recorded last session", result.stderr)
        result = subprocess.run(
            [sys.executable, str(MODULE), "directory", "--repository", self.REMOTE],
            capture_output=True,
            text=True,
            timeout=30,
            env=environment,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(constraints.default_directory(self.REMOTE), result.stdout.strip())


if __name__ == "__main__":
    unittest.main(verbosity=2)
