mod common;

common::assert_perft!(
    endgame_1: ("7k/1r5p/8/3p4/4p3/7P/5PP1/3R2K1 w - - 0 1", 5, 1697353),
    endgame_2: ("8/5np1/5p1k/7P/5R1P/7K/4P3/8 w - - 0 1", 6, 4559934),
    endgame_3: ("3n4/KP6/3k2p1/4p2p/8/2N3P1/8/8 b - - 0 1", 6, 5916628),

    pawn_ending_1: ("8/7k/6pP/6P1/8/p1p5/8/1K6 b - - 0 1", 9, 5519736),
    pawn_ending_2: ("5k2/5P1p/4K3/8/8/8/7P/8 w - - 0 1", 7, 1181584),
    pawn_ending_3: ("8/7p/8/5k2/8/5K2/6PP/8 b - - 0 1", 7, 3222274),

    en_passant_1: ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 5, 674624),
    en_passant_2: ("8/6b1/8/R7/2p3k1/4P3/P2P4/K7 w - - 0 1", 6, 1533504),
);
