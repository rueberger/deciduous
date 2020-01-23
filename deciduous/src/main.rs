mod board;


fn render_board(state: u64) -> String {
    let mut render = String::new();

    // TODO: no idea if this order is right
    for rank in (0..8).rev() {
        for file in 0..8 {
            let idx = board::square_idx(rank, file);

            render.push('[');
            // TODO: check for order here too
            if let Some(_) = board::bitscan_lsd(state & (1 << idx)) {
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
    b.initialize();


    println!("White:\n{}", render_board(b.bitboard[0]));
    println!("White bitscan:\n{}", board::bitscan_lsd(b.bitboard[0]).unwrap());
    println!("Black:\n{}", render_board(b.bitboard[1]));
    println!("Pawns:\n{}", render_board(b.bitboard[2]));
    println!("Bishops:\n{}", render_board(b.bitboard[3]));
    println!("Knights:\n{}", render_board(b.bitboard[4]));
    println!("Rooks:\n{}", render_board(b.bitboard[5]));
    println!("Kings:\n{}", render_board(b.bitboard[6]));
    println!("Queens:\n{}", render_board(b.bitboard[7]));

    for idx in 0..64 {
        println!("Ray {}\n =========================================", idx);
        // println!("north:\n{}", render_board(b.north[idx]));
        // println!("south:\n{}", render_board(b.south[idx]));
        // println!("west:\n{}", render_board(b.west[idx]));
        // println!("east:\n{}", render_board(b.east[idx]));
        println!("north east:\n{}", render_board(b.north_east[idx]));
        println!("north west:\n{}", render_board(b.north_west[idx]));
        println!("south west:\n{}", render_board(b.south_west[idx]));
        println!("south east:\n{}", render_board(b.south_east[idx]));
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
