mod bench_runtime;
use bench_runtime::{suite5_opaque_i64, suite5_print_i64};

fn gcd64(mut a: i64, mut b: i64) -> i64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn main() {
    let n: i64 = suite5_opaque_i64(2500000);
    let mut acc: i64 = 0;
    let mut i: i64 = 1;
    while i <= n {
        let a = i * 9973;
        let b = i * 1237 + 42;
        acc += gcd64(a, b);
        i += 1;
    }
    suite5_print_i64(acc);
}
