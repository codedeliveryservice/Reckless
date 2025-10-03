# Reckless – Chess Engine in Rust

[![Build and Test](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml/badge.svg)](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml)

Guided by the insights from the chess programming community, it fearlessly
combines established concepts with its reckless nature, as the name suggests.

[uci]: https://en.wikipedia.org/wiki/Universal_Chess_Interface

## Rating

| Version                   | [CCRL Blitz][ccrl-404] | [CCRL 40/15][crrl-4015] | [CCRL FRC][ccrl-frc] | Release Date |
| ------------------------- | ---------------------- | ----------------------- | -------------------- | ------------ |
| [Reckless v0.8.0][v0.8.0] | 3765 +/- 14 [#2]       | 3609 +/- 11 [#3]        | 4077 +/- 13 [#3]     | Aug 29, 2025 |
| [Reckless v0.7.0][v0.7.0] | 3500 +/- 12 [#68]      | 3422 +/- 12 [#72]       |                      | Aug 23, 2024 |
| [Reckless v0.6.0][v0.6.0] | 3386 +/- 15 [#95]      | 3317 +/- 15 [#103]      |                      | Mar 22, 2024 |
| [Reckless v0.5.0][v0.5.0] | 3240 +/- 17 [#133]     | 3211 +/- 17 [#140]      |                      | Feb 4, 2024  |
| [Reckless v0.4.0][v0.4.0] | 2934 +/- 17 [#210]     | 2926 +/- 16 [#222]      |                      | Dec 13, 2023 |
| [Reckless v0.3.0][v0.3.0] | 2616 +/- 19 [#297]     | 2617 +/- 19 [#321]      |                      | Nov 6, 2023  |
| [Reckless v0.2.0][v0.2.0] | 2349 +/- 18 [#406]     |                         |                      | Oct 7, 2023  |
| [Reckless v0.1.0][v0.1.0] | 2005 +/- 18 [#539]     |                         |                      | May 16, 2023 |

[v0.1.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.1.0
[v0.2.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.2.0
[v0.3.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.3.0
[v0.4.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.4.0
[v0.5.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.5.0
[v0.6.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.6.0
[v0.7.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.7.0
[v0.8.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.8.0
[ccrl-404]: https://www.computerchess.org.uk/ccrl/404/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on
[crrl-4015]: https://www.computerchess.org.uk/ccrl/4040/cgi/compare_engines.cgi?class=Single-CPU+engines&only_best_in_class=on
[ccrl-frc]: https://www.computerchess.org.uk/ccrl/404FRC/

## Getting started

### Precompiled binaries

You can download precompiled builds from the [GitHub Releases page](https://github.com/codedeliveryservice/Reckless/releases).

- `-avx512`: Fastest, requires a recent CPU with AVX-512 support.
- `-avx2`: Fast, supported on most modern CPUs.
- `-generic`: Compatible with virtually all CPUs, but significantly slower than AVX2 or AVX512 builds.

> [!NOTE]
> If you're unsure which binary to use, try the AVX-512 build first. If it doesn't run on your system, fall back to the AVX2 build, or the generic one as a last resort.

[microarchitecture]: https://en.wikipedia.org/wiki/X86-64#Microarchitecture_levels

### Building from source

To build Reckless from source, make sure you have:

- `Rust 1.79.0` or a later version installed ([official Rust installation guide](https://www.rust-lang.org/tools/install))
- `Clang` installed (required for building the [Fathom](https://github.com/jdart1/Fathom) library used for Syzygy endgame tablebase support)

Once installed, you can build it with:

```bash
cargo rustc --release -- -C target-cpu=native
# ./target/release/reckless
```

#### PGO builds

For profile-guided optimization (PGO) builds, you need to install additional tools:

```bash
rustup component add llvm-tools
cargo install cargo-pgo
```

Then, you can build the engine using `make`:

```bash
make
# ./reckless
```

Or run the steps manually:

```bash
cargo pgo instrument
cargo pgo run -- bench
cargo pgo optimize
# ./target/x86_64-unknown-linux-gnu/release/reckless
# (the path may vary based on your system)
```

### Usage

Reckless is not a standalone chess program but a chess engine designed for use with UCI-compatible GUIs,
such as [Cute Chess](https://github.com/cutechess/cutechess), [En Croissant](https://encroissant.org),
or [Nibbler](https://github.com/rooklift/nibbler).

### Custom commands

Along with the standard UCI commands, Reckless supports additional commands for testing and debugging:

| Command         | Description                                                                        |
| --------------- | ---------------------------------------------------------------------------------- |
| `perft <depth>` | Run a [perft][perft] test to count the number of leaf nodes at a given depth       |
| `bench <depth>` | Run a [benchmark][bench] on a set of positions to measure the engine's performance |
| `eval`          | Print the static evaluation of the current position from white's perspective       |
| `compiler`      | Print the compiler version, target and flags used to compile the engine            |

[perft]: https://www.chessprogramming.org/Perft
[bench]: /src/tools/bench.rs

## Acknowledgements

- [OpenBench](https://github.com/AndyGrant/OpenBench) is the primary testing framework powered by [Cute Chess](https://github.com/cutechess/cutechess)
- [Bullet](https://github.com/jw1912/bullet) is the NNUE trainer
- [Stockfish](https://github.com/official-stockfish/Stockfish), [Ethereal](https://github.com/AndyGrant/Ethereal), [Berserk](https://github.com/jhonnold/berserk), and many other open source chess engines
- Members of the [Stockfish Discord server](https://discord.gg/GWDRS3kU6R)
- [Chess Programming Wiki](https://www.chessprogramming.org/Main_Page)
- [CCRL](https://www.computerchess.org.uk/ccrl/) and all chess engine testers out there
