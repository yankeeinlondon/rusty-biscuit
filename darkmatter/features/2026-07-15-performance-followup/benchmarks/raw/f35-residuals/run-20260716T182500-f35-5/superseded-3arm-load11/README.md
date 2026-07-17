# Superseded 3-arm CLI attempt (load ~11-13)

An earlier candidate_A / baseline(S1) / candidate_B bracket at 40 runs. It passed
its drift gate only for the `large_fm_detailed` target (drift 8.1%, delta
-24.8%); both controls failed it (drift 14.8% vs delta -4.6% on
`small_detailed`), so the run could not certify that no control regressed.

Superseded by the accepted 4-arm run in the parent directory (100 runs, load
~5.9-6.3, drift 3.4-4.7%), which additionally adds the S0 (pre-F35.5) baseline
and therefore measures the whole sub-item rather than only its second commit.

Only `cli-baseline.json` (the S1 arm) is retained here; that run's candidate arms
were overwritten by the accepted run. Retained for provenance, not quoted.
