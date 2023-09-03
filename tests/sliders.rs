mod common;

common::assert_perft!(
    mate_queen_rook: ("8/8/8/3k4/8/8/8/5RQK w - - 0 1", 6, 3612331),
    queen_vs_rook: ("8/8/4r3/3k4/8/8/3K1Q2/8 w - - 0 1", 5, 2794712),
    queen_vs_bishop: ("8/8/3B4/6K1/8/8/2k5/q7 b - - 0 1", 5, 3669326),
    rook_vs_rook_bishop1: ("3k4/4r3/3K4/3B4/8/8/8/5R2 b - - 0 1", 5, 1993965),
    rook_vs_rook_bishop2: ("8/8/2k5/3b4/4r3/8/7K/1Q6 w - - 0 1", 5, 4074575),
);
