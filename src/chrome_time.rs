use chrono::{NaiveDate, NaiveDateTime};

/// Chrome epoch: 1601-01-01T00:00:00Z
const CHROME_EPOCH: NaiveDateTime = match NaiveDate::from_ymd_opt(1601, 1, 1) {
    Some(date) => match date.and_hms_opt(0, 0, 0) {
        Some(datetime) => datetime,
        None => panic!("Invalid time"),
    },
    None => panic!("Invalid date"),
};

/// Converts a chrono NaiveDate to Chrome's timestamp (microseconds since 1601-01-01T00:00:00Z)
pub fn from_date(date: NaiveDate) -> i64 {
    let duration = date.and_hms_opt(0, 0, 0).unwrap() - CHROME_EPOCH;
    duration.num_microseconds().unwrap()
}

/// Converts Chrome's timestamp to chrono NaiveDateTime
pub fn to_datetime(ts: i64) -> NaiveDateTime {
    CHROME_EPOCH + chrono::Duration::microseconds(ts)
}