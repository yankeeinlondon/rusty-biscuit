# Phase 5 context-capture baseline

Before the split, context capture was implemented in
`lib/src/markdown/compose/context/capture.rs`. It contained nine groups in the stable order
DateTime, Repo, FileChanges, Languages, Documents, Os, Hardware, Gpu, Agent. Demand scanning
recognized `ctx.KEY` references and delegated key selection to one central `for_key` match.

`ContextCapture::new` discovered the repository before document discovery and overlapped the
independent file-change, OS, hardware, and GPU probes. Probe diagnostics and timing labels were
`repo`, `file-changes`, `documents`, `os`, `hardware`, and `gpu`. Local datetime population was
always added by content-driven capture. The existing capture module exposed 15 unit tests in the
nextest inventory before the move.

The existing aliases are the date/time aliases populated by `populate_datetime_aliases` and the
repo-area compatibility names (`area`, `area_description`, and `area_root`). The known defect is
that a GPU-only capture ran the `gpu` probe but never inserted its result because insertion lived
inside hardware population.
