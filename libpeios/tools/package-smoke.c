/* Compile and link smoke test for the installed libpeios development surface. */
#include <peios.h>

int main(void) {
    const void *volatile mapping = &peios_token_generic_mapping;
    return mapping == 0;
}
