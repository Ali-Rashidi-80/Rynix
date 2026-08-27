#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../include/rynix_rt.h"

/* Read until end-of-headers (or buffer full / peer close). Does not wait for
 * peer close after headers — that deadlocks request/response pairs.
 * Small POST/GET bodies sent in one write arrive with the headers. */
static int64_t http_recv_message(int64_t fd, char *buf, int64_t cap) {
  int64_t total = 0;
  for (;;) {
    if (total >= cap - 1) {
      break;
    }
    int64_t got = rynix_rt_tcp_recv(fd, buf + total, cap - 1 - total);
    if (got <= 0) {
      break;
    }
    total += got;
    buf[total] = '\0';
    if (strstr(buf, "\r\n\r\n") != NULL) {
      break;
    }
  }
  buf[total] = '\0';
  return total;
}

static const char *http_body(const char *msg) {
  const char *body = strstr(msg, "\r\n\r\n");
  if (!body) {
    return NULL;
  }
  return body + 4;
}

/* HTTP/1.1 GET with JSON body field extraction (soft std/http surface). */
int64_t rynix_rt_http_get_json_i64(const char *host, int64_t port, const char *path,
                                   const char *field) {
  if (!host || !path || !field) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    return -1;
  }
  char req[512];
  int n = snprintf(req, sizeof(req),
                   "GET %s HTTP/1.1\r\nHost: %s\r\nConnection: close\r\n\r\n", path,
                   host);
  if (n <= 0 || n >= (int)sizeof(req)) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  if (rynix_rt_tcp_send(fd, req, (int64_t)n) != (int64_t)n) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  char buf[4096];
  if (http_recv_message(fd, buf, (int64_t)sizeof(buf)) <= 0) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  rynix_rt_tcp_close(fd);
  const char *body = http_body(buf);
  if (!body) {
    return -1;
  }
  return rynix_rt_json_get_i64(body, field);
}

/* HTTP/1.1 POST JSON body; parse integer `field` from response body. */
int64_t rynix_rt_http_post_json_i64(const char *host, int64_t port, const char *path,
                                    const char *json_body, const char *field) {
  if (!host || !path || !json_body || !field) {
    return -1;
  }
  int64_t fd = rynix_rt_tcp_connect(host, port);
  if (fd < 0) {
    return -1;
  }
  int body_len = (int)strlen(json_body);
  char req[2048];
  int n = snprintf(req, sizeof(req),
                   "POST %s HTTP/1.1\r\n"
                   "Host: %s\r\n"
                   "Content-Type: application/json\r\n"
                   "Content-Length: %d\r\n"
                   "Connection: close\r\n"
                   "\r\n"
                   "%s",
                   path, host, body_len, json_body);
  if (n <= 0 || n >= (int)sizeof(req)) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  if (rynix_rt_tcp_send(fd, req, (int64_t)n) != (int64_t)n) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  char buf[4096];
  if (http_recv_message(fd, buf, (int64_t)sizeof(buf)) <= 0) {
    rynix_rt_tcp_close(fd);
    return -1;
  }
  rynix_rt_tcp_close(fd);
  const char *body = http_body(buf);
  if (!body) {
    return -1;
  }
  return rynix_rt_json_get_i64(body, field);
}

int64_t rynix_rt_http_serve_once_json_i64(int64_t port, const char *path, int64_t value) {
  if (!path || port <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }

  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
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
  if (body_n <= 0 || body_n >= (int)sizeof(body)) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }

  char resp[512];
  int resp_n;
  if (path_ok) {
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
  if (resp_n <= 0 || resp_n >= (int)sizeof(resp)) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  int64_t sent = rynix_rt_tcp_send(client, resp, (int64_t)resp_n);
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  if (sent != (int64_t)resp_n) {
    return -1;
  }
  return path_ok ? 0 : 1;
}

/* Respond to one accepted client. Returns 1 if a matching GET was served
 * (200), 0 if a non-matching request got a 404, -1 on I/O error.
 * When `path_b` / `path_c` are non-NULL, either path may match. */
static int64_t http_serve_one_matching_get_json_paths(int64_t client, const char *path_a,
                                                      int64_t value_a, const char *path_b,
                                                      int64_t value_b, const char *path_c,
                                                      int64_t value_c) {
  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    return -1;
  }

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    return -1;
  }
  *line_end = '\0';
  char *method = req;
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  if (sp1) {
    *sp1 = '\0';
  }
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }

  int64_t value = value_a;
  int match = 0;
  if (req_path && strcmp(method, "GET") == 0) {
    if (path_a && strcmp(req_path, path_a) == 0) {
      match = 1;
      value = value_a;
    } else if (path_b && strcmp(req_path, path_b) == 0) {
      match = 1;
      value = value_b;
    } else if (path_c && strcmp(req_path, path_c) == 0) {
      match = 1;
      value = value_c;
    }
  }

  char body[128];
  int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)value);
  if (body_n <= 0 || body_n >= (int)sizeof(body)) {
    return -1;
  }

  char resp[512];
  int resp_n;
  if (match) {
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
  if (resp_n <= 0 || resp_n >= (int)sizeof(resp)) {
    return -1;
  }
  if (rynix_rt_tcp_send(client, resp, (int64_t)resp_n) != (int64_t)resp_n) {
    return -1;
  }
  return match ? 1 : 0;
}

static int64_t http_serve_one_matching_get_json_i64(int64_t client, const char *path,
                                                    int64_t value) {
  return http_serve_one_matching_get_json_paths(client, path, value, NULL, 0, NULL, 0);
}

int64_t rynix_rt_http_serve_loop_json_i64(int64_t port, const char *path, int64_t value,
                                          int64_t max_reqs) {
  if (!path || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc = http_serve_one_matching_get_json_i64(client, path, value);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(listen_fd);
  return 0;
}

int64_t rynix_rt_http_serve_loop_2paths_json_i64(int64_t port, const char *path_a,
                                                 int64_t value_a, const char *path_b,
                                                 int64_t value_b, int64_t max_reqs) {
  if (!path_a || !path_b || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc =
        http_serve_one_matching_get_json_paths(client, path_a, value_a, path_b, value_b, NULL, 0);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(listen_fd);
  return 0;
}

int64_t rynix_rt_http_serve_loop_3paths_json_i64(int64_t port, const char *path_a,
                                                 int64_t value_a, const char *path_b,
                                                 int64_t value_b, const char *path_c,
                                                 int64_t value_c, int64_t max_reqs) {
  if (!path_a || !path_b || !path_c || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc = http_serve_one_matching_get_json_paths(client, path_a, value_a, path_b, value_b,
                                                        path_c, value_c);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(listen_fd);
  return 0;
}

/* GET {prefix}{digits} → echo parsed i64 as JSON value. Returns 1 match, 0 miss, -1 I/O. */
static int64_t http_serve_one_matching_get_json_path_param(int64_t client, const char *prefix) {
  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    return -1;
  }

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    return -1;
  }
  *line_end = '\0';
  char *method = req;
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  if (sp1) {
    *sp1 = '\0';
  }
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }

  int match = 0;
  int64_t value = 0;
  if (req_path && prefix && strcmp(method, "GET") == 0) {
    size_t plen = strlen(prefix);
    if (plen > 0 && strncmp(req_path, prefix, plen) == 0) {
      const char *rest = req_path + plen;
      if (rest[0] != '\0') {
        char *end = NULL;
        long long parsed = strtoll(rest, &end, 10);
        if (end && end != rest && *end == '\0') {
          match = 1;
          value = (int64_t)parsed;
        }
      }
    }
  }

  char body[128];
  int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)value);
  if (body_n <= 0 || body_n >= (int)sizeof(body)) {
    return -1;
  }

  char resp[512];
  int resp_n;
  if (match) {
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
  if (resp_n <= 0 || resp_n >= (int)sizeof(resp)) {
    return -1;
  }
  if (rynix_rt_tcp_send(client, resp, (int64_t)resp_n) != (int64_t)resp_n) {
    return -1;
  }
  return match ? 1 : 0;
}

int64_t rynix_rt_http_serve_loop_path_param_json_i64(int64_t port, const char *prefix,
                                                     int64_t max_reqs) {
  if (!prefix || prefix[0] == '\0' || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc = http_serve_one_matching_get_json_path_param(client, prefix);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(listen_fd);
  return 0;
}

/* Case-insensitive ASCII compare of `n` bytes. */
static int http_strncasecmp(const char *a, const char *b, size_t n) {
  for (size_t i = 0; i < n; i++) {
    unsigned char ca = (unsigned char)a[i];
    unsigned char cb = (unsigned char)b[i];
    if (ca >= 'A' && ca <= 'Z') {
      ca = (unsigned char)(ca - 'A' + 'a');
    }
    if (cb >= 'A' && cb <= 'Z') {
      cb = (unsigned char)(cb - 'A' + 'a');
    }
    if (ca != cb) {
      return (int)ca - (int)cb;
    }
  }
  return 0;
}

/* Find header `name` (case-insensitive) value start; returns NULL if missing. */
static const char *http_header_value(const char *msg, const char *name) {
  if (!msg || !name || name[0] == '\0') {
    return NULL;
  }
  size_t nlen = strlen(name);
  const char *p = msg;
  for (;;) {
    const char *line = strstr(p, "\r\n");
    if (!line) {
      return NULL;
    }
    line += 2;
    if (line[0] == '\r' && line[1] == '\n') {
      return NULL; /* end of headers */
    }
    if (http_strncasecmp(line, name, nlen) == 0 && line[nlen] == ':') {
      const char *v = line + nlen + 1;
      while (*v == ' ' || *v == '\t') {
        v++;
      }
      return v;
    }
    p = line;
  }
}

static int http_header_i64(const char *msg, const char *name, int64_t *out) {
  const char *v = http_header_value(msg, name);
  if (!v) {
    return 0;
  }
  char *end = NULL;
  long long parsed = strtoll(v, &end, 10);
  if (!end || end == v) {
    return 0;
  }
  /* Value ends at CR or optional trailing spaces before CR. */
  while (*end == ' ' || *end == '\t') {
    end++;
  }
  if (*end != '\r' && *end != '\n' && *end != '\0') {
    return 0;
  }
  *out = (int64_t)parsed;
  return 1;
}

static int64_t http_content_length(const char *msg) {
  int64_t n = -1;
  if (!http_header_i64(msg, "Content-Length", &n)) {
    return -1;
  }
  return n;
}

/* Ensure full body (per Content-Length) is in `buf`. Headers already present. */
static int64_t http_finish_body(int64_t fd, char *buf, int64_t cap, int64_t total) {
  const char *body = http_body(buf);
  if (!body) {
    return total;
  }
  int64_t header_bytes = (int64_t)(body - buf);
  int64_t cl = http_content_length(buf);
  if (cl < 0) {
    return total;
  }
  while (total - header_bytes < cl) {
    if (total >= cap - 1) {
      break;
    }
    int64_t got = rynix_rt_tcp_recv(fd, buf + total, cap - 1 - total);
    if (got <= 0) {
      break;
    }
    total += got;
    buf[total] = '\0';
  }
  return total;
}

static int64_t http_send_json_resp(int64_t client, int status, const char *reason,
                                   const char *json_body, int keepalive) {
  int body_n = (int)strlen(json_body);
  char resp[512];
  int resp_n = snprintf(resp, sizeof(resp),
                        "HTTP/1.1 %d %s\r\n"
                        "Content-Type: application/json\r\n"
                        "Content-Length: %d\r\n"
                        "Connection: %s\r\n"
                        "\r\n"
                        "%s",
                        status, reason, body_n, keepalive ? "keep-alive" : "close", json_body);
  if (resp_n <= 0 || resp_n >= (int)sizeof(resp)) {
    return -1;
  }
  if (rynix_rt_tcp_send(client, resp, (int64_t)resp_n) != (int64_t)resp_n) {
    return -1;
  }
  return 0;
}

/* Returns 1 match (header echoed), 0 miss, -1 I/O. */
static int64_t http_serve_one_matching_get_json_header(int64_t client, const char *path,
                                                       const char *header_name) {
  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    return -1;
  }

  int64_t hdr_val = 0;
  int has_hdr = http_header_i64(req, header_name, &hdr_val);

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    return -1;
  }
  *line_end = '\0';
  char *method = req;
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  if (sp1) {
    *sp1 = '\0';
  }
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }

  int match = 0;
  if (req_path && path && strcmp(method, "GET") == 0 && strcmp(req_path, path) == 0 && has_hdr) {
    match = 1;
  }

  if (match) {
    char body[128];
    int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)hdr_val);
    if (body_n <= 0 || body_n >= (int)sizeof(body)) {
      return -1;
    }
    if (http_send_json_resp(client, 200, "OK", body, 0) != 0) {
      return -1;
    }
    return 1;
  }
  if (req_path && path && strcmp(method, "GET") == 0 && strcmp(req_path, path) == 0) {
    if (http_send_json_resp(client, 400, "Bad Request", "{\"error\":\"bad_header\"}", 0) != 0) {
      return -1;
    }
    return 0;
  }
  if (http_send_json_resp(client, 404, "Not Found", "{\"error\":\"not_found\"}", 0) != 0) {
    return -1;
  }
  return 0;
}

int64_t rynix_rt_http_serve_loop_header_json_i64(int64_t port, const char *path,
                                                 const char *header_name, int64_t max_reqs) {
  if (!path || !header_name || header_name[0] == '\0' || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc = http_serve_one_matching_get_json_header(client, path, header_name);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(listen_fd);
  return 0;
}

/* Returns 1 auth ok, 0 miss/reject (still responded), -1 I/O. */
static int64_t http_serve_one_bearer(int64_t client, const char *path,
                                     const char *expected_token) {
  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    return -1;
  }

  const char *auth = http_header_value(req, "Authorization");

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    return -1;
  }
  *line_end = '\0';
  char *method = req;
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  if (sp1) {
    *sp1 = '\0';
  }
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }

  int path_ok = req_path && path && strcmp(method, "GET") == 0 && strcmp(req_path, path) == 0;
  if (!path_ok) {
    if (http_send_json_resp(client, 404, "Not Found", "{\"error\":\"not_found\"}", 0) != 0) {
      return -1;
    }
    return 0;
  }

  int ok = 0;
  if (auth && expected_token && expected_token[0] != '\0') {
    const char *prefix = "Bearer ";
    size_t plen = 7;
    if (strncmp(auth, prefix, plen) == 0) {
      const char *tok = auth + plen;
      size_t elen = strlen(expected_token);
      size_t i = 0;
      while (i < elen && tok[i] != '\0' && tok[i] != '\r' && tok[i] != '\n' && tok[i] != ' ') {
        if (tok[i] != expected_token[i]) {
          break;
        }
        i++;
      }
      if (i == elen && (tok[i] == '\0' || tok[i] == '\r' || tok[i] == '\n' || tok[i] == ' ')) {
        ok = 1;
      }
    }
  }
  if (ok) {
    if (http_send_json_resp(client, 200, "OK", "{\"value\": 1}", 0) != 0) {
      return -1;
    }
    return 1;
  }
  if (http_send_json_resp(client, 401, "Unauthorized", "{\"error\":\"unauthorized\"}", 0) != 0) {
    return -1;
  }
  return 0;
}

int64_t rynix_rt_http_serve_loop_bearer_json_i64(int64_t port, const char *path,
                                                 const char *expected_token, int64_t max_reqs) {
  if (!path || !expected_token || expected_token[0] == '\0' || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc = http_serve_one_bearer(client, path, expected_token);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }
  rynix_rt_tcp_close(listen_fd);
  return 0;
}

/* Returns 1 echo ok, 0 miss/reject (still responded), -1 I/O. */
static int64_t http_serve_one_post_echo_bounded(int64_t client, const char *path,
                                                const char *field, int64_t max_body) {
  char req[8192];
  int64_t total = http_recv_message(client, req, (int64_t)sizeof(req));
  if (total <= 0) {
    return -1;
  }
  total = http_finish_body(client, req, (int64_t)sizeof(req), total);

  int64_t cl = http_content_length(req);
  const char *req_body = http_body(req);
  int64_t body_len = 0;
  if (req_body) {
    body_len = cl >= 0 ? cl : (int64_t)strlen(req_body);
  }
  int too_large = (body_len > max_body) || (cl >= 0 && cl > max_body);

  /* Peek JSON before mutating the request-line. */
  int has_field = !too_large && req_body && rynix_rt_json_has_i64(req_body, field);
  int64_t parsed = has_field ? rynix_rt_json_get_i64(req_body, field) : -1;

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    return -1;
  }
  *line_end = '\0';
  char *method = req;
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  if (sp1) {
    *sp1 = '\0';
  }
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }

  int path_ok =
      req_path && path && strcmp(method, "POST") == 0 && strcmp(req_path, path) == 0;

  if (path_ok && too_large) {
    if (http_send_json_resp(client, 400, "Bad Request", "{\"error\":\"body_too_large\"}", 0) !=
        0) {
      return -1;
    }
    return 0;
  }
  if (path_ok && has_field) {
    char body[128];
    int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)parsed);
    if (body_n <= 0 || body_n >= (int)sizeof(body)) {
      return -1;
    }
    if (http_send_json_resp(client, 200, "OK", body, 0) != 0) {
      return -1;
    }
    return 1;
  }
  if (!path_ok) {
    if (http_send_json_resp(client, 404, "Not Found", "{\"error\":\"not_found\"}", 0) != 0) {
      return -1;
    }
    return 0;
  }
  if (http_send_json_resp(client, 400, "Bad Request", "{\"error\":\"bad_json\"}", 0) != 0) {
    return -1;
  }
  return 0;
}

int64_t rynix_rt_http_serve_loop_post_echo_json_i64(int64_t port, const char *path,
                                                    const char *field, int64_t max_reqs,
                                                    int64_t max_body) {
  if (!path || !field || port <= 0 || max_reqs <= 0 || max_body < 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int64_t client = rynix_rt_tcp_accept(listen_fd);
    if (client < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    int64_t rc = http_serve_one_post_echo_bounded(client, path, field, max_body);
    rynix_rt_tcp_close(client);
    if (rc < 0) {
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(listen_fd);
  return 0;
}

/* Keep-alive: 1 match, 0 miss, -1 I/O. Close connection only on final match. */
static int64_t http_serve_one_keepalive_get_json(int64_t client, const char *path, int64_t value,
                                                 int close_on_match) {
  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    return -1;
  }

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    return -1;
  }
  *line_end = '\0';
  char *method = req;
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  if (sp1) {
    *sp1 = '\0';
  }
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }

  int match = req_path && path && strcmp(method, "GET") == 0 && strcmp(req_path, path) == 0;

  if (match) {
    char body[128];
    int body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)value);
    if (body_n <= 0 || body_n >= (int)sizeof(body)) {
      return -1;
    }
    if (http_send_json_resp(client, 200, "OK", body, close_on_match ? 0 : 1) != 0) {
      return -1;
    }
    return 1;
  }
  if (http_send_json_resp(client, 404, "Not Found", "{\"error\":\"not_found\"}", 1) != 0) {
    return -1;
  }
  return 0;
}

int64_t rynix_rt_http_serve_loop_keepalive_json_i64(int64_t port, const char *path,
                                                    int64_t value, int64_t max_reqs) {
  if (!path || port <= 0 || max_reqs <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }

  int64_t served = 0;
  while (served < max_reqs) {
    int close_on_match = (served + 1 >= max_reqs);
    int64_t rc = http_serve_one_keepalive_get_json(client, path, value, close_on_match);
    if (rc < 0) {
      rynix_rt_tcp_close(client);
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    if (rc == 1) {
      served++;
    }
  }

  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  return 0;
}

int64_t rynix_rt_http_serve_once_echo_json_i64(int64_t port, const char *path,
                                               const char *field) {
  if (!path || !field || port <= 0) {
    return -1;
  }
  int64_t listen_fd = rynix_rt_tcp_listen(port);
  if (listen_fd < 0) {
    return -1;
  }
  int64_t client = rynix_rt_tcp_accept(listen_fd);
  if (client < 0) {
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }

  char req[2048];
  if (http_recv_message(client, req, (int64_t)sizeof(req)) <= 0) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }

  /* Peek JSON before mutating the request-line (strstr stops at NUL). */
  const char *req_body = http_body(req);
  int has_field = req_body && rynix_rt_json_has_i64(req_body, field);
  int64_t parsed = has_field ? rynix_rt_json_get_i64(req_body, field) : -1;

  char *line_end = strstr(req, "\r\n");
  if (!line_end) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  *line_end = '\0';
  char *sp1 = strchr(req, ' ');
  char *req_path = sp1 ? sp1 + 1 : NULL;
  char *sp2 = req_path ? strchr(req_path, ' ') : NULL;
  if (sp2) {
    *sp2 = '\0';
  }
  int path_ok = req_path && strcmp(req_path, path) == 0;
  has_field = path_ok && has_field;

  char body[128];
  int body_n;
  char resp[512];
  int resp_n;
  if (has_field) {
    body_n = snprintf(body, sizeof(body), "{\"value\": %lld}", (long long)parsed);
    if (body_n <= 0 || body_n >= (int)sizeof(body)) {
      rynix_rt_tcp_close(client);
      rynix_rt_tcp_close(listen_fd);
      return -1;
    }
    resp_n = snprintf(resp, sizeof(resp),
                      "HTTP/1.1 200 OK\r\n"
                      "Content-Type: application/json\r\n"
                      "Content-Length: %d\r\n"
                      "Connection: close\r\n"
                      "\r\n"
                      "%s",
                      body_n, body);
  } else if (!path_ok) {
    static const char not_found[] = "{\"error\":\"not_found\"}";
    resp_n = snprintf(resp, sizeof(resp),
                      "HTTP/1.1 404 Not Found\r\n"
                      "Content-Type: application/json\r\n"
                      "Content-Length: %d\r\n"
                      "Connection: close\r\n"
                      "\r\n"
                      "%s",
                      (int)(sizeof(not_found) - 1), not_found);
  } else {
    static const char bad[] = "{\"error\":\"bad_json\"}";
    resp_n = snprintf(resp, sizeof(resp),
                      "HTTP/1.1 400 Bad Request\r\n"
                      "Content-Type: application/json\r\n"
                      "Content-Length: %d\r\n"
                      "Connection: close\r\n"
                      "\r\n"
                      "%s",
                      (int)(sizeof(bad) - 1), bad);
  }
  if (resp_n <= 0 || resp_n >= (int)sizeof(resp)) {
    rynix_rt_tcp_close(client);
    rynix_rt_tcp_close(listen_fd);
    return -1;
  }
  int64_t sent = rynix_rt_tcp_send(client, resp, (int64_t)resp_n);
  rynix_rt_tcp_close(client);
  rynix_rt_tcp_close(listen_fd);
  if (sent != (int64_t)resp_n) {
    return -1;
  }
  if (!path_ok) {
    return 1;
  }
  if (!has_field) {
    return -1;
  }
  return parsed;
}
