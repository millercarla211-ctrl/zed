//! Per-engine health tracking with adaptive timeouts and circuit-breaker pattern.
//!
//! Tracks P95 latency per engine over a rolling window of 100 requests and
//! automatically adjusts timeouts.  Unhealthy engines (>50% failure rate) are
//! skipped for 60 s before being retried.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::Serialize;

/// Rolling statistics for a single engine.
pub struct EngineStats {
    /// Ring buffer of last 100 response times (milliseconds).
    latencies: [u32; 100],
    cursor: usize,
    total_requests: u64,
    total_failures: u64,
    last_success: Option<Instant>,
    last_failure: Option<Instant>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EngineHealthSnapshot {
    pub name: String,
    pub total_requests: u64,
    pub total_failures: u64,
    pub failure_rate: f32,
    pub healthy: bool,
    pub adaptive_timeout_ms: u64,
    pub last_success_age_secs: Option<u64>,
    pub last_failure_age_secs: Option<u64>,
}

impl Default for EngineStats {
    fn default() -> Self {
        Self {
            latencies: [0u32; 100],
            cursor: 0,
            total_requests: 0,
            total_failures: 0,
            last_success: None,
            last_failure: None,
        }
    }
}

impl EngineStats {
    /// Record a successful response with the given latency.
    pub fn record_success(&mut self, ms: u32) {
        self.latencies[self.cursor % 100] = ms;
        self.cursor += 1;
        self.total_requests += 1;
        self.last_success = Some(Instant::now());
    }

    /// Record a failure (timeout or HTTP error).
    pub fn record_failure(&mut self) {
        // Store a high latency sentinel so it influences P95
        let sentinel = 30_000u32;
        self.latencies[self.cursor % 100] = sentinel;
        self.cursor += 1;
        self.total_requests += 1;
        self.total_failures += 1;
        self.last_failure = Some(Instant::now());
    }

    /// Compute adaptive timeout: P95 latency × 1.5, clamped to [1 s, 10 s].
    /// Returns the configured static timeout when there is insufficient data.
    pub fn adaptive_timeout(&self, static_ms: u64) -> Duration {
        let count = self.cursor.min(100);
        if count < 5 {
            return Duration::from_millis(static_ms);
        }

        let mut sorted = self.latencies[..count].to_vec();
        sorted.sort_unstable();
        let p95_idx = ((count as f32 * 0.95) as usize).min(count - 1);
        let p95_ms = sorted[p95_idx] as u64;
        let timeout_ms = (p95_ms as f64 * 1.5) as u64;

        Duration::from_millis(timeout_ms.clamp(1_000, 10_000))
    }

    /// Recent failure rate over tracked requests.
    pub fn failure_rate(&self) -> f32 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.total_failures as f32 / self.total_requests as f32
    }

    /// Circuit-breaker: skip if failure rate > 50% (with > 10 requests sampled),
    /// but retry automatically after 60 s.
    pub fn is_healthy(&self) -> bool {
        if self.total_requests > 10 && self.failure_rate() > 0.5 {
            if let Some(last_fail) = self.last_failure {
                return last_fail.elapsed() > Duration::from_secs(60);
            }
            return false;
        }
        true
    }

    pub fn snapshot(&self, name: &str, static_ms: u64) -> EngineHealthSnapshot {
        let now = Instant::now();
        EngineHealthSnapshot {
            name: name.to_string(),
            total_requests: self.total_requests,
            total_failures: self.total_failures,
            failure_rate: self.failure_rate(),
            healthy: self.is_healthy(),
            adaptive_timeout_ms: self.adaptive_timeout(static_ms).as_millis() as u64,
            last_success_age_secs: self
                .last_success
                .map(|instant| now.saturating_duration_since(instant).as_secs()),
            last_failure_age_secs: self
                .last_failure
                .map(|instant| now.saturating_duration_since(instant).as_secs()),
        }
    }
}

/// Tracks health stats for all registered engines.  Lock-free per engine.
#[derive(Default)]
pub struct EngineHealthTracker {
    stats: DashMap<String, RwLock<EngineStats>>,
}

impl EngineHealthTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful engine response.
    pub fn record_success(&self, engine: &str, latency_ms: u32) {
        self.stats
            .entry(engine.to_string())
            .or_default()
            .write()
            .record_success(latency_ms);
    }

    /// Record an engine failure.
    pub fn record_failure(&self, engine: &str) {
        self.stats
            .entry(engine.to_string())
            .or_default()
            .write()
            .record_failure();
    }

    /// Get the adaptive timeout for an engine, falling back to `static_ms`.
    pub fn timeout_for(&self, engine: &str, static_ms: u64) -> Duration {
        match self.stats.get(engine) {
            Some(s) => s.read().adaptive_timeout(static_ms),
            None => Duration::from_millis(static_ms),
        }
    }

    /// Returns `false` if the engine is circuit-broken.
    pub fn is_healthy(&self, engine: &str) -> bool {
        match self.stats.get(engine) {
            Some(s) => s.read().is_healthy(),
            None => true,
        }
    }

    pub fn tracked_engine_count(&self) -> usize {
        self.stats.len()
    }

    pub fn unhealthy_engines(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .stats
            .iter()
            .filter_map(|entry| {
                if entry.value().read().is_healthy() {
                    None
                } else {
                    Some(entry.key().clone())
                }
            })
            .collect();
        names.sort();
        names
    }

    pub fn snapshot(&self, engine: &str, static_ms: u64) -> Option<EngineHealthSnapshot> {
        self.stats
            .get(engine)
            .map(|stats| stats.read().snapshot(engine, static_ms))
    }

    pub fn snapshots(&self, default_timeout_ms: u64) -> Vec<EngineHealthSnapshot> {
        let mut snapshots: Vec<EngineHealthSnapshot> = self
            .stats
            .iter()
            .map(|entry| entry.value().read().snapshot(entry.key(), default_timeout_ms))
            .collect();
        snapshots.sort_by(|left, right| left.name.cmp(&right.name));
        snapshots
    }
}
