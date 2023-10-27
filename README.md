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
-   Iterative Deepening
-   Aspiration Windows
-   Transposition Table
-   Quiescence Search

### Selectivity

-   Check Extensions
-   Reverse Futility Pruning
-   Null Move Pruning
-   Late Move Reductions

### Move ordering

-   Hash Move
-   MVV-LVA
-   Killer Heuristic
-   History Heuristic

### Static evaluation

-   Tapered Evaluation
-   King-Relative Piece-Square Tables
-   Sliding Piece Mobility
-   Tempo Bonus

## Getting started

### Precompiled binaries

You can download precompiled builds from the [GitHub Releases page](https://github.com/codedeliveryservice/Reckless/releases).

### Building from source

To build the engine from source, make sure you have `Rust 1.65` or a later version installed.
If you don't have Rust, follow the [official Rust installation guide](https://www.rust-lang.org/tools/install).

Then, compile the engine using `Cargo`:

```bash
cargo build --release
```

### Usage

Reckless is not a standalone chess program but a chess engine designed for use with UCI-compatible GUIs,
such as [Cute Chess](https://github.com/cutechess/cutechess) or [ChessBase](https://www.chessbase.com/).

Alternatively, you can communicate with the engine directly using the [UCI protocol](https://backscattering.de/chess/uci).

## Contributing

Contributions are welcome! If you encounter issues or have suggestions for improvements,
please open an issue or submit a pull request.

## License

This project is licensed with the [MIT license](LICENSE).
