use std::fmt;

use chrono::{DateTime, Local};

pub struct Author {
    name: String,
    email: String,
    time: DateTime<Local>,
}

impl Author {
    pub fn new(name: String, email: String, time: DateTime<Local>) -> Author {
        Author { name, email, time }
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let timestamp = format!("{}", self.time.format("%s %z"));
        write!(f, "{} <{}> {}", self.name, self.email, timestamp)
    }
}
