mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn main() {
    let n: i64 = 5_000_000;
    let mut a: i64 = 0;
    let mut b: i64 = 1;
    let mut i: i64 = 0;
    while i < n {
        let c = a + b;
        a = b;
        b = c;
        i += 1;
    }
    suite5_print_i64(a);
}
