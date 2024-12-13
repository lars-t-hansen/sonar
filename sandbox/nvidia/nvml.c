/* Remember to `module load CUDA/11.1.1-GCC-10.2.0` or similar for nvml.h */
/* It would be possible to link with nvml but generally it's better to dlopen? */

#include <stddef.h>
#include <stdlib.h>
#include <stdio.h>
#include <dlfcn.h>

#include <nvml.h>

void loadNVML();

nvmlReturn_t (*xnvmlInit)();
nvmlReturn_t (*xnvmlDeviceGetCount_v2)(unsigned*);
nvmlReturn_t (*xnvmlDeviceGetHandleByIndex_v2)(int index, nvmlDevice_t* dev);
nvmlReturn_t (*xnvmlDeviceGetArchitecture)(nvmlDevice_t, nvmlDeviceArchitecture_t*);
nvmlReturn_t (*xnvmlDeviceGetMemoryInfo)(nvmlDevice_t, nvmlMemory_t*);

int main(int argv, char** argc) {
    loadNVML();
    int r = xnvmlInit();
    printf("Init: %d\n", r);
    unsigned ndev;
    r = xnvmlDeviceGetCount_v2(&ndev);
    printf("DeviceGetCount: %d %u\n", r, ndev);
    for (int i=0 ; i < ndev; i++) {
        nvmlDevice_t dev;
        r = xnvmlDeviceGetHandleByIndex_v2(i, &dev);

        nvmlDeviceArchitecture_t arch;
        r = xnvmlDeviceGetArchitecture(dev, &arch);
        printf("Arch %d %u\n", i, arch);

        nvmlMemory_t mem;
        r = xnvmlDeviceGetMemoryInfo(dev, &mem);
        printf("  Mem %llu %llu %llu\n", mem.free, mem.total, mem.used);
    }
}

/* abstraction around nvml */

void* lookup(const char* sym);

void loadNVML() {
    xnvmlInit = lookup("nvmlInit");
    xnvmlDeviceGetCount_v2 = lookup("nvmlDeviceGetCount_v2");
    xnvmlDeviceGetHandleByIndex_v2 = lookup("nvmlDeviceGetHandleByIndex_v2");
    xnvmlDeviceGetArchitecture = lookup("nvmlDeviceGetArchitecture");
    xnvmlDeviceGetMemoryInfo = lookup("nvmlDeviceGetMemoryInfo");
}

void* lib;

void endlib() {
    dlclose(lib);
}

void ensurelib() {
    if (lib == NULL) {
        lib = dlopen("/usr/lib64/libnvidia-ml.so", RTLD_NOW);
        if (lib == NULL) {
            perror("dlopen");
            exit(1);
        }
        atexit(endlib);
    }
}

void* lookup(const char* sym) {
    ensurelib();
    void *p = dlsym(lib, sym);
    if (p == NULL) {
        fprintf(stderr, "dlsym: %s\n", dlerror());
        exit(1);
    }
    return p;
}
