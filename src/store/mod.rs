//! Storage abstraction. The CLI drives every operation through `dyn Store`,
//! so adding a new backend (SQLite, HTTP client, …) is a matter of writing a
//! new `impl Store for …` in a sibling file.

use chrono::NaiveDateTime;

use crate::{error::TaktError, model::Entry};

pub use flat::FlatStore;
pub use sqlite::SqliteStore;

mod flat;
mod sqlite;

/// The operations the CLI performs on persistent state.
///
/// Implementations must preserve these contracts:
///   * `start` auto-stops any currently active entry before creating a new one.
///   * `stop` errors with `TaktError::NoActiveTask` when nothing is running.
///   * `entries_between` filters on the entry's **start** time, half-open:
///     entries with `start ∈ [start, end)`.
///   * `tag_list` must produce the same rendering across backends — route
///     through a shared helper rather than formatting ad-hoc.
pub trait Store {
    /// Start tracking `resolved_tag`, auto-stopping any active entry.
    /// Returns the newly-created entry.
    fn start(&mut self, resolved_tag: &str) -> Result<Entry, TaktError>;

    /// Stop the active entry. Returns the now-completed entry.
    /// Errors with `TaktError::NoActiveTask` if nothing is active.
    fn stop(&mut self) -> Result<Entry, TaktError>;

    /// The currently active entry, if any.
    fn active(&self) -> Result<Option<Entry>, TaktError>;

    /// Entries whose `start` falls inside `[start, end)`, chronological order.
    fn entries_between(
        &self,
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) -> Result<Vec<Entry>, TaktError>;

    /// Add a tag path like "work/project-x/new-task". No-op if already present.
    fn tag_add(&mut self, path: &str) -> Result<(), TaktError>;

    /// Rendered tag tree for display.
    fn tag_list(&self) -> Result<String, TaktError>;

    /// Resolve a tag by leaf name or full path. Returns the canonical path.
    /// Errors with `UnknownTag` or `AmbiguousTag`.
    fn tag_resolve(&self, name: &str) -> Result<String, TaktError>;
}

/// Expands into `#[test] fn <name>() { test_harness::<name>(&mut $new()); }`
/// for every name listed. Keeps each backend's test module a single macro call.
#[cfg(test)]
#[macro_export]
macro_rules! store_tests {
    ($new:expr, $($name:ident),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                $crate::store::test_harness::$name(&mut $new());
            }
        )*
    };
}

/// Shared assertions exercised by every `Store` implementation. Each helper
/// takes `&mut S: Store` and panics on contract violations. Backends run them
/// via the `store_tests!` macro above.
#[cfg(test)]
pub(crate) mod test_harness {
    use super::*;

    pub fn active_is_none_on_empty<S: Store>(store: &mut S) {
        assert!(store.active().unwrap().is_none());
    }

    pub fn start_creates_entry<S: Store>(store: &mut S) {
        let e = store.start("work/foo").unwrap();
        assert_eq!(e.tag, "work/foo");
        assert!(e.end.is_none());
    }

    pub fn start_autostops_active<S: Store>(store: &mut S) {
        store.start("work/foo").unwrap();
        store.start("work/bar").unwrap();
        let active = store.active().unwrap().expect("something should be active");
        assert_eq!(active.tag, "work/bar");
    }

    pub fn stop_returns_completed_entry<S: Store>(store: &mut S) {
        store.start("work/foo").unwrap();
        let e = store.stop().unwrap();
        assert_eq!(e.tag, "work/foo");
        assert!(e.end.is_some());
        assert!(store.active().unwrap().is_none());
    }

    pub fn stop_errors_without_active<S: Store>(store: &mut S) {
        let err = store.stop().unwrap_err();
        assert!(matches!(err, TaktError::NoActiveTask));
    }

    pub fn tag_add_then_resolve_leaf<S: Store>(store: &mut S) {
        store.tag_add("work/project-x/fix-bug").unwrap();
        assert_eq!(
            store.tag_resolve("fix-bug").unwrap(),
            "work/project-x/fix-bug"
        );
    }

    pub fn tag_list_renders_tree<S: Store>(store: &mut S) {
        store.tag_add("work/project-x").unwrap();
        store.tag_add("study/math").unwrap();
        let listed = store.tag_list().unwrap();
        assert!(listed.contains("work"));
        assert!(listed.contains("project-x"));
        assert!(listed.contains("study"));
        assert!(listed.contains("math"));
    }
}
