/* Suite12 #3 — HFT limit-order engine (End suite12_c.c). Locked checksum across langs. */

#include <stdint.h>
#include <stdio.h>

static uint64_t splitmix64(uint64_t *state) {
  *state += 0x9E3779B97F4A7C15ULL;
  uint64_t z = *state;
  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
  z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
  return z ^ (z >> 31);
}

int64_t bench_3_hft_engine(void) {
  uint64_t rng = 0x123456789ABCDEF0ULL;
  int64_t total_volume = 0;
  int32_t buy_depth[100] = {0};
  int32_t sell_depth[100] = {0};

  for (int i = 0; i < 1000000; i++) {
    uint64_t r = splitmix64(&rng);
    int side = (int)((r >> 63) & 1);
    int price = (int)(r % 100);
    int qty = (int)((r >> 8) % 50) + 1;
    int op = (int)((r >> 16) % 10);

    if (op == 0) {
      if (side == 0) {
        buy_depth[price] = 0;
      } else {
        sell_depth[price] = 0;
      }
    } else if (side == 0) {
      for (int p = price; p >= 0 && qty > 0; p--) {
        if (sell_depth[p] > 0) {
          int fill = qty < sell_depth[p] ? qty : sell_depth[p];
          sell_depth[p] -= fill;
          qty -= fill;
          total_volume += (int64_t)fill * (p + 1);
        }
      }
      if (qty > 0) {
        buy_depth[price] += qty;
      }
    } else {
      for (int p = price; p < 100 && qty > 0; p++) {
        if (buy_depth[p] > 0) {
          int fill = qty < buy_depth[p] ? qty : buy_depth[p];
          buy_depth[p] -= fill;
          qty -= fill;
          total_volume += (int64_t)fill * (p + 1);
        }
      }
      if (qty > 0) {
        sell_depth[price] += qty;
      }
    }
  }
  return total_volume;
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_3_hft_engine());
  return 0;
}
