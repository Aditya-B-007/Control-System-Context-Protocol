# `Control-System-Context-Protocol`

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![PyPI](https://img.shields.io/pypi/v/cscp-connector.svg)](https://pypi.org/project/cscp-connector/)
[![ROS 2 Humble](https://img.shields.io/badge/ROS_2-Humble-blueviolet.svg)](https://docs.ros.org/)
[![IPC Latency](https://img.shields.io/badge/IPC_Latency-%3C10%CE%BCs-success.svg)](#)

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

## 3. Folder Structure

```
Control-System-Context-Protocol/
├── Cargo.toml                  # Cargo manifest (configured for lib, cdylib, staticlib, & bin)
├── cbindgen.toml               # cbindgen config for auto C/C++ header generation
├── pyproject.toml              # Maturin configuration for Python PyPI wheel distribution
├── README.md                   # Public overview & quickstart documentation
├── LICENSE-APACHE              # Apache-2.0 License file
│
├── docs/                       # Complete specification manuals & integration guides
│   ├── MANUAL.md               # System specification manual
│   ├── MEMORY_MAP.md           # 64-Byte C-ABI offset table & alignment spec
│   └── INTEGRATION_GUIDE.md    # ROS 2, Isaac Sim, and Gymnasium setup guides
│
├── include/                    # Exported C/C++ Header Files
│   └── cscp_connector.h        # Auto-generated C-ABI header for C++, ROS 2, and C projects
│
├── src/                        # Core Rust Implementation
│   ├── lib.rs                  # Library entry point & public module exports
│   ├── shm.rs                  # 64-byte cache-aligned C-ABI shared memory struct layout
│   ├── env.rs                  # Side A: EnvironmentManager (Ingress / Egress / RuleMask)
│   ├── algo.rs                 # Side B: UniversalAlgorithmEngine & Payload Contract
│   ├── error.rs                # Error types & C-ABI status code definitions
│   ├── c_api.rs                # extern "C" FFI export functions for C++ / ROS 2
│   ├── python.rs               # PyO3 bindings for Python (pip install cscp-connector)
│   └── bin/
│       └── cscp_daemon.rs      # Standalone background process binary entry point
│
├── python/                     # Python Packaging Metadata
│   ├── cscp_connector/         # Python module wrapper package
│   │   ├── __init__.py         # Python entry point re-exports
│   │   └── py.typed            # PEP 561 type annotation flag
│   └── tests/                  # Pytest suite
│       └── test_connector.py
│
├── examples/                   # Multi-Language Reference Implementations
│   ├── rust_native/            # Pure Rust environment & control loop
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── cpp_ros2_node/          # C++ ROS 2 Node integrating cscp_connector.h
│   │   ├── CMakeLists.txt
│   │   ├── package.xml
│   │   └── src/control_node.cpp
│   └── python_gymnasium/       # Python Gymnasium environment wrapper script
│       └── gym_loop.py
│
└── tests/                      # Integration & Performance Benchmarks
    ├── shm_concurrency_test.rs # Lock-free atomic sequence & multi-threading tests
    └── latency_benchmark.rs    # Microsecond IPC loop speed test
```

---

## 4. Multi-Language Quickstart

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

## 5. License

Licensed under the Apache License, Version 2.0 ([LICENSE-APACHE](file:///Users/veenadhruva/Desktop/Aditya_projects/Working/Control-System-Context-Protocol/LICENSE-APACHE)).
