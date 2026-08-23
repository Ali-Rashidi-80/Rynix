const bench = @import("bench.zig");
const c = @cImport({
    @cInclude("stdio.h");
});

fn popcount64(x: i64) i64 {
    var v = x;
    var cnt: i64 = 0;
    while (v != 0) : (v >>= 1) {
        cnt += v & 1;
    }
    return cnt;
}

pub fn main() void {
    const n: i64 = bench.opaqueI64(25000000);
    var x: i64 = 1;
    var acc: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        x = @rem(x * 31 + i, 1000000007);
        acc += popcount64(x);
    }
    bench.printI64(acc);
}
