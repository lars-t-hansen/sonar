/* API to the sonar NVML library */

/* These return 0 on success, -1 on any kind of error.  Results are always passed via 'out' parameters,
   either in structure form or as broken-out fields.

   The intent is that the AMD API should look pretty much the same, maybe even be identical.
*/

#ifndef sonar_nvml_h_included
#define sonar_nvml_h_included

#include <inttypes.h>

int nvml_open();
int nvml_close();

int nvml_device_get_count(uint32_t* count);

/* The buffer sizes are mostly mandated by the underlying NVML API */
/* This structure must be reflected exactly on the Rust side */
struct nvml_card_info {
    char bus_addr[80];
    char model[96];
    char architecture[32];
    char driver[80];            /* Same for all cards on a node */
    char firmware[80];
    char uuid[96];
    uint64_t totalmem;          /* Bytes */
    unsigned power_limit;       /* Milliwatts */
    unsigned min_power_limit;   /* Milliwatts */
    unsigned max_power_limit;   /* Milliwatts */
    unsigned max_ce_clock;      /* MHz */
    unsigned max_mem_clock;     /* MHz */
};

/* Clear the infobuf and fill it with available information */
int nvml_device_get_card_info(uint32_t device, struct nvml_card_info* infobuf);

struct nvml_card_state {
    unsigned fan_speed;
    char compute_mode[32];
    char perf_state[8];
    uint64_t mem_reserved;
    uint64_t mem_used;
    float gpu_util;
    float mem_util;
    unsigned temp;
    unsigned power;
    unsigned power_limit;
    unsigned ce_clock;
    unsigned mem_clock;
};

/* Clear the infobuf and fill it with available information */
int nvml_device_get_card_state(uint32_t device, struct nvml_card_state* infobuf);




/* This is older stuff, it will go away */

/* The buffer should be at least 80 bytes, or the output may be chopped. */
int nvml_system_get_driver_version(char* buf, size_t bufsiz);

/* The architecture is a well-defined number, see nvml.h */
int nvml_device_get_architecture(uint32_t device, uint32_t* arch);

/* The unit is 'bytes'. */
int nvml_device_get_memory_info(
    uint32_t device, uint64_t* total, uint64_t* used, uint64_t* free);

/* The buffer should be at least 96 bytes, or the output may be chopped. */
int nvml_device_get_name(uint32_t device, char* buf, size_t bufsiz);

/* The buffer should be at least 96 bytes, or the output may be chopped. */
int nvml_device_get_uuid(uint32_t device, char* buf, size_t bufsiz);

/* The unit is 'milliwatts'. */
int nvml_device_get_power_management_limit_constraints(
    uint32_t device, uint32_t* min_limit, uint32_t* max_limit);

#endif /* sonar_nvml_h_included */
