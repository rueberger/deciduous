mod board;

fn main() {
    // println!("1, {}", board::bitscan_lsd(1 << 1));
    // println!("5, {}", board::bitscan_lsd(1 << 5));
    // println!("2, {}", board::bitscan_lsd(1 << 2));
    // println!("63, {}", board::bitscan_lsd(1 << 63));
    // println!("1 ^ 5, {}", board::bitscan_lsd((1 << 1) ^ (1 << 5)));
    let mut b = board::init_board();
    b.initialize();
    println!("{}", b.bitboard[0])
}
