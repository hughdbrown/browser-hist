use rusqlite;
use crate::chrome_time;
use chrono::NaiveDate;

const BASE_QUERY: &str = "SELECT url, title, visit_count, last_visit_time FROM urls WHERE 1=1";

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

#[derive(Default)]
pub struct QueryBuilder {
    conditions: Vec<String>,
    params: Vec<Box<dyn rusqlite::ToSql>>,
    limit: Option<String>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    fn add_condition<T>(&mut self, condition: &str, params: &[T])
        where T: rusqlite::ToSql + Clone + 'static
    {
        self.conditions.push(condition.to_string());
        for param in params {
            self.params.push(Box::new(param.clone()));
        }
    }

    pub fn date_range(mut self, start: Option<&str>, end: Option<&str>) -> Self {
        fn get_chrome_date(date_str: Option<&str>) -> Option<i64> {
            date_str.and_then(parse_date).map(chrome_time::from_date)
        }
        match (get_chrome_date(start), get_chrome_date(end)) {
            (Some(start_ts), Some(end_ts)) => {
                self.add_condition("last_visit_time BETWEEN ? AND ?", &[start_ts, end_ts]);
            },
            (Some(start_ts), None) => {
                self.add_condition("last_visit_time >= ?", &[start_ts]);
            },
            (None, Some(end_ts)) => {
                self.add_condition("last_visit_time < ?", &[end_ts]);
            },
            (None, None) => {},
        }
        self
    }

    pub fn title_search(mut self, search: Option<&str>) -> Self {
        if let Some(term) = search {
            self.add_condition("title LIKE ?", &[format!("%{}%", term)]);
        }
        self
    }

    pub fn url_search(mut self, url: Option<&str>) -> Self {
        if let Some(term) = url {
            self.add_condition("url LIKE ?", &[format!("%{}%", term)]);
        }
        self
    }

    pub fn limit(mut self, limit: Option<&str>) -> Self {
        if let Some(limit_term) = limit {
            self.limit = Some(format!(" LIMIT {}", limit_term));
        }
        self
    }

    pub fn build(self) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
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
