| profile | case | side | bracket | samples | iters/sample | median ms | min ms | max ms | spread % |
|---|---|---|---|---:|---:|---:|---:|---:|---:|
| debug | detect_repo_structure | baseline | r1 | 20 | 2 | 243.66 | 240.03 | 260.01 | 8.2 |
| debug | detect_repo_structure | baseline | r2 | 20 | 3 | 252.14 | 241.40 | 261.58 | 8.0 |
| debug | detect_repo_structure | baseline | r3 | 20 | 3 | 238.88 | 236.20 | 242.09 | 2.5 |
| debug | detect_repo_structure | after | r1 | 20 | 4 | 109.26 | 105.32 | 169.41 | 58.7 |
| debug | detect_repo_structure | after | r2 | 20 | 5 | 106.91 | 103.41 | 116.27 | 12.0 |
| debug | detect_repo_structure | after | r3 | 20 | 3 | 106.51 | 102.68 | 188.47 | 80.5 |
| debug | production | baseline | r1 | 20 | 2 | 182.67 | 176.50 | 238.65 | 34.0 |
| debug | production | baseline | r2 | 20 | 3 | 172.07 | 170.45 | 175.32 | 2.8 |
| debug | production | baseline | r3 | 20 | 3 | 169.11 | 166.90 | 174.92 | 4.7 |
| debug | production | after | r1 | 20 | 11 | 53.00 | 43.82 | 74.40 | 57.7 |
| debug | production | after | r2 | 20 | 14 | 42.14 | 38.20 | 53.48 | 36.3 |
| debug | production | after | r3 | 20 | 8 | 64.61 | 53.90 | 76.28 | 34.6 |
| debug | serial_reference | baseline | r1 | 20 | 3 | 255.26 | 178.11 | 385.38 | 81.2 |
| debug | serial_reference | baseline | r2 | 20 | 3 | 173.26 | 170.71 | 188.24 | 10.1 |
| debug | serial_reference | baseline | r3 | 20 | 3 | 174.12 | 168.03 | 182.76 | 8.5 |
| debug | serial_reference | after | r1 | 20 | 3 | 173.50 | 169.11 | 185.57 | 9.5 |
| debug | serial_reference | after | r2 | 20 | 3 | 173.83 | 167.39 | 187.37 | 11.5 |
| debug | serial_reference | after | r3 | 20 | 3 | 177.55 | 171.06 | 191.11 | 11.3 |
| release | detect_repo_structure | baseline | r1 | 20 | 6 | 80.98 | 78.35 | 85.13 | 8.4 |
| release | detect_repo_structure | baseline | r2 | 20 | 7 | 79.40 | 76.81 | 86.11 | 11.7 |
| release | detect_repo_structure | baseline | r3 | 20 | 7 | 78.11 | 77.04 | 81.99 | 6.3 |
| release | detect_repo_structure | after | r1 | 20 | 13 | 38.92 | 35.79 | 59.31 | 60.4 |
| release | detect_repo_structure | after | r2 | 20 | 15 | 33.30 | 32.75 | 34.40 | 5.0 |
| release | detect_repo_structure | after | r3 | 20 | 15 | 35.71 | 34.71 | 37.60 | 8.1 |
| release | production | baseline | r1 | 20 | 4 | 108.22 | 77.51 | 166.20 | 82.0 |
| release | production | baseline | r2 | 20 | 8 | 65.18 | 63.51 | 67.93 | 6.8 |
| release | production | baseline | r3 | 20 | 8 | 65.80 | 64.01 | 73.21 | 14.0 |
| release | production | after | r1 | 20 | 21 | 22.81 | 21.07 | 27.94 | 30.1 |
| release | production | after | r2 | 20 | 26 | 20.54 | 19.59 | 21.30 | 8.3 |
| release | production | after | r3 | 20 | 23 | 22.57 | 22.21 | 23.39 | 5.2 |
| release | serial_reference | baseline | r1 | 20 | 8 | 66.95 | 63.96 | 72.61 | 12.9 |
| release | serial_reference | baseline | r2 | 20 | 8 | 67.61 | 64.47 | 73.36 | 13.2 |
| release | serial_reference | baseline | r3 | 20 | 8 | 64.82 | 63.41 | 67.50 | 6.3 |
| release | serial_reference | after | r1 | 20 | 7 | 66.53 | 65.59 | 81.09 | 23.3 |
| release | serial_reference | after | r2 | 20 | 8 | 68.52 | 64.33 | 905.08 | 1226.9 |
| release | serial_reference | after | r3 | 20 | 8 | 64.67 | 63.83 | 66.85 | 4.7 |

| profile | case | side | bracket medians ms | median of medians ms | bracket spread % |
|---|---|---|---|---:|---:|
| debug | detect_repo_structure | baseline | 243.66, 252.14, 238.88 | 243.66 | 5.4 |
| debug | detect_repo_structure | after | 109.26, 106.91, 106.51 | 106.91 | 2.6 |
| debug | production | baseline | 182.67, 172.07, 169.11 | 172.07 | 7.9 |
| debug | production | after | 53.00, 42.14, 64.61 | 53.00 | 42.4 |
| debug | serial_reference | baseline | 255.26, 173.26, 174.12 | 174.12 | 47.1 |
| debug | serial_reference | after | 173.50, 173.83, 177.55 | 173.83 | 2.3 |
| release | detect_repo_structure | baseline | 80.98, 79.40, 78.11 | 79.40 | 3.6 |
| release | detect_repo_structure | after | 38.92, 33.30, 35.71 | 35.71 | 15.7 |
| release | production | baseline | 108.22, 65.18, 65.80 | 65.80 | 65.4 |
| release | production | after | 22.81, 20.54, 22.57 | 22.57 | 10.1 |
| release | serial_reference | baseline | 66.95, 67.61, 64.82 | 66.95 | 4.2 |
| release | serial_reference | after | 66.53, 68.52, 64.67 | 66.53 | 5.8 |

