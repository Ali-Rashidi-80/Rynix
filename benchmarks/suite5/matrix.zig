const bench = @import("bench.zig");
const c = @cImport({
    @cInclude("stdio.h");
});

pub fn main() void {
    var a: [4][4]i64 = undefined;
    var b: [4][4]i64 = undefined;
    var out: [4][4]i64 = undefined;
    for (0..4) |i| {
        for (0..4) |j| {
            a[i][j] = @intCast(i + j);
            b[i][j] = @intCast(i * j + 1);
            out[i][j] = 0;
        }
    }
    const reps: i64 = bench.opaqueI64(900000);
    var trace: i64 = 0;
    var r: i64 = 0;
    while (r < reps) : (r += 1) {
        for (0..4) |i| {
            for (0..4) |j| {
                var s: i64 = 0;
                for (0..4) |k| {
                    s += a[i][k] * b[k][j];
                }
                out[i][j] = s;
            }
        }
        trace += out[@intCast(r & 3)][@intCast(r & 3)];
    }
    bench.printI64(trace);
}
