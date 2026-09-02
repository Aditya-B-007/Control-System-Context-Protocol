use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use crate::ipc::spsc_queue::SpscQueue;

/// CSCP v0.3 magic signature — ASCII "CSCP"
pub const SHM_MAGIC: u32 = 0x43534350;

/// CSCP v0.3 ABI version
pub const SHM_ABI_VERSION: u32 = 0x00030000;

/// Global header at offset 0 of the shared memory region.
///
/// Written once by the owner (OmniSim / EnvSide) during `create`.
/// Verified by the attacher (Controller / CtrlSide) during `attach`.
#[repr(C, align(64))]
pub struct CscpShmHeader {
    pub magic: AtomicU32,
    pub abi_version: AtomicU32,
    /// 0 = running, 1 = shutting down
    pub shutdown: AtomicU8,
    pub _pad: [u8; 55],
}

impl CscpShmHeader {
    /// Initialize the header fields. Called once by the SHM owner.
    pub fn init(&self) {
        self.magic.store(SHM_MAGIC, Ordering::Relaxed);
        self.abi_version.store(SHM_ABI_VERSION, Ordering::Relaxed);
        self.shutdown.store(0, Ordering::Relaxed);
        // Ensure all writes above are visible before any consumer reads
        std::sync::atomic::fence(Ordering::Release);
    }

    /// Validate magic and ABI version. Returns `true` if valid.
    pub fn validate(&self) -> bool {
        std::sync::atomic::fence(Ordering::Acquire);
        self.magic.load(Ordering::Relaxed) == SHM_MAGIC
            && self.abi_version.load(Ordering::Relaxed) == SHM_ABI_VERSION
    }

    /// Signal shutdown to the other process.
    pub fn signal_shutdown(&self) {
        self.shutdown.store(1, Ordering::Release);
    }

    /// Check if shutdown has been signaled.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire) != 0
    }
}

/// Flat shared memory layout placed at the base of the mmap'd region.
///
/// Contains the global header and two SPSC queues:
/// - `q_env_to_ctrl`: OmniSim → Controller (state frames)
/// - `q_ctrl_to_env`: Controller → OmniSim (command frames)
///
/// `N` is the queue capacity (default 16, must be power of two).
#[repr(C)]
pub struct ShmLayout<const N: usize> {
    pub header: CscpShmHeader,
    pub q_env_to_ctrl: SpscQueue<N>,
    pub q_ctrl_to_env: SpscQueue<N>,
}

/// Default layout with 16-slot queues.
pub type DefaultShmLayout = ShmLayout<16>;

impl<const N: usize> ShmLayout<N> {
    /// Initialize all fields. Called once by the SHM owner after mmap.
    ///
    /// # Safety
    ///
    /// The caller must ensure `self` points to a valid, exclusively-owned
    /// region of at least `size_of::<ShmLayout<N>>()` bytes.
    pub unsafe fn init(&self) {
        self.header.init();
        self.q_env_to_ctrl.init();
        self.q_ctrl_to_env.init();
    }

    /// Returns the total byte size of this layout (for `ftruncate`).
    pub const fn region_size() -> usize {
        std::mem::size_of::<ShmLayout<N>>()
    }
}
