/* RFC 6455 Sec-WebSocket-Accept (SHA-1 + base64) — real handshake helper. */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../include/rynix_rt.h"

typedef struct {
  uint32_t state[5];
  uint64_t bitlen;
  uint8_t data[64];
  uint32_t datalen;
} Sha1Ctx;

static uint32_t rol(uint32_t v, uint32_t n) { return (v << n) | (v >> (32 - n)); }

static void sha1_transform(Sha1Ctx *ctx, const uint8_t data[64]) {
  uint32_t a, b, c, d, e, i, t;
  uint32_t m[80];
  for (i = 0; i < 16; ++i) {
    m[i] = ((uint32_t)data[i * 4] << 24) | ((uint32_t)data[i * 4 + 1] << 16) |
           ((uint32_t)data[i * 4 + 2] << 8) | (uint32_t)data[i * 4 + 3];
  }
  for (; i < 80; ++i) {
    m[i] = rol(m[i - 3] ^ m[i - 8] ^ m[i - 14] ^ m[i - 16], 1);
  }
  a = ctx->state[0];
  b = ctx->state[1];
  c = ctx->state[2];
  d = ctx->state[3];
  e = ctx->state[4];
  for (i = 0; i < 80; ++i) {
    if (i < 20) {
      t = rol(a, 5) + ((b & c) | ((~b) & d)) + e + m[i] + 0x5A827999u;
    } else if (i < 40) {
      t = rol(a, 5) + (b ^ c ^ d) + e + m[i] + 0x6ED9EBA1u;
    } else if (i < 60) {
      t = rol(a, 5) + ((b & c) | (b & d) | (c & d)) + e + m[i] + 0x8F1BBCDCu;
    } else {
      t = rol(a, 5) + (b ^ c ^ d) + e + m[i] + 0xCA62C1D6u;
    }
    e = d;
    d = c;
    c = rol(b, 30);
    b = a;
    a = t;
  }
  ctx->state[0] += a;
  ctx->state[1] += b;
  ctx->state[2] += c;
  ctx->state[3] += d;
  ctx->state[4] += e;
}

static void sha1_init(Sha1Ctx *ctx) {
  ctx->datalen = 0;
  ctx->bitlen = 0;
  ctx->state[0] = 0x67452301u;
  ctx->state[1] = 0xEFCDAB89u;
  ctx->state[2] = 0x98BADCFEu;
  ctx->state[3] = 0x10325476u;
  ctx->state[4] = 0xC3D2E1F0u;
}

static void sha1_update(Sha1Ctx *ctx, const uint8_t *data, size_t len) {
  size_t i;
  for (i = 0; i < len; ++i) {
    ctx->data[ctx->datalen++] = data[i];
    if (ctx->datalen == 64) {
      sha1_transform(ctx, ctx->data);
      ctx->bitlen += 512;
      ctx->datalen = 0;
    }
  }
}

static void sha1_final(Sha1Ctx *ctx, uint8_t hash[20]) {
  uint32_t i = ctx->datalen;
  if (ctx->datalen < 56) {
    ctx->data[i++] = 0x80;
    while (i < 56) {
      ctx->data[i++] = 0x00;
    }
  } else {
    ctx->data[i++] = 0x80;
    while (i < 64) {
      ctx->data[i++] = 0x00;
    }
    sha1_transform(ctx, ctx->data);
    memset(ctx->data, 0, 56);
  }
  ctx->bitlen += (uint64_t)ctx->datalen * 8;
  ctx->data[63] = (uint8_t)(ctx->bitlen);
  ctx->data[62] = (uint8_t)(ctx->bitlen >> 8);
  ctx->data[61] = (uint8_t)(ctx->bitlen >> 16);
  ctx->data[60] = (uint8_t)(ctx->bitlen >> 24);
  ctx->data[59] = (uint8_t)(ctx->bitlen >> 32);
  ctx->data[58] = (uint8_t)(ctx->bitlen >> 40);
  ctx->data[57] = (uint8_t)(ctx->bitlen >> 48);
  ctx->data[56] = (uint8_t)(ctx->bitlen >> 56);
  sha1_transform(ctx, ctx->data);
  for (i = 0; i < 4; ++i) {
    hash[i] = (uint8_t)((ctx->state[0] >> (24 - i * 8)) & 0xff);
    hash[i + 4] = (uint8_t)((ctx->state[1] >> (24 - i * 8)) & 0xff);
    hash[i + 8] = (uint8_t)((ctx->state[2] >> (24 - i * 8)) & 0xff);
    hash[i + 12] = (uint8_t)((ctx->state[3] >> (24 - i * 8)) & 0xff);
    hash[i + 16] = (uint8_t)((ctx->state[4] >> (24 - i * 8)) & 0xff);
  }
}

static void b64_20(const uint8_t in[20], char out[29]) {
  static const char *tbl =
      "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  size_t i, j = 0;
  for (i = 0; i < 18; i += 3) {
    uint32_t triple = ((uint32_t)in[i] << 16) | ((uint32_t)in[i + 1] << 8) | in[i + 2];
    out[j++] = tbl[(triple >> 18) & 63];
    out[j++] = tbl[(triple >> 12) & 63];
    out[j++] = tbl[(triple >> 6) & 63];
    out[j++] = tbl[triple & 63];
  }
  {
    uint32_t triple = ((uint32_t)in[18] << 16) | ((uint32_t)in[19] << 8);
    out[j++] = tbl[(triple >> 18) & 63];
    out[j++] = tbl[(triple >> 12) & 63];
    out[j++] = tbl[(triple >> 6) & 63];
    out[j++] = '=';
  }
  out[j] = '\0';
}

/* Returns 0 if accept key matches `want`, else -1. Soft API uses first-i64 form. */
int64_t rynix_rt_ws_accept_key_eq(const char *client_key, const char *want) {
  char combined[256];
  Sha1Ctx ctx;
  uint8_t digest[20];
  char accept[32];
  if (!client_key || !want) {
    return -1;
  }
  snprintf(combined, sizeof(combined), "%s258EAFA5-E914-47DA-95CA-C5AB0DC85B11", client_key);
  sha1_init(&ctx);
  sha1_update(&ctx, (const uint8_t *)combined, strlen(combined));
  sha1_final(&ctx, digest);
  b64_20(digest, accept);
  return strcmp(accept, want) == 0 ? 0 : -1;
}

/* First 8 bytes of SHA-1(key||GUID) as BE i64 (agent checksum / tests). */
int64_t rynix_rt_ws_accept_sha1_first_i64(const char *client_key) {
  char combined[256];
  Sha1Ctx ctx;
  uint8_t digest[20];
  if (!client_key) {
    client_key = "";
  }
  snprintf(combined, sizeof(combined), "%s258EAFA5-E914-47DA-95CA-C5AB0DC85B11", client_key);
  sha1_init(&ctx);
  sha1_update(&ctx, (const uint8_t *)combined, strlen(combined));
  sha1_final(&ctx, digest);
  return ((int64_t)digest[0] << 56) | ((int64_t)digest[1] << 48) |
         ((int64_t)digest[2] << 40) | ((int64_t)digest[3] << 32) |
         ((int64_t)digest[4] << 24) | ((int64_t)digest[5] << 16) |
         ((int64_t)digest[6] << 8) | (int64_t)digest[7];
}

static void ws_accept_b64(const char *client_key, char out[29]) {
  char combined[256];
  Sha1Ctx ctx;
  uint8_t digest[20];
  snprintf(combined, sizeof(combined), "%s258EAFA5-E914-47DA-95CA-C5AB0DC85B11", client_key);
  sha1_init(&ctx);
  sha1_update(&ctx, (const uint8_t *)combined, strlen(combined));
  sha1_final(&ctx, digest);
  b64_20(digest, out);
}

static void ws_mask_xor(uint8_t *data, int64_t n, const uint8_t mask[4]) {
  int64_t i;
  for (i = 0; i < n; i++) {
    data[i] ^= mask[i & 3];
  }
}

/* Max single-frame payload (DoS bound). RFC 6455 64-bit length MSB must be 0. */
#define RYNIX_WS_MAX_PAYLOAD (1 << 20)

/* Encode one WS data frame. mask4 non-NULL → set MASK bit (client→server).
 * fin: 1 = final fragment. opcode: 1=text, 0=continuation, …
 * Lengths: ≤125 (7-bit), ≤65535 (16-bit/126), else 64-bit/127 up to MAX. */
int64_t rynix_rt_ws_frame_encode_ex(int64_t fin, int64_t opcode, const char *payload, int64_t n,
                                    const uint8_t *mask4, uint8_t *out, int64_t out_cap) {
  int64_t need;
  int64_t hdr;
  int masked;
  int i;
  if (!out || n < 0 || n > RYNIX_WS_MAX_PAYLOAD || opcode < 0 || opcode > 15 ||
      (fin != 0 && fin != 1)) {
    return -1;
  }
  if (n > 0 && !payload) {
    return -1;
  }
  masked = mask4 != NULL;
  if (n <= 125) {
    hdr = 2;
  } else if (n <= 65535) {
    hdr = 4;
  } else {
    hdr = 10; /* 127 + 8-byte BE length */
  }
  need = hdr + (masked ? 4 : 0) + n;
  if (out_cap < need) {
    return -1;
  }
  out[0] = (uint8_t)(((fin ? 0x80 : 0) | (opcode & 0x0f)));
  if (n <= 125) {
    out[1] = (uint8_t)((masked ? 0x80 : 0) | (uint8_t)n);
  } else if (n <= 65535) {
    out[1] = (uint8_t)((masked ? 0x80 : 0) | 126);
    out[2] = (uint8_t)((n >> 8) & 0xff);
    out[3] = (uint8_t)(n & 0xff);
  } else {
    out[1] = (uint8_t)((masked ? 0x80 : 0) | 127);
    for (i = 0; i < 8; i++) {
      out[2 + i] = (uint8_t)((n >> (56 - 8 * i)) & 0xff);
    }
  }
  {
    uint8_t *body = out + hdr;
    if (masked) {
      memcpy(body, mask4, 4);
      body += 4;
      if (n > 0) {
        memcpy(body, payload, (size_t)n);
        ws_mask_xor(body, n, mask4);
      }
    } else if (n > 0) {
      memcpy(body, payload, (size_t)n);
    }
  }
  return need;
}

int64_t rynix_rt_ws_frame_encode(int64_t opcode, const char *payload, int64_t n,
                                 const uint8_t *mask4, uint8_t *out, int64_t out_cap) {
  return rynix_rt_ws_frame_encode_ex(1, opcode, payload, n, mask4, out, out_cap);
}

static int64_t ws_len_hdr_bytes(int64_t len_code) {
  if (len_code <= 125) {
    return 2;
  }
  if (len_code == 126) {
    return 4;
  }
  if (len_code == 127) {
    return 10;
  }
  return -1;
}

/* Decode one WS frame. Supports 7/16/64-bit lengths (127 capped at MAX).
 * Sets *out_fin (0/1) and *out_opcode when non-NULL. Returns payload length. */
int64_t rynix_rt_ws_frame_decode_ex(const uint8_t *in, int64_t in_len, char *payload,
                                    int64_t payload_cap, int64_t *out_fin, int64_t *out_opcode) {
  int64_t plen;
  int64_t len_code;
  int masked;
  int64_t hdr;
  int64_t len_hdr;
  const uint8_t *body;
  uint8_t mask[4];
  int i;
  if (!in || in_len < 2) {
    return -1;
  }
  if (out_fin) {
    *out_fin = (in[0] & 0x80) ? 1 : 0;
  }
  if (out_opcode) {
    *out_opcode = in[0] & 0x0f;
  }
  masked = (in[1] & 0x80) != 0;
  len_code = in[1] & 0x7f;
  len_hdr = ws_len_hdr_bytes(len_code);
  if (len_hdr < 0 || in_len < len_hdr) {
    return -1;
  }
  if (len_code <= 125) {
    plen = len_code;
  } else if (len_code == 126) {
    plen = ((int64_t)in[2] << 8) | (int64_t)in[3];
  } else {
    /* 127: 64-bit BE; MSB must be 0 per RFC. */
    if (in[2] & 0x80) {
      return -1;
    }
    plen = 0;
    for (i = 0; i < 8; i++) {
      plen = (plen << 8) | (int64_t)in[2 + i];
    }
    if (plen > RYNIX_WS_MAX_PAYLOAD) {
      return -1;
    }
  }
  hdr = len_hdr + (masked ? 4 : 0);
  if (in_len < hdr + plen) {
    return -1;
  }
  if (plen > payload_cap || (plen > 0 && !payload)) {
    return -1;
  }
  body = in + len_hdr;
  if (masked) {
    memcpy(mask, body, 4);
    body += 4;
  }
  if (plen > 0) {
    memcpy(payload, body, (size_t)plen);
    if (masked) {
      ws_mask_xor((uint8_t *)payload, plen, mask);
    }
  }
  return plen;
}

int64_t rynix_rt_ws_frame_decode(const uint8_t *in, int64_t in_len, char *payload,
                                 int64_t payload_cap, int64_t *out_opcode) {
  return rynix_rt_ws_frame_decode_ex(in, in_len, payload, payload_cap, NULL, out_opcode);
}

/* Decode a (possibly fragmented) data message from contiguous wire bytes.
 * Stops at FIN=1. Returns total payload length; *out_opcode is first frame opcode. */
int64_t rynix_rt_ws_message_decode(const uint8_t *in, int64_t in_len, char *payload,
                                   int64_t payload_cap, int64_t *out_opcode) {
  int64_t off = 0;
  int64_t total = 0;
  int64_t first_op = -1;
  if (!in || in_len < 2 || !payload || payload_cap < 0) {
    return -1;
  }
  while (off < in_len) {
    int64_t fin = 0;
    int64_t opcode = 0;
    int64_t n;
    int64_t frame_wire;
    int64_t len_code;
    int masked;
    int64_t hdr;
    n = rynix_rt_ws_frame_decode_ex(in + off, in_len - off, payload + total, payload_cap - total,
                                    &fin, &opcode);
    if (n < 0) {
      return -1;
    }
    if (first_op < 0) {
      if (opcode == 0) {
        return -1;
      }
      first_op = opcode;
    } else if (opcode != 0) {
      return -1;
    }
    total += n;
    len_code = in[off + 1] & 0x7f;
    masked = (in[off + 1] & 0x80) != 0;
    hdr = ws_len_hdr_bytes(len_code);
    if (hdr < 0) {
      return -1;
    }
    frame_wire = hdr + (masked ? 4 : 0) + n;
    off += frame_wire;
    if (fin) {
      if (out_opcode) {
        *out_opcode = first_op;
      }
      return total;
    }
  }
  return -1;
}

static int64_t ws_sock_send_all(int64_t fd, const void *buf, int64_t n) {
  const char *p = (const char *)buf;
  int64_t off = 0;
  while (off < n) {
    int64_t w = rynix_rt_tcp_send(fd, p + off, n - off);
    if (w <= 0) {
      return -1;
    }
    off += w;
  }
  return n;
}

static int64_t ws_sock_recv_at_least(int64_t fd, void *buf, int64_t want, int64_t cap) {
  char *p = (char *)buf;
  int64_t got = 0;
  while (got < want && got < cap) {
    int64_t r = rynix_rt_tcp_recv(fd, p + got, cap - got);
    if (r <= 0) {
      return got > 0 ? got : -1;
    }
    got += r;
  }
  return got;
}

/* Total on-wire bytes for one frame once the length header is complete. */
static int64_t ws_peek_frame_wire_len(const uint8_t *in, int64_t in_len) {
  int64_t len_code;
  int64_t len_hdr;
  int64_t plen;
  int masked;
  int i;
  if (!in || in_len < 2) {
    return -1;
  }
  masked = (in[1] & 0x80) != 0;
  len_code = in[1] & 0x7f;
  len_hdr = ws_len_hdr_bytes(len_code);
  if (len_hdr < 0 || in_len < len_hdr) {
    return -1;
  }
  if (len_code <= 125) {
    plen = len_code;
  } else if (len_code == 126) {
    plen = ((int64_t)in[2] << 8) | (int64_t)in[3];
  } else {
    if (in[2] & 0x80) {
      return -1;
    }
    plen = 0;
    for (i = 0; i < 8; i++) {
      plen = (plen << 8) | (int64_t)in[2 + i];
    }
    if (plen > RYNIX_WS_MAX_PAYLOAD) {
      return -1;
    }
  }
  return len_hdr + (masked ? 4 : 0) + plen;
}

/* Read one complete WS frame from TCP (heap-allocated). Caller frees *frame_out. */
static int64_t ws_tcp_read_frame(int64_t fd, uint8_t **frame_out, int64_t *frame_len_out) {
  uint8_t hdr[14];
  int64_t got;
  int64_t len_hdr;
  int64_t total;
  uint8_t *frame;
  int64_t r;

  if (!frame_out || !frame_len_out) {
    return -1;
  }
  got = ws_sock_recv_at_least(fd, hdr, 2, (int64_t)sizeof(hdr));
  if (got < 2) {
    return -1;
  }
  len_hdr = ws_len_hdr_bytes(hdr[1] & 0x7f);
  if (len_hdr < 0) {
    return -1;
  }
  while (got < len_hdr) {
    r = rynix_rt_tcp_recv(fd, hdr + got, (int64_t)sizeof(hdr) - got);
    if (r <= 0) {
      return -1;
    }
    got += r;
  }
  total = ws_peek_frame_wire_len(hdr, got);
  if (total < 0 || total > RYNIX_WS_MAX_PAYLOAD + 14) {
    return -1;
  }
  frame = (uint8_t *)malloc((size_t)total);
  if (!frame) {
    return -1;
  }
  memcpy(frame, hdr, (size_t)got);
  while (got < total) {
    r = rynix_rt_tcp_recv(fd, frame + got, total - got);
    if (r <= 0) {
      free(frame);
      return -1;
    }
    got += r;
  }
  *frame_out = frame;
  *frame_len_out = total;
  return 0;
}

static int extract_ws_key(const char *req, char *key_out, size_t key_cap) {
  const char *p = strstr(req, "Sec-WebSocket-Key:");
  size_t i = 0;
  if (!p) {
    p = strstr(req, "sec-websocket-key:");
  }
  if (!p) {
    return -1;
  }
  p = strchr(p, ':');
  if (!p) {
    return -1;
  }
  p++;
  while (*p == ' ' || *p == '\t') {
    p++;
  }
  while (*p && *p != '\r' && *p != '\n' && i + 1 < key_cap) {
    key_out[i++] = *p++;
  }
  key_out[i] = '\0';
  return i > 0 ? 0 : -1;
}

/* One-shot WS text echo: HTTP upgrade + one text frame echo. */
int64_t rynix_rt_ws_serve_once_echo(int64_t port) {
  int64_t listen_fd;
  int64_t client;
  char req[2048];
  char key[128];
  char accept[32];
  char resp[512];
  uint8_t frame[256];
  char payload[128];
  int64_t n;
  int64_t opcode = 0;
  int64_t got;
  int64_t wire;

  if (port <= 0) {
    return -1;
  }
  listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  got = ws_sock_recv_at_least(client, req, 4, (int64_t)sizeof(req) - 1);
  if (got < 0) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  req[got] = '\0';
  /* Drain until end of headers if needed. */
  while (strstr(req, "\r\n\r\n") == NULL && got < (int64_t)sizeof(req) - 1) {
    int64_t r = rynix_rt_tcp_recv(client, req + got, (int64_t)sizeof(req) - 1 - got);
    if (r <= 0) {
      break;
    }
    got += r;
    req[got] = '\0';
  }
  if (extract_ws_key(req, key, sizeof(key)) != 0) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  ws_accept_b64(key, accept);
  n = snprintf(resp, sizeof(resp),
               "HTTP/1.1 101 Switching Protocols\r\n"
               "Upgrade: websocket\r\n"
               "Connection: Upgrade\r\n"
               "Sec-WebSocket-Accept: %s\r\n"
               "\r\n",
               accept);
  if (n <= 0 || ws_sock_send_all(client, resp, n) != n) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  got = ws_sock_recv_at_least(client, frame, 2, (int64_t)sizeof(frame));
  if (got < 2) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  {
    int64_t plen = frame[1] & 0x7f;
    int64_t need = 2 + ((frame[1] & 0x80) ? 4 : 0) + plen;
    while (got < need && got < (int64_t)sizeof(frame)) {
      int64_t r = rynix_rt_tcp_recv(client, frame + got, (int64_t)sizeof(frame) - got);
      if (r <= 0) {
        break;
      }
      got += r;
    }
  }
  n = rynix_rt_ws_frame_decode(frame, got, payload, (int64_t)sizeof(payload), &opcode);
  if (n < 0) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  /* Server→client: unmasked echo, same opcode (usually text=1). */
  wire = rynix_rt_ws_frame_encode(opcode, payload, n, NULL, frame, (int64_t)sizeof(frame));
  if (wire < 0 || ws_sock_send_all(client, frame, wire) != wire) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  return 0;
}

int64_t rynix_rt_ws_client_echo(const char *host, int64_t port, const char *msg) {
  static const uint8_t mask[4] = {0x12, 0x34, 0x56, 0x78};
  int64_t fd;
  char req[512];
  char resp[1024];
  uint8_t frame[256];
  char payload[128];
  int64_t n;
  int64_t got;
  int64_t wire;
  int64_t opcode = 0;
  const char *key = "dGhlIHNhbXBsZSBub25jZQ==";

  if (!host || !msg || port <= 0) {
    return -1;
  }
  n = (int64_t)strlen(msg);
  if (n > 125) {
    return -1;
  }
  fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    return -1;
  }
  n = snprintf(req, sizeof(req),
               "GET / HTTP/1.1\r\n"
               "Host: %s\r\n"
               "Upgrade: websocket\r\n"
               "Connection: Upgrade\r\n"
               "Sec-WebSocket-Key: %s\r\n"
               "Sec-WebSocket-Version: 13\r\n"
               "\r\n",
               host, key);
  if (n <= 0 || ws_sock_send_all(fd, req, n) != n) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  got = ws_sock_recv_at_least(fd, resp, 12, (int64_t)sizeof(resp) - 1);
  if (got < 0) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  resp[got] = '\0';
  while (strstr(resp, "\r\n\r\n") == NULL && got < (int64_t)sizeof(resp) - 1) {
    int64_t r = rynix_rt_tcp_recv(fd, resp + got, (int64_t)sizeof(resp) - 1 - got);
    if (r <= 0) {
      break;
    }
    got += r;
    resp[got] = '\0';
  }
  if (strstr(resp, "101") == NULL) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  n = (int64_t)strlen(msg);
  wire = rynix_rt_ws_frame_encode(1 /* text */, msg, n, mask, frame, (int64_t)sizeof(frame));
  if (wire < 0 || ws_sock_send_all(fd, frame, wire) != wire) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  got = ws_sock_recv_at_least(fd, frame, 2, (int64_t)sizeof(frame));
  if (got < 2) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  {
    int64_t plen = frame[1] & 0x7f;
    int64_t need = 2 + ((frame[1] & 0x80) ? 4 : 0) + plen;
    while (got < need && got < (int64_t)sizeof(frame)) {
      int64_t r = rynix_rt_tcp_recv(fd, frame + got, (int64_t)sizeof(frame) - got);
      if (r <= 0) {
        break;
      }
      got += r;
    }
  }
  wire = rynix_rt_ws_frame_decode(frame, got, payload, (int64_t)sizeof(payload), &opcode);
  rynix_rt_tcp_close(fd);
  if (wire != n || opcode != 1 || memcmp(payload, msg, (size_t)n) != 0) {
    return -1;
  }
  return 0;
}

/* Large-payload WS echo (16-bit or 64-bit length on wire). msg may be non-NUL. */
int64_t rynix_rt_ws_serve_once_echo_n(int64_t port, const char *msg, int64_t n) {
  int64_t listen_fd;
  int64_t client;
  char accept[32];
  uint8_t *in_frame = NULL;
  int64_t in_len = 0;
  char *payload = NULL;
  uint8_t *out_frame = NULL;
  int64_t opcode = 0;
  int64_t plen;
  int64_t wire;
  int64_t out_cap;

  if (port <= 0 || n < 0 || n > RYNIX_WS_MAX_PAYLOAD || (n > 0 && !msg)) {
    return -1;
  }
  listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  {
    char req[2048];
    char key[128];
    char resp[512];
    int64_t got;
    int64_t rn;
    got = ws_sock_recv_at_least(client, req, 4, (int64_t)sizeof(req) - 1);
    if (got < 0) {
      goto fail;
    }
    req[got] = '\0';
    while (strstr(req, "\r\n\r\n") == NULL && got < (int64_t)sizeof(req) - 1) {
      int64_t r = rynix_rt_tcp_recv(client, req + got, (int64_t)sizeof(req) - 1 - got);
      if (r <= 0) {
        break;
      }
      got += r;
      req[got] = '\0';
    }
    if (extract_ws_key(req, key, sizeof(key)) != 0) {
      goto fail;
    }
    ws_accept_b64(key, accept);
    rn = snprintf(resp, sizeof(resp),
                  "HTTP/1.1 101 Switching Protocols\r\n"
                  "Upgrade: websocket\r\n"
                  "Connection: Upgrade\r\n"
                  "Sec-WebSocket-Accept: %s\r\n"
                  "\r\n",
                  accept);
    if (rn <= 0 || ws_sock_send_all(client, resp, rn) != rn) {
      goto fail;
    }
  }
  if (ws_tcp_read_frame(client, &in_frame, &in_len) != 0) {
    goto fail;
  }
  payload = (char *)malloc((size_t)n + 1);
  if (!payload) {
    goto fail;
  }
  plen = rynix_rt_ws_frame_decode(in_frame, in_len, payload, n, &opcode);
  free(in_frame);
  in_frame = NULL;
  if (plen != n || (n > 0 && memcmp(payload, msg, (size_t)n) != 0)) {
    goto fail;
  }
  out_cap = 10 + n;
  out_frame = (uint8_t *)malloc((size_t)out_cap);
  if (!out_frame) {
    goto fail;
  }
  wire = rynix_rt_ws_frame_encode(opcode, payload, n, NULL, out_frame, out_cap);
  free(payload);
  payload = NULL;
  if (wire < 0 || ws_sock_send_all(client, out_frame, wire) != wire) {
    goto fail;
  }
  free(out_frame);
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  return 0;
fail:
  free(in_frame);
  free(payload);
  free(out_frame);
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  return -1;
}

int64_t rynix_rt_ws_client_echo_n(const char *host, int64_t port, const char *msg, int64_t n) {
  static const uint8_t mask[4] = {0x12, 0x34, 0x56, 0x78};
  int64_t fd;
  char req[512];
  char resp[1024];
  uint8_t *out_frame = NULL;
  uint8_t *in_frame = NULL;
  int64_t in_len = 0;
  char *back = NULL;
  int64_t got;
  int64_t wire;
  int64_t out_cap;
  int64_t opcode = 0;
  const char *key = "dGhlIHNhbXBsZSBub25jZQ==";

  if (!host || port <= 0 || n < 0 || n > RYNIX_WS_MAX_PAYLOAD || (n > 0 && !msg)) {
    return -1;
  }
  fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    return -1;
  }
  got = snprintf(req, sizeof(req),
                 "GET / HTTP/1.1\r\n"
                 "Host: %s\r\n"
                 "Upgrade: websocket\r\n"
                 "Connection: Upgrade\r\n"
                 "Sec-WebSocket-Key: %s\r\n"
                 "Sec-WebSocket-Version: 13\r\n"
                 "\r\n",
                 host, key);
  if (got <= 0 || ws_sock_send_all(fd, req, got) != got) {
    goto fail;
  }
  got = ws_sock_recv_at_least(fd, resp, 12, (int64_t)sizeof(resp) - 1);
  if (got < 0) {
    goto fail;
  }
  resp[got] = '\0';
  while (strstr(resp, "\r\n\r\n") == NULL && got < (int64_t)sizeof(resp) - 1) {
    int64_t r = rynix_rt_tcp_recv(fd, resp + got, (int64_t)sizeof(resp) - 1 - got);
    if (r <= 0) {
      break;
    }
    got += r;
    resp[got] = '\0';
  }
  if (strstr(resp, "101") == NULL) {
    goto fail;
  }
  out_cap = 10 + 4 + n;
  out_frame = (uint8_t *)malloc((size_t)out_cap);
  if (!out_frame) {
    goto fail;
  }
  wire = rynix_rt_ws_frame_encode(1 /* text */, msg, n, mask, out_frame, out_cap);
  if (wire < 0 || ws_sock_send_all(fd, out_frame, wire) != wire) {
    goto fail;
  }
  free(out_frame);
  out_frame = NULL;
  if (ws_tcp_read_frame(fd, &in_frame, &in_len) != 0) {
    goto fail;
  }
  back = (char *)malloc((size_t)n);
  if (!back) {
    goto fail;
  }
  wire = rynix_rt_ws_frame_decode(in_frame, in_len, back, n, &opcode);
  free(in_frame);
  in_frame = NULL;
  rynix_rt_tcp_close(fd);
  if (wire != n || opcode != 1 || (n > 0 && memcmp(back, msg, (size_t)n) != 0)) {
    free(back);
    return -1;
  }
  free(back);
  return 0;
fail:
  free(out_frame);
  free(in_frame);
  free(back);
  rynix_rt_tcp_close(fd);
  return -1;
}

/* Offline KATs: short, 16-bit, 64-bit, and fragmented message. Returns 0 on OK. */
int64_t rynix_rt_ws_frame_roundtrip_ok(void) {
  static const uint8_t mask[4] = {0xaa, 0xbb, 0xcc, 0xdd};
  const char *msg = "rynix-ws";
  uint8_t wire[700];
  char back[300];
  int64_t opcode = -1;
  int64_t w;
  int64_t n;
  char big[200];
  int i;
  const int64_t huge_n = 70000;
  uint8_t *huge_wire = NULL;
  char *huge_pay = NULL;
  char *huge_back = NULL;

  w = rynix_rt_ws_frame_encode(1, msg, (int64_t)strlen(msg), mask, wire, (int64_t)sizeof(wire));
  if (w < 0) {
    return -1;
  }
  n = rynix_rt_ws_frame_decode(wire, w, back, (int64_t)sizeof(back), &opcode);
  if (n != (int64_t)strlen(msg) || opcode != 1 || memcmp(back, msg, (size_t)n) != 0) {
    return -1;
  }

  /* Extended 16-bit length (>125). */
  for (i = 0; i < 200; i++) {
    big[i] = (char)('A' + (i % 26));
  }
  w = rynix_rt_ws_frame_encode(1, big, 200, mask, wire, (int64_t)sizeof(wire));
  if (w < 0 || wire[1] != (uint8_t)(0x80 | 126)) {
    return -1;
  }
  n = rynix_rt_ws_frame_decode(wire, w, back, (int64_t)sizeof(back), &opcode);
  if (n != 200 || opcode != 1 || memcmp(back, big, 200) != 0) {
    return -1;
  }

  /* 64-bit length (127) for payload >65535. */
  huge_pay = (char *)malloc((size_t)huge_n);
  huge_back = (char *)malloc((size_t)huge_n);
  huge_wire = (uint8_t *)malloc((size_t)huge_n + 64);
  if (!huge_pay || !huge_back || !huge_wire) {
    free(huge_pay);
    free(huge_back);
    free(huge_wire);
    return -1;
  }
  for (i = 0; i < (int)huge_n; i++) {
    huge_pay[i] = (char)(i & 0xff);
  }
  w = rynix_rt_ws_frame_encode(1, huge_pay, huge_n, mask, huge_wire, huge_n + 64);
  if (w < 0 || (huge_wire[1] & 0x7f) != 127) {
    free(huge_pay);
    free(huge_back);
    free(huge_wire);
    return -1;
  }
  n = rynix_rt_ws_frame_decode(huge_wire, w, huge_back, huge_n, &opcode);
  if (n != huge_n || opcode != 1 || memcmp(huge_back, huge_pay, (size_t)huge_n) != 0) {
    free(huge_pay);
    free(huge_back);
    free(huge_wire);
    return -1;
  }
  free(huge_pay);
  free(huge_back);
  free(huge_wire);

  /* Fragmented: FIN=0 text + FIN=1 continuation. */
  {
    int64_t w1 = rynix_rt_ws_frame_encode_ex(0, 1, "hello", 5, mask, wire, (int64_t)sizeof(wire));
    int64_t w2;
    if (w1 < 0) {
      return -1;
    }
    w2 = rynix_rt_ws_frame_encode_ex(1, 0, "-world", 6, mask, wire + w1,
                                     (int64_t)sizeof(wire) - w1);
    if (w2 < 0) {
      return -1;
    }
    n = rynix_rt_ws_message_decode(wire, w1 + w2, back, (int64_t)sizeof(back), &opcode);
    if (n != 11 || opcode != 1 || memcmp(back, "hello-world", 11) != 0) {
      return -1;
    }
  }
  return 0;
}
