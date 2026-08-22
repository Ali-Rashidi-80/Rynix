const bench = @import("bench.zig");
const c = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const n: i64 = 5000000;
    var a: i64 = 0;
    var b: i64 = 1;
    var i: i64 = 0;
    while (i < n) : (i += 1) {
        const nxt = a + b;
        a = b;
        b = nxt;
    }
    bench.printI64(a);
}
