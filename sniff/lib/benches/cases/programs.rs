//! Program detection Criterion benches.
//!
//! Phase 4 baseline benchmarks for program lookup behavior. These exist
//! alongside the broader `inventory::programs_detect` bench (which
//! exercises the full 8-category fan-out) and zoom in on:
//!
//! - the eager vs. lazy `ExecutableIndex` build paths (the optimization
//!   restored in Phase 3.3)
//! - bulk lookup speed when many program names are resolved against a
//!   shared index (`ProgramsInfo::detect`'s hot path)
//! - the upper-bound fan-out cost via `ProgramsInfo::detect` itself,
//!   re-registered here under `programs/` so the high-level identifier
//!   sits next to the lower-level building blocks
//!
//! PATH-length scaling is intentionally measured by `hyperfine` against
//! the compiled CLI (see `sniff/lib/README.md` for the documented
//! invocation): mutating `PATH` from inside Criterion's test harness
//! would race with other benches that read environment state.

use criterion::{Criterion, Throughput, black_box};
use sniff::ProgramsInfo;
use sniff::executable_index::{ExecutableIndex, find_programs_with_source_from_index};

use crate::support::util;

/// Representative bulk-lookup workload — a mix of programs that almost
/// always exist on developer machines and a few that may not. The mix
/// keeps the eager and lazy paths comparable: the eager index resolves
/// every name from its in-memory map, while the lazy path falls back to
/// `which` on each miss.
#[cfg(unix)]
const BULK_LOOKUPS: &[&str] = &[
    "ls", "cat", "sh", "env", "git", "cargo", "rustc", "node", "python3", "make", "uv", "go",
    "ruby", "perl", "awk", "sed", "grep", "find", "tar", "gzip", "curl", "wget", "ssh", "rsync",
];

#[cfg(windows)]
const BULK_LOOKUPS: &[&str] = &[
    "cmd",
    "powershell",
    "explorer",
    "git",
    "cargo",
    "rustc",
    "node",
    "python",
    "where",
    "ping",
];

/// Large bulk-lookup workload simulating a long PATH with many program
/// names to resolve.  This stresses the O(1) eager HashMap probe vs.
/// the per-name `which` traversal on the lazy path.
#[cfg(unix)]
const BULK_LOOKUPS_LARGE: &[&str] = &[
    "ls",
    "cat",
    "sh",
    "env",
    "git",
    "cargo",
    "rustc",
    "node",
    "python3",
    "make",
    "uv",
    "go",
    "ruby",
    "perl",
    "awk",
    "sed",
    "grep",
    "find",
    "tar",
    "gzip",
    "curl",
    "wget",
    "ssh",
    "rsync",
    "bash",
    "zsh",
    "fish",
    "dash",
    "tmux",
    "screen",
    "vim",
    "nvim",
    "emacs",
    "nano",
    "code",
    "cursor",
    "zed",
    "subl",
    "sublime-text",
    "docker",
    "podman",
    "kubectl",
    "helm",
    "terraform",
    "ansible",
    "vagrant",
    "aws",
    "gcloud",
    "az",
    "firebase",
    "flyctl",
    "heroku",
    "npm",
    "pnpm",
    "yarn",
    "bun",
    "npx",
    "node",
    "deno",
    "pip",
    "pip3",
    "poetry",
    "uv",
    "conda",
    "mamba",
    "rustc",
    "rustup",
    "cargo",
    "clippy",
    "rustfmt",
    "cargo-nextest",
    "go",
    "gofmt",
    "golint",
    "dlv",
    "javac",
    "java",
    "mvn",
    "gradle",
    "kotlin",
    "gcc",
    "g++",
    "clang",
    "clang++",
    "make",
    "cmake",
    "ninja",
    "gdb",
    "lldb",
    "valgrind",
    "perf",
    "htop",
    "top",
    "ps",
    "pgrep",
    "pkill",
    "kill",
    "killall",
    "df",
    "du",
    "free",
    "uptime",
    "uname",
    "whoami",
    "id",
    "ping",
    "traceroute",
    "netstat",
    "ss",
    "nc",
    "nmap",
    "openssl",
    "gpg",
    "ssh-keygen",
    "scp",
    "sftp",
    "zip",
    "unzip",
    "bzip2",
    "xz",
    "7z",
    "jq",
    "yq",
    "xargs",
    "parallel",
    "watch",
    "timeout",
    "rg",
    "fd",
    "fzf",
    "bat",
    "exa",
    "eza",
    "dust",
    "procs",
    "sd",
    "hyperfine",
    "tldr",
    "cheat",
    "howdoi",
    "bandwhich",
    "bottom",
    "gping",
    "convert",
    "ffmpeg",
    "ffprobe",
    "youtube-dl",
    "yt-dlp",
    "play",
    "say",
    "espeak",
    "espeak-ng",
    "festival",
    "qlmanage",
    "mdls",
    "mdfind",
    "xcrun",
    "xcodebuild",
    "launchctl",
    "plutil",
    "sw_vers",
    "system_profiler",
    "brew",
    "port",
    "pkgutil",
    "installer",
    "osascript",
    "open",
    "pbcopy",
    "pbpaste",
    "say",
    // ... and some that probably don't exist to exercise misses
    "__sniff_test_01__",
    "__sniff_test_02__",
    "__sniff_test_03__",
];

#[cfg(windows)]
const BULK_LOOKUPS_LARGE: &[&str] = &[
    "cmd",
    "powershell",
    "explorer",
    "git",
    "cargo",
    "rustc",
    "node",
    "python",
    "where",
    "ping",
    "winget",
    "choco",
    "scoop",
    "npm",
    "pnpm",
    "yarn",
    "dotnet",
    "msbuild",
    "cl",
    "link",
    "nmake",
    "cmake",
    "docker",
    "kubectl",
    "aws",
    "az",
    "gcloud",
    "ssh",
    "curl",
    "tar",
    "gzip",
    "7z",
    "code",
    "notepad",
    "calc",
    "tasklist",
    "taskkill",
    "ipconfig",
    "tracert",
    "nslookup",
    "netstat",
    "systeminfo",
    "driverquery",
    "sc",
    "reg",
    "findstr",
    "sort",
    "more",
    "type",
    "copy",
    "move",
    "del",
    "mkdir",
    "rmdir",
    "ren",
    "cls",
    "echo",
    "set",
    "path",
    "ver",
    "vol",
    "date",
    "time",
    "start",
    "runas",
    "shutdown",
    "sfc",
    "dism",
    "chkdsk",
    "diskpart",
    "format",
    "defrag",
    "robocopy",
    "xcopy",
    "fc",
    "comp",
    "attrib",
    "cipher",
    "compact",
    "convert",
    "expand",
    "replace",
    "takeown",
    "icacls",
    "cacls",
    "xcalcs",
    "subinacl",
    "psexec",
    "procmon",
    "procexp",
    "tcpview",
    "autoruns",
    "sigcheck",
    "streams",
    "shareenum",
    "accesschk",
    "listdlls",
    "handle",
    "vmmap",
    "rammap",
    "diskext",
    "contig",
    "du",
    "junction",
    "ldmdump",
    "livekd",
    "loadord",
    "logonsessions",
    "pipelist",
    "portmon",
    "pstools",
    "rootkitrevealer",
    "shellrunas",
    "syinternals",
    "zoomit",
    "__sniff_test_01__",
    "__sniff_test_02__",
    "__sniff_test_03__",
];

pub fn register(c: &mut Criterion) {
    // ---------- ExecutableIndex build modes ----------
    let mut build_group = util::configure_group(c, "programs");

    build_group.bench_function("executable_index_build_lazy", |b| {
        b.iter(|| {
            let index = ExecutableIndex::build();
            black_box(index);
        });
    });

    build_group.bench_function("executable_index_build_eager_path_scan", |b| {
        b.iter(|| {
            let index = ExecutableIndex::build_eager_path();
            black_box(index);
        });
    });

    build_group.finish();

    // ---------- bulk lookup against pre-built indexes ----------
    let mut lookup_group = util::configure_group(c, "programs_bulk_lookup");

    let lazy_index = ExecutableIndex::build();
    lookup_group.throughput(Throughput::Elements(BULK_LOOKUPS.len() as u64));
    lookup_group.bench_function("bulk_lookup_25_names_lazy_index", |b| {
        b.iter(|| {
            let results = find_programs_with_source_from_index(
                black_box(&lazy_index),
                black_box(BULK_LOOKUPS),
            );
            black_box(results);
        });
    });

    let eager_index = ExecutableIndex::build_eager_path();
    lookup_group.throughput(Throughput::Elements(BULK_LOOKUPS.len() as u64));
    lookup_group.bench_function("bulk_lookup_25_names_eager_index", |b| {
        b.iter(|| {
            let results = find_programs_with_source_from_index(
                black_box(&eager_index),
                black_box(BULK_LOOKUPS),
            );
            black_box(results);
        });
    });

    // Large workload: ~150 names to simulate long-PATH bulk detection.
    lookup_group.throughput(Throughput::Elements(BULK_LOOKUPS_LARGE.len() as u64));
    lookup_group.bench_function("bulk_lookup_150_names_lazy_index", |b| {
        b.iter(|| {
            let results = find_programs_with_source_from_index(
                black_box(&lazy_index),
                black_box(BULK_LOOKUPS_LARGE),
            );
            black_box(results);
        });
    });

    lookup_group.throughput(Throughput::Elements(BULK_LOOKUPS_LARGE.len() as u64));
    lookup_group.bench_function("bulk_lookup_150_names_eager_index", |b| {
        b.iter(|| {
            let results = find_programs_with_source_from_index(
                black_box(&eager_index),
                black_box(BULK_LOOKUPS_LARGE),
            );
            black_box(results);
        });
    });

    lookup_group.finish();

    // ---------- end-to-end fan-out ----------
    let mut fanout_group = util::configure_slow_group(c, "programs_fanout");

    fanout_group.bench_function("programs_detect_all_8_categories_fanout", |b| {
        b.iter(|| {
            let programs = ProgramsInfo::detect();
            black_box(programs);
        });
    });

    fanout_group.finish();
}
