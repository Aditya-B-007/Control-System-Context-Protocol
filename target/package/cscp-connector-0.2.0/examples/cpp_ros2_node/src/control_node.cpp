#include <iostream>
#include "cscp_connector.h"

int main() {
    std::cout << "[C++ Node] Initializing Control-System-Context-Protocol Shared Memory..." << std::endl;
    CscpSharedMemory* shm = cscp_shm_create();

    float obs[16] = {0.1f};
    float rewards[2] = {1.0f, 0.0f};
    uint8_t mask[8] = {1, 1, 1, 1, 1, 1, 1, 1};

    CscpStatusCode status = cscp_env_step(shm, obs, rewards, mask);
    if (status == CSCP_SUCCESS) {
        std::cout << "[C++ Node] Step executed successfully!" << std::endl;
    }

    float action[4];
    float confidence;
    uint32_t latency;
    cscp_env_read_actuation(shm, action, &confidence, &latency);

    std::cout << "[C++ Node] Actuation read successfully." << std::endl;
    cscp_shm_destroy(shm);
    return 0;
}
