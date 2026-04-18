//! Storage abstraction. The CLI drives every operation through `dyn Store`,
//! so adding a new backend (SQLite, HTTP client, …) is a matter of writing a
//! new `impl Store for …` in a sibling file.

use chrono::NaiveDateTime;

use crate::{error::TaktError, model::Entry};

pub use flat::FlatStore;

mod flat;

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
