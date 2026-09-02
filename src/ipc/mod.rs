//! CSCP v0.3 IPC Transport Layer
//!
//! Provides real OS shared memory (POSIX `shm_open` + `mmap`) with
//! two bounded SPSC ring buffer queues and leaky-bucket flow control.
//!
//! ## Architecture
//!
//! ```text
//! OmniSim Process              Controller Process
//!      │                              │
//!      │ EnvSide::create()            │ CtrlSide::attach()
//!      ▼                              ▼
//! ┌───────────────────────────────────────┐
//! │         POSIX Shared Memory           │
//! │                                       │
//! │  ┌─────────────────────────────────┐  │
//! │  │ CscpShmHeader (magic, version)  │  │
//! │  ├─────────────────────────────────┤  │
//! │  │ q_env_to_ctrl: SpscQueue<N>     │  │
//! │  │   OmniSim → Controller          │  │
//! │  ├─────────────────────────────────┤  │
//! │  │ q_ctrl_to_env: SpscQueue<N>     │  │
//! │  │   Controller → OmniSim          │  │
//! │  └─────────────────────────────────┘  │
//! └───────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use cscp_connector::ipc::{EnvSide, CtrlSide, CscpFrame};
//!
//! // OmniSim process:
//! let mut env = EnvSide::<16>::create("cscp_sim", 1000.0).unwrap();
//! let frame = CscpFrame::default();
//! env.publish_state(&frame).unwrap();
//!
//! // Controller process:
//! let mut ctrl = CtrlSide::<16>::attach("cscp_sim", 1000.0).unwrap();
//! if let Some(state) = ctrl.recv_state() {
//!     println!("Got state seq={}", state.sequence);
//! }
//! ```

pub mod ctrl_side;
pub mod env_side;
pub mod layout;
pub mod leaky_bucket;
pub mod shm_region;
pub mod spsc_queue;

pub use ctrl_side::CtrlSide;
pub use env_side::EnvSide;
pub use layout::{CscpShmHeader, DefaultShmLayout, ShmLayout, SHM_ABI_VERSION, SHM_MAGIC};
pub use leaky_bucket::LeakyBucket;
pub use shm_region::ShmRegion;
pub use spsc_queue::{CscpFrame, SpscQueue};
