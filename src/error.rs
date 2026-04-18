use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaktError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("could not locate a data directory (is $HOME set?)")]
    NoDataDir,
    #[error("unknown tag: {0}")]
    UnknownTag(String),
    #[error("ambiguous tag, could be: {0:?}")]
    AmbiguousTag(Vec<String>),
    #[error("no active task — run `takt start <tag>` first")]
    NoActiveTask,
    #[error("malformed line {}: {content}", line + 1)]
    MalformedLine { line: usize, content: String },
    #[error("unexpected indent on line {}: expected depth ≤ {max}, got {depth}", line + 1)]
    UnexpectedIndent {
        line: usize,
        max: usize,
        depth: usize,
    },
    #[error("invalid datetime on line {}: '{value}' ({source})", line + 1)]
    BadDateTime {
        line: usize,
        value: String,
        #[source]
        source: chrono::format::ParseError,
    },
}
