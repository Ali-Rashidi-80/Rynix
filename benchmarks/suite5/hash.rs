mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn main() {
    let n: i64 = 3_000_000;
    let mut h: i64 = 0;
    let mut i: i64 = 0;
    while i < n {
        h = (h * 31 + i) % 1_000_000_007;
        i += 1;
    }
    suite5_print_i64(h);
}
