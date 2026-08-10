# `Control-System-Context-Protocol` (`cscp-connector`) Specification & Documentation Manual

**Version**: 0.2.0

**License**: Apache-2.0 License

**Architecture**: Zero-Copy C-ABI Shared Memory Data & Control Plane

---

## 1. Executive Summary & Core Philosophy

The **`Control-System-Context-Protocol`** (`cscp-connector`) ecosystem is an open-source, language-agnostic, zero-copy C-ABI interface designed to connect physical or simulated robotics environments (**Side A**) with real-time reinforcement learning and control engines (**Side B**).

### The Problem

Industrial automation, robotics, and machine learning operate in fragmented silos:

* **ML Researchers** build Python-first Gymnasium/PyTorch pipelines.
* **Roboticists** run ROS 2, Zenoh, or vendor SDKs in C++/Rust.
* **Industrial Engineers** rely on hardware fieldbuses like OPC UA, Modbus, or SocketCAN.

### The Solution

`Control-System-Context-Protocol` bridges these worlds by providing a **Universal C-ABI Data Plane** over cache-aligned shared memory. To Side A, Side B is an abstract black-box controller yielding action vectors. To Side B, Side A is a standardized stream of states, multi-objective rewards, and safety masks.

---

## 2. System Architecture & Dual-Sided Boundary

```
                           CONTROL-SYSTEM-CONTEXT-PROTOCOL BOUNDARY
                               (Zero-Copy C-ABI Shared Memory)
                                               │
        ┌──────────────────────────────────────┴──────────────────────────────────────┐
        │                                                                             │
        ▼                                                                             ▼
┌─────────────────────────────────────────────┐               ┌─────────────────────────────────────────────┐
│          SIDE A: ENVIRONMENT SIDE           │               │          SIDE B: ALGORITHM SIDE            │
│       (Hardware / Simulator Drivers)        │               │        (Universal Control Engines)          │
├─────────────────────────────────────────────┤               ├─────────────────────────────────────────────┤
│ • Ingress Plugins (Sensors / Telemetry)     │               │ • Control Engine Plugins                    │
│ • Reward Ingress (Dense / Multi-Objective)  │   Data Plane  │   (PID, MPC, Single ONNX, CSCP Ensemble)    │
│ • Rule-Mask Plugins (Safety Interlocks)     │  (SHM <10µs)  │ • Compute Backend Plugins                   │
│ • Egress Plugins (Actuator Registers)       │◄─────────────►│   (CPU SIMD, TensorRT, Bare-Metal)          │
│ • Connector Client SDK                      │               │ • Diagnostic Plugins                        │
│   (ROS 2, Gym, Isaac, PLC, Custom C/Rust)   │               │   (ROS 2 Diagnostics, WandB, Terminal)      │
└─────────────────────────────────────────────┘               └─────────────────────────────────────────────┘

```

---

## 3. The Universal MDP & Control Payload Contract ($\mathcal{U}_t$)

To ensure compatibility across all RL frameworks, classical controllers, and sequence models, data passing into Side B follows the **Universal MDP & State-Space Payload Contract**:

$$\mathcal{U}_t = \left\{ \mathbf{S}_t, \mathbf{A}_{\text{hist}}, \mathbf{R}_{\text{hist}}, \mathbf{m}_t, d_t, \tau_t, \mathbf{I}_t \right\}$$

### Field Specifications

* **Observation State ($\mathbf{S}_t \in \mathbb{R}^{d_S}$)**: Dense array of normalized kinematic, spatial, or sensory telemetry.
* **Previous Action History ($\mathbf{A}_{\text{hist}} \in \mathbb{R}^{(k+1) \times d_a}$)**: Sliding temporal matrix of executed physical commands ($\mathbf{a}_{t-1}^*, \dots, \mathbf{a}_{t-k}^*$).
* **Reward History Matrix ($\mathbf{R}_{\text{hist}} \in \mathbb{R}^{(k+1) \times d_r}$)**: Multi-objective reward/cost array from step $t$ down to historical step $t-k$.
* **Rule Mask Vector ($\mathbf{m}_t \in \{0, 1\}^{d_m}$)**: Binary constraint flags representing physical interlocks (`1` = Safe/Legal, `0` = Blocked/Unsafe).
* **Episode Status Flags ($d_t, \tau_t \in \{0, 1\}$)**:
* $d_t$ (**Terminated**): Set to `1` on true task completion or physical crash.
* $\tau_t$ (**Truncated**): Set to `1` on artificial time limits or manual resets.


* **Auxiliary Info Payload ($\mathbf{I}_t$)**: Fixed-size diagnostic and environment metadata.

---

## 4. Memory Layout & C-ABI Specification (`cscp_shm.h`)

To achieve sub-10 microsecond IPC latencies, data is exchanged over a **64-byte cache-aligned C-contiguous shared memory map**:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                               SHARED MEMORY LAYOUT (cscp_shm.h)                       │
├─────────────────┬──────────────────┬─────────────────┬─────────────────┬───────────────┤
│ Header & Flags  │ Observation Stack│ Action Stack    │ Reward Stack    │ Rule Mask     │
│ (0x00 - 0x3F)   │ (S_stack)        │ (A_stack)       │ (R_stack)       │ (m_stack)     │
└─────────────────┴──────────────────┴─────────────────┴─────────────────┴───────────────┘
  Offset 0x00       Offset 0x40        Offset = 0x40 +   Offset = Prev +   Offset = Prev +
                                       size(S_stack)     size(A_stack)     size(R_stack)

```

### Memory Map Table

| Field Name | Offset | Data Type | Dimensions | Description |
| --- | --- | --- | --- | --- |
| **`magic_signature`** | `0x00` | `uint32_t` | Scalar (`0x43534350`) | ASCII signature verification (`"CSCP"`). |
| **`abi_version`** | `0x04` | `uint32_t` | Scalar (`0x00020000`) | Rejects mismatched ABI versions. |
| **`sequence_counter`** | `0x08` | `uint64_t` | Scalar (Atomic) | Atomic lock-free sequence counter. |
| **`timestamp_us`** | `0x10` | `uint64_t` | Scalar | Microsecond clock at sensor capture. |
| **`terminated`** | `0x18` | `uint8_t` | Scalar (`0` or `1`) | True terminal flag $d_t$. |
| **`truncated`** | `0x19` | `uint8_t` | Scalar (`0` or `1`) | Timeout / boundary reset flag $\tau_t$. |
| **`reserved_pad`** | `0x1A` | `uint8_t` | 38 Bytes | Padding to align data matrix to `0x40`. |
| **`state_stack`** | `0x40` | `float32_t` | $(k+1) \times d_S$ | Temporal observation matrix $\mathbf{S}_{\text{stack}}$. |
| **`action_stack`** | Variable | `float32_t` | $(k+1) \times d_a$ | Previously executed actions $\mathbf{a}_{\text{stack}}$. |
| **`reward_stack`** | Variable | `float32_t` | $(k+1) \times d_r$ | Multi-objective reward history $\mathbf{r}_{\text{stack}}$. |
| **`rule_mask`** | Variable | `uint8_t` | $(k+1) \times d_m$ | Active safety mask matrix $\mathbf{m}_{\text{stack}}$. |
| **`actuation_out`** | Variable | `float32_t` | $1 \times d_a$ | Final control command $\mathbf{a}_t^*$ written by Side B. |

---

## 5. Dual-Sided Plugin Architecture

### Side A: Environment Side Plugins

1. **`IngressPlugin`**: Normalizes raw hardware or simulator sensor feeds into state array $S_t$.
2. **`RewardIngressPlugin`**: Computes or parses multi-objective rewards $\mathbf{r}_t \in \mathbb{R}^{d_r}$.
3. **`RuleMaskPlugin`**: Inspects thermal limits and boundary flags to populate rule mask $m_t$.
4. **`EgressPlugin`**: Reads final actuation vector $\mathbf{a}_t^*$ and writes physical voltages, torques, or CAN bus frames.

### Side B: Algorithm Side Plugins

1. **`ControlEnginePlugin` (`IControlEngine`)**: Implements the decision strategy (`PIDControllerPlugin`, `MPCSolverPlugin`, `ONNXSinglePolicyPlugin`, `CscpMothershipPlugin`).
2. **`ComputeBackendPlugin`**: Defines mathematical execution providers (`CPU_SIMD_Backend`, `TensorRT_Backend`, `TFLite_Micro_Backend`).
3. **`DiagnosticPlugin`**: Non-blocking telemetry output (`ROS2DiagnosticPublisher`, `TensorBoardLogger`).

---

## 6. Multi-Language SDK & Usage Examples

### A. Importing in Rust (`Cargo.toml`)

```toml
[dependencies]
cscp-connector = "0.2"

```

```rust
use cscp_connector::{EnvironmentManager, CscpSharedMemory};
use std::sync::Arc;

fn main() {
    let shm = Arc::new(CscpSharedMemory::new());
    let mut env_driver = EnvironmentManager::new(shm);

    let obs = [0.1f32; 16];
    let rewards = [1.0f32, 0.0f32];
    let mask = [1u8; 8];

    // Zero-copy step execution
    env_driver.step(&obs, &rewards, &mask);
    let (action, confidence, latency) = env_driver.read_actuation();
}

```

---

### B. Importing in C++ / ROS 2 (`CMakeLists.txt`)

```cmake
include(FetchContent)
FetchContent_Declare(
    cscp_connector
    GIT_REPOSITORY https://github.com/aditya/Control-System-Context-Protocol.git
    GIT_TAG        v0.2.0
)
FetchContent_MakeAvailable(cscp_connector)

target_link_libraries(my_robot_node PRIVATE cscp_connector::cscp_connector)

```

```cpp
#include <rclcpp/rclcpp.hpp>
#include "cscp_connector.h"

class RobotControlNode : public rclcpp::Node {
public:
    RobotControlNode() : Node("cscp_control_node") {
        shm_handle_ = cscp_shm_create();
    }

    ~RobotControlNode() {
        cscp_shm_destroy(shm_handle_);
    }

    void control_loop() {
        float obs[16] = { /* Read sensors */ };
        float rewards[2] = {1.0f, -0.01f};
        uint8_t mask[8] = {1, 1, 1, 1, 1, 1, 1, 1};

        cscp_env_step(shm_handle_, obs, rewards, mask);

        float action_cmd[4];
        float confidence;
        uint32_t latency;
        cscp_env_read_actuation(shm_handle_, action_cmd, &confidence, &latency);
    }

private:
    CscpSharedMemory* shm_handle_;
};

```

---

### C. Importing in Python (`pip install cscp-connector`)

```python
import cscp_connector
import numpy as np

shm = cscp_connector.CscpSharedMemory()

obs = np.random.randn(16).astype(np.float32)
rewards = np.array([1.0, 0.0], dtype=np.float32)
mask = np.ones(8, dtype=np.uint8)

shm.step(obs, rewards, mask)
action, confidence, latency = shm.read_actuation()

```

---

## 7. Technology Stack & Performance Profiles

| Layer | Platform / Technology | Performance Characteristic |
| --- | --- | --- |
| **Data Plane IPC** | POSIX Shared Memory / Win32 SHM | $< 10\text{ }\mu\text{s}$ memory transfer latency |
| **Synchronization** | Lock-free Atomic Sequence Counters (`AtomicU64`) | Zero OS thread context-switch overhead |
| **Control Plane** | gRPC over HTTP/2 / Unix Sockets | Initialization, discovery, and remote fallback |
| **Compiler Targets** | Rust 2021, C++17/20, C99, Python PyO3 | Native SIMD execution (AVX-512, ARM NEON, RISC-V) |

---

## 8. Distribution & Ecosystem Package Registry Indexing

To make `cscp-connector` discoverable and installable globally across ecosystems:

* **Rust Registry**: Published to **[crates.io/crates/cscp-connector](https://crates.io)**. Searchable via `cargo add cscp-connector`. Auto-generated API documentation hosted on **[docs.rs/cscp-connector](https://docs.rs)**.
* **Python Registry**: Built via `maturin` and published to **PyPI** (`pip install cscp-connector`).
* **ROS 2 Index**: Packaged as `ros-humble-cscp-connector` and registered in `ros/rosdistro`.
* **C++ Package Managers**: Direct support for CMake `FetchContent`, `vcpkg`, and `Conan`.