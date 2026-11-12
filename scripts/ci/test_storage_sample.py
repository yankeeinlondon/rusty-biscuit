#!/usr/bin/env python3
"""Contract tests for the build owner's storage diagnostic.

The sampler's job is to leave an honest record behind a job that may die of a
full disk: every category measured apart, every measurement it could not take
marked unavailable rather than zero, and no failure of its own allowed to stop
the line being written. These fixtures use a few tiny files; nothing fills a
disk.
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import storage_sample  # noqa: E402


def write(path: Path, size: int) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"x" * size)
    return path


def profile(path: Path) -> Path:
    """A directory Cargo would recognize as a profile output directory."""
    (path / ".fingerprint").mkdir(parents=True, exist_ok=True)
    write(path / "deps" / "libdep.rlib", 4096)
    return path


class Fixture(unittest.TestCase):
    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="storage-sample-"))
        self.addCleanup(self._cleanup)
        self.target = self.root / "target"
        self.output = self.root / "build"
        self.out = self.root / "diagnostics" / "storage.jsonl"

    def _cleanup(self) -> None:
        import shutil

        shutil.rmtree(self.root, ignore_errors=True)

    def run_main(self, *extra: str) -> int:
        return storage_sample.main(
            [
                "--out", str(self.out),
                "--target", str(self.target),
                "--output", str(self.output),
                "--temp", str(self.root),
                *extra,
            ]
        )

    def lines(self) -> list[dict]:
        return [json.loads(line) for line in self.out.read_text(encoding="utf-8").splitlines()]


class AllocatedBytesTests(Fixture):
    def test_tiny_files_are_counted_by_allocation_not_by_length(self) -> None:
        write(self.target / "a.bin", 10)
        write(self.target / "nested" / "b.bin", 10)
        measured = storage_sample.allocated(self.target)
        self.assertTrue(measured["exists"])
        self.assertEqual(measured["files"], 2)
        self.assertEqual(measured["errors"], 0)
        if hasattr(os.stat_result, "st_blocks"):
            # Two 10-byte files occupy at least one block each, never 20 bytes.
            self.assertGreater(measured["allocated_bytes"], 20)
            self.assertNotIn("measure", measured)
        else:
            self.assertEqual(measured["measure"], "apparent")

    def test_a_hard_link_is_counted_once(self) -> None:
        original = write(self.target / "a.bin", 8192)
        alone = storage_sample.allocated(self.target)["allocated_bytes"]
        os.link(original, self.target / "b.bin")
        linked = storage_sample.allocated(self.target)
        self.assertEqual(linked["files"], 1)
        self.assertEqual(linked["allocated_bytes"], alone)

    @unittest.skipIf(os.name == "nt", "creating a symlink needs privileges on Windows")
    def test_a_symlink_is_not_followed_out_of_the_measured_tree(self) -> None:
        write(self.root / "elsewhere" / "big.bin", 1 << 20)
        self.target.mkdir()
        (self.target / "link").symlink_to(self.root / "elsewhere", target_is_directory=True)
        measured = storage_sample.allocated(self.target)
        self.assertLess(measured["allocated_bytes"], 1 << 20)

    def test_a_missing_directory_is_absent_with_zero_bytes(self) -> None:
        measured = storage_sample.allocated(self.root / "never-created")
        self.assertEqual(
            measured,
            {
                "path": str(self.root / "never-created"),
                "exists": False,
                "allocated_bytes": 0,
                "files": 0,
                "errors": 0,
            },
        )


class FilesystemTests(Fixture):
    def test_a_missing_path_is_measured_at_its_nearest_existing_parent(self) -> None:
        missing = self.root / "a" / "b" / "c"
        measured = storage_sample.filesystem(missing)
        self.assertFalse(measured["exists"])
        self.assertEqual(measured["probed"], str(self.root))
        self.assertIsInstance(measured["free_bytes"], int)
        self.assertGreater(measured["free_bytes"], 0)

    def test_a_failed_probe_is_reported_unavailable_not_zero(self) -> None:
        def broken(_path: str):
            raise OSError(5, "Input/output error")

        measured = storage_sample.filesystem(self.root, statvfs=broken)
        self.assertIn("unavailable", measured["free_bytes"])
        self.assertIn("Input/output error", measured["free_bytes"]["unavailable"])
        self.assertIn("unavailable", measured["free_inodes"])
        self.assertNotIn("total_bytes", measured)

    def test_a_failed_probe_still_writes_its_line(self) -> None:
        def broken(_path: str):
            raise OSError(5, "Input/output error")

        args = storage_sample.parser().parse_args(
            ["--out", str(self.out), "--label", "x", "--target", str(self.target),
             "--output", str(self.output), "--temp", str(self.root)]
        )
        document = storage_sample.sample(args, statvfs=broken)
        for location in ("target", "output", "temp"):
            self.assertIn("unavailable", document["locations"][location]["free_bytes"])
        # Directory sizes are independent of the probe and still measured.
        self.assertEqual(document["categories"]["target"]["allocated_bytes"], 0)


class SampleDocumentTests(Fixture):
    def test_categories_stay_apart_and_name_their_overlap(self) -> None:
        write(self.target / "debug" / "deps" / "x.rlib", 100)
        write(self.target / storage_sample.SCHEMATIC_CHILD_TARGET / "debug" / "y", 100)
        write(self.output / "build-a.tar.zst", 100)
        write(self.output / "build-a-sidecars" / "md", 100)
        write(self.output / "build-b-inventory" / "scratch", 100)
        write(self.output / "build-a.manifest.json", 10)
        write(self.output / "build-a.status.json", 10)

        self.assertEqual(self.run_main("--label", "record", "--record", "build-a",
                                       "--outcome", "success:produce"), 0)
        (line,) = self.lines()
        self.assertEqual(line["schema"], storage_sample.SCHEMA)
        self.assertEqual((line["label"], line["record"], line["outcome"]),
                         ("record", "build-a", "success:produce"))
        categories = line["categories"]
        self.assertEqual(categories["archives"]["entries"], 2)
        self.assertEqual(categories["archives"]["files"], 2)
        self.assertEqual(categories["inventory"]["entries"], 1)
        self.assertEqual(categories["metadata"]["entries"], 2)
        self.assertEqual(categories["schematic_guard"]["overlaps"], ["target"])
        self.assertEqual(categories["target"]["overlaps"], [])
        # The guard's output is inside the target and is not added to it twice.
        self.assertGreaterEqual(
            categories["target"]["allocated_bytes"],
            categories["schematic_guard"]["allocated_bytes"],
        )

    def test_locations_on_one_filesystem_are_reported_shared(self) -> None:
        self.target.mkdir()
        self.output.mkdir()
        self.run_main("--label", "start")
        (line,) = self.lines()
        self.assertIs(line["shared_filesystem"]["target+output"], True)
        self.assertEqual(
            line["locations"]["target"]["device"], line["locations"]["output"]["device"]
        )

    def test_missing_locations_are_sampled_without_failing(self) -> None:
        self.assertEqual(self.run_main("--label", "before-cache-restore"), 0)
        (line,) = self.lines()
        self.assertFalse(line["locations"]["target"]["exists"])
        self.assertFalse(line["locations"]["output"]["exists"])
        self.assertFalse(line["categories"]["target"]["errors"])
        self.assertEqual(line["categories"]["archives"]["entries"], 0)

    def test_later_samples_append_and_measure_elapsed_from_the_first(self) -> None:
        self.run_main("--label", "one")
        self.run_main("--label", "two", "--cache-hit", "false")
        first, second = self.lines()
        self.assertEqual((first["label"], second["label"]), ("one", "two"))
        self.assertEqual(first["elapsed_s"], 0.0)
        self.assertGreaterEqual(second["elapsed_s"], 0.0)
        self.assertEqual(second["cache_hit"], "false")
        self.assertIsNone(first["cache_hit"])

    def test_an_unwritable_output_file_exits_one(self) -> None:
        self.out.mkdir(parents=True)
        self.assertEqual(self.run_main("--label", "x"), 1)

    def test_a_missing_required_argument_is_a_usage_error(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            storage_sample.main(["--label", "x"])
        self.assertEqual(raised.exception.code, 2)


class RestoreClassificationTests(Fixture):
    def plan(self) -> Path:
        path = self.root / "plan.json"
        path.write_text(json.dumps({"builds": [
            {"producer": "ubuntu-latest",
             "identity": {"target": "x86_64-unknown-linux-gnu", "profile": "test"}},
            {"producer": "windows-latest",
             "identity": {"target": "x86_64-pc-windows-msvc", "profile": "release"}},
        ]}), encoding="utf-8")
        return path

    def test_live_paths_follow_this_owners_identities_only(self) -> None:
        plan = json.loads(self.plan().read_text(encoding="utf-8"))
        self.assertEqual(
            storage_sample.live_paths(plan, "ubuntu-latest", ["release"]),
            ["debug", "release", "x86_64-unknown-linux-gnu/debug"],
        )

    def test_only_restored_profiles_no_identity_names_are_obsolete(self) -> None:
        self.target.mkdir()
        write(self.target / "existing.bin", 10)
        self.run_main("--label", "before-cache-restore", "--inventory")

        triple = self.target / "x86_64-unknown-linux-gnu"
        profile(triple / "debug")            # live: the planned triple/profile
        profile(triple / "bench-ish")        # obsolete: a profile nobody plans
        profile(self.target / "debug")       # live: host build scripts
        profile(self.target / "profiling")   # obsolete: an unplanned host profile
        profile(self.target / "release")     # live: the producer tool
        profile(self.target / storage_sample.SCHEMATIC_CHILD_TARGET / "debug")
        write(self.target / "nextest" / "junit.xml", 10)  # not a profile: unknown

        self.run_main(
            "--label", "after-cache-restore", "--cache-hit", "true", "--inventory",
            "--restored-since", "before-cache-restore", "--plan", str(self.plan()),
            "--producer", "ubuntu-latest", "--live-path", "release",
            "--live-path", storage_sample.SCHEMATIC_CHILD_TARGET,
        )
        restore = self.lines()[-1]["restore"]
        self.assertEqual(
            sorted(restore["obsolete"]),
            ["profiling", "x86_64-unknown-linux-gnu/bench-ish"],
        )
        self.assertEqual(restore["obsolete_bytes"], sum(restore["obsolete"].values()))
        self.assertGreater(restore["obsolete_bytes"], 0)
        self.assertEqual(
            sorted(restore["live"]),
            ["debug", "release", storage_sample.SCHEMATIC_CHILD_TARGET,
             "x86_64-unknown-linux-gnu/debug"],
        )
        self.assertIn("nextest", restore["unknown"])
        self.assertNotIn("existing.bin", json.dumps(restore))

    def test_without_an_earlier_inventory_the_classification_is_unavailable(self) -> None:
        self.run_main("--label", "after-cache-restore", "--inventory",
                      "--restored-since", "before-cache-restore")
        self.assertIn("unavailable", self.lines()[-1]["restore"])

    def test_an_unreadable_plan_leaves_the_classification_unavailable(self) -> None:
        self.target.mkdir()
        self.run_main("--label", "before-cache-restore", "--inventory")
        bad = self.root / "plan.json"
        bad.write_text("{not json", encoding="utf-8")
        self.assertEqual(
            self.run_main("--label", "after", "--inventory", "--restored-since",
                          "before-cache-restore", "--plan", str(bad), "--producer", "x"),
            0,
        )
        self.assertIn("plan unreadable", self.lines()[-1]["restore"]["unavailable"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
