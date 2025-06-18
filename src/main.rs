mod board;
mod interface;
mod moves;
mod utils;

use std::time;
use std::io;

fn m_perft(depth: usize) {
    println!("| depth | nodes | time | nodes/s |");
    for d in 1..(depth + 1) {
        let start = time::Instant::now();
        let board = board::Board::new();
        let nodes = moves::perft(board, d);
        let elapsed = start.elapsed();
        let secs = elapsed.as_secs();
        let ms = elapsed.subsec_millis();
        let ns: f32 = nodes as f32 / (secs as f32 + (0.001 * ms as f32));
        println!("| {d} | {nodes} | {secs}.{ms} |{ns} |");
    }
}

fn d_perft(mut board: board::Board, depth: usize) {
    let mut nodes = 0;
    let move_gen = moves::MoveGen::new();

    for m in move_gen.legal_moves(&board) {
        let mut branch_move_str = String::new();
        branch_move_str.push_str(&board::fmt_sq(m.from));
        branch_move_str.push_str(&board::fmt_sq(m.to));

        let u = board.make_move(&m);
        let branch_nodes = moves::perft(board.clone(), depth - 1);
        board.unmake_move(&m, &u);

        nodes += branch_nodes;
        println!("{}: {}", branch_move_str, branch_nodes);
    }

    println!("Total: {}", nodes)
}




fn main() {
    let mut b = board::Board::new();
    let mg = moves::MoveGen::new();

    println!("input moves: ");
    let move_list = interface::read_moves(&b);

    for m in move_list.iter() {
        let _ = b.make_move(&m);
    }

    println!("\n{b:#?}\n");

    println!("input perft depth: (input 0 to print legal moves)");
    let depth = interface::read_int();

    if depth == 0 {
        println!("{:#?}", mg.legal_moves(&b));
    } else {
        d_perft(b, depth);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_little_endian() -> bool {
        if cfg!(target_endian = "little") {
            return true;
        } else {
            return false;
        }
    }

    #[test]
    fn test_little_endian() {
        assert!(is_little_endian());
    }
}
