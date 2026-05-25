ken in darkmatter/darkmatter on  darkmatter [$!?⇡]
💻❯ RUST_LOG=trace c compose prompts/implement-phase.md plan="features/2026-05-08-expression-syntax/plan.md" -y --claude total_phases=6
11:17:21.604 TRACE [cli_invocation>from_str] close (busy: 1.13ms, idle: 1.00µs) in biscuit-file/lib/src/json5/types.rs:117
11:17:21.607 DEBUG [cli_invocation] Loaded ClaudineConfig (user) in claudine/lib/src/dispatch/loader.rs:404
11:17:21.609 TRACE [cli_invocation>from_str] close (busy: 134µs, idle: 958ns) in biscuit-file/lib/src/json5/types.rs:117
11:17:21.609 TRACE [cli_invocation>from_str] close (busy: 1.46µs, idle: 333ns) in biscuit-file/lib/src/json5/types.rs:117
11:17:21.610 TRACE [cli_invocation] parsing file reference in biscuit-file/lib/src/file_reference/parse.rs:10
11:17:21.610 DEBUG [cli_invocation] parsed reference in biscuit-file/lib/src/file_reference/parse.rs:39
11:17:21.610 TRACE [cli_invocation] searching for git root in biscuit-file/lib/src/file_reference/context.rs:57
11:17:21.618 DEBUG [cli_invocation] found git root in biscuit-file/lib/src/file_reference/context.rs:63
11:17:21.618 TRACE [cli_invocation] searching for package area in biscuit-file/lib/src/file_reference/context.rs:86
11:17:21.656 DEBUG [cli_invocation] built resolution context in biscuit-file/lib/src/file_reference/context.rs:22
11:17:21.656 TRACE [cli_invocation] searching for git root in biscuit-file/lib/src/file_reference/context.rs:57
11:17:21.656 DEBUG [cli_invocation] found git root in biscuit-file/lib/src/file_reference/context.rs:63
11:17:21.656 TRACE [cli_invocation] checking candidate in biscuit-file/lib/src/file_reference/resolve.rs:43
11:17:21.656 TRACE [cli_invocation] checking candidate in biscuit-file/lib/src/file_reference/resolve.rs:43
11:17:21.656 DEBUG [cli_invocation] resolved file reference in biscuit-file/lib/src/file_reference/resolve.rs:45
11:17:21.659 INFO [cli_invocation>compose>discover] close (busy: 295µs, idle: 709ns) in sniff/lib/src/filesystem/git/types.rs:511
11:17:22.225 INFO [cli_invocation>compose>discover] close (busy: 374µs, idle: 1.21µs) in sniff/lib/src/filesystem/git/types.rs:511
11:17:22.920 TRACE [cli_invocation>compose>from_str] close (busy: 19.0µs, idle: 1.79µs) in biscuit-file/lib/src/json5/types.rs:117
11:17:22.920 DEBUG [cli_invocation>compose] Loaded ClaudineConfig (user) in claudine/lib/src/dispatch/loader.rs:404
11:17:22.921 TRACE [cli_invocation>compose>from_str] close (busy: 2.54µs, idle: 541ns) in biscuit-file/lib/src/json5/types.rs:117
11:17:22.921 DEBUG [cli_invocation>compose] Loaded RepoOverrideConfig in claudine/lib/src/dispatch/loader.rs:429
11:17:22.922 TRACE registering event source with poller: token=Token(1), interests=READABLE
11:17:22.925 TRACE registering event source with poller: token=Token(39942897536), interests=READABLE | WRITABLE
11:17:22.925 TRACE registering event source with poller: token=Token(39942896000), interests=READABLE | WRITABLE
11:17:23.507 TRACE deregistering event source from poller
11:17:23.507 TRACE deregistering event source from poller
11:17:23.508 TRACE registering event source with poller: token=Token(39942890112), interests=READABLE | WRITABLE
11:17:23.508 TRACE registering event source with poller: token=Token(39942895232), interests=READABLE | WRITABLE
11:17:24.076 TRACE deregistering event source from poller
11:17:24.076 TRACE deregistering event source from poller
11:17:24.079 INFO [cli_invocation>compose>discover] close (busy: 389µs, idle: 999ns) in sniff/lib/src/filesystem/git/types.rs:511
11:17:24.079 DEBUG [detect_os_with_request] performance stage complete in sniff/lib/src/performance.rs:234
11:17:24.079 DEBUG [detect_os_with_request>build_path_only] lazy PATH index created in sniff/lib/src/executable_index.rs:118
11:17:24.079 INFO [detect_os_with_request>build_path_only] close (busy: 7.42µs, idle: 249ns) in sniff/lib/src/executable_index.rs:79
11:17:24.079 INFO [discover] close (busy: 324µs, idle: 917ns) in sniff/lib/src/filesystem/git/types.rs:511
11:17:24.080 DEBUG [detect_hardware_with_request] performance stage complete in sniff/lib/src/performance.rs:234
11:17:24.080 INFO [detect_hardware_with_request] close (busy: 588µs, idle: 583ns) in sniff/lib/src/hardware/mod.rs:61
11:17:24.081 TRACE [detect_os_with_request] PATH command probe complete in sniff/lib/src/os/package_manager.rs:472
11:17:24.081 TRACE [detect_os_with_request] PATH command probe complete in sniff/lib/src/os/package_manager.rs:472
11:17:24.081 DEBUG [detect_os_with_request] performance stage complete in sniff/lib/src/performance.rs:234
11:17:24.081 DEBUG [detect_os_with_request] performance stage complete in sniff/lib/src/performance.rs:234
11:17:24.081 DEBUG [detect_os_with_request] performance stage complete in sniff/lib/src/performance.rs:234
11:17:24.081 INFO [detect_os_with_request] close (busy: 2.16ms, idle: 500ns) in sniff/lib/src/os/mod.rs:210
11:17:24.087 INFO [detect_repo_structure] close (busy: 8.10ms, idle: 584ns) in sniff/lib/src/filesystem/repo/types.rs:296
11:17:24.088 TRACE [cli_invocation>compose] automatically discovered CWD: /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter
11:17:24.089 DEBUG [cli_invocation>compose] opened gitignore file: /Users/ken/.config/git/ignore
11:17:24.089 DEBUG [cli_invocation>compose] built glob set; 1 literals, 0 basenames, 0 extensions, 0 prefixes, 1 suffixes, 0 required extensions, 0 regexes
11:17:24.090 DEBUG [cli_invocation>compose] opened gitignore file: /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/npm-debug.log*")` converted to regex: `"(?-u)^(?:/?|.*/)npm\\-debug\\.log[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/yarn-debug.log*")` converted to regex: `"(?-u)^(?:/?|.*/)yarn\\-debug\\.log[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/yarn-error.log*")` converted to regex: `"(?-u)^(?:/?|.*/)yarn\\-error\\.log[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("frontend/.vite-ssg-dist/**/*")` converted to regex: `"(?-u)^frontend/\\.vite\\-ssg\\-dist(?:/|/.*/)[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("trace/*")` converted to regex: `"(?-u)^trace/[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob(".trace/*")` converted to regex: `"(?-u)^\\.trace/[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/target/*")` converted to regex: `"(?-u)^(?:/?|.*/)target/[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/*.afphoto~lock*")` converted to regex: `"(?-u)^(?:/?|.*/)[^/]*\\.afphoto\\~lock[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/.aider*")` converted to regex: `"(?-u)^(?:/?|.*/)\\.aider[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/debug/*")` converted to regex: `"(?-u)^(?:/?|.*/)debug/[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] glob `Glob("**/Cargo.lock/*")` converted to regex: `"(?-u)^(?:/?|.*/)Cargo\\.lock/[^/]*$"`
11:17:24.090 DEBUG [cli_invocation>compose] built glob set; 8 literals, 20 basenames, 5 extensions, 0 prefixes, 4 suffixes, 5 required extensions, 11 regexes
11:17:24.090 DEBUG [cli_invocation>compose] opened gitignore file: /Volumes/coding/personal/rusty-biscuit/.git/worktrees/darkmatter/../../info/exclude
11:17:24.091 DEBUG [cli_invocation>compose] opened gitignore file: /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/research/cli/.gitignore
11:17:24.091 DEBUG [cli_invocation>compose] built glob set; 1 literals, 0 basenames, 0 extensions, 0 prefixes, 0 suffixes, 0 required extensions, 0 regexes
11:17:24.091 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/research/cli/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.091 DEBUG [cli_invocation>compose] opened gitignore file: /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/research/lib/.gitignore
11:17:24.091 DEBUG [cli_invocation>compose] built glob set; 1 literals, 0 basenames, 0 extensions, 0 prefixes, 0 suffixes, 0 required extensions, 0 regexes
11:17:24.091 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/research/lib/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.092 DEBUG [cli_invocation>compose] opened gitignore file: /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/homelab/server/frontend/.gitignore
11:17:24.092 DEBUG [cli_invocation>compose] built glob set; 0 literals, 3 basenames, 0 extensions, 0 prefixes, 0 suffixes, 0 required extensions, 0 regexes
11:17:24.092 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/homelab/server/frontend/dist: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/homelab/server/frontend/.gitignore"), original: "dist/", actual: "**/dist", is_whitelist: false, is_only_dir: true })))
11:17:24.093 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/homelab/server/frontend/node_modules: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/homelab/server/frontend/.gitignore"), original: "node_modules/", actual: "**/node_modules", is_whitelist: false, is_only_dir: true })))
11:17:24.094 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/schematic/target/.rustc_info.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.094 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/schematic/target/CACHEDIR.TAG: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.094 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/schematic/target/tmp: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.095 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/schematic/target/debug: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.095 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/schematic/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.096 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/schematic/schema/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.097 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/unchained-ai/target/.rustc_info.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.097 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/unchained-ai/target/CACHEDIR.TAG: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.097 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/unchained-ai/target/tmp: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.097 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/unchained-ai/target/debug: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.098 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/unchained-ai/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.099 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/.rustc_info.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.099 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/.rustdoc_fingerprint.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.099 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/rust-analyzer: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/CACHEDIR.TAG: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/release: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/criterion: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/flycheck0: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/doc: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/tmp: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.100 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/target/debug: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.101 DEBUG [cli_invocation>compose] opened gitignore file: /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/.gitignore
11:17:24.101 DEBUG [cli_invocation>compose] built glob set; 0 literals, 4 basenames, 0 extensions, 0 prefixes, 0 suffixes, 0 required extensions, 0 regexes
11:17:24.101 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/node_modules: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/.gitignore"), original: "node_modules", actual: "**/node_modules", is_whitelist: false, is_only_dir: false })))
11:17:24.101 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/bun.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/.gitignore"), original: "bun.lock", actual: "**/bun.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.101 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/.gitignore: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/.gitignore"), original: ".gitignore", actual: "**/.gitignore", is_whitelist: false, is_only_dir: false })))
11:17:24.101 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/package-lock.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: ".opencode/package-lock.json", actual: ".opencode/package-lock.json", is_whitelist: false, is_only_dir: false })))
11:17:24.101 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/package.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.opencode/.gitignore"), original: "package.json", actual: "**/package.json", is_whitelist: false, is_only_dir: false })))
11:17:24.101 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/node_modules: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "node_modules/", actual: "**/node_modules", is_whitelist: false, is_only_dir: true })))
11:17:24.103 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.111 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/tree-hugger/target/.rustc_info.json: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.111 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/tree-hugger/target/CACHEDIR.TAG: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.111 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/tree-hugger/target/tmp: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.111 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/tree-hugger/target/debug: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/target/*", actual: "**/target/*", is_whitelist: false, is_only_dir: false })))
11:17:24.111 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/tree-hugger/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.117 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/scripts/Cargo.lock: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "Cargo.lock", actual: "**/Cargo.lock", is_whitelist: false, is_only_dir: false })))
11:17:24.117 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lint-darkmatter-cli-timing.jsonl: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/lint-*timing.jsonl", actual: "**/lint-*timing.jsonl", is_whitelist: false, is_only_dir: false })))
11:17:24.119 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/test-darkmatter-cli-timing.jsonl: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/test-*timing.jsonl", actual: "**/test-*timing.jsonl", is_whitelist: false, is_only_dir: false })))
11:17:24.119 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/lint-darkmatter-timing.jsonl: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/lint-*timing.jsonl", actual: "**/lint-*timing.jsonl", is_whitelist: false, is_only_dir: false })))
11:17:24.121 DEBUG [cli_invocation>compose] ignoring /Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/darkmatter/test-darkmatter-timing.jsonl: Ignore(IgnoreMatch(Gitignore(Glob { from: Some("/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter/.gitignore"), original: "**/test-*timing.jsonl", actual: "**/test-*timing.jsonl", is_whitelist: false, is_only_dir: false })))
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.238 DEBUG [cli_invocation>compose] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.238 TRACE [cli_invocation>compose] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.238 TRACE [cli_invocation>compose] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.239 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.239 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: text replacements applied in darkmatter/lib/src/markdown/compose/mod.rs:1084
11:17:24.239 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.239 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.239 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running page blocks in darkmatter/lib/src/markdown/compose/mod.rs:1181
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] page_blocks: rendering in darkmatter/lib/src/markdown/compose/page_blocks/engine.rs:22
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: interpolations applied in darkmatter/lib/src/markdown/compose/mod.rs:1120
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] close (busy: 1.32ms, idle: 417ns) in darkmatter/lib/src/markdown/compose/mod.rs:455
11:17:24.240 INFO [cli_invocation>compose>compose_with] close (busy: 1.72ms, idle: 750ns) in darkmatter/lib/src/markdown/compose/mod.rs:401
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: text replacements applied in darkmatter/lib/src/markdown/compose/mod.rs:1084
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running page blocks in darkmatter/lib/src/markdown/compose/mod.rs:1181
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] page_blocks: rendering in darkmatter/lib/src/markdown/compose/page_blocks/engine.rs:22
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluating in darkmatter/lib/src/markdown/compose/conditions.rs:113
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] conditions: evaluated in darkmatter/lib/src/markdown/compose/conditions.rs:129
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.240 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.240 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: found expressions in darkmatter/lib/src/markdown/compose/expression/lexer.rs:155
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.240 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: evaluating expression in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:227
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] interpolation: resolved in darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs:235
11:17:24.241 DEBUG [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: interpolations applied in darkmatter/lib/src/markdown/compose/mod.rs:1120
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.241 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: starting transclusion phase in darkmatter/lib/src/markdown/compose/mod.rs:830
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: running operation in darkmatter/lib/src/markdown/compose/mod.rs:602
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 TRACE [cli_invocation>compose>compose_with>run_compose_pipeline_internal] compose: checking operation in darkmatter/lib/src/markdown/compose/mod.rs:597
11:17:24.241 INFO [cli_invocation>compose>compose_with>run_compose_pipeline_internal] close (busy: 701µs, idle: 333ns) in darkmatter/lib/src/markdown/compose/mod.rs:455
11:17:24.241 INFO [cli_invocation>compose>compose_with] close (busy: 716µs, idle: 376ns) in darkmatter/lib/src/markdown/compose/mod.rs:401
11:17:24.241 INFO [cli_invocation>compose>discover] close (busy: 390µs, idle: 333ns) in sniff/lib/src/filesystem/git/types.rs:511
^C
