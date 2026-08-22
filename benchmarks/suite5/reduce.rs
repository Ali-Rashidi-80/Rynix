mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn main() {
    let n: i64 = 10_000_000;
    let mut acc: i64 = 0;
    let mut i: i64 = 0;
    while i < n {
        acc = acc + i * 31 - i / 8 + i % 13;
        i += 1;
    }
    suite5_print_i64(acc);
}
