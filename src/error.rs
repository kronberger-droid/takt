use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaktError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Unknown tag: {0}")]
    UnknownTag(String),
    #[error("Ambiguous tag, could be: {0:?}")]
    AmbiguousTag(Vec<String>),
    #[error("No active task")]
    NoActiveTask,
    #[error("Malformed line {line}: {content}")]
    MalformedLine { line: usize, content: String },
    #[error("Invalid datetime on line {line}: '{value}' ({source})")]
    BadDateTime {
        line: usize,
        value: String,
        #[source]
        source: chrono::format::ParseError,
    },
}
