"""Produce a read-only sweep report from the Unix sweeper's policy and candidates."""

from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import plistlib
import re
import socket
import stat
import subprocess
import sys

GIB = 1024**3


def display(size):
    for power, unit in ((3, "GiB"), (2, "MiB"), (1, "KiB")):
        if size >= 1024**power:
            return f"{size / 1024**power:,.1f} {unit}"
    return f"{size:,.1f} B"


def apfs_container(path):
    try:
        # diskutil accepts the mount point or its device, not an arbitrary
        # directory within the volume. df resolves a checkout to that device.
        usage = subprocess.check_output(["/bin/df", "-P", str(path)],
                                        stderr=subprocess.DEVNULL, text=True)
        device = usage.splitlines()[1].split()[0]
        if not device.startswith("/dev/"):
            return None
        data = subprocess.check_output(
            ["/usr/sbin/diskutil", "info", "-plist", device],
            stderr=subprocess.DEVNULL,
        )
        return plistlib.loads(data).get("APFSContainerReference")
    except (OSError, subprocess.CalledProcessError, plistlib.InvalidFileException, ValueError, IndexError):
        return None


class Allocation:
    def __init__(self, root):
        self.volumes = {}
        self.devices = {}
        self.selected = set()
        self.links_seen = Counter()
        self.incomplete = False
        for label, path in (("Checkout", root), ("System", Path("/")), ("Home", Path.home())):
            self.volume(path, label)
        output = os.environ.get("CARGO_TARGET_DIR")
        if output and Path(output).is_dir():
            self.volume(Path(output), "Cargo output")

    def volume(self, path, label):
        device = path.stat().st_dev
        if device not in self.devices:
            info = os.statvfs(path)
            block = info.f_frsize
            capacity = info.f_blocks * block
            pool = device
            if sys.platform == "darwin":
                container = apfs_container(path)
                if container:
                    # Volumes with individual capacity limits keep their own row.
                    pool = (container, capacity)
                    label = f"{label} ({container})"
            self.devices[device] = pool
            self.volumes.setdefault(pool, (label, capacity,
                                           (info.f_blocks - info.f_bfree) * block, info.f_bavail * block))
        return self.devices[device]

    def measure(self, candidates, label):
        logical = 0
        blocks = Counter()
        for candidate in sorted({os.fspath(path) for path in candidates}, key=len):
            ancestor = candidate
            while ancestor not in self.selected:
                parent = os.path.dirname(ancestor)
                if parent == ancestor:
                    break
                ancestor = parent
            if ancestor in self.selected:
                continue
            pending = [candidate]
            while pending:
                path = pending.pop()
                if path in self.selected:
                    continue
                try:
                    metadata = os.lstat(path)
                    directory = stat.S_ISDIR(metadata.st_mode)
                    if directory:
                        with os.scandir(path) as entries:
                            pending.extend(entry.path for entry in entries)
                    else:
                        logical += metadata.st_size
                    device = metadata.st_dev
                    pool = self.devices.get(device)
                    if pool is None:
                        volume_path = os.path.dirname(path) if stat.S_ISLNK(metadata.st_mode) else path
                        pool = self.volume(Path(volume_path), label)
                except OSError:
                    self.incomplete = True
                    continue
                if not directory and metadata.st_nlink > 1:
                    inode = (device, metadata.st_ino)
                    self.links_seen[inode] += 1
                    if self.links_seen[inode] != metadata.st_nlink:
                        continue
                blocks[pool] += metadata.st_blocks * 512
            self.selected.add(candidate)
        return logical, blocks


def cargo_candidates(root, *criterion):
    candidates = defaultdict(set)
    excess = {}
    failed = False
    target = None
    criterion_name = {
        "--time": "remove_older_than",
        "--installed": "remove_not_built_with",
        "--maxsize": "remove_older_until_fits",
    }[criterion[0]]
    header = "[DEBUG] cleaning: "
    suffix = f" with {criterion_name}"
    try:
        child = subprocess.Popen(
            ["cargo", "sweep", "--dry-run", "--verbose", "--recursive", *criterion, str(root)],
            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace",
        )
    except OSError:
        return candidates, excess, True
    with child:
        for line in child.stdout:
            line = line.rstrip()
            try:
                if line.startswith(header) and line.endswith(suffix):
                    path = line[len(header):-len(suffix)]
                    target = Path(json.loads(path))
                elif target is not None and line.startswith("[DEBUG] Would remove: "):
                    path = line[len("[DEBUG] Would remove: "):]
                    candidates[target].add(Path(json.loads(path)))
                elif target is not None and line.startswith("[DEBUG] size_to_remove: "):
                    excess[target] = int(line.rsplit(": ", 1)[1])
                elif line.startswith("[ERROR]"):
                    failed = True
                elif line.startswith("[INFO] Would clean:") and "nothing from" not in line:
                    if target is None or not candidates[target]:
                        failed = True
            except (ValueError, TypeError):
                failed = True
        failed |= child.wait() != 0
    return candidates, excess, failed


def preview(root, age, floor, pressure, roomy, incremental, orphans):
    disk = Allocation(root)
    checkout = disk.volume(root, "Checkout")
    logical = Counter()
    savings = {}
    missing = set()

    def record(label, groups, *, excess=None):
        savings[label] = Counter()
        for target, candidates in groups.items():
            if excess is not None and target in excess and excess[target] <= logical[target]:
                continue
            removed, blocks = disk.measure(candidates, label)
            if excess is not None and removed:
                # Without changing the tree, Cargo sizes the cap pass against
                # the original target. Scale its additional candidates by the
                # remaining logical overage after the earlier passes.
                remaining = max(0, excess.get(target, removed) - logical[target])
                blocks = Counter({dev: int(size * min(removed, remaining) / removed)
                                  for dev, size in blocks.items()})
            logical[target] += removed
            savings[label].update(blocks)

    # The toolchain scan reads fingerprints. It can overlap the incremental
    # size walk, but the age scan still follows it to preserve access-time ordering.
    with ThreadPoolExecutor(max_workers=1) as worker:
        toolchains = worker.submit(cargo_candidates, root, "--installed")
        record("Incremental caches", incremental)
        groups, _, failed = toolchains.result()
    label = "Artifacts from removed toolchains"
    record(label, groups)
    if failed:
        missing.add(label)

    label = f"Artifacts older than {age} days"
    groups, _, failed = cargo_candidates(root, "--time", age)
    record(label, groups)
    if failed:
        missing.add(label)

    early = sum(items[checkout] for items in savings.values())
    cap = roomy if disk.volumes[checkout][3] + early >= int(floor) * GIB else pressure
    cap_display = " ".join(re.findall(r"\d+(?:\.\d+)?|[A-Za-z]+", cap)) or cap
    label = f"Cargo size cap ({cap_display})"
    groups, excess, failed = cargo_candidates(root, "--maxsize", cap)
    record(label, groups, excess=excess)
    if failed:
        missing.add(label)

    record("Stale external targets", {root: orphans})
    return disk, savings, missing


def size_cell(size, width=12, *, estimated=False):
    amount, unit = display(size).split()
    number = ("~" if estimated else "") + amount
    return f"{number:>{width - 4}} {unit:<3}"


def draw(disk, savings, missing, schedule, *, mode="plain"):
    def style(text, code):
        if mode == "bold":
            code = "1" if code.split(";")[0] == "1" else ""
        return f"\033[{code}m{text}\033[0m" if mode != "plain" and code else text

    def heading(text):
        return style(f"{text}:", "1;36")

    print(f"{heading('Sweep preview')} {style(socket.gethostname(), '1')}")
    print(f"\n{heading('Disk space')}")
    print(style(f"  {'Volume':<20} {'Total':>12} {'Used':>12} {'Free now':>12} {'Free after':>14}", "1"))
    totals = Counter()
    for buckets in savings.values():
        for device, value in buckets.items():
            used = disk.volumes[device][2]
            buckets[device] = min(value, used - totals[device])
        totals.update(buckets)
    partial = disk.incomplete or bool(missing)
    for device, (label, total, used, free) in disk.volumes.items():
        after = ("Incomplete".rjust(14) if partial
                 else size_cell(free + totals[device], 14, estimated=True))
        cells = " ".join(size_cell(size) for size in (total, used, free))
        print(f"  {label:<20} {cells} {style(after.rstrip(), '1;32' if not partial else '33')}")

    print(f"\n{heading('Expected to reclaim')}")
    positive = {label: sum(items.values()) for label, items in savings.items() if sum(items.values())}
    if positive:
        for label, size in positive.items():
            print(f"  {label:<36} {size_cell(size, estimated=True).rstrip()}")
        total = size_cell(sum(positive.values()), estimated=True).rstrip()
        print(f"  {style('Estimated reclaim'.ljust(36), '1')} {style(total, '1;32')}")
    else:
        print("  None")

    print(f"\n{heading('No savings expected')}")
    zero = [label for label in savings if label not in positive and label not in missing]
    print("\n".join(f"  {label}" for label in zero) or "  None")
    if partial:
        names = ", ".join(label for label in savings if label in missing)
        print(f"\n{style('Estimate incomplete', '1;33')}{f' for {names}' if names else ''}.")

    state, *details = schedule.split(" | ")
    warning = state.startswith("Not ") or any(
        detail.startswith(("Checkout missing", "Last result")) for detail in details
    )
    detail_text = "".join(f" {style('|', '2')} {detail}" for detail in details)
    print(f"\n{heading('Schedule')} {style(state, '33' if warning else '32')}{detail_text}")
    print(style("Shared blocks and snapshots can reduce actual savings.", "2"))


def main():
    fields = iter(os.fsdecode(sys.stdin.buffer.read()).rstrip("\0").split("\0"))
    root, age, floor, pressure, roomy = Path(next(fields)), *(next(fields) for _ in range(4))
    incremental = defaultdict(set)
    orphans = set()
    for kind in fields:
        if kind == "incremental":
            target = Path(next(fields))
            incremental[target].add(Path(next(fields)))
        else:
            orphans.add(Path(next(fields)))
    mode = os.environ.get("SWEEP_STATUS_RENDER")
    if mode is None and sys.stdout.isatty() and os.environ.get("TERM", "dumb") != "dumb":
        mode = "bold" if os.environ.get("NO_COLOR") else "color"
    draw(*preview(root, age, floor, pressure, roomy, incremental, orphans), sys.argv[1], mode=mode or "plain")


if __name__ == "__main__":
    main()
