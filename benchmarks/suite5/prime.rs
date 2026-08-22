mod bench_runtime;
use bench_runtime::suite5_print_i64;

fn main() {
    let limit: i64 = 100_000;
    let mut count: i64 = 0;
    for i in 2..=limit {
        let mut prime = 1i64;
        let mut j = 2i64;
        while j * j <= i {
            if i % j == 0 {
                prime = 0;
                break;
            }
            j += 1;
        }
        if prime != 0 {
            count += 1;
        }
    }
    suite5_print_i64(count);
}
