/// Module containing all move generation logic
use std::iter;

use crate::board;
use crate::utils;

static PROMOTION_OPTIONS: [board::Piece; 4] = [
    board::Piece::Bishop,
    board::Piece::Knight,
    board::Piece::Rook,
    board::Piece::Queen,
];

static ORIENTATIONS: [Orientation; 8] = [
    Orientation::East,
    Orientation::NorthEast,
    Orientation::North,
    Orientation::NorthWest,
    Orientation::West,
    Orientation::SouthWest,
    Orientation::South,
    Orientation::SouthEast,
];

// subset of orientations a piece on the first rank can be threatened from
static FIRST_RANK_THREAT_ORIENTATIONS: [Orientation; 5] = [
    Orientation::East,
    Orientation::NorthEast,
    Orientation::North,
    Orientation::NorthWest,
    Orientation::West,
];

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

    // TODO: use 2d array struct
    // Cardinal direction from from to to if it exists, empty if not
    // Indexed as from * 64 + to
    rel_orientation: [Option<Orientation>; 64 * 64],

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
    king_movement: [u64; 64],
}

impl MoveGen {
    // TODO: I think using new as a constructor is idiomatic, but not sure? also idk about using initialize here
    pub fn new() -> Self {
        let mut move_gen = Self {
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
            rel_orientation: [None; 4096],
            knight_movement: [0; 64],
            king_movement: [0; 64],
        };
        move_gen.initialize();
        move_gen
    }

    /// Initialize tables
    fn initialize(&mut self) {
        // initialize orthogonal mask tables
        for idx in 0..8 {
            self.mask_rank[idx] = fill_rank(idx as u8);
            self.mask_file[idx] = fill_file(idx as u8);
            self.clear_rank[idx] = !self.mask_rank[idx];
            self.clear_file[idx] = !self.mask_file[idx];
        }

        // initialize diagonal mask tables
        for idx in 0..64 {
            let diag_idx = board::diag_index(idx);
            let anti_diag_idx = board::anti_diag_index(idx);

            self.mask_diag[diag_idx as usize] |= 1 << idx;
            self.mask_anti_diag[anti_diag_idx as usize] |= 1 << idx;
        }

        // initialize ray tables
        // rectangular rays
        for rank in 0..8 {
            for file in 0..8 {
                let vertical = self.mask_file[file] & self.clear_rank[rank];
                let horizontal = self.mask_rank[rank] & self.clear_file[file];
                let idx = board::square_index(rank as u8, file as u8) as usize;

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

        // rel orientation
        for from in 0..64 {
            for ori in Orientation::ortho() {
                for to in serialize_board(ori.ray(&self, from)) {
                    self.rel_orientation[from * 64 + to as usize] = Some(ori);
                }
            }

            for ori in Orientation::diag() {
                for to in serialize_board(ori.ray(&self, from)) {
                    self.rel_orientation[from * 64 + to as usize] = Some(ori);
                }
            }
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

    // TODO this overflows?? add test
    fn north_west_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mask = empty & self.clear_file[7];
        let mut flood = (sliders << 7) & mask;
        for _ in 0..14 {
            flood |= (flood << 7) & mask;
        }
        flood
    }

    fn north_west_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        (flood << 7) & self.clear_file[7] & board.opp_pieces
    }

    fn north_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = (sliders << 8) & empty;
        for _ in 0..6 {
            flood |= (flood << 8) & empty;
        }
        flood
    }

    fn north_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        flood << 8 & board.opp_pieces
    }

    fn north_east_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mask = empty & self.clear_file[0];
        let mut flood = (sliders << 9) & mask;
        for _ in 0..14 {
            flood |= (flood << 9) & mask;
        }
        flood
    }

    fn north_east_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        (flood << 9) & self.clear_file[0] & board.opp_pieces
    }

    fn east_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mask = empty & self.clear_file[0];
        let mut flood = (sliders << 1) & mask;
        for _ in 0..6 {
            flood |= (flood << 1) & mask;
        }

        flood
    }

    fn east_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        (flood << 1) & self.clear_file[0] & board.opp_pieces
    }

    fn south_east_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mask = empty & self.clear_file[0];
        let mut flood = (sliders >> 7) & mask;
        for _ in 0..14 {
            flood |= (flood >> 7) & mask;
        }
        flood
    }

    fn south_east_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        (flood >> 7) & self.clear_file[0] & board.opp_pieces
    }

    pub fn south_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mut flood = (sliders >> 8) & empty;
        for _ in 0..6 {
            flood |= (flood >> 8) & empty;
        }
        flood
    }

    pub fn south_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        flood >> 8 & board.opp_pieces
    }

    fn south_west_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mask = empty & self.clear_file[7];
        let mut flood = (sliders >> 9) & mask;
        for _ in 0..14 {
            flood |= (flood >> 9) & mask;
        }
        flood
    }

    fn south_west_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        (flood >> 9) & self.clear_file[7] & board.opp_pieces
    }

    fn west_moves(&self, sliders: u64, empty: u64) -> u64 {
        let mask = empty & self.clear_file[7];
        let mut flood = (sliders >> 1) & mask;
        for _ in 0..6 {
            flood |= (flood >> 1) & mask;
        }
        flood
    }

    fn west_captures(&self, sliders: u64, board: &board::Board) -> u64 {
        let mut flood = sliders;
        (flood >> 1) & self.clear_file[7] & board.opp_pieces
    }

    pub fn fill(&self, orientation: &Orientation, sliders: u64, empty: u64) -> u64 {
        match orientation {
            Orientation::North => self.north_moves(sliders, empty),
            Orientation::NorthEast => self.north_east_moves(sliders, empty),
            Orientation::East => self.east_moves(sliders, empty),
            Orientation::SouthEast => self.south_east_moves(sliders, empty),
            Orientation::South => self.south_moves(sliders, empty),
            Orientation::SouthWest => self.south_west_moves(sliders, empty),
            Orientation::West => self.west_moves(sliders, empty),
            Orientation::NorthWest => self.north_west_moves(sliders, empty),
        }
    }

    // TODO: rename to step or somethjing like that
    // Only propagates by one step so ie north west would give some pawn captures
    fn sliding_captures(
        &self,
        orientation: &Orientation,
        sliders: u64,
        board: &board::Board,
    ) -> u64 {
        match orientation {
            Orientation::North => self.north_captures(sliders, board),
            Orientation::NorthEast => self.north_east_captures(sliders, board),
            Orientation::East => self.east_captures(sliders, board),
            Orientation::SouthEast => self.south_east_captures(sliders, board),
            Orientation::South => self.south_captures(sliders, board),
            Orientation::SouthWest => self.south_west_captures(sliders, board),
            Orientation::West => self.west_captures(sliders, board),
            Orientation::NorthWest => self.north_west_captures(sliders, board),
        }
    }

    // =================================
    //         PAWN MOVE GEN
    // =================================

    /// Return possible double and single pawn pushes, and promotions
    pub fn pawn_pushes(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let own_pawns = board.own_pieces & board.pawns & self.clear_rank[0];
        let empty = board.empty();

        let single_pushes = (own_pawns << 8) & empty;
        let normal_single_pushes = single_pushes & self.clear_rank[7];
        let promotions = single_pushes & self.mask_rank[7];
        let double_pushes = ((((own_pawns & self.mask_rank[1]) << 8) & empty) << 8) & empty;

        for (from_idx, to_idx) in self
            .parse_sliding_moves(normal_single_pushes, own_pawns, Orientation::North)
            .iter()
        {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: None,
                category: MoveCategory::Normal,
                check: false,
                double_check: false,
            })
        }

        for (from_idx, to_idx) in self
            .parse_sliding_moves(promotions, own_pawns, Orientation::North)
            .iter()
        {
            for promotion in PROMOTION_OPTIONS {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: promotion,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Promotion,
                    check: false,
                    double_check: false,
                })
            }
        }

        for (from_idx, to_idx) in self
            .parse_sliding_moves(double_pushes, own_pawns, Orientation::North)
            .iter()
        {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: None,
                category: MoveCategory::DoublePawnPush,
                check: false,
                double_check: false,
            })
        }

        move_list
    }

    // Return possible captures. Handles capture promotions and en passant.
    pub fn pawn_captures(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let own_pawns = board.own_pieces & board.pawns & board::CLEAR_FIRST_LAST_RANK;
        let opp_pawns = board.opp_pieces & board.pawns & board::CLEAR_FIRST_LAST_RANK;

        let right_captures = ((own_pawns << 9) & self.clear_file[0]) & board.opp_pieces;
        let normal_right_captures = self.parse_sliding_moves(
            right_captures & self.clear_rank[7],
            own_pawns,
            Orientation::NorthEast,
        );
        let right_capture_promotions = self.parse_sliding_moves(
            right_captures & self.mask_rank[7],
            own_pawns,
            Orientation::NorthEast,
        );

        let left_captures = ((own_pawns << 7) & self.clear_file[7]) & board.opp_pieces;
        let normal_left_captures = self.parse_sliding_moves(
            left_captures & self.clear_rank[7],
            own_pawns,
            Orientation::NorthWest,
        );

        let left_capture_promotions = self.parse_sliding_moves(
            left_captures & self.mask_rank[7],
            own_pawns,
            Orientation::NorthWest,
        );

        for (from_idx, to_idx) in normal_right_captures.iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board.identify(*to_idx)),
                category: MoveCategory::Normal,
                check: false,
                double_check: false,
            })
        }

        for (from_idx, to_idx) in right_capture_promotions.iter() {
            for piece in PROMOTION_OPTIONS {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Promotion,
                    check: false,
                    double_check: false,
                })
            }
        }

        for (from_idx, to_idx) in normal_left_captures.iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board.identify(*to_idx)),
                category: MoveCategory::Normal,
                check: false,
                double_check: false,
            })
        }

        for (from_idx, to_idx) in left_capture_promotions.iter() {
            for piece in PROMOTION_OPTIONS {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Promotion,
                    check: false,
                    double_check: false,
                })
            }
        }

        let opp_ep_pawns = board.pawns & self.mask_rank[7];

        let right_ep_move = (own_pawns << 25) & self.clear_file[0] & self.mask_rank[7];
        if right_ep_move & opp_ep_pawns != 0 {
            let (from_idx, to_idx) = self.parse_ep_capture_move(right_ep_move, Orientation::East);
            move_list.push(Move {
                from: from_idx,
                to: to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board::Piece::Pawn),
                category: MoveCategory::EnPassant,
                check: false,
                double_check: false,
            })
        }

        let left_ep_move = (own_pawns << 23) & self.clear_file[7] & self.mask_rank[7];
        if left_ep_move & opp_ep_pawns != 0 {
            let (from_idx, to_idx) = self.parse_ep_capture_move(left_ep_move, Orientation::West);
            move_list.push(Move {
                from: from_idx,
                to: to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board::Piece::Pawn),
                category: MoveCategory::EnPassant,
                check: false,
                double_check: false,
            })
        }

        move_list
    }

    // =================================
    //        KNIGHT MOVE GEN
    // =================================

    pub fn knight_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();
        let own_knights = board.knights() & board.own_pieces;
        let empty = board.empty();
        let color = board.color();

        for knight_idx in serialize_board(own_knights).iter() {
            let movement = self.knight_movement[*knight_idx as usize];
            let moves = movement & empty;
            let captures = movement & board.opp_pieces;

            for (from_idx, to_idx) in self.parse_single_piece_moves(moves, *knight_idx).iter() {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Knight,
                    color,
                    capture: None,
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self.parse_single_piece_moves(captures, *knight_idx).iter() {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Knight,
                    color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }
        }

        move_list
    }

    // =================================
    //        KING MOVE GEN
    // =================================

    pub fn king_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();
        let king_idx = board.own_king();
        let empty = board.empty();
        let color = board.color();

        let movement = self.king_movement[king_idx as usize];
        let moves = movement & empty;
        let captures = movement & board.opp_pieces;

        for (from_idx, to_idx) in self.parse_single_piece_moves(moves, king_idx).iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::King,
                color,
                capture: None,
                category: MoveCategory::Normal,
                check: false,
                double_check: false,
            })
        }

        for (from_idx, to_idx) in self.parse_single_piece_moves(captures, king_idx).iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::King,
                color,
                capture: Some(board.identify(*to_idx)),
                category: MoveCategory::Normal,
                check: false,
                double_check: false,
            })
        }

        move_list
    }

    // Pseudo-legal castling move generation
    // Returned moves describe rook moves
    pub fn castling_moves(&self, board: &board::Board) -> Vec<Move> {
        debug_assert!(!board.own_king_checked());
        debug_assert!(!board.own_king_double_checked());

        let mut move_list = Vec::new();

        let occupied = board.own_pieces | board.opp_pieces;

        if board.own_castling_rights.kingside {
            let kingside_clearance = (1 << 5) | (1 << 6);

            if (occupied & kingside_clearance) == 0
                && !self.threatened(5, &FIRST_RANK_THREAT_ORIENTATIONS, board)
                && !self.threatened(6, &FIRST_RANK_THREAT_ORIENTATIONS, board)
            {
                move_list.push(Move {
                    from: 7,
                    to: 5,
                    piece: board::Piece::Rook,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::KingsideCastle,
                    check: false,
                    double_check: false,
                })
            }
        }
        if board.own_castling_rights.queenside {
            let queenside_clearance = (1 << 1) | (1 << 2) | (1 << 3);

            if (occupied & queenside_clearance) == 0
                && !self.threatened(3, &FIRST_RANK_THREAT_ORIENTATIONS, board)
                && !self.threatened(2, &FIRST_RANK_THREAT_ORIENTATIONS, board)
            {
                move_list.push(Move {
                    from: 0,
                    to: 3,
                    piece: board::Piece::Rook,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::QueensideCastle,
                    check: false,
                    double_check: false,
                })
            }
        }

        move_list
    }

    // =================================
    //         SLIDING MOVE GEN
    // =================================

    fn ortho_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list: Vec<Move> = Vec::new();

        let pieces = board.ortho_sliders & board.own_pieces;

        if pieces == 0 {
            return move_list;
        }

        let empty = board.empty();
        let color = board.color();

        for orientation in Orientation::ortho() {
            let axis = orientation.axis();

            let moves = self.fill(&orientation, pieces, empty);
            let captures = self.sliding_captures(&orientation, moves | pieces, board);

            let queens = board.queens() & board.own_pieces;
            let rooks = board.rooks() & board.own_pieces;

            let mut queen_mask: u64 = 0;
            for queen_idx in serialize_board(queens) {
                let queen_axis_idx = axis.axis_idx(queen_idx);
                queen_mask |= axis.mask(queen_axis_idx.into(), &self);
            }
            let rook_mask = !queen_mask;

            let queen_moves = moves & queen_mask;
            let queen_captures = captures & queen_mask;
            let rook_moves = moves & rook_mask;
            let rook_captures = captures & rook_mask;

            for (from_idx, to_idx) in self
                .parse_sliding_moves(queen_moves, queens, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color,
                    capture: None,
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self
                .parse_sliding_moves(queen_captures, queens, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self
                .parse_sliding_moves(rook_moves, rooks, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Rook,
                    color,
                    capture: None,
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self
                .parse_sliding_moves(rook_captures, rooks, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Rook,
                    color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }
        }

        move_list
    }

    fn diag_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list: Vec<Move> = Vec::new();

        let pieces = board.diag_sliders & board.own_pieces;

        if pieces == 0 {
            return move_list;
        }

        let empty = board.empty();
        let color = board.color();

        for orientation in Orientation::diag() {
            let axis = orientation.axis();

            let moves = self.fill(&orientation, pieces, empty);
            let captures = self.sliding_captures(&orientation, moves | pieces, board);

            let queens = board.queens() & board.own_pieces;
            let bishops = board.bishops() & board.own_pieces;

            let mut queen_mask: u64 = 0;
            for queen_idx in serialize_board(queens) {
                let queen_axis_idx = axis.axis_idx(queen_idx);
                queen_mask |= axis.mask(queen_axis_idx.into(), &self);
            }
            let bishop_mask = !queen_mask;

            let queen_moves = moves & queen_mask;
            let queen_captures = captures & queen_mask;
            let bishop_moves = moves & bishop_mask;
            let bishop_captures = captures & bishop_mask;

            for (from_idx, to_idx) in self
                .parse_sliding_moves(queen_moves, queens, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color,
                    capture: None,
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self
                .parse_sliding_moves(queen_captures, queens, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self
                .parse_sliding_moves(bishop_moves, bishops, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Bishop,
                    color,
                    capture: None,
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }

            for (from_idx, to_idx) in self
                .parse_sliding_moves(bishop_captures, bishops, orientation)
                .iter()
            {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Bishop,
                    color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                    check: false,
                    double_check: false,
                })
            }
        }

        move_list
    }

    // TODO: I have a hard time believing that even an optimized version of this
    // will be faster than simply generating sliding moves independently for each piece.
    // I also haven't really optimized this at all
    //
    // Assumes pieces is not empty
    fn parse_sliding_moves(
        &self,
        moves: u64,
        pieces: u64,
        orientation: Orientation,
    ) -> Vec<(u8, u8)> {
        let mut move_list: Vec<(u8, u8)> = Vec::new();

        if moves == 0 {
            return move_list;
        }

        let axis = orientation.axis();
        let ortho_axis = axis.orthogonal_axis();

        let mut piece_axis_idxs: Vec<u8> = Vec::new();
        for sq in serialize_board(pieces) {
            piece_axis_idxs.push(axis.axis_idx(sq));
        }

        let (singleton_axes, colinear_axes) = utils::partition_unique(piece_axis_idxs);

        // special handling for colinear pieces, must mask moves
        for axis_idx in colinear_axes.into_iter() {
            let mut piece_idxs = serialize_board(pieces & axis.mask(axis_idx.into(), self));

            // sort by position along axis
            piece_idxs.sort_by_cached_key(|&i| ortho_axis.axis_idx(i));

            // now some tricky orientation dependent stuff.
            // possibly reverse list to ensure mask will always be non-empty
            // sign of shift corresponds to direction of increasing axis idx

            if orientation.shift() < 0 {
                piece_idxs.reverse();
            }

            for (piece_1, piece_2) in iter::zip(
                piece_idxs[..(piece_idxs.len() - 1)].iter(),
                piece_idxs[1..].iter(),
            ) {
                let piece_mask = orientation.ray(&self, *piece_1 as usize)
                    & orientation.antipode().ray(&self, *piece_2 as usize);

                move_list.append(&mut self.parse_single_piece_moves(moves & piece_mask, *piece_1));
            }

            // handle last piece
            let last_piece = piece_idxs[piece_idxs.len() - 1];
            move_list.append(&mut self.parse_single_piece_moves(
                moves & orientation.ray(&self, last_piece as usize),
                last_piece,
            ));
        }

        for axis_idx in singleton_axes {
            let axis_mask = axis.mask(axis_idx.into(), self);
            let piece_idx = serialize_board(pieces & axis_mask).pop().unwrap();

            move_list.append(&mut self.parse_single_piece_moves(moves & axis_mask, piece_idx));
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

    // Parse en passant capture bitboard. Assumes moves is not empty.
    // Returned (from_idx, to_idx) describe movement of capturing piece
    fn parse_ep_capture_move(&self, moves: u64, orientation: Orientation) -> (u8, u8) {
        let to_idx = serialize_board(moves)[0] - 16;

        let from_idx = match orientation {
            Orientation::West => to_idx - 9,
            Orientation::East => to_idx - 7,
            // TODO: what error should I throw for the catch-all case?
            _ => panic!("Disallowed value"),
        };

        (from_idx, to_idx)
    }

    // ========================
    //   PSEUDO-LEGAL MOVE GEN
    // ========================

    /// Generate all pseudo-legal moves
    pub fn pseudo_legal_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        // =================
        //    PAWN MOVES
        // =================

        move_list.append(&mut self.pawn_pushes(&board));
        move_list.append(&mut self.pawn_captures(&board));

        // =================
        //  SLIDING MOVES
        // =================

        move_list.append(&mut self.ortho_moves(&board));
        move_list.append(&mut self.diag_moves(&board));

        // =================
        //    KNIGHT MOVES
        // =================

        move_list.append(&mut self.knight_moves(&board));

        // =================
        //    KING MOVES
        // =================

        move_list.append(&mut self.king_moves(&board));
        move_list.append(&mut self.castling_moves(&board));

        move_list
    }

    // ========================
    //     LEGAL MOVE GEN
    // ========================

    // Explicitly checks if own king is currently in check
    fn in_check(&self, board: &board::Board) -> bool {
        self.threatened(board.own_king(), &ORIENTATIONS, board)
    }

    // Checks if sq is threatened by enemies given current state of board
    // Orientations are relative to sq (ie north for vertical slider threats for a piece on the first rank)
    fn threatened(&self, sq: u8, orientations: &[Orientation], board: &board::Board) -> bool {
        if self.knight_movement[sq as usize] & board.knights() & board.opp_pieces != 0 {
            return true;
        }

        for orientation in orientations {
            if self.threatened_in(sq, orientation, board) {
                return true;
            }
        }

        false
    }

    // Checks if sq is threatened by enemy sliders or pawns in given direction
    // Orientations are relative to sq (ie north for vertical slider threats for a piece on the first rank)
    fn threatened_in(&self, sq: u8, orientation: &Orientation, board: &board::Board) -> bool {
        // pawn threats
        match orientation {
            Orientation::NorthWest => {
                let pawns = board.pawns & board::CLEAR_FIRST_LAST_RANK & board.opp_pieces;
                if self.north_west_captures(1 << sq, &board) & pawns != 0 {
                    return true;
                }
            }
            Orientation::NorthEast => {
                let pawns = board.pawns & board::CLEAR_FIRST_LAST_RANK & board.opp_pieces;
                if self.north_east_captures(1 << sq, &board) & pawns != 0 {
                    return true;
                }
            }
            _ => (),
        }


        // slider threats
        let enemy_sliders = board.sliders(&orientation) & board.opp_pieces;
        // excluding enemy sliders from mask saves a call to sliding_captures and a few instructions (?)
        let mask = board.empty() ^ enemy_sliders;
        (self.fill(&orientation, 1 << sq, mask) & enemy_sliders) != 0
    }

    // Returns true if there are discover attacks on opp due to own move (for general sq)
    // A move directly along the ray towards sq where moving piece threatens sq is not considered a discover attack
    // i.e, enemy king on e8, two own rooks an e1 e2, move e2e4 is not a discover attack on e8
    fn is_discover_attack(&self, sq: u8, m: &Move, board: &board::Board) -> bool {
        match m.category {
            MoveCategory::KingsideCastle => return false,
            MoveCategory::QueensideCastle => return false,
            MoveCategory::EnPassant => {
                let mut ep_threat = false;
                // check orientation exposed by movement of own pawn
                let rel_orientation = self.rel_orientation[(sq as usize) * 64 + (m.from as usize)];
                if let Some(orientation) = rel_orientation {
                    ep_threat |= self.is_discover_attack_in(
                        sq,
                        (1 << m.from) | (1 << m.to),
                        (1 << (m.to - 8)),
                        &orientation,
                        board,
                    )
                }

                // check orientation exposed by now captured enemy pawn
                let rel_orientation =
                    self.rel_orientation[(sq as usize) * 64 + ((m.to - 8) as usize)];
                if let Some(orientation) = rel_orientation {
                    ep_threat |= self.is_discover_attack_in(
                        sq,
                        (1 << m.from) | (1 << m.to),
                        !(1 << (m.to - 8)),
                        &orientation,
                        board,
                    )
                }

                return ep_threat;
            }
            _ => match m.capture {
                // capture move case
                Some(_) => {
                    let rel_orientation =
                        self.rel_orientation[(sq as usize) * 64 + (m.from as usize)];
                    if let Some(orientation) = rel_orientation {
                        return self.is_discover_attack_in(
                            sq,
                            (1 << m.from) | (1 << m.to),
                            (1 << m.to),
                            &orientation,
                            board,
                        );
                    }
                }
                // normal move case
                None => {
                    let rel_orientation =
                        self.rel_orientation[(sq as usize) * 64 + (m.from as usize)];
                    if let Some(orientation) = rel_orientation {
                        return self.is_discover_attack_in(
                            sq,
                            (1 << m.from) | (1 << m.to),
                            0,
                            &orientation,
                            board,
                        );
                    }
                }
            },
        }
        false
    }

    // Checks if sq is exposed to a discover attack from own pieces in given direction
    // Orientations are relative to own piece (ie north for vertical slider threats for a piece on first rank)
    fn is_discover_attack_in(
        &self,
        sq: u8,
        own_move: u64,
        capture: u64,
        orientation: &Orientation,
        board: &board::Board,
    ) -> bool {
        // mask off both from and to bits of own_move
        // even if moving piece is a slider, threats from moving piece are not counted as discover attacks
        let own_sliders = board.sliders(&orientation) & board.own_pieces & !(own_move);
        // excluding own sliders from mask saves a call to sliding_captures and a few instructions (?)
        let mask = board.empty() ^ own_move ^ own_sliders ^ capture;
        (self.fill(&orientation, 1 << sq, mask) & own_sliders) != 0
    }

    // TODO: handle moves where we start in check, I think need to do a full make move for those
    // TODO: small optimization in separately handling castling moves
    // TODO: complete rewrite
    pub fn legal_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut l_moves = Vec::new();

        let pl_moves = self.pseudo_legal_moves(board);

        // TODO: need special special handling for EP double checks
        // can have it such that EP capturing pawn does not check king but causes double check anyways

        // TODO: king-king checks when castling
        // TODO: handle rook checks when castling, no discover attacks I think

        l_moves
    }
}

// Tally all legal moves up to depth
pub fn perft(mut board: board::Board, depth: usize) -> u64 {
    let mut nodes = 0;
    let move_gen = MoveGen::new();

    // (move, depth)
    let mut move_stack: Vec<(Move, usize)> = Vec::new();
    // (move, undo)
    let mut undo_stack: Vec<(Move, board::UndoInfo)> = Vec::new();

    let moves = move_gen.legal_moves(&board);
    move_stack.extend(moves.into_iter().zip(iter::repeat(1)));

    while !move_stack.is_empty() {
        let (m, d) = move_stack.pop().unwrap();

        // pop positions until we're behind the next move
        while d <= undo_stack.len() {
            let (mp, up) = undo_stack.pop().unwrap();
            board.unmake_move(&mp, &up);
        }

        let u = board.make_move(&m);
        undo_stack.push((m, u));

        // only count leaf nodes
        if d == depth {
            nodes += 1;
        }

        if d < depth {
            let moves = move_gen.legal_moves(&board);
            move_stack.extend(moves.into_iter().zip(iter::repeat(d + 1)));
        }
    }

    nodes
}

/// Fill rank at rank_idx
fn fill_rank(rank_idx: u8) -> u64 {
    assert!(rank_idx < 8);

    let result: u64 = board::FIRST_RANK;
    result << (rank_idx * 8)
}

/// Fill file at file_idx
fn fill_file(file_idx: u8) -> u64 {
    assert!(file_idx < 8);

    let result: u64 = board::A_FILE;
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

// TODO: maybe this should be in board? idk
/// Return square index of first set bit
/// If no bit is set returns None
pub fn bitscan_lsd(state: u64) -> Option<u8> {
    let trailing = state.trailing_zeros() as u8;
    if trailing == 64 {
        return None;
    } else {
        return Some(trailing);
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
pub fn pop_count(state: u64) -> u8 {
    serialize_board(state).len() as u8
}

// TODO: I don't think color is necessary
#[derive(Debug, Clone)]
pub struct Move {
    pub from: u8, // integer 0-63
    pub to: u8,   // integer 0-63
    pub piece: board::Piece,
    pub color: board::Color,
    pub capture: Option<board::Piece>,
    pub category: MoveCategory,
    // follows same convention as board state check bits
    pub check: bool,
    pub double_check: bool,
}

#[derive(PartialEq, Debug, Clone)]
pub enum MoveCategory {
    Normal,
    QueensideCastle,
    KingsideCastle,
    EnPassant,
    DoublePawnPush,
    Promotion,
}

#[derive(Debug)]
pub enum Axis {
    // horizontal
    Rank,
    // vertical
    File,
    Diagonal,
    AntiDiagonal,
}

impl Axis {
    // Dispatches onto appropriate axis index method
    fn axis_idx(&self, sq: u8) -> u8 {
        match self {
            Axis::Rank => board::rank_index(sq),
            Axis::File => board::file_index(sq),
            Axis::Diagonal => board::diag_index(sq),
            Axis::AntiDiagonal => board::anti_diag_index(sq),
        }
    }

    // TODO: rename to mask
    // Dispatches onto appropriate axis mask
    fn mask(&self, axis_idx: usize, move_gen: &MoveGen) -> u64 {
        match self {
            Axis::Rank => move_gen.mask_rank[axis_idx],
            Axis::File => move_gen.mask_file[axis_idx],
            Axis::Diagonal => move_gen.mask_diag[axis_idx],
            Axis::AntiDiagonal => move_gen.mask_anti_diag[axis_idx],
        }
    }

    fn orthogonal_axis(&self) -> Self {
        match self {
            Axis::Rank => Axis::File,
            Axis::File => Axis::Rank,
            Axis::Diagonal => Axis::AntiDiagonal,
            Axis::AntiDiagonal => Axis::Diagonal,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Orientation {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
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
            Orientation::NorthWest => Orientation::SouthEast,
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
            Orientation::NorthWest => Axis::AntiDiagonal,
        }
    }

    fn ortho() -> [Orientation; 4] {
        [
            Orientation::North,
            Orientation::East,
            Orientation::South,
            Orientation::West,
        ]
    }

    fn diag() -> [Orientation; 4] {
        [
            Orientation::NorthEast,
            Orientation::SouthEast,
            Orientation::SouthWest,
            Orientation::NorthWest,
        ]
    }

    fn ray(&self, move_gen: &MoveGen, sq_idx: usize) -> u64 {
        match self {
            Orientation::North => return move_gen.north[sq_idx],
            Orientation::NorthEast => return move_gen.north_east[sq_idx],
            Orientation::East => return move_gen.east[sq_idx],
            Orientation::SouthEast => return move_gen.south_east[sq_idx],
            Orientation::South => return move_gen.south[sq_idx],
            Orientation::SouthWest => return move_gen.south_west[sq_idx],
            Orientation::West => return move_gen.west[sq_idx],
            Orientation::NorthWest => return move_gen.north_west[sq_idx],
        }
    }

    fn shift(&self) -> i8 {
        match self {
            Orientation::North => 8,
            Orientation::NorthEast => 9,
            Orientation::East => 1,
            Orientation::SouthEast => -7,
            Orientation::South => -8,
            Orientation::SouthWest => -9,
            Orientation::West => -1,
            Orientation::NorthWest => 7,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pawns(own_pawns: Vec<u8>, opp_pawns: Vec<u8>) -> board::Board {
        let mut b = board::Board::empty_board();
        for own_pawn in own_pawns.iter() {
            b.own_pieces |= 1 << own_pawn;
            b.pawns |= 1 << own_pawn;
        }

        for opp_pawn in opp_pawns.iter() {
            b.opp_pieces |= 1 << opp_pawn;
            b.pawns |= 1 << opp_pawn;
        }

        b
    }

    // TODO: could probably avoid the duplicated logic by turning the other one into an iterator?
    // Returns:
    //  - nodes
    //  - captures
    //  - eps
    //  - castles
    //  - promotions
    fn perft_debug(depth: usize) -> (u64, u64, u64, u64, u64) {
        let mut nodes = 0;
        let mut captures = 0;
        let mut eps = 0;
        let mut castles = 0;
        let mut promotions = 0;

        let mut board = board::Board::new();
        let move_gen = MoveGen::new();

        // (move, depth)
        let mut move_stack: Vec<(Move, usize)> = Vec::new();
        // (move, undo)
        let mut undo_stack: Vec<(Move, board::UndoInfo)> = Vec::new();

        let moves = move_gen.legal_moves(&board);
        move_stack.extend(moves.into_iter().zip(iter::repeat(1)));

        while !move_stack.is_empty() {
            let (m, d) = move_stack.pop().unwrap();

            // pop positions until we're behind the next move
            while d <= undo_stack.len() {
                let (mp, up) = undo_stack.pop().unwrap();
                board.unmake_move(&mp, &up);
            }

            let u = board.make_move(&m);

            // only count leaf nodes
            if d == depth {
                nodes += 1;

                match m.category {
                    MoveCategory::QueensideCastle => castles += 1,
                    MoveCategory::KingsideCastle => castles += 1,
                    MoveCategory::EnPassant => eps += 1,
                    MoveCategory::Promotion => promotions += 1,
                    _ => (),
                }

                if m.capture.is_some() {
                    captures += 1;
                }
            }

            undo_stack.push((m, u));

            if d < depth {
                let moves = move_gen.legal_moves(&board);
                move_stack.extend(moves.into_iter().zip(iter::repeat(d + 1)));
            }
        }

        (nodes, captures, eps, castles, promotions)
    }

    #[test]
    fn test_fill_rank_0() {
        assert_eq!(fill_rank(0), board::FIRST_RANK);
    }

    #[test]
    fn test_fill_file_0() {
        assert_eq!(fill_file(0), board::A_FILE)
    }

    #[test]
    fn test_bitscan_lsd() {
        assert_eq!(Some(1), bitscan_lsd(1 << 1));
        assert_eq!(Some(63), bitscan_lsd(1 << 63));
        assert_eq!(Some(1), bitscan_lsd((1 << 1) ^ (1 << 5)));
        assert_eq!(None, bitscan_lsd(0));
    }

    #[test]
    fn test_serialize_board() {
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

    #[test]
    fn single_pawn_second_rank_push() {
        let move_gen = MoveGen::new();

        for file_idx in 0..8 {
            let b = pawns(vec![board::square_index(1, file_idx)], Vec::new());
            assert_eq!(move_gen.pawn_pushes(&b).len(), 2);
        }
    }

    #[test]
    fn single_pawn_other_rank_push() {
        let move_gen = MoveGen::new();

        for rank_idx in 3..6 {
            for file_idx in 0..8 {
                let b = pawns(vec![board::square_index(rank_idx, file_idx)], Vec::new());
                assert_eq!(move_gen.pawn_pushes(&b).len(), 1);
            }
        }
    }

    #[test]
    fn two_pawn_second_rank_push() {
        let move_gen = MoveGen::new();

        for file_idx in 0..6 {
            let b = pawns(
                vec![
                    board::square_index(1, file_idx),
                    board::square_index(1, file_idx + 1),
                ],
                Vec::new(),
            );
            assert_eq!(move_gen.pawn_pushes(&b).len(), 4);
        }
    }

    #[test]
    fn two_pawn_other_rank_push() {
        let move_gen = MoveGen::new();

        for rank_idx in 3..6 {
            for file_idx in 0..6 {
                let b = pawns(
                    vec![
                        board::square_index(rank_idx, file_idx),
                        board::square_index(rank_idx, file_idx + 1),
                    ],
                    Vec::new(),
                );
                assert_eq!(move_gen.pawn_pushes(&b).len(), 2);
            }
        }
    }

    #[test]
    fn single_pawn_promotion() {
        let move_gen = MoveGen::new();

        for file_idx in 0..8 {
            let b = pawns(vec![board::square_index(6, file_idx)], Vec::new());

            let moves = move_gen.pawn_pushes(&b);
            assert_eq!(moves.len(), 4);

            for m in moves.iter() {
                assert_eq!(m.category, MoveCategory::Promotion);
            }
        }
    }

    #[test]
    fn two_pawn_promotion() {
        let move_gen = MoveGen::new();

        for file_idx in 0..6 {
            let b = pawns(
                vec![
                    board::square_index(6, file_idx),
                    board::square_index(6, file_idx + 1),
                ],
                Vec::new(),
            );

            let moves = move_gen.pawn_pushes(&b);
            assert_eq!(moves.len(), 8);

            for m in moves.iter() {
                assert_eq!(m.category, MoveCategory::Promotion);
            }
        }
    }

    #[test]
    fn one_pawn_normal_capture() {
        let move_gen = MoveGen::new();

        for rank_idx in 1..5 {
            for file_idx in 1..6 {
                let b = pawns(
                    vec![board::square_index(rank_idx, file_idx)],
                    vec![
                        board::square_index(rank_idx + 1, file_idx - 1),
                        board::square_index(rank_idx + 1, file_idx + 1),
                    ],
                );

                let moves = move_gen.pawn_captures(&b);
                assert_eq!(
                    moves.len(),
                    2,
                    "rank_idx: {rank_idx}, file_idx: {file_idx}, moves: {moves:#?} \n{b:#?}"
                );

                for m in moves.iter() {
                    assert_eq!(m.capture, Some(board::Piece::Pawn));
                }
            }
        }
    }

    #[test]
    fn two_pawn_normal_capture() {
        let move_gen = MoveGen::new();

        for rank_idx in 1..5 {
            for file_idx in 1..5 {
                let b = pawns(
                    vec![
                        board::square_index(rank_idx, file_idx),
                        board::square_index(rank_idx, file_idx + 1),
                    ],
                    vec![
                        board::square_index(rank_idx + 1, file_idx - 1),
                        board::square_index(rank_idx + 1, file_idx),
                        board::square_index(rank_idx + 1, file_idx + 1),
                        board::square_index(rank_idx + 1, file_idx + 2),
                    ],
                );

                let moves = move_gen.pawn_captures(&b);
                assert_eq!(
                    moves.len(),
                    4,
                    "rank_idx: {rank_idx}, file_idx: {file_idx}, moves: {moves:#?}"
                );

                for m in moves.iter() {
                    assert_eq!(m.capture, Some(board::Piece::Pawn));
                }
            }
        }
    }

    #[test]
    fn one_pawn_normal_capture_wrap_l() {
        let move_gen = MoveGen::new();

        for rank_idx in 1..5 {
            let b = pawns(
                vec![board::square_index(rank_idx, 0)],
                vec![
                    board::square_index(rank_idx, 7),
                    board::square_index(rank_idx + 1, 1),
                ],
            );

            let moves = move_gen.pawn_captures(&b);
            assert_eq!(moves.len(), 1, "moves: {moves:#?}");

            for m in moves.iter() {
                assert_eq!(m.capture, Some(board::Piece::Pawn));
            }
        }
    }

    #[test]
    fn one_pawn_normal_capture_wrap_r() {
        let move_gen = MoveGen::new();

        for rank_idx in 1..5 {
            let b = pawns(
                vec![board::square_index(rank_idx, 7)],
                vec![
                    board::square_index(rank_idx + 1, 6),
                    board::square_index(rank_idx + 2, 0),
                ],
            );

            let moves = move_gen.pawn_captures(&b);
            assert_eq!(moves.len(), 1, "moves: {moves:#?}");

            for m in moves.iter() {
                assert_eq!(m.capture, Some(board::Piece::Pawn));
            }
        }
    }

    #[test]
    fn one_pawn_ep_capture() {
        let move_gen = MoveGen::new();

        for file_idx in 1..6 {
            for ep_side in [-1, 1] {
                let b = pawns(
                    vec![board::square_index(4, file_idx)],
                    vec![board::square_index(7, (file_idx as i8 + ep_side) as u8)],
                );

                let moves = move_gen.pawn_captures(&b);
                assert_eq!(moves.len(), 1, "moves: {moves:#?}");

                for m in moves.iter() {
                    assert_eq!(m.capture, Some(board::Piece::Pawn));
                    assert_eq!(m.category, MoveCategory::EnPassant);
                }
            }
        }
    }

    #[test]
    fn two_pawn_ep_capture() {
        let move_gen = MoveGen::new();

        for file_idx in 1..6 {
            let b = pawns(
                vec![
                    board::square_index(4, file_idx - 1),
                    board::square_index(4, file_idx + 1),
                ],
                vec![board::square_index(7, file_idx)],
            );

            let moves = move_gen.pawn_captures(&b);
            assert_eq!(moves.len(), 2, "moves: {moves:#?}");

            for m in moves.iter() {
                assert_eq!(m.capture, Some(board::Piece::Pawn));
                assert_eq!(m.category, MoveCategory::EnPassant);
            }
        }
    }

    #[test]
    fn one_pawn_promotion_capture() {
        let move_gen = MoveGen::new();

        // check for allowed captures
        for file_idx in 1..6 {
            for capture_piece in [
                board::Piece::Bishop,
                board::Piece::Rook,
                board::Piece::Queen,
                board::Piece::Knight,
                board::Piece::King,
            ] {
                for capture_side in [-1, 1] {
                    let mut b = pawns(vec![board::square_index(6, file_idx)], Vec::new());
                    b.set_piece(
                        false,
                        board::square_index(7, (file_idx as i8 + capture_side) as u8),
                        capture_piece,
                    );

                    let moves = move_gen.pawn_captures(&b);
                    assert_eq!(moves.len(), 4);
                    for m in moves.iter() {
                        assert_eq!(m.capture, Some(capture_piece));
                        assert_eq!(m.category, MoveCategory::Promotion);
                    }
                }
            }
        }

        // check nothing weird happens with ep bits
        for file_idx in 1..6 {
            for capture_side in [-1, 1] {
                let mut b = pawns(vec![board::square_index(6, file_idx)], Vec::new());
                let ep_bit_sq = board::square_index(7, (file_idx as i8 + capture_side) as u8);
                b.pawns |= 1 << ep_bit_sq;

                let moves = move_gen.pawn_captures(&b);
                assert_eq!(moves.len(), 0);
            }
        }
    }

    #[test]
    fn sliding_ortho_moves_single() {
        let move_gen = MoveGen::new();

        for sq in 0..63 {
            let mut b = board::Board::empty_board();
            b.set_piece(true, sq as u8, board::Piece::Queen);

            let moves = move_gen.ortho_moves(&b);

            let expected = pop_count(
                move_gen.north[sq] | move_gen.east[sq] | move_gen.south[sq] | move_gen.west[sq],
            );
            let rank_idx = board::rank_index(sq as u8);
            let file_idx = board::file_index(sq as u8);
            assert_eq!(
                moves.len(),
                expected as usize,
                "rank_idx: {rank_idx}, file_idx: {file_idx}, moves: {moves:#?}"
            )
        }
    }

    #[test]
    fn sliding_diag_moves_single() {
        let move_gen = MoveGen::new();

        for sq in 0..63 {
            let mut b = board::Board::empty_board();
            b.set_piece(true, sq as u8, board::Piece::Queen);

            let moves = move_gen.diag_moves(&b);

            let expected = pop_count(
                move_gen.north_east[sq]
                    | move_gen.south_east[sq]
                    | move_gen.south_west[sq]
                    | move_gen.north_west[sq],
            );
            let rank_idx = board::rank_index(sq as u8);
            let file_idx = board::file_index(sq as u8);
            assert_eq!(
                moves.len(),
                expected as usize,
                "rank_idx: {rank_idx}, file_idx: {file_idx}, moves: {moves:#?}"
            )
        }
    }

    #[test]
    fn sliding_ortho_captures_single() {
        let move_gen = MoveGen::new();

        for sq in 0..63 {
            let enemy_sqs = serialize_board(
                move_gen.north[sq] | move_gen.east[sq] | move_gen.south[sq] | move_gen.west[sq],
            );

            for enemy_sq in enemy_sqs {
                let mut b = board::Board::empty_board();
                b.set_piece(true, sq as u8, board::Piece::Queen);
                b.set_piece(false, enemy_sq as u8, board::Piece::Queen);

                let moves = move_gen.ortho_moves(&b);

                let mut captures = 0;

                for m in moves.iter() {
                    if m.capture.is_some() {
                        captures += 1;
                    }
                }

                assert_eq!(
                    captures, 1,
                    "sq: {sq}, enemy sq: {enemy_sq}, moves: {moves:#?}"
                );
            }
        }
    }

    // it is annoying to do this in general, so just north for now
    #[test]
    fn sliding_ortho_colinear_north_2() {
        let move_gen = MoveGen::new();

        for file_idx in 0..8 {
            for rank_idx in 1..8 {
                let mut b = board::Board::empty_board();
                b.set_piece(true, board::square_index(0, file_idx), board::Piece::Queen);

                b.set_piece(
                    true,
                    board::square_index(rank_idx, file_idx),
                    board::Piece::Queen,
                );

                let moves = move_gen.parse_sliding_moves(
                    move_gen.north_moves(b.queens(), b.empty()),
                    b.queens(),
                    Orientation::North,
                );

                assert_eq!(
                    moves.len(),
                    6,
                    "rank_idx: {rank_idx}, file_idx: {file_idx}, moves: {moves:#?}"
                );
            }
        }
    }

    #[test]
    fn sliding_ortho_colinear_north_3() {
        let move_gen = MoveGen::new();

        for file_idx in 0..8 {
            for rank_idx_1 in 1..8 {
                for rank_idx_2 in (rank_idx_1 + 1)..8 {
                    let mut b = board::Board::empty_board();
                    b.set_piece(true, board::square_index(0, file_idx), board::Piece::Queen);

                    b.set_piece(
                        true,
                        board::square_index(rank_idx_1, file_idx),
                        board::Piece::Queen,
                    );

                    b.set_piece(
                        true,
                        board::square_index(rank_idx_2, file_idx),
                        board::Piece::Queen,
                    );

                    let moves = move_gen.parse_sliding_moves(
                        move_gen.north_moves(b.queens(), b.empty()),
                        b.queens(),
                        Orientation::North,
                    );

                    assert_eq!(
                        moves.len(),
                        5,
                        "rank_idx_1: {rank_idx_1}, rank_idx_2: {rank_idx_2}, file_idx: {file_idx}, moves: {moves:#?}"
                    );
                }
            }
        }
    }

    #[test]
    fn threatened_by_knight() {
        let mg = MoveGen::new();
        let mut b = board::Board::empty_board();

        b.set_piece(false, 17, board::Piece::Knight);

        assert!(mg.threatened(0, &ORIENTATIONS, &b));
    }

    #[test]
    fn threatened_in() {
        let mg = MoveGen::new();
        let mut b = board::Board::empty_board();

        b.set_piece(false, 16, board::Piece::Queen);

        assert!(mg.threatened_in(0, &Orientation::North, &b));
    }

    #[test]
    fn threatened_in_occluded() {
        let mg = MoveGen::new();
        let mut b = board::Board::empty_board();

        b.set_piece(false, 16, board::Piece::Queen);
        b.set_piece(true, 16, board::Piece::Pawn);

        assert!(mg.threatened_in(0, &Orientation::North, &b));
    }

    #[test]
    fn threatened_in_pawn() {
        let mg = MoveGen::new();
        let mut b = board::Board::empty_board();

        b.set_piece(false, 8, board::Piece::Pawn);
        b.set_piece(false, 10, board::Piece::Pawn);

        assert!(mg.threatened_in(1, &Orientation::NorthEast, &b), "{:#?}", b);
        assert!(mg.threatened_in(1, &Orientation::NorthWest, &b));
    }

    #[test]
    fn threatened_in_not_rel_ray() {
        let mg = MoveGen::new();
        let mut b = board::Board::empty_board();

        b.set_piece(false, 17, board::Piece::Queen);

        assert!(!mg.threatened_in(0, &Orientation::North, &b));
    }

    // TODO: discover attack tests cases:
    // - normal move along orientation axis
    // - normal move not along orientation axis
    //  various occluded normal moves
    // - captures ''
    // whole bunch of EP cases
    // at least one where the discover is horizontal

    #[test]
    fn discover_attack_normal_move_off_axis() {
        let mg = MoveGen::new();

        let mut b = board::Board::empty_board();
        b.set_piece(true, 4, board::Piece::Queen);
        b.set_piece(true, 12, board::Piece::Rook);

        let m = Move {
            from: 12,
            to: 13,
            piece: board::Piece::Rook,
            color: board::Color::White,
            capture: None,
            category: MoveCategory::Normal,
            check: false,
            double_check: false
        };

        assert_eq!(mg.is_discover_attack(36, &m, &b), true);

        let mut b = board::Board::empty_board();
        b.set_piece(true, 2, board::Piece::Queen);
        b.set_piece(true, 12, board::Piece::Rook);

        let m = Move {
            from: 12,
            to: 13,
            piece: board::Piece::Rook,
            color: board::Color::White,
            capture: None,
            category: MoveCategory::Normal,
            check: false,
            double_check: false
        };

        assert_eq!(mg.is_discover_attack(29, &m, &b), true);
    }

    #[test]
    fn discover_attack_normal_move_on_axis() {
        let mg = MoveGen::new();

        let mut b = board::Board::empty_board();
        b.set_piece(true, 4, board::Piece::Queen);
        b.set_piece(true, 12, board::Piece::Rook);

        let m = Move {
            from: 12,
            to: 20,
            piece: board::Piece::Rook,
            color: board::Color::White,
            capture: None,
            category: MoveCategory::Normal,
            check: false,
            double_check: false
        };

        assert_eq!(mg.is_discover_attack(36, &m, &b), false);

        let mut b = board::Board::empty_board();
        b.set_piece(true, 3, board::Piece::Queen);
        b.set_piece(true, 4, board::Piece::Rook);

        let m = Move {
            from: 4,
            to: 6,
            piece: board::Piece::Rook,
            color: board::Color::White,
            category: MoveCategory::Normal,
            capture: None,
            check: false,
            double_check: false
        };

        assert_eq!(mg.is_discover_attack(7, &m, &b), false);
    }

    #[test]
    fn perft_1() {
        let (nodes, captures, eps, castles, promotions) = perft_debug(1);

        assert_eq!(captures, 0);
        assert_eq!(eps, 0);
        assert_eq!(castles, 0);
        assert_eq!(promotions, 0);
        assert_eq!(nodes, 20);
    }

    #[test]
    fn perft_2() {
        let (nodes, captures, eps, castles, promotions) = perft_debug(2);

        assert_eq!(captures, 0);
        assert_eq!(eps, 0);
        assert_eq!(castles, 0);
        assert_eq!(promotions, 0);
        assert_eq!(nodes, 400);
    }

    #[test]
    fn perft_3() {
        let (nodes, captures, eps, castles, promotions) = perft_debug(3);

        assert_eq!(captures, 34);
        assert_eq!(eps, 0);
        assert_eq!(castles, 0);
        assert_eq!(promotions, 0);
        assert_eq!(nodes, 8902);
    }

    #[test]
    #[ignore]
    fn perft_4() {
        let (nodes, captures, eps, castles, promotions) = perft_debug(4);

        assert_eq!(captures, 1576);
        assert_eq!(eps, 0);
        assert_eq!(castles, 0);
        assert_eq!(promotions, 0);
        assert_eq!(nodes, 197_281)
    }

    #[test]
    #[ignore]
    fn perft_5() {
        let (nodes, captures, eps, castles, promotions) = perft_debug(5);

        assert_eq!(captures, 82_719);
        assert_eq!(eps, 258);
        assert_eq!(castles, 0);
        assert_eq!(promotions, 0);
        assert_eq!(nodes, 4_865_609)
    }
}
