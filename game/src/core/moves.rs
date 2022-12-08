use super::square::Square;

/// Represents a chess move containing the starting and target squares, as well as flags for special moves.
/// The information fits into a 16-bit integer, 6 bits for the start/target square and 4 bits for the flags.
///
/// See [Chess Programming Wiki article](https://www.chessprogramming.org/Encoding_Moves#From-To_Based) for more information.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Move(u16);

// BITS      INFO
// 0-5       start square
// 6-11      target square
// 12-15     flags

// BIN       FLAG
// 0000      quiets
// 0001      double pawn push
// 0010      king castle
// 0011      queen castle
// 0100      captures
// 0101      e.p. capture
// 1000      knight promotion
// 1001      bishop promotion
// 1010      rook   promotion
// 1011      queen  promotion
// 1100      knight promotion capture
// 1101      bishop promotion capture
// 1110      rook   promotion capture
// 1111      queen  promotion capture

const START_MASK: u16 = (1 << 6) - 1;
const TARGET_MASK: u16 = START_MASK << 6;
const CAPTURE_FLAG: u16 = 1 << 14;

impl Move {
    /// Constructs a new quiet move.
    #[inline(always)]
    pub(crate) fn quiet(start: Square, target: Square) -> Self {
        Self(start.0 as u16 | (target.0 as u16) << 6)
    }

    /// Constructs a new move with a capture.
    #[inline(always)]
    pub(crate) fn capture(start: Square, target: Square) -> Self {
        Self(start.0 as u16 | (target.0 as u16) << 6 | CAPTURE_FLAG)
    }

    /// Returns the start square.
    #[inline(always)]
    pub fn start(self) -> Square {
        Square((self.0 & START_MASK) as u8)
    }

    /// Returns the target square.
    #[inline(always)]
    pub fn target(self) -> Square {
        Square(((self.0 & TARGET_MASK) >> 6) as u8)
    }

    /// Returns `true` if `self` is a capture.
    #[inline(always)]
    pub fn is_capture(self) -> bool {
        self.0 & CAPTURE_FLAG != 0
    }
}

#[cfg(test)]
mod tests {
    use crate::core::square::Square;

    use super::Move;

    const START: Square = Square(11);
    const TARGET: Square = Square(47);

    #[test]
    fn quiet() {
        let m = Move::quiet(START, TARGET);

        assert_eq!(m.start(), START);
        assert_eq!(m.target(), TARGET);
        assert_eq!(m.is_capture(), false);
    }

    #[test]
    fn capture() {
        let m = Move::capture(START, TARGET);

        assert_eq!(m.start(), START);
        assert_eq!(m.target(), TARGET);
        assert_eq!(m.is_capture(), true);
    }
}
