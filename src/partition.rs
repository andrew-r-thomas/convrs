pub const fn generate_partition(n: usize, block_size: usize) -> Vec<(usize, usize)> {
    let out = vec![];
    let mut pointer = 0;
    let mut node = (1, 1, 1);

    while pointer < n {
        pointer += block_size;
    }
    out
}
