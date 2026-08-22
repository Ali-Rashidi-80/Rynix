mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn popcount64(x: i64) -> i64 {
    let mut v = x;
    let mut c = 0i64;
    while v != 0 {
        c += v & 1;
        v >>= 1;
    }
    c
}

fn main() {
    let n: i64 = 25_000_000;
    let mut x: i64 = 1;
    let mut acc: i64 = 0;
    for i in 0..n {
        x = (x * 31 + i) % 1_000_000_007;
        acc += popcount64(x);
    }
    suite5_print_i64(acc);
}
