mod board;
mod moves;
mod utils;

use std::time;

fn m_perft(depth: usize) {
    println!("| depth | nodes | time |");
    for d in 1..(depth + 1) {
        let start = time::Instant::now();
        let nodes = moves::perft(d);
        let elapsed = start.elapsed();
        let secs = elapsed.as_secs();
        let ms = elapsed.subsec_millis();
        println!("| {d} | {nodes} | {secs}.{ms} |");
    }
}

fn main() {
    m_perft(5);
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
