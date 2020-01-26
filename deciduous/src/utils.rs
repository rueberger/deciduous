/// Module for general utilities, not related to chess


// TODO: there must be something in std that does this
/// crappy arg sort that only works efficiently for unique values in tiny ranges
pub bad_argsort(arr: [Option<u8>]) -> Vec<u8> {
    let mut sorted_vals = Vec::new();

    for idx in 0..arr.len() {
        if let Some(val) = arr[idx] {
            indices.put(val as u8)
        }
    }

    sorted_vals
}
