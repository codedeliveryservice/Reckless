# Reckless – Bitboard Chess Engine

[![Build and Test](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml/badge.svg)](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml)

An attempt to create a simple yet playable [UCI][uci] chess engine.

Please note that the very idea of the project is mostly educational for exploring the Rust programming language.

## Board representation

-   Bitboards with Little-Endian Rank-File Mapping (LERF)
-   Make/undo approach

## Move generation

-   Pseudo legal move generator
-   Pre-calculated attack maps with [Fancy Magic Bitboards][fancy-bitboards]
-   Magic numbers are pre-generated using [Reckless Magics][reckless-magics]
-   Compile time generation of move maps using a [build script](/game/src/lookup/build.rs)

## Static evaluation

-   [Material](https://www.chessprogramming.org/Material)
-   [Piece-Square Tables](https://www.chessprogramming.org/Piece-Square_Tables)

## Search

-   [Negamax search](https://www.chessprogramming.org/Negamax)
-   [Alpha-Beta pruning](https://www.chessprogramming.org/Alpha-Beta)
-   Iterative Deepening
-   Aspiration Windows
-   Principle Variation Search
-   Transposition Table
-   [Quiescence search](https://www.chessprogramming.org/Quiescence_Search)

## Move ordering

-   TT-move ordering
-   [MVV-LVA](https://www.chessprogramming.org/MVV-LVA)
-   Killer Heuristic

## Getting started

To get started, download the source code and compile it with Rust. Once compiled, you can start playing against the engine by running the executable with the UCI protocol or, more preferably, with an external GUI such as Arena Chess GUI, Cute Chess, ChessBase or another GUI of your choice.

[uci]: https://en.wikipedia.org/wiki/Universal_Chess_Interface
[fancy-bitboards]: https://www.chessprogramming.org/Magic_Bitboards#Fancy
[reckless-magics]: https://github.com/codedeliveryservice/RecklessMagics
