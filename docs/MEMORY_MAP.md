# CSCP v0.3 Shared Memory Layout

## Region Structure

The entire POSIX shared memory region is a single `ShmLayout<N>` struct:

```
┌─────────────────────────────────────────────────────┐
│ CscpShmHeader        (64 bytes, aligned to 64)      │
├─────────────────────────────────────────────────────┤
│ SpscQueue<N>: q_env_to_ctrl (OmniSim → Controller)  │
│   ├── head: AtomicUsize    (64 bytes with padding)  │
│   ├── tail: AtomicUsize    (64 bytes with padding)  │
│   └── slots: [CscpFrame; N]                         │
├─────────────────────────────────────────────────────┤
│ SpscQueue<N>: q_ctrl_to_env (Controller → OmniSim)  │
│   ├── head: AtomicUsize    (64 bytes with padding)  │
│   ├── tail: AtomicUsize    (64 bytes with padding)  │
│   └── slots: [CscpFrame; N]                         │
└─────────────────────────────────────────────────────┘
```

## Header (64 Bytes)

| Field | Offset | Data Type | Description |
| --- | --- | --- | --- |
| **`magic`** | `0x00` | `AtomicU32` | ASCII signature `0x43534350` ("CSCP") |
| **`abi_version`** | `0x04` | `AtomicU32` | ABI version `0x00030000` (v0.3.0) |
| **`shutdown`** | `0x08` | `AtomicU8` | 0 = running, 1 = shutting down |
| **`_pad`** | `0x09` | `[u8; 55]` | Padding to 64-byte cache line boundary |

## CscpFrame (Per-Slot, Aligned to 64)

| Field | Offset (within slot) | Data Type | Size | Description |
| --- | --- | --- | --- | --- |
| **`sequence`** | `0x00` | `u64` | 8 B | Monotonic frame counter |
| **`timestamp_us`** | `0x08` | `u64` | 8 B | Microsecond timestamp at capture |
| **`terminated`** | `0x10` | `u8` | 1 B | True terminal flag |
| **`truncated`** | `0x11` | `u8` | 1 B | Timeout / truncation flag |
| **`_pad`** | `0x12` | `[u8; 6]` | 6 B | Alignment padding |
| **`state`** | `0x18` | `[f32; 16]` | 64 B | Observation vector $\mathbf{S}_t$ |
| **`action`** | `0x58` | `[f32; 4]` | 16 B | Action vector $\mathbf{a}_t$ |
| **`reward`** | `0x68` | `[f32; 2]` | 8 B | Multi-objective reward $\mathbf{r}_t$ |
| **`rule_mask`** | `0x70` | `[u8; 8]` | 8 B | Safety mask $\mathbf{m}_t$ |

Total per frame: ~120 bytes (padded to 128 or 192 depending on alignment).

## Default Configuration (N=16)

With the default `ShmLayout<16>`:
- 16 slots per queue × 2 queues = 32 frame slots
- Total region size: `size_of::<ShmLayout<16>>()` bytes

## Legacy v0.2 Layout

The v0.2 `CscpSharedMemory` struct remains available in the `shm` module for single-process use. It uses `ABI_VERSION = 0x00020000` and includes history stacks (`HISTORY_LEN = 4`).
