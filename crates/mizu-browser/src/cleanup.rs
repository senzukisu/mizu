//! Automatic cookie cleanup: cookies and site storage of sites you have not visited for a
//! while are deleted, except sites marked "Keep me signed in". Runs a minute after start and
//! then once a day, only when Settings → Privacy turns it on.

use std::collections::{BTreeSet, HashMap};

use crate::app::App;

/// Cookie domains (without leading "." or "www.") that should be forgotten: none of their
/// hosts was visited since `cutoff` (ms), and none is kept.
pub fn stale_domains(cookie_domains: &[String], visits: &HashMap<String, (i64, i64)>, keep: &[String], cutoff: i64) -> Vec<String> {
    let related = |a: &str, b: &str| a == b || a.ends_with(&format!(".{b}")) || b.ends_with(&format!(".{a}"));
    let mut out = BTreeSet::new();
    for raw in cookie_domains {
        let d = raw.trim_start_matches('.').trim_start_matches("www.").to_ascii_lowercase();
        if d.is_empty() || keep.iter().any(|k| related(&d, k)) {
            continue;
        }
        // A cookie for "example.com" counts visits to "example.com" and its subdomains.
        let last = visits
            .iter()
            .filter(|(h, _)| *h == &d || h.ends_with(&format!(".{d}")))
            .map(|(_, (last, _))| *last)
            .max()
            .unwrap_or(0);
        if last < cutoff {
            out.insert(d);
        }
    }
    out.into_iter().collect()
}

impl App {
    pub fn run_cookie_cleanup(&mut self) {
        let days = self.settings.privacy.cookie_cleanup_days;
        if days == 0 {
            return;
        }
        let Some(view) = crate::internal::any_normal_view(self) else { return };
        let visits = self.storage.host_visits().unwrap_or_default();
        let keep = self.settings.privacy.keep_signed_in.clone();
        let cutoff = mizu_storage::now_ms() - i64::from(days) * 86_400_000;
        let v = view.clone();
        view.all_cookies(move |list| {
            let domains: Vec<String> = list.iter().map(|c| c.domain.clone()).collect();
            let stale = stale_domains(&domains, &visits, &keep, cutoff);
            tracing::info!(days, sites = stale.len(), "cookie cleanup");
            for d in stale {
                v.delete_cookies_for(&format!("https://{d}/"), |_| {});
                v.delete_cookies_for(&format!("https://www.{d}/"), |_| {});
                v.clear_origin_storage(&format!("https://{d}"));
                v.clear_origin_storage(&format!("https://www.{d}"));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::stale_domains;
    use std::collections::HashMap;

    #[test]
    fn keeps_visited_and_kept_sites() {
        let mut visits = HashMap::new();
        visits.insert("mail.example.com".to_owned(), (1_000, 3));
        visits.insert("old.test".to_owned(), (10, 1));
        let cookies = [".example.com", "old.test", "www.bank.test", "ads.tracker.test"].map(String::from);
        let keep = vec!["bank.test".to_owned()];
        let stale = stale_domains(&cookies, &visits, &keep, 500);
        assert_eq!(stale, vec!["ads.tracker.test".to_owned(), "old.test".to_owned()]);
    }
}
