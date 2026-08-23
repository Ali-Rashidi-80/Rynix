/* SHA-256 known-answer + KV smoke. */

#include "../include/rynix_rt.h"

#include <stdio.h>

/* SHA256("") first 8 bytes BE = e3b0c44298fc1c14 */
#define SHA_EMPTY ((int64_t)0xe3b0c44298fc1c14LL)
/* SHA256("abc") first 8 bytes BE = ba7816bf8f01cfea */
#define SHA_ABC ((int64_t)0xba7816bf8f01cfeaLL)

int main(void) {
  int64_t empty = rynix_rt_sha256_first_i64("");
  int64_t abc = rynix_rt_sha256_first_i64("abc");
  if (empty != SHA_EMPTY) {
    fprintf(stderr, "sha empty got=%llx want=%llx\n", (long long)empty,
            (long long)SHA_EMPTY);
    return 1;
  }
  if (abc != SHA_ABC) {
    fprintf(stderr, "sha abc got=%llx want=%llx\n", (long long)abc, (long long)SHA_ABC);
    return 1;
  }

  /* RFC 4231 Test Case 1 — key=0x0b*20, data="Hi There"
   * HMAC = b0344c61d8db3853… → first 8 BE */
#define HMAC_TC1 ((int64_t)0xb0344c61d8db3853LL)
  {
    char key[21];
    int i;
    for (i = 0; i < 20; i++) {
      key[i] = (char)0x0b;
    }
    key[20] = '\0';
    int64_t h = rynix_rt_hmac_sha256_first_i64(key, "Hi There");
    if (h != HMAC_TC1) {
      fprintf(stderr, "hmac tc1 got=%llx want=%llx\n", (long long)h, (long long)HMAC_TC1);
      return 1;
    }
  }

  rynix_rt_region_create(0);
  void *kv = rynix_rt_kv_new(0);
  if (!kv) {
    fprintf(stderr, "kv_new failed\n");
    return 1;
  }
  rynix_rt_kv_put(kv, "a", 1);
  rynix_rt_kv_put(kv, "b", 2);
  rynix_rt_kv_put(kv, "a", 9);
  if (rynix_rt_kv_get(kv, "a") != 9 || rynix_rt_kv_get(kv, "b") != 2) {
    fprintf(stderr, "kv get mismatch\n");
    return 1;
  }
  if (rynix_rt_kv_len(kv) != 2) {
    fprintf(stderr, "kv len mismatch\n");
    return 1;
  }
  if (rynix_rt_kv_get(kv, "missing") != 0) {
    fprintf(stderr, "kv missing should be 0\n");
    return 1;
  }

  {
    int64_t tag = rynix_rt_aes128_gcm_nist_empty_tag_first_i64();
    if (tag == -2) {
      /* no AEAD backend on this host */
    } else if (tag != (int64_t)0x58e2fccefa7e3061LL) {
      fprintf(stderr, "aead nist tag got=%llx\n", (long long)tag);
      return 1;
    }
  }

  puts("crypto_kv_smoke ok");
  return 0;
}
