use cscp_connector::{CscpSharedMemory, EnvironmentManager, UniversalAlgorithmEngine};
use std::sync::Arc;
use std::time::Instant;

#[test]
fn test_ipc_latency_benchmark() {
    let shm = Arc::new(CscpSharedMemory::new());
    let mut env = EnvironmentManager::new(Arc::clone(&shm));
    let mut algo = UniversalAlgorithmEngine::new(Arc::clone(&shm));

    let obs = [0.1f32; 16];
    let rewards = [1.0f32, 0.0f32];
    let mask = [1u8; 8];
    let action = [0.5f32; 4];

    let iterations = 100_000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = env.step(&obs, &rewards, &mask);
        let _ = algo.read_payload();
        let _ = algo.write_actuation(&action, 0.99, 1);
        let _ = env.read_actuation();
    }

    let elapsed = start.elapsed();
    let avg_latency_us = (elapsed.as_micros() as f64) / (iterations as f64);
    println!(
        "\n[IPC Benchmark] {} round-trips completed in {:?}. Average Latency per step: {:.3} µs",
        iterations, elapsed, avg_latency_us
    );

    assert!(
        avg_latency_us < 10.0,
        "Average IPC latency ({:.3} µs) exceeded 10 µs threshold!",
        avg_latency_us
    );
}
