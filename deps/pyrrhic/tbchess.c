/*
 * (c) 2015 basil, all rights reserved,
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

enum {

    PYRRHIC_PAWN    = 1, PYRRHIC_KNIGHT  = 2,
    PYRRHIC_BISHOP  = 3, PYRRHIC_ROOK    = 4,
    PYRRHIC_QUEEN   = 5, PYRRHIC_KING    = 6,

    PYRRHIC_WPAWN   = 1, PYRRHIC_BPAWN   = 9,
    PYRRHIC_WKNIGHT = 2, PYRRHIC_BKNIGHT = 10,
    PYRRHIC_WBISHOP = 3, PYRRHIC_BBISHOP = 11,
    PYRRHIC_WROOK   = 4, PYRRHIC_BROOK   = 12,
    PYRRHIC_WQUEEN  = 5, PYRRHIC_BQUEEN  = 13,
    PYRRHIC_WKING   = 6, PYRRHIC_BKING   = 14,
};

enum {

    PYRRHIC_PROMOSQS      = 0XFF000000000000FFULL,

    PYRRHIC_PRIME_WKING   = 00000000000000000000ULL,
    PYRRHIC_PRIME_WQUEEN  = 11811845319353239651ULL,
    PYRRHIC_PRIME_WROOK   = 10979190538029446137ULL,
    PYRRHIC_PRIME_WBISHOP = 12311744257139811149ULL,
    PYRRHIC_PRIME_WKNIGHT = 15202887380319082783ULL,
    PYRRHIC_PRIME_WPAWN   = 17008651141875982339ULL,
    PYRRHIC_PRIME_BKING   = 00000000000000000000ULL,
    PYRRHIC_PRIME_BQUEEN  = 15484752644942473553ULL,
    PYRRHIC_PRIME_BROOK   = 18264461213049635989ULL,
    PYRRHIC_PRIME_BBISHOP = 15394650811035483107ULL,
    PYRRHIC_PRIME_BKNIGHT = 13469005675588064321ULL,
    PYRRHIC_PRIME_BPAWN   = 11695583624105689831ULL,
    PYRRHIC_PRIME_NONE    = 00000000000000000000ULL,
};

typedef struct PyrrhicPosition {
    uint64_t white, black;
    uint64_t kings, queens, rooks;
    uint64_t bishops, knights, pawns;
    uint8_t rule50, ep; bool turn;
} PyrrhicPosition;

unsigned pyrrhic_move_to        (PyrrhicMove move) { return (move >> PYRRHIC_SHIFT_TO    ) & PYRRHIC_MASK_TO         ; }
unsigned pyrrhic_move_from      (PyrrhicMove move) { return (move >> PYRRHIC_SHIFT_FROM  ) & PYRRHIC_MASK_FROM       ; }
unsigned pyrrhic_move_promotes  (PyrrhicMove move) { return (move >> PYRRHIC_SHIFT_FLAGS ) & PYRRHIC_MASK_PROMO_FLAGS; }

int pyrrhic_colour_of_piece     (uint8_t piece) { return !(piece >>  3); }
int pyrrhic_type_of_piece       (uint8_t piece) { return  (piece & 0x7); }

bool pyrrhic_test_bit           (uint64_t bb, int sq)  { return (bb >> sq) & 0x1;               }
void pyrrhic_enable_bit         (uint64_t *b, int sq)  { *b |=  (1ull << sq);                   }
void pyrrhic_disable_bit        (uint64_t *b, int sq)  { *b &= ~(1ull << sq);                   }
bool pyrrhic_promo_square       (int sq)               { return (PYRRHIC_PROMOSQS >> sq) & 0x1; }
bool pyrrhic_pawn_start_square  (int colour, int sq)   { return (sq >> 3) == (colour ? 1 : 6);  }

// The only two forward-declarations that are needed
bool pyrrhic_do_move(PyrrhicPosition *pos, const PyrrhicPosition *pos0, PyrrhicMove move);
bool pyrrhic_legal_move(const PyrrhicPosition *pos, PyrrhicMove move);


const char pyrrhic_piece_to_char[] = " PNBRQK  pnbrqk";

uint64_t pyrrhic_pieces_by_type(const PyrrhicPosition *pos, int colour, int piece) {

    assert(PYRRHIC_PAWN <= piece && piece <= PYRRHIC_KING);
    assert(colour == PYRRHIC_WHITE || colour == PYRRHIC_BLACK);

    uint64_t side = (colour == PYRRHIC_WHITE ? pos->white : pos->black);

    switch (piece) {
        case PYRRHIC_PAWN   : return pos->pawns   & side;
        case PYRRHIC_KNIGHT : return pos->knights & side;
        case PYRRHIC_BISHOP : return pos->bishops & side;
        case PYRRHIC_ROOK   : return pos->rooks   & side;
        case PYRRHIC_QUEEN  : return pos->queens  & side;
        case PYRRHIC_KING   : return pos->kings   & side;
        default: assert(0); return 0;
    }
}

int pyrrhic_char_to_piece_type(char c) {

    for (int i = PYRRHIC_PAWN; i <= PYRRHIC_KING; i++)
        if (c == pyrrhic_piece_to_char[i])
            return i;
    return 0;
}


uint64_t pyrrhic_calc_key(const PyrrhicPosition *pos, int mirror) {

    uint64_t white = mirror ? pos->black : pos->white;
    uint64_t black = mirror ? pos->white : pos->black;

    return PYRRHIC_POPCOUNT(white & pos->queens ) * PYRRHIC_PRIME_WQUEEN
         + PYRRHIC_POPCOUNT(white & pos->rooks  ) * PYRRHIC_PRIME_WROOK
         + PYRRHIC_POPCOUNT(white & pos->bishops) * PYRRHIC_PRIME_WBISHOP
         + PYRRHIC_POPCOUNT(white & pos->knights) * PYRRHIC_PRIME_WKNIGHT
         + PYRRHIC_POPCOUNT(white & pos->pawns  ) * PYRRHIC_PRIME_WPAWN
         + PYRRHIC_POPCOUNT(black & pos->queens ) * PYRRHIC_PRIME_BQUEEN
         + PYRRHIC_POPCOUNT(black & pos->rooks  ) * PYRRHIC_PRIME_BROOK
         + PYRRHIC_POPCOUNT(black & pos->bishops) * PYRRHIC_PRIME_BBISHOP
         + PYRRHIC_POPCOUNT(black & pos->knights) * PYRRHIC_PRIME_BKNIGHT
         + PYRRHIC_POPCOUNT(black & pos->pawns  ) * PYRRHIC_PRIME_BPAWN;
}

uint64_t pyrrhic_calc_key_from_pcs(int *pieces, int mirror) {

    return pieces[PYRRHIC_WQUEEN  ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_WQUEEN
         + pieces[PYRRHIC_WROOK   ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_WROOK
         + pieces[PYRRHIC_WBISHOP ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_WBISHOP
         + pieces[PYRRHIC_WKNIGHT ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_WKNIGHT
         + pieces[PYRRHIC_WPAWN   ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_WPAWN
         + pieces[PYRRHIC_BQUEEN  ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_BQUEEN
         + pieces[PYRRHIC_BROOK   ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_BROOK
         + pieces[PYRRHIC_BBISHOP ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_BBISHOP
         + pieces[PYRRHIC_BKNIGHT ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_BKNIGHT
         + pieces[PYRRHIC_BPAWN   ^ (mirror ? 8 : 0)] * PYRRHIC_PRIME_BPAWN;
}

uint64_t pyrrhic_calc_key_from_pieces(uint8_t *pieces, int length) {

    static const uint64_t PyrrhicPrimes[] = {
        PYRRHIC_PRIME_NONE , PYRRHIC_PRIME_WPAWN , PYRRHIC_PRIME_WKNIGHT, PYRRHIC_PRIME_WBISHOP,
        PYRRHIC_PRIME_WROOK, PYRRHIC_PRIME_WQUEEN, PYRRHIC_PRIME_WKING  , PYRRHIC_PRIME_NONE   ,
        PYRRHIC_PRIME_NONE , PYRRHIC_PRIME_BPAWN , PYRRHIC_PRIME_BKNIGHT, PYRRHIC_PRIME_BBISHOP,
        PYRRHIC_PRIME_BROOK, PYRRHIC_PRIME_BQUEEN, PYRRHIC_PRIME_BKING  , PYRRHIC_PRIME_NONE   ,
    };

    uint64_t key = 0;
    for (int i = 0; i < length; i++)
        key += PyrrhicPrimes[pieces[i]];

    return key;
}


uint64_t pyrrhic_do_bb_move(uint64_t bb, unsigned from, unsigned to) {
    return (((bb >> from) & 0x1) << to) | (bb & (~(1ull << from) & ~(1ull << to)));
}

PyrrhicMove pyrrhic_build_move(unsigned flags, unsigned from, unsigned to) {
    return ((to    & PYRRHIC_MASK_TO    ) << PYRRHIC_SHIFT_TO    )
         | ((from  & PYRRHIC_MASK_FROM  ) << PYRRHIC_SHIFT_FROM  )
         | ((flags & PYRRHIC_MASK_FLAGS ) << PYRRHIC_SHIFT_FLAGS );
}

PyrrhicMove* pyrrhic_add_move(PyrrhicMove *moves, bool promotes, bool enpass, unsigned from, unsigned to) {

    if (enpass)
        *moves++ = pyrrhic_build_move(PYRRHIC_FLAG_ENPASS, from, to);

    else if (promotes) {
        *moves++ = pyrrhic_build_move(PYRRHIC_FLAG_QPROMO, from, to);
        *moves++ = pyrrhic_build_move(PYRRHIC_FLAG_RPROMO, from, to);
        *moves++ = pyrrhic_build_move(PYRRHIC_FLAG_BPROMO, from, to);
        *moves++ = pyrrhic_build_move(PYRRHIC_FLAG_NPROMO, from, to);
    }

    else
        *moves++ = pyrrhic_build_move(PYRRHIC_FLAG_NONE, from, to);

    return moves;
}


PyrrhicMove* pyrrhic_gen_captures(const PyrrhicPosition *pos, PyrrhicMove *moves) {

    uint64_t us   = pos->turn ? pos->white : pos->black;
    uint64_t them = pos->turn ? pos->black : pos->white;
    uint64_t b, att;

    // Generate captures for the King
    for (b = us & pos->kings; b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_KING_ATTACKS(PYRRHIC_LSB(b)) & them; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate captures for the Rooks & Queens
    for (b = us & (pos->rooks | pos->queens); b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_ROOK_ATTACKS(PYRRHIC_LSB(b), us | them) & them; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate captures for the Bishops & Queens
    for (b = us & (pos->bishops | pos->queens); b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_BISHOP_ATTACKS(PYRRHIC_LSB(b), us | them) & them; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate captures for the Knights
    for (b = us & pos->knights; b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_KNIGHT_ATTACKS(PYRRHIC_LSB(b)) & them; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate captures for the Pawns
    for (b = us & pos->pawns; b; PYRRHIC_POPLSB(&b)) {

        unsigned from = PYRRHIC_LSB(b);

        // Generate Enpassant Captures
        if (pos->ep && pyrrhic_test_bit(PYRRHIC_PAWN_ATTACKS(from, pos->turn), pos->ep))
            moves = pyrrhic_add_move(moves, false, true, from, pos->ep);

        // Generate non-Enpassant Captures
        for (att = PYRRHIC_PAWN_ATTACKS(from, pos->turn) & them; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, pyrrhic_promo_square(PYRRHIC_LSB(att)), false, from, PYRRHIC_LSB(att));
    }

    return moves;
}

PyrrhicMove* pyrrhic_gen_moves(const PyrrhicPosition *pos, PyrrhicMove *moves) {

    const unsigned Forward = (pos->turn == PYRRHIC_WHITE ? 8 : -8);

    uint64_t us   = pos->turn ? pos->white : pos->black;
    uint64_t them = pos->turn ? pos->black : pos->white;
    uint64_t b, att;

    // Generate moves for the King
    for (b = us & pos->kings; b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_KING_ATTACKS(PYRRHIC_LSB(b)) & ~us; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate moves for the Rooks
    for (b = us & (pos->rooks | pos->queens); b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_ROOK_ATTACKS(PYRRHIC_LSB(b), us | them) & ~us; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate moves for the Bishops
    for (b = us & (pos->bishops | pos->queens); b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_BISHOP_ATTACKS(PYRRHIC_LSB(b), us | them) & ~us; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate moves for the Knights
    for (b = us & pos->knights; b; PYRRHIC_POPLSB(&b))
        for (att = PYRRHIC_KNIGHT_ATTACKS(PYRRHIC_LSB(b)) & ~us; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, false, false, PYRRHIC_LSB(b), PYRRHIC_LSB(att));

    // Generate moves for the Pawns
    for (b = us & pos->pawns; b; PYRRHIC_POPLSB(&b)) {

        unsigned from = PYRRHIC_LSB(b);

        // Generate Enpassant Captures
        if (pos->ep && pyrrhic_test_bit(PYRRHIC_PAWN_ATTACKS(from, pos->turn), pos->ep))
            moves = pyrrhic_add_move(moves, false, true, from, pos->ep);

        // Generate any single pawn pushes
        if (!pyrrhic_test_bit(us | them, from + Forward))
            moves = pyrrhic_add_move(moves, pyrrhic_promo_square(from + Forward), false, from, from + Forward);

        // Generate any double pawn pushes
        if (    pyrrhic_pawn_start_square(pos->turn, from)
            && !pyrrhic_test_bit(us | them, from + Forward)
            && !pyrrhic_test_bit(us | them, from + 2 * Forward))
            moves = pyrrhic_add_move(moves, false, false, from, from + 2 * Forward);

        // Generate non-Enpassant Captures
        for (att = PYRRHIC_PAWN_ATTACKS(from, pos->turn) & them; att; PYRRHIC_POPLSB(&att))
            moves = pyrrhic_add_move(moves, pyrrhic_promo_square(PYRRHIC_LSB(att)), false, from, PYRRHIC_LSB(att));
    }

    return moves;
}

PyrrhicMove* pyrrhic_gen_legal(const PyrrhicPosition *pos, PyrrhicMove *moves) {

    PyrrhicMove _moves[TB_MAX_MOVES];
    PyrrhicMove *end = pyrrhic_gen_moves(pos, _moves);
    PyrrhicMove *results = moves;

    for (PyrrhicMove *m = _moves; m < end; m++)
        if (pyrrhic_legal_move(pos, *m))
            *results++ = *m;
    return results;
}


bool pyrrhic_is_pawn_move(const PyrrhicPosition *pos, PyrrhicMove move) {
    uint64_t us = pos->turn ? pos->white : pos->black;
    return pyrrhic_test_bit(us & pos->pawns, pyrrhic_move_from(move));
}

bool pyrrhic_is_en_passant(const PyrrhicPosition *pos, PyrrhicMove move) {
    return pyrrhic_is_pawn_move(pos, move)
        && pyrrhic_move_to(move) == pos->ep && pos->ep;
}

bool pyrrhic_is_capture(const PyrrhicPosition *pos, PyrrhicMove move) {
   uint64_t them = pos->turn ? pos->black : pos->white;
   return pyrrhic_test_bit(them, pyrrhic_move_to(move))
       || pyrrhic_is_en_passant(pos, move);
}

bool pyrrhic_is_legal(const PyrrhicPosition *pos) {

    uint64_t us   = pos->turn ? pos->black : pos->white;
    uint64_t them = pos->turn ? pos->white : pos->black;
    unsigned sq   = PYRRHIC_LSB(pos->kings & us);

    return !(PYRRHIC_KING_ATTACKS(sq) & pos->kings & them)
        && !(PYRRHIC_ROOK_ATTACKS(sq, us | them) & (pos->rooks | pos->queens) & them)
        && !(PYRRHIC_BISHOP_ATTACKS(sq, us | them) & (pos->bishops | pos->queens) & them)
        && !(PYRRHIC_KNIGHT_ATTACKS(sq) & pos->knights & them)
        && !(PYRRHIC_PAWN_ATTACKS(sq, !pos->turn) & pos->pawns & them);
}

bool pyrrhic_is_check(const PyrrhicPosition *pos) {

    uint64_t us   = pos->turn ? pos->white : pos->black;
    uint64_t them = pos->turn ? pos->black : pos->white;
    unsigned sq   = PYRRHIC_LSB(pos->kings & us);

    return (PYRRHIC_ROOK_ATTACKS(sq, us | them) & ((pos->rooks | pos->queens) & them))
        || (PYRRHIC_BISHOP_ATTACKS(sq, us | them) & ((pos->bishops | pos->queens) & them))
        || (PYRRHIC_KNIGHT_ATTACKS(sq) & (pos->knights & them))
        || (PYRRHIC_PAWN_ATTACKS(sq, pos->turn) & (pos->pawns & them));
}

bool pyrrhic_is_mate(const PyrrhicPosition *pos) {

    if (!pyrrhic_is_check(pos)) return 0;

    PyrrhicPosition pos1;
    PyrrhicMove moves0[TB_MAX_MOVES];
    PyrrhicMove *moves = moves0;
    PyrrhicMove *end = pyrrhic_gen_moves(pos, moves);

    for (; moves < end; moves++)
        if (pyrrhic_do_move(&pos1, pos, *moves))
            return 0;
    return 1;
}


bool pyrrhic_do_move(PyrrhicPosition *pos, const PyrrhicPosition *pos0, PyrrhicMove move) {

    unsigned from     = pyrrhic_move_from(move);
    unsigned to       = pyrrhic_move_to(move);
    unsigned promotes = pyrrhic_move_promotes(move);

    // Swap the turn and update every Bitboard as needed
    pos->turn    = !pos0->turn;
    pos->white   = pyrrhic_do_bb_move(pos0->white,   from, to);
    pos->black   = pyrrhic_do_bb_move(pos0->black,   from, to);
    pos->kings   = pyrrhic_do_bb_move(pos0->kings,   from, to);
    pos->queens  = pyrrhic_do_bb_move(pos0->queens,  from, to);
    pos->rooks   = pyrrhic_do_bb_move(pos0->rooks,   from, to);
    pos->bishops = pyrrhic_do_bb_move(pos0->bishops, from, to);
    pos->knights = pyrrhic_do_bb_move(pos0->knights, from, to);
    pos->pawns   = pyrrhic_do_bb_move(pos0->pawns,   from, to);
    pos->ep      = 0;

    // Promotions reset the Fifty-Move Rule and add a piece
    if (promotes) {

        pyrrhic_disable_bit(&pos->pawns, to);

        switch (promotes) {
            case PYRRHIC_FLAG_QPROMO: pyrrhic_enable_bit(&pos->queens , to); break;
            case PYRRHIC_FLAG_RPROMO: pyrrhic_enable_bit(&pos->rooks  , to); break;
            case PYRRHIC_FLAG_BPROMO: pyrrhic_enable_bit(&pos->bishops, to); break;
            case PYRRHIC_FLAG_NPROMO: pyrrhic_enable_bit(&pos->knights, to); break;
        }

        pos->rule50 = 0;
    }

    // Pawn moves can be Enpassant, or allow a future Enpassant
    else if (pyrrhic_test_bit(pos0->pawns, from)) {

        pos->rule50 = 0; // Pawn move

        // Check for a double push by White
        if (   (from ^ to) == 16
            &&  pos0->turn == PYRRHIC_WHITE
            && (PYRRHIC_PAWN_ATTACKS(from + 8, PYRRHIC_WHITE) & pos0->pawns & pos0->black))
            pos->ep = from + 8;

        // Check for a double push by Black
        if (   (from ^ to) == 16
            &&  pos0->turn == PYRRHIC_BLACK
            && (PYRRHIC_PAWN_ATTACKS(from - 8, PYRRHIC_BLACK) & pos0->pawns & pos0->white))
            pos->ep = from - 8;

        // Check for an Enpassant being played
        else if (to == pos0->ep) {
            pyrrhic_disable_bit(&pos->white, pos0->turn ? to - 8: to + 8);
            pyrrhic_disable_bit(&pos->black, pos0->turn ? to - 8: to + 8);
            pyrrhic_disable_bit(&pos->pawns, pos0->turn ? to - 8: to + 8);
        }
    }

    // Any other sort of capture also resets the Fifty-Move Rule
    else if (pyrrhic_test_bit(pos0->white | pos0->black, to))
        pos->rule50 = 0;

    // Otherwise, carry on as normal
    else
        pos->rule50 = pos0->rule50 + 1;

    // Provider the caller information about legality
    return pyrrhic_is_legal(pos);
}

bool pyrrhic_legal_move(const PyrrhicPosition *pos, PyrrhicMove move) {
   PyrrhicPosition pos1;
   return pyrrhic_do_move(&pos1, pos, move);
}

