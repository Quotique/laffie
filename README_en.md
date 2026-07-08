**English** | [Русский](README.md)

## Build

A Rust toolchain is required. Installation instructions are on the official site:
<https://www.rust-lang.org/tools/install>.

Once installed, build and run the system with:

```bash
cargo run --release --bin cli -- -c config/cli.yaml
```

A release build (the `--release` flag) is strongly recommended — debug builds are noticeably
slower.

## Documentation

English documentation is being prepared. For now, please refer to the
[Russian version](README.md), which is the source of truth.

## Paper

The architecture of the system is described in the paper (in Russian):

A. P. Annenkov. **Architecture of a Simplified Automated Reasoning System Based on
A. S. Podkolzin's Approach** // Intelligent Systems. Theory and Applications. — 2026. — Vol. 30,
no. 1. — <https://new.intsysmagazine.ru/issues/2026/1/article/9>.

Published under the [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) license.
