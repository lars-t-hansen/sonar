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

/* The architecture is a well-defined number, see nvml.h */
int nvml_device_get_architecture(uint32_t device, uint32_t* arch);

/* The unit is 'bytes' */
int nvml_device_get_memory_info(uint32_t device, uint64_t* total, uint64_t* used, uint64_t* free);

#endif /* sonar_nvml_h_included */
