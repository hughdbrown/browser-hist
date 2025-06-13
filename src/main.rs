use std::fs;
use std::path::PathBuf;

use dirs;
use tempfile::NamedTempFile;

use clap::{
    Arg,
    ArgMatches,
    Command,
};
use rusqlite::{
    Connection,
    OpenFlags,
};

mod chrome_time;
mod query_builder;
mod row;
mod browser_hist_error;

use query_builder::QueryBuilder;
use row::Row;
use browser_hist_error::BrowserHistError;

const CHROME_HISTORY_PATH: &str = "Library/Application Support/Google/Chrome/Default/History";


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
            row.get::<_, String>("url")?,
            row.get::<_, String>("title")?,
            row.get::<_, i32>("visit_count")?,
            row.get::<_, i64>("last_visit_time")?,
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
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(CHROME_HISTORY_PATH)
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
