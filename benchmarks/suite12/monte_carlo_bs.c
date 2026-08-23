/* Suite12 #11 — Monte Carlo Black-Scholes (End suite12_c.c). Locked checksum across langs. */

#include <math.h>
#include <stdint.h>
#include <stdio.h>

static uint64_t splitmix64(uint64_t *state) {
  *state += 0x9E3779B97F4A7C15ULL;
  uint64_t z = *state;
  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
  z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
  return z ^ (z >> 31);
}

int64_t bench_11_monte_carlo(void) {
  const int PATHS = 2000000;
  const double S0 = 100.0, K = 100.0, T = 1.0, r = 0.05, sigma = 0.20;
  const double drift = (r - 0.5 * sigma * sigma) * T;
  const double vol = sigma * sqrt(T);
  const double discount = exp(-r * T);
  uint64_t prng = 0xFEEDFACECAFE1234ULL;
  double total_payoff = 0.0;

  for (int i = 0; i < PATHS; i += 2) {
    double u1 = (double)((splitmix64(&prng) >> 11) + 1) / 9007199254740992.0;
    double u2 = (double)((splitmix64(&prng) >> 11) + 1) / 9007199254740992.0;
    double radius = sqrt(-2.0 * log(u1));
    double theta = 2.0 * 3.14159265358979323846 * u2;
    double z1 = radius * cos(theta);
    double z2 = radius * sin(theta);
    double s_t1 = S0 * exp(drift + vol * z1);
    double s_t2 = S0 * exp(drift + vol * z2);
    double payoff1 = s_t1 > K ? (s_t1 - K) : 0.0;
    double payoff2 = s_t2 > K ? (s_t2 - K) : 0.0;
    total_payoff += (payoff1 + payoff2);
  }
  {
    double option_price = (total_payoff / (double)PATHS) * discount;
    return (int64_t)(option_price * 1000000.0);
  }
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_11_monte_carlo());
  return 0;
}
