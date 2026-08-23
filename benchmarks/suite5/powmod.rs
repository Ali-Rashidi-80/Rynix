mod bench_runtime;
use bench_runtime::{suite5_opaque_i64, suite5_print_i64};

fn main() {
    let mut acc: i64 = 1;
    let base: i64 = 3;
    let n: i64 = suite5_opaque_i64(2500000);
    for _ in 0..n {
        acc = (acc * base) % 1_000_000_007;
    }
    suite5_print_i64(acc);
}
