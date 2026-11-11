//! Numbered schema migrations. Each runs inside a transaction and bumps `user_version`.
//! Never edit a migration that has shipped – append a new one instead.

use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    // 1 – initial schema
    r#"
    CREATE TABLE history_urls (
        id          INTEGER PRIMARY KEY,
        url         TEXT NOT NULL UNIQUE,
        title       TEXT NOT NULL DEFAULT '',
        visit_count INTEGER NOT NULL DEFAULT 0,
        last_visit  INTEGER NOT NULL
    );
    CREATE INDEX history_urls_last_visit ON history_urls(last_visit DESC);

    CREATE TABLE history_visits (
        id         INTEGER PRIMARY KEY,
        url_id     INTEGER NOT NULL REFERENCES history_urls(id) ON DELETE CASCADE,
        visited_at INTEGER NOT NULL
    );
    CREATE INDEX history_visits_time ON history_visits(visited_at);
    CREATE INDEX history_visits_url ON history_visits(url_id);

    CREATE TABLE bookmarks (
        id         INTEGER PRIMARY KEY,
        parent_id  INTEGER REFERENCES bookmarks(id) ON DELETE CASCADE,
        kind       TEXT NOT NULL CHECK (kind IN ('folder', 'url')),
        title      TEXT NOT NULL,
        url        TEXT,
        position   INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL
    );
    CREATE INDEX bookmarks_parent ON bookmarks(parent_id, position);
    CREATE INDEX bookmarks_url ON bookmarks(url);
    INSERT INTO bookmarks (id, parent_id, kind, title, position, created_at)
        VALUES (1, NULL, 'folder', 'Bookmarks bar', 0, 0),
               (2, NULL, 'folder', 'Other bookmarks', 1, 0);

    CREATE TABLE downloads (
        id             INTEGER PRIMARY KEY,
        url            TEXT NOT NULL,
        path           TEXT NOT NULL,
        mime           TEXT NOT NULL DEFAULT '',
        total_bytes    INTEGER NOT NULL DEFAULT -1,
        received_bytes INTEGER NOT NULL DEFAULT 0,
        state          TEXT NOT NULL,
        started_at     INTEGER NOT NULL,
        finished_at    INTEGER
    );

    CREATE TABLE permissions (
        origin     TEXT NOT NULL,
        kind       TEXT NOT NULL,
        state      TEXT NOT NULL CHECK (state IN ('allow', 'block')),
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (origin, kind)
    );

    CREATE TABLE session (
        id       INTEGER PRIMARY KEY CHECK (id = 1),
        data     TEXT NOT NULL,
        saved_at INTEGER NOT NULL
    );
    "#,
    // 2 – password manager (secrets are encrypted by the browser before storage)
    r#"
    CREATE TABLE logins (
        id         INTEGER PRIMARY KEY,
        origin     TEXT NOT NULL,
        username   TEXT NOT NULL,
        secret     BLOB NOT NULL,
        note       TEXT NOT NULL DEFAULT '',
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        last_used  INTEGER NOT NULL DEFAULT 0,
        UNIQUE (origin, username)
    );
    CREATE INDEX logins_origin ON logins(origin);
    CREATE TABLE login_never (origin TEXT PRIMARY KEY);
    "#,
];

pub fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    let current: usize = conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    if current > MIGRATIONS.len() {
        tracing::warn!(current, known = MIGRATIONS.len(), "database is newer than this build");
        return Ok(());
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let version = index + 1;
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version as i64)?;
        tx.commit()?;
        tracing::info!(version, "applied database migration");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();
        run(&mut conn).unwrap();
        let v: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(v as usize, MIGRATIONS.len());
    }
}
