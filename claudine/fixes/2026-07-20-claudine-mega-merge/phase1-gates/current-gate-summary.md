# Current Execution-Seed Gates

Revision: `72a5843af470ba75c1ae6f6e1ccf16ba10a427eb`

| Command | Result | Assertions reached | Classification |
|---|---|---|---|
| `cd claudine && just test` | Exit 130 after bounded SIGINT | Catalog-types 21/21 passed; Claudine 3,423/3,423 passed on every attempt; contract 47/47 passed after cache warm-up; CLI never reached assertions because native dependencies were still compiling at the ceiling | timed-out / blocked by non-interactive 60-second ceiling |
| `cd claudine && just lint` | Exit 130 after bounded SIGINT | Error-transport and lifecycle-doc-facets guards passed; catalog-types, Claudine library, and contract lint completed; CLI lint was still compiling native dependencies | timed-out / blocked by non-interactive 60-second ceiling |

No assertion or lint diagnostic failed. One Claudine test,
`composition::interpolation_conformance::loop_and_lifecycle_agree_on_shared_syntax`,
was retry-classified as flaky and passed on attempt 2 of 4 in one run. This is
not a complete area-gate pass because the CLI portions did not reach their
assertions/checks within the command ceiling.
