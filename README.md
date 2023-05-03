# Reckless – Bitboard Chess Engine

[![Build and Test](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml/badge.svg)](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml)

An attempt to create a simple yet playable [UCI][uci] chess engine.

Please note that the very idea of the project is mostly educational for exploring the Rust programming language.

[uci]: https://en.wikipedia.org/wiki/Universal_Chess_Interface

## Board representation

-   Bitboards with Little-Endian Rank-File Mapping
-   Magic Bitboards for sliding pieces
-   Make/undo approach

## Move generation

-   Pseudo legal move generator
-   Pre-calculated attack maps with [Fancy Magic Bitboards][fancy-bitboards]
-   Magic numbers are pre-generated using [Reckless Magics][reckless-magics]
-   Compile time generation of move maps using a [build script](/game/src/lookup/build.rs)

[fancy-bitboards]: https://www.chessprogramming.org/Magic_Bitboards#Fancy
[reckless-magics]: https://github.com/codedeliveryservice/RecklessMagics

## Search

-   Iterative Deepening
-   Aspiration Windows
-   Principle Variation Search
-   Transposition Table
-   Quiescence Search

## Pruning techniques

-   Null Move Pruning

## Move ordering

-   TT-move ordering
-   MVV-LVA
-   Killer Heuristic
-   History Heuristic

## Static evaluation

-   Material
-   Mobility
-   Piece-Square Tables

## Getting started

To get started, download the source code and build it using Rust `1.64` or later.
Once compiled, you can start using the engine with any UCI-compatible chess GUI, such as ChessBase, Cute Chess, or Arena Chess GUI.

## License

This project is licensed with the [MIT license](LICENSE).
