//! Counters for the Shield dashboard. Kept on this device only: the lifetime total, blocks
//! per day (last 60 days) and per site (the sites you visited, top 300) are saved in the
//! profile's `stats.json`; per-tracker counts are for this session.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BlockStats {
    /// Requests blocked since Mizu started.
    #[serde(skip)]
    pub session_blocked: u64,
    /// Requests blocked since the profile was created (persisted).
    pub lifetime_blocked: u64,
    /// Blocked request counts by tracker host (this session).
    #[serde(skip)]
    pub by_tracker: HashMap<String, u64>,
    /// Blocked requests per day, "YYYY-MM-DD" (UTC) → count (persisted).
    pub daily: BTreeMap<String, u64>,
    /// Blocked requests per site you visited (persisted).
    pub by_site: HashMap<String, u64>,
}

const DAYS_KEPT: usize = 60;
const SITES_KEPT: usize = 300;

/// The UTC date of a Unix time, as "YYYY-MM-DD" (civil-from-days, no time zone data).
pub fn date_of(unix_secs: u64) -> String {
    let z = (unix_secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn today() -> String {
    date_of(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0))
}

fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_ascii_lowercase()))
}

impl BlockStats {
    /// A request to `request_url` was blocked on the page `page_url`.
    pub fn record(&mut self, request_url: &str, page_url: &str) {
        self.record_on(request_url, page_url, &today());
    }

    pub fn record_on(&mut self, request_url: &str, page_url: &str, day: &str) {
        self.session_blocked += 1;
        self.lifetime_blocked += 1;
        if let Some(host) = host_of(request_url) {
            *self.by_tracker.entry(host).or_default() += 1;
        }
        if let Some(site) = host_of(page_url) {
            *self.by_site.entry(site).or_default() += 1;
        }
        *self.daily.entry(day.to_owned()).or_default() += 1;
    }

    /// Drops old days and rarely blocked sites so the file stays small.
    pub fn trim(&mut self) {
        while self.daily.len() > DAYS_KEPT {
            let first = self.daily.keys().next().cloned();
            if let Some(k) = first {
                self.daily.remove(&k);
            }
        }
        if self.by_site.len() > SITES_KEPT {
            let mut v: Vec<(String, u64)> = self.by_site.drain().collect();
            v.sort_by_key(|a| std::cmp::Reverse(a.1));
            v.truncate(SITES_KEPT);
            self.by_site = v.into_iter().collect();
        }
    }

    /// Top trackers by count, descending.
    pub fn top_trackers(&self, n: usize) -> Vec<(String, u64)> {
        top(&self.by_tracker, n)
    }

    /// Sites where the most was blocked, descending.
    pub fn top_sites(&self, n: usize) -> Vec<(String, u64)> {
        top(&self.by_site, n)
    }

    /// The last `n` days, oldest first, with zero for days without blocks.
    pub fn last_days(&self, n: usize) -> Vec<(String, u64)> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        (0..n as u64)
            .rev()
            .map(|back| {
                let day = date_of(now.saturating_sub(back * 86_400));
                let c = self.daily.get(&day).copied().unwrap_or(0);
                (day, c)
            })
            .collect()
    }
}

fn top(map: &HashMap<String, u64>, n: usize) -> Vec<(String, u64)> {
    let mut v: Vec<_> = map.iter().map(|(h, c)| (h.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_ranks() {
        let mut s = BlockStats::default();
        s.record_on("https://a.tracker/x", "https://www.news.example/", "2026-11-25");
        s.record_on("https://a.tracker/y", "https://news.example/a", "2026-11-25");
        s.record_on("https://b.tracker/z", "https://shop.example/", "2026-11-24");
        assert_eq!(s.session_blocked, 3);
        assert_eq!(s.top_trackers(1), vec![("a.tracker".to_string(), 2)]);
        assert_eq!(s.top_sites(1), vec![("news.example".to_string(), 2)]);
        assert_eq!(s.daily.get("2026-11-25"), Some(&2));
        // Saved and loaded: session counters are not kept, the rest is.
        let back: BlockStats = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.session_blocked, 0);
        assert_eq!(back.lifetime_blocked, 3);
        assert_eq!(back.by_site.get("shop.example"), Some(&1));
    }

    #[test]
    fn dates_and_trimming() {
        assert_eq!(date_of(0), "1970-01-01");
        assert_eq!(date_of(1_764_028_800), "2025-11-25");
        let mut s = BlockStats::default();
        for d in 1..=70u64 {
            s.daily.insert(date_of(d * 86_400), 1);
        }
        s.trim();
        assert_eq!(s.daily.len(), 60);
        assert_eq!(s.last_days(7).len(), 7);
    }
}
