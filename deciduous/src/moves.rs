/// Module containing all move generation logic

use crate::board::*;

// All bits set in the a-file
static A_FILE: u64 = 0x0101010101010101;
// All bits set in the 1st-rank
static FIRST_RANK: u64 = 0x00000000000000FF;

/// This struct holds state required for move generation (tables)
pub struct MoveGen {
    // Value at i performs the eponymous operation when '&'ed with a state
    pub clear_rank: [u64; 8],
    pub clear_file: [u64; 8],
    pub mask_rank: [u64; 8],
    pub mask_file: [u64; 8],

    // Each value is the eponymous ray for that square
    //  Allowed values of orientation:
    //
    //   nowe         nort         noea
    //          +7    +8    +9
    //              \  |  /
    //  west    -1 <-  0 -> +1    east
    //              /  |
    //          -9    -8    -7
    //  soWe         sout         soEa
    //
    north: [u64; 64],
    north_west: [u64; 64],
    west: [u64; 64],
    south_west: [u64; 64],
    south: [u64; 64],
    south_east: [u64; 64],
    east: [u64; 64],
    north_east: [u64; 64],

    //  Knight compass rose:
    //
    //        noNoWe    noNoEa
    //            +15  +17
    //             |     |
    //noWeWe  +6 __|     |__+10  noEaEa
    //              \   /
    //               >0<
    //           __ /   \ __
    //soWeWe -10   |     |   -6  soEaEa
    //             |     |
    //            -17  -15
    //        soSoWe    soSoEa
    pub knight_moves: [u64; 64]
}

impl MoveGen {

    /// Initialize tables
    pub fn initialize(&mut self) {
        // initialize mask tables
        for idx in 0..8 {
            self.mask_rank[idx] = fill_rank(idx as u8);
            self.mask_file[idx]= fill_file(idx as u8);
            self.clear_rank[idx] = !self.mask_rank[idx];
            self.clear_file[idx] = !self.mask_file[idx];
        }

        // initialize ray tables
        // rectangular rays
        for rank in 0..8 {
            for file in 0..8 {
                let vertical = self.mask_file[file] & self.clear_rank[rank];
                let horizontal = self.mask_rank[rank] & self.clear_file[file];
                let idx = square_index(rank as u8, file as u8);

                self.north[idx] = rank_range(rank as u8, 7) & vertical;
                self.south[idx] = rank_range(0, rank as u8) & vertical;
                self.west[idx] = file_range(0, file as u8) & horizontal;
                self.east[idx] = file_range(file as u8, 7) & horizontal;
            }
        }

        // diagonal rays
        for idx in 0..64 {
            self.north_east[idx] = 1 << idx;
            self.north_west[idx] = 1 << idx;
            self.south_east[idx] = 1 << idx;
            self.south_west[idx] = 1 << idx;

            for _ in 0..8 {
                self.north_east[idx] |= (self.north_east[idx] & self.clear_file[7]) << 9;
                self.north_west[idx] |= (self.north_west[idx] & self.clear_file[0]) << 7;
                self.south_east[idx] |= (self.south_east[idx] & self.clear_file[7]) >> 7;
                self.south_west[idx] |= (self.south_west[idx] & self.clear_file[0]) >> 9;
            }

            self.north_east[idx] &= !(1 << idx);
            self.north_west[idx] &= !(1 << idx);
            self.south_east[idx] &= !(1 << idx);
            self.south_west[idx] &= !(1 << idx);
        }

        // initialize knight movement tables
        for idx in 0..64 {
            let sq = 1 << idx;

            self.knight_moves[idx] |= (sq << 17) & self.clear_file[0];
            self.knight_moves[idx] |= (sq << 10) & (self.clear_file[0] & self.clear_file[1]);
            self.knight_moves[idx] |= (sq >> 6) & (self.clear_file[0] & self.clear_file[1]);
            self.knight_moves[idx] |= (sq >> 15) & self.clear_file[0];
            self.knight_moves[idx] |= (sq >> 17) & self.clear_file[7];
            self.knight_moves[idx] |= (sq >> 10) & (self.clear_file[6] & self.clear_file[7]);
            self.knight_moves[idx] |= (sq << 6) & (self.clear_file[6] & self.clear_file[7]);
            self.knight_moves[idx] |= (sq << 15) & self.clear_file[7];
        }
    }

    // TODO: deprecate? kinda useless
    pub fn ray(&self, sq_idx: usize, orientation: Orientation) -> u64 {
        match orientation {
            Orientation::North => return self.north[sq_idx],
            Orientation::NorthEast => return self.north_east[sq_idx],
            Orientation::East => return self.east[sq_idx],
            Orientation::SouthEast => return self.south_east[sq_idx],
            Orientation::South => return self.south[sq_idx],
            Orientation::SouthWest => return self.south_west[sq_idx],
            Orientation::West => return self.west[sq_idx],
            Orientation::NorthWest => return self.north_west[sq_idx]
        }
    }


    /// Calculates all north attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn north_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        for _ in 0..7 {
            flood |= (flood << 8) & empty;
        }
        flood << 8
    }

    /// Calculates all north east attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn north_east_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        let mask = empty & self.clear_file[0];
        for _ in 0..7 {
            flood |= (flood << 9) & mask;
        }
        (flood << 9) & self.clear_file[0]
    }

    /// Calculates all east attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn east_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        let mask = empty & self.clear_file[0];
        for _ in 0..7 {
            flood |= (flood << 1) & mask;
        }
        (flood << 1) & self.clear_file[0]
    }

    /// Calculates all south east attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn south_east_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        let mask = empty & self.clear_file[0];
        for _ in 0..7 {
            flood |= (flood >> 7) & mask;
        }
        (flood >> 7) & self.clear_file[0]
    }

    /// Calculates all south attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn south_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        for _ in 0..7 {
            flood |= (flood >> 8) & empty;
        }
        flood >> 8
    }


    /// Calculates all west attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn west_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        let mask = empty & self.clear_file[7];
        for _ in 0..7 {
            flood |= (flood >> 1) & mask;
        }
        (flood >> 1) & self.clear_file[7]
    }

    /// Calculates all north west attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn north_west_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        let mask = empty & self.clear_file[7];
        for _ in 0..7 {
            flood |= (flood << 7) & mask;
        }
        (flood << 7) & self.clear_file[7]
    }

    // =================================
    //         PAWN MOVE GEN
    // =================================

    /// Return possible double and single pawn pushes
    /// Does not treat captures, promotion or en passant
    pub fn pawn_pushes(board: Board) -> u64 {
        let own_pawns = board.own_pieces & board.pawns;
        let empty = !(board.own_pieces & board.opp_pieces);

        let mut flood = (own_pawns << 8) & empty;
        flood |= (flood << 8) & empty;

        flood
    }

    /// Return possible captures
    /// Does not treat en passant
    pub fn pawn_captures(&self, board: Board) -> u64 {
        let own_pawns = board.own_pieces & board.pawns;
        let right_moves = (own_pawns << 9) & self.clear_file[0];
        let left_moves = (own_pawns << 7) & self.clear_file[7];
        (right_moves & board.opp_pieces) | (left_moves & board.opp_pieces)
    }
}

pub fn init_move_gen() -> MoveGen {
    let mut move_gen = MoveGen {
        clear_rank: [0; 8],
        clear_file: [0; 8],
        mask_rank: [0; 8],
        mask_file: [0; 8],
        north: [0; 64],
        north_west: [0; 64],
        west: [0; 64],
        south_west: [0; 64],
        south: [0; 64],
        south_east: [0; 64],
        east: [0; 64],
        north_east: [0; 64],
        knight_moves: [0; 64]
    };
    move_gen
}


// fn make_move(board: &mut [u64; 8], m: Move) -> &mut [u64; 8] {
//     match m.color {
//         Color::White => {
//             board[0] ^= 1 << m.from;
//             board[0] ^= 1 << m.to;
//             if let Some(_capture) = &m.capture {
//                 board[1] ^= 1 << m.to
//             }
//         }
//         Color::Black => {
//             board[1] ^= 1 << m.from;
//             board[1] ^= 1 << m.to;
//             if let Some(_capture) = &m.capture {
//                 board[0] ^= 1 << m.to;
//             }
//         }
//     }
//     if let Some(capture) = &m.capture {
//         board[capture.board_index() as usize] ^= 1 << m.to;
//     }
//     board[m.piece.board_index()] ^= 1 << m.from;
//     board[m.piece.board_index()] ^= 1 << m.to;
//     return board;
// }

// fn unmake_move(board: &mut [u64; 8], m: Move) -> &mut [u64; 8] {
//     // xor is its own inverse operation
//     return make_move(board, m)
// }


/// Fill rank at rank_idx
fn fill_rank(rank_idx: u8) ->  u64 {
    assert!(rank_idx < 8);

    let mut result: u64 = FIRST_RANK;
    result << (rank_idx * 8)
}

/// Fill file at file_idx
fn fill_file(file_idx: u8) -> u64 {
    assert!(file_idx < 8);

    let mut result: u64 = A_FILE;
    result << file_idx
}

/// Fill board ranks from [start, end]
/// NOTE: end idx inclusive
fn rank_range(start: u8, end: u8) -> u64 {
    assert!((start <= end) & (start <= 8) & (end <= 8));

    let mut result: u64 = 0;
    for rank_idx in start..=end {
        result |= fill_rank(rank_idx)
    }
    return result;
}

/// Fill board files from [start, end]
/// NOTE: end idx inclusive
fn file_range(start: u8, end: u8) -> u64 {
    assert!((start <= end) & (start <= 8) & (end <= 8));

    let mut result: u64 = 0;
    for file_idx in start..=end {
        result |= fill_file(file_idx)
    }
    return result;
}

/// Return square index of first set bit
/// If no bit is set returns None
pub fn bitscan_lsd(state: u64) -> Option<u8> {
    let trailing = state.trailing_zeros() as u8;
    if trailing == 64 {
        return None;
    } else {
        return Some(trailing)
    }
}

struct Move {
    from: u8, // integer 0-63
    to: u8, // integer 0-63
    piece: Piece,
    color: Color,
    capture: Option<Piece>
}

enum Color {
    White,
    Black
}

enum Piece {
    Pawn,
    Bishop,
    Knight,
    Rook,
    King,
    Queen
}

enum Orientation {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest
}
