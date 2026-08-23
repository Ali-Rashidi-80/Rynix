mod bench_runtime;
use bench_runtime::{suite5_opaque_i64, suite5_print_i64};

fn main() {
    let n: i64 = suite5_opaque_i64(1500000);
    let mut acc: i64 = 0;
    for i in 0..n {
        acc += i * i;
    }
    suite5_print_i64(acc);
}
