use std::fmt::Display;
use chrono::NaiveDateTime;
use crate::chrome_time;

#[derive(Debug)]
pub struct Row {
    pub url: String,
    pub title: String,
    pub visit_count: i32,
    pub last_visit_time: i64,
}

impl Row {
    pub fn new(url: String, title: String, visit_count: i32, last_visit_time: i64) -> Self {
        Self { url, title, visit_count, last_visit_time }
    }
}

impl Display for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dt: NaiveDateTime = chrome_time::to_datetime(self.last_visit_time);
        write!(
            f,
            "[{}] {} ({} visits)\n    {}",
            dt.format("%Y-%m-%d %H:%M:%S"),
            self.title,
            self.visit_count,
            self.url
        )
    }
}