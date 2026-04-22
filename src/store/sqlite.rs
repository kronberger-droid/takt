//! SQLite-backed `Store` implementation via `rusqlite`.
//!
//! Every query is scoped to a single `user_id` passed at construction — this
//! keeps the signature ready for v0.3's multi-user server without adding
//! user-handling complexity to the single-user CLI path.
//!
//! Timestamps are stored as INTEGER unix seconds; `NaiveDateTime` is treated
//! as UTC-equivalent for the conversion (consistent with how the text log
//! format already ignores timezones).

use std::path::Path;

use chrono::{DateTime, Local, NaiveDateTime};
use rusqlite::{Connection, OptionalExtension, params};

use crate::{error::TaktError, model::Entry, store::Store, tags::TagTree};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    name  TEXT UNIQUE NOT NULL,
    token TEXT UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS entries (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    start   INTEGER NOT NULL,
    end     INTEGER,
    tag     TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE INDEX IF NOT EXISTS idx_entries_user_start ON entries(user_id, start);
CREATE INDEX IF NOT EXISTS idx_entries_user_active ON entries(user_id) WHERE end IS NULL;

CREATE TABLE IF NOT EXISTS tags (
    user_id INTEGER NOT NULL,
    path    TEXT NOT NULL,
    PRIMARY KEY (user_id, path),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
"#;

pub struct SqliteStore {
    conn: Connection,
    user_id: i64,
}

impl SqliteStore {
    /// Open (or create) the database at `path` and scope operations to
    /// `user_id`. Runs schema migrations on first open.
    pub fn open(path: &Path, user_id: i64) -> Result<Self, TaktError> {
        let conn = Connection::open(path)?;
        Self::migrate(&conn)?;
        Ok(Self { conn, user_id })
    }

    /// Ensure a user row exists for `user_id`, inserting a default one
    /// if needed. Used by the server in the pre-auth phase.
    pub fn ensure_default_user(&self) -> Result<(), TaktError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO users (id, name, token) \
             VALUES (?1, 'default', 'default')",
            params![self.user_id],
        )?;
        Ok(())
    }

    /// In-memory database for tests.
    #[cfg(test)]
    pub fn new_in_memory(user_id: i64) -> Result<Self, TaktError> {
        let conn = Connection::open_in_memory()?;
        Self::migrate(&conn)?;
        // Satisfy the FK by inserting the user row up front.
        conn.execute(
            "INSERT INTO users (id, name, token) VALUES (?1, ?2, ?3)",
            params![
                user_id,
                format!("user{user_id}"),
                format!("token{user_id}")
            ],
        )?;
        Ok(Self { conn, user_id })
    }

    fn migrate(conn: &Connection) -> Result<(), TaktError> {
        conn.execute_batch(SCHEMA)?;
        // TODO(future): insert a row into schema_version and gate future
        // migrations on reading it. For v0.3 we only have one schema.
        Ok(())
    }
}

/// Convert `NaiveDateTime` → unix seconds. Treats the naive value as UTC
/// for the purpose of the conversion, matching the text log format's
/// timezone-free semantics.
fn to_ts(dt: NaiveDateTime) -> i64 {
    dt.and_utc().timestamp()
}

/// Inverse of `to_ts`.
fn from_ts(ts: i64) -> NaiveDateTime {
    DateTime::from_timestamp(ts, 0)
        .expect("valid unix timestamp from DB")
        .naive_utc()
}

/// Materialize a single `Entry` from a row with columns `(start, end, tag)`.
/// Every query in this module that returns entries selects those three
/// columns in that order so this helper works uniformly.
fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<Entry> {
    let start_ts: i64 = row.get(0)?;
    let end_ts: Option<i64> = row.get(1)?;
    let tag: String = row.get(2)?;
    Ok(Entry {
        start: from_ts(start_ts),
        end: end_ts.map(from_ts),
        tag,
    })
}

impl Store for SqliteStore {
    fn start(&mut self, resolved_tag: &str) -> Result<Entry, TaktError> {
        let now_ts = to_ts(Local::now().naive_local());
        let tx = self.conn.transaction()?;

        tx.execute(
            "UPDATE entries SET end = ?1 \
             WHERE user_id = ?2 AND end IS NULL",
            params![now_ts, self.user_id],
        )?;

        let entry = tx.query_row(
            "INSERT INTO entries (user_id, start, tag) \
             VALUES (?1, ?2, ?3) \
             RETURNING start, end, tag",
            params![self.user_id, now_ts, resolved_tag],
            row_to_entry,
        )?;

        tx.commit()?;
        Ok(entry)
    }

    fn stop(&mut self) -> Result<Entry, TaktError> {
        let now_ts = to_ts(Local::now().naive_local());
        let tx = self.conn.transaction()?;

        let entry = tx
            .query_row(
                "UPDATE entries SET end = ?1 \
                 WHERE user_id = ?2 AND end IS NULL \
                 RETURNING start, end, tag",
                params![now_ts, self.user_id],
                row_to_entry,
            )
            .optional()?
            .ok_or(TaktError::NoActiveTask)?;

        tx.commit()?;
        Ok(entry)
    }

    fn active(&self) -> Result<Option<Entry>, TaktError> {
        self.conn
            .query_row(
                "SELECT start, end, tag FROM entries \
                 WHERE user_id = ?1 AND end IS NULL \
                 LIMIT 1",
                params![self.user_id],
                row_to_entry,
            )
            .optional()
            .map_err(Into::into)
    }

    fn entries_between(
        &self,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<Entry>, TaktError> {
        let start_ts = to_ts(start);
        let end_ts = to_ts(end);

        self.conn
            .prepare(
                "SELECT start, end, tag FROM entries \
                 WHERE user_id = ?1 AND start >= ?2 AND start < ?3 \
                 ORDER BY start",
            )?
            .query_map(params![self.user_id, start_ts, end_ts], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn tag_add(&mut self, path: &str) -> Result<(), TaktError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO tags (user_id, path) VALUES (?1, ?2)",
            params![self.user_id, path],
        )?;
        Ok(())
    }

    fn tag_list(&self) -> Result<String, TaktError> {
        let mut stmt = self.conn.prepare(
            "SELECT path FROM tags WHERE user_id = ?1 ORDER BY path",
        )?;
        let rows = stmt
            .query_map(params![self.user_id], |row| row.get::<_, String>(0))?;

        let mut tree = TagTree::default();

        for row in rows {
            tree.add(&row?);
        }

        Ok(tree.write())
    }

    fn tag_resolve(&self, name: &str) -> Result<String, TaktError> {
        let mut stmt = self
            .conn
            .prepare("SELECT path FROM tags WHERE user_id = ?1")?;
        let rows = stmt
            .query_map(params![self.user_id], |row| row.get::<_, String>(0))?;

        let mut tree = TagTree::default();

        for row in rows {
            tree.add(&row?);
        }

        tree.resolve(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store_tests;

    fn new_store() -> SqliteStore {
        SqliteStore::new_in_memory(1).unwrap()
    }

    store_tests!(
        new_store,
        active_is_none_on_empty,
        start_creates_entry,
        start_autostops_active,
        stop_returns_completed_entry,
        stop_errors_without_active,
        tag_add_then_resolve_leaf,
        tag_list_renders_tree,
    );
}
