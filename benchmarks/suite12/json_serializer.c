/* Suite12 #8 — JSON serializer hash (End suite12_c.c). Locked checksum across langs. */

#include <stdint.h>
#include <stdio.h>

int64_t bench_8_json_serializer(void) {
  char buf[512];
  int64_t hash = 0;

  for (int i = 0; i < 100000; i++) {
    int len = sprintf(buf,
                      "{\"id\":%d,\"status\":\"active\",\"latency_us\":%d,\"tags\":[\"prod\","
                      "\"edge\",\"v2\"],\"metrics\":{\"cpu\":%.1f,\"mem\":%.1f}}",
                      i, (i * 37) % 500, 42.5f + (float)(i % 10), 128.4f + (float)(i % 50));
    hash = (hash * 31) + len + buf[len / 2];
  }
  return hash;
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_8_json_serializer());
  return 0;
}
