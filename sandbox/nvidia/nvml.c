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
    printf("devices: %u\n", ndev);

    char buf[128];
    if (nvml_system_get_driver_version(buf, sizeof(buf)) != 0) {
        panic("system_get_driver_version");
    }
    printf("driver: %s\n", buf);

    for (uint32_t i=0 ; i < ndev; i++) {
        printf("device %u\n", i);

        char buf[128];
        if (nvml_device_get_name(i, buf, sizeof(buf)) != 0) {
            panic("device_get_name");
        }
        printf("  name %s\n", buf);

        if (nvml_device_get_uuid(i, buf, sizeof(buf)) != 0) {
            panic("device_get_uuid");
        }
        printf("  uuid %s\n", buf);

        uint32_t arch;
        if (nvml_device_get_architecture(i, &arch) != 0) {
            panic("device_get_architecture");
        }
        printf("  arch %u\n", arch);

        uint64_t total, used, free;
        if (nvml_device_get_memory_info(i, &total, &used, &free) != 0) {
            panic("device_get_memory_info");
        }
        printf("  mem  tot=%llu used=%llu free=%llu\n",
               (unsigned long long)total, (unsigned long long)used, (unsigned long long)free);

        uint32_t plmin, plmax;
        if (nvml_device_get_power_management_limit_constraints(i, &plmin, &plmax) != 0) {
            panic("device_get_power_management_limit_constraints");
        }
        printf("  powr min=%d max=%d\n", plmin, plmax);
    }

    nvml_close();
}
