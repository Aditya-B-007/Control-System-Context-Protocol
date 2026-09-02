//! SPSC queue correctness tests.
//!
//! Verifies push/pop ordering, capacity limits, wrap-around, and
//! concurrent single-producer/single-consumer behavior.

use cscp_connector::ipc::spsc_queue::{CscpFrame, SpscQueue};

/// Helper: create a zeroed SpscQueue in a Box (simulates mmap'd memory).
fn make_queue<const N: usize>() -> Box<SpscQueue<N>> {
    // Allocate zeroed memory to simulate mmap behavior
    let layout = std::alloc::Layout::new::<SpscQueue<N>>();
    let ptr = unsafe { std::alloc::alloc_zeroed(layout) as *mut SpscQueue<N> };
    let queue = unsafe { Box::from_raw(ptr) };
    queue.init();
    queue
}

#[test]
fn test_push_pop_single() {
    let queue = make_queue::<4>();
    let mut frame = CscpFrame::new();
    frame.sequence = 42;
    frame.state[0] = 3.14;

    assert!(queue.try_push(&frame));
    let popped = queue.try_pop().expect("should have one frame");
    assert_eq!(popped.sequence, 42);
    assert!((popped.state[0] - 3.14).abs() < f32::EPSILON);
}

#[test]
fn test_empty_pop_returns_none() {
    let queue = make_queue::<4>();
    assert!(queue.try_pop().is_none());
    assert!(queue.is_empty());
}

#[test]
fn test_capacity_limit() {
    let queue = make_queue::<4>();

    // Fill all 4 slots
    for i in 0..4u64 {
        let mut f = CscpFrame::new();
        f.sequence = i;
        assert!(queue.try_push(&f), "push {} should succeed", i);
    }

    assert!(queue.is_full());
    assert_eq!(queue.len(), 4);

    // 5th push should fail
    let mut overflow = CscpFrame::new();
    overflow.sequence = 999;
    assert!(!queue.try_push(&overflow), "push beyond capacity should fail");
}

#[test]
fn test_fifo_ordering() {
    let queue = make_queue::<8>();

    for i in 0..5u64 {
        let mut f = CscpFrame::new();
        f.sequence = i * 10;
        assert!(queue.try_push(&f));
    }

    for i in 0..5u64 {
        let f = queue.try_pop().expect("should have frame");
        assert_eq!(f.sequence, i * 10);
    }

    assert!(queue.is_empty());
}

#[test]
fn test_wrap_around() {
    let queue = make_queue::<4>();

    // Fill, drain, refill — forces index wrap-around
    for round in 0..10u64 {
        for i in 0..4u64 {
            let mut f = CscpFrame::new();
            f.sequence = round * 100 + i;
            assert!(queue.try_push(&f), "round {} push {} should succeed", round, i);
        }

        for i in 0..4u64 {
            let f = queue.try_pop().expect("should pop");
            assert_eq!(f.sequence, round * 100 + i);
        }

        assert!(queue.is_empty());
    }
}

#[test]
fn test_concurrent_spsc() {
    let layout = std::alloc::Layout::new::<SpscQueue<16>>();
    let ptr = unsafe { std::alloc::alloc_zeroed(layout) as *mut SpscQueue<16> };
    // We need to share this pointer between threads safely.
    // Use Arc around the raw pointer (like shared memory would work).
    let queue_ptr = ptr as usize; // usize is Send

    unsafe { (*ptr).init() };

    let total = 100_000u64;
    let queue_ptr_send = queue_ptr;

    let producer = std::thread::spawn(move || {
        let q = unsafe { &*(queue_ptr_send as *const SpscQueue<16>) };
        for i in 0..total {
            let mut f = CscpFrame::new();
            f.sequence = i;
            f.state[0] = i as f32;
            while !q.try_push(&f) {
                std::hint::spin_loop();
            }
        }
    });

    let consumer = std::thread::spawn(move || {
        let q = unsafe { &*(queue_ptr as *const SpscQueue<16>) };
        let mut received = Vec::with_capacity(total as usize);
        while received.len() < total as usize {
            if let Some(f) = q.try_pop() {
                received.push(f);
            } else {
                std::hint::spin_loop();
            }
        }
        received
    });

    producer.join().unwrap();
    let received = consumer.join().unwrap();

    assert_eq!(received.len(), total as usize);
    for (i, f) in received.iter().enumerate() {
        assert_eq!(f.sequence, i as u64, "wrong sequence at index {}", i);
        assert!(
            (f.state[0] - i as f32).abs() < f32::EPSILON,
            "wrong state at index {}",
            i
        );
    }

    // Clean up
    unsafe { std::alloc::dealloc(ptr as *mut u8, layout) };
}
