const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = bench.opaqueI64(10000000);
    var acc: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        acc = acc + i * 31 - @divTrunc(i, 8) + @rem(i, 13);
    }
    bench.printI64(acc);
}
