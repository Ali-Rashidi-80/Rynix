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
