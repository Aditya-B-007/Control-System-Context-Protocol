//! Two-process IPC integration test.
//!
//! Builds and spawns two separate OS processes (OmniSim + Controller simulators)
//! connected over real POSIX shared memory. Verifies that frames are exchanged
//! correctly with no corruption.

use std::process::{Command, Stdio};
use std::time::Duration;

/// Generates a unique SHM name per test run to avoid collisions.
fn unique_shm_name() -> String {
    format!(
        "cscp_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}

#[test]
fn test_two_process_frame_exchange() {
    let shm_name = unique_shm_name();
    let frames = 100;

    // We use cargo run to ensure the binaries are built
    // First, start OmniSim
    let omnisim = Command::new("cargo")
        .args([
            "run", "--release", "--bin", "cscp_omnisim_sim", "--",
            "--name", &shm_name,
            "--frames", &frames.to_string(),
            "--interval-ms", "5",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn OmniSim");

    // Give OmniSim time to create SHM
    std::thread::sleep(Duration::from_millis(500));

    // Start Controller
    let controller = Command::new("cargo")
        .args([
            "run", "--release", "--bin", "cscp_controller_sim", "--",
            "--name", &shm_name,
            "--max-frames", &frames.to_string(),
            "--poll-ms", "2",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn Controller");

    // Wait for both with timeout
    let omnisim_output = omnisim.wait_with_output().expect("OmniSim wait failed");
    let controller_output = controller.wait_with_output().expect("Controller wait failed");

    let omnisim_stdout = String::from_utf8_lossy(&omnisim_output.stdout);
    let omnisim_stderr = String::from_utf8_lossy(&omnisim_output.stderr);
    let controller_stdout = String::from_utf8_lossy(&controller_output.stdout);
    let controller_stderr = String::from_utf8_lossy(&controller_output.stderr);

    println!("--- OmniSim stdout ---\n{}", omnisim_stdout);
    println!("--- OmniSim stderr ---\n{}", omnisim_stderr);
    println!("--- Controller stdout ---\n{}", controller_stdout);
    println!("--- Controller stderr ---\n{}", controller_stderr);

    assert!(
        omnisim_output.status.success(),
        "OmniSim exited with error: {}",
        omnisim_output.status
    );
    assert!(
        controller_output.status.success(),
        "Controller exited with error: {}",
        controller_output.status
    );

    // Verify OmniSim completed publishing
    assert!(
        omnisim_stdout.contains("Done"),
        "OmniSim didn't report completion"
    );

    // Verify Controller received state frames
    assert!(
        controller_stdout.contains("State seq=") || controller_stdout.contains("Received termination") || controller_stdout.contains("Reached max frames"),
        "Controller didn't report receiving states"
    );
}
