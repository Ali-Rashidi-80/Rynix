/* TLS 1.2+ echo helpers — real crypto, not a simulated handshake.
 *
 * Windows: SChannel (secur32/crypt32).
 * Linux: OpenSSL when <openssl/ssl.h> is present (CI: libssl-dev).
 * Else: return -2 (unsupported).
 *
 * SURPASS D1 — evidence via rt/tests/tls_echo_smoke.c.
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../include/rynix_rt.h"

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#define SECURITY_WIN32
#include <windows.h>
#include <wincrypt.h>
#include <schannel.h>
#include <security.h>

#pragma comment(lib, "secur32.lib")
#pragma comment(lib, "crypt32.lib")

typedef struct {
  CredHandle cred;
  CtxtHandle ctx;
  int have_cred;
  int have_ctx;
  int server;
  SecPkgContext_StreamSizes sizes;
  char pending[16384];
  int pending_len;
} RynixTls;

static void tls_free(RynixTls *t) {
  if (!t) {
    return;
  }
  if (t->have_ctx) {
    DeleteSecurityContext(&t->ctx);
    t->have_ctx = 0;
  }
  if (t->have_cred) {
    FreeCredentialsHandle(&t->cred);
    t->have_cred = 0;
  }
}

static PCCERT_CONTEXT make_self_signed(void) {
  BYTE name_buf[256];
  DWORD name_len = sizeof(name_buf);
  if (!CertStrToNameA(X509_ASN_ENCODING, "CN=localhost", CERT_X500_NAME_STR, NULL, name_buf,
                      &name_len, NULL)) {
    return NULL;
  }
  CERT_NAME_BLOB nb;
  nb.cbData = name_len;
  nb.pbData = name_buf;
  SYSTEMTIME good_for;
  GetSystemTime(&good_for);
  good_for.wYear = (WORD)(good_for.wYear + 2);
  return CertCreateSelfSignCertificate(0, &nb, 0, NULL, NULL, NULL, &good_for, NULL);
}

static int tls_acquire(RynixTls *t, int server, PCCERT_CONTEXT cert) {
  memset(t, 0, sizeof(*t));
  t->server = server;
  SCHANNEL_CRED sc;
  memset(&sc, 0, sizeof(sc));
  sc.dwVersion = SCHANNEL_CRED_VERSION;
  /* Prefer TLS 1.2+; TLS 1.3 flag is optional on older SDKs. */
#ifdef SP_PROT_TLS1_3
  sc.grbitEnabledProtocols = SP_PROT_TLS1_2 | SP_PROT_TLS1_3;
#else
  sc.grbitEnabledProtocols = SP_PROT_TLS1_2;
#endif
  if (server) {
    sc.cCreds = 1;
    sc.paCred = &cert;
    sc.dwFlags = SCH_USE_STRONG_CRYPTO;
  } else {
    sc.dwFlags = SCH_CRED_MANUAL_CRED_VALIDATION | SCH_CRED_NO_DEFAULT_CREDS |
                 SCH_CRED_NO_SERVERNAME_CHECK | SCH_USE_STRONG_CRYPTO;
  }
  TimeStamp ts;
  SECURITY_STATUS st = AcquireCredentialsHandleA(
      NULL, UNISP_NAME_A, server ? SECPKG_CRED_INBOUND : SECPKG_CRED_OUTBOUND, NULL, &sc, NULL,
      NULL, &t->cred, &ts);
  if (st != SEC_E_OK) {
    return -1;
  }
  t->have_cred = 1;
  return 0;
}

static int sock_send_all(int64_t fd, const char *buf, int n) {
  int off = 0;
  while (off < n) {
    int64_t w = rynix_rt_tcp_send(fd, buf + off, n - off);
    if (w <= 0) {
      return -1;
    }
    off += (int)w;
  }
  return 0;
}

static int tls_handshake(RynixTls *t, int64_t fd, const char *target) {
  SecBufferDesc out_desc, in_desc;
  SecBuffer out_bufs[1], in_bufs[2];
  DWORD flags_in;
  if (t->server) {
    flags_in = ASC_REQ_SEQUENCE_DETECT | ASC_REQ_REPLAY_DETECT | ASC_REQ_CONFIDENTIALITY |
               ASC_REQ_ALLOCATE_MEMORY | ASC_REQ_STREAM;
  } else {
    flags_in = ISC_REQ_SEQUENCE_DETECT | ISC_REQ_REPLAY_DETECT | ISC_REQ_CONFIDENTIALITY |
               ISC_REQ_ALLOCATE_MEMORY | ISC_REQ_STREAM | ISC_REQ_MANUAL_CRED_VALIDATION |
               ISC_REQ_USE_SUPPLIED_CREDS;
  }
  DWORD flags_out = 0;
  SECURITY_STATUS st = SEC_I_CONTINUE_NEEDED;
  char io[16384];
  int io_len = 0;
  int first = 1;

  while (st == SEC_I_CONTINUE_NEEDED || st == SEC_E_INCOMPLETE_MESSAGE ||
         st == SEC_I_INCOMPLETE_CREDENTIALS) {
    int need_recv = 0;
    if (t->server) {
      need_recv = (io_len == 0 || st == SEC_E_INCOMPLETE_MESSAGE);
    } else {
      /* ClientHello first — only recv after the initial token is sent. */
      need_recv = (!first && (io_len == 0 || st == SEC_E_INCOMPLETE_MESSAGE));
    }
    if (need_recv) {
      int64_t r = rynix_rt_tcp_recv(fd, io + io_len, (int64_t)(sizeof(io) - (size_t)io_len));
      if (r <= 0) {
        return -1;
      }
      io_len += (int)r;
    }

    memset(in_bufs, 0, sizeof(in_bufs));
    out_bufs[0].pvBuffer = NULL;
    out_bufs[0].BufferType = SECBUFFER_TOKEN;
    out_bufs[0].cbBuffer = 0;
    out_desc.ulVersion = SECBUFFER_VERSION;
    out_desc.cBuffers = 1;
    out_desc.pBuffers = out_bufs;

    if (t->server) {
      in_bufs[0].pvBuffer = io;
      in_bufs[0].cbBuffer = (ULONG)io_len;
      in_bufs[0].BufferType = SECBUFFER_TOKEN;
      in_bufs[1].pvBuffer = NULL;
      in_bufs[1].cbBuffer = 0;
      in_bufs[1].BufferType = SECBUFFER_EMPTY;
      in_desc.ulVersion = SECBUFFER_VERSION;
      in_desc.cBuffers = 2;
      in_desc.pBuffers = in_bufs;

      st = AcceptSecurityContext(&t->cred, t->have_ctx ? &t->ctx : NULL, &in_desc, flags_in, 0,
                                 &t->ctx, &out_desc, &flags_out, NULL);
    } else {
      SecBufferDesc *p_in = NULL;
      if (t->have_ctx) {
        in_bufs[0].pvBuffer = io;
        in_bufs[0].cbBuffer = (ULONG)io_len;
        in_bufs[0].BufferType = SECBUFFER_TOKEN;
        in_bufs[1].pvBuffer = NULL;
        in_bufs[1].cbBuffer = 0;
        in_bufs[1].BufferType = SECBUFFER_EMPTY;
        in_desc.ulVersion = SECBUFFER_VERSION;
        in_desc.cBuffers = 2;
        in_desc.pBuffers = in_bufs;
        p_in = &in_desc;
      }
      st = InitializeSecurityContextA(&t->cred, t->have_ctx ? &t->ctx : NULL, (SEC_CHAR *)target,
                                      flags_in, 0, 0, p_in, 0, &t->ctx, &out_desc, &flags_out,
                                      NULL);
    }
    t->have_ctx = 1;
    first = 0;

    if (st == SEC_E_OK || st == SEC_I_CONTINUE_NEEDED ||
        st == SEC_I_COMPLETE_AND_CONTINUE || st == SEC_I_COMPLETE_NEEDED) {
      if (out_bufs[0].cbBuffer != 0 && out_bufs[0].pvBuffer != NULL) {
        if (sock_send_all(fd, (const char *)out_bufs[0].pvBuffer, (int)out_bufs[0].cbBuffer) !=
            0) {
          FreeContextBuffer(out_bufs[0].pvBuffer);
          return -1;
        }
        FreeContextBuffer(out_bufs[0].pvBuffer);
      }
      if (st == SEC_I_COMPLETE_NEEDED || st == SEC_I_COMPLETE_AND_CONTINUE) {
        CompleteAuthToken(&t->ctx, &out_desc);
      }
    }

    if (in_bufs[1].BufferType == SECBUFFER_EXTRA && in_bufs[1].cbBuffer > 0) {
      int extra = (int)in_bufs[1].cbBuffer;
      memmove(io, io + (io_len - extra), (size_t)extra);
      io_len = extra;
    } else if (st != SEC_E_INCOMPLETE_MESSAGE) {
      io_len = 0;
    }

    if (st == SEC_E_OK) {
      break;
    }
    if (st != SEC_I_CONTINUE_NEEDED && st != SEC_E_INCOMPLETE_MESSAGE &&
        st != SEC_I_INCOMPLETE_CREDENTIALS && st != SEC_I_COMPLETE_AND_CONTINUE &&
        st != SEC_I_COMPLETE_NEEDED) {
      return -1;
    }
  }

  if (QueryContextAttributes(&t->ctx, SECPKG_ATTR_STREAM_SIZES, &t->sizes) != SEC_E_OK) {
    return -1;
  }
  if (io_len > 0) {
    if (io_len > (int)sizeof(t->pending)) {
      return -1;
    }
    memcpy(t->pending, io, (size_t)io_len);
    t->pending_len = io_len;
  }
  return 0;
}

static int tls_encrypt_send(RynixTls *t, int64_t fd, const char *data, int n) {
  if (n < 0 || n > (int)t->sizes.cbMaximumMessage) {
    return -1;
  }
  int total = (int)t->sizes.cbHeader + n + (int)t->sizes.cbTrailer;
  char *buf = (char *)malloc((size_t)total);
  if (!buf) {
    return -1;
  }
  memcpy(buf + t->sizes.cbHeader, data, (size_t)n);
  SecBuffer bufs[4];
  bufs[0].BufferType = SECBUFFER_STREAM_HEADER;
  bufs[0].pvBuffer = buf;
  bufs[0].cbBuffer = t->sizes.cbHeader;
  bufs[1].BufferType = SECBUFFER_DATA;
  bufs[1].pvBuffer = buf + t->sizes.cbHeader;
  bufs[1].cbBuffer = (ULONG)n;
  bufs[2].BufferType = SECBUFFER_STREAM_TRAILER;
  bufs[2].pvBuffer = buf + t->sizes.cbHeader + n;
  bufs[2].cbBuffer = t->sizes.cbTrailer;
  bufs[3].BufferType = SECBUFFER_EMPTY;
  bufs[3].pvBuffer = NULL;
  bufs[3].cbBuffer = 0;
  SecBufferDesc desc = {SECBUFFER_VERSION, 4, bufs};
  SECURITY_STATUS st = EncryptMessage(&t->ctx, 0, &desc, 0);
  if (st != SEC_E_OK) {
    free(buf);
    return -1;
  }
  int wire = (int)(bufs[0].cbBuffer + bufs[1].cbBuffer + bufs[2].cbBuffer);
  int rc = sock_send_all(fd, buf, wire);
  free(buf);
  return rc;
}

static int tls_decrypt_recv(RynixTls *t, int64_t fd, char *out, int out_cap) {
  char msg[16384];
  int msg_len = t->pending_len;
  if (msg_len > 0) {
    if (msg_len > (int)sizeof(msg)) {
      return -1;
    }
    memcpy(msg, t->pending, (size_t)msg_len);
    t->pending_len = 0;
  }
  for (;;) {
    if (msg_len == 0) {
      int64_t r = rynix_rt_tcp_recv(fd, msg, (int64_t)sizeof(msg));
      if (r <= 0) {
        return -1;
      }
      msg_len = (int)r;
    }
    SecBuffer bufs[4];
    bufs[0].BufferType = SECBUFFER_DATA;
    bufs[0].pvBuffer = msg;
    bufs[0].cbBuffer = (ULONG)msg_len;
    bufs[1].BufferType = SECBUFFER_EMPTY;
    bufs[2].BufferType = SECBUFFER_EMPTY;
    bufs[3].BufferType = SECBUFFER_EMPTY;
    SecBufferDesc desc = {SECBUFFER_VERSION, 4, bufs};
    SECURITY_STATUS st = DecryptMessage(&t->ctx, &desc, 0, NULL);
    if (st == SEC_E_INCOMPLETE_MESSAGE) {
      if (msg_len >= (int)sizeof(msg)) {
        return -1;
      }
      int64_t r = rynix_rt_tcp_recv(fd, msg + msg_len, (int64_t)(sizeof(msg) - (size_t)msg_len));
      if (r <= 0) {
        return -1;
      }
      msg_len += (int)r;
      continue;
    }
    if (st != SEC_E_OK && st != SEC_I_RENEGOTIATE) {
      return -1;
    }
    for (int i = 0; i < 4; i++) {
      if (bufs[i].BufferType == SECBUFFER_DATA) {
        int n = (int)bufs[i].cbBuffer;
        if (n > out_cap) {
          return -1;
        }
        memcpy(out, bufs[i].pvBuffer, (size_t)n);
        /* Preserve leftover ciphertext for the next read. */
        for (int j = 0; j < 4; j++) {
          if (bufs[j].BufferType == SECBUFFER_EXTRA && bufs[j].cbBuffer > 0) {
            int extra = (int)bufs[j].cbBuffer;
            if (extra > (int)sizeof(t->pending)) {
              return -1;
            }
            memcpy(t->pending, bufs[j].pvBuffer, (size_t)extra);
            t->pending_len = extra;
            break;
          }
        }
        return n;
      }
    }
    return -1;
  }
}

static int tls_http_recv_message(RynixTls *t, int64_t fd, char *buf, int cap) {
  int total = 0;
  for (;;) {
    if (total >= cap - 1) {
      break;
    }
    int n = tls_decrypt_recv(t, fd, buf + total, cap - 1 - total);
    if (n <= 0) {
      break;
    }
    total += n;
    buf[total] = '\0';
    if (strstr(buf, "\r\n\r\n") != NULL) {
      break;
    }
  }
  buf[total] = '\0';
  return total;
}

static const char *tls_http_body(const char *msg) {
  const char *body = strstr(msg, "\r\n\r\n");
  if (!body) {
    return NULL;
  }
  return body + 4;
}

int64_t rynix_rt_tls_serve_once_echo(int64_t port) {
  if (port <= 0) {
    return -1;
  }
  PCCERT_CONTEXT cert = make_self_signed();
  if (!cert) {
    return -1;
  }
  RynixTls tls;
  if (tls_acquire(&tls, 1, cert) != 0) {
    CertFreeCertificateContext(cert);
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    tls_free(&tls);
    CertFreeCertificateContext(cert);
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    tls_free(&tls);
    CertFreeCertificateContext(cert);
    return -1;
  }
  int rc = -1;
  if (tls_handshake(&tls, client, NULL) == 0) {
    char buf[1024];
    int n = tls_decrypt_recv(&tls, client, buf, (int)sizeof(buf));
    if (n >= 0 && tls_encrypt_send(&tls, client, buf, n) == 0) {
      rc = 0;
    }
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  tls_free(&tls);
  CertFreeCertificateContext(cert);
  return rc;
}

int64_t rynix_rt_tls_client_echo(const char *host, int64_t port, const char *msg) {
  if (!host || !msg || port <= 0) {
    return -1;
  }
  RynixTls tls;
  if (tls_acquire(&tls, 0, NULL) != 0) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    tls_free(&tls);
    return -1;
  }
  int rc = -1;
  int nmsg = (int)strlen(msg);
  if (tls_handshake(&tls, fd, host) == 0 && tls_encrypt_send(&tls, fd, msg, nmsg) == 0) {
    char buf[1024];
    int n = tls_decrypt_recv(&tls, fd, buf, (int)sizeof(buf));
    if (n == nmsg && memcmp(buf, msg, (size_t)n) == 0) {
      rc = 0;
    }
  }
  rynix_rt_tcp_close(fd);
  tls_free(&tls);
  return rc;
}

int64_t rynix_rt_http_tls_serve_once_json_i64(int64_t port, const char *path, int64_t value) {
  if (!path || port <= 0) {
    return -1;
  }
  PCCERT_CONTEXT cert = make_self_signed();
  if (!cert) {
    return -1;
  }
  RynixTls tls;
  if (tls_acquire(&tls, 1, cert) != 0) {
    CertFreeCertificateContext(cert);
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    tls_free(&tls);
    CertFreeCertificateContext(cert);
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    tls_free(&tls);
    CertFreeCertificateContext(cert);
    return -1;
  }
  int64_t rc = -1;
  if (tls_handshake(&tls, client, NULL) == 0) {
    char req[2048];
    if (tls_http_recv_message(&tls, client, req, (int)sizeof(req)) > 0) {
      char *line_end = strstr(req, "\r\n");
      if (line_end) {
        *line_end = '\0';
        char *sp1 = strchr(req, ' ');
        char *req_path = sp1 ? sp1 + 1 : NULL;
        char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
        if (sp2) {
          *sp2 = '\0';
        }
        int path_ok = req_path && strcmp(req_path, path) == 0;
        char body[128];
        int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)value);
        char resp[512];
        int resp_n;
        if (path_ok && body_n > 0 && body_n < (int)sizeof(body)) {
          resp_n = snprintf(resp, sizeof(resp),
                            "HTTP/1.1 200 OK\r\n"
                            "Content-Type: application/json\r\n"
                            "Content-Length: %d\r\n"
                            "Connection: close\r\n"
                            "\r\n"
                            "%s",
                            body_n, body);
        } else {
          static const char not_found[] = "{\"error\":\"not_found\"}";
          resp_n = snprintf(resp, sizeof(resp),
                            "HTTP/1.1 404 Not Found\r\n"
                            "Content-Type: application/json\r\n"
                            "Content-Length: %d\r\n"
                            "Connection: close\r\n"
                            "\r\n"
                            "%s",
                            (int)(sizeof(not_found) - 1), not_found);
        }
        if (resp_n > 0 && resp_n < (int)sizeof(resp) &&
            tls_encrypt_send(&tls, client, resp, resp_n) == 0) {
          rc = path_ok ? 0 : 1;
        }
      }
    }
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  tls_free(&tls);
  CertFreeCertificateContext(cert);
  return rc;
}

int64_t rynix_rt_http_tls_get_json_i64(const char *host, int64_t port, const char *path,
                                       const char *field) {
  if (!host || !path || !field || port <= 0) {
    return -1;
  }
  RynixTls tls;
  if (tls_acquire(&tls, 0, NULL) != 0) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    tls_free(&tls);
    return -1;
  }
  int64_t rc = -1;
  if (tls_handshake(&tls, fd, host) == 0) {
    char req[512];
    int n = snprintf(req, sizeof(req),
                     "GET %s HTTP/1.1\r\nHost: %s\r\nConnection: close\r\n\r\n", path, host);
    if (n > 0 && n < (int)sizeof(req) && tls_encrypt_send(&tls, fd, req, n) == 0) {
      char buf[4096];
      if (tls_http_recv_message(&tls, fd, buf, (int)sizeof(buf)) > 0) {
        const char *body = tls_http_body(buf);
        if (body) {
          rc = rynix_rt_json_get_i64(body, field);
        }
      }
    }
  }
  rynix_rt_tcp_close(fd);
  tls_free(&tls);
  return rc;
}

#elif defined(RYNIX_RT_OPENSSL)

#include <openssl/err.h>
#include <openssl/evp.h>
#include <openssl/pem.h>
#include <openssl/rsa.h>
#include <openssl/ssl.h>
#include <openssl/x509.h>

static SSL_CTX *make_ctx(int server) {
  SSL_library_init();
  SSL_load_error_strings();
  OpenSSL_add_all_algorithms();
  const SSL_METHOD *method = server ? TLS_server_method() : TLS_client_method();
  SSL_CTX *ctx = SSL_CTX_new(method);
  if (!ctx) {
    return NULL;
  }
  if (!server) {
    SSL_CTX_set_verify(ctx, SSL_VERIFY_NONE, NULL);
    return ctx;
  }
  EVP_PKEY *pkey = EVP_PKEY_new();
  RSA *rsa = RSA_generate_key(2048, RSA_F4, NULL, NULL);
  if (!pkey || !rsa || !EVP_PKEY_assign_RSA(pkey, rsa)) {
    RSA_free(rsa);
    EVP_PKEY_free(pkey);
    SSL_CTX_free(ctx);
    return NULL;
  }
  X509 *x509 = X509_new();
  if (!x509) {
    EVP_PKEY_free(pkey);
    SSL_CTX_free(ctx);
    return NULL;
  }
  ASN1_INTEGER_set(X509_get_serialNumber(x509), 1);
  X509_gmtime_adj(X509_get_notBefore(x509), 0);
  X509_gmtime_adj(X509_get_notAfter(x509), 31536000L);
  X509_set_pubkey(x509, pkey);
  X509_NAME *name = X509_get_subject_name(x509);
  X509_NAME_add_entry_by_txt(name, "CN", MBSTRING_ASC, (unsigned char *)"localhost", -1, -1, 0);
  X509_set_issuer_name(x509, name);
  X509_sign(x509, pkey, EVP_sha256());
  if (SSL_CTX_use_certificate(ctx, x509) != 1 || SSL_CTX_use_PrivateKey(ctx, pkey) != 1) {
    X509_free(x509);
    EVP_PKEY_free(pkey);
    SSL_CTX_free(ctx);
    return NULL;
  }
  X509_free(x509);
  EVP_PKEY_free(pkey);
  return ctx;
}

static int bio_sock_write(BIO *b, const char *data, int n) {
  int64_t fd = (int64_t)(intptr_t)BIO_get_data(b);
  int64_t w = rynix_rt_tcp_send(fd, data, n);
  if (w <= 0) {
    return -1;
  }
  return (int)w;
}

static int bio_sock_read(BIO *b, char *data, int n) {
  int64_t fd = (int64_t)(intptr_t)BIO_get_data(b);
  int64_t r = rynix_rt_tcp_recv(fd, data, n);
  if (r <= 0) {
    return -1;
  }
  return (int)r;
}

static long bio_ctrl(BIO *b, int cmd, long num, void *ptr) {
  (void)b;
  (void)num;
  (void)ptr;
  if (cmd == BIO_CTRL_FLUSH) {
    return 1;
  }
  return 0;
}

static BIO_METHOD *sock_biomethod(void) {
  static BIO_METHOD *m;
  if (!m) {
    m = BIO_meth_new(BIO_TYPE_SOURCE_SINK, "rynix-tcp");
    BIO_meth_set_write(m, bio_sock_write);
    BIO_meth_set_read(m, bio_sock_read);
    BIO_meth_set_ctrl(m, bio_ctrl);
  }
  return m;
}

static SSL *ssl_wrap(SSL_CTX *ctx, int64_t fd) {
  SSL *ssl = SSL_new(ctx);
  if (!ssl) {
    return NULL;
  }
  BIO *bio = BIO_new(sock_biomethod());
  if (!bio) {
    SSL_free(ssl);
    return NULL;
  }
  BIO_set_data(bio, (void *)(intptr_t)fd);
  BIO_set_init(bio, 1);
  SSL_set_bio(ssl, bio, bio);
  return ssl;
}

int64_t rynix_rt_tls_serve_once_echo(int64_t port) {
  if (port <= 0) {
    return -1;
  }
  SSL_CTX *ctx = make_ctx(1);
  if (!ctx) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    SSL_CTX_free(ctx);
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    SSL_CTX_free(ctx);
    return -1;
  }
  SSL *ssl = ssl_wrap(ctx, client);
  int rc = -1;
  if (ssl && SSL_accept(ssl) == 1) {
    char buf[1024];
    int n = SSL_read(ssl, buf, (int)sizeof(buf));
    if (n >= 0 && SSL_write(ssl, buf, n) == n) {
      rc = 0;
    }
  }
  if (ssl) {
    SSL_free(ssl);
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  SSL_CTX_free(ctx);
  return rc;
}

int64_t rynix_rt_tls_client_echo(const char *host, int64_t port, const char *msg) {
  if (!host || !msg || port <= 0) {
    return -1;
  }
  SSL_CTX *ctx = make_ctx(0);
  if (!ctx) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    SSL_CTX_free(ctx);
    return -1;
  }
  SSL *ssl = ssl_wrap(ctx, fd);
  int rc = -1;
  int nmsg = (int)strlen(msg);
  if (ssl && SSL_connect(ssl) == 1 && SSL_write(ssl, msg, nmsg) == nmsg) {
    char buf[1024];
    int n = SSL_read(ssl, buf, (int)sizeof(buf));
    if (n == nmsg && memcmp(buf, msg, (size_t)n) == 0) {
      rc = 0;
    }
  }
  if (ssl) {
    SSL_free(ssl);
  }
  rynix_rt_tcp_close(fd);
  SSL_CTX_free(ctx);
  return rc;
}

static int ssl_http_recv_message(SSL *ssl, char *buf, int cap) {
  int total = 0;
  for (;;) {
    if (total >= cap - 1) {
      break;
    }
    int n = SSL_read(ssl, buf + total, cap - 1 - total);
    if (n <= 0) {
      break;
    }
    total += n;
    buf[total] = '\0';
    if (strstr(buf, "\r\n\r\n") != NULL) {
      break;
    }
  }
  buf[total] = '\0';
  return total;
}

static const char *ssl_http_body(const char *msg) {
  const char *body = strstr(msg, "\r\n\r\n");
  if (!body) {
    return NULL;
  }
  return body + 4;
}

int64_t rynix_rt_http_tls_serve_once_json_i64(int64_t port, const char *path, int64_t value) {
  if (!path || port <= 0) {
    return -1;
  }
  SSL_CTX *ctx = make_ctx(1);
  if (!ctx) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    SSL_CTX_free(ctx);
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    SSL_CTX_free(ctx);
    return -1;
  }
  SSL *ssl = ssl_wrap(ctx, client);
  int64_t rc = -1;
  if (ssl && SSL_accept(ssl) == 1) {
    char req[2048];
    if (ssl_http_recv_message(ssl, req, (int)sizeof(req)) > 0) {
      char *line_end = strstr(req, "\r\n");
      if (line_end) {
        *line_end = '\0';
        char *sp1 = strchr(req, ' ');
        char *req_path = sp1 ? sp1 + 1 : NULL;
        char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
        if (sp2) {
          *sp2 = '\0';
        }
        int path_ok = req_path && strcmp(req_path, path) == 0;
        char body[128];
        int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)value);
        char resp[512];
        int resp_n;
        if (path_ok && body_n > 0 && body_n < (int)sizeof(body)) {
          resp_n = snprintf(resp, sizeof(resp),
                            "HTTP/1.1 200 OK\r\n"
                            "Content-Type: application/json\r\n"
                            "Content-Length: %d\r\n"
                            "Connection: close\r\n"
                            "\r\n"
                            "%s",
                            body_n, body);
        } else {
          static const char not_found[] = "{\"error\":\"not_found\"}";
          resp_n = snprintf(resp, sizeof(resp),
                            "HTTP/1.1 404 Not Found\r\n"
                            "Content-Type: application/json\r\n"
                            "Content-Length: %d\r\n"
                            "Connection: close\r\n"
                            "\r\n"
                            "%s",
                            (int)(sizeof(not_found) - 1), not_found);
        }
        if (resp_n > 0 && resp_n < (int)sizeof(resp) && SSL_write(ssl, resp, resp_n) == resp_n) {
          rc = path_ok ? 0 : 1;
        }
      }
    }
  }
  if (ssl) {
    SSL_free(ssl);
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  SSL_CTX_free(ctx);
  return rc;
}

int64_t rynix_rt_http_tls_get_json_i64(const char *host, int64_t port, const char *path,
                                       const char *field) {
  if (!host || !path || !field || port <= 0) {
    return -1;
  }
  SSL_CTX *ctx = make_ctx(0);
  if (!ctx) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    SSL_CTX_free(ctx);
    return -1;
  }
  SSL *ssl = ssl_wrap(ctx, fd);
  int64_t rc = -1;
  if (ssl && SSL_connect(ssl) == 1) {
    char req[512];
    int n = snprintf(req, sizeof(req),
                     "GET %s HTTP/1.1\r\nHost: %s\r\nConnection: close\r\n\r\n", path, host);
    if (n > 0 && n < (int)sizeof(req) && SSL_write(ssl, req, n) == n) {
      char buf[4096];
      if (ssl_http_recv_message(ssl, buf, (int)sizeof(buf)) > 0) {
        const char *body = ssl_http_body(buf);
        if (body) {
          rc = rynix_rt_json_get_i64(body, field);
        }
      }
    }
  }
  if (ssl) {
    SSL_free(ssl);
  }
  rynix_rt_tcp_close(fd);
  SSL_CTX_free(ctx);
  return rc;
}

#else

int64_t rynix_rt_tls_serve_once_echo(int64_t port) {
  (void)port;
  return -2; /* unsupported on this platform / no OpenSSL */
}

int64_t rynix_rt_tls_client_echo(const char *host, int64_t port, const char *msg) {
  (void)host;
  (void)port;
  (void)msg;
  return -2;
}

int64_t rynix_rt_http_tls_serve_once_json_i64(int64_t port, const char *path, int64_t value) {
  (void)port;
  (void)path;
  (void)value;
  return -2;
}

int64_t rynix_rt_http_tls_get_json_i64(const char *host, int64_t port, const char *path,
                                       const char *field) {
  (void)host;
  (void)port;
  (void)path;
  (void)field;
  return -2;
}

#endif
