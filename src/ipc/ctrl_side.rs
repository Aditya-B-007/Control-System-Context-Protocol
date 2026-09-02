use crate::error::CscpError;
use crate::ipc::layout::ShmLayout;
use crate::ipc::leaky_bucket::LeakyBucket;
use crate::ipc::shm_region::ShmRegion;
use crate::ipc::spsc_queue::CscpFrame;

/// Controller-side handle to the CSCP shared memory transport.
///
/// `CtrlSide` **attaches** to an existing shared memory region created by OmniSim.
/// It never calls `shm_unlink` — ownership belongs to OmniSim.
///
/// - Receives state frames from `q_env_to_ctrl`.
/// - Publishes command frames into `q_ctrl_to_env`.
pub struct CtrlSide<const N: usize = 16> {
    region: ShmRegion,
    cmd_bucket: LeakyBucket,
}

impl<const N: usize> CtrlSide<N> {
    /// Attach to an existing shared memory region created by OmniSim.
    ///
    /// Validates the SHM header (magic + ABI version) before returning.
    ///
    /// # Arguments
    /// * `name` — POSIX SHM name matching what OmniSim used.
    /// * `rate_limit` — Maximum command frames per second. Pass `f64::INFINITY` to disable.
    pub fn attach(name: &str, rate_limit: f64) -> Result<Self, CscpError> {
        let size = ShmLayout::<N>::region_size();
        let region = ShmRegion::attach(name, size)?;

        // SAFETY: region.as_ptr() points to a shared memory region that was
        // already initialized by EnvSide::create. We verify the header.
        let layout = unsafe { &*(region.as_ptr() as *const ShmLayout<N>) };
        if !layout.header.validate() {
            return Err(CscpError::AbiVersionMismatch {
                expected: crate::ipc::layout::SHM_ABI_VERSION,
                actual: layout.header.abi_version.load(std::sync::atomic::Ordering::Relaxed),
            });
        }

        let burst = if rate_limit.is_infinite() { f64::MAX } else { (rate_limit * 0.016).max(1.0) };
        let cmd_bucket = LeakyBucket::new(burst, rate_limit);

        Ok(Self { region, cmd_bucket })
    }

    /// Receive a state frame from the OmniSim → Controller queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn recv_state(&self) -> Option<CscpFrame> {
        // SAFETY: region pointer is valid, and only CtrlSide calls try_pop
        // on q_env_to_ctrl (SPSC consumer).
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        layout.q_env_to_ctrl.try_pop()
    }

    /// Publish a command frame into the Controller → OmniSim queue.
    ///
    /// Returns `Err(QueueFull)` if the queue is full.
    /// Commands are never silently dropped — the caller must handle the error.
    pub fn publish_command(&mut self, frame: &CscpFrame) -> Result<(), CscpError> {
        if !self.cmd_bucket.try_consume() {
            return Err(CscpError::RateLimited);
        }

        // SAFETY: region pointer is valid, and only CtrlSide calls try_push
        // on q_ctrl_to_env (SPSC producer).
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        if layout.q_ctrl_to_env.try_push(frame) {
            Ok(())
        } else {
            Err(CscpError::QueueFull)
        }
    }

    /// Signal shutdown to the OmniSim process.
    pub fn signal_shutdown(&self) {
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        layout.header.signal_shutdown();
    }

    /// Check if shutdown has been signaled (e.g. by OmniSim).
    pub fn is_shutdown(&self) -> bool {
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        layout.header.is_shutdown()
    }

    /// Returns the SHM region name.
    pub fn name(&self) -> &str {
        self.region.name()
    }
}
