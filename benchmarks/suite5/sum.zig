const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = 1_500_000;
    var acc: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        acc += i * i;
    }
    bench.printI64(acc);
}
