//! Counters for the Shield panel. Kept in memory; only the lifetime total is persisted by the
//! browser. No per-site data ever leaves the device.

use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize)]
pub struct BlockStats {
    /// Requests blocked since Mizu started.
    pub session_blocked: u64,
    /// Requests blocked since the profile was created (persisted).
    pub lifetime_blocked: u64,
    /// Blocked request counts by tracker host (this session).
    pub by_tracker: HashMap<String, u64>,
}

impl BlockStats {
    pub fn record(&mut self, request_url: &str) {
        self.session_blocked += 1;
        self.lifetime_blocked += 1;
        if let Some(host) = url::Url::parse(request_url).ok().and_then(|u| u.host_str().map(str::to_owned)) {
            *self.by_tracker.entry(host).or_default() += 1;
        }
    }

    /// Top trackers by count, descending.
    pub fn top_trackers(&self, n: usize) -> Vec<(String, u64)> {
        let mut v: Vec<_> = self.by_tracker.iter().map(|(h, c)| (h.clone(), *c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_ranks() {
        let mut s = BlockStats::default();
        s.record("https://a.tracker/x");
        s.record("https://a.tracker/y");
        s.record("https://b.tracker/z");
        assert_eq!(s.session_blocked, 3);
        assert_eq!(s.top_trackers(1), vec![("a.tracker".to_string(), 2)]);
    }
}
