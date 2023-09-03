# Reckless – Bitboard Chess Engine

[![Build and Test](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml/badge.svg)](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml)

Reckless is a lightweight [UCI][uci] chess engine developed as a pet project to explore the Rust
programming language. While not designed to be a competitive engine, it provides a playable chess
engine with a reasonable strength. Please note that it is a learning experiment, and as such,
it may not follow the best practices or conventions typically found in high-quality engines.

[uci]: https://en.wikipedia.org/wiki/Universal_Chess_Interface

## Board representation

-   Bitboards with Little-Endian Rank-File Mapping
-   Magic Bitboards for sliding pieces
-   Copy-Make approach

## Move generation

-   Pseudo legal move generator
-   Pre-calculated attack maps with [Fancy Magic Bitboards][fancy-bitboards]
-   Magic numbers are pre-generated using [Reckless Magics][reckless-magics]
-   Compile time generation of move maps using a [build script](/src/lookup/build.rs)

[fancy-bitboards]: https://www.chessprogramming.org/Magic_Bitboards#Fancy
[reckless-magics]: https://github.com/codedeliveryservice/RecklessMagics

## Search

-   Iterative Deepening
-   Aspiration Windows
-   Principle Variation Search
-   Transposition Table
-   Quiescence Search

## Selectivity

-   Check Extensions
-   Null Move Pruning
-   Late Move Reduction

## Move ordering

-   Hash Move
-   MVV-LVA
-   Killer Heuristic
-   History Heuristic

## Static evaluation

-   Tapered Evaluation
-   Material
-   Mobility
-   Piece-Square Tables

## Getting started

### Precompiled binaries

You can download precompiled builds from the [GitHub Releases page][releases].

[releases]: https://github.com/codedeliveryservice/Reckless/releases

### Building from source

To build the engine from source, make sure you have `Rust 1.64` or a later version installed.
If you don't have Rust, follow the [official Rust installation guide][rust-guide].

[rust-guide]: https://www.rust-lang.org/tools/install

### Usage

Once you have the engine compiled or downloaded, you can use it with a UCI-compatible chess GUI.
Some popular options include:

-   ChessBase (Paid)
-   [Cute Chess][cute-chess] (Free)
-   [Arena Chess GUI][arena-chess] (Free)

[cute-chess]: https://github.com/cutechess/cutechess
[arena-chess]: http://www.playwitharena.de/

## Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements,
please open an issue or submit a pull request.

## License

This project is licensed with the [MIT license](LICENSE).
