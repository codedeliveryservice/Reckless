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

#ifndef TBPROBE_H
#define TBPROBE_H

#include <stdint.h>
#include <stdbool.h>

#include "tbconfig.h"

/// Definitions for PyrrhicMoves

#define PYRRHIC_FLAG_NONE   0x0
#define PYRRHIC_FLAG_QPROMO 0x1
#define PYRRHIC_FLAG_RPROMO 0x2
#define PYRRHIC_FLAG_BPROMO 0x3
#define PYRRHIC_FLAG_NPROMO 0x4
#define PYRRHIC_FLAG_ENPASS 0x8

#define PYRRHIC_SHIFT_TO    0
#define PYRRHIC_SHIFT_FROM  6
#define PYRRHIC_SHIFT_FLAGS 12

#define PYRRHIC_MASK_TO          0x3F
#define PYRRHIC_MASK_FROM        0x3F
#define PYRRHIC_MASK_FLAGS       0x0F
#define PYRRHIC_MASK_PROMO_FLAGS 0x07

#define PYRRHIC_MOVE_FLAGS(x) (((x) >> PYRRHIC_SHIFT_FLAGS) & PYRRHIC_MASK_FLAGS)

/****************************************************************************/
/* MAIN API                                                                 */
/****************************************************************************/

#define TB_MAX_CAPTURES             64
#define TB_MAX_PLY                  256

#define TB_RESULT_WDL_MASK          0x0000000F
#define TB_RESULT_TO_MASK           0x000003F0
#define TB_RESULT_FROM_MASK         0x0000FC00
#define TB_RESULT_PROMOTES_MASK     0x00070000
#define TB_RESULT_EP_MASK           0x00080000
#define TB_RESULT_DTZ_MASK          0xFFF00000
#define TB_RESULT_WDL_SHIFT         0
#define TB_RESULT_TO_SHIFT          4
#define TB_RESULT_FROM_SHIFT        10
#define TB_RESULT_PROMOTES_SHIFT    16
#define TB_RESULT_EP_SHIFT          19
#define TB_RESULT_DTZ_SHIFT         20

#define TB_GET_WDL(_res)                        \
    (((_res) & TB_RESULT_WDL_MASK) >> TB_RESULT_WDL_SHIFT)
#define TB_GET_TO(_res)                         \
    (((_res) & TB_RESULT_TO_MASK) >> TB_RESULT_TO_SHIFT)
#define TB_GET_FROM(_res)                       \
    (((_res) & TB_RESULT_FROM_MASK) >> TB_RESULT_FROM_SHIFT)
#define TB_GET_PROMOTES(_res)                   \
    (((_res) & TB_RESULT_PROMOTES_MASK) >> TB_RESULT_PROMOTES_SHIFT)
#define TB_GET_EP(_res)                         \
    (((_res) & TB_RESULT_EP_MASK) >> TB_RESULT_EP_SHIFT)
#define TB_GET_DTZ(_res)                        \
    (((_res) & TB_RESULT_DTZ_MASK) >> TB_RESULT_DTZ_SHIFT)

#define TB_SET_WDL(_res, _wdl)                  \
    (((_res) & ~TB_RESULT_WDL_MASK) |           \
     (((_wdl) << TB_RESULT_WDL_SHIFT) & TB_RESULT_WDL_MASK))
#define TB_SET_TO(_res, _to)                    \
    (((_res) & ~TB_RESULT_TO_MASK) |            \
     (((_to) << TB_RESULT_TO_SHIFT) & TB_RESULT_TO_MASK))
#define TB_SET_FROM(_res, _from)                \
    (((_res) & ~TB_RESULT_FROM_MASK) |          \
     (((_from) << TB_RESULT_FROM_SHIFT) & TB_RESULT_FROM_MASK))
#define TB_SET_PROMOTES(_res, _promotes)        \
    (((_res) & ~TB_RESULT_PROMOTES_MASK) |      \
     (((_promotes) << TB_RESULT_PROMOTES_SHIFT) & TB_RESULT_PROMOTES_MASK))
#define TB_SET_EP(_res, _ep)                    \
    (((_res) & ~TB_RESULT_EP_MASK) |            \
     (((_ep) << TB_RESULT_EP_SHIFT) & TB_RESULT_EP_MASK))
#define TB_SET_DTZ(_res, _dtz)                  \
    (((_res) & ~TB_RESULT_DTZ_MASK) |           \
     (((_dtz) << TB_RESULT_DTZ_SHIFT) & TB_RESULT_DTZ_MASK))


typedef uint16_t PyrrhicMove;

#include "api.h"

/*
 * The tablebase can be probed for any position where #pieces <= TB_LARGEST.
 */
extern int TB_LARGEST;
extern int TB_NUM_WDL;
extern int TB_NUM_DTM;
extern int TB_NUM_DTZ;

#endif
