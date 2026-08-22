/* JSON soft-std smoke — exit 0 when parse returns 42. */
#include "../include/rynix_rt.h"

int main(void) {
  int64_t v = rynix_rt_json_get_i64("{\"value\":42,\"other\":7}", "value");
  return v == 42 ? 0 : 1;
}
