/// Module for general utilities, not related to chess

// TODO: there must be something in std that does this
/// crappy arg sort that only works efficiently for unique values in tiny ranges
pub fn bad_argsort(arr: Vec<Option<u8>>) -> Vec<u8> {
    let mut sorted_vals = Vec::new();

    for idx in 0..arr.len() {
        if let Some(val) = arr[idx] {
            sorted_vals.push(val as u8)
        }
    }

    sorted_vals
}

// Partitions list of ints into sets of unique and duplicated values.
// Expected to out perform hashing approach only for small inputs.
// Panics if arr is empty.
// Neither returned vector will have duplicated elements
//
// Returns
//   unique - Vec<u8>
//   dup - Vec<u8>
fn partition_unique(mut arr: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    // TODO: debug asserts good practice??
    debug_assert!(arr.len() >= 1);

    let mut unique: Vec<u8> = Vec::new();
    let mut dup: Vec<u8> = Vec::new();

    if arr.len() == 1 {
        unique.push(arr[0]);
        return (unique, dup);
    }

    arr.sort_unstable();

    let mut last_idx = 0;
    let mut last = arr[0];
    let mut curr_idx = 0;
    let mut curr = arr[0];

    loop {
        curr_idx += 1;

        if curr_idx < arr.len() {
            curr = arr[curr_idx];
            if curr == last {
                continue;
            }

            if curr_idx - last_idx > 1 {
                dup.push(last);
            } else {
                unique.push(last);
            }

            last_idx = curr_idx;
            last = curr;
        } else {
            if curr == last && curr_idx - last_idx > 1 {
                dup.push(last);
            } else {
                unique.push(last);
            }
            break;
        }
    }

    (unique, dup)
}

#[cfg(test)]
mod tests {
    use super::partition_unique;

    #[test]
    fn partition_unique_all_dup() {
        let arr = vec![1, 1, 1];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 0);
        assert_eq!(dup.len(), 1);
    }

    #[test]
    fn partition_unique_all_unq() {
        let arr = vec![1, 2, 3];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 3);
        assert_eq!(dup.len(), 0);
    }

    #[test]
    fn partition_unique_normal_case_1() {
        let arr = vec![1, 1, 1, 2, 3, 4, 5, 5, 5];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 3);
        assert_eq!(dup.len(), 2);
    }

    #[test]
    fn partition_unique_normal_case_2() {
        let arr = vec![1, 1, 1, 2, 3, 4];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 3);
        assert_eq!(dup.len(), 1);
    }

    #[test]
    fn partition_unique_normal_case_3() {
        let arr = vec![1, 2, 3, 4, 4];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 3);
        assert_eq!(dup.len(), 1);
    }

    #[test]
    fn partition_unique_short_case_1() {
        let arr = vec![1];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 1);
        assert_eq!(dup.len(), 0);
    }

    #[test]
    fn partition_unique_short_case_2() {
        let arr = vec![1, 2];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 2);
        assert_eq!(dup.len(), 0);
    }

    #[test]
    fn partition_unique_short_case_3() {
        let arr = vec![1, 1];
        let (unq, dup) = partition_unique(arr);

        assert_eq!(unq.len(), 0);
        assert_eq!(dup.len(), 1);
    }
}
