//! Simulates the Controller side of the CSCP transport.
//!
//! Attaches to an existing shared memory region created by OmniSim,
//! reads state frames, and publishes command frames back.

use clap::Parser;
use cscp_connector::ipc::{CscpFrame, CtrlSide};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about = "CSCP Controller Simulator — reads state, publishes commands")]
struct Args {
    /// POSIX shared memory region name (must match OmniSim)
    #[arg(short, long, default_value = "cscp_default")]
    name: String,

    /// Maximum number of state frames to process (0 = until shutdown)
    #[arg(short, long, default_value_t = 0)]
    max_frames: u64,

    /// Poll interval in milliseconds
    #[arg(short, long, default_value_t = 5)]
    poll_ms: u64,

    /// Rate limit for command frames per second (0 = unlimited)
    #[arg(short, long, default_value_t = 0.0)]
    rate_limit: f64,
}

fn main() {
    let args = Args::parse();
    let rate = if args.rate_limit <= 0.0 { f64::INFINITY } else { args.rate_limit };

    println!("[Controller] Attaching to SHM region '{}'", args.name);
    let mut ctrl = CtrlSide::<16>::attach(&args.name, rate)
        .expect("Failed to attach to SHM region");

    println!("[Controller] Connected. Polling for state frames...");
    let poll_interval = Duration::from_millis(args.poll_ms);
    let mut states_received: u64 = 0;
    let mut commands_sent: u64 = 0;

    loop {
        // Read all available state frames
        while let Some(state) = ctrl.recv_state() {
            states_received += 1;

            if states_received % 50 == 0 || states_received == 1 {
                println!(
                    "[Controller] State seq={}, ts={}µs, state[0]={:.3}{}",
                    state.sequence,
                    state.timestamp_us,
                    state.state[0],
                    if state.terminated != 0 { " [TERMINATED]" } else { "" }
                );
            }

            // Generate a command response
            let mut cmd = CscpFrame::new();
            cmd.sequence = states_received;
            // Simple proportional response: action = -0.1 * state
            for i in 0..cmd.action.len() {
                cmd.action[i] = -0.1 * state.state[i.min(state.state.len() - 1)];
            }

            match ctrl.publish_command(&cmd) {
                Ok(()) => commands_sent += 1,
                Err(e) => eprintln!("[Controller] Command publish error: {}", e),
            }

            // Check if OmniSim signaled termination
            if state.terminated != 0 {
                println!(
                    "[Controller] Received termination. States: {}, Commands: {}",
                    states_received, commands_sent
                );
                return;
            }

            if args.max_frames > 0 && states_received >= args.max_frames {
                println!(
                    "[Controller] Reached max frames. States: {}, Commands: {}",
                    states_received, commands_sent
                );
                ctrl.signal_shutdown();
                return;
            }
        }

        // Check for shutdown signal
        if ctrl.is_shutdown() {
            println!(
                "[Controller] Shutdown signaled. States: {}, Commands: {}",
                states_received, commands_sent
            );
            return;
        }

        std::thread::sleep(poll_interval);
    }
}
