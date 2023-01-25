# Reckless – Bitboard Chess Engine

[![Build and Test](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml/badge.svg)](https://github.com/codedeliveryservice/Reckless/actions/workflows/rust.yml)

An attempt to create a simple yet playable chess engine.

Please note that the very idea of the project is mostly educational for exploring the Rust programming language.

## Board representation

The board representation is based on bitboards, also known as bitmaps, which represent a piece centric manner. This approach is an efficient and quite convenient way to manage pieces, especially when it comes to pre-calculated move maps.

At the moment, the results of [Perft][perft] (listing legal moves and making/undoing every move on the board) give ≈40,000 kNPM (40 million notes per second) in the starting position.

## Features of move generation

-   Pseudo legal move generator
-   Pre-calculated attack maps with [Fancy Magic Bitboards][fancy-bitboards]
-   Magic numbers are pre-generated using [Reckless Magics][reckless-magics]
-   Compile time generation of move maps using a [build script](/game/src/lookup/build.rs)
-   Make/undo approach

## Getting started

To get started, download the source code and compile it with Rust. Once compiled, you can start playing against the engine by running the executable with the UCI protocol or, more preferably, with an external GUI such as Arena Chess GUI, Cute Chess, ChessBase or another GUI of your choice.

[perft]: https://www.chessprogramming.org/Perft
[fancy-bitboards]: https://www.chessprogramming.org/Magic_Bitboards#Fancy
[reckless-magics]: https://github.com/codedeliveryservice/RecklessMagics
