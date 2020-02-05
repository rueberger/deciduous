/// Module containing all move generation logic

use crate::board::*;
use crate::utils::*;

// All bits set in the a-file
static A_FILE: u64 = 0x0101010101010101;
// All bits set in the 1st-rank
static FIRST_RANK: u64 = 0x00000000000000FF;
// All bits set from a1-h8
static DIAGONAL: u64 = 0x8040201008040201;
// All bits set from a8-h1
static ANTI_DIAGONAL: u64 = 0x0102040810204080;

/// This struct holds state required for move generation (tables)
pub struct MoveGen {
    // Value at i performs the eponymous operation when '&'ed with a state
    pub clear_rank: [u64; 8],
    pub clear_file: [u64; 8],
    pub mask_rank: [u64; 8],
    pub mask_file: [u64; 8],
    pub mask_diag: [u64; 15],
    pub mask_anti_diag: [u64; 15],

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
    // NOTE: the ray at idx does not include idx
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
    knight_movement: [u64; 64],
    king_movement: [u64; 64]
}

impl MoveGen {

    /// Initialize tables
    pub fn initialize(&mut self) {
        // initialize orthogonal mask tables
        for idx in 0..8 {
            self.mask_rank[idx] = fill_rank(idx as u8);
            self.mask_file[idx]= fill_file(idx as u8);
            self.clear_rank[idx] = !self.mask_rank[idx];
            self.clear_file[idx] = !self.mask_file[idx];
        }

        // initialize diagonal mask tables
        for idx in 0..64 {
            let diag_idx = diag_index(idx);
            let anti_diag_idx = anti_diag_index(idx);

            self.mask_diag[diag_idx as usize] |= 1 << idx;
            self.mask_anti_diag[anti_diag_idx as usize]|= 1 << idx;
        }

        // initialize ray tables
        // rectangular rays
        for rank in 0..8 {
            for file in 0..8 {
                let vertical = self.mask_file[file] & self.clear_rank[rank];
                let horizontal = self.mask_rank[rank] & self.clear_file[file];
                let idx = square_index(rank as u8, file as u8) as usize;

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

            self.knight_movement[idx] |= (sq << 17) & self.clear_file[0];
            self.knight_movement[idx] |= (sq << 10) & (self.clear_file[0] & self.clear_file[1]);
            self.knight_movement[idx] |= (sq >> 6) & (self.clear_file[0] & self.clear_file[1]);
            self.knight_movement[idx] |= (sq >> 15) & self.clear_file[0];
            self.knight_movement[idx] |= (sq >> 17) & self.clear_file[7];
            self.knight_movement[idx] |= (sq >> 10) & (self.clear_file[6] & self.clear_file[7]);
            self.knight_movement[idx] |= (sq << 6) & (self.clear_file[6] & self.clear_file[7]);
            self.knight_movement[idx] |= (sq << 15) & self.clear_file[7];

        }

        // initialize king movement tables
        for idx in 0..64 {
            let sq = 1 << idx;

            self.king_movement[idx] |= sq << 8;
            self.king_movement[idx] |= (sq << 9) & self.clear_file[0];
            self.king_movement[idx] |= (sq << 1) & self.clear_file[0];
            self.king_movement[idx] |= (sq >> 7) & self.clear_file[0];
            self.king_movement[idx] |= sq >> 8;
            self.king_movement[idx] |= (sq >> 9) & self.clear_file[7];
            self.king_movement[idx] |= (sq >> 1) & self.clear_file[7];
            self.king_movement[idx] |= (sq << 7) & self.clear_file[7];
        }
    }

    // TODO: deprecate? kinda useless
    pub fn ray(&self, sq_idx: usize, orientation: &Orientation) -> u64 {
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

    /// Calculates all south west attacks using dumb7fill
    ///
    /// Args:
    ///   sliders: bits set wherever attacking pieces are
    ///   empty: bits set at all empty squares
    pub fn south_west_attacks(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = sliders;
        let mask = empty & self.clear_file[7];
        for _ in 0..7 {
            flood |= (flood >> 9) & mask;
        }
        (flood >> 9) & self.clear_file[7]
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
    pub fn pawn_pushes(&self, board: &Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let own_pawns = board.own_pieces & board.pawns;
        let empty = board.empty();

        let mut flood = (own_pawns << 8) & empty;
        flood |= (flood << 8) & empty;

        let pushes = self.parse_vertical_moves(flood, own_pawns);

        for (from_idx, to_idx) in pushes.iter() {
            move_list.push(
                Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: Piece::Pawn,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Normal
                }
            )
        }

        move_list
    }

    /// Return possible captures
    /// Does not treat en passant
    pub fn pawn_captures(&self, board: &Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let own_pawns = board.own_pieces & board.pawns;

        let right_moves = (own_pawns << 9) & self.clear_file[0];
        let right_captures = self.parse_diagonal_moves(right_moves & board.opp_pieces, own_pawns);

        let left_moves = (own_pawns << 7) & self.clear_file[7];
        let left_captures = self.parse_anti_diagonal_moves(left_moves & board.opp_pieces, own_pawns);

        for (from_idx, to_idx) in right_captures.iter() {
            move_list.push(
                Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: Piece::Pawn,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal
                }
            )
        }

        for (from_idx, to_idx) in left_captures.iter() {
            move_list.push(
                Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: Piece::Pawn,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal
                }
            )
        }

        move_list
    }

    // TODO: treat en passant and promotion

    // =================================
    //        KNIGHT MOVE GEN
    // =================================

    pub fn knight_moves(&self, board: &Board) -> Vec<Move> {
        let mut move_list = Vec::new();
        let own_knights = board.knights() & board.own_pieces;
        let empty = board.empty();
        let color = board.color();

        for knight_idx in serialize_board(own_knights).iter() {
            let movement = self.knight_movement[*knight_idx as usize];
            let moves = movement & empty;
            let captures = movement & board.opp_pieces;

            for (from_idx, to_idx) in self.parse_single_piece_moves(moves, *knight_idx).iter() {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Knight,
                        color: color,
                        capture: None,
                        category: MoveCategory::Normal
                    }
                )
            }

            for (from_idx, to_idx) in self.parse_single_piece_moves(captures, *knight_idx).iter() {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Knight,
                        color: color,
                        capture: Some(board.identify(*to_idx)),
                        category: MoveCategory::Normal
                    }
                )
            }
        }

        move_list
    }

    // =================================
    //        KING MOVE GEN
    // =================================

    // TODO: castling
    pub fn king_moves(&self, board: &Board) -> Vec<Move> {
        let mut move_list = Vec::new();
        let king_idx = board.own_king;
        let empty = board.empty();
        let color = board.color();

        let movement = self.king_movement[king_idx as usize];
        let moves = movement & empty;
        let captures = movement & board.opp_pieces;

        for (from_idx, to_idx) in self.parse_single_piece_moves(moves, king_idx).iter() {
            move_list.push(
                Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: Piece::King,
                    color: color,
                    capture: None,
                    category: MoveCategory::Normal
                }
            )
        }

        for (from_idx, to_idx) in self.parse_single_piece_moves(moves, king_idx).iter() {
            move_list.push(
                Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: Piece::King,
                    color: color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal
                }
            )
        }

        move_list
    }

    /// Pseudo-legal castling move generation
    // NOTE: must check the squares we're passing through for check
    pub fn castling_moves(&self, board: &Board) -> Vec<Move> {
        let mut move_list = Vec::new();
        let color = board.color();
        let occupied = board.own_pieces | board.opp_pieces;

        if board.own_castling_rights.kingside {
            let kingside_clearance = (1 << 5) | (1 << 6);

            if (occupied & kingside_clearance) == 0 {
                move_list.push(
                    Move {
                        from: 4,
                        to: 6,
                        piece: Piece::King,
                        color: color,
                        capture: None,
                        category: MoveCategory::Kingside
                    }
                )
            }
        }
        if board.own_castling_rights.queenside {
            let queenside_clearance = (1 << 1) | (1 << 2) | (1 << 3);

            if (occupied & queenside_clearance) == 0 {
                move_list.push(
                    Move {
                        from: 4,
                        to: 2,
                        piece: Piece::King,
                        color: color,
                        capture: None,
                        category: MoveCategory::Queenside
                    }
                )
            }
        }

        move_list
    }


    // =================================
    //         SLIDING MOVE GEN
    // =================================

    /// Returns all possible orthogonal moves (rooks and queens)
    pub fn ortho_moves(&self, board: &Board) -> Vec<Move> {
        let mut move_list: Vec<Move> = Vec::new();
        let mut moves: Vec<(u8, u8)> = Vec::new();
        let mut captures: Vec<(u8, u8)> = Vec::new();

        let queens = board.queens();
        let empty = board.empty();

        let north = self.north_attacks(board.ortho_sliders, empty);
        let north_captures = north & board.opp_pieces;
        let north_moves = north & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(north_captures, board.ortho_sliders, Orientation::North));
        moves.append(&mut self.parse_sliding_moves(north_moves, board.ortho_sliders, Orientation::North));

        let east = self.east_attacks(board.ortho_sliders, empty);
        let east_captures = east & board.opp_pieces;
        let east_moves = east & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(east_captures, board.ortho_sliders, Orientation::East));
        moves.append(&mut self.parse_sliding_moves(east_moves, board.ortho_sliders, Orientation::East));

        let south = self.south_attacks(board.ortho_sliders, empty);
        let south_captures = south & board.opp_pieces;
        let south_moves = south & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(south_captures, board.ortho_sliders, Orientation::South));
        moves.append(&mut self.parse_sliding_moves(south_moves, board.ortho_sliders, Orientation::South));

        let west = self.west_attacks(board.ortho_sliders, empty);
        let west_captures = west & board.opp_pieces;
        let west_moves = west & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(west_captures, board.ortho_sliders, Orientation::West));
        moves.append(&mut self.parse_sliding_moves(west_moves, board.ortho_sliders, Orientation::West));

        for (from_idx, to_idx) in moves.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Queen,
                        color: board.color(),
                        capture: None,
                        category: MoveCategory::Normal
                    }
                )
            } else {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Rook,
                        color: board.color(),
                        capture: None,
                        category: MoveCategory::Normal
                    }
                )
            }
        }

        for (from_idx, to_idx) in captures.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Queen,
                        color: board.color(),
                        capture: Some(board.identify(*to_idx)),
                        category: MoveCategory::Normal
                    }
                )
            } else {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Rook,
                        color: board.color(),
                        capture: Some(board.identify(*to_idx)),
                        category: MoveCategory::Normal
                    }
                )
            }
        }


        move_list
    }

    /// Returns all possible diagonal moves (bishops and queens)
    pub fn diag_moves(&self, board: &Board) -> Vec<Move> {
        let mut move_list: Vec<Move> = Vec::new();
        let mut moves: Vec<(u8, u8)> = Vec::new();
        let mut captures: Vec<(u8, u8)> = Vec::new();

        let queens = board.queens();
        let empty = board.empty();

        let north_east = self.north_east_attacks(board.ortho_sliders, empty);
        let north_east_captures = north_east & board.opp_pieces;
        let north_east_moves = north_east & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(north_east_captures, board.ortho_sliders, Orientation::NorthEast));
        moves.append(&mut self.parse_sliding_moves(north_east_moves, board.ortho_sliders, Orientation::NorthEast));

        let south_east = self.south_east_attacks(board.ortho_sliders, empty);
        let south_east_captures = south_east & board.opp_pieces;
        let south_east_moves = south_east & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(south_east_captures, board.ortho_sliders, Orientation::SouthEast));
        moves.append(&mut self.parse_sliding_moves(south_east_moves, board.ortho_sliders, Orientation::SouthEast));

        let south_west = self.south_west_attacks(board.ortho_sliders, empty);
        let south_west_captures = south_west & board.opp_pieces;
        let south_west_moves = south_west & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(south_west_captures, board.ortho_sliders, Orientation::SouthWest));
        moves.append(&mut self.parse_sliding_moves(south_west_moves, board.ortho_sliders, Orientation::SouthWest));

        let north_west = self.north_west_attacks(board.ortho_sliders, empty);
        let north_west_captures = north_west & board.opp_pieces;
        let north_west_moves = north_west & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(north_west_captures, board.ortho_sliders, Orientation::NorthWest));
        moves.append(&mut self.parse_sliding_moves(north_west_moves, board.ortho_sliders, Orientation::NorthWest));

        for (from_idx, to_idx) in moves.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Queen,
                        color: board.color(),
                        capture: None,
                        category: MoveCategory::Normal
                    }
                )
            } else {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Bishop,
                        color: board.color(),
                        capture: None,
                        category: MoveCategory::Normal
                    }
                )
            }
        }

        for (from_idx, to_idx) in captures.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Queen,
                        color: board.color(),
                        capture: Some(board.identify(*to_idx)),
                        category: MoveCategory::Normal
                    }
                )
            } else {
                move_list.push(
                    Move {
                        from: *from_idx,
                        to: *to_idx,
                        piece: Piece::Bishop,
                        color: board.color(),
                        capture: Some(board.identify(*to_idx)),
                        category: MoveCategory::Normal
                    }
                )
            }
        }


        move_list
    }


    // TODO: assumption that the max number of colinear pieces is 3 is a bug, promotions
    // TODO: test
    // TODO: worth it to check for colinearity to call a simpler routine?
    // TODO: profile how much all these conditionals cost us
    /// Parse a bitboard of sliding moves for a single orientation into a vector of (from, to) coordinates
    /// Handles colinear pieces
    fn parse_sliding_moves(&self, moves: u64, pieces: u64, orientation: Orientation) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();

        // 1. sort pieces
        // this is just a really unnecessarily complicated sort
        let piece_idxs = serialize_board(pieces);
        let mut axis_wise: [Option<u8>; 15] = [None; 15];

        for piece_idx in piece_idxs.iter() {
            match orientation.axis() {
                Axis::Rank => {
                    axis_wise[file_index(*piece_idx) as usize] = Some(*piece_idx);
                }
                Axis::File => {
                    axis_wise[rank_index(*piece_idx) as usize] = Some(*piece_idx);
                }
                Axis::Diagonal => {
                    axis_wise[anti_diag_index(*piece_idx) as usize] = Some(*piece_idx);
                }
                Axis::AntiDiagonal => {
                    axis_wise[diag_index(*piece_idx) as usize] = Some(*piece_idx);
                }
            }
        }

        // indices of pieces along the current axis
        let mut sorted_piece_idxs = bad_argsort(axis_wise.to_vec());
        let mut masks = Vec::new();

        // some directions require sorted_piece_idxs to be reversed
        match orientation {
            Orientation::North => sorted_piece_idxs.reverse(),
            Orientation::East => sorted_piece_idxs.reverse(),
            Orientation::NorthEast => sorted_piece_idxs.reverse(),
            Orientation::NorthWest => sorted_piece_idxs.reverse(),
            _ => (),
        }

        // 2. generate up to 3 masks
        // the first mask requires special handling
        masks.push(self.ray(sorted_piece_idxs[0] as usize, &orientation));
        for idx in 1..sorted_piece_idxs.len() {
            let forward_mask = self.ray(sorted_piece_idxs[idx] as usize, &orientation);
            let backward_mask = self.ray(sorted_piece_idxs[idx - 1] as usize, &orientation.antipode());
            masks.push(forward_mask & backward_mask)
        }


        // 3. Pass off to directional move-list generator
        for idx in 0..sorted_piece_idxs.len() {
            let piece_idx = sorted_piece_idxs[idx];
            let masked = masks[idx] & moves;
            move_list.append(&mut self.parse_single_piece_moves(masked, piece_idx))
        }
        move_list
    }

    /// Parse a bitboard of moves for a single piece
    fn parse_single_piece_moves(&self, moves: u64, piece_idx: u8) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();

        for move_idx in serialize_board(moves).iter() {
            move_list.push((piece_idx, *move_idx as u8));
        }

        move_list
    }

    // TODO: check validity of pieces?
    /// Parse a bitboard of horizontal moves into a move list
    /// All pieces must be on their own rank
    fn parse_horizontal_moves(&self, moves: u64, pieces: u64) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();
        let piece_idxs = serialize_board(pieces);

        for piece_idx in piece_idxs.iter() {
            let piece_rank = rank_index(*piece_idx);
            let masked = moves & self.mask_rank[piece_rank as usize];
            for move_idx in serialize_board(masked).iter() {
                move_list.push((*piece_idx as u8, *move_idx as u8));
            }
        }

        move_list
    }

    // TODO: check validity of pieces?
    /// Parse a bitboard of vertical moves into a move list
    /// All pieces must be on their own file
    fn parse_vertical_moves(&self, moves: u64, pieces: u64) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();
        let piece_idxs = serialize_board(pieces);

        for piece_idx in piece_idxs.iter() {
            let piece_file = file_index(*piece_idx);
            let masked = moves & self.mask_file[piece_file as usize];
            for move_idx in serialize_board(masked).iter() {
                move_list.push((*piece_idx as u8, *move_idx as u8));
            }
        }

        move_list
    }

    // TODO: check validity of pieces?
    /// Parse a bitboard of diagonal moves into a move list
    /// All pieces must be on their own diagonal
    fn parse_diagonal_moves(&self, moves: u64, pieces: u64) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();
        let piece_idxs = serialize_board(pieces);

        for piece_idx in piece_idxs.iter() {
            let piece_diag = diag_index(*piece_idx);
            let masked = moves & self.mask_diag[piece_diag as usize];
            for move_idx in serialize_board(masked).iter() {
                move_list.push((*piece_idx as u8, *move_idx as u8));
            }
        }

        move_list
    }

    // TODO: check validity of pieces?
    /// Parse a bitboard of anti-diagonal moves into a move list
    /// All pieces must be on their own anti-diagonal
    fn parse_anti_diagonal_moves(&self, moves: u64, pieces: u64) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();
        let piece_idxs = serialize_board(pieces);

        for piece_idx in piece_idxs.iter() {
            let piece_anti_diag = anti_diag_index(*piece_idx);
            let masked = moves & self.mask_anti_diag[piece_anti_diag as usize];
            for move_idx in serialize_board(masked).iter() {
                move_list.push((*piece_idx as u8, *move_idx as u8));
            }
        }

        move_list
    }


    // =================================
    //   PSEUDO-LEGAL MOVE LIST GEN
    // =================================

    /// Generate all pseudo-legal moves
    fn psuedo_legal_moves(&self, board: Board) -> Vec<Move>{
        let mut move_list = Vec::new();

        // =================
        //    PAWN MOVES
        // =================

        move_list.append(&mut self.pawn_pushes(&board));
        move_list.append(&mut self.pawn_captures(&board));

        // TODO: add en passant


        // =================
        //  SLIDING MOVES
        // =================

        move_list.append(&mut self.ortho_moves(&board));
        move_list.append(&mut self.diag_moves(&board));

        // TODO: knights, king


        // =================
        //    KNIGHT MOVES
        // =================

        move_list.append(&mut self.knight_moves(&board));

        // =================
        //    KING MOVES
        // =================

        move_list.append(&mut self.king_moves(&board));

        move_list
    }


}

pub fn init_move_gen() -> MoveGen {
    let mut move_gen = MoveGen {
        clear_rank: [0; 8],
        clear_file: [0; 8],
        mask_rank: [0; 8],
        mask_file: [0; 8],
        mask_diag: [0; 15],
        mask_anti_diag: [0; 15],
        north: [0; 64],
        north_west: [0; 64],
        west: [0; 64],
        south_west: [0; 64],
        south: [0; 64],
        south_east: [0; 64],
        east: [0; 64],
        north_east: [0; 64],
        knight_movement: [0; 64],
        king_movement: [0; 64]
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

// TODO: optimize. so many more sophisticated approaches
// easy speed up is divide and conquer with same strat
// much fancier is magic hashing stuff
// TODO: would it be faster to return a slice from a 64 len arr?
pub fn serialize_board(mut state: u64) -> Vec<u8> {
    let mut occupied = Vec::new();

    while state != 0 {
        let occ_idx = bitscan_lsd(state).unwrap();
        occupied.push(occ_idx);
        state ^= 1 << occ_idx
    }
    occupied
}

// TODO: optimize, currently uses naive implementation
fn pop_count(state: u64) -> u8 {
    serialize_board(state).len() as u8
}

// TODO: I don't think color is necessary
pub struct Move {
    pub from: u8, // integer 0-63
    pub to: u8, // integer 0-63
    pub piece: Piece,
    pub color: Color,
    pub capture: Option<Piece>,
    pub category: MoveCategory
}

pub enum MoveCategory {
    Normal,
    Queenside,
    Kingside,
    EnPassant
}

#[derive(Copy, Clone)]
pub enum Color {
    White,
    Black
}

#[derive(PartialEq)]
pub enum Piece {
    Pawn,
    Bishop,
    Knight,
    Rook,
    King,
    Queen
}

enum Axis {
    // horizontal
    Rank,
    // vertical
    File,
    Diagonal,
    AntiDiagonal
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

impl Orientation {

    /// Returns the opposite orientation
    pub fn antipode(&self) -> Orientation {
        match self {
            Orientation::North => Orientation::South,
            Orientation::NorthEast => Orientation::SouthWest,
            Orientation::East => Orientation::West,
            Orientation::SouthEast => Orientation::NorthWest,
            Orientation::South => Orientation::North,
            Orientation::SouthWest => Orientation::NorthEast,
            Orientation::West => Orientation::East,
            Orientation::NorthWest => Orientation::SouthEast
        }
    }

    /// Returns the axis this orientation lies along
    pub fn axis(&self) -> Axis {
        match self {
            Orientation::North => Axis::File,
            Orientation::NorthEast => Axis::Diagonal,
            Orientation::East => Axis::Rank,
            Orientation::SouthEast => Axis::AntiDiagonal,
            Orientation::South => Axis::File,
            Orientation::SouthWest => Axis::Diagonal,
            Orientation::West => Axis::Rank,
            Orientation::NorthWest => Axis::AntiDiagonal
        }
    }
}


// TODO: move test module to descendent of move gen module to test private details
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fill_rank_0() {
        assert_eq!(fill_rank(0), FIRST_RANK);
    }

    #[test]
    fn test_fill_file_0() {
        assert_eq!(fill_file(0), A_FILE)
    }

    #[test]
    fn test_bitscan_lsd() {
        assert_eq!(Some(1), bitscan_lsd(1 << 1));
        assert_eq!(Some(63), bitscan_lsd(1 << 63));
        assert_eq!(Some(1), bitscan_lsd((1 << 1) ^ (1 << 5)));
        assert_eq!(None, bitscan_lsd(0));
    }

    #[test]
    fn test_serialize_board(){
        let mut test_vec = Vec::new();
        let mut test_board = 0;
        assert_eq!(test_vec, serialize_board(test_board));

        test_vec.push(1);
        test_board |= 1 << 1;
        assert_eq!(test_vec, serialize_board(test_board));

        test_vec.push(5);
        test_board |= 1 << 5;
        assert_eq!(test_vec, serialize_board(test_board));

    }

}
