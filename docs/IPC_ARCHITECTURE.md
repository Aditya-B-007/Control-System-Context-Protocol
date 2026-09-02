# CSCP v0.3 IPC Architecture

## Overview

CSCP v0.3 introduces real OS-level inter-process communication (IPC) via POSIX shared memory. Two independent processes — **OmniSim** (the simulator) and the **Controller** — exchange data through a single named shared memory region containing two bounded SPSC (Single-Producer Single-Consumer) ring buffer queues.

```
OmniSim Process                     Controller Process
┌──────────────┐                    ┌──────────────┐
│  EnvSide<N>  │                    │  CtrlSide<N> │
│              │                    │              │
│ publish_state│──── q_env_to_ctrl ────▶│recv_state │
│              │                    │              │
│ recv_command │◀─── q_ctrl_to_env ────│publish_cmd │
└──────────────┘                    └──────────────┘
        │                                   │
        └──────── POSIX Shared Memory ──────┘
                  shm_open + mmap
```

## Shared Memory Region Layout

A single POSIX named shared memory region (`/cscp_<name>`) contains:

```
Offset 0x00: CscpShmHeader (64 bytes, cache-aligned)
├── magic:        AtomicU32 = 0x43534350 ("CSCP")
├── abi_version:  AtomicU32 = 0x00030000 (v0.3.0)
├── shutdown:     AtomicU8 (0 = running, 1 = shutting down)
└── _pad:         [u8; 55]

Offset 0x40: SpscQueue<N> — q_env_to_ctrl (OmniSim → Controller)
├── head: AtomicUsize + 56-byte pad (cache line 0)
├── tail: AtomicUsize + 56-byte pad (cache line 1)
└── slots: [CscpFrame; N]

Offset varies: SpscQueue<N> — q_ctrl_to_env (Controller → OmniSim)
├── head: AtomicUsize + 56-byte pad
├── tail: AtomicUsize + 56-byte pad
└── slots: [CscpFrame; N]
```

Total region size: `size_of::<ShmLayout<N>>()` bytes (compile-time constant).

## CscpFrame — Per-Timestep Data Unit

Each queue slot carries a single timestep:

| Field          | Type           | Size    | Description |
|----------------|----------------|---------|-------------|
| `sequence`     | `u64`          | 8 bytes | Monotonic frame counter |
| `timestamp_us` | `u64`          | 8 bytes | Microsecond clock at capture |
| `terminated`   | `u8`           | 1 byte  | True terminal flag |
| `truncated`    | `u8`           | 1 byte  | Timeout / truncation flag |
| `_pad`         | `[u8; 6]`      | 6 bytes | Alignment padding |
| `state`        | `[f32; 16]`    | 64 bytes| Observation vector |
| `action`       | `[f32; 4]`     | 16 bytes| Action vector |
| `reward`       | `[f32; 2]`     | 8 bytes | Multi-objective reward |
| `rule_mask`    | `[u8; 8]`      | 8 bytes | Safety constraint mask |

History stacks (v0.2) are removed from the frame. The consumer maintains its own sliding window internally.

## SPSC Queue Synchronization

The queue uses **Acquire/Release** atomic ordering:

**Producer (push):**
```
tail = tail.load(Relaxed)           // only producer writes tail
head = head.load(Acquire)           // see consumer's latest head
if tail - head >= N: return FULL
write frame into slots[tail % N]
tail.store(tail + 1, Release)       // publish — makes write visible
```

**Consumer (pop):**
```
head = head.load(Relaxed)           // only consumer writes head
tail = tail.load(Acquire)           // see producer's latest tail
if head == tail: return EMPTY
read frame from slots[head % N]
head.store(head + 1, Release)       // advance
```

This guarantees the consumer **never reads a partially-written frame**.

## Leaky Bucket Flow Control

Each producer has a process-local `LeakyBucket` token bucket:

- **State queue (OmniSim → Controller):** If rate-limited, frames are dropped silently (`Err(RateLimited)`). Stale state is acceptable.
- **Command queue (Controller → OmniSim):** If rate-limited, returns `Err(RateLimited)` to the caller. Commands are never silently dropped.

## SHM Ownership

- **OmniSim** (`EnvSide::create`) is always the owner. It calls `shm_unlink` on `Drop`.
- **Controller** (`CtrlSide::attach`) never calls `shm_unlink`.
- On OmniSim crash: the next `EnvSide::create` call cleans up the stale region.

## Performance

Measured on real cross-process IPC (100,000 round-trips):

| Metric | Value |
|--------|-------|
| p50    | < 1 µs |
| p95    | 1 µs  |
| p99    | 1 µs  |
