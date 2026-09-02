//! Simulates the OmniSim side of the CSCP transport.
//!
//! Creates a shared memory region, publishes state frames at a fixed rate,
//! and reads back command frames from the Controller.

use clap::Parser;
use cscp_connector::ipc::{CscpFrame, EnvSide};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(author, version, about = "CSCP OmniSim Simulator — publishes state, reads commands")]
struct Args {
    /// POSIX shared memory region name
    #[arg(short, long, default_value = "cscp_default")]
    name: String,

    /// Number of state frames to publish
    #[arg(short, long, default_value_t = 200)]
    frames: u64,

    /// Interval between frames in milliseconds
    #[arg(short, long, default_value_t = 10)]
    interval_ms: u64,

    /// Rate limit in frames per second (0 = unlimited)
    #[arg(short, long, default_value_t = 0.0)]
    rate_limit: f64,
}

fn main() {
    let args = Args::parse();
    let rate = if args.rate_limit <= 0.0 { f64::INFINITY } else { args.rate_limit };

    println!("[OmniSim] Creating SHM region '{}'", args.name);
    let mut env = EnvSide::<16>::create(&args.name, rate)
        .expect("Failed to create SHM region");

    println!("[OmniSim] Publishing {} frames at {}ms intervals", args.frames, args.interval_ms);
    let interval = Duration::from_millis(args.interval_ms);
    let mut commands_received: u64 = 0;

    for seq in 1..=args.frames {
        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let mut frame = CscpFrame::new();
        frame.sequence = seq;
        frame.timestamp_us = now_us;
        // Fill state with a recognizable pattern
        for (i, s) in frame.state.iter_mut().enumerate() {
            *s = seq as f32 + i as f32 * 0.01;
        }
        frame.terminated = if seq == args.frames { 1 } else { 0 };

        match env.publish_state(&frame) {
            Ok(()) => {},
            Err(e) => eprintln!("[OmniSim] Publish error at seq {}: {}", seq, e),
        }

        // Drain any incoming commands
        while let Some(cmd) = env.recv_command() {
            commands_received += 1;
            if commands_received % 50 == 0 || commands_received == 1 {
                println!(
                    "[OmniSim] Received command seq={}, action[0]={:.3}",
                    cmd.sequence, cmd.action[0]
                );
            }
        }

        if seq % 50 == 0 {
            println!("[OmniSim] Published {} frames, received {} commands", seq, commands_received);
        }

        std::thread::sleep(interval);
    }

    println!(
        "[OmniSim] Done. Published {} frames, received {} commands total.",
        args.frames, commands_received
    );
    env.signal_shutdown();
}
