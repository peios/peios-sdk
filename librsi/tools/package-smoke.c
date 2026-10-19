/* Compile and link smoke test for the installed librsi development surface. */
#include <rsi.h>

int main(void) {
    int (*volatile entry)(const struct rsi_hive *, uint32_t, uint64_t) = rsi_register;
    return entry == 0;
}
