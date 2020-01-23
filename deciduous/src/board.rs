/// Module of operations for manipulating the board representation
/// The board is represented as a bitboard, an array of 64 bit integers
/// As the chess board has 64 squares, we assign each square a bit, with the value of each bit determined by the
/// occupancy of the corresponding square.
/// Supports only little-endian architectures

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
pub fn square_index(rank_idx: u8, file_idx: u8) -> usize {
    assert!((rank_idx < 8) & (file_idx < 8));

    (rank_idx * 8 + file_idx) as usize
}

pub fn rank_idx(square_idx: u8) -> u8 {
    square_idx >> 3
}

pub fn file_idx(square_idx: u8) -> u8 {
    square_idx & 7
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
    pub white_to_move: bool
}

pub struct CastlingRights {
    pub kingside: bool,
    pub queenside: bool
}

pub fn init_board() -> Board {
    let board = Board {
        own_pieces: WHITE_PIECES,
        opp_pieces: BLACK_PIECES,
        ortho_sliders: ROOKS | QUEENS,
        diag_sliders: BISHOPS | QUEENS,
        pawns: PAWNS,
        own_king: square_index(0, 4),
        opp_king: square_index(7, 4),
        own_castling_rights: CastlingRights {
            kingside: true,
            queenside: true
        },
        opp_castling_rights: CastlingRights {
            kingside: true,
            queenside: true
        },
        white_to_move: true
    }
    board
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
            assert_eq!(square_index(rank, file), sq_idx)
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
