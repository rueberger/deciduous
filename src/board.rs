use crate::moves;
use std::char;
use std::fmt;
/// Module of operations for manipulating the board representation
/// The board is represented as a bitboard, an array of 64 bit integers
/// As the chess board has 64 squares, we assign each square a bit, with the value of each bit determined by the
/// occupancy of the corresponding square.
/// Supports only little-endian architectures
use std::mem;

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
// The initial location of rooks
static ROOKS: u64 = 9295429630892703873;
// The initial location of the kings
static KINGS: u64 = 576460752303423496;
// The initial location of the queens
static QUEENS: u64 = 576460752303423496;
// All bits set in the a-file
pub static A_FILE: u64 = 0x0101010101010101;
// All bits set in the 1st-rank
pub static FIRST_RANK: u64 = 0x00000000000000FF;
// All bits excepting the first rank set
static CLEAR_FIRST_RANK: u64 = 18446744073709551360;
// All bits excepting the last rank set
static CLEAR_LAST_RANK: u64 = 72057594037927935;
// All bits excepting the first and last rank set
pub static CLEAR_FIRST_LAST_RANK: u64 = CLEAR_FIRST_RANK & CLEAR_LAST_RANK;
// All bits set from a1-h8
static DIAGONAL: u64 = 0x8040201008040201;
// All bits set from a8-h1
static ANTI_DIAGONAL: u64 = 0x0102040810204080;
// The empty set with no bits set
static EMPTY_SET: u64 = 0;
// The universal set with all bits set
pub static UNIVERSAL_SET: u64 = 18446744073709551615;
// Masks for unpacking king state
static KING_STATE_POS_MASK: u8 = 0b00111111;
static KING_STATE_CHECK_MASK: u8 = 0b01000000;
static KING_STATE_DOUBLE_CHECK_MASK: u8 = 0b10000000;

/// Square ordering is Little-Endian Rank-File
///
/// 1: A B C D E F G H | 0  1  2  3  4  5  6  7
/// 2: A B C D E F G H | 8  9  10 11 12 13 14 15
/// 3: A B C D E F G H | 16 17 18 19 20 21 22 23
//  ...                | 24 25 26 27 28 29 30 31
//                     | 32 33 34 35 36 37 38 39
//                     | 40 41 42 43 44 45 46 47
//                     | 48 49 50 51 52 53 54 55
/// 8: A B C D E F G H | 56 57 58 59 60 61 62 63
//
// =============================================
//
/// 8: A B C D E F G H | 56 57 58 59 60 61 62 63
//                     | 48 49 50 51 52 53 54 55
//                     | 40 41 42 43 44 45 46 47
//                     | 32 33 34 35 36 37 38 39
//  ...                | 24 25 26 27 28 29 30 31
/// 3: A B C D E F G H | 16 17 18 19 20 21 22 23
/// 2: A B C D E F G H | 8  9  10 11 12 13 14 15
/// 1: A B C D E F G H | 0  1  2  3  4  5  6  7

pub fn square_index(rank_idx: u8, file_idx: u8) -> u8 {
    assert!(
        (rank_idx < 8) & (file_idx < 8),
        "r {} f {}",
        rank_idx,
        file_idx
    );

    rank_idx * 8 + file_idx
}

pub fn rank_index(square_idx: u8) -> u8 {
    square_idx >> 3
}

pub fn file_index(square_idx: u8) -> u8 {
    square_idx & 7
}

pub fn fmt_sq(sq: u8) -> String {
    let mut c = String::new();

    c.push((b'a' as u8 + file_index(sq)) as char);
    c.push(std::char::from_digit((rank_index(sq) + 1) as u32, 10).unwrap());

    c
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
    return moves::bitscan_lsd(flipped).unwrap();
}

#[derive(Clone)]
pub struct Board {
    pub own_pieces: u64,
    pub opp_pieces: u64,
    pub ortho_sliders: u64,
    pub diag_sliders: u64,
    pub pawns: u64,
    // 6 bits for square index, two bits for check state
    // convention for 7th and 8th bits:
    // | 7 | 8 |
    // |---|---|
    // | 1 | 0 | check
    // | 0 | 1 | double check
    // | 1 | 1 | knight single check
    // | 0 | 0 | not in check
    own_king_state: u8,
    opp_king_state: u8,
    pub own_castling_rights: CastlingRights,
    pub opp_castling_rights: CastlingRights,
    pub flipped: bool,
}

impl Board {
    pub fn new() -> Self {
        let board = Board {
            own_pieces: WHITE_PIECES,
            opp_pieces: BLACK_PIECES,
            ortho_sliders: ROOKS | QUEENS,
            diag_sliders: BISHOPS | QUEENS,
            pawns: PAWNS,
            own_king_state: square_index(0, 4) as u8,
            opp_king_state: square_index(7, 4) as u8,
            own_castling_rights: CastlingRights {
                kingside: true,
                queenside: true,
            },
            opp_castling_rights: CastlingRights {
                kingside: true,
                queenside: true,
            },
            flipped: false,
        };
        board
    }

    // TODO: add tests, especially for king state
    pub fn color_flip(&mut self) {
        self.own_pieces = self.own_pieces.swap_bytes();
        self.opp_pieces = self.opp_pieces.swap_bytes();
        mem::swap(&mut self.own_pieces, &mut self.opp_pieces);
        self.ortho_sliders = self.ortho_sliders.swap_bytes();
        self.diag_sliders = self.diag_sliders.swap_bytes();
        self.pawns = self.pawns.swap_bytes();
        let own_king_pos = flip_square_index(self.own_king());
        let opp_king_pos = flip_square_index(self.opp_king());
        self.own_king_state &= !KING_STATE_POS_MASK;
        self.own_king_state |= own_king_pos;
        self.opp_king_state &= !KING_STATE_POS_MASK;
        self.opp_king_state |= opp_king_pos;
        mem::swap(&mut self.own_king_state, &mut self.opp_king_state);
        mem::swap(&mut self.own_castling_rights, &mut self.opp_castling_rights);
        self.flipped = !self.flipped;
    }

    pub fn empty(&self) -> u64 {
        !(self.own_pieces | self.opp_pieces)
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

    pub fn sliders(&self, orientation: &moves::Orientation) -> u64 {
        match orientation.axis() {
            moves::Axis::Rank => self.ortho_sliders,
            moves::Axis::File => self.ortho_sliders,
            moves::Axis::Diagonal => self.diag_sliders,
            moves::Axis::AntiDiagonal => self.diag_sliders,
        }
    }

    pub fn knights(&self) -> u64 {
        let kings = ((1 as u64) << self.own_king()) | ((1 as u64) << self.opp_king());
        let pawns = self.pawns & CLEAR_FIRST_LAST_RANK;
        let other_pieces = self.ortho_sliders | self.diag_sliders | pawns | kings;
        (self.own_pieces | self.opp_pieces) & !other_pieces
    }

    // own king position
    pub fn own_king(&self) -> u8 {
        self.own_king_state & KING_STATE_POS_MASK
    }

    // opp king position
    pub fn opp_king(&self) -> u8 {
        self.opp_king_state & KING_STATE_POS_MASK
    }

    pub fn own_king_checked(&self) -> bool {
        self.own_king_state & KING_STATE_CHECK_MASK != 0
    }

    pub fn own_king_double_checked(&self) -> bool {
        self.own_king_state & KING_STATE_DOUBLE_CHECK_MASK != 0 && !self.own_king_state & KING_STATE_CHECK_MASK != 0
    }

    pub fn own_king_knight_checked(&self) -> bool {
        self.own_king_state & KING_STATE_CHECK_MASK != 0
            && self.own_king_state & KING_STATE_DOUBLE_CHECK_MASK != 0
    }

    pub fn color(&self) -> Color {
        match self.flipped {
            false => Color::White,
            true => Color::Black,
        }
    }

    /// Identify the type of the piece at piece_idx
    pub fn identify(&self, piece_idx: u8) -> Piece {
        let piece: u64 = 1 << piece_idx;
        let kings: u64 = (1 << self.own_king()) | (1 << self.opp_king());

        if (self.pawns & CLEAR_FIRST_LAST_RANK & piece) != 0 {
            return Piece::Pawn;
        } else if (self.rooks() & piece) != 0 {
            return Piece::Rook;
        } else if (self.bishops() & piece) != 0 {
            return Piece::Bishop;
        } else if (self.queens() & piece) != 0 {
            return Piece::Queen;
        } else if (kings & piece) != 0 {
            return Piece::King;
        } else {
            return Piece::Knight;
        }
    }

    /// Handles the subset of make_move that is an involution (self-inverting)
    /// Does not check move legality
    pub fn move_involution(&mut self, m: &moves::Move) {
        let move_bb: u64 = match m.category {
            // for promotions the piece field of m denotes promotion choice
            moves::MoveCategory::Promotion => {
                self.pawns ^= 1 << m.from;
                self.own_pieces ^= (1 << m.from) | (1 << m.to);
                1 << m.to
            }
            _ => {
                let move_bb = (1 << m.from) | (1 << m.to);
                self.own_pieces ^= move_bb;
                move_bb
            }
        };

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
            _ => (),
        }

        if let Some(captured) = &m.capture {
            let capture_bb: u64 = match m.category {
                // En passant is the only move where capturing piece doesn't
                // move to location of captured piece.
                // (from, to) describes capturing piece for EP
                moves::MoveCategory::EnPassant => 1 << (m.to - 8),
                _ => 1 << m.to,
            };
            self.opp_pieces ^= capture_bb;

            // TODO: throw an error on king capture? how to handle king attacks?
            match captured {
                Piece::Pawn => {
                    self.pawns ^= capture_bb;
                }
                Piece::Bishop => {
                    self.diag_sliders ^= capture_bb;
                }
                Piece::Rook => {
                    self.ortho_sliders ^= capture_bb;
                }
                Piece::Queen => {
                    self.diag_sliders ^= capture_bb;
                    self.ortho_sliders ^= capture_bb;
                }
                _ => (),
            }
        }
    }

    pub fn make_move(&mut self, m: &moves::Move) -> UndoInfo {
        let undo = UndoInfo {
            castling_rights: self.own_castling_rights,
            // Casting behavior keeps the least significant bits
            en_passant_state: self.pawns as u8,
            check: self.own_king_state & KING_STATE_CHECK_MASK != 0,
            double_check: self.own_king_state & KING_STATE_DOUBLE_CHECK_MASK != 0,
        };

        // Clear own check state. Checkmate is detected by counting number of legal moves.
        self.own_king_state &= KING_STATE_POS_MASK;

        // Set opp check state
        if m.check {
            self.opp_king_state |= KING_STATE_CHECK_MASK;
        }
        if m.double_check {
            self.opp_king_state |= KING_STATE_DOUBLE_CHECK_MASK
        }

        // Clear en passant state from previous turn
        self.pawns &= CLEAR_FIRST_RANK;

        // Set bit in bottom rank to mark pawn as eligible for capture by en passant next turn
        if m.category == moves::MoveCategory::DoublePawnPush {
            self.pawns |= 1 << file_index(m.from);
        }

        // Castling, king movement logic
        match m.category {
            moves::MoveCategory::KingsideCastle => {
                self.own_castling_rights.king_moved();
                self.own_king_state &= !KING_STATE_POS_MASK;
                self.own_king_state |= 6;
                self.own_pieces ^= (1 << 4) | (1 << 6);
            }
            moves::MoveCategory::QueensideCastle => {
                self.own_castling_rights.king_moved();
                self.own_king_state &= !KING_STATE_POS_MASK;
                self.own_king_state |= 2;
                self.own_pieces ^= (1 << 4) | (1 << 2);
            }
            _ => {
                match m.piece {
                    // TODO: use bitboard for king rep so I can use an involution?
                    Piece::King => {
                        self.own_king_state &= !KING_STATE_POS_MASK;
                        self.own_king_state |= m.to;
                        self.own_castling_rights.king_moved();
                    }
                    Piece::Rook => {
                        if m.from == 0 {
                            self.own_castling_rights.queenside_moved();
                        } else if m.from == 7 {
                            self.own_castling_rights.kingside_moved();
                        }
                    }
                    _ => (),
                }

                if m.capture == Some(Piece::Rook) {
                    // TODO: can replace these conditions with logical or ops
                    if m.to == 56 {
                        self.opp_castling_rights.queenside_moved();
                    } else if m.to == 63 {
                        self.opp_castling_rights.kingside_moved();
                    }
                }
            }
        }

        self.move_involution(m);
        self.color_flip();

        undo
    }

    pub fn unmake_move(&mut self, m: &moves::Move, undo: &UndoInfo) {
        self.color_flip();
        self.move_involution(m);

        match m.category {
            moves::MoveCategory::KingsideCastle => {
                self.own_king_state &= !KING_STATE_POS_MASK;
                self.own_king_state |= 4;
                self.own_pieces ^= (1 << 4) | (1 << 6);
            }
            moves::MoveCategory::QueensideCastle => {
                self.own_king_state &= !KING_STATE_POS_MASK;
                self.own_king_state |= 4;
                self.own_pieces ^= (1 << 4) | (1 << 2);
            }
            _ => {
                match m.piece {
                    // TODO: use bitboard for king rep so I can use an involution?
                    Piece::King => {
                        self.own_king_state &= !KING_STATE_POS_MASK;
                        self.own_king_state = m.from;
                    }
                    _ => (),
                }
            }
        }

        self.own_castling_rights = undo.castling_rights;

        self.pawns &= CLEAR_FIRST_RANK;
        self.pawns |= undo.en_passant_state as u64;

        // clear opp check state
        self.opp_king_state &= KING_STATE_POS_MASK;
        // pop own check state
        // TODO: benchmark conditional vs multiplication by check flags
        if undo.check {
            self.own_king_state |= KING_STATE_CHECK_MASK;
        }
        if undo.double_check {
            self.own_king_state |= KING_STATE_DOUBLE_CHECK_MASK
        }
    }

    pub fn set_piece(&mut self, own: bool, sq: u8, piece: Piece) {
        match piece {
            Piece::Bishop => self.diag_sliders |= 1 << sq,
            Piece::Rook => self.ortho_sliders |= 1 << sq,
            Piece::Queen => {
                self.diag_sliders |= 1 << sq;
                self.ortho_sliders |= 1 << sq
            }
            Piece::Pawn => self.pawns |= 1 << sq,
            Piece::Knight => (),
            Piece::King => match own {
                true => {
                    self.own_king_state &= !KING_STATE_POS_MASK;
                    self.own_king_state |= sq;
                }
                false => {
                    self.opp_king_state &= !KING_STATE_POS_MASK;
                    self.opp_king_state |= sq;
                }
            },
        }
        match own {
            true => self.own_pieces |= 1 << sq,
            false => self.opp_pieces |= 1 << sq,
        }
    }

    pub fn empty_board() -> Board{
        let mut b = Board::new();
        b.own_pieces = 0;
        b.opp_pieces = 0;
        b.ortho_sliders = 0;
        b.diag_sliders = 0;
        b.pawns = 0;
        b.own_king_state = 0;
        b.opp_king_state = 0;
        b
    }

    fn format_board(&self) -> String {
        let mut render = String::new();

        let mut b = self.clone();

        if self.flipped {
            b.color_flip();
        }

        b.color_flip();

        // black EP state
        for idx in 0..8 {
            render.push('[');
            if b.pawns & (1 << idx) != 0 {
                render.push('E');
                render.push('P');
            } else {
                render.push(' ');
                render.push(' ');
            }
            render.push(']');
        }

        b.color_flip();

        render.push('\n');
        render.push_str(&"-".repeat(32));
        render.push('\n');

        for rank in (0..8).rev() {
            for file in 0..8 {
                let idx = square_index(rank, file);

                render.push('[');
                if b.empty() & (1 << idx) != 0 {
                    render.push(' ');
                    render.push(' ');
                } else {
                    if b.own_pieces & (1 << idx) != 0 {
                        render.push('w');
                    } else {
                        render.push('b')
                    }

                    match b.identify(idx) {
                        Piece::Pawn => render.push('P'),
                        Piece::Bishop => render.push('B'),
                        Piece::Knight => render.push('N'),
                        Piece::Rook => render.push('R'),
                        Piece::King => render.push('K'),
                        Piece::Queen => render.push('Q'),
                    }
                }
                render.push(']')
            }
            render.push('\n')
        }

        render.push_str(&"-".repeat(32));
        render.push('\n');

        // white EP state
        for idx in 0..8 {
            render.push('[');
            if b.pawns & (1 << idx) != 0 {
                render.push('E');
                render.push('P');
            } else {
                render.push(' ');
                render.push(' ');
            }
            render.push(']')
        }

        return render;
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_board())
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CastlingRights {
    pub kingside: bool,
    pub queenside: bool,
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

#[derive(Debug, Clone)]
pub struct UndoInfo {
    castling_rights: CastlingRights,
    en_passant_state: u8,
    check: bool,
    double_check: bool,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn flip(&self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Piece {
    Pawn,
    Bishop,
    Knight,
    Rook,
    King,
    Queen,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_idx() {
        for idx in 0..8 {
            assert_eq!(file_index(square_index(0, idx)), idx)
        }
    }

    #[test]
    fn test_rank_idx() {
        for idx in 0..8 {
            assert_eq!(rank_index(square_index(idx, 0)), idx)
        }
    }

    #[test]
    fn test_idx_bijection() {
        for sq_idx in 0..63 {
            let rank = rank_index(sq_idx);
            let file = file_index(sq_idx);
            assert_eq!(square_index(rank, file), sq_idx as u8)
        }
    }

    #[test]
    fn make_sets_check_bit() {
        let mut b = Board::new();
        let m = moves::Move {
            from: 8,
            to: 24,
            piece: Piece::Pawn,
            color: Color::White,
            capture: None,
            category: moves::MoveCategory::Normal,
            check: true,
            double_check: false
        };

        let _u = b.make_move(&m);

        assert_eq!(b.own_king_checked(), true);
        assert_eq!(b.own_king_double_checked(), false);
        assert_eq!(b.own_king_knight_checked(), false);
    }

    #[test]
    fn make_sets_double_check_bit() {
        let mut b = Board::new();
        let m = moves::Move {
            from: 8,
            to: 24,
            piece: Piece::Pawn,
            color: Color::White,
            capture: None,
            category: moves::MoveCategory::Normal,
            check: false,
            double_check: true
        };

        let _u = b.make_move(&m);

        assert_eq!(b.own_king_checked(), false);
        assert_eq!(b.own_king_double_checked(), true);
        assert_eq!(b.own_king_knight_checked(), false);
    }

    #[test]
    fn make_sets_knight_check_bit() {
        let mut b = Board::new();
        let m = moves::Move {
            from: 8,
            to: 24,
            piece: Piece::Pawn,
            color: Color::White,
            capture: None,
            category: moves::MoveCategory::Normal,
            check: true,
            double_check: true
        };

        let _u = b.make_move(&m);

        assert_eq!(b.own_king_checked(), true);
        assert_eq!(b.own_king_double_checked(), false);
        assert_eq!(b.own_king_knight_checked(), true);
    }

    #[test]
    fn king_state_bitpacking() {
        for sq in 0..64 {
            let mut b = Board::empty_board();

            let m = moves::Move {
                from: 8,
                to: 24,
                piece: Piece::Pawn,
                color: Color::White,
                capture: None,
                category: moves::MoveCategory::Normal,
                check: false,
                double_check: false
            };

            let _u = b.make_move(&m);

            b.set_piece(true, sq, Piece::King);

            assert_eq!(b.own_king_checked(), false);
            assert_eq!(b.own_king_double_checked(), false);
            assert_eq!(b.own_king(), sq, "king_state {:08b}", b.own_king_state);

            let mut b = Board::empty_board();

            let m = moves::Move {
                from: 8,
                to: 24,
                piece: Piece::Pawn,
                color: Color::White,
                capture: None,
                category: moves::MoveCategory::Normal,
                check: true,
                double_check: false
            };

            let _u = b.make_move(&m);

            b.set_piece(true, sq, Piece::King);

            assert_eq!(b.own_king_checked(), true);
            assert_eq!(b.own_king_double_checked(), false);
            assert_eq!(b.own_king(), sq, "king_state {:08b}", b.own_king_state);
        }
    }

    #[test]
    fn make_unmake_check_state() {
        let mut b = Board::empty_board();

        assert_eq!(b.own_king_state, 0);
        assert_eq!(b.opp_king_state, 0);

        let m1 = moves::Move {
            from: 8,
            to: 24,
            piece: Piece::Pawn,
            color: Color::White,
            capture: None,
            category: moves::MoveCategory::Normal,
            check: true,
            double_check: true
        };

        let u1 = b.make_move(&m1);

        assert_eq!(b.own_king_checked(), true);
        assert_eq!(b.own_king_double_checked(), false);
        assert_eq!(b.own_king_knight_checked(), true);
        assert_eq!(b.opp_king_state, 56, "{:08b}", b.opp_king_state);

        let m2 = moves::Move {
            from: 8,
            to: 24,
            piece: Piece::Pawn,
            color: Color::Black,
            capture: None,
            category: moves::MoveCategory::Normal,
            check: false,
            double_check: false
        };

        let u2 = b.make_move(&m2);

        assert_eq!(b.own_king_state, 0);
        assert_eq!(b.opp_king_state, 0);

        b.unmake_move(&m2, &u2);

        assert_eq!(b.own_king_checked(), true);
        assert_eq!(b.own_king_double_checked(), false);
        assert_eq!(b.own_king_knight_checked(), true);
        assert_eq!(b.opp_king_state, 56);

        b.unmake_move(&m1, &u1);

        assert_eq!(b.own_king_state, 0);
        assert_eq!(b.opp_king_state, 0);

    }


    #[test]
    fn make_unmake_preserves_ep_state() {
        let last_rank = FIRST_RANK << 56;

        // make, unmake, make
        for idx in 0..8 {
            let mut b = Board::new();

            let m = moves::Move {
                from: idx + 8,
                to: idx + 24,
                piece: Piece::Pawn,
                color: Color::White,
                capture: None,
                category: moves::MoveCategory::DoublePawnPush,
                check: false,
                double_check: false,
            };

            let u = b.make_move(&m);

            assert!(b.pawns & (1 << idx + 56) != 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                1,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );

            b.unmake_move(&m, &u);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );

            let u = b.make_move(&m);
            assert!(b.pawns & (1 << idx + 56) != 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                1,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
        }

        // make (double pawn push), make (normal), make (normal), unmake, unmake, unmake
        for idx in 0..8 {
            let mut b = Board::new();

            let m1 = moves::Move {
                from: idx + 8,
                to: idx + 24,
                piece: Piece::Pawn,
                color: Color::White,
                capture: None,
                category: moves::MoveCategory::DoublePawnPush,
                check: false,
                double_check: false,
            };

            let m2 = moves::Move {
                from: 1,
                to: 18,
                piece: Piece::Knight,
                color: Color::Black,
                capture: None,
                category: moves::MoveCategory::Normal,
                check: false,
                double_check: false,
            };

            let m3 = moves::Move {
                from: 1,
                to: 18,
                piece: Piece::Knight,
                color: Color::White,
                capture: None,
                category: moves::MoveCategory::Normal,
                check: false,
                double_check: false,
            };

            println!("\n{:#?}", b);

            let u1 = b.make_move(&m1);
            // EP bit set for white, blacks turn
            println!("\n{:#?}", b);
            assert!(b.pawns & (1 << idx + 56) != 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                1,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );

            let u2 = b.make_move(&m2);
            // EP bit still set for white, whites turn
            println!("\n{:#?}", b);
            assert!(b.pawns & (1 << idx) != 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                1,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );

            let u3 = b.make_move(&m3);
            // EP bit no longer set anywhere, blacks turn
            println!("\n{:#?}", b);
            assert!(b.pawns & (1 << idx + 56) == 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            b.unmake_move(&m3, &u3);
            // EP bit should again be set for white, whites turn
            println!("\n{:#?}", b);
            assert!(
                b.pawns & (1 << idx) != 0,
                "idx: {}, u:\n{:#?}, b:\n{:#?}",
                idx,
                u3,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                1,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );

            b.unmake_move(&m2, &u2);
            // EP bit should still be set for white, blacks turn
            println!("\n{:#?}", b);
            assert!(b.pawns & (1 << idx + 56) != 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                1,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );

            b.unmake_move(&m1, &u1);
            // EP bit no longer set anywhere, whites turn
            println!("\n{:#?}", b);
            assert!(b.pawns & (1 << idx) == 0, "idx: {}, b:\n{:#?}", idx, b);
            assert_eq!(
                moves::pop_count(b.pawns & FIRST_RANK),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
            assert_eq!(
                moves::pop_count(b.pawns & last_rank),
                0,
                "idx: {}, b:\n{:#?}",
                idx,
                b
            );
        }
    }

    // TODO: test that setting EP state doesn't  set  own_pieces
}
