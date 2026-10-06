# Filesystem process usage queries

**Planned:** the API and CLI described here are not implemented yet.

Query a file or directory tree to find observable processes using it. The
focused library API is `sniff::filesystem::query::query_path_usage(path,
&PathUsageOptions::default())`; a directory query includes descendants by
default. Normal host and repository reports will not perform this scan.

The result will contain process identity, matched paths, evidence, observation
times, and coverage limitations. Evidence distinguishes watcher registrations,
open handles, working directories, and loaded modules. A handle identifies usage;
it does not necessarily prevent deletion. An empty result does not establish
that the directory is unused.

| Environment | Planned discovery | Coverage limit |
| --- | --- | --- |
| Linux/WSL2 | Open descriptors, cwd, inotify registrations | Permissions and unsupported watcher mechanisms |
| macOS | Vnode descriptors and cwd, including observable event-only opens | FSEvents subscriptions cannot be enumerated systemwide |
| Windows | File/directory handle and loaded-module owners | Handle ownership does not establish watcher status or deletion blocking |

For example, a Node process with its cwd in a checkout will appear as working
directory evidence. A Node process watching that checkout through FSEvents may
have no observable path association and be absent from matches.

Each mechanism will report `complete`, `partial`, `unsupported`, or `failed`
coverage. These statuses describe that mechanism's observation, not the absence
of all possible writers. Access denials and exhausted deadlines will remain
visible in partial reports. The default deadline will be two seconds.

Discovery will be read-only, without privilege elevation, process termination,
network requests, or persistent process caches. Callers will decide whether
observed usage justifies a warning or refusal before their own operation.

Use the planned [CLI query](../cli/filesystem_query.md) to inspect the same
report from a terminal.
