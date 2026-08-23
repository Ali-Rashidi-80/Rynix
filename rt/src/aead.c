/* AES-128-GCM NIST KAT (empty PT/AAD) — real AEAD, not End’s len+16 stub.
 * Windows: BCrypt. Linux: -DRYNIX_RT_OPENSSL. Else: -2.
 */

#include <stdint.h>
#include <string.h>

#include "../include/rynix_rt.h"

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <bcrypt.h>

#pragma comment(lib, "bcrypt.lib")

int64_t rynix_rt_aes128_gcm_nist_empty_tag_first_i64(void) {
  static const UCHAR key[16] = {0};
  static const UCHAR iv[12] = {0};
  static const UCHAR expect[8] = {0x58, 0xe2, 0xfc, 0xce, 0xfa, 0x7e, 0x30, 0x61};
  BCRYPT_ALG_HANDLE alg = NULL;
  BCRYPT_KEY_HANDLE hkey = NULL;
  UCHAR tag[16];
  NTSTATUS st;
  BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO info;
  ULONG cb = 0;

  st = BCryptOpenAlgorithmProvider(&alg, BCRYPT_AES_ALGORITHM, NULL, 0);
  if (st < 0) {
    return -1;
  }
  st = BCryptSetProperty(alg, BCRYPT_CHAINING_MODE, (PUCHAR)BCRYPT_CHAIN_MODE_GCM,
                         (ULONG)((wcslen(BCRYPT_CHAIN_MODE_GCM) + 1) * sizeof(WCHAR)), 0);
  if (st < 0) {
    BCryptCloseAlgorithmProvider(alg, 0);
    return -1;
  }
  st = BCryptGenerateSymmetricKey(alg, &hkey, NULL, 0, (PUCHAR)key, 16, 0);
  if (st < 0) {
    BCryptCloseAlgorithmProvider(alg, 0);
    return -1;
  }
  BCRYPT_INIT_AUTH_MODE_INFO(info);
  info.pbNonce = (PUCHAR)iv;
  info.cbNonce = 12;
  info.pbTag = tag;
  info.cbTag = 16;
  memset(tag, 0, sizeof(tag));
  st = BCryptEncrypt(hkey, NULL, 0, &info, NULL, 0, NULL, 0, &cb, 0);
  BCryptDestroyKey(hkey);
  BCryptCloseAlgorithmProvider(alg, 0);
  if (st < 0) {
    return -1;
  }
  if (memcmp(tag, expect, 8) != 0) {
    return -1;
  }
  return ((int64_t)tag[0] << 56) | ((int64_t)tag[1] << 48) | ((int64_t)tag[2] << 40) |
         ((int64_t)tag[3] << 32) | ((int64_t)tag[4] << 24) | ((int64_t)tag[5] << 16) |
         ((int64_t)tag[6] << 8) | (int64_t)tag[7];
}

#elif defined(RYNIX_RT_OPENSSL)

#include <openssl/evp.h>

int64_t rynix_rt_aes128_gcm_nist_empty_tag_first_i64(void) {
  static const unsigned char key[16] = {0};
  static const unsigned char iv[12] = {0};
  static const unsigned char expect[8] = {0x58, 0xe2, 0xfc, 0xce, 0xfa, 0x7e, 0x30, 0x61};
  unsigned char tag[16];
  int outl = 0;
  EVP_CIPHER_CTX *ctx = EVP_CIPHER_CTX_new();
  if (!ctx) {
    return -1;
  }
  if (EVP_EncryptInit_ex(ctx, EVP_aes_128_gcm(), NULL, NULL, NULL) != 1 ||
      EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_SET_IVLEN, 12, NULL) != 1 ||
      EVP_EncryptInit_ex(ctx, NULL, NULL, key, iv) != 1 ||
      EVP_EncryptUpdate(ctx, NULL, &outl, NULL, 0) != 1 ||
      EVP_EncryptFinal_ex(ctx, NULL, &outl) != 1 ||
      EVP_CIPHER_CTX_ctrl(ctx, EVP_CTRL_GCM_GET_TAG, 16, tag) != 1) {
    EVP_CIPHER_CTX_free(ctx);
    return -1;
  }
  EVP_CIPHER_CTX_free(ctx);
  if (memcmp(tag, expect, 8) != 0) {
    return -1;
  }
  return ((int64_t)tag[0] << 56) | ((int64_t)tag[1] << 48) | ((int64_t)tag[2] << 40) |
         ((int64_t)tag[3] << 32) | ((int64_t)tag[4] << 24) | ((int64_t)tag[5] << 16) |
         ((int64_t)tag[6] << 8) | (int64_t)tag[7];
}

#else

int64_t rynix_rt_aes128_gcm_nist_empty_tag_first_i64(void) {
  return -2;
}

#endif
