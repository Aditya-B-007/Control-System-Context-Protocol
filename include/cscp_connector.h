/* Automatically generated C-ABI header for Control-System-Context-Protocol */

#ifndef CSCP_CONNECTOR_H
#define CSCP_CONNECTOR_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define CSCP_MAGIC_SIGNATURE 0x43534350
#define CSCP_ABI_VERSION     0x00020000

#define CSCP_STATE_DIM       16
#define CSCP_ACTION_DIM      4
#define CSCP_REWARD_DIM      2
#define CSCP_RULE_MASK_DIM   8
#define CSCP_HISTORY_LEN     4

typedef enum {
    CSCP_SUCCESS = 0,
    CSCP_INVALID_MAGIC = -1,
    CSCP_ABI_VERSION_MISMATCH = -2,
    CSCP_NULL_POINTER = -3,
    CSCP_BUFFER_TOO_SMALL = -4,
    CSCP_SHM_ACCESS_ERROR = -5,
    CSCP_POISONED_LOCK = -6,
    CSCP_UNKNOWN_ERROR = -99
} CscpStatusCode;

typedef struct CscpSharedMemory CscpSharedMemory;

/* Function Declarations */
CscpSharedMemory* cscp_shm_create(void);
CscpStatusCode cscp_shm_destroy(CscpSharedMemory* shm);

CscpStatusCode cscp_env_step(
    CscpSharedMemory* shm,
    const float* obs,
    const float* rewards,
    const uint8_t* mask
);

CscpStatusCode cscp_env_read_actuation(
    const CscpSharedMemory* shm,
    float* out_action,
    float* out_confidence,
    uint32_t* out_latency_us
);

#ifdef __cplusplus
}
#endif

#endif /* CSCP_CONNECTOR_H */
