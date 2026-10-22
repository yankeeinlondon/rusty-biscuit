# Windows leg of the cross-OS PNG difference (R-S9, AC-S8)

Run 2026-10-21. It closes the Windows gap that `FINDINGS.md` ("Cross-OS PNG difference") left open.
It covers one diagram, `upstream_cypress_sequencediagram_spec_example_001`, in both themes. All three hosts built the same spike source tree in this run.

## Hosts

| Label | Host | Fonts the Merman column drew with |
|---|---|---|
| macOS | macOS 27.2, Apple M4 Max, 16 threads (local) | `"trebuchet ms" -> Arial`, 29 labels |
| Linux | `$BUILD_LINUX`: Debian GNU/Linux 13 (trixie), AMD EPYC 9175F, 16 threads | `"trebuchet ms" -> DejaVu Sans`, 29 labels |
| Windows | `$BUILD_WIN`: Microsoft Windows 11 Pro 10.0.26300, AMD EPYC 9175F, 12 threads | `"trebuchet ms" -> Arial`, 29 labels |

The binary's `host_description()` has no Windows branch, so it printed `windows, unknown CPU, 12 threads`. The OS version and CPU above come from `Get-CimInstance Win32_OperatingSystem` and `Win32_Processor`.

**Fonts resolved on Windows.** On Windows, fontique chose Arial as the sans-serif fallback, Times New Roman as the serif, and Consolas as the monospace. Arial, Trebuchet MS, and Verdana were all found through the `stack` step, the same step that found them on macOS. The Merman column's survey reported 29 labels, 0 empty texts, and 0 clipped. Its PNGs match the macOS PNGs pixel for pixel (table below), so the text was drawn and is not blank.

## Commands

- **macOS:** from the spike directory, run `CARGO_TARGET_DIR=/tmp/s1-target cargo build --release`, then `/tmp/s1-target/release/s1-comparison --os macos --out /tmp/s1-win-leg/mac --iterations 0 --only upstream_cypress_sequencediagram_spec_example_001`.
- **Shipped files.** One tar holds `biscuit-visualized/src`, `biscuit-hash/lib`, and the spike directory without `gallery/`, at their repository-relative paths. The spike reads `corpus/` at run time through `CARGO_MANIFEST_DIR`, so the corpus must travel with it. Nothing else in the path dependencies is needed: `biscuit-visualized`'s path dev-dependency is not resolved for a non-root package.
- **Windows:** the tar was extracted to the scratch directory `B:\coding\s1-winleg` with `System32\tar.exe`. A `-File` PowerShell script then ran the following from the spike directory, with `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` set to empty:
    - `cargo build --release`. The target is the default `target\` inside the scratch copy, on `B:`, with no `CARGO_TARGET_DIR` override. It took 3 min 01 s.
    - `.\target\release\s1-comparison.exe --os windows --out B:\coding\s1-winleg\out --iterations 0 --only upstream_cypress_sequencediagram_spec_example_001`
- **Linux:** the tar was extracted to `~/scratch/s1-linleg`. The run used `CARGO_TARGET_DIR=~/scratch/s1-linleg-target` and empty wrappers, then the same command with `--os linux`. The build took 57 s.
- **Fetch and cleanup.** `out/` was fetched back from each host to `/tmp/s1-win-leg/{mac,linux,win}`. Both remote scratch directories, including their target directories, were deleted afterward.
- **Comparison.** The metric is the one `scripts/cross_os_diff.py` uses: ImageMagick `magick compare -metric AE` (fuzz 0) divided by the pixel count, and the normalized value of `-metric RMSE`. Only same-size pairs are compared. The script was run pairwise over the three hosts.

## Results: Merman column (`merman`)

| Pair | Theme | AE share (pixels differing) | Normalized RMSE | Size (all three hosts) |
|---|---|---|---|---|
| macOS–Windows | dark | 0.000% (0 px) | 0.0000 | 1000×1298 |
| Linux–Windows | dark | 0.268% (3475 px) | 0.0365 | 1000×1298 |
| macOS–Linux | dark | 0.268% (3475 px) | 0.0365 | 1000×1298 |
| macOS–Windows | light | 0.000% (0 px) | 0.0000 | 1000×1298 |
| Linux–Windows | light | 0.735% (9537 px) | 0.0661 | 1000×1298 |
| macOS–Linux | light | 0.735% (9537 px) | 0.0661 | 1000×1298 |

- **The SVGs are identical on all three hosts.** Merman's SVG output is byte-identical everywhere, so every pixel difference comes from rasterization.
- **macOS and Windows produce identical PNGs.** Both draw the labels with Arial through the same resvg 0.45 path.
- **The whole Linux difference is font substitution.** Linux has no Arial and draws the labels with DejaVu Sans. That substitution, not rasterizer noise between OSes, is the entire Linux difference. The macOS–Linux figures match the earlier `FINDINGS.md` pass (0.27% / 0.037 dark, 0.73% / 0.066 light).

## Other columns (for context)

- **`merman-real` and `merman-native`, macOS–Windows:** the same size (1000×1249 and 1000×1254). Dark differs in 4 px (RMSE 0.0004), light in 11 px (RMSE 0.0008). The two hosts use different Arial font files.
- **The same columns against Linux:** the size differs (1182 and 1215 high), because they lay text out with real font metrics.
- **`current`:** the size differs in every pair except macOS–Windows light, which is identical at 1000×1425. Dark is 1270 high on macOS, 1366 on Linux, and 1141 on Windows.

## Traps hit

- **Native stderr written through `*>` is wrapped as an error record.** On Windows PowerShell, `*>` redirection wraps each line of a native program's stderr in an error record (`s1-comparison.exe : … + CategoryInfo … RemoteException`). The record is also hard-wrapped at the console width, which split the single-line `font chain:` report. Read structured values from `results-windows.json` instead.
- **The spike's `host_description()` has no Windows branch** (`sysctl` and `/proc/cpuinfo` only).
- **scp needs the drive in the remote path.** It reached the `B:` volume with the spelling `"$BUILD_WIN":/B:/coding/...`.
