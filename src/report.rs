use std::collections::HashMap;

use chrono::{NaiveDateTime, TimeDelta};

use crate::log::Entry;

enum ReportRange {
    Today,
    Week,
    Month,
}

impl ReportRange {
    fn date_range(&self) -> (NaiveDateTime, NaiveDateTime) {
        match self {
            Self::Today => todo!(),
            Self::Week => todo!(),
            Self::Month => todo!(),
        }
    }
}

pub struct Report {
    totals: HashMap<String, TimeDelta>,
}

impl Report {
    pub fn generate(entries: &[Entry], tange: ReportRange) -> Report {
        todo!()
    }

    pub fn display(&self) -> String {
        todo!()
    }
}
