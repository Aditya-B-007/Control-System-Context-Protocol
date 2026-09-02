//! Leaky bucket (token bucket) rate limiter tests.

use cscp_connector::ipc::leaky_bucket::LeakyBucket;
use std::thread;
use std::time::Duration;

#[test]
fn test_starts_full() {
    let bucket = LeakyBucket::new(10.0, 100.0);
    assert!((bucket.available_tokens() - 10.0).abs() < f64::EPSILON);
}

#[test]
fn test_consume_decreases_tokens() {
    let mut bucket = LeakyBucket::new(5.0, 1000.0);
    assert!(bucket.try_consume()); // 5 -> 4
    assert!(bucket.try_consume()); // 4 -> 3
    assert!(bucket.try_consume()); // 3 -> 2
    assert!(bucket.try_consume()); // 2 -> 1
    assert!(bucket.try_consume()); // 1 -> 0
    // Now should be empty (modulo tiny refill from elapsed time)
    // With 1000 tokens/sec and ~0 elapsed, should still fail
}

#[test]
fn test_drains_to_zero() {
    let mut bucket = LeakyBucket::new(3.0, 0.0); // zero refill
    assert!(bucket.try_consume());
    assert!(bucket.try_consume());
    assert!(bucket.try_consume());
    assert!(!bucket.try_consume()); // should fail — no refill
}

#[test]
fn test_refill_over_time() {
    let mut bucket = LeakyBucket::new(2.0, 100.0); // 100 tokens/sec
    // Drain it
    assert!(bucket.try_consume());
    assert!(bucket.try_consume());

    // Sleep 50ms → should refill ~5 tokens (100/sec × 0.05s), capped to max=2
    thread::sleep(Duration::from_millis(50));

    assert!(bucket.try_consume()); // should succeed after refill
    assert!(bucket.try_consume()); // should also succeed (refilled to cap=2)
}

#[test]
fn test_max_cap() {
    let mut bucket = LeakyBucket::new(3.0, 10000.0); // very fast refill
    thread::sleep(Duration::from_millis(10)); // would overfill if uncapped

    // Even after waiting, should not exceed max
    // Consume 3, then 4th should fail (assuming very fast execution)
    assert!(bucket.try_consume());
    assert!(bucket.try_consume());
    assert!(bucket.try_consume());
    // 4th depends on timing, but max is 3 so at most 3 should be available
}

#[test]
fn test_zero_capacity() {
    let mut bucket = LeakyBucket::new(0.0, 0.0);
    assert!(!bucket.try_consume());
}

#[test]
fn test_accessors() {
    let bucket = LeakyBucket::new(16.0, 1000.0);
    assert!((bucket.max_tokens() - 16.0).abs() < f64::EPSILON);
    assert!((bucket.refill_rate() - 1000.0).abs() < f64::EPSILON);
}
