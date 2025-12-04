/*
 * Copyright (c) 2013-2020 Ronald de Man
 * Copyright (c) 2015 Basil, all rights reserved,
 * Modifications Copyright (c) 2016-2019 by Jon Dart
 * Modifications Copyright (c) 2020-2024 by Andrew Grant
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

#pragma once

/// General compatibility for executing tablebase calls

#define PYRRHIC_BLACK   0
#define PYRRHIC_WHITE   1

/// For providing Results arrays to tb_probe_root()

#define TB_MAX_MOVES    256

/// Possible return values from a successful tb_probe_wdl()

#define TB_LOSS         0  /* LOSS */
#define TB_BLESSED_LOSS 1  /* LOSS but 50-move draw */
#define TB_DRAW         2  /* DRAW */
#define TB_CURSED_WIN   3  /* WIN but 50-move draw  */
#define TB_WIN          4  /* WIN  */

/// Possible return values from a failed tb_probe_wdl() or tb_probe_root()

#define TB_RESULT_CHECKMATE     TB_SET_WDL(0, TB_WIN)
#define TB_RESULT_STALEMATE     TB_SET_WDL(0, TB_DRAW)
#define TB_RESULT_FAILED        0xFFFFFFFF

/// Decoding Tablebase Result -> Your Engine's Move Encoding, + WDL/DTZ data

#define TB_RESULT_WDL(_res)       (((_res) & TB_RESULT_WDL_MASK     ) >> TB_RESULT_WDL_SHIFT     )
#define TB_RESULT_DTZ(_res)       (((_res) & TB_RESULT_DTZ_MASK     ) >> TB_RESULT_DTZ_SHIFT     )

#define TB_RESULT_TO(_res)        (((_res) & TB_RESULT_TO_MASK      ) >> TB_RESULT_TO_SHIFT      )
#define TB_RESULT_FROM(_res)      (((_res) & TB_RESULT_FROM_MASK    ) >> TB_RESULT_FROM_SHIFT    )
#define TB_RESULT_IS_ENPASS(_res) (((_res) & TB_RESULT_EP_MASK      ) >> TB_RESULT_EP_SHIFT      )

#define TB_RESULT_IS_QPROMO(_res) (TB_GET_PROMOTES((_res)) == PYRRHIC_FLAG_QPROMO)
#define TB_RESULT_IS_RPROMO(_res) (TB_GET_PROMOTES((_res)) == PYRRHIC_FLAG_RPROMO)
#define TB_RESULT_IS_BPROMO(_res) (TB_GET_PROMOTES((_res)) == PYRRHIC_FLAG_BPROMO)
#define TB_RESULT_IS_NPROMO(_res) (TB_GET_PROMOTES((_res)) == PYRRHIC_FLAG_NPROMO)

/// Decoding PyrrhicMove -> Your Engine's Move Encoding

#define PYRRHIC_MOVE_TO(x)    (((x) >> PYRRHIC_SHIFT_TO  ) & PYRRHIC_MASK_TO  )
#define PYRRHIC_MOVE_FROM(x)  (((x) >> PYRRHIC_SHIFT_FROM) & PYRRHIC_MASK_FROM)

#define PYRRHIC_MOVE_IS_ENPASS(x) (PYRRHIC_MOVE_FLAGS((x)) == PYRRHIC_FLAG_ENPASS)
#define PYRRHIC_MOVE_IS_QPROMO(x) (PYRRHIC_MOVE_FLAGS((x)) == PYRRHIC_FLAG_QPROMO)
#define PYRRHIC_MOVE_IS_RPROMO(x) (PYRRHIC_MOVE_FLAGS((x)) == PYRRHIC_FLAG_RPROMO)
#define PYRRHIC_MOVE_IS_BPROMO(x) (PYRRHIC_MOVE_FLAGS((x)) == PYRRHIC_FLAG_BPROMO)
#define PYRRHIC_MOVE_IS_NPROMO(x) (PYRRHIC_MOVE_FLAGS((x)) == PYRRHIC_FLAG_NPROMO)

/// For tb_probe_root_dtz() and tb_probe_root_wdl()

struct TbRootMove {
    PyrrhicMove move;
    int32_t tbRank;
};

struct TbRootMoves {
    unsigned size;
    struct TbRootMove moves[TB_MAX_MOVES];
};

/// Init/Deinit for Pyrrhic

bool tb_init(const char *_path);
void tb_free(void);

/// Pyrrhic Tablebase Probing Functions

unsigned tb_probe_wdl(
    uint64_t white,     uint64_t black,
    uint64_t kings,     uint64_t queens,
    uint64_t rooks,     uint64_t bishops,
    uint64_t knights,   uint64_t pawns,
    unsigned ep,        bool     turn);

unsigned tb_probe_root(
    uint64_t white,    uint64_t black,
    uint64_t kings,    uint64_t queens,
    uint64_t rooks,    uint64_t bishops,
    uint64_t knights,  uint64_t pawns,
    unsigned rule50,   unsigned ep,
    bool     turn,     unsigned *results);

int tb_probe_root_dtz(
    uint64_t white,    uint64_t black,
    uint64_t kings,    uint64_t queens,
    uint64_t rooks,    uint64_t bishops,
    uint64_t knights,  uint64_t pawns,
    unsigned rule50,   unsigned ep,
    bool     turn,     bool hasRepeated,
    struct TbRootMoves *results);

int tb_probe_root_wdl(
    uint64_t white,    uint64_t black,
    uint64_t kings,    uint64_t queens,
    uint64_t rooks,    uint64_t bishops,
    uint64_t knights,  uint64_t pawns,
    unsigned rule50,   unsigned ep,
    bool     turn,     bool useRule50,
    struct TbRootMoves *results);
