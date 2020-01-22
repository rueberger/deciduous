/// Module of operations for manipulating the board representation
/// The board is represented as a bitboard, an array of 64 bit integers
/// As the chess board has 64 squares, we assign each square a bit, with the value of each bit determined by the
/// occupancy of the corresponding square.

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


// TODO: figure out module structure

// Some magic constants
/* Initial configuration of white */
static WHITE_PIECES: u64 = 18446462598732840960;
/* Initial configuration of black */
static BLACK_PIECES: u64 = 65535;
/* Initial configuration of pawns */
static PAWNS: u64 = 71776119061282560;
/* Initial configuration of bishops */
static BISHOPS: u64 = 2594073385365405732;
/* Initial confiuration of knights */
static KNIGHTS: u64 = 4755801206503243842;
/* The initla location of rooks */
static ROOKS: u64 = 9295429630892703873;
/* The initial location of the kings */
static KINGS: u64 = 576460752303423496;
/* The initial location of the queens */
static QUEENS: u64 = 1152921504606846992;


pub struct Board {
    // board representation
    pub bitboard: [u64; 8],
    // Lookup tables

    // Value at i performs the eponymous operation when '&'ed with a state
    clear_rank: [u64; 8],
    clear_file: [u64; 8],
    mask_rank: [u64; 8],
    mask_file: [u64; 8],

    // Each value is the eponymous ray for that square
    north: [u64; 64],
    north_west: [u64; 64],
    west: [u64; 64],
    south_west: [u64; 64],
    south: [u64; 64],
    south_east: [u64; 64],
    east: [u64; 64],
    north_east: [u64; 64],
}

impl Board {


    /// Sets the initial board state
    /// Each u64 treated as a 64 bit word
    /// Bit indexing:
    ///    Least significant digit is 0
    ///    MSD is 63
    ///
    /// Square ordering:
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
    pub fn init_bitboard(&mut self) {
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
    };
    board
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

    let mut result: u64 = 11111111;
    return result << (rank_idx * 8);
}

/// Fill file at file_idx
fn fill_file(file_idx: u8) -> u64 {
    assert!(file_idx < 8);

    let mut result: u64 = 0;
    for idx in 0..8 {
        result |= (file_idx as u64) << (idx * 8);
    }
    return result
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


pub fn bitscan_lsd(state: u64) -> u8 {
    return state.trailing_zeros() as u8;
}

// fn ray_attack(board: &[u64, 8], square: u8, direction: u8) {
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
