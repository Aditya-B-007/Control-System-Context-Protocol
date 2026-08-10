# 64-Byte Cache Alignment & Shared Memory Layout

| Field Name | Offset | Data Type | Dimensions | Description |
| --- | --- | --- | --- | --- |
| **`magic_signature`** | `0x00` | `uint32_t` | `0x43534350` | ASCII signature verification (`"CSCP"`). |
| **`abi_version`** | `0x04` | `uint32_t` | `0x00020000` | Rejects mismatched ABI versions. |
| **`sequence_counter`** | `0x08` | `uint64_t` | Scalar (Atomic) | Atomic lock-free sequence counter. |
| **`timestamp_us`** | `0x10` | `uint64_t` | Scalar (Atomic) | Microsecond clock at sensor capture. |
| **`terminated`** | `0x18` | `uint8_t` | Scalar (`0` or `1`) | True terminal flag $d_t$. |
| **`truncated`** | `0x19` | `uint8_t` | Scalar (`0` or `1`) | Timeout flag $\tau_t$. |
| **`reserved_pad`** | `0x1A` | `uint8_t` | 38 Bytes | Padding to align state stack to `0x40`. |
| **`state_stack`** | `0x40` | `float32_t` | $4 \times 16$ | Observation matrix $\mathbf{S}_{\text{stack}}$. |
| **`action_stack`** | `0x140` | `float32_t` | $4 \times 4$ | Previously executed actions $\mathbf{a}_{\text{stack}}$. |
| **`reward_stack`** | `0x180` | `float32_t` | $4 \times 2$ | Multi-objective reward history $\mathbf{r}_{\text{stack}}$. |
| **`rule_mask`** | `0x1A0` | `uint8_t` | $4 \times 8$ | Active safety mask matrix $\mathbf{m}_{\text{stack}}$. |
| **`actuation_out`** | `0x1C0` | `float32_t` | $1 \times 4$ | Final control command written by Side B. |
