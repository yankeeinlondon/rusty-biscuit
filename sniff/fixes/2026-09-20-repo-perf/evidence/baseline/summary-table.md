| profile | bracket | case | samples | iters/sample | median ms | min ms | max ms | spread % |
|---|---|---|---:|---:|---:|---:|---:|---:|
| debug | r1 | detect_repo_structure | 20 | 3 | 241.04 | 236.57 | 246.69 | 4.2 |
| debug | r1 | production | 20 | 3 | 168.74 | 166.71 | 175.62 | 5.3 |
| debug | r1 | serial_reference | 20 | 3 | 168.59 | 166.48 | 176.92 | 6.2 |
| debug | r2 | detect_repo_structure | 20 | 3 | 239.09 | 235.45 | 242.19 | 2.8 |
| debug | r2 | production | 20 | 3 | 168.43 | 164.91 | 174.89 | 5.9 |
| debug | r2 | serial_reference | 20 | 3 | 168.83 | 166.56 | 175.11 | 5.1 |
| release | r1 | detect_repo_structure | 20 | 7 | 75.65 | 74.11 | 77.57 | 4.6 |
| release | r1 | production | 20 | 9 | 62.40 | 60.01 | 70.87 | 17.4 |
| release | r1 | serial_reference | 20 | 8 | 62.80 | 60.55 | 65.11 | 7.3 |
| release | r2 | detect_repo_structure | 20 | 7 | 78.61 | 74.32 | 81.62 | 9.3 |
| release | r2 | production | 20 | 8 | 62.75 | 59.87 | 65.39 | 8.8 |
| release | r2 | serial_reference | 20 | 8 | 63.50 | 60.66 | 64.96 | 6.8 |

Bracket-to-bracket drift of the median (r1 vs r2):

- debug `detect_repo_structure`: 241.04 → 239.09 ms (-0.8%)
- debug `production`: 168.74 → 168.43 ms (-0.2%)
- debug `serial_reference`: 168.59 → 168.83 ms (+0.1%)
- release `detect_repo_structure`: 75.65 → 78.61 ms (+3.9%)
- release `production`: 62.40 → 62.75 ms (+0.6%)
- release `serial_reference`: 62.80 → 63.50 ms (+1.1%)
