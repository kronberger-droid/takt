//! File-backed `Store` implementation. Wraps the existing `TaskLog` and
//! `TagTree` types plus the multi-month file walk that currently lives in
//! `main.rs`.
//!
//! All file-layout knowledge lives in this module: other code should construct
//! a `FlatStore` and go through the trait.

use std::path::PathBuf;

use chrono::{Datelike, Local, Months, NaiveDate, NaiveDateTime};

use crate::{
    error::TaktError, log::TaskLog, model::Entry, store::Store, tags::TagTree,
};

pub struct FlatStore {
    /// Root directory for takt data (typically `$XDG_DATA_HOME/takt`).
    /// Contains `tags` file and a `log/` subdirectory with `YYYY-MM.takt` files.
    data_dir: PathBuf,
}

impl FlatStore {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn tags_path(&self) -> PathBuf {
        self.data_dir.join("tags")
    }

    fn log_path_for_date(&self, date: NaiveDate) -> PathBuf {
        self.data_dir
            .join(format!("log/{}.takt", date.format("%Y-%m")))
    }

    fn active_log(&self) -> Result<Option<(TaskLog, PathBuf)>, TaktError> {
        let today = Local::now().date_naive();
        let path = self.log_path_for_date(today);
        let log = TaskLog::load(&path)?;
        if log.active().is_some() {
            return Ok(Some((log, path)));
        }
        let prev = today.checked_sub_months(Months::new(1)).unwrap();
        let prev_path = self.log_path_for_date(prev);
        let prev_log = TaskLog::load(&prev_path)?;
        if prev_log.active().is_some() {
            return Ok(Some((prev_log, prev_path)));
        }
        Ok(None)
    }
}

impl Store for FlatStore {
    fn start(&mut self, resolved_tag: &str) -> Result<Entry, TaktError> {
        let log_path = self.log_path_for_date(Local::now().date_naive());
        let mut log = TaskLog::load(&log_path)?;
        log.start(resolved_tag)?;
        log.save(&log_path)?;
        Ok(log.entries().last().expect("just started").clone())
    }

    fn stop(&mut self) -> Result<Entry, TaktError> {
        let (mut active, path) =
            self.active_log()?.ok_or(TaktError::NoActiveTask)?;
        active.stop()?;
        active.save(&path)?;
        Ok(active.entries().last().expect("just stopped").clone())
    }

    fn active(&self) -> Result<Option<Entry>, TaktError> {
        Ok(self
            .active_log()?
            .and_then(|(log, _)| log.active().cloned()))
    }

    fn entries_between(
        &self,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<Entry>, TaktError> {
        let mut cursor = start.date().with_day(1).unwrap();
        let end_month = end.with_day(1).unwrap().date();
        let mut entries = Vec::new();

        while cursor <= end_month {
            let path = self.log_path_for_date(cursor);
            if path.exists() {
                match TaskLog::load(&path) {
                    Ok(log) => entries.extend(
                        log.entries()
                            .iter()
                            .filter(|e| e.start >= start && e.start < end)
                            .cloned(),
                    ),
                    Err(e) => eprintln!(
                        "warning: failed to load {} ({e}) — skipped",
                        cursor.format("%Y-%m")
                    ),
                }
            }
            cursor = cursor + chrono::Months::new(1);
        }
        Ok(entries)
    }

    fn tag_add(&mut self, path: &str) -> Result<(), TaktError> {
        let mut tag_tree = TagTree::load(&self.tags_path())?;
        tag_tree.add(path);
        tag_tree.save(&self.tags_path())?;

        Ok(())
    }

    fn tag_list(&self) -> Result<String, TaktError> {
        let tag_tree = TagTree::load(&self.tags_path())?;
        Ok(tag_tree.write())
    }

    fn tag_resolve(&self, name: &str) -> Result<String, TaktError> {
        let tag_tree = TagTree::load(&self.tags_path())?;
        tag_tree.resolve(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store_tests;
    use tempfile::TempDir;

    /// Each test gets its own tempdir so the shared harness can `start`/`stop`
    /// without leaking state across tests or onto the developer's real
    /// `$XDG_DATA_HOME/takt/` directory.
    fn new_store() -> FlatStore {
        let dir = TempDir::new().unwrap();
        // Leak the TempDir: v0.3 tests are short-lived and OS cleanup handles it.
        // A cleaner solution wraps FlatStore in a test guard that owns TempDir.
        let path = dir.keep();
        FlatStore::new(path)
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
