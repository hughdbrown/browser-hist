use rusqlite;

#[derive(Debug)]
pub enum BrowserHistError {
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


