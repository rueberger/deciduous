/// Module containing all move generation logic
use crate::board;
use crate::utils;

static PROMOTION_OPTIONS: [board::Piece; 4] = [
    board::Piece::Bishop,
    board::Piece::Knight,
    board::Piece::Rook,
    board::Piece::Queen,
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

    // TODO: I am skeptical about the correctness of these fills
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
        for _ in 0..15 {
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
        for _ in 0..15 {
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
        for _ in 0..15 {
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
        for _ in 0..15 {
            flood |= (flood << 7) & mask;
        }
        (flood << 7) & self.clear_file[7]
    }

    // Calculates all sliding attacks with dumb fill by dispatch
    fn sliding_attacks(&self, orientation: &Orientation, sliders: u64, empty: u64) -> u64 {
        match orientation {
            Orientation::North => self.north_attacks(sliders, empty),
            Orientation::NorthEast => self.north_east_attacks(sliders, empty),
            Orientation::East => self.east_attacks(sliders, empty),
            Orientation::SouthEast => self.south_east_attacks(sliders, empty),
            Orientation::South => self.south_attacks(sliders, empty),
            Orientation::SouthWest => self.south_west_attacks(sliders, empty),
            Orientation::West => self.west_attacks(sliders, empty),
            Orientation::NorthWest => self.north_west_attacks(sliders, empty),
        }
    }

    // =================================
    //         PAWN MOVE GEN
    // =================================

    /// Return possible double and single pawn pushes, and promotions
    pub fn pawn_pushes(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let own_pawns = board.own_pieces & board.pawns;
        let empty = board.empty();

        let single_pushes = (own_pawns << 8) & empty;
        let normal_single_pushes = single_pushes & self.clear_rank[7];
        let promotions = single_pushes & self.mask_rank[7];
        let double_pushes = ((((own_pawns & self.mask_rank[1]) << 8) & empty) << 8) & empty;

        for (from_idx, to_idx) in self
            .parse_vertical_moves(normal_single_pushes, own_pawns)
            .iter()
        {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: None,
                category: MoveCategory::Normal,
            })
        }

        for (from_idx, to_idx) in self.parse_vertical_moves(promotions, own_pawns).iter() {
            for piece in PROMOTION_OPTIONS {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: piece,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Promotion,
                })
            }
        }

        for (from_idx, to_idx) in self.parse_vertical_moves(double_pushes, own_pawns).iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: None,
                category: MoveCategory::DoublePawnPush,
            })
        }

        move_list
    }

    // Return possible captures. Handles capture promotions and en passant.
    pub fn pawn_captures(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let own_pawns = board.own_pieces & board.pawns;
        let opp_pawns = board.opp_pieces & board.pawns;

        let right_captures = ((own_pawns << 9) & self.clear_file[0]) & board.opp_pieces;
        let normal_right_captures =
            self.parse_diagonal_moves(right_captures & self.clear_rank[7], own_pawns);
        let right_capture_promotions =
            self.parse_diagonal_moves(right_captures & self.mask_rank[7], own_pawns);

        let left_captures = ((own_pawns << 7) & self.clear_file[7]) & board.opp_pieces;
        let normal_left_captures =
            self.parse_anti_diagonal_moves(left_captures & self.clear_rank[7], own_pawns);
        let left_capture_promotions =
            self.parse_anti_diagonal_moves(left_captures & self.mask_rank[7], own_pawns);

        for (from_idx, to_idx) in normal_right_captures.iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board.identify(*to_idx)),
                category: MoveCategory::Normal,
            })
        }

        for (from_idx, to_idx) in right_capture_promotions.iter() {
            for piece in PROMOTION_OPTIONS {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: piece,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Promotion,
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
            })
        }

        for (from_idx, to_idx) in left_capture_promotions.iter() {
            for piece in PROMOTION_OPTIONS {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: piece,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Promotion,
                })
            }
        }

        let right_ep_move = (own_pawns << 25) & self.clear_file[0];
        if right_ep_move & opp_pawns != 0 {
            let (from_idx, to_idx) = self.parse_ep_capture_move(right_ep_move, Orientation::East);
            move_list.push(Move {
                from: from_idx,
                to: to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board::Piece::Pawn),
                category: MoveCategory::EnPassant,
            })
        }

        let left_ep_move = (own_pawns << 23) & self.clear_file[7];
        if left_ep_move & opp_pawns != 0 {
            let (from_idx, to_idx) = self.parse_ep_capture_move(left_ep_move, Orientation::West);
            move_list.push(Move {
                from: from_idx,
                to: to_idx,
                piece: board::Piece::Pawn,
                color: board.color(),
                capture: Some(board::Piece::Pawn),
                category: MoveCategory::EnPassant,
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
                    color: color,
                    capture: None,
                    category: MoveCategory::Normal,
                })
            }

            for (from_idx, to_idx) in self.parse_single_piece_moves(captures, *knight_idx).iter() {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Knight,
                    color: color,
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
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
        let king_idx = board.own_king;
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
                color: color,
                capture: None,
                category: MoveCategory::Normal,
            })
        }

        for (from_idx, to_idx) in self.parse_single_piece_moves(captures, king_idx).iter() {
            move_list.push(Move {
                from: *from_idx,
                to: *to_idx,
                piece: board::Piece::King,
                color: color,
                capture: Some(board.identify(*to_idx)),
                category: MoveCategory::Normal,
            })
        }

        move_list
    }

    // TODO: must check the squares we're passing through for check, must also check king square
    // Pseudo-legal castling move generation
    // Returned moves describe rook moves
    pub fn castling_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list = Vec::new();

        let king_threatened = self.threatened(4, &FIRST_RANK_THREAT_ORIENTATIONS, board);
        if king_threatened {
            return move_list;
        }

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
                })
            }
        }

        move_list
    }

    // =================================
    //         SLIDING MOVE GEN
    // =================================

    // TODO: obvious bugs, using all sliders not enemy sliders. just rewrite
    /// Returns all possible orthogonal moves (rooks and queens)
    pub fn ortho_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list: Vec<Move> = Vec::new();
        let mut moves: Vec<(u8, u8)> = Vec::new();
        let mut captures: Vec<(u8, u8)> = Vec::new();

        let queens = board.queens();
        let empty = board.empty();

        let north = self.north_attacks(board.ortho_sliders, empty);
        let north_captures = north & board.opp_pieces;
        let north_moves = north & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            north_captures,
            board.ortho_sliders,
            Orientation::North,
        ));
        moves.append(&mut self.parse_sliding_moves(
            north_moves,
            board.ortho_sliders,
            Orientation::North,
        ));

        let east = self.east_attacks(board.ortho_sliders, empty);
        let east_captures = east & board.opp_pieces;
        let east_moves = east & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            east_captures,
            board.ortho_sliders,
            Orientation::East,
        ));
        moves.append(&mut self.parse_sliding_moves(
            east_moves,
            board.ortho_sliders,
            Orientation::East,
        ));

        let south = self.south_attacks(board.ortho_sliders, empty);
        let south_captures = south & board.opp_pieces;
        let south_moves = south & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            south_captures,
            board.ortho_sliders,
            Orientation::South,
        ));
        moves.append(&mut self.parse_sliding_moves(
            south_moves,
            board.ortho_sliders,
            Orientation::South,
        ));

        let west = self.west_attacks(board.ortho_sliders, empty);
        let west_captures = west & board.opp_pieces;
        let west_moves = west & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            west_captures,
            board.ortho_sliders,
            Orientation::West,
        ));
        moves.append(&mut self.parse_sliding_moves(
            west_moves,
            board.ortho_sliders,
            Orientation::West,
        ));

        for (from_idx, to_idx) in moves.iter() {
            // TODO: faster just to use Board::identify? more elegatnt, certainly
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Normal,
                })
            } else {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Rook,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Normal,
                })
            }
        }

        for (from_idx, to_idx) in captures.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                })
            } else {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Rook,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                })
            }
        }

        move_list
    }

    /// Returns all possible diagonal moves (bishops and queens)
    pub fn diag_moves(&self, board: &board::Board) -> Vec<Move> {
        let mut move_list: Vec<Move> = Vec::new();
        let mut moves: Vec<(u8, u8)> = Vec::new();
        let mut captures: Vec<(u8, u8)> = Vec::new();

        let queens = board.queens();
        let empty = board.empty();

        let north_east = self.north_east_attacks(board.ortho_sliders, empty);
        let north_east_captures = north_east & board.opp_pieces;
        let north_east_moves = north_east & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            north_east_captures,
            board.ortho_sliders,
            Orientation::NorthEast,
        ));
        moves.append(&mut self.parse_sliding_moves(
            north_east_moves,
            board.ortho_sliders,
            Orientation::NorthEast,
        ));

        let south_east = self.south_east_attacks(board.ortho_sliders, empty);
        let south_east_captures = south_east & board.opp_pieces;
        let south_east_moves = south_east & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            south_east_captures,
            board.ortho_sliders,
            Orientation::SouthEast,
        ));
        moves.append(&mut self.parse_sliding_moves(
            south_east_moves,
            board.ortho_sliders,
            Orientation::SouthEast,
        ));

        let south_west = self.south_west_attacks(board.ortho_sliders, empty);
        let south_west_captures = south_west & board.opp_pieces;
        let south_west_moves = south_west & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            south_west_captures,
            board.ortho_sliders,
            Orientation::SouthWest,
        ));
        moves.append(&mut self.parse_sliding_moves(
            south_west_moves,
            board.ortho_sliders,
            Orientation::SouthWest,
        ));

        let north_west = self.north_west_attacks(board.ortho_sliders, empty);
        let north_west_captures = north_west & board.opp_pieces;
        let north_west_moves = north_west & !board.opp_pieces;
        captures.append(&mut self.parse_sliding_moves(
            north_west_captures,
            board.ortho_sliders,
            Orientation::NorthWest,
        ));
        moves.append(&mut self.parse_sliding_moves(
            north_west_moves,
            board.ortho_sliders,
            Orientation::NorthWest,
        ));

        for (from_idx, to_idx) in moves.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Normal,
                })
            } else {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Bishop,
                    color: board.color(),
                    capture: None,
                    category: MoveCategory::Normal,
                })
            }
        }

        for (from_idx, to_idx) in captures.iter() {
            if ((1 << *from_idx) & queens) != 0 {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Queen,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                })
            } else {
                move_list.push(Move {
                    from: *from_idx,
                    to: *to_idx,
                    piece: board::Piece::Bishop,
                    color: board.color(),
                    capture: Some(board.identify(*to_idx)),
                    category: MoveCategory::Normal,
                })
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
    fn parse_sliding_moves(
        &self,
        moves: u64,
        pieces: u64,
        orientation: Orientation,
    ) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();

        // 1. sort pieces
        // this is just a really unnecessarily complicated sort
        let piece_idxs = serialize_board(pieces);
        let mut axis_wise: [Option<u8>; 15] = [None; 15];

        for piece_idx in piece_idxs.iter() {
            match orientation.axis() {
                Axis::Rank => {
                    axis_wise[board::file_index(*piece_idx) as usize] = Some(*piece_idx);
                }
                Axis::File => {
                    axis_wise[board::rank_index(*piece_idx) as usize] = Some(*piece_idx);
                }
                Axis::Diagonal => {
                    axis_wise[board::anti_diag_index(*piece_idx) as usize] = Some(*piece_idx);
                }
                Axis::AntiDiagonal => {
                    axis_wise[board::diag_index(*piece_idx) as usize] = Some(*piece_idx);
                }
            }
        }

        // indices of pieces along the current axis

        let mut sorted_piece_idxs = utils::bad_argsort(axis_wise.to_vec());
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
        masks.push(orientation.ray(&self, sorted_piece_idxs[0] as usize));
        for idx in 1..sorted_piece_idxs.len() {
            let forward_mask = orientation.ray(&self, sorted_piece_idxs[idx] as usize);
            let backward_mask = orientation
                .antipode()
                .ray(&self, sorted_piece_idxs[idx - 1] as usize);
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

    // TODO: check validity of pieces?
    /// Parse a bitboard of horizontal moves into a move list
    /// All pieces must be on their own rank
    fn parse_horizontal_moves(&self, moves: u64, pieces: u64) -> Vec<(u8, u8)> {
        let mut move_list = Vec::new();
        let piece_idxs = serialize_board(pieces);

        for piece_idx in piece_idxs.iter() {
            let piece_rank = board::rank_index(*piece_idx);
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
            let piece_file = board::file_index(*piece_idx);
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
            let piece_diag = board::diag_index(*piece_idx);
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
            let piece_anti_diag = board::anti_diag_index(*piece_idx);
            let masked = moves & self.mask_anti_diag[piece_anti_diag as usize];
            for move_idx in serialize_board(masked).iter() {
                move_list.push((*piece_idx as u8, *move_idx as u8));
            }
        }

        move_list
    }

    // ========================
    //   PSEUDO-LEGAL MOVE GEN
    // ========================

    /// Generate all pseudo-legal moves
    pub fn psuedo_legal_moves(&self, board: board::Board) -> Vec<Move> {
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

    // Identifies new threats sq may now be exposed to due to move. Returns threat bitboard
    // Performs no checks for castling, legality of castling is delegated to primary move gen
    fn exposed_threats(&self, sq: u8, m: &Move, board: &board::Board) -> u64 {
        match m.category {
            MoveCategory::KingsideCastle => return 0,
            MoveCategory::QueensideCastle => return 0,
            MoveCategory::EnPassant => {
                let mut threats: u64 = 0;
                // check orientation exposed by movement of own pawn
                match self.rel_orientation[(sq * 64 + m.from) as usize] {
                    Some(orientation) => threats |= self.sliding_threats(sq, &orientation, board),
                    None => (),
                }
                // check orientation exposed by now captured enemy pawn
                match self.rel_orientation[(sq * 64 + m.to - 8) as usize] {
                    Some(orientation) => threats |= self.sliding_threats(sq, &orientation, board),
                    None => (),
                }
                threats
            }
            _ => match self.rel_orientation[(sq * 64 + m.from) as usize] {
                Some(orientation) => self.sliding_threats(sq, &orientation, board),
                None => 0,
            },
        }
    }

    // Identifies threats sq is exposed to in dir orientation
    fn sliding_threats(&self, sq: u8, orientation: &Orientation, board: &board::Board) -> u64 {
        // enemy sliders are excluded from mask so occluded enemy sliders will be correctly counted
        let enemy_sliders = board.sliders(&orientation) & board.opp_pieces;
        let mask = board.empty() | enemy_sliders;
        let exposed_bb = self.sliding_attacks(&orientation, 1 << sq, mask);
        exposed_bb & enemy_sliders
    }

    // Checks if sq is threatened by knights or by sliding pieces in list of orientations
    fn threatened(&self, sq: u8, orientations: &[Orientation], board: &board::Board) -> bool {
        if self.knight_movement[sq as usize] & board.knights() & board.opp_pieces != 0 {
            return true;
        }

        for orientation in orientations {
            if self.sliding_threats(sq, orientation, board) != 0 {
                return true;
            }
        }

        false
    }
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
fn pop_count(state: u64) -> u8 {
    serialize_board(state).len() as u8
}

// TODO: I don't think color is necessary
#[derive(Debug)]
pub struct Move {
    pub from: u8, // integer 0-63
    pub to: u8,   // integer 0-63
    pub piece: board::Piece,
    pub color: board::Color,
    pub capture: Option<board::Piece>,
    pub category: MoveCategory,
}

#[derive(PartialEq, Debug)]
pub enum MoveCategory {
    Normal,
    QueensideCastle,
    KingsideCastle,
    EnPassant,
    DoublePawnPush,
    Promotion,
}

pub enum Axis {
    // horizontal
    Rank,
    // vertical
    File,
    Diagonal,
    AntiDiagonal,
}

#[derive(Copy, Clone)]
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

    // there isn't really a notion of empty castling rights, so those are left to the default
    fn empty_board() -> board::Board {
        let mut b = board::Board::new();
        b.own_pieces = 0;
        b.opp_pieces = 0;
        b.ortho_sliders = 0;
        b.diag_sliders = 0;
        b.pawns = 0;
        b.own_king = 0;
        b.opp_king = 0;
        b
    }

    fn pawns(own_pawns: Vec<u8>, opp_pawns: Vec<u8>) -> board::Board {
        let mut b = empty_board();
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

        for file_idx in 0..7 {
            let b = pawns(vec![board::square_index(1, file_idx)], Vec::new());
            assert_eq!(move_gen.pawn_pushes(&b).len(), 2);
        }
    }

    #[test]
    fn single_pawn_other_rank_push() {
        let move_gen = MoveGen::new();

        for rank_idx in 3..6 {
            for file_idx in 0..7 {
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

        for file_idx in 0..7 {
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
                    "rank_idx: {rank_idx}, file_idx: {file_idx}, moves: {moves:#?}"
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

}
