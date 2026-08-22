mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn main() {
    let n: i64 = 450;
    let mut s: i64 = 0;
    let mut i: i64 = 0;
    while i < n {
        let mut j: i64 = 0;
        while j < n {
            s = s + (i * j + i) % 97;
            j += 1;
        }
        i += 1;
    }
    suite5_print_i64(s);
}
