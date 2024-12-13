#include <stddef.h>
#include <inttypes.h>
#include <dlfcn.h>

/* Remember to `module load CUDA/11.1.1-GCC-10.2.0` or similar for nvml.h.

   On the UiO ML nodes, after that it will be in a location like this:
     /storage/software/CUDA/11.3.1/targets/x86_64-linux/include/nvml.h
     /storage/software/CUDAcore/11.1.1/targets/x86_64-linux/include/nvml.h
*/

#include <nvml.h>

#include "sonar-nvml.h"

static int load_nvml();
static void unload_nvml();

static nvmlReturn_t (*xnvmlInit)();
static nvmlReturn_t (*xnvmlDeviceGetCount_v2)(unsigned*);
static nvmlReturn_t (*xnvmlDeviceGetHandleByIndex_v2)(int index, nvmlDevice_t* dev);
static nvmlReturn_t (*xnvmlDeviceGetArchitecture)(nvmlDevice_t, nvmlDeviceArchitecture_t*);
static nvmlReturn_t (*xnvmlDeviceGetMemoryInfo)(nvmlDevice_t, nvmlMemory_t*);
static nvmlReturn_t (*xnvmlDeviceGetName)(nvmlDevice_t,char*,unsigned);

static int is_open;

int nvml_open() {
    if (load_nvml() != 0) {
        return -1;
    }
    if (xnvmlInit() != 0) {
        return -1;
    }
    is_open = 1;
    return 0;
}

int nvml_close() {
    if (!is_open) {
        return -1;
    }
    unload_nvml();
    is_open = 0;
    return 0;
}

int nvml_device_get_count(uint32_t* count) {
    if (!is_open) {
        return -1;
    }
    unsigned ndev;
    if (xnvmlDeviceGetCount_v2(&ndev) != 0) {
        return -1;
    }
    *count = ndev;
    return 0;
}

int nvml_device_get_architecture(uint32_t device, uint32_t* arch) {
    if (!is_open) {
        return -1;
    }
    nvmlDevice_t dev;
    if (xnvmlDeviceGetHandleByIndex_v2(device, &dev) != 0) {
        return -1;
    }
    nvmlDeviceArchitecture_t n_arch;
    if (xnvmlDeviceGetArchitecture(dev, &n_arch) != 0) {
        return -1;
    }
    *arch = n_arch;
    return 0;
}

int nvml_device_get_memory_info(uint32_t device, uint64_t* total, uint64_t* used, uint64_t* free) {
    if (!is_open) {
        return -1;
    }
    nvmlDevice_t dev;
    if (xnvmlDeviceGetHandleByIndex_v2(device, &dev) != 0) {
        return -1;
    }
    nvmlMemory_t mem;
    if (xnvmlDeviceGetMemoryInfo(dev, &mem) != 0) {
        return -1;
    }
    *total = mem.total;
    *used = mem.used;
    *free = mem.free;
    return 0;
}

int nvml_device_get_name(uint32_t device, char* buf, size_t bufsiz) {
    if (!is_open) {
        return -1;
    }
    nvmlDevice_t dev;
    if (xnvmlDeviceGetHandleByIndex_v2(device, &dev) != 0) {
        return -1;
    }
    if (xnvmlDeviceGetName(dev, buf, (unsigned)bufsiz) != 0) {
        return -1;
    }
    return 0;
}

// dynamic library management

static void* lib;

static void* lookup(const char* sym) {
    if (lib == NULL) {
        lib = dlopen("/usr/lib64/libnvidia-ml.so", RTLD_NOW);
        if (lib == NULL) {
            return NULL;
        }
    }
    return dlsym(lib, sym);
}

static int load_nvml() {
    if ((xnvmlInit = lookup("nvmlInit")) == NULL) {
        return -1;
    }
    if ((xnvmlDeviceGetCount_v2 = lookup("nvmlDeviceGetCount_v2")) == NULL) {
        return -1;
    }
    if ((xnvmlDeviceGetHandleByIndex_v2 = lookup("nvmlDeviceGetHandleByIndex_v2")) == NULL) {
        return -1;
    }
    if ((xnvmlDeviceGetArchitecture = lookup("nvmlDeviceGetArchitecture")) == NULL) {
        return -1;
    }
    if ((xnvmlDeviceGetMemoryInfo = lookup("nvmlDeviceGetMemoryInfo")) == NULL) {
        return -1;
    }
    if ((xnvmlDeviceGetName = lookup("nvmlDeviceGetName")) == NULL) {
        return -1;
    }
    return 0;
}

static void unload_nvml() {
    dlclose(lib);
}

