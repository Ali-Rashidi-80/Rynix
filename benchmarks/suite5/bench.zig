const std = @import("std");
const c = @cImport({
    @cInclude("stdio.h");
    @cInclude("stdlib.h");
});

var sink: i64 = 0;

pub fn printI64(n: i64) void {
    if (c.getenv("SUITE5_BENCH") != null) {
        sink = n;
        return;
    }
    _ = c.printf("%lld\n", n);
}
