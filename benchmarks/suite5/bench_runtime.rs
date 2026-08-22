use std::env;

static mut SUITE5_BENCH_SINK: i64 = 0;

pub fn suite5_print_i64(n: i64) {
    if env::var("SUITE5_BENCH").is_ok() {
        unsafe {
            SUITE5_BENCH_SINK = n;
        }
        return;
    }
    println!("{n}");
}
