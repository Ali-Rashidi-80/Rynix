const bench = @import("bench.zig");
const clib = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    const limit: i64 = 100_000;
    var count: i64 = 0;
    var i: i64 = 2;
    while (i <= limit) : (i += 1) {
        var prime: i64 = 1;
        var j: i64 = 2;
        while (j * j <= i) : (j += 1) {
            if (@rem(i, j) == 0) {
                prime = 0;
                break;
            }
        }
        if (prime != 0) count += 1;
    }
    bench.printI64(count);
}
