/// Module of operations for manipulating the board representation
/// The board is represented as a bitboard, an array of 64 bit integers
/// As the chess board has 64 squares, we assign each square a bit, with the value of each bit determined by the
/// occupancy of the corresponding square.

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

/// Returns an array of longs representing the board state.
/// Each long treated as a 64 bit word
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
fn init_bit_board() -> [u64; 8] {
    let board: [u64; 8] = [
        WHITE_PIECES,
        BLACK_PIECES,
        PAWNS,
        BISHOPS,
        KNIGHTS,
        ROOKS,
        KINGS,
        QUEENS
    ];
    return board;
}

fn make_move(board: &mut [u64; 8], m: Move) -> &mut [u64; 8]{
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
    board[m.piece.board_index() as usize] ^= 1 << m.from;
    board[m.piece.board_index() as usize] ^= 1 << m.to;
    return board;
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

impl Piece {
    /// Returns the index of the bitboard for piece type
    fn board_index(&self) -> u8 {
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
