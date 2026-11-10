//! Mizu Shield wiring: building the content blocker off the UI thread, refreshing filter lists,
//! and the per-site allowlist.

use crate::queue::{WorkerResult, WorkerSender};
use mizu_core::paths::ProfilePaths;
use mizu_privacy::{ContentBlocker, FilterStore};
use mizu_settings::PrivacySettings;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Whether the latest requested build is `generation` (older builds are discarded).
pub fn is_current(generation: u64) -> bool {
    GENERATION.load(Ordering::SeqCst) == generation
}

/// Compiles the blocker from cached lists, then (if `update`) refreshes stale lists and
/// compiles again when something changed. Results arrive as [`WorkerResult::BlockerReady`].
pub fn rebuild(paths: &ProfilePaths, privacy: &PrivacySettings, sender: WorkerSender, update: bool, force: bool) {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let dir = paths.filters();
    let subs = privacy.filter_lists.clone();
    let custom = privacy.custom_rules.clone();
    std::thread::Builder::new()
        .name("mizu-shield".into())
        .spawn(move || {
            let build = |store: &FilterStore| {
                let started = std::time::Instant::now();
                let texts = store.enabled_texts(&subs, &custom);
                let blocker = ContentBlocker::build(&texts, store.resources(), &[]);
                tracing::info!(
                    rules = blocker.rule_count(),
                    ms = started.elapsed().as_millis() as u64,
                    "content blocker compiled"
                );
                Arc::new(blocker)
            };
            let mut store = FilterStore::open(&dir);
            sender.send(WorkerResult::BlockerReady(build(&store), generation));
            if !update {
                return;
            }
            let mut changed = false;
            for sub in store.due_for_update(&subs, force).into_iter().cloned().collect::<Vec<_>>() {
                if !is_current(generation) {
                    return;
                }
                changed |= store.update_list(&sub).is_ok();
            }
            if store.resources_due(force) {
                changed |= store.update_resources().is_ok();
            }
            sender.send(WorkerResult::FiltersUpdated);
            if changed && is_current(generation) {
                sender.send(WorkerResult::BlockerReady(build(&store), generation));
            }
        })
        .map_err(|e| tracing::error!(error = %e, "could not spawn shield worker"))
        .ok();
}

pub fn normalize_host(h: &str) -> String {
    h.trim().trim_start_matches("www.").to_ascii_lowercase()
}

/// Whether blocking is disabled for the page `url` (host or a parent domain allowlisted).
pub fn allowlisted(allowlist: &HashSet<String>, url: &str) -> bool {
    if allowlist.is_empty() {
        return false;
    }
    let Some(host) = url::Url::parse(url).ok().and_then(|u| u.host_str().map(normalize_host)) else {
        return false;
    };
    let mut h = host.as_str();
    loop {
        if allowlist.contains(h) {
            return true;
        }
        match h.split_once('.') {
            Some((_, rest)) if rest.contains('.') => h = rest,
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_matches_parents() {
        let set: HashSet<String> = ["example.com".to_string()].into();
        assert!(allowlisted(&set, "https://www.example.com/x"));
        assert!(allowlisted(&set, "https://shop.example.com/"));
        assert!(!allowlisted(&set, "https://example.org/"));
        assert!(!allowlisted(&set, "https://notexample.com/"));
    }
}
