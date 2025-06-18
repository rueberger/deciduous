use crate::board;
use crate::moves;

use std::io;

// Reads moves from stdin in coord str representation (eg a2a4 e7e5)
// Doesn't validate move legality, but panics if you try to move an empty square
// If board passed generates moves against it, otherwise will be for startpos
pub fn read_moves(b: &board::Board) -> Vec<moves::Move> {
    let mut move_list: Vec<moves::Move> = Vec::new();

    let mut input = String::new();

    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");

    for mstr in input.trim().split(" ") {
        assert!(
            mstr.chars().count() == 4,
            "{:#?}: {}",
            mstr.chars(),
            mstr.chars().count()
        );
        let from = parse_coord_str(&mstr[0..2]);
        let to = parse_coord_str(&mstr[2..4]);

        move_list.push(create_move(&b, from, to));
    }

    move_list
}

// converts coord str (eg a4, g5) to board sq
pub fn parse_coord_str(coord_str: &str) -> u8 {
    assert!(coord_str.len() == 2);

    let mut coord_chars = coord_str.chars();
    let file_char = coord_chars.next().unwrap();
    let rank_char = coord_chars.next().unwrap();

    assert!('a' <= file_char);
    assert!(file_char <= 'h');
    let file_idx = file_char as u8 - 97;

    assert!('1' <= rank_char);
    assert!(rank_char <= '8');
    let rank_idx = rank_char as u8 - 49;

    board::square_index(rank_idx, file_idx)
}


// Create a move given from and to square given current state of board
// Doesn't validate move legality, but panics if you try to move an empty square
fn create_move(b: &board::Board, from: u8, to: u8) -> moves::Move {
    println!("f {} t {}", from, to);
    if b.empty() & (1 << from) != 0 {
        panic!("Can't generate a move from empty square");
    }

    let capture = if b.empty() & (1 << to) == 0 {
        Some(b.identify(to))
    } else {
        None
    };

    moves::Move {
        from,
        to,
        piece: b.identify(from),
        color: b.color(),
        capture,
        category: identify_move_category(from, to)
    }
}

// TODO, kind of a pain in the ass
// actually just need to look it up in the move list
// identify category of move
fn identify_move_category(from: u8, to: u8) -> moves::MoveCategory{
    moves::MoveCategory::Normal
}
