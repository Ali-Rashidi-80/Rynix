const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = bench.opaqueI64(8000000);
    var acc: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        if (@rem(i, 3) == 0 or @rem(i, 7) == 0) acc += 1;
    }
    bench.printI64(acc);
}
