# Integration Guide: ROS 2, Isaac Sim & Gymnasium

## 1. ROS 2 Node Integration (C++)

Add `cscp_connector` to `CMakeLists.txt`:
```cmake
find_package(cscp_connector REQUIRED)
target_link_libraries(my_node PRIVATE cscp_connector::cscp_connector)
```

Include `cscp_connector.h` in your control loop node and call `cscp_env_step()` inside the timer callback.

---

## 2. Python Gymnasium Wrapper

```python
import gym
import cscp_connector
import numpy as np

class CscpGymWrapper(gym.Wrapper):
    def __init__(self, env):
        super().__init__(env)
        self.shm = cscp_connector.PyCscpSharedMemory()

    def step(self, action):
        obs, reward, terminated, truncated, info = self.env.step(action)
        self.shm.step(obs, [reward, 0.0], [1]*8)
        actuation, confidence, latency = self.shm.read_actuation()
        return obs, reward, terminated, truncated, info
```
