const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = 3000000;
    var h: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        h = @rem(h * 31 + i, 1000000007);
    }
    bench.printI64(h);
}
