/// Module of operations for manipulating the board representation
/// The board is represented as a bitboard, an array of 64 bit integers
/// As the chess board has 64 squares, we assign each square a bit, with the value of each bit determined by the
/// occupancy of the corresponding square.
/// Supports only little-endian architectures



// TODO: figure out module structure

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
// All bits set in the a-file
static A_FILE: u64 = 0x0101010101010101;
// All bits set in the 1st-rank
static FIRST_RANK: u64 = 0x00000000000000FF;

pub struct Board {

    // TODO: switch to more compact bitboard structure soon
    /// Square ordering is Little-Endian Rank-File
    ///
    /// 1: A B C D E F G H | 0 1 2 3 4 5 6 7
    /// 2: A B C D E F G H | 8 9 10 11 12 13 14 15
    /// 3: A B C D E F G H | 16 17 18 19 20 21 22 23
    /// ...
    /// 8: A B C D E F G H | 56 57 58 59 60 61 62 63
    ///
    /// Array contents:
    ///
    /// 0:White pieces
    /// 1:Black pieces
    /// 2:pawns
    /// 3:bishops
    /// 4:knights
    /// 5:rooks
    /// 6:kings
    /// 7:queens
    pub bitboard: [u64; 8],
    // Lookup tables

    // Value at i performs the eponymous operation when '&'ed with a state
    pub clear_rank: [u64; 8],
    pub clear_file: [u64; 8],
    pub mask_rank: [u64; 8],
    pub mask_file: [u64; 8],

    // Each value is the eponymous ray for that square
    //
    //  Allowed values of orientation:
    //
    //   nowe         nort         noea
    //          +7    +8    +9
    //              \  |  /
    //  west    -1 <-  0 -> +1    east
    //              /  |                        \
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

impl Board {

    /// Sets the initial board state
    pub fn initialize(&mut self) {
        self.bitboard = [
            WHITE_PIECES,
            BLACK_PIECES,
            PAWNS,
            BISHOPS,
            KNIGHTS,
            ROOKS,
            KINGS,
            QUEENS
        ];

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
                let idx = square_idx(rank as u8, file as u8);

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
}

pub fn init_board() -> Board {
    let mut board = Board {
        bitboard: [0; 8],
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
    board
}

pub fn square_idx(rank_idx: u8, file_idx: u8) -> usize {
    assert!((rank_idx < 8) & (file_idx < 8));

    (rank_idx * 8 + file_idx) as usize
}

fn rank_idx(square_idx: u8) -> u8 {
    square_idx >> 3
}

fn file_idx(square_idx: u8) -> u8 {
    square_idx & 7
}


fn make_move(board: &mut [u64; 8], m: Move) -> &mut [u64; 8] {
    match m.color {
        Color::White => {
            board[0] ^= 1 << m.from;
            board[0] ^= 1 << m.to;
            if let Some(_capture) = &m.capture {
                board[1] ^= 1 << m.to
            }
        }
        Color::Black => {
            board[1] ^= 1 << m.from;
            board[1] ^= 1 << m.to;
            if let Some(_capture) = &m.capture {
                board[0] ^= 1 << m.to;
            }
        }
    }
    if let Some(capture) = &m.capture {
        board[capture.board_index() as usize] ^= 1 << m.to;
    }
    board[m.piece.board_index()] ^= 1 << m.from;
    board[m.piece.board_index()] ^= 1 << m.to;
    return board;
}

fn unmake_move(board: &mut [u64; 8], m: Move) -> &mut [u64; 8] {
    // xor is its own inverse operation
    return make_move(board, m)
}

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

// fn ray_attack(board: &[u64, 8], square: u8, direction: u8) {
//
//
//}







// fn generate_rook_moves(board: &[u64; 8], color: Color) -> Vec<[u64; 8]> {
//     let rook_moves = Vec::new();
//     let rooks = board[color.board_index()] & board[Piece::Rook.board_index()];
//     while (rooks != 0) {
//     }
// }

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

impl Color {
    /// Returns the index of the bitboard for color
    fn board_index(&self) ->  usize {
        match self {
            Color::White => 0,
            Color::Black => 1
        }
    }
}

enum Piece {
    Pawn,
    Bishop,
    Knight,
    Rook,
    King,
    Queen
}

impl Piece {
    /// Returns the index of the bitboard for piece type
    fn board_index(&self) -> usize {
        match self {
            Piece::Pawn => 2,
            Piece::Bishop => 3,
            Piece::Knight => 4,
            Piece::Rook => 5,
            Piece::King => 6,
            Piece::Queen => 7
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitscan_lsd() {
        assert_eq!(Some(1), bitscan_lsd(1 << 1));
        assert_eq!(Some(63), bitscan_lsd(1 << 63));
        assert_eq!(Some(1), bitscan_lsd((1 << 1) ^ (1 << 5)));
        assert_eq!(None, bitscan_lsd(0));
    }

    #[test]
    fn test_idx_bijection() {
        for sq_idx in 0..63 {
            let rank = rank_idx(sq_idx);
            let file = file_idx(sq_idx);
            assert_eq!(square_idx(rank, file), sq_idx)
        }
    }

    #[test]
    fn test_fill_rank_0() {
        assert_eq!(fill_rank(0), FIRST_RANK);
    }

    #[test]
    fn test_fill_file_0() {
        assert_eq!(fill_file(0), A_FILE)
    }
}
