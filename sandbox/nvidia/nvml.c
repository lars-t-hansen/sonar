#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <inttypes.h>

#include "sonar-nvml.h"

void panic(const char* s) {
    fprintf(stderr, "panic: %s\n", s);
    exit(1);
}

int main(int argv, char** argc) {
    if (nvml_open() != 0) {
        panic("Could not load");
    }

    uint32_t ndev;
    if (nvml_device_get_count(&ndev) != 0) {
        panic("device_get_count");
    }
    printf("device_get_count: %u\n", ndev);

    for (uint32_t i=0 ; i < ndev; i++) {
        uint32_t arch;
        if (nvml_device_get_architecture(i, &arch) != 0) {
            panic("device_get_architecture");
        }
        printf("device_get_architecture %u %u\n", i, arch);

        uint64_t total, used, free;
        if (nvml_device_get_memory_info(i, &total, &used, &free) != 0) {
            panic("device_get_memory_info");
        }
        printf("device_get_memory_info %llu %llu %llu\n", total, used, free);
    }

    nvml_close();
}