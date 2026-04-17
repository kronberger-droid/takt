use std::collections::BTreeMap;

use chrono::{Datelike, Local, Months, NaiveDate, NaiveDateTime, TimeDelta};

use crate::log::Entry;

enum Period {
    Day,
    Week,
    Month,
}

enum ReportRange {
    This(Period),
    Last(u16, Period),
}

impl ReportRange {
    fn date_range(&self) -> (NaiveDateTime, NaiveDateTime) {
        let now = Local::now().naive_local();
        let today = now.date();

        // TODO: implement these two closures
        let start_of_week = |d: NaiveDate| -> NaiveDate {
            d - TimeDelta::days(d.weekday().num_days_from_monday() as i64)
        };
        let start_of_month =
            |d: NaiveDate| -> NaiveDate { d.with_day(1).unwrap() };

        let (start, end) = match self {
            Self::This(Period::Day) => (today, now),
            Self::This(Period::Week) => (start_of_week(today), now),
            Self::This(Period::Month) => (start_of_month(today), now),
            Self::Last(n, Period::Day) => {
                let past = today - TimeDelta::days(*n as i64);
                (past, now)
            }
            Self::Last(n, Period::Week) => {
                let past = start_of_week(today) - TimeDelta::weeks(*n as i64);
                (past, now)
            }
            Self::Last(n, Period::Month) => {
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
        let (start, _) = range.date_range();
        let entries_range =
            entries.iter().filter(|&entry| entry.start >= start);

        for entry in entries_range {
            let duration =
                entry.end.unwrap_or(Local::now().naive_local()) - entry.start;

            if let Some(total) = totals.get_mut(&entry.tag) {
                *total += duration;
            } else {
                totals.insert(entry.tag.clone(), duration);
            };
        }
        Report { totals }
    }

    pub fn display(&self) -> String {
        let mut output = String::new();
        let total: TimeDelta = self.totals.values().sum();

        let width = self.totals.keys().map(|k| k.len()).max().unwrap_or(0);

        for (tag, duration) in &self.totals {
            let hours = duration.num_hours();
            let minutes = duration.num_minutes() - hours * 60;
            let entry_str = format!(
                "{:<width$} {}h {}m\n",
                tag,
                duration.num_hours(),
                minutes
            );
            output.push_str(&entry_str);
        }

        let total_hours = total.num_hours();
        let total_minutes = total.num_minutes() - total_hours * 60;
        let total_tag = "Total";

        output.push_str(&format!(
            "{:<width$} {}h {}m\n",
            total_tag, total_hours, total_minutes
        ));

        output
    }
}
