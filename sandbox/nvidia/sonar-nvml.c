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
static nvmlReturn_t (*xnvmlDeviceGetUUID)(nvmlDevice_t,char*,unsigned);
static nvmlReturn_t (*xnvmlDeviceGetPowerManagementLimitConstraints)(nvmlDevice_t,unsigned*,unsigned*);
static nvmlReturn_t (*xnvmlSystemGetDriverVersion)(char*,unsigned);

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

int nvml_device_get_uuid(uint32_t device, char* buf, size_t bufsiz) {
    if (!is_open) {
        return -1;
    }
    nvmlDevice_t dev;
    if (xnvmlDeviceGetHandleByIndex_v2(device, &dev) != 0) {
        return -1;
    }
    if (xnvmlDeviceGetUUID(dev, buf, (unsigned)bufsiz) != 0) {
        return -1;
    }
    return 0;
}

int nvml_system_get_driver_version(char* buf, size_t bufsiz) {
    if (!is_open) {
        return -1;
    }
    if (xnvmlSystemGetDriverVersion(buf, (unsigned)bufsiz) != 0) {
        return -1;
    }
    return 0;
}

int nvml_device_get_power_management_limit_constraints(
    uint32_t device, uint32_t* min_limit, uint32_t* max_limit) {
    if (!is_open) {
        return -1;
    }
    nvmlDevice_t dev;
    if (xnvmlDeviceGetHandleByIndex_v2(device, &dev) != 0) {
        return -1;
    }
    unsigned in, ax;
    if (xnvmlDeviceGetPowerManagementLimitConstraints(dev, &in, &ax) != 0) {
        return -1;
    }
    *min_limit = (uint32_t)in;
    *max_limit = (uint32_t)ax;
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

    /* You'll be tempted to try some magic here with # and ## but it won't work because sometimes
       nvml.h introduces #defines of some of the names we want to use. */

#define DLSYM(var, str) if ((var = lookup(str)) == NULL) { return -1; }

    DLSYM(xnvmlInit, "nvmlInit");
    DLSYM(xnvmlDeviceGetCount_v2, "nvmlDeviceGetCount_v2");
    DLSYM(xnvmlDeviceGetHandleByIndex_v2, "nvmlDeviceGetHandleByIndex_v2");
    DLSYM(xnvmlDeviceGetArchitecture, "nvmlDeviceGetArchitecture");
    DLSYM(xnvmlDeviceGetMemoryInfo, "nvmlDeviceGetMemoryInfo");
    DLSYM(xnvmlDeviceGetName, "nvmlDeviceGetName");
    DLSYM(xnvmlDeviceGetUUID, "nvmlDeviceGetUUID");
    DLSYM(xnvmlSystemGetDriverVersion, "nvmlSystemGetDriverVersion");
    DLSYM(xnvmlDeviceGetPowerManagementLimitConstraints, "nvmlDeviceGetPowerManagementLimitConstraints");

    return 0;
}

static void unload_nvml() {
    dlclose(lib);
}

