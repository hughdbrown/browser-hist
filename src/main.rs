//use std::error::Error as StdError;
use std::fmt::{
    Display,
    Debug,
};
use std::fs;
use std::path::PathBuf;

use tempfile::NamedTempFile;

use clap::{
    Arg,
    ArgMatches,
    Command,
};
use rusqlite::{
    Connection,
    Result,
    OpenFlags,
};
use chrono::{
    NaiveDate, NaiveDateTime,
};

const CHROME_HISTORY_PATH: &str = "Library/Application Support/Google/Chrome/Default/History";
const QUERY_LIMIT: i32 = 100;
const BASE_QUERY: &str = "SELECT url, title, visit_count, last_visit_time FROM urls WHERE 1=1";

mod chrome_time {
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
}

#[derive(Debug)]
struct Row {
    url: String,
    title: String,
    visit_count: i32,
    last_visit_time: i64,
}

impl Row {
    fn new(url: String, title: String, visit_count: i32, last_visit_time: i64) -> Self {
        Row { url, title, visit_count, last_visit_time }
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

struct QueryBuilder {
    conditions: Vec<String>,
    params: Vec<Box<dyn rusqlite::ToSql>>,
    limit: Option<String>,
}

impl QueryBuilder {
    fn new() -> Self {
        QueryBuilder {
            conditions: Vec::new(),
            params: Vec::new(),
            limit: None,
        }
    }


    fn date_range(mut self, start: Option<&str>, end: Option<&str>) -> Self {
        fn get_chrome_date(date_str: Option<&str>) -> Option<i64> {
            date_str.and_then(parse_date).map(chrome_time::from_date)
        }
        match (get_chrome_date(start), get_chrome_date(end)) {
            (Some(start_ts), Some(end_ts)) => {
                self.conditions.push("last_visit_time BETWEEN ? AND ?".to_string());
                self.params.push(Box::new(start_ts));
                self.params.push(Box::new(end_ts));
            },
            (Some(start_ts), None) => {
                self.conditions.push("last_visit_time >= ?".to_string());
                self.params.push(Box::new(start_ts));
            },
            (None, Some(end_ts)) => {
                self.conditions.push("last_visit_time < ?".to_string());
                self.params.push(Box::new(end_ts));
            },
            (None, None) => {},
        }
        self
    }

    fn title_search(mut self, search: Option<&str>) -> Self {
        if let Some(search_term) = search {
            self.conditions.push("title LIKE ?".to_string());
            self.params.push(Box::new(format!("%{}%", search_term)));
        }
        self
    }

    fn url_search(mut self, url: Option<&str>) -> Self {
        if let Some(url_term) = url {
            self.conditions.push("url LIKE ?".to_string());
            self.params.push(Box::new(format!("%{}%", url_term)));
        }
        self
    }

    fn limit(mut self, limit: Option<&str>) -> Self {
        if let Some(limit_term) = limit {
            self.limit = Some(format!(" LIMIT {}", limit_term));
        }
        self
    }
    fn build(self) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
        let mut query = String::from(BASE_QUERY);

        for condition in &self.conditions {
            query.push_str(" AND ");
            query.push_str(condition);
        }
        
        query.push_str(" ORDER BY last_visit_time DESC");
        if let Some(limit) = self.limit {
            query.push_str(&limit);
        }
        
        (query, self.params)
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}


#[derive(Debug)]
enum BrowserHistError {
    Rus(rusqlite::Error),
    Io(std::io::Error),
}

impl From<std::io::Error> for BrowserHistError {
    fn from(err: std::io::Error) -> Self {
        BrowserHistError::Io(err)
    }
}

impl From<rusqlite::Error> for BrowserHistError {
    fn from(err: rusqlite::Error) -> Self {
        BrowserHistError::Rus(err)
    }
}

fn get_matches() -> ArgMatches {
    Command::new("chrome-history-search")
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
        .arg(Arg::new("limit")
            .long("limit")
            .short('l')
            .help("Limit number of matches"))
        .get_matches()
}

fn build_sql(matches: &ArgMatches) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    QueryBuilder::new()
        .date_range(
            matches.get_one::<String>("start-date").map(|s| s.as_str()),
            matches.get_one::<String>("end-date").map(|s| s.as_str())
        )
        .title_search(matches.get_one::<String>("search").map(|s| s.as_str()))
        .url_search(matches.get_one::<String>("url").map(|s| s.as_str()))
        .limit(matches.get_one::<String>("limit").map(|s| s.as_str()))
        .build()
}

fn get_rows(
    conn: &Connection,
    query: &str,
    params_vec: &[Box<dyn rusqlite::ToSql>]
) -> Result<Vec<Row>, BrowserHistError> {
    let mut stmt = conn.prepare(query)?;
    let params: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| &**b).collect();
    
    stmt.query_map(params.as_slice(), |row| {
        Ok(Row::new(
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i32>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?.collect::<Result<Vec<_>, _>>()
        .map_err(BrowserHistError::from)
}

fn print_rows(rows: &[Row]) {
    for row in rows {
        println!("{}", row);
    }
}

fn get_history_db() -> PathBuf {
    // Get Chrome history path
    let home: String = std::env::var("HOME").expect("Could not determine home directory");
    let mut history_path: PathBuf = PathBuf::from(home);
    history_path.push(CHROME_HISTORY_PATH);
    history_path
}

fn main() -> Result<(), BrowserHistError> {
    let matches: ArgMatches = get_matches();
    let (query, params_vec): (String, Vec<Box<dyn rusqlite::ToSql>>) = build_sql(&matches);

    let history_path: PathBuf = get_history_db();
    let temp_file: NamedTempFile = NamedTempFile::new()?;
    fs::copy(history_path, temp_file.path())?;

    let conn: Connection = Connection::open_with_flags(
        temp_file.path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

    let rows: Vec<Row> = get_rows(&conn, &query, &params_vec)?;
    print_rows(&rows);

    Ok(())
}
