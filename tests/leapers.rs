mod common;

common::assert_perft!(
    mate_bishop_knight: ("7k/5K2/8/8/8/6N1/7B/8 w - - 16 9", 7, 998819),
    queen_vs_knight: ("8/8/8/3nk3/8/3Q4/8/7K w - - 0 1", 5, 1330200),
    rook_vs_knight1: ("8/8/8/8/8/3k4/r7/3NK3 w - - 0 1", 6, 1113805),
    rook_vs_knight2: ("8/8/8/8/8/6k1/r7/6NK w - - 0 1", 7, 3108289),
    knight_bishop_vs_knight: ("6k1/6n1/3K4/4N3/8/3B4/8/8 b - - 0 1", 6, 2784713),
);
