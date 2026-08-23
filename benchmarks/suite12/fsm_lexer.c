/* Suite12 #9 — FSM lexer (End suite12_c.c peer path).
 * Locked to C/Zig/Rust/Go checksum; End C11 emit historically diverges — do not
 * claim End match for this id without re-verifying End output.
 */

#include <stdint.h>
#include <stdio.h>
#include <string.h>

int64_t bench_9_fsm_lexer(void) {
  const char *sample =
      "pub fn calculate_metrics(id: u64, active: bool) -> i64 { val base = id * 31; ret base + 10; } ";
  int sample_len = (int)strlen(sample);
  int64_t token_count = 0;
  int64_t token_hash = 0;

  enum State { STATE_START, STATE_IDENT, STATE_NUMBER, STATE_OP };
  enum State st = STATE_START;

  for (int i = 0; i < 10000000; i++) {
    char c = sample[i % sample_len];
    switch (st) {
    case STATE_START:
      if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_') {
        st = STATE_IDENT;
      } else if (c >= '0' && c <= '9') {
        st = STATE_NUMBER;
      } else if (c != ' ' && c != '\n' && c != '\t') {
        st = STATE_OP;
      }
      break;
    case STATE_IDENT:
      if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') ||
            c == '_')) {
        token_count++;
        token_hash = (token_hash * 33) + 1;
        st = STATE_START;
      }
      break;
    case STATE_NUMBER:
      if (!(c >= '0' && c <= '9')) {
        token_count++;
        token_hash = (token_hash * 33) + 2;
        st = STATE_START;
      }
      break;
    case STATE_OP:
      token_count++;
      token_hash = (token_hash * 33) + 3;
      st = STATE_START;
      break;
    }
  }
  return token_hash + token_count;
}

int main(void) {
  printf("checksum=%lld\n", (long long)bench_9_fsm_lexer());
  return 0;
}
