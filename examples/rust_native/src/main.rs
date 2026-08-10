use cscp_connector::{CscpSharedMemory, EnvironmentManager, UniversalAlgorithmEngine};
use std::sync::Arc;

fn main() {
    println!("[Rust Native Example] Initializing CscpSharedMemory");
    let shm = Arc::new(CscpSharedMemory::new());

    let mut env = EnvironmentManager::new(Arc::clone(&shm));
    let mut algo = UniversalAlgorithmEngine::new(Arc::clone(&shm));

    let obs = [0.5f32; 16];
    let rewards = [1.0f32, -0.01f32];
    let mask = [1u8; 8];

    println!("[Side A] Stepping environment");
    let seq = env.step(&obs, &rewards, &mask).unwrap();
    println!("[Side A] Step complete. Sequence: {}", seq);

    println!("[Side B] Reading algorithm payload contract");
    let payload = algo.read_payload();
    println!("[Side B] Received sequence: {}", payload.sequence);

    println!("[Side B] Writing actuation command");
    algo.write_actuation(&[0.1, 0.2, 0.3, 0.4], 0.98, 12).unwrap();

    let (actuation, confidence, latency) = env.read_actuation();
    println!(
        "[Side A] Read actuation: {:?}, confidence: {}, latency: {}us",
        actuation, confidence, latency
    );
}
