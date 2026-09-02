use crate::error::CscpError;
use crate::ipc::layout::ShmLayout;
use crate::ipc::leaky_bucket::LeakyBucket;
use crate::ipc::shm_region::ShmRegion;
use crate::ipc::spsc_queue::CscpFrame;

/// OmniSim-side handle to the CSCP shared memory transport.
///
/// `EnvSide` is the **owner** of the shared memory region.
/// It creates the region, initializes the layout, and calls `shm_unlink` on drop.
///
/// - Publishes state frames into `q_env_to_ctrl`.
/// - Receives command frames from `q_ctrl_to_env`.
pub struct EnvSide<const N: usize = 16> {
    region: ShmRegion,
    state_bucket: LeakyBucket,
}

impl<const N: usize> EnvSide<N> {
    /// Create and own a new shared memory region.
    ///
    /// The region is zero-initialized and the SHM layout (header + queues)
    /// is set up before this function returns.
    ///
    /// # Arguments
    /// * `name` — POSIX SHM name (e.g. `"cscp_default"`). A leading `/` is added if absent.
    /// * `rate_limit` — Maximum state frames per second (leaky bucket refill rate).
    ///                   Pass `f64::INFINITY` to disable rate limiting.
    pub fn create(name: &str, rate_limit: f64) -> Result<Self, CscpError> {
        let size = ShmLayout::<N>::region_size();
        let region = ShmRegion::create(name, size)?;

        // SAFETY: region.as_ptr() points to a freshly mmap'd, zero-initialized region
        // of exactly `size` bytes. We have exclusive access at this point (no other
        // process has attached yet).
        unsafe {
            let layout = &*(region.as_ptr() as *const ShmLayout<N>);
            layout.init();
        }

        let burst = if rate_limit.is_infinite() { f64::MAX } else { (rate_limit * 0.016).max(1.0) };
        let state_bucket = LeakyBucket::new(burst, rate_limit);

        Ok(Self { region, state_bucket })
    }

    /// Publish a state frame into the OmniSim → Controller queue.
    ///
    /// Returns `Err(QueueFull)` if the queue is full or rate-limited.
    /// State frames are droppable — the latest state is what matters.
    pub fn publish_state(&mut self, frame: &CscpFrame) -> Result<(), CscpError> {
        if !self.state_bucket.try_consume() {
            return Err(CscpError::RateLimited);
        }

        // SAFETY: region pointer is valid for the lifetime of self, and
        // only EnvSide calls try_push on q_env_to_ctrl (SPSC producer).
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        if layout.q_env_to_ctrl.try_push(frame) {
            Ok(())
        } else {
            Err(CscpError::QueueFull)
        }
    }

    /// Receive a command frame from the Controller → OmniSim queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn recv_command(&self) -> Option<CscpFrame> {
        // SAFETY: region pointer is valid, and only EnvSide calls try_pop
        // on q_ctrl_to_env (SPSC consumer).
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        layout.q_ctrl_to_env.try_pop()
    }

    /// Signal shutdown to the Controller process.
    pub fn signal_shutdown(&self) {
        // SAFETY: region pointer is valid for the lifetime of self.
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        layout.header.signal_shutdown();
    }

    /// Check if shutdown has been signaled (e.g. by the Controller).
    pub fn is_shutdown(&self) -> bool {
        let layout = unsafe { &*(self.region.as_ptr() as *const ShmLayout<N>) };
        layout.header.is_shutdown()
    }

    /// Returns the SHM region name.
    pub fn name(&self) -> &str {
        self.region.name()
    }
}
