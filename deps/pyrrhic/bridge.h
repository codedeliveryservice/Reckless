#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

uint32_t reckless_popcount(uint64_t bitboard);
uint32_t reckless_lsb(uint64_t bitboard);
uint64_t reckless_poplsb(uint64_t* bitboard);

uint64_t reckless_pawn_attacks(uint32_t square, bool color);
uint64_t reckless_knight_attacks(uint32_t square);
uint64_t reckless_bishop_attacks(uint32_t square, uint64_t occupancies);
uint64_t reckless_rook_attacks(uint32_t square, uint64_t occupancies);
uint64_t reckless_queen_attacks(uint32_t square, uint64_t occupancies);
uint64_t reckless_king_attacks(uint32_t square);

#ifdef __cplusplus
}
#endif
