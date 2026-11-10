use crate::{Result, Storage, like_pattern, now_ms};
use rusqlite::{OptionalExtension, params};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub last_visit: i64,
}

/// Period for "clear history". Times are Unix milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeRange {
    LastHour,
    LastDay,
    LastWeek,
    LastMonth,
    All,
}

impl TimeRange {
    pub fn since(self, now: i64) -> i64 {
        const HOUR: i64 = 3_600_000;
        match self {
            Self::LastHour => now - HOUR,
            Self::LastDay => now - 24 * HOUR,
            Self::LastWeek => now - 7 * 24 * HOUR,
            Self::LastMonth => now - 30 * 24 * HOUR,
            Self::All => i64::MIN,
        }
    }
}

fn row_to_entry(r: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: r.get(0)?,
        url: r.get(1)?,
        title: r.get(2)?,
        visit_count: r.get(3)?,
        last_visit: r.get(4)?,
    })
}

const COLUMNS: &str = "id, url, title, visit_count, last_visit";

impl Storage {
    /// Records a top-level navigation. Only web URLs are recorded.
    pub fn record_visit(&mut self, url: &str, title: &str) -> Result<()> {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Ok(());
        }
        let now = now_ms();
        let tx = self.conn_mut().transaction()?;
        tx.execute(
            "INSERT INTO history_urls (url, title, visit_count, last_visit) VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(url) DO UPDATE SET
                visit_count = visit_count + 1,
                last_visit = excluded.last_visit,
                title = CASE WHEN excluded.title = '' THEN title ELSE excluded.title END",
            params![url, title, now],
        )?;
        let url_id: i64 =
            tx.query_row("SELECT id FROM history_urls WHERE url = ?1", [url], |r| r.get(0))?;
        tx.execute(
            "INSERT INTO history_visits (url_id, visited_at) VALUES (?1, ?2)",
            params![url_id, now],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Titles usually arrive after the navigation has been recorded.
    pub fn update_history_title(&self, url: &str, title: &str) -> Result<()> {
        if title.is_empty() {
            return Ok(());
        }
        self.conn().execute(
            "UPDATE history_urls SET title = ?2 WHERE url = ?1",
            params![url, title],
        )?;
        Ok(())
    }

    /// Most recent first. Empty query lists everything.
    pub fn search_history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        self.history_page(query, limit, 0)
    }

    pub fn history_page(&self, query: &str, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM history_urls
             WHERE ?1 = '' OR url LIKE ?2 ESCAPE '\\' OR title LIKE ?2 ESCAPE '\\'
             ORDER BY last_visit DESC LIMIT ?3 OFFSET ?4"
        ))?;
        let rows = stmt.query_map(
            params![query, like_pattern(query), limit as i64, offset as i64],
            row_to_entry,
        )?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Address-bar suggestions: prefix matches on the host rank first, then by frecency.
    pub fn suggest_history(&self, input: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let prefix = {
            let mut p = like_pattern(input);
            p.remove(0); // anchor to the start
            p
        };
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS},
                (CASE WHEN replace(replace(replace(url, 'https://', ''), 'http://', ''), 'www.', '')
                      LIKE ?2 ESCAPE '\\' THEN 1000 ELSE 0 END)
                + visit_count * 10
                + (last_visit / 86400000) AS score
             FROM history_urls
             WHERE url LIKE ?1 ESCAPE '\\' OR title LIKE ?1 ESCAPE '\\'
             ORDER BY score DESC LIMIT ?3"
        ))?;
        let rows = stmt.query_map(params![like_pattern(input), prefix, limit as i64], row_to_entry)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn delete_history_entry(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM history_urls WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Deletes visits in the range and any URL left without visits.
    pub fn clear_history(&mut self, range: TimeRange) -> Result<usize> {
        let since = range.since(now_ms());
        let tx = self.conn_mut().transaction()?;
        let removed = tx.execute("DELETE FROM history_visits WHERE visited_at >= ?1", [since])?;
        tx.execute(
            "DELETE FROM history_urls WHERE id NOT IN (SELECT DISTINCT url_id FROM history_visits)",
            [],
        )?;
        // Keep the denormalised counters in sync for URLs that still have older visits.
        tx.execute(
            "UPDATE history_urls SET
                visit_count = (SELECT COUNT(*) FROM history_visits v WHERE v.url_id = history_urls.id),
                last_visit  = (SELECT MAX(visited_at) FROM history_visits v WHERE v.url_id = history_urls.id)",
            [],
        )?;
        tx.commit()?;
        Ok(removed)
    }

    /// Most visited sites for the new-tab page.
    pub fn top_sites(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn().prepare_cached(&format!(
            "SELECT {COLUMNS} FROM history_urls ORDER BY visit_count DESC, last_visit DESC LIMIT ?1"
        ))?;
        let rows = stmt.query_map([limit as i64], row_to_entry)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn history_entry(&self, url: &str) -> Result<Option<HistoryEntry>> {
        Ok(self
            .conn()
            .query_row(
                &format!("SELECT {COLUMNS} FROM history_urls WHERE url = ?1"),
                [url],
                row_to_entry,
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_searches() {
        let mut s = Storage::in_memory().unwrap();
        s.record_visit("https://example.com/", "Example Domain").unwrap();
        s.record_visit("https://example.com/", "").unwrap();
        s.record_visit("https://rust-lang.org/", "Rust").unwrap();
        s.record_visit("mizu://settings/", "Settings").unwrap();

        let all = s.search_history("", 10).unwrap();
        assert_eq!(all.len(), 2);
        let ex = s.history_entry("https://example.com/").unwrap().unwrap();
        assert_eq!(ex.visit_count, 2);
        assert_eq!(ex.title, "Example Domain");

        assert_eq!(s.search_history("rust", 10).unwrap().len(), 1);
        assert_eq!(s.search_history("100%", 10).unwrap().len(), 0);
    }

    #[test]
    fn suggestions_prefer_prefix() {
        let mut s = Storage::in_memory().unwrap();
        s.record_visit("https://docs.rs/serde", "serde docs").unwrap();
        for _ in 0..3 {
            s.record_visit("https://www.reddit.com/r/rust", "rust").unwrap();
        }
        let got = s.suggest_history("re", 5).unwrap();
        assert_eq!(got[0].url, "https://www.reddit.com/r/rust");
    }

    #[test]
    fn deletes_and_clears() {
        let mut s = Storage::in_memory().unwrap();
        s.record_visit("https://a.example/", "A").unwrap();
        s.record_visit("https://b.example/", "B").unwrap();
        let a = s.history_entry("https://a.example/").unwrap().unwrap();
        s.delete_history_entry(a.id).unwrap();
        assert_eq!(s.search_history("", 10).unwrap().len(), 1);
        s.clear_history(TimeRange::LastHour).unwrap();
        assert!(s.search_history("", 10).unwrap().is_empty());
    }
}
