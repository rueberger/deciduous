mod board;
mod moves;
mod utils;


fn render_board(state: u64) -> String {
    let mut render = String::new();

    // TODO: no idea if this order is right
    for rank in (0..8).rev() {
        for file in 0..8 {
            let idx = board::square_index(rank, file);

            render.push('[');
            // TODO: check for order here too
            if let Some(_) = moves::bitscan_lsd(state & (1 << idx)) {
                render.push('X')
            } else {
                render.push(' ')
            }
            render.push(']')
        }
        render.push('\n')
    }
    return render
}


fn main() {
    let mut b = board::init_board();
    let mut move_gen = moves::init_move_gen();
    move_gen.initialize();

    println!("White:\n{}", render_board(b.own_pieces));
    b.color_flip();
    println!("White flipped:\n{}", render_board(b.own_pieces));
    println!("{}", moves::serialize_board(b.own_pieces).as_slice()[12]);

    let xs: [i32; 5] = [1, 2, 3, 4, 5];


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
