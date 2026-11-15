# License notice review (Phase 2, "Bundle license notices")

Generated 2026-11-09 on macOS arm64 from the working tree on
`feat/fuzz-infra-release` with cargo-about 0.8.4 (release asset SHA-256 verified,
see below), `about.toml`, `scripts/distribution/about.hbs`, and
`scripts/distribution/licenses.sh`. `Cargo.lock` was byte-identical before and
after every run, and a second `git-wt-cli` run produced identical bytes. The
outputs are `/tmp/dist-work/notices/<package>.md` (not checked in).

## Per package

Crates are third-party crates in cargo-about's graph (the four `TARGETS`,
default features, normal dependencies; first-party path crates are covered by
`LICENSE` and not listed). "Added" are crates `cargo tree -e normal` links that
cargo-about 0.8.4's graph omits; `native_notices.py` lists them with their
packaged license files. "Native" are vendored C libraries whose license text is
appended from the vendored sources.

| Package | Crates | Added | Native notices | Notices SHA-256 (prefix) |
| ------- | -----: | ----- | -------------- | ------------------------ |
| `biscuit-hash-cli` | 51 | none | none | `dbaa15dfc9d5099b` |
| `biscuit-file-cli` | 238 | none | none | `4752c21f9622c2a6` |
| `tree-hugger-cli` | 448 | none | PCRE2 (`pcre2-sys 0.2.10`), ICU (`tree-sitter 0.26.9`) | `e1efdf1bfacc508d` |
| `git-wt-cli` | 604 | `dlib 0.5.3`, `libloading 0.8.9`, `yeslogic-fontconfig-sys 6.0.1` | PCRE2 (`pcre2-sys 0.2.10`), AWS-LC (`aws-lc-sys 0.40.0`) | `1ccab78310ad32a0` |
| `sniff-detection-cli` | 632 | `dlib 0.5.3`, `libloading 0.8.9`, `yeslogic-fontconfig-sys 6.0.1` | PCRE2 (`pcre2-sys 0.2.10`), Oniguruma (`onig_sys 69.9.1`), AWS-LC (`aws-lc-sys 0.40.0`) | `dc0935b0cf85864b` |
| `darkmatter-cli` | 642 | `dlib 0.5.3`, `libloading 0.8.9`, `yeslogic-fontconfig-sys 6.0.1` | PCRE2 (`pcre2-sys 0.2.10`), Oniguruma (`onig_sys 69.9.1`), AWS-LC (`aws-lc-sys 0.40.0`) | `5cf695c840804af2` |
| `darkmatter-lsp` | 619 | `dlib 0.5.3`, `libloading 0.8.9`, `yeslogic-fontconfig-sys 6.0.1` | PCRE2 (`pcre2-sys 0.2.10`), Oniguruma (`onig_sys 69.9.1`), AWS-LC (`aws-lc-sys 0.40.0`) | `c56ef2ebd89227ff` |
| `claudine-cli` | 747 | `dlib 0.5.3`, `libloading 0.8.9`, `rustls-platform-verifier 0.7.0`, `yeslogic-fontconfig-sys 6.0.1` | PCRE2 (`pcre2-sys 0.2.10`), Oniguruma (`onig_sys 69.9.1`), AWS-LC (`aws-lc-sys 0.40.0`), BoringSSL-derived code in ring (`ring 0.17.14`) | `a6fd0ecd596bdf1b` |

License texts and crate license expressions:

- `biscuit-hash-cli`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `MIT`, `Unicode-3.0`; 11 distinct crate expressions: `(MIT OR Apache-2.0) AND Unicode-3.0`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-3-Clause`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `MIT`; `MIT OR Apache-2.0`.
- `biscuit-file-cli`: license texts for `Apache-2.0`, `BSD-3-Clause`, `MIT`, `Unicode-3.0`, `Zlib`; 17 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `MIT`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.
- `tree-hugger-cli`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `ISC`, `MIT`, `MPL-2.0`, `Unicode-3.0`, `Zlib`; 25 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-3-Clause`; `BSD-3-Clause OR Apache-2.0`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `ISC`; `MIT`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `MPL-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.
- `git-wt-cli`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `ISC`, `MIT`, `MPL-2.0`, `Unicode-3.0`, `Zlib`; 29 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR ISC OR MIT`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-3-Clause`; `BSD-3-Clause OR Apache-2.0`; `BSD-3-Clause OR MIT`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `ISC`; `ISC AND (Apache-2.0 OR ISC)`; `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`; `MIT`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `MPL-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.
- `sniff-detection-cli`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `ISC`, `MIT`, `MIT-0`, `MPL-2.0`, `Unicode-3.0`, `Zlib`; 30 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR ISC OR MIT`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-3-Clause`; `BSD-3-Clause OR Apache-2.0`; `BSD-3-Clause OR MIT`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `ISC`; `ISC AND (Apache-2.0 OR ISC)`; `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`; `MIT`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `MIT-0`; `MPL-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.
- `darkmatter-cli`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `ISC`, `MIT`, `MIT-0`, `MPL-2.0`, `Unicode-3.0`, `Zlib`; 30 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR ISC OR MIT`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-3-Clause`; `BSD-3-Clause OR Apache-2.0`; `BSD-3-Clause OR MIT`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `ISC`; `ISC AND (Apache-2.0 OR ISC)`; `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`; `MIT`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `MIT-0`; `MPL-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.
- `darkmatter-lsp`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `ISC`, `MIT`, `MIT-0`, `MPL-2.0`, `Unicode-3.0`, `Zlib`; 30 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR ISC OR MIT`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-3-Clause`; `BSD-3-Clause OR Apache-2.0`; `BSD-3-Clause OR MIT`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `ISC`; `ISC AND (Apache-2.0 OR ISC)`; `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`; `MIT`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `MIT-0`; `MPL-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.
- `claudine-cli`: license texts for `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `BSL-1.0`, `ISC`, `MIT`, `MIT-0`, `MPL-2.0`, `Unicode-3.0`, `Zlib`; 33 distinct crate expressions: `(Apache-2.0 OR MIT) AND BSD-3-Clause`; `(MIT OR Apache-2.0) AND Unicode-3.0`; `0BSD OR MIT OR Apache-2.0`; `Apache-2.0`; `Apache-2.0 AND ISC`; `Apache-2.0 OR BSL-1.0`; `Apache-2.0 OR ISC OR MIT`; `Apache-2.0 OR MIT`; `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT`; `BSD-2-Clause`; `BSD-2-Clause OR Apache-2.0 OR MIT`; `BSD-2-Clause OR MIT OR Apache-2.0`; `BSD-3-Clause`; `BSD-3-Clause OR Apache-2.0`; `BSD-3-Clause OR MIT`; `BSL-1.0`; `CC0-1.0 OR Apache-2.0`; `CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception`; `CC0-1.0 OR MIT-0 OR Apache-2.0`; `ISC`; `ISC AND (Apache-2.0 OR ISC)`; `ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`; `MIT`; `MIT AND BSD-3-Clause`; `MIT OR Apache-2.0`; `MIT OR Apache-2.0 OR Zlib`; `MIT OR Zlib OR Apache-2.0`; `MIT-0`; `MPL-2.0`; `Unicode-3.0`; `Unlicense OR MIT`; `Zlib`; `Zlib OR Apache-2.0 OR MIT`.

## Native code review

| Crate | Native code | Notice |
| ----- | ----------- | ------ |
| `pcre2-sys 0.2.10` | PCRE2 10.46 and sljit (JIT on all four targets), static with `PCRE2_SYS_STATIC=1` | PCRE2 terms from `upstream/src/pcre2_compile.c` and sljit terms from `upstream/deps/sljit/sljit_src/sljitLir.c`; the crate does not package upstream `LICENCE` |
| `onig_sys 69.9.1` | Oniguruma, compiled statically by default | `oniguruma/COPYING` (BSD-2-Clause, K.Kosako) |
| `tree-sitter 0.26.9` | runtime C plus ICU `utf8.h`/`utf16.h`/`umachine.h` | `src/unicode/LICENSE` (ICU/Unicode) |
| `aws-lc-sys 0.40.0` | AWS-LC (BoringSSL/OpenSSL-derived, fiat, s2n-bignum, ...) | crate-root `LICENSE` (cargo-about otherwise falls back to SPDX texts without these copyrights) |
| `ring 0.17.14` (claudine-cli only) | BoringSSL-derived C/assembly | `LICENSE-BoringSSL` (cargo-about uses only `LICENSE-other-bits` and an Apache-2.0 text) |
| `libsqlite3-sys 0.28.0` (`bundled`) | SQLite amalgamation | none: public domain |
| `alsa-sys`, `coreaudio-sys`, `yeslogic-fontconfig-sys` (`dlopen`) | system libraries/frameworks, nothing vendored | none |
| `blake3`, `rav1e`, `av-scenechange`, `signal-hook`, `objc2-exception-helper`, `mac-notification-sys`, `tree-sitter-*` grammars | the crate's own C/assembly/Objective-C or generated parsers | covered by crate metadata |
| `aws-lc-rs`, `rayon-core`, `tree-sitter-language`, `wasm-bindgen-shared` | `links` key only | none |

`native_notices.py` fails packaging for any crate that declares `links` or
builds with `cc`/`cmake` and is in neither table.

## Clarifications and workarounds

- `[content-policy.clarify]`: `content-policy` (first-party, in the
  `sniff-detection-cli`, `darkmatter-cli`, `darkmatter-lsp`, and `claudine-cli`
  graphs) has no `license` field, so cargo-about cannot synthesize an
  expression. The clarification states `AGPL-3.0-only` against the root
  `LICENSE` (SHA-256 `0d96a4ff...abcb0`). Adding `license = "AGPL-3.0-only"` to
  `content-policy/lib/Cargo.toml` (and `content-policy/cli`) would make it
  unnecessary.
- No built-in `workarounds`: the `ring` workaround produced byte-identical
  output for ring 0.17.14, so it is not enabled.
- `no-clearly-defined = true` keeps the output reproducible; crates that package
  no license file get cargo-about's standard SPDX text (marked "(standard
  text)"), without the crate's copyright line.

## cargo-about installation

`cargo binstall cargo-about@0.8.4 --root /tmp/cargo-about --no-confirm
--disable-telemetry --no-track --strategies crate-meta-data --targets
aarch64-apple-darwin` fetched the GitHub release asset. The asset
`cargo-about-0.8.4-aarch64-apple-darwin.tar.gz` matched its published `.sha256`
(`d5255ead3ac861a11c785bf19d4f70f16e59ac8b9519c312da61213d3d57351d`), and the
installed binary is byte-identical to the one in that archive (SHA-256
`29f8089e60d783f2d2521a31e4aed4880996e0313ae5118b87a95c2fbadcc008`).
`cargo-about --version` reports `cargo-about 0.8.4`.
