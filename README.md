# Reckless – Bitboard Chess Engine

[![Build and Test](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml/badge.svg)](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml)

Reckless is a [UCI][uci] chess engine written in Rust as a personal project.

Guided by the insights from the chess programming community, it fearlessly
combines established concepts, as the name suggests.

[uci]: https://en.wikipedia.org/wiki/Universal_Chess_Interface

## Features

### Board representation

-   Bitboards with Little-Endian Rank-File Mapping
-   Magic Bitboards for sliding piece attacks
-   Copy-Make approach

### Move generation

-   Pseudo legal move generator
-   Pre-calculated attack maps using [Fancy Magic Bitboards](https://www.chessprogramming.org/Magic_Bitboards#Fancy)
-   Magic numbers are pre-generated using [Reckless Magics](https://github.com/codedeliveryservice/RecklessMagics)
-   Compile time generation of move maps using a [build script](/src/lookup/build.rs)

### Search

-   Fail-Soft Alpha-Beta
-   Principle Variation Search
-   Quiescence Search
-   Iterative Deepening
-   Aspiration Windows
-   Lockless Transposition Table
-   Lazy SMP (Shared-Memory Parallel)

### Selectivity

#### Pruning

-   Reverse Futility Pruning
-   Null Move Pruning
-   Razoring
-   Futility Pruning
-   Late Move Pruning
-   Delta Pruning

#### Reductions

-   Fractional Late Move Reductions
-   Internal Iterative Reductions

#### Extensions

-   Check Extensions

### Move ordering

-   Hash Move
-   MVV-LVA
-   Killer Move Heuristic
-   History Heuristic
    -   Butterfly History
    -   Counter Move History
    -   Follow-Up History

### Evaluation

-   [NNUE](https://www.chessprogramming.org/NNUE)
-   Architecture: `(768 -> 128)x2 -> 1`
-   Activation Function: `SCReLU` (Squared Clipped Rectified Linear Unit)
-   Quantization: `i16` (`256`/`64`)
-   Trained on original data generated entirely through self-play
-   Handwritten SIMD for AVX2 instructions

## Rating

| Version                   | [CCRL Blitz][ccrl-404] | [CCRL 40/15][crrl-4015] | Release Date |
| ------------------------- | ---------------------- | ----------------------- | ------------ |
| [Reckless v0.5.0][v0.5.0] | 3243 +/- 19 [#94]      | 3213 +/- 21 [#94]       | Feb 4, 2024  |
| [Reckless v0.4.0][v0.4.0] | 2933 +/- 19 [#151]     | 2929 +/- 21 [#158]      | Dec 13, 2023 |
| [Reckless v0.3.0][v0.3.0] | 2617 +/- 20 [#229]     | 2615 +/- 21 [#251]      | Nov 6, 2023  |
| [Reckless v0.2.0][v0.2.0] | 2358 +/- 19 [#333]     |                         | Oct 7, 2023  |
| [Reckless v0.1.0][v0.1.0] | 2020 +/- 20 [#471]     |                         | May 16, 2023 |

[v0.1.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.1.0
[v0.2.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.2.0
[v0.3.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.3.0
[v0.4.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.4.0
[v0.5.0]: https://github.com/codedeliveryservice/Reckless/releases/tag/v0.5.0
[ccrl-404]: https://www.computerchess.org.uk/ccrl/404/
[crrl-4015]: https://www.computerchess.org.uk/ccrl/4040/

## Getting started

### Precompiled binaries

You can download precompiled builds from the [GitHub Releases page](https://github.com/codedeliveryservice/Reckless/releases).

-   `x86_64-v1`: Slowest, compatible with any x86-64 CPU.
-   `x86_64-v2`: Faster, requires support for `POPCNT`, `SEE3`, etc.
-   `x86_64-v3`: Even faster, requires support for `AVX2`, etc.
-   `x86_64-v4`: Fastest, requires support for `AVX512`.

For detailed information on the specific features needed for each level, refer to the [x86-64 microarchitecture levels][microarchitecture] Wikipedia page.

[microarchitecture]: https://en.wikipedia.org/wiki/X86-64#Microarchitecture_levels

### Building from source

To build the engine from source, make sure you have `Rust 1.65` or a later version installed.
If you don't have Rust, follow the [official Rust installation guide](https://www.rust-lang.org/tools/install).

Then, compile the engine using `Cargo`:

```bash
cargo rustc --release -- -C target-cpu=native
```

Alternatively, you can use the provided `Makefile`:

```bash
# Build for the current CPU architecture
make
# Build release binaries for all microarchitecture levels
make release
```

### Usage

Reckless is not a standalone chess program but a chess engine designed for use with UCI-compatible GUIs,
such as [Cute Chess](https://github.com/cutechess/cutechess) or [ChessBase](https://www.chessbase.com/).

Alternatively, you can communicate with the engine directly using the [UCI protocol](https://backscattering.de/chess/uci).

### Custom commands

Along with the standard UCI commands, Reckless supports additional commands for testing and debugging:

| Command         | Description                                                                         |
| --------------- | ----------------------------------------------------------------------------------- |
| `perft <depth>` | Run a [perft][perft] test to count the number of leaf nodes at a given depth.       |
| `bench <depth>` | Run a [benchmark][bench] on a set of positions to measure the engine's performance. |
| `eval`          | Print the static evaluation of the current position from white's perspective.       |

[perft]: https://www.chessprogramming.org/Perft
[bench]: /src/tools/bench.rs

## Acknowledgements

-   [OpenBench](https://github.com/AndyGrant/OpenBench) is the primary testing framework powered by [Cute Chess](https://github.com/cutechess/cutechess).
-   Open source chess engines, like [Stockfish](https://github.com/official-stockfish/Stockfish), [Ethereal](https://github.com/AndyGrant/Ethereal), [Berserk](https://github.com/jhonnold/berserk), and numerous others, for serving as inspiration and providing ideas that fuel development.
-   [Stockfish Discord server](https://discord.gg/GWDRS3kU6R) for providing relevant insights and feedback.
-   [Chess Programming Wiki](https://www.chessprogramming.org/Main_Page) for contributing to the project's foundation.
-   Many thanks to the [CCRL](https://www.computerchess.org.uk/ccrl/) team and all chess engine testers for their valuable contributions.

## License

This project is licensed with the [MIT license](LICENSE).
