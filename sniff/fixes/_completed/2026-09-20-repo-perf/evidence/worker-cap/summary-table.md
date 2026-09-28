## Criterion corpus (release), per-bracket medians r1,r2,r3 (ms)

| case | side | bracket medians | median | sample min-max |
|---|---|---|---:|---|
| detect_repo_structure | w12 | 32.23, 34.25, 35.00 | 34.25 | 31.85-38.75 |
| detect_repo_structure | w4 | 40.34, 38.20, 38.79 | 38.79 | 36.40-47.32 |
| production | w12 | 18.87, 23.09, 21.77 | 21.77 | 18.50-24.93 |
| production | w4 | 24.02, 25.84, 24.09 | 24.09 | 22.42-36.30 |
| serial_reference | w12 | 69.10, 64.54, 65.57 | 65.57 | 62.05-102.15 |
| serial_reference | w4 | 64.39, 65.33, 65.75 | 65.33 | 62.51-72.78 |

## Tiny-tree latency (median us) p1 / p2

| profile | case | w12 | w4 |
|---|---|---|---|
| debug | detect_repo_structure | 3,979.9 / 3,660.9 | 2,704.0 / 2,573.8 |
| debug | production | 3,442.0 / 3,244.8 | 2,146.8 / 2,126.8 |
| debug | serial_reference | 526.8 / 474.5 | 510.6 / 513.5 |
| release | detect_repo_structure | 3,337.9 / 3,090.0 | 1,991.3 / 2,118.2 |
| release | production | 2,760.6 / 2,682.5 | 1,557.4 / 1,552.3 |
| release | serial_reference | 284.2 / 268.9 | 277.7 / 298.7 |

## Concurrent (release) p1 / p2

| tree | threads x req | side | rps | median ms | p95 ms | user+sys s | max RSS MB | invol ctx |
|---|---|---|---:|---:|---:|---:|---:|---:|
| tiny | 1 x 400 | w12 | 305.3 / 333.3 | 3.13 / 3.03 | 4.79 / 4.34 | 0.75 / 0.49 | 14.7 / 14.7 | 12,160 / 11,699 |
| tiny | 1 x 400 | w4 | 497.8 / 499.7 | 1.89 / 1.88 | 3.10 / 3.09 | 0.41 / 0.41 | 14.1 / 14.2 | 3,890 / 3,883 |
| tiny | 14 x 400 | w12 | 4,325.7 / 4,338.3 | 3.06 / 3.08 | 4.00 / 4.15 | 13.54 / 13.35 | 23.0 / 22.8 | 193,880 / 190,870 |
| tiny | 14 x 400 | w4 | 4,607.8 / 4,618.3 | 2.87 / 2.88 | 4.04 / 4.03 | 13.65 / 13.74 | 19.0 / 19.1 | 72,729 / 73,070 |
| corpus | 1 x 20 | w12 | 29.8 / 30.4 | 32.79 / 32.67 | 36.82 / 34.16 | 4.62 / 4.63 | 37.0 / 33.7 | 9,642 / 5,572 |
| corpus | 1 x 20 | w4 | 27.6 / 28.1 | 35.85 / 34.87 | 38.92 / 38.52 | 2.19 / 2.15 | 32.7 / 32.1 | 608 / 509 |
| corpus | 14 x 10 | w12 | 53.0 / 54.0 | 247.11 / 250.67 | 427.43 / 380.60 | 38.47 / 37.87 | 200.8 / 170.1 | 67,585 / 65,504 |
| corpus | 14 x 10 | w4 | 55.0 / 54.4 | 245.56 / 252.58 | 314.66 / 332.65 | 37.18 / 38.76 | 157.2 / 129.8 | 55,235 / 49,641 |

## Load

- w12-debug-p1-tiny-latency.txt: start: 2026-09-26T19:30:43Z load: { 4.23 5.40 6.86 } | end: 2026-09-26T19:31:18Z load: { 4.04 5.22 6.73 }
- w12-debug-p2-tiny-latency.txt: start: 2026-09-26T19:33:18Z load: { 8.18 6.74 7.17 } | end: 2026-09-26T19:33:53Z load: { 9.08 7.22 7.33 }
- w12-release-p1-concurrent-corpus-t1.txt: start: 2026-09-26T19:31:20Z load: { 4.04 5.20 6.71 } | end: 2026-09-26T19:31:21Z load: { 4.04 5.20 6.71 }
- w12-release-p1-concurrent-corpus-t14.txt: start: 2026-09-26T19:31:21Z load: { 4.04 5.20 6.71 } | end: 2026-09-26T19:31:24Z load: { 13.32 7.10 7.38 }
- w12-release-p1-concurrent-tiny-t1.txt: start: 2026-09-26T19:31:18Z load: { 4.04 5.22 6.73 } | end: 2026-09-26T19:31:19Z load: { 4.04 5.20 6.71 }
- w12-release-p1-concurrent-tiny-t14.txt: start: 2026-09-26T19:31:19Z load: { 4.04 5.20 6.71 } | end: 2026-09-26T19:31:20Z load: { 4.04 5.20 6.71 }
- w12-release-p1-tiny-latency.txt: start: 2026-09-26T19:30:16Z load: { 3.95 5.47 6.93 } | end: 2026-09-26T19:30:43Z load: { 4.23 5.40 6.86 }
- w12-release-p2-concurrent-corpus-t1.txt: start: 2026-09-26T19:33:56Z load: { 8.75 7.18 7.32 } | end: 2026-09-26T19:33:56Z load: { 8.75 7.18 7.32 }
- w12-release-p2-concurrent-corpus-t14.txt: start: 2026-09-26T19:33:57Z load: { 8.75 7.18 7.32 } | end: 2026-09-26T19:33:59Z load: { 18.54 9.23 8.05 }
- w12-release-p2-concurrent-tiny-t1.txt: start: 2026-09-26T19:33:53Z load: { 9.08 7.22 7.33 } | end: 2026-09-26T19:33:54Z load: { 8.75 7.18 7.32 }
- w12-release-p2-concurrent-tiny-t14.txt: start: 2026-09-26T19:33:54Z load: { 8.75 7.18 7.32 } | end: 2026-09-26T19:33:56Z load: { 8.75 7.18 7.32 }
- w12-release-p2-tiny-latency.txt: start: 2026-09-26T19:32:52Z load: { 7.06 6.46 7.10 } | end: 2026-09-26T19:33:18Z load: { 8.18 6.74 7.17 }
- w4-debug-p1-tiny-latency.txt: start: 2026-09-26T19:31:40Z load: { 10.75 6.83 7.28 } | end: 2026-09-26T19:32:03Z load: { 8.39 6.55 7.16 }
- w4-debug-p2-tiny-latency.txt: start: 2026-09-26T19:32:25Z load: { 7.26 6.48 7.12 } | end: 2026-09-26T19:32:47Z load: { 6.28 6.30 7.04 }
- w4-release-p1-concurrent-corpus-t1.txt: start: 2026-09-26T19:32:05Z load: { 9.32 6.77 7.24 } | end: 2026-09-26T19:32:06Z load: { 9.32 6.77 7.24 }
- w4-release-p1-concurrent-corpus-t14.txt: start: 2026-09-26T19:32:06Z load: { 9.32 6.77 7.24 } | end: 2026-09-26T19:32:08Z load: { 9.32 6.77 7.24 }
- w4-release-p1-concurrent-tiny-t1.txt: start: 2026-09-26T19:32:03Z load: { 8.39 6.55 7.16 } | end: 2026-09-26T19:32:03Z load: { 8.39 6.55 7.16 }
- w4-release-p1-concurrent-tiny-t14.txt: start: 2026-09-26T19:32:03Z load: { 8.39 6.55 7.16 } | end: 2026-09-26T19:32:05Z load: { 9.32 6.77 7.24 }
- w4-release-p1-tiny-latency.txt: start: 2026-09-26T19:31:24Z load: { 13.32 7.10 7.38 } | end: 2026-09-26T19:31:40Z load: { 10.75 6.83 7.28 }
- w4-release-p2-concurrent-corpus-t1.txt: start: 2026-09-26T19:32:49Z load: { 7.06 6.46 7.10 } | end: 2026-09-26T19:32:50Z load: { 7.06 6.46 7.10 }
- w4-release-p2-concurrent-corpus-t14.txt: start: 2026-09-26T19:32:50Z load: { 7.06 6.46 7.10 } | end: 2026-09-26T19:32:52Z load: { 7.06 6.46 7.10 }
- w4-release-p2-concurrent-tiny-t1.txt: start: 2026-09-26T19:32:47Z load: { 6.28 6.30 7.04 } | end: 2026-09-26T19:32:48Z load: { 6.28 6.30 7.04 }
- w4-release-p2-concurrent-tiny-t14.txt: start: 2026-09-26T19:32:48Z load: { 6.28 6.30 7.04 } | end: 2026-09-26T19:32:49Z load: { 7.06 6.46 7.10 }
- w4-release-p2-tiny-latency.txt: start: 2026-09-26T19:32:08Z load: { 9.32 6.77 7.24 } | end: 2026-09-26T19:32:25Z load: { 7.26 6.48 7.12 }
- w12-release-r1.log: start: 2026-09-26T19:33:59Z load: { 18.54 9.23 8.05 } | end: 2026-09-26T19:34:54Z load: { 14.75 9.74 8.31 }
- w12-release-r2.log: start: 2026-09-26T19:36:42Z load: { 8.88 9.08 8.21 } | end: 2026-09-26T19:37:36Z load: { 8.23 8.86 8.18 }
- w12-release-r3.log: start: 2026-09-26T19:37:36Z load: { 8.23 8.86 8.18 } | end: 2026-09-26T19:38:31Z load: { 11.44 9.75 8.56 }
- w4-release-r1.log: start: 2026-09-26T19:34:54Z load: { 14.75 9.74 8.31 } | end: 2026-09-26T19:35:49Z load: { 10.51 9.31 8.23 }
- w4-release-r2.log: start: 2026-09-26T19:35:49Z load: { 10.51 9.31 8.23 } | end: 2026-09-26T19:36:42Z load: { 8.88 9.08 8.21 }
- w4-release-r3.log: start: 2026-09-26T19:38:31Z load: { 11.44 9.75 8.56 } | end: 2026-09-26T19:39:24Z load: { 9.08 9.32 8.47 }
