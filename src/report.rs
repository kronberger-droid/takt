use std::collections::HashMap;

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
        let start_of_week = |d: NaiveDate| -> NaiveDate { todo!() };
        let start_of_month = |d: NaiveDate| -> NaiveDate { todo!() };

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

        (start.into(), end.into())
    }
}

pub struct Report {
    totals: HashMap<String, TimeDelta>,
}

impl Report {
    pub fn generate(entries: &[Entry], range: ReportRange) -> Report {
        todo!()
    }

    pub fn display(&self) -> String {
        todo!()
    }
}
