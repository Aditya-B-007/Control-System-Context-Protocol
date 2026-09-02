//! Real two-process IPC latency benchmark for CSCP shared memory transport.
//!
//! Spawns two child processes (OmniSim + Controller) that exchange frames
//! over OS shared memory and measures round-trip latency.
//!
//! Reports p50, p95, p99, and max latency.

use clap::Parser;
use cscp_connector::ipc::{CscpFrame, CtrlSide, EnvSide};
use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(author, version, about = "CSCP IPC Latency Benchmark")]
struct Args {
    /// Number of round-trip frames to measure
    #[arg(short, long, default_value_t = 100_000)]
    frames: u64,

    /// POSIX shared memory region name
    #[arg(short, long, default_value = "cscp_bench")]
    name: String,

    /// Run as the internal env (OmniSim) child process
    #[arg(long, hide = true)]
    role_env: bool,

    /// Run as the internal ctrl (Controller) child process
    #[arg(long, hide = true)]
    role_ctrl: bool,
}

fn main() {
    let args = Args::parse();

    if args.role_env {
        run_env_child(&args.name, args.frames);
    } else if args.role_ctrl {
        run_ctrl_child(&args.name, args.frames);
    } else {
        run_parent(args);
    }
}

/// Parent process: spawns two children, collects latency data, reports stats.
fn run_parent(args: Args) {
    let exe = std::env::current_exe().expect("Failed to get current executable path");

    println!("[CSCP IPC Bench] Spawning two child processes...");
    println!("[CSCP IPC Bench] SHM region: {}", args.name);
    println!("[CSCP IPC Bench] Frames: {}", args.frames);
    println!();

    // Spawn env (OmniSim) child first so it creates the SHM region
    let env_child = process::Command::new(&exe)
        .args(["--role-env", "--name", &args.name, "--frames", &args.frames.to_string()])
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::inherit())
        .spawn()
        .expect("Failed to spawn env child");

    // Small delay to ensure SHM is created before controller attaches
    std::thread::sleep(Duration::from_millis(100));

    // Spawn ctrl (Controller) child
    let mut ctrl_child = process::Command::new(&exe)
        .args(["--role-ctrl", "--name", &args.name, "--frames", &args.frames.to_string()])
        .stderr(process::Stdio::inherit())
        .stdout(process::Stdio::null())
        .spawn()
        .expect("Failed to spawn ctrl child");

    // Read latency data from env child's stdout
    let env_output = env_child.wait_with_output().expect("Failed to wait for env child");
    let ctrl_status = ctrl_child.wait().expect("Failed to wait for ctrl child");

    if !env_output.status.success() {
        eprintln!("[CSCP IPC Bench] env child exited with: {}", env_output.status);
        process::exit(1);
    }
    if !ctrl_status.success() {
        eprintln!("[CSCP IPC Bench] ctrl child exited with: {}", ctrl_status);
        process::exit(1);
    }

    // Parse latencies (one per line, microseconds)
    let stdout = String::from_utf8_lossy(&env_output.stdout);
    let mut latencies: Vec<u64> = stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .collect();

    if latencies.is_empty() {
        eprintln!("[CSCP IPC Bench] No latency data received!");
        process::exit(1);
    }

    latencies.sort_unstable();

    let count = latencies.len();
    let p50 = percentile(&latencies, 50.0);
    let p95 = percentile(&latencies, 95.0);
    let p99 = percentile(&latencies, 99.0);
    let max = latencies[count - 1];
    let avg = latencies.iter().sum::<u64>() as f64 / count as f64;

    println!("[CSCP IPC Bench] {} round-trips over OS shared memory", count);
    println!("  avg:  {:.1} µs", avg);
    println!("  p50:  {} µs", p50);
    println!("  p95:  {} µs", p95);
    println!("  p99:  {} µs", p99);
    println!("  max:  {} µs", max);
}

/// OmniSim child: creates SHM, publishes frames, waits for echoed commands.
/// Measures round-trip latency and writes raw µs values to stdout (one per line).
fn run_env_child(name: &str, frames: u64) {
    let mut env = EnvSide::<16>::create(name, f64::INFINITY)
        .expect("Env child: failed to create SHM");

    // Wait briefly for controller to attach
    std::thread::sleep(Duration::from_millis(50));

    for seq in 1..=frames {
        let send_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        let mut frame = CscpFrame::new();
        frame.sequence = seq;
        frame.timestamp_us = send_us;
        for (i, s) in frame.state.iter_mut().enumerate() {
            *s = seq as f32 + i as f32 * 0.001;
        }

        // Spin-push until accepted
        while env.publish_state(&frame).is_err() {
            std::hint::spin_loop();
        }

        // Spin-wait for the echo command
        loop {
            if let Some(_cmd) = env.recv_command() {
                break;
            }
            std::hint::spin_loop();
        }

        let recv_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;

        // Output round-trip latency in µs
        println!("{}", recv_us.saturating_sub(send_us));
    }

    env.signal_shutdown();
}

/// Controller child: attaches to SHM, reads state frames, echoes back commands.
fn run_ctrl_child(name: &str, frames: u64) {
    let mut ctrl = CtrlSide::<16>::attach(name, f64::INFINITY)
        .expect("Ctrl child: failed to attach to SHM");

    let mut count: u64 = 0;

    while count < frames {
        if let Some(state) = ctrl.recv_state() {
            // Echo back as a command immediately
            let mut cmd = CscpFrame::new();
            cmd.sequence = state.sequence;
            cmd.timestamp_us = state.timestamp_us;
            for i in 0..cmd.action.len() {
                cmd.action[i] = state.state[i.min(state.state.len() - 1)] * -0.1;
            }

            while ctrl.publish_command(&cmd).is_err() {
                std::hint::spin_loop();
            }

            count += 1;
        } else {
            std::hint::spin_loop();
        }
    }
}

/// Computes the p-th percentile from a sorted slice.
fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
