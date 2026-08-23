/* JSON unit tests — exit 0 only when all cases pass. */
#include "../include/rynix_rt.h"

#include <stdio.h>
#include <stdlib.h>

static int expect_i64(const char *label, int64_t got, int64_t want) {
  if (got != want) {
    fprintf(stderr, "FAIL %s: got %lld want %lld\n", label, (long long)got,
            (long long)want);
    return 1;
  }
  return 0;
}

int main(void) {
  int fail = 0;
  fail |= expect_i64("basic", rynix_rt_json_get_i64("{\"value\":42}", "value"), 42);
  fail |= expect_i64("spaces", rynix_rt_json_get_i64("{ \"n\" : 7 }", "n"), 7);
  fail |= expect_i64("missing", rynix_rt_json_get_i64("{\"a\":1}", "b"), -1);
  fail |= expect_i64("null_json", rynix_rt_json_get_i64(NULL, "x"), -1);
  fail |= expect_i64("nested_key",
                     rynix_rt_json_get_i64("{\"value\":42,\"other\":7}", "other"), 7);
  fail |= expect_i64("has_yes", rynix_rt_json_has_i64("{\"n\":42}", "n"), 1);
  fail |= expect_i64("has_no", rynix_rt_json_has_i64("{\"n\":42}", "x"), 0);
  fail |= expect_i64("has_neg", rynix_rt_json_has_i64("{\"n\":-1}", "n"), 1);
  fail |= expect_i64("get_neg", rynix_rt_json_get_i64("{\"n\":-1}", "n"), -1);
  if (fail) {
    return 1;
  }
  puts("json_unit ok");
  return 0;
}
