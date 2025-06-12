//use std::error::Error as StdError;
use std::fmt::{
    //Display,
    Debug,
};
use std::fs;
use std::path::PathBuf;
//use std::fmt::Error;
// use std::io::Error;
//use core::error::Error;

use tempfile::NamedTempFile;

use clap::{
    Arg,
    ArgMatches,
    Command,
    // Parser,
};
use rusqlite::{
    Connection,
    Result,
    OpenFlags,
    // params,
    // NO_PARAMS,
};
use chrono::{
    NaiveDate, NaiveDateTime,
    // TimeZone, Utc,
};


fn parse_date(s: &str) -> Option<NaiveDate> {
    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    Some(date)
}

/// Converts a chrono NaiveDate to Chrome's timestamp (microseconds since 1601-01-01T00:00:00Z)
fn date_to_chrome_time(date: NaiveDate) -> i64 {
    // Chrome epoch: 1601-01-01T00:00:00Z
    let chrome_epoch = NaiveDate::from_ymd_opt(1601, 1, 1).unwrap()
        .and_hms_opt(0, 0, 0).unwrap();
    let duration = date.and_hms_opt(0, 0, 0).unwrap() - chrome_epoch;
    duration.num_microseconds().unwrap()
}

/// Converts Chrome's timestamp to chrono NaiveDateTime
fn chrome_time_to_naive(ts: i64) -> NaiveDateTime {
    let chrome_epoch = NaiveDate::from_ymd_opt(1601, 1, 1).unwrap()
        .and_hms_opt(0, 0, 0).unwrap();
    chrome_epoch + chrono::Duration::microseconds(ts)
}

// std::error::Error, std::fmt::Display, and std::fmt::Debug
#[derive(Debug)]
enum CustomError {
    Rus(rusqlite::Error),
    Io(std::io::Error),
}

impl From<std::io::Error> for CustomError {
    fn from(err: std::io::Error) -> Self {
        CustomError::Io(err)
    }
}

impl From<rusqlite::Error> for CustomError {
    fn from(err: rusqlite::Error) -> Self {
        CustomError::Rus(err)
    }
}

fn get_matches() -> ArgMatches {
    let matches: ArgMatches = Command::new("chrome-history-search")
        .version("1.0")
        .about("Search Chrome browser history on macOS")
        .arg(Arg::new("start-date")
            .long("start-date")
            .help("Date (YYYY-MM-DD) for last visit"))
        .arg(Arg::new("end-date")
            .long("end-date")
            .help("Date (YYYY-MM-DD) for last visit"))
        .arg(Arg::new("search")
            .long("search")
            .short('s')
            .help("Text to search for in the page title"))
        .arg(Arg::new("url")
            .long("url")
            .short('u')
            .help("Domain or text to search for in the URL"))
        .get_matches();

    matches
}

fn build_sql(matches: &ArgMatches)
-> (
    String,
    Vec<Box<dyn rusqlite::ToSql>>
)
{
    // Build SQL query
    let mut query = String::from("SELECT url, title, visit_count, last_visit_time FROM urls WHERE 1=1");
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    // Date range
    let sd: Option<&String> = matches.get_one::<String>("start-date");
    let ed: Option<&String> = matches.get_one::<String>("end-date");
    match(sd, ed) {
        (Some(sd), Some(ed)) => {
            let sd: Option<NaiveDate> = parse_date(sd.as_str());
            let sd: NaiveDate = sd.unwrap();
            let start_ts = date_to_chrome_time(sd);
            let ed: Option<NaiveDate> = parse_date(ed.as_str());
            let ed: NaiveDate = ed.unwrap();
            let end_ts = date_to_chrome_time(ed);
            query.push_str(" AND last_visit_time BETWEEN ? AND ?");
            params_vec.push(Box::new(start_ts));
            params_vec.push(Box::new(end_ts));
        },
        (Some(sd), None) => {
            let sd: Option<NaiveDate> = parse_date(sd.as_str());
            let sd: NaiveDate = sd.unwrap();
            let start_ts = date_to_chrome_time(sd);
            query.push_str(" AND last_visit_time >= ?");
            params_vec.push(Box::new(start_ts));
        },
        (None, Some(ed)) => {
            let ed: Option<NaiveDate> = parse_date(ed.as_str());
            let ed: NaiveDate = ed.unwrap();
            let end_ts = date_to_chrome_time(ed);
            query.push_str(" AND last_visit_time < ?");
            params_vec.push(Box::new(end_ts));
        },
        (None, None) => {},
    } 

    // Title search
    if let Some(search) = matches.get_one::<String>("search") {
        query.push_str(" AND title LIKE ?");
        params_vec.push(Box::new(format!("%{}%", search)));
    }

    // URL/domain search
    if let Some(url) = matches.get_one::<String>("url") {
        query.push_str(" AND url LIKE ?");
        params_vec.push(Box::new(format!("%{}%", url)));
    }

    query.push_str(" ORDER BY last_visit_time DESC LIMIT 100");

    (query, params_vec)
}

fn main() -> Result<(), CustomError> {
    let matches: ArgMatches = get_matches();
    let (query, params_vec) = build_sql(&matches);

    // Get Chrome history path
    let home = std::env::var("HOME").expect("Could not determine home directory");
    let mut history_path = PathBuf::from(home);
    history_path.push("Library/Application Support/Google/Chrome/Default/History");

    let temp_file = NamedTempFile::new()?;
    fs::copy(history_path, temp_file.path())?;

    let conn = Connection::open_with_flags(
        temp_file.path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    let mut stmt = conn.prepare(&query)?;
    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| &**b).collect();
    let rows = stmt.query_map(
        params.as_slice(),
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i64>(3)?,
            ))
        }
    )?;

    for row in rows {
        let (url, title, visit_count, last_visit_time) = row?;
        let dt = chrome_time_to_naive(last_visit_time);
        println!(
            "[{}] {} ({} visits)\n    {}\n",
            dt.format("%Y-%m-%d %H:%M:%S"),
            title,
            visit_count,
            url
        );
    }

    Ok(())
}
