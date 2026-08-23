/* Suite12 foothold: End #12 ALU reduction — C reference with printed checksum.
 * Gate: checksum must stay stable (SURPASS E2 evidence, not marketing ms).
 */

#include <stdint.h>
#include <stdio.h>

typedef struct {
  uint64_t id;
  int32_t payload_size;
  int64_t checksum;
} Req12;

static Req12 process_req12(uint64_t id, int32_t size) {
  uint64_t hash = id ^ 0x9E3779B97F4A7C15ULL;
  for (int64_t j = 0; j < 50; j++) {
    hash ^= (hash << 13);
    hash ^= (hash >> 7);
    hash ^= (hash << 17);
    hash += (uint64_t)j + 0xBF58476D1CE4E5B9ULL;
  }
  return (Req12){.id = id, .payload_size = size, .checksum = (int64_t)hash};
}

int64_t bench_12_reduction(void) {
  const uint64_t iterations = 10000000ULL;
  int64_t sum0 = 0, sum1 = 0, sum2 = 0, sum3 = 0;
  for (uint64_t i = 0; i < iterations; i += 4) {
    sum0 += process_req12(i, 256).checksum;
    sum1 += process_req12(i + 1, 256).checksum;
    sum2 += process_req12(i + 2, 256).checksum;
    sum3 += process_req12(i + 3, 256).checksum;
  }
  return sum0 + sum1 + sum2 + sum3;
}

int main(void) {
  int64_t check = bench_12_reduction();
  printf("checksum=%lld\n", (long long)check);
  return 0;
}
