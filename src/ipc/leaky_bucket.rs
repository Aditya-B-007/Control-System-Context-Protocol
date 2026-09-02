use std::time::Instant;

/// Token-bucket rate limiter for SPSC queue producers.
///
/// Prevents a fast producer from overwhelming a slow consumer.
/// Each `try_consume()` call refills tokens proportional to elapsed time,
/// then checks if a token is available.
pub struct LeakyBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl LeakyBucket {
    /// Create a new leaky bucket.
    ///
    /// - `max_tokens`: bucket capacity (burst size)
    /// - `refill_rate`: tokens added per second
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens, // start full
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token. Returns `true` if a token was available.
    ///
    /// Call this before each `try_push` on the SPSC queue.
    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Returns the current number of available tokens.
    pub fn available_tokens(&self) -> f64 {
        self.tokens
    }

    /// Returns the configured maximum tokens (burst capacity).
    pub fn max_tokens(&self) -> f64 {
        self.max_tokens
    }

    /// Returns the configured refill rate in tokens per second.
    pub fn refill_rate(&self) -> f64 {
        self.refill_rate
    }
}
