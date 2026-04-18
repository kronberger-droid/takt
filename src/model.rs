use chrono::NaiveDateTime;

/// A single tracked time entry. Storage-agnostic: every `Store` implementation
/// produces and consumes `Entry` values.
#[derive(Clone, Debug)]
pub struct Entry {
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub tag: String,
}
