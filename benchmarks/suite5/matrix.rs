mod bench_runtime;
use bench_runtime::{suite5_opaque_i64, suite5_print_i64};

fn main() {
    let mut a = [[0i64; 4]; 4];
    let mut b = [[0i64; 4]; 4];
    let mut c = [[0i64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            a[i][j] = (i + j) as i64;
            b[i][j] = (i * j + 1) as i64;
            c[i][j] = 0;
        }
    }
    let reps: i64 = suite5_opaque_i64(900000);
    let mut trace: i64 = 0;
    for r in 0..reps {
        for i in 0..4 {
            for j in 0..4 {
                let mut s = 0i64;
                for k in 0..4 {
                    s += a[i][k] * b[k][j];
                }
                c[i][j] = s;
            }
        }
        trace += c[(r as usize) & 3][(r as usize) & 3];
    }
    suite5_print_i64(trace);
}
