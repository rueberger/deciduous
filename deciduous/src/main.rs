mod board;


fn render_board(state: u64) -> String {
    let mut render = String::new();

    // TODO: no idea if this order is right
    for rank in 0..8 {
        for file in 0..8 {
            let idx = rank * 8 + file;

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
    println!("Black:\n{}", render_board(b.bitboard[1]));
    println!("Pawns:\n{}", render_board(b.bitboard[2]));
    println!("Bishops:\n{}", render_board(b.bitboard[3]));
    println!("Knights:\n{}", render_board(b.bitboard[4]));
    println!("Rooks:\n{}", render_board(b.bitboard[5]));
    println!("Kings:\n{}", render_board(b.bitboard[6]));
    println!("Queens:\n{}", render_board(b.bitboard[7]));

    println!("a-file:\n{}", render_board(0x0101010101010101));
    println!("1st-rank:\n{}", render_board(0x00000000000000FF));


    // for idx in 0..8 {
    //     println!("index: {} \n", idx);
    //     println!("mask rank:\n{}", render_board(b.mask_rank[idx]));
    //     println!("mask file:\n{}", render_board(b.mask_file[idx]));
    //     println!("clear rank:\n{}", render_board(b.clear_rank[idx]));
    //     println!("clear file:\n{}", render_board(b.clear_file[idx]));
    // }
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
