use std::{io, path::Path};

use chrono::NaiveDateTime;

use crate::error::TaktError;

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) start: NaiveDateTime,
    pub(crate) end: Option<NaiveDateTime>,
    pub(crate) tag: String,
}

#[derive(Debug)]
pub struct TaskLog {
    entries: Vec<Entry>,
}

impl TaskLog {
    pub fn load(path: &Path) -> Result<TaskLog, TaktError> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(TaskLog {
                    entries: Vec::new(),
                });
            }
            Err(e) => return Err(TaktError::Io(e)),
        };
        Self::parse(&content)
    }

    pub fn write(&self) -> String {
        let mut content = String::new();
        let mut entry_string;
        for entry in &self.entries {
            if let Some(end) = entry.end {
                entry_string =
                    format!("{} -- {} | {}", entry.start, end, entry.tag);
            } else {
                entry_string = format!("{} -- * | {}", entry.start, entry.tag);
            }
            content.push_str(&entry_string);
            content.push('\n');
        }
        content
    }

    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.write())
    }

    pub fn parse(content: &str) -> Result<TaskLog, TaktError> {
        let mut entries = Vec::new();

        for (i, raw) in content.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }

            let (range, tag) = line.split_once(" | ").ok_or_else(|| {
                TaktError::MalformedLine {
                    line: i,
                    content: line.to_string(),
                }
            })?;

            let (start_str, end_str) =
                range.split_once(" -- ").ok_or_else(|| {
                    TaktError::MalformedLine {
                        line: i,
                        content: line.to_string(),
                    }
                })?;

            let parse_dt = |s: &str| {
                NaiveDateTime::parse_from_str(s.trim(), "%Y-%m-%d %H:%M:%S")
                    .map_err(|e| TaktError::BadDateTime {
                        line: i,
                        value: s.to_string(),
                        source: e,
                    })
            };

            let start = parse_dt(start_str)?;

            let end = match end_str.trim() {
                "*" => None,
                s => Some(parse_dt(s)?),
            };
            entries.push(Entry {
                start,
                end,
                tag: tag.trim().to_string(),
            });
        }

        Ok(TaskLog { entries })
    }

    pub fn active(&self) -> Option<&Entry> {
        self.entries.iter().find(|e| e.end.is_none())
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn start(&mut self, tag: &str) -> Result<(), TaktError> {
        if self.active().is_some() {
            self.stop()?;
        }
        let entry = Entry {
            start: chrono::Local::now().naive_local(),
            end: None,
            tag: tag.into(),
        };
        self.entries.push(entry);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), TaktError> {
        let active = self.entries.iter_mut().find(|e| e.end.is_none());
        match active {
            Some(entry) => {
                entry.end = Some(chrono::Local::now().naive_local());
                Ok(())
            }
            None => Err(TaktError::NoActiveTask),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
2026-04-06 09:15:04 -- 2026-04-06 11:30:10 | work/project-x/fix-bug
2026-04-06 11:35:02 -- * | study/math/linear-algebra";

    #[test]
    fn parse_and_write_round_trip() {
        let log = TaskLog::parse(SAMPLE).unwrap();
        let output = log.write();
        assert_eq!(output.trim_end(), SAMPLE)
    }

    #[test]
    fn parse_completed_entry() {
        let log = TaskLog::parse(SAMPLE).unwrap();
        let entry = &log.entries[0];
        assert_eq!(entry.tag, "work/project-x/fix-bug");
        assert!(entry.end.is_some());
    }

    #[test]
    fn parse_active_entry() {
        let log = TaskLog::parse(SAMPLE).unwrap();
        let entry = &log.entries[1];
        assert_eq!(entry.tag, "study/math/linear-algebra");
        assert!(entry.end.is_none());
    }

    #[test]
    fn active_returns_running_task() {
        let log = TaskLog::parse(SAMPLE).unwrap();
        let active = log.active().unwrap();
        assert_eq!(active.tag, "study/math/linear-algebra");
    }

    #[test]
    fn active_returns_none_when_all_stopped() {
        let input = "2026-04-06 09:15:04 -- 2026-04-06 11:30:10 | work/fix-bug";
        let log = TaskLog::parse(input).unwrap();
        assert!(log.active().is_none());
    }

    #[test]
    fn start_adds_entry() {
        let mut log = TaskLog::parse("").unwrap();
        log.start("work/project-x").unwrap();
        assert_eq!(log.entries.len(), 1);
        assert_eq!(log.entries[0].tag, "work/project-x");
        assert!(log.entries[0].end.is_none());
    }

    #[test]
    fn start_stops_active_task() {
        let mut log = TaskLog::parse(SAMPLE).unwrap();
        // "study/math/linear-algebra" is active
        log.start("work/new-task").unwrap();
        // previous task should now be stopped
        assert!(log.entries[1].end.is_some());
        // new task should be active
        assert_eq!(log.entries[2].tag, "work/new-task");
        assert!(log.entries[2].end.is_none());
    }

    #[test]
    fn stop_sets_end_time() {
        let mut log = TaskLog::parse(SAMPLE).unwrap();
        log.stop().unwrap();
        assert!(log.entries[1].end.is_some());
        assert!(log.active().is_none());
    }

    #[test]
    fn stop_with_no_active_task_errors() {
        let input = "2026-04-06 09:15:04 -- 2026-04-06 11:30:10 | work/fix-bug";
        let mut log = TaskLog::parse(input).unwrap();
        let err = log.stop().unwrap_err();
        assert!(matches!(err, TaktError::NoActiveTask));
    }

    #[test]
    fn parse_malformed_line_errors() {
        let input = "this is not valid";
        let err = TaskLog::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::MalformedLine { .. }));
    }

    #[test]
    fn parse_bad_datetime_errors() {
        let input = "not-a-date -- 2026-04-06 11:30:10 | work/task";
        let err = TaskLog::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::BadDateTime { .. }));
    }
}
