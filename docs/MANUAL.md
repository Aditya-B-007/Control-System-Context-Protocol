# `Control-System-Context-Protocol` Specification Manual

## 1. Overview & Protocol Lifecycle

`cscp-connector` standardizes real-time sensor, reward, and action telemetry exchanging between physical environments (**Side A**) and control algorithms (**Side B**).

### Lifecycle Flow:
1. **Initialize**: Side A and Side B attach to POSIX Shared Memory segment.
2. **Side A Step**: `EnvironmentManager::step()` writes normalized sensory observation $\mathbf{S}_t$, multi-objective rewards $\mathbf{r}_t$, and active rule safety mask $\mathbf{m}_t$.
3. **Atomic Counter Trigger**: `sequence_counter` increments atomically (`SeqCst`), signaling Side B without OS context switches.
4. **Side B Evaluation**: `UniversalAlgorithmEngine` parses the payload contract $\mathcal{U}_t$ and writes back actuation vector $\mathbf{a}_t^*$, confidence score, and latency.
5. **Side A Egress**: `EnvironmentManager::read_actuation()` transfers action values to CAN bus or actuator controllers.

---

## 2. Dynamic Memory Slices & Offsets

- Header Offset: `0x00` - `0x3F` (64 bytes)
- State Stack: `0x40` ($16 \times 4 \times 4$ bytes = 256 bytes)
- Action Stack: `0x140` ($4 \times 4 \times 4$ bytes = 64 bytes)
- Reward Stack: `0x180` ($2 \times 4 \times 4$ bytes = 32 bytes)
- Rule Mask: `0x1A0` ($8 \times 4 \times 1$ bytes = 32 bytes)
- Actuation Command Out: `0x1C0` ($4 \times 4$ bytes = 16 bytes)
