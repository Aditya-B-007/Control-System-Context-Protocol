use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::shm::{ACTION_DIM, REWARD_DIM, RULE_MASK_DIM, STATE_DIM};

/// A single timestep frame exchanged between OmniSim and Controller.
/// 
/// Unlike v0.2's history stacks, each frame carries only the current
/// timestep. The consumer maintains its own sliding window if history is needed.
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct CscpFrame {
    pub sequence:     u64,
    pub timestamp_us: u64,
    pub terminated:   u8,
    pub truncated:    u8,
    pub _pad:         [u8; 6],
    pub state:        [f32; STATE_DIM],      // 16 floats = 64 bytes
    pub action:       [f32; ACTION_DIM],     // 4 floats  = 16 bytes
    pub reward:       [f32; REWARD_DIM],     // 2 floats  = 8 bytes
    pub rule_mask:    [u8; RULE_MASK_DIM],   // 8 bytes
}

impl Default for CscpFrame {
    fn default() -> Self {
        Self {
            sequence: 0,
            timestamp_us: 0,
            terminated: 0,
            truncated: 0,
            _pad: [0; 6],
            state: [0.0; STATE_DIM],
            action: [0.0; ACTION_DIM],
            reward: [0.0; REWARD_DIM],
            rule_mask: [0; RULE_MASK_DIM],
        }
    }
}

impl fmt::Debug for CscpFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CscpFrame")
            .field("sequence", &self.sequence)
            .field("terminated", &self.terminated)
            .field("truncated", &self.truncated)
            .finish()
    }
}

impl CscpFrame {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Lock-free bounded SPSC ring buffer for shared memory.
/// 
/// `N` must be a power of two. The queue uses separate cache lines
/// for `head` and `tail` to prevent false sharing.
/// 
/// Synchronization: producer writes frame then publishes via `tail.store(Release)`.
/// Consumer reads `tail.load(Acquire)` to see published frames.
#[repr(C, align(64))]
pub struct SpscQueue<const N: usize> {
    /// Consumer's read index (only consumer writes this)
    head: AtomicUsize,
    _pad_head: [u8; 56],  // pad to separate cache line
    
    /// Producer's write index (only producer writes this)
    tail: AtomicUsize,
    _pad_tail: [u8; 56],  // pad to separate cache line
    
    /// Ring buffer slots
    slots: [CscpFrame; N],
}

impl<const N: usize> SpscQueue<N> {
    pub fn init(&self) {
        self.head.store(0, Ordering::Relaxed);
        self.tail.store(0, Ordering::Relaxed);
    }

    pub fn try_push(&self, frame: &CscpFrame) -> bool {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail.wrapping_sub(head) >= N {
            return false;
        }

        // SAFETY: 
        // - Only one producer exists (SPSC invariant).
        // - The slot at `tail % N` is not being read by the consumer (because head <= tail,
        //   and the slot was previously consumed or never used).
        unsafe {
            let slot_ptr = self.slots.as_ptr().add(tail % N) as *mut CscpFrame;
            std::ptr::copy_nonoverlapping(frame, slot_ptr, 1);
        }

        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn try_pop(&self) -> Option<CscpFrame> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head == tail {
            return None;
        }

        // SAFETY:
        // - Only one consumer exists (SPSC invariant).
        // - The slot at `head % N` has been published by the producer (tail > head).
        let frame = unsafe {
            let slot_ptr = self.slots.as_ptr().add(head % N);
            std::ptr::read(slot_ptr)
        };

        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(frame)
    }

    pub fn len(&self) -> usize {
        self.tail.load(Ordering::Relaxed).wrapping_sub(self.head.load(Ordering::Relaxed))
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.len() >= N
    }

    pub fn capacity(&self) -> usize {
        N
    }
}
