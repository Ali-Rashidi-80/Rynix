const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    var acc: i64 = 1;
    const base: i64 = 3;
    const n: i64 = bench.opaqueI64(2500000);
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        acc = @rem(acc * base, 1000000007);
    }
    bench.printI64(acc);
}
