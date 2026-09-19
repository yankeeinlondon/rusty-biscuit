#!/usr/bin/env python3
"""Contract tests for the one hashing boundary build keys are computed through.

The rule this suite exists to defend is negative: there is no second digest.
A planner that quietly fell back to `hashlib` when `ci-build` was missing would
still emit a plan, and every downstream comparison against a producer's
realized manifest would silently stop meaning anything.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import build_key  # noqa: E402
import schema  # noqa: E402


class HelperResolutionTests(unittest.TestCase):
    def setUp(self) -> None:
        build_key.reset_helper_cache()
        self.addCleanup(build_key.reset_helper_cache)
        self.previous = os.environ.get(build_key.HELPER_ENV)
        self.addCleanup(self.restore)

    def restore(self) -> None:
        if self.previous is None:
            os.environ.pop(build_key.HELPER_ENV, None)
        else:
            os.environ[build_key.HELPER_ENV] = self.previous

    def test_an_override_naming_no_file_fails_rather_than_falling_back(self) -> None:
        os.environ[build_key.HELPER_ENV] = str(Path(__file__).parent / "no-such-binary")
        with self.assertRaises(RuntimeError) as raised:
            build_key.planned_keys(["alpha"])
        message = str(raised.exception)
        self.assertIn(build_key.HELPER_ENV, message)
        self.assertNotIn("sha", message.lower())

    @unittest.skipIf(os.name == "nt", "the POSIX shim below is not a Windows executable")
    def test_a_helper_that_exits_non_zero_is_an_error_not_a_digest(self) -> None:
        # `false` is a real executable that answers nothing. A caller that
        # tolerated it would publish a plan with no keys in it.
        os.environ[build_key.HELPER_ENV] = "/usr/bin/false"
        with self.assertRaises(RuntimeError) as raised:
            build_key.planned_keys(["alpha"])
        self.assertIn("exited", str(raised.exception))

    @unittest.skipIf(os.name == "nt", "the POSIX shim below is not a Windows executable")
    def test_a_helper_answering_another_generation_is_refused(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        stub = Path(temporary.name)
        script = stub / "stub.py"
        script.write_text(
            "import json,sys\n"
            "sys.stdin.read()\n"
            'print(json.dumps({"schema_version": 99, "keys": ["0000000000000000"]}))\n',
            encoding="utf-8",
        )
        shim = stub / "ci-build"
        shim.write_text(f'#!/bin/sh\nexec "{sys.executable}" "{script}"\n', encoding="utf-8")
        shim.chmod(0o755)
        os.environ[build_key.HELPER_ENV] = str(shim)
        with self.assertRaises(RuntimeError) as raised:
            build_key.planned_keys(["alpha"])
        self.assertIn("schema version", str(raised.exception))

    def test_the_helper_is_resolved_beside_this_module_not_in_the_planned_root(self) -> None:
        # A synthetic fixture workspace has no `scripts/`; resolving the tool
        # there would make a plan's digest depend on where the plan lives.
        self.assertTrue(str(build_key.ROOT.joinpath("scripts", "Cargo.toml")))
        command = build_key.helper_command()
        self.assertTrue(command)
        self.assertTrue(
            command[0].endswith(("ci-build", "ci-build.exe")) or command[0] == "cargo",
            command,
        )


class DigestTests(unittest.TestCase):
    """The digest itself, pinned so a change to it cannot pass unnoticed."""

    #: XXH64 of the canonical bytes below, through `biscuit-hash`. Hard-coded
    #: rather than recomputed so a swapped hashing implementation fails here
    #: instead of silently re-keying every build in the repository.
    PINNED = {
        "abc": "44bc2cf5ad770999",
        '{"package":"claudine","target":"x86_64-unknown-linux-gnu"}': "344a9290ccd5c52a",
    }

    def test_the_digest_is_the_pinned_xxhash_of_the_canonical_bytes(self) -> None:
        material = sorted(self.PINNED)
        self.assertEqual(
            [self.PINNED[item] for item in material], build_key.planned_keys(material)
        )

    def test_a_key_is_sixteen_lowercase_hex_digits(self) -> None:
        key = build_key.planned_keys(["alpha"])[0]
        self.assertEqual(16, len(key))
        self.assertEqual(key, key.lower())
        int(key, 16)

    def test_an_empty_batch_needs_no_helper_at_all(self) -> None:
        self.assertEqual([], build_key.planned_keys([]))

    def test_field_order_does_not_change_a_key(self) -> None:
        # Canonicalization is this side's, so two spellings of one record are
        # one build rather than two.
        first = build_key.planned_key({"target": "x86_64", "package": "claudine"})
        second = build_key.planned_key({"package": "claudine", "target": "x86_64"})
        self.assertEqual(first, second)

    def test_one_differing_input_field_splits_the_key(self) -> None:
        base = build_key.planned_key({"package": "claudine", "features": ""})
        other = build_key.planned_key({"package": "claudine", "features": "--all-features"})
        self.assertNotEqual(base, other)

    def test_a_batch_answers_in_order(self) -> None:
        keys = build_key.planned_keys(["a", "b", "a"])
        self.assertEqual(3, len(keys))
        self.assertEqual(keys[0], keys[2])
        self.assertNotEqual(keys[0], keys[1])

    def test_the_cli_and_the_module_agree_on_one_input(self) -> None:
        # The module writes the request; this drives the binary directly, so a
        # change to either side of the wire format fails here.
        command = build_key.helper_command()
        request = json.dumps(
            {"schema_version": build_key.KEY_SCHEMA_VERSION, "material": ["alpha"]}
        )
        completed = subprocess.run(
            [*command, "key"],
            input=request,
            capture_output=True,
            text=True,
            cwd=build_key.ROOT,
            check=True,
            # Generous because `command` may be the `cargo run` fallback, which
            # compiles the `build-tools` closure before it answers: 14s on a
            # 16-core host, 26s with four jobs and a cold registry. Matching the
            # ceiling the other build-capable suites in this directory use.
            timeout=300,
        )
        answered = json.loads(completed.stdout.strip().splitlines()[-1])
        self.assertEqual(build_key.planned_keys(["alpha"]), answered["keys"])


class CanonicalizationTests(unittest.TestCase):
    def test_the_module_digests_the_schemas_canonical_form(self) -> None:
        identity = {"package": "claudine", "target_kinds": ["lib", "test"]}
        self.assertEqual(
            build_key.planned_keys([schema.canonical(identity)])[0],
            build_key.planned_key(identity),
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
