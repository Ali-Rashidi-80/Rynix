# Examples



Runnable samples (need `clang` for `build` / `run`):



| File | Shows |

|------|--------|

| `01_hello.ryx` | print |

| `02_match_loop.ryx` | match + loop |

| `03_vec.ryx` | `Vec[i64]` methods |

| `04_bool_logic.ryx` | `and` / `or` + match |

| `05_http_json.ryx` | `json_get_i64` (no network) |



```sh

rynixc run examples/01_hello.ryx

rynixc run examples/05_http_json.ryx    # prints 42

rynixc build examples/03_vec.ryx -o target/ex_vec --runtime=portable

```

