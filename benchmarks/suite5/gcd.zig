const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

fn gcd64(a: i64, b: i64) i64 {
    var x = a;
    var y = b;
    while (y != 0) {
        const t = @rem(x, y);
        x = y;
        y = t;
    }
    return x;
}

pub fn main() void {
    const n: i64 = bench.opaqueI64(2500000);
    var acc: i64 = 0;
    var i: i64 = 1;
    while (i <= n) : (i += 1) {
        const a = i * 9973;
        const b = i * 1237 + 42;
        acc += gcd64(a, b);
    }
    bench.printI64(acc);
}
