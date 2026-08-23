mod bench_runtime;
use bench_runtime::{suite5_opaque_i64, suite5_print_i64};

fn main() {
    let n: i64 = suite5_opaque_i64(8000000);
    let mut acc: i64 = 0;
    for i in 0..n {
        if i % 3 == 0 || i % 7 == 0 {
            acc += 1;
        }
    }
    suite5_print_i64(acc);
}
