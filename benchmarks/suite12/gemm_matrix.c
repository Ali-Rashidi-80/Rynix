/* Suite12 #10 — blocked GEMM 512×512 (End suite12_c.c). Locked checksum across langs. */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int64_t bench_10_gemm_matrix(void) {
  const int N = 512;
  double *A = (double *)malloc((size_t)N * (size_t)N * sizeof(double));
  double *B = (double *)malloc((size_t)N * (size_t)N * sizeof(double));
  double *C = (double *)calloc((size_t)N * (size_t)N, sizeof(double));
  const int BLOCK = 32;
  double trace = 0.0;

  if (!A || !B || !C) {
    free(A);
    free(B);
    free(C);
    return -1;
  }
  for (int i = 0; i < N * N; i++) {
    A[i] = (double)(i % 100) * 0.01;
    B[i] = (double)((i * 3) % 100) * 0.01;
  }
  for (int sj = 0; sj < N; sj += BLOCK) {
    for (int si = 0; si < N; si += BLOCK) {
      for (int sk = 0; sk < N; sk += BLOCK) {
        for (int i = si; i < si + BLOCK; i++) {
          for (int k = sk; k < sk + BLOCK; k++) {
            double a_ik = A[i * N + k];
            for (int j = sj; j < sj + BLOCK; j++) {
              C[i * N + j] += a_ik * B[k * N + j];
            }
          }
        }
      }
    }
  }
  for (int i = 0; i < N; i++) {
    trace += C[i * N + i];
  }
  free(A);
  free(B);
  free(C);
  return (int64_t)(trace * 100.0);
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_10_gemm_matrix());
  return 0;
}
