use std::collections::BTreeMap;

use chrono::{Datelike, Local, Months, NaiveDate, NaiveDateTime, TimeDelta};
use clap::{Subcommand, ValueEnum};

use crate::log::Entry;

#[derive(Clone, Copy, ValueEnum)]
pub enum Period {
    Day,
    Week,
    Month,
}

#[derive(Clone, Subcommand)]
pub enum ReportRange {
    /// Report for the current day/week/month
    This { period: Period },
    /// Report spanning the last N day/week/month
    Last { n: u16, period: Period },
}

impl ReportRange {
    pub fn date_range(&self) -> (NaiveDateTime, NaiveDateTime) {
        let now = Local::now().naive_local();
        let today = now.date();

        let start_of_week = |d: NaiveDate| -> NaiveDate {
            d - TimeDelta::days(d.weekday().num_days_from_monday() as i64)
        };
        let start_of_month =
            |d: NaiveDate| -> NaiveDate { d.with_day(1).unwrap() };

        let (start, end) = match self {
            Self::This {
                period: Period::Day,
            } => (today, now),
            Self::This {
                period: Period::Week,
            } => (start_of_week(today), now),
            Self::This {
                period: Period::Month,
            } => (start_of_month(today), now),
            Self::Last {
                n,
                period: Period::Day,
            } => {
                let past = today - TimeDelta::days(*n as i64);
                (past, now)
            }
            Self::Last {
                n,
                period: Period::Week,
            } => {
                let past = start_of_week(today) - TimeDelta::weeks(*n as i64);
                (past, now)
            }
            Self::Last {
                n,
                period: Period::Month,
            } => {
                let past = start_of_month(today - Months::new(*n as u32));
                (past, now)
            }
        };

        (start.into(), end)
    }
}

pub struct Report {
    totals: BTreeMap<String, TimeDelta>,
}

impl Report {
    pub fn generate(entries: &[Entry], range: ReportRange) -> Report {
        let mut totals = BTreeMap::new();
        let (start, end) = range.date_range();
        let entries_range =
            entries.iter().filter(|&entry| entry.start >= start);

        for entry in entries_range {
            let duration = entry.end.unwrap_or(end) - entry.start;
            *totals.entry(entry.tag.clone()).or_insert(TimeDelta::zero()) +=
                duration;
        }
        Report { totals }
    }

    pub fn display(&self) -> String {
        let mut output = String::new();
        let total: TimeDelta = self.totals.values().sum();

        let tag_width = self
            .totals
            .keys()
            .map(|k| k.len())
            .chain(std::iter::once("Total".len()))
            .max()
            .unwrap_or(0);

        let hours_width = self
            .totals
            .values()
            .chain(std::iter::once(&total))
            .map(|d| d.num_hours().to_string().len())
            .max()
            .unwrap_or(1);

        for (tag, duration) in &self.totals {
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() - hours * 60;
            output.push_str(&format!(
                "{tag:<tag_width$} {hours:>hours_width$}h {minutes:>2}m\n"
            ));
        }

        let total_hours = total.num_hours();
        let total_minutes = total.num_minutes() - total_hours * 60;
        output.push_str(&format!(
            "{:<tag_width$} {total_hours:>hours_width$}h {total_minutes:>2}m\n",
            "Total"
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};

    fn entry(tag: &str, start: NaiveDateTime, end: Option<NaiveDateTime>) -> Entry {
        Entry {
            start,
            end,
            tag: tag.to_string(),
        }
    }

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(h, min, 0).unwrap())
    }

    #[test]
    fn this_day_starts_at_midnight_today() {
        let (start, _) = ReportRange::This { period: Period::Day }.date_range();
        let today = Local::now().date_naive();
        assert_eq!(start.date(), today);
        assert_eq!(start.time(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn last_n_days_goes_back_exactly_n_days() {
        let (start, _) =
            ReportRange::Last { n: 7, period: Period::Day }.date_range();
        let expected = Local::now().date_naive() - TimeDelta::days(7);
        assert_eq!(start.date(), expected);
    }

    #[test]
    fn this_week_starts_on_monday() {
        let (start, _) =
            ReportRange::This { period: Period::Week }.date_range();
        assert_eq!(start.date().weekday().num_days_from_monday(), 0);
    }

    #[test]
    fn this_month_starts_on_the_first() {
        let (start, _) =
            ReportRange::This { period: Period::Month }.date_range();
        assert_eq!(start.date().day(), 1);
    }

    #[test]
    fn generate_sums_durations_per_tag() {
        // Use a wide range so fixture entries always fall inside it.
        let range = ReportRange::Last { n: 3650, period: Period::Day };
        let entries = vec![
            entry("work", dt(2026, 4, 1, 9, 0), Some(dt(2026, 4, 1, 10, 30))),
            entry("work", dt(2026, 4, 2, 9, 0), Some(dt(2026, 4, 2, 11, 0))),
            entry("study", dt(2026, 4, 3, 14, 0), Some(dt(2026, 4, 3, 15, 0))),
        ];
        let report = Report::generate(&entries, range);
        assert_eq!(
            report.totals.get("work").copied(),
            Some(TimeDelta::minutes(90 + 120))
        );
        assert_eq!(
            report.totals.get("study").copied(),
            Some(TimeDelta::hours(1))
        );
    }

    #[test]
    fn generate_filters_entries_before_range_start() {
        // Window is "last 1 day" — old fixture entries should not be counted.
        let range = ReportRange::Last { n: 1, period: Period::Day };
        let long_ago = dt(2020, 1, 1, 9, 0);
        let entries = vec![entry("work", long_ago, Some(long_ago + TimeDelta::hours(2)))];
        let report = Report::generate(&entries, range);
        assert!(report.totals.is_empty());
    }

    #[test]
    fn display_includes_total_row() {
        let range = ReportRange::Last { n: 3650, period: Period::Day };
        let entries = vec![
            entry("work", dt(2026, 4, 1, 9, 0), Some(dt(2026, 4, 1, 10, 0))),
            entry("study", dt(2026, 4, 1, 11, 0), Some(dt(2026, 4, 1, 12, 30))),
        ];
        let rendered = Report::generate(&entries, range).display();
        assert!(rendered.contains("work"));
        assert!(rendered.contains("study"));
        assert!(rendered.contains("Total"));
        assert!(rendered.contains("2h 30m"));
    }
}
