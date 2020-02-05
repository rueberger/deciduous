/// Module of operations for manipulating the board representation
/// The board is represented as a bitboard, an array of 64 bit integers
/// As the chess board has 64 squares, we assign each square a bit, with the value of each bit determined by the
/// occupancy of the corresponding square.
/// Supports only little-endian architectures

use std::mem;
use crate::moves::*;

// Some magic constants
// Initial configuration of white
static WHITE_PIECES: u64 = 65535;
// Initial configuration of black
static BLACK_PIECES: u64 = 18446462598732840960;
// Initial configuration of pawns
static PAWNS: u64 = 71776119061282560;
// Initial configuration of bishops
static BISHOPS: u64 = 2594073385365405732;
// Initial confiuration of knights
static KNIGHTS: u64 = 4755801206503243842;
// The initla location of rooks
static ROOKS: u64 = 9295429630892703873;
// The initial location of the kings
static KINGS: u64 = 576460752303423496;
// The initial location of the queens
static QUEENS: u64 = 1152921504606846992;
// The empty set with no bits set
static EMPTY_SET: u64 = 0;
// The universal set with all bits set
static UNIVERSAL_SET: u64 = 18446744073709551615;

/// Square ordering is Little-Endian Rank-File
///
/// 1: A B C D E F G H | 0 1 2 3 4 5 6 7
/// 2: A B C D E F G H | 8 9 10 11 12 13 14 15
/// 3: A B C D E F G H | 16 17 18 19 20 21 22 23
/// ...
/// 8: A B C D E F G H | 56 57 58 59 60 61 62 63
pub fn square_index(rank_idx: u8, file_idx: u8) -> u8 {
    assert!((rank_idx < 8) & (file_idx < 8));

    rank_idx * 8 + file_idx
}

pub fn rank_index(square_idx: u8) -> u8 {
    square_idx >> 3
}

pub fn file_index(square_idx: u8) -> u8 {
    square_idx & 7
}

/// Returns the index of the diagonal square_idx lies on
/// diagonals run along a northwest/southeast heading
/// diagonal indices have range 0..15
/// ordering from top-left to bottom-right:
/// 14 13 12 11 10 9 8 7 6 5 4 3 2 1 0
pub fn diag_index(square_idx: u8) -> u8 {
    let rank = rank_index(square_idx) as i8;
    let file = file_index(square_idx) as i8;
    (7 + rank - file) as u8
}

/// Returns the index of the anti-diagonal square_idx lies on
/// anti-diagonals run along a northeast/southwest heading
/// anti-diagonal indices have range 0..15
/// ordering from bottom-left to top-right:
/// 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14
pub fn anti_diag_index(square_idx: u8) -> u8 {
    let rank = rank_index(square_idx);
    let file = file_index(square_idx);
    rank + file
}


// TODO: optimize, there should be an explicit form
// looks like sq_idx ^ 0b111000 does the trick
/// Return square index after flipping about the horizontal axis
pub fn flip_square_index(sq_idx: u8) -> u8 {
    let flipped = ((1 as u64) << sq_idx).to_be();
    return bitscan_lsd(flipped).unwrap()
}

pub struct Board {
    pub own_pieces: u64,
    pub opp_pieces: u64,
    pub ortho_sliders: u64,
    pub diag_sliders: u64,
    // TODO: lc0 encodes additional info about en passant in ranks 1 and 8
    pub pawns: u64,
    // king positions are represented by square index
    pub own_king: u8,
    pub opp_king: u8,
    pub own_castling_rights: CastlingRights,
    pub opp_castling_rights: CastlingRights,
    pub flipped: bool
}

impl Board {

    // TODO: add tests
    pub fn color_flip(&mut self) {
        self.own_pieces = self.own_pieces.to_be();
        self.opp_pieces = self.opp_pieces.to_be();
        mem::swap(&mut self.own_pieces, &mut self.opp_pieces);
        self.ortho_sliders = self.ortho_sliders.to_be();
        self.diag_sliders = self.ortho_sliders.to_be();
        self.pawns = self.pawns.to_be();
        // TODO: lc0 uses a BoardSquare class for this. should I?
        self.own_king = flip_square_index(self.own_king);
        self.opp_king = flip_square_index(self.opp_king);
        mem::swap(&mut self.own_king, &mut self.opp_king);
        mem::swap(&mut self.own_castling_rights, &mut self.opp_castling_rights);
        self.flipped = !self.flipped;
    }

    pub fn empty(&self) -> u64 {
        !(self.own_pieces & self.opp_pieces)
    }

    pub fn rooks(&self) -> u64 {
        self.ortho_sliders & !self.diag_sliders
    }

    pub fn bishops(&self) -> u64 {
        self.diag_sliders & !self.ortho_sliders
    }

    pub fn queens(&self) -> u64 {
        self.diag_sliders & self.ortho_sliders
    }

    pub fn knights(&self) -> u64 {
        let kings = ((1 as u64) << self.own_king) | ((1 as u64) << self.opp_king);
        let other_pieces = self.ortho_sliders | self.diag_sliders | self.pawns | kings;
        (self.own_pieces | self.opp_pieces) & !other_pieces
    }

    pub fn color(&self) -> Color {
        match self.flipped {
            false => Color::White,
            true => Color::Black
        }
    }

    /// Identify the type of the piece at piece_idx
    pub fn identify(&self, piece_idx: u8) -> Piece {
        let piece: u64 = 1 << piece_idx;
        let kings: u64 = (1 << self.own_king) | (1 << self.opp_king);

        if (self.pawns & piece) != 0 {
            return Piece::Pawn
        } else if (self.rooks() & piece) != 0 {
            return Piece::Rook
        } else if (self.bishops() & piece) != 0 {
            return Piece::Bishop
        } else if (self.queens() & piece) != 0 {
            return Piece::Queen
        } else if (kings & piece) != 0 {
            return Piece::King
        } else {
            return Piece::Knight
        }
    }

    /// Handles the subset of make_move that is an involution (self-inverting)
    /// Does not check  move legality
    pub fn move_involution(&mut self, m: &Move) {
        let move_bb = (1 << m.from) | (1 << m.to);
        let capture_bb = 1 << m.to;

        self.own_pieces ^= move_bb;

        // NOTE: knights do not require explicit treatment, as they are derived
        //  from the complement of all other pieces
        match m.piece {
            Piece::Pawn => {
                self.pawns ^= move_bb;
            }
            Piece::Bishop => {
                self.diag_sliders ^= move_bb;
            }
            Piece::Rook => {
                self.ortho_sliders ^= move_bb;
            }
            Piece::Queen => {
                self.diag_sliders ^= move_bb;
                self.ortho_sliders ^= move_bb
            }
            _ => ()
        }

        if let Some(captured) = &m.capture {
            self.opp_pieces ^= capture_bb;

            // TODO: throw an error on king capture? how to handle king attacks?
            match captured {
                Piece::Pawn => {
                    self.pawns ^= capture_bb;
                },
                Piece::Bishop => {
                    self.diag_sliders ^= capture_bb;
                }
                Piece::Rook => {
                    self.ortho_sliders ^= capture_bb;
                }
                Piece::Queen => {
                    self.diag_sliders ^= capture_bb;
                    self.ortho_sliders ^= capture_bb
                }
                _ => ()
            }
        }

    }

    // TODO: en passant
    /// Make move. Mutates state of self.
    /// Does not check move legality
    /// Returns undo information
    pub fn make_move(&mut self, m: &Move) -> UndoInfo {
        self.move_involution(m);

        let undo = UndoInfo {
            own_castling_rights: self.own_castling_rights,
            opp_castling_rights: self.opp_castling_rights
        };

        match m.piece {
            // TODO: use bitboard for king rep so I can use an involution?
            Piece::King => {
                self.own_king = m.to;
                self.own_castling_rights.king_moved();
            }
            Piece::Rook => {
                if m.from == 0 {
                    self.own_castling_rights.queenside_moved();
                } else if m.from == 7 {
                    self.own_castling_rights.kingside_moved()
                }
            }
            _ => ()
        }

        if m.capture == Some(Piece::Rook) {
            if m.to == 56 {
                self.opp_castling_rights.queenside_moved();
            } else if m.to == 7 {
                self.own_castling_rights.kingside_moved()
            }
        }

        undo
    }

    /// unmake move. Mutates state of self.
    /// Does not check move legality
    pub fn unmake_move(&mut self, m: &Move, undo: &UndoInfo) {
        self.move_involution(m);

        match m.piece {
            // TODO: use bitboard for king rep so I can use an involution?
            Piece::King => {
                self.own_king = m.from;
            }
            _ => ()
        }

        self.own_castling_rights = undo.own_castling_rights;
        self.opp_castling_rights = undo.opp_castling_rights

    }
}


#[derive(Copy, Clone)]
pub struct CastlingRights {
    pub kingside: bool,
    pub queenside: bool
}

impl CastlingRights {
    pub fn king_moved(&mut self) {
        self.kingside = false;
        self.queenside = false
    }

    pub fn kingside_moved(&mut self) {
        self.kingside = false;
    }

    pub fn queenside_moved(&mut self) {
        self.kingside = false;
    }
}

// TODO: en passant
pub struct UndoInfo {
    pub own_castling_rights: CastlingRights,
    pub opp_castling_rights: CastlingRights
}



pub fn init_board() -> Board {
    let board = Board {
        own_pieces: WHITE_PIECES,
        opp_pieces: BLACK_PIECES,
        ortho_sliders: ROOKS | QUEENS,
        diag_sliders: BISHOPS | QUEENS,
        pawns: PAWNS,
        own_king: square_index(0, 4) as u8,
        opp_king: square_index(7, 4) as u8,
        own_castling_rights: CastlingRights {
            kingside: true,
            queenside: true
        },
        opp_castling_rights: CastlingRights {
            kingside: true,
            queenside: true
        },
        flipped: false
    };
    board
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idx_bijection() {
        for sq_idx in 0..63 {
            let rank = rank_index(sq_idx);
            let file = file_index(sq_idx);
            assert_eq!(square_index(rank, file), sq_idx as u8)
        }
    }

}
