/* Suite12 #7 — DNA Levenshtein (End suite12_c.c). Locked checksum across langs. */

#include <stdint.h>
#include <stdio.h>

static uint64_t splitmix64(uint64_t *state) {
  *state += 0x9E3779B97F4A7C15ULL;
  uint64_t z = *state;
  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
  z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
  return z ^ (z >> 31);
}

int64_t bench_7_dna_alignment(void) {
  const int N = 1000;
  int32_t dp[1001];
  uint64_t prng = 0x9999888877776666ULL;
  char s1[1000], s2[1000];
  const char bases[] = {'A', 'C', 'G', 'T'};
  int64_t total_distance = 0;

  for (int pair = 0; pair < 1000; pair++) {
    for (int i = 0; i < N; i++) {
      s1[i] = bases[splitmix64(&prng) % 4];
      s2[i] = bases[splitmix64(&prng) % 4];
    }
    for (int j = 0; j <= N; j++) {
      dp[j] = j;
    }
    for (int i = 1; i <= N; i++) {
      int32_t prev = dp[0];
      dp[0] = i;
      for (int j = 1; j <= N; j++) {
        int32_t temp = dp[j];
        int cost = (s1[i - 1] == s2[j - 1]) ? 0 : 1;
        int32_t d1 = dp[j - 1] + 1;
        int32_t d2 = dp[j] + 1;
        int32_t d3 = prev + cost;
        int32_t min_d = d1 < d2 ? d1 : d2;
        dp[j] = min_d < d3 ? min_d : d3;
        prev = temp;
      }
    }
    total_distance += dp[N];
  }
  return total_distance;
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_7_dna_alignment());
  return 0;
}
