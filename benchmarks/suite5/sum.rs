mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn main() {
    let n: i64 = 1_500_000;
    let mut acc: i64 = 0;
    for i in 0..n {
        acc += i * i;
    }
    suite5_print_i64(acc);
}
