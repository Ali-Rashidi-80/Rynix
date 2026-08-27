# Tutorials (runnable)

Five beginner tutorials under `examples/tutorial_0N_*.ryx`:

| # | File | Theme |
|---|------|-------|
| 01 | [tutorial_01_hello.ryx](../../examples/tutorial_01_hello.ryx) | print_i64 |
| 02 | [tutorial_02_match.ryx](../../examples/tutorial_02_match.ryx) | nullary match |
| 03 | [tutorial_03_vec.ryx](../../examples/tutorial_03_vec.ryx) | Vec[i64] |
| 04 | [tutorial_04_map.ryx](../../examples/tutorial_04_map.ryx) | Map[str,i64] |
| 05 | [tutorial_05_agent_check.ryx](../../examples/tutorial_05_agent_check.ryx) | agent check path |

```sh
rynixc run examples/tutorial_01_hello.ryx
rynixc check examples/tutorial_05_agent_check.ryx --explain-alloc --error-format=json
```

See also [01_getting_started.md](01_getting_started.md) and [SPEC.md](../SPEC.md).
