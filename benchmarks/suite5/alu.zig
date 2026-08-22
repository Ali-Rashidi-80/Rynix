const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = 2000000;
    var acc: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        acc = acc + i * 3 - @divTrunc(i, 2) + @rem(i, 7);
    }
    bench.printI64(acc);
}
