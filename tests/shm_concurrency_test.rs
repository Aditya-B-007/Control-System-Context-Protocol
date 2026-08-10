use cscp_connector::{CscpSharedMemory, EnvironmentManager, UniversalAlgorithmEngine};
use std::sync::Arc;
use std::thread;

#[test]
fn test_shm_lockfree_concurrency() {
    let shm = Arc::new(CscpSharedMemory::new());
    let shm_env = Arc::clone(&shm);
    let shm_algo = Arc::clone(&shm);

    let env_thread = thread::spawn(move || {
        let mut env = EnvironmentManager::new(shm_env);
        let obs = [1.0f32; 16];
        let rewards = [0.5f32; 2];
        let mask = [1u8; 8];

        for _ in 0..100 {
            let _ = env.step(&obs, &rewards, &mask);
        }
    });

    let algo_thread = thread::spawn(move || {
        let mut algo = UniversalAlgorithmEngine::new(shm_algo);
        let action = [0.2f32; 4];

        for _ in 0..100 {
            let _ = algo.read_payload();
            let _ = algo.write_actuation(&action, 0.95, 5);
        }
    });

    env_thread.join().unwrap();
    algo_thread.join().unwrap();

    assert_eq!(shm.get_sequence(), 100);
}
