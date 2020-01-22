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

    println!("Whiten:\n{}", render_board(b.bitboard[0]));
    println!("Black:\n{}", render_board(b.bitboard[1]));
    println!("Pawns:\n{}", render_board(b.bitboard[2]));
    println!("Bishops:\n{}", render_board(b.bitboard[3]));
    println!("Knights:\n{}", render_board(b.bitboard[4]));
    println!("Rooks:\n{}", render_board(b.bitboard[5]));
    println!("Kings:\n{}", render_board(b.bitboard[6]));
    println!("Queens:\n{}", render_board(b.bitboard[7]));
}
