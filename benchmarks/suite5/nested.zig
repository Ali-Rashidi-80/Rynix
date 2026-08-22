const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = 450;
    var s: i64 = 0;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        var j: i64 = 0;
        while (j < n) : (j += 1) {
            s = s + @rem(i * j + i, 97);
        }
    }
    bench.printI64(s);
}
